# Plan 2 — Solver and Generator (Track 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `fap-solver` crate that checks uniqueness and independent solvability exactly, grades human difficulty by level, and generates region-aware puzzles in parallel with progress and cancellation. It is exposed through `fap check`, `fap grade` and `fap generate`.

**Architecture:** Every check runs on a `Problem`: a set of clue constraints plus fixed givens over cell variables, with a list of *target* cells. A `Problem` is built for the whole puzzle or for one region's sub-puzzle (the region plus its margin D).

Two solvers share one propagation state:

- **Exact solver** (propagation + DFS). It is the authority on uniqueness.
- **Graded solver.** It applies levels 1–4 simplest-first and records which level resolved each cell. It only makes sound deductions, so "the graded solver resolved every target" implies "the targets are forced".

That implication lets the generator use the graded solver alone as its acceptance test. The exact solver is needed only for preflight and final verification.

Generation runs region by region, easiest first, locking the margin clues each region depends on. Then it handles the unassigned cells, then verifies.

**Tech Stack:** Rust 2024; `fap-core` (Plan 0); `rand` + `rand_chacha` (seeded ChaCha8 for determinism); `serde` + `toml` for the `[generation]`/`[stats]` schemas; `thiserror`; `std::thread::scope` for parallel candidates (no rayon).

**Spec:** `docs/superpowers/specs/2026-10-29-fill-a-pix-design.md`: §6 throughout, §4 (puzzle file), §8 Track 2, §9.

**Builds on:** `docs/superpowers/plans/2026-10-29-plan-0-foundation.md`. Use its interfaces exactly: `Pos`, `Grid`, `Image`, `Clue`, `Puzzle`, `Regions`, `RegionId`, `RegionSettings`, `clues::neighborhood`, `clues::clue_at`, `format::*`. Read both documents before starting.

> **Plan 0 final-review additions (merged after this plan was drafted):** `fap-core` now also provides `Regions::validate()`, `Project::validate()`, `regions::MAX_REGION_ID` (= `u32::MAX / 2`; larger IDs are rejected on load) and `pub use toml` (use `fap_core::toml` instead of adding your own `toml` dependency). `Regions` equality ignores the internal `next_id` cursor, so a saved and reloaded value compares equal. `write_project_file` / `write_puzzle_file` validate before writing and return `Err` without touching the old file. `Puzzle::validate()` also validates its regions and rejects digits above 9. Opaque `[generation]` / `[pdf]` / `[stats]` tables round-trip exactly, datetimes included, but a NaN float makes `Project`/`Puzzle` compare unequal, so never write NaN into them (e.g. use 0.0 for an empty average).

## Global Constraints

- **From Plan 0, verbatim:**
  - prefix every command with `nix develop -c`;
  - edition 2024, no `unsafe`, `license = "GPL-3.0-or-later"`;
  - add dependencies with `cargo add`;
  - conventional commits (`feat(solver): …`, `feat(cli): …`) with the two attribution lines;
  - before every commit, run fmt, clippy with `-D warnings`, and `cargo test --workspace`.
- **Levels** are exactly 1–4 (spec §6.2); level 0 means "given" in reports. Margin D is 0–2. Floors apply only to targets of 2 or more.
- **Independent solvability** (spec §6.4) uses only the clues (digits and givens) in cells within Chebyshev distance D of some region cell. Every other cell is unknown, including the true values of cells outside the region. Only the region's own cells must be forced.
- **Givens:** a cell is never both a given and a digit (spec §4).
- **Determinism:** the same image, regions, settings and seed always give the same puzzle, regardless of thread count.
  - All randomness comes from `rand_chacha::ChaCha8Rng` seeded from a `u64`.
  - No iteration over a `HashMap` or `HashSet` may affect output. Use `BTreeMap`, a sorted `Vec`, or index order.
- **Termination:** every graded or exact evaluation runs under a `Budget`, and generation loops are finite (each clue is tried a bounded number of times). Generation therefore always terminates.
- **Verification:** the generator never exports a puzzle that fails final exact verification (spec §2 anti-goal).
- **`fap-core` is off limits:** this plan does not modify it. Tracks 1 and 3 share it, so if a core change seems needed, stop and report it.

## Review Focus

1. **Trivial images.** An all-blank or all-shaded image (all clues 0, or 9 in the interior) must generate a sparse valid puzzle, not crash or keep every clue. Test: `blank_image_generates` (Task 6).
2. **Regions at a grid edge or corner.** The margin must clip to the grid without panicking or losing clues. Test: `sub_clips_margin_at_grid_edge` (Task 1).
3. **Cancel mid-run.** Pressing Cancel during generation must return within about a second, not after the current region finishes. Test: `cancel_mid_run_returns_promptly` (Task 8).
4. **Bad `[generation]` values in a hand-edited project** (`givens = "sometimes"`, a negative density, `candidates = 0`). These give a clear error naming the key, never a panic, and missing keys take their defaults. Test: `settings_from_table_rejects_bad_values` (Task 4).
5. **A puzzle the grader can never finish within budget.** Generation still terminates, returns an exactly verified (denser) puzzle, and flags the ceiling as unmet instead of failing. Test: `tiny_budget_still_terminates_with_valid_puzzle` (Task 6).

---

## File Structure

```
crates/fap-solver/
  Cargo.toml
  src/lib.rs            re-exports (see "Public API" below)
  src/budget.rs         Halt, Budget, Control, Progress, Phase
  src/problem.rs        Var, Constraint, Problem, margin_cells
  src/state.rs          shared propagation state (assignments, per-constraint need/unknown, trail)
  src/exact.rs          Verdict, check
  src/graded.rs         GradeReport, grade (levels 1–4)
  src/score.rs          score
  src/settings.rs       GenerationSettings, GivensMode (+ toml::Table conversion)
  src/stats.rs          PuzzleStats, RegionStats, UnassignedStats (+ toml::Table conversion)
  src/evaluate.rs       RegionEval, UnassignedEval, Evaluation, evaluate_region, evaluate_unassigned, evaluate_puzzle
  src/preflight.rs      preflight (fully-clued checks, givens modes)
  src/generate.rs       Candidate, GenerateError, candidate_seed, region_order, generate_one
  src/parallel.rs       generate (parallel candidates)
  src/testing.rs        #[doc(hidden)] pub helpers for tests, examples, perf
  examples/find_fixtures.rs, examples/experiments.rs
  tests/{problem,exact,graded,evaluate,preflight,generate,regions_gen,parallel}.rs
  tests/perf.rs         #[ignore] performance checks
  tests/fixtures/level3.puzzle.toml, level4.puzzle.toml   (written by find_fixtures)
crates/fap-cli/
  src/main.rs           add Check, Grade, Generate variants to Plan 0's Command enum
  src/solve_commands.rs new: check / grade / generate implementations
  tests/solve_cli.rs
docs/notes/2026-10-29-solver-experiments.md   (Tasks 10–11)
```

**Public API (crate root re-exports):** `Budget, Halt, Control, Progress, Phase, Problem, Constraint, Var, margin_cells, GenerationSettings, GivensMode, PuzzleStats, RegionStats, UnassignedStats, Candidate, GenerateError, candidate_seed, region_order, generate_one, generate`. Modules: `exact, graded, score, evaluate, preflight, testing`.

---

### Task 1: Crate scaffold, budget/control, Problem construction

**Files:**
- Create: `crates/fap-solver/Cargo.toml`, `src/lib.rs`, `src/budget.rs`, `src/problem.rs`, `src/testing.rs`
- Test: `crates/fap-solver/tests/problem.rs`

**Interfaces:**
- Consumes: `fap_core::{Pos, Grid, Image, Clue, Puzzle, Regions, RegionId, clues}`.
- Produces:
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Halt { BudgetExhausted, Cancelled }`.
  - `pub struct Budget`: an `AtomicI64` of remaining steps plus an optional `Arc<AtomicBool>` cancel flag.
    - `Budget::new(steps: i64)` and `Budget::with_cancel(steps: i64, cancel: Arc<AtomicBool>)`.
    - `spend(&self, n: i64) -> Result<(), Halt>`: checks the cancel flag first (giving `Cancelled`), then subtracts. Returns `BudgetExhausted` once the remaining count would drop below 0.
    - `remaining(&self) -> i64`.
  - `#[derive(Clone, Default)] pub struct Control { cancel: Arc<AtomicBool>, progress: Option<std::sync::mpsc::Sender<Progress>> }`.
    - `Control::new()`, `Control::with_progress(tx)`, `cancel(&self)`, `is_cancelled(&self) -> bool`, `cancel_flag(&self) -> Arc<AtomicBool>`.
    - `pub(crate) fn report(&self, p: Progress)`, which ignores send errors.
  - `#[derive(Debug, Clone, PartialEq)] pub struct Progress { pub candidate: u32, pub phase: Phase, pub fraction: f32 }`. `fraction` is completed removal trials divided by total trials for this candidate, in 0..=1, and never decreases.
  - `#[derive(Debug, Clone, PartialEq)] pub enum Phase { Preflight, Region { id: RegionId, index: u32, of: u32 }, Unassigned, Verify, Done }`.
  - `pub type Var = u32;` and `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Constraint { pub at: Pos, pub vars: Vec<Var>, pub sum: u8 }`.
  - `pub struct Problem` (private fields).
    - `Problem::whole(clues: &Grid<Clue>) -> Problem`: every cell is a target, and every digit is a constraint.
    - `Problem::sub(clues: &Grid<Clue>, region: &[Pos], margin: u8) -> Problem`.
    - `num_vars()`, `pos(v: Var) -> Pos`, `var(p: Pos) -> Option<Var>`.
    - `constraints() -> &[Constraint]`, sorted row-major by `at`.
    - `fixed() -> &[(Var, bool)]`, sorted row-major by position.
    - `targets() -> impl Iterator<Item = Pos> + '_` (row-major) and `target_vars() -> &[Var]`.
  - `pub fn margin_cells(width: usize, height: usize, region: &[Pos], margin: u8) -> Vec<Pos>`: the Chebyshev dilation of `region` by `margin`, clipped to the grid, sorted row-major, and including the region itself.
  - `testing` (all `pub`, module `#[doc(hidden)]`):
    - `img(rows: &[&str]) -> Image` (`#` shaded);
    - `full_clues(image: &Image) -> Grid<Clue>`;
    - `random_image(width, height, density: f64, seed: u64) -> Image`;
    - `thin(clues: &Grid<Clue>, keep: f64, seed: u64) -> Grid<Clue>`, which keeps each `Digit` with probability `keep` and otherwise makes it `Blank`;
    - `synthetic_image(width, height, seed: u64) -> Image`, an XOR of `width*height/60` random filled rectangles and discs of size 2–8, so it resembles pixel-art blobs rather than noise.
- `Problem::sub` rules. Let K = `margin_cells(region, margin)`.
  - Constraints: one per `Digit` clue in K, over that cell's `clues::neighborhood`.
  - Fixed: one `(var, value)` per given in K. `GivenShaded` gives true, `GivenUnshaded` gives false.
  - Variables: the region cells, every cell in any constraint's neighbourhood, and every fixed cell.
  - Targets: the region cells.
  - Clues outside K are ignored entirely.

- [ ] **Step 1: Create the crate and add deps**

Run: `nix develop -c cargo new --lib crates/fap-solver && nix develop -c cargo add -p fap-solver --path crates/fap-core && nix develop -c cargo add -p fap-solver rand rand_chacha thiserror toml && nix develop -c cargo add -p fap-solver serde --features derive`
Then set `edition.workspace = true`, `license.workspace = true` and `version.workspace = true`.

- [ ] **Step 2: Write the failing tests** in `crates/fap-solver/tests/problem.rs`

```rust
use fap_core::{Clue, Pos};
use fap_solver::testing::{full_clues, img};
use fap_solver::{margin_cells, Budget, Control, Halt, Problem};

fn at(p: &Problem) -> Vec<usize> { p.constraints().iter().map(|k| k.at.col).collect() }
fn var_cols(p: &Problem) -> Vec<usize> {
    let mut v: Vec<usize> = (0..p.num_vars() as u32).map(|i| p.pos(i).col).collect();
    v.sort();
    v
}

#[test]
fn whole_problem_covers_every_cell() {
    let p = Problem::whole(&full_clues(&img(&["#..", ".#."])));
    assert_eq!(p.num_vars(), 6);
    assert_eq!(p.constraints().len(), 6);
    assert_eq!(p.targets().count(), 6);
    assert!(p.fixed().is_empty());
}

#[test]
fn sub_problem_scope_grows_with_margin() {
    let c = full_clues(&img(&["#...#"]));
    let r = [Pos::new(0, 2)];
    let p0 = Problem::sub(&c, &r, 0);
    assert_eq!(at(&p0), vec![2]);
    assert_eq!(var_cols(&p0), vec![1, 2, 3]);
    let p1 = Problem::sub(&c, &r, 1);
    assert_eq!(at(&p1), vec![1, 2, 3]);
    assert_eq!(var_cols(&p1), vec![0, 1, 2, 3, 4]);
    let p2 = Problem::sub(&c, &r, 2);
    assert_eq!(at(&p2), vec![0, 1, 2, 3, 4]);
    assert_eq!(p2.targets().collect::<Vec<_>>(), vec![Pos::new(0, 2)]);
}

#[test]
fn sub_clips_margin_at_grid_edge() {
    let c = full_clues(&img(&["#..", "...", "..#"]));
    let p = Problem::sub(&c, &[Pos::new(0, 0)], 2);
    assert_eq!(p.constraints().len(), 9);
    assert_eq!(p.num_vars(), 9);
}

#[test]
fn givens_in_scope_become_fixed_and_blanks_are_skipped() {
    let mut c = full_clues(&img(&["#...#"]));
    c.set(Pos::new(0, 1), Clue::Blank);
    c.set(Pos::new(0, 3), Clue::GivenUnshaded);
    c.set(Pos::new(0, 4), Clue::GivenShaded);
    let p = Problem::sub(&c, &[Pos::new(0, 2)], 1);
    assert_eq!(at(&p), vec![2]);
    let fixed: Vec<(Pos, bool)> = p.fixed().iter().map(|(v, b)| (p.pos(*v), *b)).collect();
    assert_eq!(fixed, vec![(Pos::new(0, 3), false)]);
}

#[test]
fn margin_cells_dilate_and_clip() {
    assert_eq!(margin_cells(5, 3, &[Pos::new(0, 0)], 1),
        vec![Pos::new(0, 0), Pos::new(0, 1), Pos::new(1, 0), Pos::new(1, 1)]);
    assert_eq!(margin_cells(5, 3, &[Pos::new(1, 2)], 0), vec![Pos::new(1, 2)]);
}

#[test]
fn budget_spends_and_halts() {
    let b = Budget::new(10);
    assert!(b.spend(4).is_ok());
    assert_eq!(b.remaining(), 6);
    assert_eq!(b.spend(7), Err(Halt::BudgetExhausted));
}

#[test]
fn cancel_flag_halts_budget() {
    let ctl = Control::new();
    let b = Budget::with_cancel(1_000, ctl.cancel_flag());
    assert!(b.spend(1).is_ok());
    ctl.cancel();
    assert!(ctl.is_cancelled());
    assert_eq!(b.spend(1), Err(Halt::Cancelled));
}
```

- [ ] **Step 3: Run to verify failure**: `nix develop -c cargo test -p fap-solver --test problem`. Expected: compile errors (items not found).

- [ ] **Step 4: Implement `budget.rs`, `problem.rs`, `testing.rs`, and the re-exports** per Interfaces.

- [ ] **Step 5: Run to verify pass**: same command. Expected: 7 passed.

- [ ] **Step 6: Commit**: `feat(solver): add solver crate with problem model and budget`

---

### Task 2: Exact solver

**Files:**
- Create: `crates/fap-solver/src/state.rs`, `crates/fap-solver/src/exact.rs`
- Test: `crates/fap-solver/tests/exact.rs`

**Interfaces:**
- Consumes: `Problem`, `Budget`, `Halt` (Task 1).
- Produces:
  - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum Verdict { Unsolvable, Forced { values: Vec<(Pos, bool)> }, NotForced { unforced: Vec<Pos> } }`.
    - Both lists cover target cells only, in row-major order.
    - For `Problem::whole`, `Forced` means the puzzle is uniquely solvable.
  - `exact::check(p: &Problem, budget: &Budget) -> Result<Verdict, Halt>`.
    - **The unforced set is exact:** a target is listed if and only if some solution gives it the opposite value.
  - `state.rs` (crate-private, reused by Task 3): a `State` with
    - `value: Vec<Option<bool>>`;
    - per-constraint `need` (sum minus known shaded) and `unknown` counts;
    - a trail for undo, with `undo_to(mark)`;
    - `assign(var, bool) -> Result<(), Contradiction>`, where a contradiction means `need < 0` or `need > unknown`;
    - an index from each variable to its constraints.
- Algorithm (baseline):
  - Propagation looks at one constraint at a time, from a work queue. Each constraint visit spends 1 budget step.
  - Search is depth-first. It branches on an unknown variable of the constraint with the fewest unknowns above 0 (ties go to the lowest index), trying true first.
  - `check`:
    1. Find any solution S. If there is none, return `Unsolvable`.
    2. Targets resolved by propagation at the root are forced.
    3. For each remaining target t not yet marked unforced, search with t set to ¬S[t]. If a solution S′ exists, mark every target where S′ ≠ S.
    4. Return `Forced` with S's target values if nothing was marked, else `NotForced`.

- [ ] **Step 1: Write the failing tests** in `crates/fap-solver/tests/exact.rs`

```rust
use fap_core::{Clue, Grid, Pos};
use fap_solver::exact::{check, Verdict};
use fap_solver::testing::{full_clues, img, random_image, thin};
use fap_solver::{Budget, Halt, Problem};

fn big() -> Budget { Budget::new(100_000_000) }

#[test]
fn unique_image_is_forced() {
    let p = Problem::whole(&full_clues(&img(&["#.."])));
    assert_eq!(check(&p, &big()).unwrap(), Verdict::Forced {
        values: vec![(Pos::new(0, 0), true), (Pos::new(0, 1), false), (Pos::new(0, 2), false)] });
}

#[test]
fn two_by_two_with_two_shaded_is_not_forced() {
    let p = Problem::whole(&full_clues(&img(&["#.", ".#"])));
    assert_eq!(check(&p, &big()).unwrap(), Verdict::NotForced {
        unforced: vec![Pos::new(0, 0), Pos::new(0, 1), Pos::new(1, 0), Pos::new(1, 1)] });
}

#[test]
fn empty_and_full_two_by_two_are_unique() {
    for rows in [["..", ".."], ["##", "##"]] {
        assert!(matches!(check(&Problem::whole(&full_clues(&img(&rows))), &big()).unwrap(), Verdict::Forced { .. }));
    }
}

#[test]
fn contradictory_clues_have_no_solution() {
    let p = Problem::whole(&Grid::new(1, 1, Clue::Digit(2)));
    assert_eq!(check(&p, &big()).unwrap(), Verdict::Unsolvable);
}

#[test]
fn margin_decides_independent_solvability() {
    let c = full_clues(&img(&["#.."]));
    let r = [Pos::new(0, 0)];
    for m in [0, 1] {
        assert_eq!(check(&Problem::sub(&c, &r, m), &big()).unwrap(),
            Verdict::NotForced { unforced: vec![Pos::new(0, 0)] }, "margin {m}");
    }
    assert_eq!(check(&Problem::sub(&c, &r, 2), &big()).unwrap(),
        Verdict::Forced { values: vec![(Pos::new(0, 0), true)] });
}

/// Brute force over all 2^16 assignments of a 4x4 grid (test helper; implementer writes the body):
/// keep assignments satisfying every Digit and given; none -> Unsolvable;
/// cells differing between kept assignments -> NotForced (row-major); else Forced with values.
fn brute(clues: &Grid<Clue>) -> Verdict { todo!() }

#[test]
fn matches_brute_force_on_random_4x4() {
    for seed in 0..30u64 {
        let clues = thin(&full_clues(&random_image(4, 4, 0.5, seed)), 0.6, seed);
        assert_eq!(check(&Problem::whole(&clues), &big()).unwrap(), brute(&clues), "seed {seed}");
    }
}

#[test]
fn exhausted_budget_halts() {
    let p = Problem::whole(&thin(&full_clues(&random_image(8, 8, 0.5, 1)), 0.3, 1));
    assert_eq!(check(&p, &Budget::new(1)), Err(Halt::BudgetExhausted));
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-solver --test exact`. Expected: compile errors (`exact` not found).

- [ ] **Step 3: Implement `state.rs` and `exact.rs`** per Interfaces.

- [ ] **Step 4: Run to verify pass**: same command. Expected: 7 passed.

- [ ] **Step 5: Commit**: `feat(solver): add exact solver with unforced-cell reporting`

---

### Task 3: Graded solver (levels 1–4)

**Files:**
- Create: `crates/fap-solver/src/graded.rs`, `crates/fap-solver/examples/find_fixtures.rs`, `crates/fap-solver/tests/fixtures/level3.puzzle.toml`, `crates/fap-solver/tests/fixtures/level4.puzzle.toml`
- Test: `crates/fap-solver/tests/graded.rs`

**Interfaces:**
- Consumes: `Problem`, `Budget`, `State` (Tasks 1–2); `exact::check` (tests only).
- Produces:
  - `#[derive(Debug, Clone, PartialEq)] pub struct GradeReport` with these public fields:
    - `solved: bool`: every target is resolved;
    - `levels: BTreeMap<Pos, u8>`: each resolved target, with 0 for a given;
    - `values: BTreeMap<Pos, bool>`;
    - `hardest: u8`: the max of `levels`, or 0 if empty;
    - `counts: [u32; 5]`: resolved targets per level 0..=4;
    - `targets: u32`;
    - `avg_unsolved: f64`;
    - `steps: i64`.
  - `graded::grade(p: &Problem, max_level: u8, budget: &Budget) -> Result<GradeReport, Halt>`, with `max_level` in 1..=4.
- Levels (spec §6.2; baseline definitions, revisited in Task 11):
  - **L1 (single clue):** for one constraint, need 0 makes all its unknowns unshaded; need equal to its unknown count makes them all shaded.
  - **L2 (two overlapping clues):** take constraints A and B whose clue cells are within Chebyshev distance 2, so their squares overlap. Split their unknowns into O (in both), Â (only in A) and B̂ (only in B). The number of shaded cells x in O satisfies `lo = max(0, needA − |Â|, needB − |B̂|)` and `hi = min(|O|, needA, needB)`. Then:
    - Â contains between `needA − hi` and `needA − lo` shaded cells. If the maximum is 0, all of Â is unshaded; if the minimum is |Â|, all of Â is shaded.
    - The same rule applies to B̂.
    - If `lo = |O|`, O is all shaded; if `hi = 0`, O is all unshaded.
  - **L3 (short hypothesis):** for an unknown variable c and a value v, assume c = v and propagate with L1 + L2 to a fixpoint. A contradiction proves c = ¬v.
  - **L4 (longer chains):** the same as L3, but propagation inside the hypothesis also uses L3, so hypotheses nest to depth 2.
- Driver (simplest-first, sketch):

  ```
  apply fixed → level 0
  loop until all targets resolved:
      for level in 1..=max_level:
          L1/L2: one sweep collecting every deduction; L3/L4: stop at the first successful probe
          if any deduction: assign them all at `level`; record unresolved-target count; continue outer loop from level 1
      if no level deduced anything: stop (solved = false)
  ```

  - **Probe order (L3/L4 baseline):** unknown variables in index order, skipping variables that appear in no constraint with unknowns.
  - **Budget:** every constraint or pair evaluation spends 1 step, including those inside hypotheses.
  - **What gets reported:** levels are recorded for non-target variables too, but `counts`, `hardest` and `levels` cover targets only.
  - **`avg_unsolved`:** the mean of the unresolved-target counts recorded after each deduction round.
- **Prefix property (must hold; tested):** `grade(p, k)` is exactly the run of `grade(p, k+1)` up to the point where the k-run gets stuck. So the levels and values for k are a subset of those for k+1, with equal entries.

- [ ] **Step 1: Write the failing tests** in `crates/fap-solver/tests/graded.rs`

```rust
use fap_core::{format, Clue, Grid, Pos};
use fap_solver::exact::{check, Verdict};
use fap_solver::graded::{grade, GradeReport};
use fap_solver::testing::{full_clues, img, random_image, thin};
use fap_solver::{Budget, Halt, Problem};

fn big() -> Budget { Budget::new(100_000_000) }

#[test]
fn all_zero_clues_resolve_at_level_one() {
    let r = grade(&Problem::whole(&full_clues(&img(&["...", "...", "..."]))), 4, &big()).unwrap();
    assert!(r.solved);
    assert_eq!((r.hardest, r.counts), (1, [0, 9, 0, 0, 0]));
}

#[test]
fn six_left_of_three_needs_level_two() {
    // solution "###." / "#..." / "##..": clue (1,1) = 6, clue (1,2) = 3, nothing else shown
    let mut c: Grid<Clue> = Grid::new(4, 3, Clue::Blank);
    c.set(Pos::new(1, 1), Clue::Digit(6));
    c.set(Pos::new(1, 2), Clue::Digit(3));
    let p = Problem::whole(&c);
    assert!(grade(&p, 1, &big()).unwrap().levels.is_empty());
    let r = grade(&p, 2, &big()).unwrap();
    assert!(!r.solved);
    for row in 0..3 {
        assert_eq!(r.levels.get(&Pos::new(row, 0)), Some(&2));
        assert_eq!(r.values.get(&Pos::new(row, 0)), Some(&true));
        assert_eq!(r.levels.get(&Pos::new(row, 3)), Some(&2));
        assert_eq!(r.values.get(&Pos::new(row, 3)), Some(&false));
    }
    assert_eq!(r.levels.len(), 6);
}

#[test]
fn givens_are_level_zero() {
    let mut c = full_clues(&img(&["#.."]));
    c.set(Pos::new(0, 0), Clue::GivenShaded);
    let r = grade(&Problem::whole(&c), 4, &big()).unwrap();
    assert!(r.solved);
    assert_eq!(r.levels[&Pos::new(0, 0)], 0);
    assert_eq!(r.counts[0], 1);
}

#[test]
fn sound_monotone_and_solved_implies_forced() {
    for seed in 0..40u64 {
        let image = random_image(6, 6, 0.5, seed);
        let p = Problem::whole(&thin(&full_clues(&image), 0.5, seed));
        let mut prev: Option<GradeReport> = None;
        for k in 1..=4 {
            let r = grade(&p, k, &big()).unwrap();
            for (pos, v) in &r.values { assert_eq!(*v, *image.get(*pos), "seed {seed} unsound at {pos:?}"); }
            if let Some(pr) = &prev {
                for (pos, l) in &pr.levels { assert_eq!(r.levels.get(pos), Some(l), "seed {seed} prefix broken"); }
            }
            if r.solved { assert!(matches!(check(&p, &big()).unwrap(), Verdict::Forced { .. })); }
            prev = Some(r);
        }
    }
}

fn fixture(name: &str) -> Problem {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    Problem::whole(&format::read_puzzle_file(std::path::Path::new(&path)).unwrap().clues)
}

#[test]
fn level_three_fixture() {
    let p = fixture("level3.puzzle.toml");
    let r = grade(&p, 4, &big()).unwrap();
    assert!(r.solved && r.hardest == 3);
    assert!(!grade(&p, 2, &big()).unwrap().solved);
}

#[test]
fn level_four_fixture() {
    let p = fixture("level4.puzzle.toml");
    let r = grade(&p, 4, &big()).unwrap();
    assert!(r.solved && r.hardest == 4);
    assert!(!grade(&p, 3, &big()).unwrap().solved);
}

#[test]
fn no_clues_terminates_unsolved() {
    let r = grade(&Problem::whole(&Grid::new(12, 12, Clue::Blank)), 4, &big()).unwrap();
    assert!(!r.solved && r.levels.is_empty());
}

#[test]
fn tiny_budget_halts() {
    let p = Problem::whole(&full_clues(&random_image(8, 8, 0.5, 3)));
    assert_eq!(grade(&p, 4, &Budget::new(2)), Err(Halt::BudgetExhausted));
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-solver --test graded`. Expected: compile errors (`graded` not found).

- [ ] **Step 3: Implement `graded.rs`** per Interfaces. Reuse `State`, and either clone it or undo via the trail for hypotheses.

- [ ] **Step 4: Write `examples/find_fixtures.rs` and generate the fixtures.**
  - For seeds 0.., take a `random_image` of 6×6 up to 10×10 and start fully clued.
  - Greedily remove digits in seeded random order, keeping a removal only if `grade(…, 4)` still solves.
  - Write the first puzzle with `hardest == 3` to `tests/fixtures/level3.puzzle.toml`, and the first with `hardest == 4` to `level4.puzzle.toml`. Build each as a `Puzzle` with `clues`, `solution: image`, `regions: Regions::new(..)` and empty tables, and save it with `format::write_puzzle_file`.
  - Print the seeds used.

Run: `nix develop -c cargo run --release -p fap-solver --example find_fixtures`
Expected: both files are written.

If no level-4 puzzle turns up within 20,000 attempts, stop and report it. That would mean L4 as defined is never needed at these sizes, which is a finding for Task 11. Don't weaken the test.

- [ ] **Step 5: Run to verify pass**: `nix develop -c cargo test -p fap-solver --test graded`. Expected: 8 passed.

- [ ] **Step 6: Commit**: `feat(solver): add graded human-style solver with level fixtures`

---

### Task 4: Settings and stats schemas, scoring, evaluation

**Files:**
- Create: `crates/fap-solver/src/settings.rs`, `src/stats.rs`, `src/score.rs`, `src/evaluate.rs`
- Test: `crates/fap-solver/tests/evaluate.rs`

**Interfaces:**
- Consumes: Tasks 1–3; `fap_core::{Regions, RegionId, RegionSettings, Puzzle, format}`.
- Produces:
  - **`GivensMode`:** `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum GivensMode { Freely, Disambiguate, Never }`. In TOML: `"freely"`, `"disambiguate"`, `"never"`.
  - **`GenerationSettings`** (the `[generation]` schema, spec §6.6): `#[derive(Debug, Clone, PartialEq)] pub struct GenerationSettings`. Fields, with their `Default` values and valid ranges:

    | Field | Type | Default | Valid |
    |---|---|---|---|
    | `seed` | `u64` | 0 | any |
    | `candidates` | `u32` | 4 | ≥ 1 |
    | `givens` | `GivensMode` | `Disambiguate` | — |
    | `given_density` | `f64` | 0.05 | 0..=1 |
    | `floor_share` | `f64` | 0.05 | 0..=1 |
    | `region_attempts` | `u32` | 3 | ≥ 1 |
    | `eval_budget` | `i64` | 2_000_000 | > 0 |
    | `exact_budget` | `i64` | 50_000_000 | > 0 |

    - `from_table(t: &toml::Table) -> Result<Self, String>`: missing keys take their defaults, and each error message names the key.
    - `to_table(&self) -> toml::Table`.
    - `seed` is written as the hex string `"0x%016x"`, because TOML integers are i64. Loading also accepts a non-negative integer.
  - **`RegionStats`:** `#[derive(Debug, Clone, PartialEq)] pub struct RegionStats { pub id: u32, pub target_level: u8, pub margin: u8, pub cells: u32, pub hardest_level: u8, pub cells_per_level: [u32; 5], pub score: f64, pub avg_unsolved: f64, pub ceiling_met: bool, pub floor_met: bool, pub warning: Option<String> }`.
  - **`UnassignedStats`:** `#[derive(Debug, Clone, PartialEq)] pub struct UnassignedStats { pub cells: u32, pub default_level: u8, pub hardest_level: u8, pub cells_per_level: [u32; 5], pub score: f64, pub ceiling_met: bool, pub floor_met: bool }`.
  - **`PuzzleStats`:** `#[derive(Debug, Clone, PartialEq)] pub struct PuzzleStats { pub score: f64, pub hardest_level: u8, pub cells_per_level: [u32; 5], pub digit_clues: u32, pub givens: u32, pub avg_unsolved: f64, pub floors_met: u32, pub floors_total: u32, pub regions: Vec<RegionStats>, pub unassigned: Option<UnassignedStats> }`.
    - It has `to_table` / `from_table`. In the file, top-level keys go in `[stats]`, each region in a `[[stats.region]]`, and the unassigned area in an optional `[stats.unassigned]`.
    - `cells_per_level` is a 5-integer array, indexed by level 0–4.
  - **Score:** `score::score(counts: &[u32; 5], targets: u32) -> f64` = `Σ_{l=1..4} w_l · counts[l] / targets`, with `w = [0, 1, 2, 4, 8]` (baseline; spec §6.3). It is 0 when `targets == 0`. Givens add nothing, so a puzzle that is easy only because of givens scores low.
  - **`RegionEval`:** `evaluate::RegionEval { pub id: RegionId, pub settings: RegionSettings, pub report: GradeReport, pub ceiling_met: bool, pub floor_met: bool, pub warning: Option<String> }`.
  - **`evaluate_region`:** `evaluate::evaluate_region(clues: &Grid<Clue>, regions: &Regions, id: RegionId, floor_share: f64, budget: &Budget) -> Result<RegionEval, Halt>`.
    - It grades `Problem::sub(clues, cells, margin)` at max level 4.
    - `ceiling_met = report.solved && report.hardest <= target`.
    - `floor_met = target < 2 || (ceiling_met && counts[target] >= max(1, ceil(floor_share × cells)))`.
    - `warning = Some("region too small to reach its floor")` when the floor is missed and the region has fewer than 25 cells.
  - **`UnassignedEval`:** `evaluate::UnassignedEval { pub cells: Vec<Pos>, pub report: GradeReport, pub ceiling_met: bool, pub floor_met: bool }`. `report` is restricted to the unassigned cells.
  - **`evaluate_unassigned`:** `evaluate::evaluate_unassigned(clues: &Grid<Clue>, regions: &Regions, floor_share: f64, budget: &Budget) -> Result<Option<UnassignedEval>, Halt>`.
    - It returns `None` when every cell has a region.
    - Otherwise it grades `Problem::whole` at level 4 and keeps only the unassigned cells. Ceiling and floor work as for regions, with `regions.default_level()` as the target.
  - **`Evaluation`:** `evaluate::Evaluation { pub stats: PuzzleStats, pub levels: Grid<Option<u8>> }`. `levels` holds each cell's resolving level: from its region's sub-puzzle, or from the whole-puzzle run for unassigned cells. It is the GUI's colour-by-level preview (spec §5.10).
  - **`evaluate_puzzle`:** `evaluate::evaluate_puzzle(clues: &Grid<Clue>, regions: &Regions, settings: &GenerationSettings, ctl: &Control) -> Result<Evaluation, Halt>`.
    - Each grade gets a fresh `Budget::with_cancel(settings.eval_budget, ctl.cancel_flag())`.
    - The overall counts sum regions and unassigned cells. `hardest` is the max, `score` is computed over all cells, and `avg_unsolved` is the cell-weighted mean.
    - `floors_total` counts the regions with target ≥ 2, plus the unassigned area if its default level is ≥ 2.
  - **Budget exhaustion, in all three evaluate functions:** a grade that returns `Halt::BudgetExhausted` is recorded as an empty, unsolved report: all counts are 0, so `ceiling_met = false`, and the cells' levels are `None`. Only `Halt::Cancelled` propagates as `Err`. This is what lets Review Focus 5 produce flagged stats instead of an error.

- [ ] **Step 1: Write the failing tests** in `crates/fap-solver/tests/evaluate.rs`

```rust
use fap_core::{Puzzle, RegionId, Regions};
use fap_solver::evaluate::{evaluate_puzzle, evaluate_region, evaluate_unassigned};
use fap_solver::score::score;
use fap_solver::testing::{full_clues, synthetic_image};
use fap_solver::{Budget, Control, GenerationSettings, GivensMode, PuzzleStats};

#[test]
fn score_weights_levels() {
    assert_eq!(score(&[0, 10, 0, 0, 0], 10), 1.0);
    assert_eq!(score(&[0, 0, 0, 10, 0], 10), 4.0);
    assert_eq!(score(&[10, 0, 0, 0, 0], 10), 0.0);
    assert!(score(&[0, 5, 0, 0, 5], 10) > score(&[0, 5, 0, 5, 0], 10));
    assert_eq!(score(&[0; 5], 0), 0.0);
}

#[test]
fn fully_clued_regions_meet_ceiling_but_not_hard_floors() {
    let image = synthetic_image(20, 10, 4);
    let regions = Regions::strips(20, 10, 10, 1, 3, 2, 2); // id 1 level 1, id 2 level 3
    let clues = full_clues(&image);
    let b = || Budget::new(50_000_000);
    let easy = evaluate_region(&clues, &regions, RegionId(1), 0.05, &b()).unwrap();
    assert!(easy.ceiling_met && easy.floor_met);
    let hard = evaluate_region(&clues, &regions, RegionId(2), 0.05, &b()).unwrap();
    assert!(hard.ceiling_met);
    assert!(!hard.floor_met, "a fully clued region should be too easy for a level-3 floor");
    assert!(evaluate_unassigned(&clues, &regions, 0.05, &b()).unwrap().is_none());
}

#[test]
fn evaluate_puzzle_aggregates_and_levels_cover_grid() {
    let image = synthetic_image(20, 10, 4);
    let regions = Regions::strips(20, 10, 10, 1, 3, 2, 2);
    let e = evaluate_puzzle(&full_clues(&image), &regions, &GenerationSettings::default(), &Control::new()).unwrap();
    assert_eq!(e.stats.regions.len(), 2);
    assert_eq!(e.stats.cells_per_level.iter().sum::<u32>(), 200);
    assert_eq!(e.stats.floors_total, 1);
    assert_eq!(e.stats.digit_clues, 200);
    assert!(e.levels.positions().all(|p| e.levels.get(p).is_some()));
}

#[test]
fn settings_defaults_and_round_trip() {
    let d = GenerationSettings::default();
    assert_eq!((d.candidates, d.givens, d.region_attempts), (4, GivensMode::Disambiguate, 3));
    assert_eq!(GenerationSettings::from_table(&toml::Table::new()).unwrap(), d);
    let s = GenerationSettings { seed: u64::MAX, givens: GivensMode::Freely, given_density: 0.2, ..d };
    let t = s.to_table();
    assert_eq!(t.get("seed").and_then(|v| v.as_str()), Some("0xffffffffffffffff"));
    assert_eq!(GenerationSettings::from_table(&t).unwrap(), s);
}

#[test]
fn settings_from_table_rejects_bad_values() {
    for (k, v) in [("givens", toml::Value::from("sometimes")), ("given_density", toml::Value::from(-0.1)),
                   ("candidates", toml::Value::from(0)), ("eval_budget", toml::Value::from(0))] {
        let mut t = toml::Table::new();
        t.insert(k.into(), v);
        let e = GenerationSettings::from_table(&t).unwrap_err();
        assert!(e.contains(k), "{e}");
    }
}

#[test]
fn stats_round_trip_through_puzzle_file() {
    let image = synthetic_image(20, 10, 4);
    let regions = Regions::strips(20, 10, 10, 1, 3, 2, 2);
    let e = evaluate_puzzle(&full_clues(&image), &regions, &GenerationSettings::default(), &Control::new()).unwrap();
    let mut p = Puzzle::fully_clued(&image, regions);
    p.stats = e.stats.to_table();
    let back = fap_core::format::load_puzzle(&fap_core::format::save_puzzle(&p)).unwrap();
    assert_eq!(PuzzleStats::from_table(&back.stats).unwrap(), e.stats);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-solver --test evaluate`. Expected: compile errors.

- [ ] **Step 3: Implement `settings.rs`, `stats.rs`, `score.rs`, `evaluate.rs`** per Interfaces. Use serde structs with `#[serde(default)]`, then validate explicitly. Convert with `toml::Table::try_from` / `.try_into()`.

- [ ] **Step 4: Run to verify pass**: same command. Expected: 6 passed.

- [ ] **Step 5: Commit**: `feat(solver): add generation settings, stats schema and evaluation`

---

### Task 5: Preflight and the three givens modes

**Files:**
- Create: `crates/fap-solver/src/preflight.rs`; create `crates/fap-solver/src/generate.rs` containing only `GenerateError` (the rest comes in Task 6)
- Test: `crates/fap-solver/tests/preflight.rs`

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces:
  - `#[derive(Debug, thiserror::Error)] pub enum GenerateError`, with these variants and display strings:

    | Variant | Display |
    |---|---|
    | `Ambiguous { unforced: Vec<Pos> }` | `"image is not uniquely solvable even with every clue shown ({n} cells can change)"` |
    | `RegionUnsolvable { regions: Vec<RegionId>, unforced: Vec<Pos> }` | `"regions {ids} cannot be solved on their own even fully clued"` |
    | `Halted(Halt)` | `"stopped: {halt:?}"` |
    | `Invalid(String)` | `"invalid input: {0}"` (e.g. the region map's size differs from the image's) |
    | `Internal(String)` | `"internal error: {0}"` |
  - `preflight::preflight(image: &Image, regions: &Regions, settings: &GenerationSettings, rng: &mut ChaCha8Rng, ctl: &Control) -> Result<Grid<Clue>, GenerateError>`. It returns the starting clue grid: every cell holds its digit, except where this function placed a given.
- Behaviour (spec §6.5). Every check below is `exact::check` under `Budget::with_cancel(settings.exact_budget, …)`.
  1. **Fully clued checks:** run `Problem::whole` and each region's `Problem::sub`, with every digit shown.
  2. **Never:**
     - If the whole check fails, return `Ambiguous` with its unforced cells.
     - Otherwise, if any region fails, return `RegionUnsolvable` with the failing region IDs (ascending) and the union of their unforced cells.
  3. **Disambiguate and Freely:** while a check fails, pick a cell uniformly from the current unforced set, using `rng`. The set is the whole-puzzle unforced cells if that check fails, otherwise the failing regions' unforced cells. Replace that cell's digit with the matching given and recheck. This terminates, since each iteration fixes one more cell.
  4. **Freely only, afterwards:**
     - For each region (in ascending ID order) and then the unassigned area, place up to `round(given_density × cells × (5 − level) / 4)` extra givens at random cells of the area.
     - `level` is the region's target, or `regions.default_level()` for the unassigned area.
     - After each placement, recheck the whole puzzle and every region whose margin cells include the changed cell. Revert the placement if any check fails.

- [ ] **Step 1: Write the failing tests** in `crates/fap-solver/tests/preflight.rs`

```rust
use fap_core::{Clue, Grid, Pos, Puzzle, RegionId, RegionSettings, Regions};
use fap_solver::exact::{check, Verdict};
use fap_solver::preflight::preflight;
use fap_solver::testing::{img, synthetic_image};
use fap_solver::{Budget, Control, GenerateError, GenerationSettings, GivensMode, Halt, Problem};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn settings(givens: GivensMode) -> GenerationSettings { GenerationSettings { givens, ..Default::default() } }
fn rng(s: u64) -> ChaCha8Rng { ChaCha8Rng::seed_from_u64(s) }
fn givens(c: &Grid<Clue>) -> Vec<Pos> {
    c.positions().filter(|p| matches!(c.get(*p), Clue::GivenShaded | Clue::GivenUnshaded)).collect()
}

#[test]
fn never_mode_reports_ambiguous_image() {
    let r = preflight(&img(&["#.", ".#"]), &Regions::new(2, 2, 2), &settings(GivensMode::Never), &mut rng(0), &Control::new());
    match r { Err(GenerateError::Ambiguous { unforced }) => assert_eq!(unforced.len(), 4), other => panic!("{other:?}") }
}

#[test]
fn disambiguate_adds_givens_until_unique() {
    let c = preflight(&img(&["#.", ".#"]), &Regions::new(2, 2, 2), &settings(GivensMode::Disambiguate), &mut rng(0), &Control::new()).unwrap();
    assert!(!givens(&c).is_empty());
    assert!(matches!(check(&Problem::whole(&c), &Budget::new(1_000_000)).unwrap(), Verdict::Forced { .. }));
}

#[test]
fn unsolvable_region_is_reported_or_fixed_inside_region() {
    let image = img(&["#.."]);
    let mut regions = Regions::new(3, 1, 2);
    let id = regions.create_region(RegionSettings { level: 1, margin: 0, name: None });
    regions.assign(Pos::new(0, 0), Some(id));
    match preflight(&image, &regions, &settings(GivensMode::Never), &mut rng(0), &Control::new()) {
        Err(GenerateError::RegionUnsolvable { regions: ids, .. }) => assert_eq!(ids, vec![id]),
        other => panic!("{other:?}"),
    }
    let c = preflight(&image, &regions, &settings(GivensMode::Disambiguate), &mut rng(0), &Control::new()).unwrap();
    assert_eq!(givens(&c), vec![Pos::new(0, 0)]);
}

#[test]
fn freely_places_fewer_givens_in_harder_regions() {
    let image = synthetic_image(10, 10, 2);
    let regions = Regions::strips(10, 10, 5, 1, 4, 2, 2); // id 1 level 1 (50 cells), id 2 level 4 (50 cells)
    let s = GenerationSettings { givens: GivensMode::Freely, given_density: 0.4, ..Default::default() };
    let c = preflight(&image, &regions, &s, &mut rng(9), &Control::new()).unwrap();
    let n = |id| givens(&c).into_iter().filter(|p| regions.region_at(*p) == Some(RegionId(id))).count();
    assert!(n(1) <= 20 && n(2) <= 5, "{} {}", n(1), n(2));
    assert!(n(1) > n(2));
    let mut p = Puzzle::fully_clued(&image, regions.clone());
    p.clues = c;
    p.validate().unwrap();
}

#[test]
fn cancelled_preflight_halts() {
    let ctl = Control::new();
    ctl.cancel();
    let r = preflight(&synthetic_image(10, 10, 1), &Regions::new(10, 10, 2), &settings(GivensMode::Never), &mut rng(0), &ctl);
    assert!(matches!(r, Err(GenerateError::Halted(Halt::Cancelled))));
}
```

(`Regions::create_region` and `Regions::assign` are Plan 0 APIs. A single-cell region is connected, so it doesn't need `normalize`.)

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-solver --test preflight`. Expected: compile errors.

- [ ] **Step 3: Implement `preflight.rs` and `GenerateError`.**

- [ ] **Step 4: Run to verify pass**: same command. Expected: 5 passed.

- [ ] **Step 5: Commit**: `feat(solver): add preflight checks and givens modes`

---

### Task 6: Generator — removal pass, single area, verification

Spec §6.6, development step (4): one region or no regions, at a single difficulty.

**Files:**
- Modify: `crates/fap-solver/src/generate.rs`
- Test: `crates/fap-solver/tests/generate.rs`

**Interfaces:**
- Consumes: Tasks 1–5.
- Produces:
  - `#[derive(Debug, Clone)] pub struct Candidate { pub index: u32, pub seed: u64, pub puzzle: Puzzle, pub stats: PuzzleStats, pub levels: Grid<Option<u8>> }`.
    - `puzzle.generation` = `GenerationSettings { seed: <candidate seed>, ..settings }.to_table()`.
    - `puzzle.stats` = `stats.to_table()`.
    - `puzzle.regions` is a clone of the input regions.
    - `puzzle.title = None`; callers set it.
  - `generate_one(image: &Image, regions: &Regions, settings: &GenerationSettings, seed: u64, ctl: &Control) -> Result<Candidate, GenerateError>`. It returns `index = 0`; Task 8 sets the real index.
  - `candidate_seed(base: u64, index: u32) -> u64`: the SplitMix64 output for state `base.wrapping_add(index as u64)`.
- Algorithm (baseline):
  1. **Setup.**
     - The region map's dimensions must equal the image's, else return `Invalid`.
     - `rng = ChaCha8Rng::seed_from_u64(seed)`.
     - `clues = preflight(...)`.
     - `locked` = all givens.
  2. **Removal pass** over an *area*, given a pool of candidate cells and an acceptance test. It makes `settings.region_attempts` attempts.
     - Each attempt restarts from the pre-pass clue grid. It shuffles the pool with `rng` and tries each digit in turn: blank it, and keep it blank if `accept(&clues)` passes, otherwise restore it.
     - Keep the attempt whose final state ranks best by `(floor_met, ceiling_met, score)`, compared lexicographically. Ties go to the earlier attempt.
     - Report `Progress` after each trial.
     - If any budget returns `Halt::Cancelled`, abort with `Halted(Cancelled)`. `BudgetExhausted` inside `accept` means "reject this removal", not an error.
  3. **Single-area cases (this task):**
     - **One region covering every cell.** The pool is all unlocked digits. `accept` = `grade(Problem::sub(clues, cells, margin), target, budget)` solves every target.
     - **No regions.** The pool is all unlocked digits. `accept` = `grade(Problem::whole(clues), 4, budget)` solves every cell, and every cell's level is ≤ `default_level`.
     - Each `accept` call gets a fresh `Budget::with_cancel(settings.eval_budget, …)`.
     - If the starting state already fails `accept`, the pass removes nothing. The unmet ceiling then shows up in the stats (Review Focus 5).
  4. **Verify** (`Phase::Verify`).
     - `exact::check(Problem::whole)` must be `Forced`, and so must every region's sub-problem. A failure returns `Internal("verification failed: …")`.
     - `Puzzle::validate()` must also pass.
     - Then `evaluate_puzzle` fills in the stats and levels.
  5. **Finish:** report `Phase::Done`.
- Invariant: graded "solved" implies exact "forced" (Task 3), so every accepted state is unique for its area. Verification is a safety net, not a filter.

- [ ] **Step 1: Write the failing tests** in `crates/fap-solver/tests/generate.rs`

```rust
use fap_core::{format, Clue, Grid, Regions};
use fap_solver::exact::{check, Verdict};
use fap_solver::testing::{img, synthetic_image};
use fap_solver::{candidate_seed, generate_one, Budget, Control, GenerationSettings, Problem, PuzzleStats};

fn digits(c: &Grid<Clue>) -> usize { c.positions().filter(|p| matches!(c.get(*p), Clue::Digit(_))).count() }
fn unique(c: &Grid<Clue>) -> bool {
    matches!(check(&Problem::whole(c), &Budget::new(100_000_000)).unwrap(), Verdict::Forced { .. })
}
fn one_region(w: usize, h: usize, level: u8) -> Regions { Regions::strips(w, h, w, level, level, 2, 2) }

#[test]
fn blank_image_generates() {
    let image = img(&["........"; 8]);
    let c = generate_one(&image, &Regions::new(8, 8, 1), &GenerationSettings::default(), 1, &Control::new()).unwrap();
    assert!(unique(&c.puzzle.clues));
    assert!(digits(&c.puzzle.clues) < 64);
    assert!(c.stats.hardest_level <= 1);
}

#[test]
fn full_region_respects_ceiling() {
    let c = generate_one(&synthetic_image(12, 12, 5), &one_region(12, 12, 2), &GenerationSettings::default(), 3, &Control::new()).unwrap();
    assert!(unique(&c.puzzle.clues));
    let r = &c.stats.regions[0];
    assert!(r.ceiling_met && r.hardest_level <= 2);
}

#[test]
fn same_seed_same_puzzle_different_seed_differs() {
    let image = synthetic_image(12, 12, 5);
    let s = GenerationSettings::default();
    let a = generate_one(&image, &one_region(12, 12, 2), &s, 3, &Control::new()).unwrap();
    let b = generate_one(&image, &one_region(12, 12, 2), &s, 3, &Control::new()).unwrap();
    let c = generate_one(&image, &one_region(12, 12, 2), &s, 4, &Control::new()).unwrap();
    assert_eq!(a.puzzle, b.puzzle);
    assert_ne!(a.puzzle.clues, c.puzzle.clues);
}

#[test]
fn easier_target_keeps_more_clues() {
    let image = synthetic_image(14, 14, 6);
    let s = GenerationSettings::default();
    let easy = generate_one(&image, &one_region(14, 14, 1), &s, 2, &Control::new()).unwrap();
    let hard = generate_one(&image, &one_region(14, 14, 3), &s, 2, &Control::new()).unwrap();
    assert!(digits(&easy.puzzle.clues) > digits(&hard.puzzle.clues));
}

#[test]
fn tiny_budget_still_terminates_with_valid_puzzle() {
    let s = GenerationSettings { eval_budget: 1, region_attempts: 1, ..Default::default() };
    let c = generate_one(&synthetic_image(10, 10, 7), &one_region(10, 10, 2), &s, 1, &Control::new()).unwrap();
    assert!(unique(&c.puzzle.clues));
    assert_eq!(digits(&c.puzzle.clues), 100);
    assert!(!c.stats.regions[0].ceiling_met);
}

#[test]
fn candidate_round_trips_through_puzzle_file() {
    let c = generate_one(&synthetic_image(10, 10, 8), &one_region(10, 10, 2), &GenerationSettings::default(), 5, &Control::new()).unwrap();
    let back = format::load_puzzle(&format::save_puzzle(&c.puzzle)).unwrap();
    assert_eq!(back, c.puzzle);
    assert_eq!(PuzzleStats::from_table(&back.stats).unwrap(), c.stats);
    assert_eq!(back.generation.get("seed").and_then(|v| v.as_str()), Some("0x0000000000000005"));
}

#[test]
fn candidate_seed_is_splitmix64() {
    assert_eq!(candidate_seed(0, 0), 0xE220_A839_7B1D_CDAF);
    let seeds: std::collections::BTreeSet<u64> = (0..8).map(|i| candidate_seed(42, i)).collect();
    assert_eq!(seeds.len(), 8);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-solver --test generate`. Expected: compile errors (`generate_one`, `candidate_seed` not found).

- [ ] **Step 3: Implement `generate.rs`** for the single-area cases. Structure the pass so Task 7 can add areas without rewriting it: one private `removal_pass(clues, locked, pool: Vec<Pos>, accept, rank, rng, ctl, progress)` that both tasks call.

- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test --release -p fap-solver --test generate`. Expected: 7 passed.

- [ ] **Step 5: Commit**: `feat(solver): add single-area puzzle generator with verification`

---

### Task 7: Generator — multiple regions, locks, unassigned cells last

Spec §6.6, development step (5). The same code serves both the strip preset and arbitrary regions.

**Files:**
- Modify: `crates/fap-solver/src/generate.rs`
- Test: `crates/fap-solver/tests/regions_gen.rs`

**Interfaces:**
- Consumes: Task 6's removal pass.
- Produces: `region_order(regions: &Regions) -> Vec<RegionId>`. Regions are sorted by target level ascending. Ties are broken by centroid column, then centroid row (as f64, where the centroid is the mean of the cell coordinates), then by ID.
- Algorithm (spec §6.6 "Region by region"). This extends `generate_one`.
  - **Region passes:** process each region R in `region_order` under `Phase::Region { id, index, of }`.
    - Pool: unlocked digits in `margin_cells(R, D)` that are not in an already finished region. This includes R's own cells, margin cells belonging to later regions, and unassigned margin cells.
    - `accept`: R's sub-problem, graded at R's target level.
    - After the pass, R is *finished*. Every digit still in R's margin outside R is **locked**.
    - Record R's `ceiling_met` at the end of its pass.
  - **Unassigned pass** (`Phase::Unassigned`, only if any cell has no region):
    - Pool: unlocked digits in unassigned cells.
    - `accept`: `grade(Problem::whole, 4)` solves every cell, and each unassigned cell's level is ≤ `default_level`.
  - **Lock check in Verify:** re-evaluate each region. If a region met its ceiling at the end of its own pass but no longer does, return `Internal("region {id} changed after it was generated")`. This catches lock bugs.
  - `Progress.fraction` counts over the total pool size of all passes.

- [ ] **Step 1: Write the failing tests** in `crates/fap-solver/tests/regions_gen.rs`

```rust
use fap_core::{Pos, RegionId, Regions};
use fap_solver::exact::{check, Verdict};
use fap_solver::testing::synthetic_image;
use fap_solver::{generate_one, region_order, Budget, Control, GenerationSettings, Problem};

fn forced(p: &Problem) -> bool { matches!(check(p, &Budget::new(200_000_000)).unwrap(), Verdict::Forced { .. }) }

#[test]
fn region_order_is_easiest_first_then_left_to_right() {
    let mut r = Regions::strips(30, 4, 10, 3, 3, 2, 2); // ids 1,2,3 all level 3
    r.settings_mut(RegionId(2)).unwrap().level = 1;
    r.settings_mut(RegionId(3)).unwrap().level = 2;
    assert_eq!(region_order(&r), vec![RegionId(2), RegionId(3), RegionId(1)]);
    assert_eq!(region_order(&Regions::strips(30, 4, 10, 2, 2, 2, 2)), vec![RegionId(1), RegionId(2), RegionId(3)]);
}

#[test]
fn strip_regions_each_independently_solvable_across_seeds() {
    let image = synthetic_image(40, 10, 11);
    let regions = Regions::strips(40, 10, 10, 1, 4, 2, 2);
    for seed in 0..4 {
        let c = generate_one(&image, &regions, &GenerationSettings::default(), seed, &Control::new())
            .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        assert!(forced(&Problem::whole(&c.puzzle.clues)));
        for (id, s) in regions.table() {
            assert!(forced(&Problem::sub(&c.puzzle.clues, &regions.cells(*id), s.margin)), "seed {seed} region {id:?}");
        }
        assert!(c.stats.regions.iter().all(|r| r.ceiling_met), "seed {seed}");
    }
}

#[test]
fn small_margins_work_via_disambiguation() {
    let image = synthetic_image(30, 8, 12);
    let regions = Regions::strips(30, 8, 10, 2, 2, 0, 2);
    let c = generate_one(&image, &regions, &GenerationSettings::default(), 1, &Control::new()).unwrap();
    for (id, _) in regions.table() {
        assert!(forced(&Problem::sub(&c.puzzle.clues, &regions.cells(*id), 0)));
    }
}

#[test]
fn unassigned_cells_generated_last_and_unique() {
    let image = synthetic_image(30, 10, 13);
    let mut regions = Regions::strips(30, 10, 10, 1, 2, 2, 2);
    for row in 0..10 { for col in 20..30 { regions.assign(Pos::new(row, col), None); } }
    regions.normalize();
    let c = generate_one(&image, &regions, &GenerationSettings::default(), 2, &Control::new()).unwrap();
    assert!(forced(&Problem::whole(&c.puzzle.clues)));
    let u = c.stats.unassigned.as_ref().expect("unassigned stats");
    assert_eq!(u.cells, 100);
    assert!(u.ceiling_met && u.hardest_level <= 2);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test --release -p fap-solver --test regions_gen`. Expected: compile error (`region_order` not found).

- [ ] **Step 3: Implement** per Interfaces.

- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test --release -p fap-solver --test regions_gen` (expect 4 passed), then `nix develop -c cargo test --workspace`.

- [ ] **Step 5: Commit**: `feat(solver): generate region by region with locked margin clues`

---

### Task 8: Parallel candidates, progress, cancellation

**Files:**
- Create: `crates/fap-solver/src/parallel.rs`
- Test: `crates/fap-solver/tests/parallel.rs`

**Interfaces:**
- Consumes: `generate_one`, `candidate_seed`, `Control`, `Progress`.
- Produces: `generate(image: &Image, regions: &Regions, settings: &GenerationSettings, ctl: &Control) -> Vec<Result<Candidate, GenerateError>>`.
  - It returns `settings.candidates` entries in index order. Candidate i uses `candidate_seed(settings.seed, i)` and has `index = i`.
  - It runs `min(candidates, std::thread::available_parallelism())` worker threads inside a `std::thread::scope`. Workers pull indices from an `AtomicU32`, and each uses its own `Control` clone. Progress messages carry `Progress.candidate = i`.
  - This is the call the GUI makes from a background thread (Plan 4). The GUI keeps the `Control`, calls `cancel()` from the UI thread, and drains the `Receiver<Progress>` every frame.

- [ ] **Step 1: Write the failing tests** in `crates/fap-solver/tests/parallel.rs`

```rust
use fap_core::Regions;
use fap_solver::testing::synthetic_image;
use fap_solver::{candidate_seed, generate, generate_one, Control, GenerateError, GenerationSettings, Halt, Phase};
use std::time::{Duration, Instant};

#[test]
fn parallel_matches_sequential() {
    let image = synthetic_image(16, 12, 21);
    let regions = Regions::strips(16, 12, 8, 1, 2, 2, 2);
    let s = GenerationSettings { candidates: 3, seed: 77, ..Default::default() };
    let par = generate(&image, &regions, &s, &Control::new());
    assert_eq!(par.len(), 3);
    for (i, r) in par.into_iter().enumerate() {
        let c = r.unwrap();
        let seq = generate_one(&image, &regions, &s, candidate_seed(77, i as u32), &Control::new()).unwrap();
        assert_eq!((c.index, c.seed), (i as u32, candidate_seed(77, i as u32)));
        assert_eq!(c.puzzle.clues, seq.puzzle.clues);
    }
}

#[test]
fn progress_reports_phases() {
    let (tx, rx) = std::sync::mpsc::channel();
    let s = GenerationSettings { candidates: 1, ..Default::default() };
    let _ = generate(&synthetic_image(10, 10, 22), &Regions::strips(10, 10, 5, 1, 1, 2, 2), &s, &Control::with_progress(tx));
    let msgs: Vec<_> = rx.try_iter().collect();
    assert_eq!(msgs.first().unwrap().phase, Phase::Preflight);
    assert_eq!(msgs.last().unwrap().phase, Phase::Done);
    assert!(msgs.iter().any(|m| matches!(m.phase, Phase::Region { .. })));
    assert!(msgs.windows(2).all(|w| w[0].fraction <= w[1].fraction));
}

#[test]
fn cancel_before_start_halts_all() {
    let ctl = Control::new();
    ctl.cancel();
    let s = GenerationSettings { candidates: 2, ..Default::default() };
    let r = generate(&synthetic_image(10, 10, 23), &Regions::new(10, 10, 2), &s, &ctl);
    assert!(r.iter().all(|x| matches!(x, Err(GenerateError::Halted(Halt::Cancelled)))));
}

#[test]
fn cancel_mid_run_returns_promptly() {
    let ctl = Control::new();
    let c2 = ctl.clone();
    let image = synthetic_image(120, 30, 24);
    let regions = Regions::strips(120, 30, 20, 1, 4, 2, 2);
    let s = GenerationSettings { candidates: 2, ..Default::default() };
    let h = std::thread::spawn(move || generate(&image, &regions, &s, &c2));
    std::thread::sleep(Duration::from_millis(300));
    let t = Instant::now();
    ctl.cancel();
    let r = h.join().unwrap();
    assert!(t.elapsed() < Duration::from_secs(1), "took {:?}", t.elapsed());
    assert!(r.iter().any(|x| matches!(x, Err(GenerateError::Halted(Halt::Cancelled)))));
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test --release -p fap-solver --test parallel`. Expected: compile error (`generate` not found).

- [ ] **Step 3: Implement `parallel.rs`.** If `cancel_mid_run_returns_promptly` fails, the cause is a loop that never spends budget. Make every loop in preflight, the passes and verify call `budget.spend` or `ctl.is_cancelled()` at least once per trial.

- [ ] **Step 4: Run to verify pass**: same command. Expected: 4 passed.

- [ ] **Step 5: Commit**: `feat(solver): run candidates in parallel with progress and cancellation`

---

### Task 9: CLI `check`, `grade`, `generate`

**Files:**
- Modify: `crates/fap-cli/Cargo.toml` (add `fap-solver` as a path dependency), `crates/fap-cli/src/main.rs` (three new `Command` variants plus dispatch)
- Create: `crates/fap-cli/src/solve_commands.rs`
- Test: `crates/fap-cli/tests/solve_cli.rs`

**Interfaces:**
- Consumes: Plan 0's CLI structure (`Command` enum; errors print `error: {message}` and exit 1); fap-solver's public API.
- Produces:
  - **`fap check IN`.**
    - Input: a puzzle (checks its clues) or a project (checks the fully clued version, i.e. what `--givens never` would test).
    - Output lines:
      ```
      unique: yes | unique: no ({n} unforced cells) | unique: no solution
      region {id}: independently solvable with margin {d} | region {id}: NOT independently solvable with margin {d} ({k} unforced cells)
      ```
    - After any line that reports unforced cells, print `  unforced: ` followed by the first 20 `Pos::describe()` strings, joined by `; `.
    - Exit codes: 0 if everything passes, 2 if any check fails, 1 on errors.
  - **`fap grade IN.puzzle.toml [--whole]`.**
    - Output comes from `evaluate_puzzle`, using the puzzle's `[generation]` settings or the defaults:
      ```
      score {score:.2}, hardest level {h}
      cells per level: given {n0}, L1 {n1}, L2 {n2}, L3 {n3}, L4 {n4}
      region {id} (target {t}, margin {d}): hardest {h}, score {s:.2}, ceiling met|ceiling NOT met, floor met|floor NOT met
      unassigned (default level {d}): hardest {h}, score {s:.2}, ...
      ```
    - `--whole` adds `whole-puzzle flow: hardest {h}, cells per level: ...`, from `grade(Problem::whole, 4)`. This is the comparison measure from spec §6.3.
  - **`fap generate IN.project.toml -o OUT.puzzle.toml [--seed S] [--candidates N] [--givens freely|disambiguate|never] [--given-density F] [--floor-share F]`.**
    - Settings start from the project's `[generation]` table (via `from_table`), and flags override them. The project file is never modified.
    - Progress goes to stderr, one line per phase change.
    - One line per candidate: `candidate {i+1} (seed 0x{seed:016x}): score {s:.2}, hardest {h}, digits {d}, givens {g}, floors {met}/{total}`, or `candidate {i+1}: failed: {error}`.
    - It writes the best candidate: most floors met, then highest score, then lowest index. The puzzle title is the project's title. It then prints `wrote {OUT} (candidate {i+1})`.
    - If every candidate fails, it exits 1 with the first error. For `Ambiguous` and `RegionUnsolvable` errors, it also prints the first 20 unforced cells.

- [ ] **Step 1: Write the failing tests** in `crates/fap-cli/tests/solve_cli.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::path::{Path, PathBuf};

fn fap() -> Command { Command::cargo_bin("fap").unwrap() }
fn project_from_p1(dir: &Path, name: &str, p1: &str) -> PathBuf {
    let (pbm, proj) = (dir.join(format!("{name}.pbm")), dir.join(format!("{name}.project.toml")));
    std::fs::write(&pbm, p1).unwrap();
    fap().arg("import-pbm").arg(&pbm).arg("-o").arg(&proj).assert().success();
    proj
}
const SIX: &str = "P1\n6 6\n110000\n100011\n001001\n011100\n000010\n100111\n";

#[test]
fn check_reports_unique_puzzle() {
    let d = tempfile::tempdir().unwrap();
    let proj = project_from_p1(d.path(), "a", "P1\n3 1\n100\n");
    let puz = d.path().join("a.puzzle.toml");
    fap().arg("clue-all").arg(&proj).arg("-o").arg(&puz).assert().success();
    fap().arg("check").arg(&puz).assert().code(0).stdout(contains("unique: yes"));
}

#[test]
fn check_project_reports_ambiguous_image() {
    let d = tempfile::tempdir().unwrap();
    let proj = project_from_p1(d.path(), "b", "P1\n2 2\n10\n01\n");
    fap().arg("check").arg(&proj).assert().code(2)
        .stdout(contains("unique: no (4 unforced cells)")).stdout(contains("row 1, column 1"));
}

#[test]
fn generate_writes_checkable_puzzle_and_grade_prints_levels() {
    let d = tempfile::tempdir().unwrap();
    let proj = project_from_p1(d.path(), "c", SIX);
    let out = d.path().join("c.puzzle.toml");
    fap().arg("generate").arg(&proj).arg("-o").arg(&out).args(["--candidates", "2", "--seed", "7"])
        .assert().success().stdout(contains("candidate 1")).stdout(contains("candidate 2")).stdout(contains("wrote"));
    fap().arg("check").arg(&out).assert().code(0).stdout(contains("unique: yes"));
    fap().arg("grade").arg(&out).arg("--whole").assert().success()
        .stdout(contains("cells per level")).stdout(contains("whole-puzzle flow"));
}

#[test]
fn generate_never_mode_fails_on_ambiguous_image() {
    let d = tempfile::tempdir().unwrap();
    let proj = project_from_p1(d.path(), "e", "P1\n2 2\n10\n01\n");
    fap().arg("generate").arg(&proj).arg("-o").arg(d.path().join("e.puzzle.toml")).args(["--givens", "never"])
        .assert().code(1).stderr(contains("not uniquely solvable"));
}

#[test]
fn generate_uses_project_generation_settings() {
    let d = tempfile::tempdir().unwrap();
    let proj = project_from_p1(d.path(), "f", SIX);
    let mut text = std::fs::read_to_string(&proj).unwrap();
    text.push_str("\n[generation]\ncandidates = 1\n");
    std::fs::write(&proj, text).unwrap();
    fap().arg("generate").arg(&proj).arg("-o").arg(d.path().join("f.puzzle.toml"))
        .assert().success().stdout(contains("candidate 1")).stdout(contains("candidate 2").not());
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-cli --test solve_cli`. Expected: failures (unknown subcommands).

- [ ] **Step 3: Implement `solve_commands.rs` and wire up the `Command` variants in `main.rs`.** Limit the `main.rs` diff to the variant definitions and dispatch arms, to keep merge conflicts with other tracks small.

- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test --workspace`. Expected: all pass, including 5 new CLI tests.

- [ ] **Step 5: Commit**: `feat(cli): add check, grade and generate commands`

---

### Task 10: Performance check at full size (30×300)

**Files:**
- Create: `crates/fap-solver/tests/perf.rs`, `docs/notes/2026-10-29-solver-experiments.md` (starting with a "Performance" section)
- Modify: solver sources as needed for optimisation

**Target (spec §6.6, "a few minutes"):** one candidate on `synthetic_image(300, 30, 1)` (300 columns × 30 rows), with regions `Regions::strips(300, 30, 20, 1, 4, 2, 2)` (15 strips ramping 1→4) and default settings, finishes in **≤ 180 s**. Release build, on the development Mac.

- [ ] **Step 1: Write two ignored perf tests.**
  - `perf_30x300_strips`: runs that case through `generate_one` and asserts `elapsed <= 180 s`. It prints elapsed time, `stats.score`, digit count and floors met.
  - `perf_30x300_no_regions`: the same image with `Regions::new(300, 30, 2)`, the plain "difficulty slider" case. A timer thread calls `ctl.cancel()` after 600 s, and the test prints either the elapsed time or `did not finish in 10 min`. It is informational, with no assert.

- [ ] **Step 2: Run**

Run: `nix develop -c cargo test --release -p fap-solver --test perf -- --ignored --nocapture`
Record both timings in the notes file.

- [ ] **Step 3: If over target, optimise and re-run until it passes.**
  - Measure first: add per-level and per-phase step counters, printed by the perf test.
  - Candidate optimisations, in rough order of expected payoff:
    - (a) skip L3/L4 probes on variables whose constraints haven't changed since their last failed probe;
    - (b) in `accept`, stop grading as soon as a target can't be resolved at the ceiling level;
    - (c) incremental worklists for the L1/L2 sweeps instead of full sweeps;
    - (d) for large regions, warm-start from the previous accepted grade for the unchanged part.
  - Behaviour must stay identical; the tests from Tasks 1–9 are the oracle.
  - If the target can't be met with reasonable effort, stop. Record the best timing and profile in the notes and report it, rather than loosening the test.

- [ ] **Step 4: Commit**: `perf(solver): meet full-size generation target` (or `docs(solver): record performance findings` if the target was not met)

---

### Task 11: Experiment — techniques and heuristics (exploratory)

This task is **exploratory**. Its deliverable is a written comparison, plus changed defaults where an option clearly wins. The interfaces and tests from Tasks 1–10 stay unchanged: if an experiment points to an interface change, write it up instead of making it.

**Files:**
- Modify: `docs/notes/2026-10-29-solver-experiments.md`
- Create: `crates/fap-solver/examples/experiments.rs`
- Modify (only if a variant wins): defaults in `settings.rs`, probe order in `graded.rs`, pool ordering in `generate.rs`

- [ ] **Step 1: Write `examples/experiments.rs`.**
  - **Matrix:** `synthetic_image(120, 30, s)` for s in 0..5, with strips of width 20, a 1→4 ramp and margin 2.
  - **Output:** a markdown table with, for each variant: mean digits per cell, floors met / total, mean score per target level, and mean wall time.
  - **Variants:**
    1. L3/L4 probe order: index order (baseline) vs "frontier first", i.e. variables adjacent to the most recently resolved cells, as a model of where a human looks (spec §6.2).
    2. Removal-pool order: uniform shuffle (baseline) vs "extreme digits last" (0, 9 and edge-saturated clues) vs "extreme digits first".
    3. `region_attempts` of 1, 3 and 5: floor hit rate against time cost.
    4. Per-region measure (spec §6.3): sub-puzzle levels (baseline) vs whole-puzzle flow levels. Record how often they disagree on a region's hardest level.
    5. `floor_share` of 0.02, 0.05 and 0.10: floor hit rate.
- [ ] **Step 2: Run and record**

Run: `nix develop -c cargo run --release -p fap-solver --example experiments >> docs/notes/2026-10-29-solver-experiments.md`
Then add a short "Conclusions" paragraph per variant: what was observed, and whether the default changed.

- [ ] **Step 3: Apply only clear winners**, meaning a better floor hit rate at no more than 1.5× the time. Then re-run `nix develop -c cargo test --workspace` and the perf test.

- [ ] **Step 4: Commit**: `docs(solver): record technique and heuristic experiments`. If defaults changed, commit those separately as `feat(solver): …`.

---

## Done when

- `nix develop -c cargo test --workspace`, clippy and fmt are clean. Run the generator test files in release.
- The perf target is met, or the shortfall is documented in the notes.
- `fap generate` on a 30×300 strip project produces a puzzle where:
  - `fap check` reports it as unique, with every region independently solvable;
  - `fap grade` shows per-region levels.
- `generate`, `Control` and `Progress` are ready for the GUI's generate dialog in Plan 4, and `Candidate.levels` feeds its colour-by-level preview.
