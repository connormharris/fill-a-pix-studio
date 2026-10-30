# Plan 0 — Foundation (core library, file formats, CLI skeleton) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Cargo workspace with `fap-core` (grid, clue, region, puzzle and project types; TOML project/puzzle formats; PBM I/O) and a `fap` CLI. Together they fix the shared file formats so Tracks 1–3 can proceed in parallel worktrees.

**Architecture:** `fap-core` is a pure library with no GUI, solver or PDF dependencies. Everything other crates share lives here: types, validation, region normalisation (split rule), the strip preset, and serialisation. Generation settings, stats and PDF settings are stored as opaque `toml::Table`s, so Tracks 2 and 3 own their own schemas without editing core. `fap-cli` is a thin clap front end. It covers file utilities plus a trivial fully-clued "generator", so Track 3 has puzzle files to render from day one.

**Tech Stack:** Rust (edition 2024, rustc 1.99 via a Nix flake dev shell), `serde`, `toml`, `thiserror`, `clap` (derive), with `assert_cmd`, `predicates`, `tempfile` for CLI tests.

**Spec:** `docs/superpowers/specs/2026-10-29-fill-a-pix-design.md` (§3, §4, §5.6 split rule and strip preset, §8 Stage 0, §9). Read it before starting.

**Followed by:** Plan 1 (front end), Plan 2 (solver/generator), Plan 3 (PDF), run in parallel worktrees after this plan is merged, then Plan 4 (integration).

## Global Constraints

- Every shell command runs inside the dev shell: prefix with `nix develop -c` (e.g. `nix develop -c cargo test`). The user's global `cargo` has no `rustc`.
- Rust edition `2024`; no `unsafe`.
- Every crate's `Cargo.toml` has `license = "GPL-3.0-or-later"`. Dependencies must be GPL-3.0-compatible (MIT, Apache-2.0, BSD, MPL-2.0, LGPL, GPL are all fine).
- Add dependencies with `cargo add` (it picks current versions); never hand-pin versions from memory.
- Commit messages: conventional commits `type(scope): summary` (e.g. `feat(core): ...`), ending with the two attribution lines:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01PDLsJXHj2PzKi7fqi39L9q`.
- Before every commit, all three pass: `nix develop -c cargo fmt --all --check`, `nix develop -c cargo clippy --workspace --all-targets -- -D warnings`, `nix develop -c cargo test --workspace`.
- Coordinates: `Pos { row, col }`, zero-based, row 0 at top; all iteration is row-major. Human-facing messages use one-based `row R, column C`.
- Format identifiers, verbatim: project `format = "fill-a-pix-project"`, puzzle `format = "fill-a-pix-puzzle"`, `version = 1`.
- Grid characters, verbatim: image `#` shaded, `.` unshaded; clues `0`–`9` digit, `.` no clue, `#` given shaded, `x` given unshaded; regions: whitespace-separated tokens, a positive integer region ID or `.` for no region.
- Region limits: level 1–4, margin D 0–2. Defaults: project default level 2; new region settings = level `default_level`, margin 2, no name.
- File names (builder's call, fixed here): projects `*.project.toml`, puzzles `*.puzzle.toml`, images `*.pbm`. Loaders decide by the `format` field, never by the extension.

## Review Focus

1. **Hand-edited or Windows-edited files:** CRLF line endings, indented rows, trailing spaces and missing final newline in grid strings must load exactly like clean files. Test: `crlf_and_indentation_load_identically` (Task 4).
2. **PBM edge cases from other tools:** P4 widths not divisible by 8 (padding bits ignored), P1 with `#` comments and with digits not separated by whitespace. Tests: `p4_padding_bits_ignored`, `p1_comments_and_unseparated_digits` (Task 7).
3. **Hand-written region maps:** a region split into pieces, or a `[[region]]` entry with no cells, is normalised on load rather than rejected. A token naming an undefined ID is rejected with the ID and row in the message. Tests: `disconnected_region_in_file_is_split`, `undefined_region_id_names_row_and_id` (Task 5).
4. **Dimension mismatches:** a `width`/`height` that disagrees with the rows gives an error naming the section and one-based row, not a panic. Test: `row_length_mismatch_names_section_and_row` (Task 5).
5. **Wrong kind or newer file:** loading a puzzle as a project (or vice versa), or a `version = 2` file, says exactly what was found. Tests: `puzzle_file_loaded_as_project_is_rejected_by_kind`, `newer_version_is_rejected` (Task 5).

---

## File Structure

```
Cargo.toml                         workspace (members = ["crates/*"], shared [workspace.package])
flake.nix, flake.lock              dev shell: rustc cargo clippy rustfmt rust-analyzer
LICENSE                            GPL-3.0 text
README.md                          what it is, `nix develop`, crate map
.gitignore                         /target, /result
crates/fap-core/
  Cargo.toml
  src/lib.rs                       module declarations + re-exports
  src/error.rs                     CoreError, Result
  src/grid.rs                      Pos, Grid<T>, Image
  src/clues.rs                     neighborhood, clue_at, all_clues
  src/puzzle.rs                    Clue, Puzzle (+ validate, fully_clued)
  src/regions.rs                   RegionId, RegionSettings, Regions (+ normalize, strips)
  src/project.rs                   Project
  src/format/mod.rs                FileKind, detect_kind, load/save + file helpers
  src/format/gridtext.rs           row-string codecs for image, clue and region grids
  src/format/project_file.rs       project TOML schema
  src/format/puzzle_file.rs        puzzle TOML schema
  src/pbm.rs                       PBM read (P1, P4) / write (P4)
  tests/grid.rs, tests/clues.rs, tests/regions.rs, tests/gridtext.rs,
  tests/project_file.rs, tests/puzzle_file.rs, tests/pbm.rs
crates/fap-cli/
  Cargo.toml                       [[bin]] name = "fap"
  src/main.rs                      clap definitions + dispatch
  src/commands.rs                  one fn per subcommand
  tests/cli.rs
```

`fap-core` public API is re-exported at the crate root: `fap_core::{Pos, Grid, Image, CoreError, Result, Clue, Puzzle, RegionId, RegionSettings, Regions, Project}` plus modules `fap_core::{clues, format, pbm}`. `format::gridtext` is public too, so later tracks can reuse it.

---

### Task 1: Workspace, dev shell, grid types

**Files:**
- Create: `Cargo.toml`, `flake.nix`, `LICENSE`, `README.md`, `.gitignore`, `crates/fap-core/Cargo.toml`, `crates/fap-core/src/{lib.rs,error.rs,grid.rs}`
- Test: `crates/fap-core/tests/grid.rs`

**Interfaces:**
- Produces:
  - `pub struct Pos { pub row: usize, pub col: usize }` (derive `Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord`; ordering is row-major), `Pos::new(row, col) -> Pos`, `Pos::describe(&self) -> String` returning `"row {row+1}, column {col+1}"`.
  - `pub struct Grid<T>` (row-major `Vec<T>`; derive `Clone, Debug, PartialEq, Eq`):
    - `Grid::new(width: usize, height: usize, fill: T) -> Grid<T>` where `T: Clone`; panics if either dimension is 0.
    - `Grid::from_rows(rows: Vec<Vec<T>>) -> Result<Grid<T>>`; `CoreError::Format` if empty, any row empty, or ragged.
    - `width(&self) -> usize`, `height(&self) -> usize`, `get(&self, p: Pos) -> &T`, `get_mut(&mut self, p: Pos) -> &mut T`, `set(&mut self, p: Pos, v: T)` (panic out of bounds).
    - `pos_at(&self, row: isize, col: isize) -> Option<Pos>` (bounds-checked conversion).
    - `positions(&self) -> impl Iterator<Item = Pos>` (row-major), `row(&self, r: usize) -> &[T]`, `map<U>(&self, f: impl FnMut(&T) -> U) -> Grid<U>`.
  - `pub type Image = Grid<bool>;` (`true` = shaded).
  - `pub enum CoreError { Io(#[from] std::io::Error), Toml(String), Format(String), Unsupported(String), Pbm(String) }` via `thiserror`, with display strings `"I/O error: {0}"`, `"TOML syntax error: {0}"`, `"{0}"`, `"unsupported file: {0}"`, `"PBM error: {0}"`. `pub type Result<T> = std::result::Result<T, CoreError>;`

- [ ] **Step 1: Create the dev shell and workspace skeleton**

`flake.nix` (exact content):

```nix
{
  description = "Fill-a-Pix puzzle studio";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ rustc cargo clippy rustfmt rust-analyzer ];
        };
      });
}
```

Root `Cargo.toml`: `[workspace] resolver = "3"`, `members = ["crates/*"]`; `[workspace.package] edition = "2024"`, `license = "GPL-3.0-or-later"`, `version = "0.1.0"`. Crates use `edition.workspace = true` etc. Create `crates/fap-core` as a library (`cargo new --lib crates/fap-core` inside the dev shell is fine). Add `thiserror` to it with `cargo add -p fap-core thiserror`. `.gitignore`: `/target` and `/result`. `LICENSE`: `curl -fsSL https://www.gnu.org/licenses/gpl-3.0.txt -o LICENSE`. `README.md`: three short paragraphs: what the project is (one line from the spec's §1), how to work on it (`nix develop`, then `cargo test` / `cargo run -p fap-cli -- --help`), and the crate map from spec §3.

Run: `git add flake.nix && nix develop -c rustc --version`
Expected: `rustc 1.99.0` (or newer); `flake.lock` is created.

- [ ] **Step 2: Write the failing tests** in `crates/fap-core/tests/grid.rs`

```rust
use fap_core::{CoreError, Grid, Pos};

#[test]
fn new_grid_has_dimensions_and_fill() {
    let g = Grid::new(3, 2, false);
    assert_eq!((g.width(), g.height()), (3, 2));
    assert_eq!(g.positions().count(), 6);
    assert!(g.positions().all(|p| !*g.get(p)));
}

#[test]
fn positions_are_row_major() {
    let g = Grid::new(2, 2, 0u8);
    let v: Vec<Pos> = g.positions().collect();
    assert_eq!(v, vec![Pos::new(0, 0), Pos::new(0, 1), Pos::new(1, 0), Pos::new(1, 1)]);
}

#[test]
fn set_get_and_row() {
    let mut g = Grid::new(3, 2, 0u8);
    g.set(Pos::new(1, 2), 7);
    *g.get_mut(Pos::new(0, 0)) = 1;
    assert_eq!(*g.get(Pos::new(1, 2)), 7);
    assert_eq!(g.row(0), &[1, 0, 0]);
    assert_eq!(g.row(1), &[0, 0, 7]);
}

#[test]
fn from_rows_builds_and_rejects_bad_shapes() {
    let g = Grid::from_rows(vec![vec![1, 2, 3], vec![4, 5, 6]]).unwrap();
    assert_eq!((g.width(), g.height()), (3, 2));
    assert_eq!(*g.get(Pos::new(1, 0)), 4);
    assert!(matches!(Grid::from_rows(vec![vec![1, 2], vec![3]]), Err(CoreError::Format(_))));
    assert!(matches!(Grid::<u8>::from_rows(vec![]), Err(CoreError::Format(_))));
    assert!(matches!(Grid::<u8>::from_rows(vec![vec![]]), Err(CoreError::Format(_))));
}

#[test]
fn pos_at_checks_bounds() {
    let g = Grid::new(3, 2, ());
    assert_eq!(g.pos_at(1, 2), Some(Pos::new(1, 2)));
    assert_eq!(g.pos_at(-1, 0), None);
    assert_eq!(g.pos_at(0, -1), None);
    assert_eq!(g.pos_at(2, 0), None);
    assert_eq!(g.pos_at(0, 3), None);
}

#[test]
fn map_preserves_shape() {
    let g = Grid::new(2, 3, 5u8).map(|v| *v as u32 * 2);
    assert_eq!((g.width(), g.height()), (2, 3));
    assert_eq!(*g.get(Pos::new(2, 1)), 10u32);
}

#[test]
fn pos_describe_is_one_based() {
    assert_eq!(Pos::new(0, 4).describe(), "row 1, column 5");
}

#[test]
#[should_panic]
fn zero_size_panics() {
    let _ = Grid::new(0, 3, false);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `nix develop -c cargo test -p fap-core --test grid`
Expected: compile errors (`Grid`, `Pos`, `CoreError` not found).

- [ ] **Step 4: Implement `error.rs`, `grid.rs`, and the re-exports in `lib.rs`** per the Interfaces block.

- [ ] **Step 5: Run tests to verify they pass**

Run: `nix develop -c cargo test -p fap-core --test grid`
Expected: 8 passed.

- [ ] **Step 6: Commit** (after the three pre-commit checks)

```bash
git add Cargo.toml Cargo.lock flake.nix flake.lock LICENSE README.md .gitignore crates/fap-core
git commit  # message: "feat(core): add workspace, dev shell and grid types"
```

---

### Task 2: Regions model, split rule, strip preset

**Files:**
- Create: `crates/fap-core/src/regions.rs`
- Modify: `crates/fap-core/src/lib.rs`
- Test: `crates/fap-core/tests/regions.rs`

**Interfaces:**
- Consumes: `Pos`, `Grid`, `CoreError` (Task 1).
- Produces:
  - `pub struct RegionId(pub u32)` (derive `Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord`).
  - Constants `regions::{MIN_LEVEL = 1, MAX_LEVEL = 4, MAX_MARGIN = 2, DEFAULT_MARGIN = 2, DEFAULT_LEVEL = 2}` (all `u8`).
  - `pub struct RegionSettings { pub level: u8, pub margin: u8, pub name: Option<String> }` (derive `Clone, Debug, PartialEq, Eq`).
  - `pub struct Regions` (private fields: `map: Grid<Option<RegionId>>`, `table: BTreeMap<RegionId, RegionSettings>`, `default_level: u8`, `next_id: u32`; derive `Clone, Debug, PartialEq, Eq`):
    - `Regions::new(width, height, default_level: u8) -> Regions`: no regions; `next_id = 1`.
    - `Regions::from_parts(map, table, default_level) -> Result<Regions>`: validates every map ID is in the table (error message contains `"region {id}"` and the cell's `describe()`), that levels (including `default_level`) are 1–4 and margins 0–2, then calls `normalize()`. `next_id = max id + 1`.
    - Accessors: `map(&self) -> &Grid<Option<RegionId>>`, `table(&self) -> &BTreeMap<RegionId, RegionSettings>`, `default_level(&self) -> u8`, `set_default_level(&mut self, l: u8)`, `region_at(&self, p: Pos) -> Option<RegionId>`, `settings(&self, id) -> Option<&RegionSettings>`, `settings_mut(&mut self, id) -> Option<&mut RegionSettings>`, `cells(&self, id) -> Vec<Pos>` (row-major), `width()`, `height()`.
    - `default_settings(&self) -> RegionSettings` = `{ level: default_level, margin: DEFAULT_MARGIN, name: None }`.
    - `create_region(&mut self, s: RegionSettings) -> RegionId`: allocates `next_id`, adds the table entry, assigns no cells.
    - `assign(&mut self, p: Pos, id: Option<RegionId>)`: raw write; panics if `id` is not in the table. Callers run `normalize()` after a batch.
    - `normalize(&mut self)`: **split rule** (spec §5.6). For each region, find its 4-connected (edge) components. If there is more than one, the original ID is removed and each piece gets a fresh ID (allocated in row-major order of each piece's first cell) with a clone of the settings. Table entries with no cells are removed. A connected region keeps its ID.
    - `Regions::strips(width, height, strip_width: usize, start_level: u8, end_level: u8, margin: u8, default_level: u8) -> Regions`. `k = max(1, width / strip_width)` strips, IDs `1..=k` left to right. Strip `i` covers columns `[i*strip_width, (i+1)*strip_width)`, except the last, which extends to `width`. Levels are `start` if `k == 1`, else `round(start + (end - start) * i / (k - 1))` (f64, `round()` half away from zero). Every cell is assigned. Panics if `strip_width == 0`.

- [ ] **Step 1: Write the failing tests** in `crates/fap-core/tests/regions.rs`

```rust
use fap_core::{CoreError, Grid, Pos, RegionId, RegionSettings, Regions};
use std::collections::BTreeMap;

fn s(level: u8) -> RegionSettings { RegionSettings { level, margin: 2, name: None } }

fn ids(rows: &[&str]) -> Grid<Option<RegionId>> {
    Grid::from_rows(rows.iter().map(|r| r.split_whitespace()
        .map(|t| if t == "." { None } else { Some(RegionId(t.parse().unwrap())) }).collect()).collect()).unwrap()
}

#[test]
fn new_has_no_regions() {
    let r = Regions::new(4, 3, 2);
    assert!(r.table().is_empty());
    assert!(r.map().positions().all(|p| r.region_at(p).is_none()));
    assert_eq!(r.default_settings(), s(2));
}

#[test]
fn connected_region_keeps_id() {
    let r = Regions::from_parts(ids(&["1 1 .", ". 1 ."]), BTreeMap::from([(RegionId(1), s(3))]), 2).unwrap();
    assert_eq!(r.table().len(), 1);
    assert_eq!(r.cells(RegionId(1)), vec![Pos::new(0, 0), Pos::new(0, 1), Pos::new(1, 1)]);
}

#[test]
fn split_rule_gives_each_piece_new_id_and_same_settings() {
    let named = RegionSettings { level: 3, margin: 1, name: Some("x".into()) };
    let r = Regions::from_parts(ids(&["1 . 1"]), BTreeMap::from([(RegionId(1), named.clone())]), 2).unwrap();
    assert_eq!(r.table().len(), 2);
    assert!(r.settings(RegionId(1)).is_none());
    assert!(r.table().values().all(|v| *v == named));
    let a = r.region_at(Pos::new(0, 0)).unwrap();
    let b = r.region_at(Pos::new(0, 2)).unwrap();
    assert_ne!(a, b);
    assert!(a < b, "pieces numbered in row-major order");
}

#[test]
fn diagonal_contact_is_not_connected() {
    let r = Regions::from_parts(ids(&["1 .", ". 1"]), BTreeMap::from([(RegionId(1), s(2))]), 2).unwrap();
    assert_eq!(r.table().len(), 2);
}

#[test]
fn empty_table_entries_are_dropped() {
    let r = Regions::from_parts(ids(&["1 ."]), BTreeMap::from([(RegionId(1), s(2)), (RegionId(5), s(4))]), 2).unwrap();
    assert_eq!(r.table().keys().copied().collect::<Vec<_>>(), vec![RegionId(1)]);
}

#[test]
fn from_parts_rejects_unknown_id_and_bad_settings() {
    let e = Regions::from_parts(ids(&["1 7"]), BTreeMap::from([(RegionId(1), s(2))]), 2).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().contains("region 7") && e.to_string().contains("row 1, column 2"), "{e}");
    assert!(Regions::from_parts(ids(&["1"]), BTreeMap::from([(RegionId(1), s(5))]), 2).is_err());
    let bad_margin = RegionSettings { level: 2, margin: 3, name: None };
    assert!(Regions::from_parts(ids(&["1"]), BTreeMap::from([(RegionId(1), bad_margin)]), 2).is_err());
    assert!(Regions::from_parts(ids(&["."]), BTreeMap::new(), 0).is_err());
}

#[test]
fn create_assign_normalize() {
    let mut r = Regions::new(3, 1, 2);
    let id = r.create_region(r.default_settings());
    r.assign(Pos::new(0, 0), Some(id));
    r.assign(Pos::new(0, 2), Some(id));
    r.normalize();
    assert_eq!(r.table().len(), 2);
    let next = r.create_region(s(1));
    assert!(r.table().keys().all(|k| *k <= next));
}

#[test]
fn strips_fold_leftover_into_last() {
    let r = Regions::strips(23, 2, 5, 1, 4, 2, 2);
    let widths: Vec<usize> = (1..=4).map(|i| r.cells(RegionId(i)).len() / 2).collect();
    assert_eq!(widths, vec![5, 5, 5, 8]);
    let levels: Vec<u8> = (1..=4).map(|i| r.settings(RegionId(i)).unwrap().level).collect();
    assert_eq!(levels, vec![1, 2, 3, 4]);
    assert!(r.map().positions().all(|p| r.region_at(p).is_some()));
    assert_eq!(r.region_at(Pos::new(1, 22)), Some(RegionId(4)));
}

#[test]
fn strips_narrower_than_strip_width_is_one_region() {
    let r = Regions::strips(3, 2, 5, 1, 4, 1, 2);
    assert_eq!(r.table().len(), 1);
    assert_eq!(r.settings(RegionId(1)).unwrap(), &RegionSettings { level: 1, margin: 1, name: None });
}
```

- [ ] **Step 2: Run to verify failure**

Run: `nix develop -c cargo test -p fap-core --test regions`
Expected: compile errors (`Regions` etc. not found).

- [ ] **Step 3: Implement `regions.rs`** per Interfaces (BFS flood fill for components).

- [ ] **Step 4: Run to verify pass**

Run: `nix develop -c cargo test -p fap-core --test regions`
Expected: 9 passed.

- [ ] **Step 5: Commit**: `feat(core): add region model with split rule and strip preset`

---

### Task 3: Clue computation and the Puzzle type

**Files:**
- Create: `crates/fap-core/src/clues.rs`, `crates/fap-core/src/puzzle.rs`
- Modify: `crates/fap-core/src/lib.rs`
- Test: `crates/fap-core/tests/clues.rs`

**Interfaces:**
- Consumes: `Pos`, `Grid`, `Image`, `CoreError` (Task 1); `Regions` (Task 2).
- Produces:
  - `clues::neighborhood(width: usize, height: usize, p: Pos) -> impl Iterator<Item = Pos>`: in-grid cells of the 3×3 square centred on `p`, including `p`, row-major.
  - `clues::clue_at(image: &Image, p: Pos) -> u8`, and `clues::all_clues(image: &Image) -> Grid<u8>`.
  - `pub enum Clue { Blank, Digit(u8), GivenShaded, GivenUnshaded }` (derive `Clone, Copy, Debug, PartialEq, Eq`).
  - `pub struct Puzzle { pub title: Option<String>, pub clues: Grid<Clue>, pub solution: Image, pub regions: Regions, pub generation: toml::Table, pub stats: toml::Table }` (derive `Clone, Debug, PartialEq`).
  - `Puzzle::fully_clued(image: &Image, regions: Regions) -> Puzzle`: every cell `Digit(clue_at)`, empty tables, no title.
  - `Puzzle::validate(&self) -> Result<()>`: `CoreError::Format` if the dimensions of clues/solution/regions differ, if a digit doesn't equal `clue_at(solution, p)`, or if a given doesn't match the solution cell. The message includes `p.describe()`.

- [ ] **Step 1: Write the failing tests** in `crates/fap-core/tests/clues.rs`

```rust
use fap_core::{clues, Clue, CoreError, Grid, Image, Pos, Puzzle, Regions};

fn img(rows: &[&str]) -> Image {
    Grid::from_rows(rows.iter().map(|r| r.chars().map(|c| c == '#').collect()).collect()).unwrap()
}

#[test]
fn neighborhood_sizes_corner_edge_interior() {
    assert_eq!(clues::neighborhood(3, 3, Pos::new(0, 0)).count(), 4);
    assert_eq!(clues::neighborhood(3, 3, Pos::new(0, 1)).count(), 6);
    assert_eq!(clues::neighborhood(3, 3, Pos::new(1, 1)).count(), 9);
    assert!(clues::neighborhood(3, 3, Pos::new(1, 1)).any(|p| p == Pos::new(1, 1)));
    assert_eq!(clues::neighborhood(1, 1, Pos::new(0, 0)).count(), 1);
}

#[test]
fn off_grid_counts_as_unshaded() {
    let full = img(&["###", "###", "###"]);
    assert_eq!(clues::clue_at(&full, Pos::new(0, 0)), 4);
    assert_eq!(clues::clue_at(&full, Pos::new(0, 1)), 6);
    assert_eq!(clues::clue_at(&full, Pos::new(1, 1)), 9);
    assert_eq!(clues::clue_at(&img(&["#"]), Pos::new(0, 0)), 1);
}

#[test]
fn clue_counts_own_cell() {
    let i = img(&["...", ".#.", "..."]);
    let c = clues::all_clues(&i);
    assert!(i.positions().all(|p| *c.get(p) == 1));
}

#[test]
fn two_by_two_clues_all_equal_total() {
    let c = clues::all_clues(&img(&["#.", ".#"]));
    assert!(c.positions().all(|p| *c.get(p) == 2));
}

#[test]
fn fully_clued_puzzle_validates() {
    let i = img(&["#..#", ".##.", "#..#"]);
    let p = Puzzle::fully_clued(&i, Regions::new(4, 3, 2));
    assert_eq!(*p.clues.get(Pos::new(1, 1)), Clue::Digit(clues::clue_at(&i, Pos::new(1, 1))));
    assert!(p.validate().is_ok());
}

#[test]
fn validate_rejects_wrong_digit_and_wrong_given() {
    let i = img(&["#.", ".."]);
    let mut p = Puzzle::fully_clued(&i, Regions::new(2, 2, 2));
    p.clues.set(Pos::new(0, 1), Clue::Digit(3));
    let e = p.validate().unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().contains("row 1, column 2"), "{e}");

    let mut p = Puzzle::fully_clued(&i, Regions::new(2, 2, 2));
    p.clues.set(Pos::new(1, 1), Clue::GivenShaded);
    assert!(p.validate().unwrap_err().to_string().contains("row 2, column 2"));
    p.clues.set(Pos::new(1, 1), Clue::GivenUnshaded);
    assert!(p.validate().is_ok());
}

#[test]
fn validate_rejects_dimension_mismatch() {
    let mut p = Puzzle::fully_clued(&img(&["#.", ".."]), Regions::new(2, 2, 2));
    p.solution = img(&["#..", "..."]);
    assert!(matches!(p.validate(), Err(CoreError::Format(_))));
}
```

- [ ] **Step 2: Run to verify failure**

Run: `nix develop -c cargo test -p fap-core --test clues`
Expected: compile errors (`clues`, `Clue`, `Puzzle` not found).

- [ ] **Step 3: Add `toml` to fap-core** (`cargo add -p fap-core toml`) and **implement `clues.rs` and `puzzle.rs`** per Interfaces.

- [ ] **Step 4: Run to verify pass**

Run: `nix develop -c cargo test -p fap-core --test clues`
Expected: 7 passed.

- [ ] **Step 5: Commit**: `feat(core): add clue computation and puzzle type`

---

### Task 4: Grid text codecs

**Files:**
- Create: `crates/fap-core/src/format/mod.rs` (module declarations only for now), `crates/fap-core/src/format/gridtext.rs`
- Modify: `crates/fap-core/src/lib.rs` (`pub mod format;`)
- Test: `crates/fap-core/tests/gridtext.rs`

**Interfaces:**
- Consumes: `Grid`, `Image`, `Clue`, `RegionId`, `CoreError`.
- Produces (`fap_core::format::gridtext`):
  - `parse_image_rows(text: &str, width: usize, height: usize) -> Result<Image>`
  - `parse_clue_rows(text: &str, width: usize, height: usize) -> Result<Grid<Clue>>`
  - `parse_region_rows(text: &str, width: usize, height: usize) -> Result<Grid<Option<RegionId>>>`
  - `image_rows(img: &Image) -> String`, `clue_rows(c: &Grid<Clue>) -> String`, `region_rows(m: &Grid<Option<RegionId>>) -> String`
- Parsing rules: split on `\n`, strip a trailing `\r` and surrounding whitespace from each line, and drop blank lines. Then require exactly `height` lines and `width` cells per line. Char grids (image, clues) treat each char as a cell; region rows split on whitespace. Errors are `CoreError::Format`, worded `"row {r}: expected {width} cells, found {n}"`, `"expected {height} rows, found {n}"`, `"row {r}, column {c}: unexpected character '{ch}'"`, `"row {r}, column {c}: invalid region token '{tok}'"`, all one-based. Callers prefix the section (e.g. `"[image] "`).
- Emission: one line per row, each ending in `\n`. Region tokens are right-aligned to the widest token, with one space between columns.

- [ ] **Step 1: Write the failing tests** in `crates/fap-core/tests/gridtext.rs`

```rust
use fap_core::format::gridtext::*;
use fap_core::{Clue, Grid, Pos, RegionId};

#[test]
fn image_round_trip() {
    let img = parse_image_rows("\n.#\n#.\n", 2, 2).unwrap();
    assert!(*img.get(Pos::new(0, 1)) && !*img.get(Pos::new(0, 0)));
    assert_eq!(image_rows(&img), ".#\n#.\n");
}

#[test]
fn crlf_and_indentation_load_identically() {
    let clean = parse_image_rows(".#\n#.\n", 2, 2).unwrap();
    assert_eq!(parse_image_rows("\r\n   .#  \r\n\t#.\r\n\r\n", 2, 2).unwrap(), clean);
    assert_eq!(parse_image_rows(".#\n#.", 2, 2).unwrap(), clean);
}

#[test]
fn row_errors_are_one_based_and_specific() {
    let e = parse_image_rows(".#\n#..\n", 2, 2).unwrap_err().to_string();
    assert!(e.contains("row 2: expected 2 cells, found 3"), "{e}");
    let e = parse_image_rows(".#\n", 2, 2).unwrap_err().to_string();
    assert!(e.contains("expected 2 rows, found 1"), "{e}");
    let e = parse_image_rows(".q\n..\n", 2, 2).unwrap_err().to_string();
    assert!(e.contains("row 1, column 2: unexpected character 'q'"), "{e}");
}

#[test]
fn clue_chars() {
    let c = parse_clue_rows("2.#x9\n", 5, 1).unwrap();
    assert_eq!(c.row(0), &[Clue::Digit(2), Clue::Blank, Clue::GivenShaded, Clue::GivenUnshaded, Clue::Digit(9)]);
    assert_eq!(clue_rows(&c), "2.#x9\n");
    assert!(parse_clue_rows("X\n", 1, 1).is_err());
}

#[test]
fn region_tokens() {
    let m = parse_region_rows(" 1   10\n.  2\n", 2, 2).unwrap();
    assert_eq!(m.row(0), &[Some(RegionId(1)), Some(RegionId(10))]);
    assert_eq!(m.row(1), &[None, Some(RegionId(2))]);
    assert_eq!(region_rows(&m), " 1 10\n .  2\n");
    let e = parse_region_rows("1 a\n", 2, 1).unwrap_err().to_string();
    assert!(e.contains("row 1, column 2: invalid region token 'a'"), "{e}");
    assert!(parse_region_rows("0\n", 1, 1).is_err(), "IDs are positive");
}

#[test]
fn empty_region_map_emits_dots() {
    let m: Grid<Option<RegionId>> = Grid::new(3, 1, None);
    assert_eq!(region_rows(&m), ". . .\n");
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-core --test gridtext`. Expected: compile errors.

- [ ] **Step 3: Implement `gridtext.rs`** per Interfaces.

- [ ] **Step 4: Run to verify pass**: same command. Expected: 6 passed.

- [ ] **Step 5: Commit**: `feat(core): add grid text codecs`

---

### Task 5: Project type and project file format

**Files:**
- Create: `crates/fap-core/src/project.rs`, `crates/fap-core/src/format/project_file.rs`
- Modify: `crates/fap-core/src/format/mod.rs`, `crates/fap-core/src/lib.rs`
- Test: `crates/fap-core/tests/project_file.rs`

**Interfaces:**
- Consumes: Tasks 1, 2, 4.
- Produces:
  - `pub struct Project { pub title: Option<String>, pub image: Image, pub regions: Regions, pub generation: toml::Table, pub pdf: toml::Table }` (derive `Clone, Debug, PartialEq`). `Project::new(width, height) -> Project` makes a blank image, `Regions::new(w, h, DEFAULT_LEVEL)` and empty tables. `Project::width()` / `height()`.
  - In `fap_core::format`: consts `PROJECT_FORMAT = "fill-a-pix-project"`, `PUZZLE_FORMAT = "fill-a-pix-puzzle"`, `FORMAT_VERSION: u32 = 1`; `pub enum FileKind { Project, Puzzle }`; `detect_kind(text: &str) -> Result<FileKind>`; `load_project(text: &str) -> Result<Project>`; `save_project(p: &Project) -> String`; `read_project_file(path: &Path) -> Result<Project>`; `write_project_file(path: &Path, p: &Project) -> Result<()>`.
- File layout (spec §4), top-level keys `format`, `version`, `width`, `height`, optional `title`. `[image] rows`. `[regions] default_level, rows` is optional; when missing, there are no regions and the default level is 2. `[[region]] id, level, margin, name?`. `[generation]` and `[pdf]` are optional opaque tables, emitted only when non-empty.
- Rules:
  - Unknown keys anywhere are ignored (no `deny_unknown_fields`).
  - A wrong `format` gives `CoreError::Unsupported` naming the format found, e.g. `"expected fill-a-pix-project, found fill-a-pix-puzzle"`. `version > 1` gives `Unsupported` with `"version 2 is newer than supported version 1"`.
  - TOML parse failure gives `CoreError::Toml`. Content errors give `CoreError::Format`, prefixed with the section (`"[image] row 2: …"`, `"[regions] …"`).
  - Regions are built via `Regions::from_parts`, so a loaded file is normalised.
  - Grids are written as TOML multi-line basic strings (`rows = """` then a newline, then the rows). If `toml::to_string` doesn't produce that form, build the document with `toml_edit` (`cargo add -p fap-core toml_edit`) and set the string's repr explicitly.
  - Writes go to a temp file in the same directory, which is then renamed over the target, so a crash never leaves a half-written project.

- [ ] **Step 1: Write the failing tests** in `crates/fap-core/tests/project_file.rs`

```rust
use fap_core::format::{self, FileKind};
use fap_core::{CoreError, Pos, Project, RegionId};

const SAMPLE: &str = r#"
format = "fill-a-pix-project"
version = 1
width = 3
height = 2
title = "Sample"
future_key = 1

[image]
rows = """
#.#
.#.
"""
unknown_inner = true

[regions]
default_level = 3
rows = """
1 1 .
. 2 2
"""

[[region]]
id = 1
level = 1
margin = 2
name = "spine"

[[region]]
id = 2
level = 4
margin = 0

[generation]
seed = 42
"#;

#[test]
fn loads_sample_and_ignores_unknown_keys() {
    let p = format::load_project(SAMPLE).unwrap();
    assert_eq!((p.width(), p.height()), (3, 2));
    assert_eq!(p.title.as_deref(), Some("Sample"));
    assert!(*p.image.get(Pos::new(0, 0)) && !*p.image.get(Pos::new(0, 1)));
    assert_eq!(p.regions.default_level(), 3);
    assert_eq!(p.regions.settings(RegionId(1)).unwrap().name.as_deref(), Some("spine"));
    assert_eq!(p.regions.region_at(Pos::new(1, 2)), Some(RegionId(2)));
    assert_eq!(p.generation.get("seed").and_then(|v| v.as_integer()), Some(42));
    assert!(p.pdf.is_empty());
    assert!(matches!(format::detect_kind(SAMPLE).unwrap(), FileKind::Project));
}

#[test]
fn round_trip_and_multiline_rows() {
    let p = format::load_project(SAMPLE).unwrap();
    let text = format::save_project(&p);
    assert!(text.contains("format = \"fill-a-pix-project\""));
    assert!(text.contains("version = 1"));
    assert!(text.contains("rows = \"\"\"\n#.#\n.#.\n"), "{text}");
    assert_eq!(format::load_project(&text).unwrap(), p);
}

#[test]
fn new_project_round_trips_without_regions_section() {
    let p = Project::new(4, 2);
    let back = format::load_project(&format::save_project(&p)).unwrap();
    assert_eq!(back, p);
    let minimal = "format = \"fill-a-pix-project\"\nversion = 1\nwidth = 2\nheight = 1\n[image]\nrows = \"\"\"\n#.\n\"\"\"\n";
    let m = format::load_project(minimal).unwrap();
    assert!(m.regions.table().is_empty());
    assert_eq!(m.regions.default_level(), 2);
}

#[test]
fn disconnected_region_in_file_is_split() {
    let text = SAMPLE.replace("1 1 .\n. 2 2", "1 . 1\n. 2 2");
    let p = format::load_project(&text).unwrap();
    assert_eq!(p.regions.table().len(), 3);
}

#[test]
fn undefined_region_id_names_row_and_id() {
    let text = SAMPLE.replace(". 2 2", ". 2 9");
    let e = format::load_project(&text).unwrap_err().to_string();
    assert!(e.contains("region 9") && e.contains("row 2, column 3"), "{e}");
}

#[test]
fn row_length_mismatch_names_section_and_row() {
    let text = SAMPLE.replace(".#.\n", ".#\n");
    let e = format::load_project(&text).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().contains("[image] row 2"), "{e}");
}

#[test]
fn puzzle_file_loaded_as_project_is_rejected_by_kind() {
    let text = SAMPLE.replace("fill-a-pix-project", "fill-a-pix-puzzle");
    let e = format::load_project(&text).unwrap_err();
    assert!(matches!(e, CoreError::Unsupported(_)));
    assert!(e.to_string().contains("found fill-a-pix-puzzle"), "{e}");
}

#[test]
fn newer_version_is_rejected() {
    let e = format::load_project(&SAMPLE.replace("version = 1", "version = 2")).unwrap_err();
    assert!(e.to_string().contains("version 2 is newer than supported version 1"), "{e}");
}

#[test]
fn toml_syntax_error() {
    assert!(matches!(format::load_project("format = "), Err(CoreError::Toml(_))));
}

#[test]
fn file_helpers_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.project.toml");
    let mut p = Project::new(2, 2);
    p.regions = fap_core::Regions::strips(2, 2, 1, 1, 2, 1, 2);
    format::write_project_file(&path, &p).unwrap();
    assert_eq!(format::read_project_file(&path).unwrap(), p);
}
```

- [ ] **Step 2: Add test deps and run to verify failure**

Run: `nix develop -c cargo add -p fap-core --dev tempfile && nix develop -c cargo add -p fap-core serde --features derive && nix develop -c cargo test -p fap-core --test project_file`
Expected: compile errors (`Project`, `format::load_project` not found).

- [ ] **Step 3: Implement `project.rs`, `format/project_file.rs`, and the `format/mod.rs` API** (serde structs for the raw file → convert via `gridtext` and `Regions::from_parts`; `detect_kind` reads only the `format` key).

- [ ] **Step 4: Run to verify pass**: same test command. Expected: 10 passed.

- [ ] **Step 5: Commit**: `feat(core): add project type and TOML project format`

---

### Task 6: Puzzle file format

**Files:**
- Create: `crates/fap-core/src/format/puzzle_file.rs`
- Modify: `crates/fap-core/src/format/mod.rs`
- Test: `crates/fap-core/tests/puzzle_file.rs`

**Interfaces:**
- Consumes: Tasks 3, 4, 5 (shares the `[regions]` / `[[region]]` serde structs and the format/version checks with `project_file.rs`; factor those into `format/mod.rs` rather than duplicating them).
- Produces: `format::load_puzzle(text: &str) -> Result<Puzzle>`, `format::save_puzzle(p: &Puzzle) -> String`, `format::read_puzzle_file(path) -> Result<Puzzle>`, `format::write_puzzle_file(path, p) -> Result<()>` (atomic like projects).
- Layout: top-level `format = "fill-a-pix-puzzle"`, `version`, `width`, `height`, optional `title`. Then `[clues] rows`, `[solution] rows`, optional `[regions]` + `[[region]]` (same as projects), and optional opaque `[generation]` and `[stats]`. `load_puzzle` calls `Puzzle::validate()` and returns its error unchanged.

- [ ] **Step 1: Write the failing tests** in `crates/fap-core/tests/puzzle_file.rs`

```rust
use fap_core::format::{self, FileKind};
use fap_core::{Clue, CoreError, Grid, Pos, Puzzle, Regions};

fn sample() -> Puzzle {
    let img: fap_core::Image = Grid::from_rows(vec![vec![true, false, true], vec![false, true, false]]).unwrap();
    let mut p = Puzzle::fully_clued(&img, Regions::strips(3, 2, 3, 2, 2, 1, 2));
    p.title = Some("T".into());
    p.clues.set(Pos::new(0, 1), Clue::Blank);
    p.clues.set(Pos::new(0, 0), Clue::GivenShaded);
    p.clues.set(Pos::new(1, 0), Clue::GivenUnshaded);
    p.stats.insert("score".into(), toml::Value::Float(3.5));
    p
}

#[test]
fn round_trip() {
    let p = sample();
    let text = format::save_puzzle(&p);
    assert!(text.contains("format = \"fill-a-pix-puzzle\""));
    assert!(text.contains("rows = \"\"\"\n#.2\nx32\n"), "{text}");
    assert!(matches!(format::detect_kind(&text).unwrap(), FileKind::Puzzle));
    assert_eq!(format::load_puzzle(&text).unwrap(), p);
}

#[test]
fn mismatched_clue_is_rejected_on_load() {
    let text = format::save_puzzle(&sample());
    let bad = text.replacen("#.2", "#.5", 1);
    assert_ne!(bad, text, "fixture must contain the clue row");
    let e = format::load_puzzle(&bad).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().contains("row 1, column 3"), "{e}");
}

#[test]
fn project_loaded_as_puzzle_is_rejected_by_kind() {
    let text = format::save_project(&fap_core::Project::new(2, 2));
    let e = format::load_puzzle(&text).unwrap_err();
    assert!(e.to_string().contains("found fill-a-pix-project"), "{e}");
}

#[test]
fn regions_section_is_optional() {
    let text = "format = \"fill-a-pix-puzzle\"\nversion = 1\nwidth = 1\nheight = 1\n\
                [clues]\nrows = \"\"\"\n1\n\"\"\"\n[solution]\nrows = \"\"\"\n#\n\"\"\"\n";
    let p = format::load_puzzle(text).unwrap();
    assert!(p.regions.table().is_empty());
}
```

(Fixture check: the solution is `#.#` / `.#.`, whose full clues are `232` / `232`; with the three overrides the clue rows are `#.2` / `x32`.)

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-core --test puzzle_file`. Expected: compile errors (`load_puzzle` not found).

- [ ] **Step 3: Implement `puzzle_file.rs`**, sharing the region and header code with the project format.

- [ ] **Step 4: Run to verify pass**: same command. Expected: 4 passed.

- [ ] **Step 5: Commit**: `feat(core): add TOML puzzle format`

---

### Task 7: PBM I/O

**Files:**
- Create: `crates/fap-core/src/pbm.rs`
- Modify: `crates/fap-core/src/lib.rs`
- Test: `crates/fap-core/tests/pbm.rs`

**Interfaces:**
- Produces: `pbm::read_pbm(bytes: &[u8]) -> Result<Image>`, `pbm::write_pbm(img: &Image) -> Vec<u8>`, `pbm::read_pbm_file(path: &Path) -> Result<Image>`, `pbm::write_pbm_file(path: &Path, img: &Image) -> Result<()>`.
- Netpbm rules:
  - `1` = black = shaded.
  - The header is magic, then width, then height, separated by whitespace. A `#` comment runs to the end of the line and can appear anywhere in the header (and in P1 data).
  - **P1:** after the header, each `0`/`1` char is one pixel, whitespace optional.
  - **P4:** exactly one whitespace byte after the height, then rows of `ceil(width/8)` bytes, MSB first, with padding bits ignored.
  - **Writing:** always P4, header `"P4\n{w} {h}\n"`.
  - **Errors:** `CoreError::Pbm`, for unsupported magic (message contains the magic, e.g. `"unsupported PBM type P5"`), a zero dimension, or truncated data.

- [ ] **Step 1: Write the failing tests** in `crates/fap-core/tests/pbm.rs`

```rust
use fap_core::pbm::{read_pbm, write_pbm};
use fap_core::{CoreError, Grid, Image, Pos};

fn ten_by_two() -> Image {
    let mut g = Grid::new(10, 2, false);
    g.set(Pos::new(0, 0), true);
    g.set(Pos::new(0, 9), true);
    g
}

#[test]
fn writes_p4_exact_bytes() {
    let mut expected = b"P4\n10 2\n".to_vec();
    expected.extend_from_slice(&[0x80, 0x40, 0x00, 0x00]);
    assert_eq!(write_pbm(&ten_by_two()), expected);
}

#[test]
fn p4_round_trip() {
    assert_eq!(read_pbm(&write_pbm(&ten_by_two())).unwrap(), ten_by_two());
}

#[test]
fn p4_padding_bits_ignored() {
    let mut bytes = b"P4\n# made elsewhere\n10 2\n".to_vec();
    bytes.extend_from_slice(&[0x80, 0x7F, 0x00, 0x3F]);
    assert_eq!(read_pbm(&bytes).unwrap(), ten_by_two());
}

#[test]
fn p1_comments_and_unseparated_digits() {
    let img = read_pbm(b"P1\n# comment\n3 2\n101\n0 1 # trailing\n 0\n").unwrap();
    let expect: Image = Grid::from_rows(vec![vec![true, false, true], vec![false, true, false]]).unwrap();
    assert_eq!(img, expect);
}

#[test]
fn errors() {
    let e = read_pbm(b"P5\n1 1\n255\n\0").unwrap_err();
    assert!(matches!(e, CoreError::Pbm(_)) && e.to_string().contains("P5"), "{e}");
    assert!(matches!(read_pbm(b"P4\n10 2\n\x80"), Err(CoreError::Pbm(_))));
    assert!(matches!(read_pbm(b"P1\n0 2\n"), Err(CoreError::Pbm(_))));
    assert!(matches!(read_pbm(b"P1\n2 1\n1"), Err(CoreError::Pbm(_))));
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-core --test pbm`. Expected: compile errors.

- [ ] **Step 3: Implement `pbm.rs`** (hand-written parser; no image crate needed).

- [ ] **Step 4: Run to verify pass**: same command. Expected: 5 passed.

- [ ] **Step 5: Commit**: `feat(core): add PBM reader and writer`

---

### Task 8: `fap` CLI skeleton

**Files:**
- Create: `crates/fap-cli/Cargo.toml` (`[[bin]] name = "fap"`), `crates/fap-cli/src/main.rs`, `crates/fap-cli/src/commands.rs`
- Test: `crates/fap-cli/tests/cli.rs`

**Interfaces:**
- Consumes: the `fap-core` public API.
- Produces (subcommands; later plans add `generate`, `check`, `grade`, `render` as new variants):
  - `fap new --width W --height H -o OUT`: writes a blank project.
  - `fap import-pbm IN.pbm -o OUT`: new project from a PBM image.
  - `fap export-pbm IN -o OUT.pbm`: IN may be a project (exports the image) or a puzzle (exports the solution), detected by `format::detect_kind`.
  - `fap strips IN --strip-width N --start L --end L --margin D [-o OUT]`: replaces IN's regions with `Regions::strips(...)`, keeping the project's default level. Writes OUT, or IN in place when `-o` is absent.
  - `fap clue-all IN -o OUT`: writes `Puzzle::fully_clued(image, regions)` with the project's title (the trivial generator from spec §6.6 step 1).
  - `fap info IN` prints, for a project:
    ```
    project {w}x{h}
    title: {title or (none)}
    shaded: {n}
    default level: {d}
    regions: {k}
      region {id}: {cells} cells, level {l}, margin {m}[, name "{name}"]
    ```
    For a puzzle, the same with the first line `puzzle {w}x{h}`, `shaded: {n}` counting the solution, plus `digits: {n}` and `givens: {n}` lines after `shaded`.
  - Errors: print `error: {message}` to stderr and exit with code 1. File errors include the path.

- [ ] **Step 1: Create the crate and add deps**

Run: `nix develop -c cargo new --bin crates/fap-cli --name fap-cli && nix develop -c cargo add -p fap-cli clap --features derive && nix develop -c cargo add -p fap-cli --path crates/fap-core && nix develop -c cargo add -p fap-cli --dev assert_cmd predicates tempfile`
Then set `[[bin]] name = "fap"`, `path = "src/main.rs"` and the workspace package fields in `crates/fap-cli/Cargo.toml`.

- [ ] **Step 2: Write the failing tests** in `crates/fap-cli/tests/cli.rs`

```rust
use assert_cmd::Command;
use predicates::str::contains;

fn fap() -> Command { Command::cargo_bin("fap").unwrap() }

#[test]
fn new_then_info() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.project.toml");
    fap().args(["new", "--width", "4", "--height", "3", "-o"]).arg(&p).assert().success();
    fap().arg("info").arg(&p).assert().success()
        .stdout(contains("project 4x3")).stdout(contains("shaded: 0")).stdout(contains("regions: 0"));
}

#[test]
fn pbm_import_export_round_trip() {
    let d = tempfile::tempdir().unwrap();
    let (pbm, proj, out) = (d.path().join("i.pbm"), d.path().join("i.project.toml"), d.path().join("o.pbm"));
    let mut bytes = b"P4\n10 2\n".to_vec();
    bytes.extend_from_slice(&[0x80, 0x40, 0x00, 0x00]);
    std::fs::write(&pbm, &bytes).unwrap();
    fap().arg("import-pbm").arg(&pbm).arg("-o").arg(&proj).assert().success();
    fap().arg("info").arg(&proj).assert().stdout(contains("shaded: 2"));
    fap().arg("export-pbm").arg(&proj).arg("-o").arg(&out).assert().success();
    assert_eq!(std::fs::read(&out).unwrap(), bytes);
}

#[test]
fn strips_in_place() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("s.project.toml");
    fap().args(["new", "--width", "23", "--height", "2", "-o"]).arg(&p).assert().success();
    fap().arg("strips").arg(&p).args(["--strip-width", "5", "--start", "1", "--end", "4", "--margin", "2"]).assert().success();
    fap().arg("info").arg(&p).assert().success()
        .stdout(contains("regions: 4"))
        .stdout(contains("region 1: 10 cells, level 1, margin 2"))
        .stdout(contains("region 4: 16 cells, level 4, margin 2"));
}

#[test]
fn clue_all_produces_valid_puzzle_and_solution_exports() {
    let d = tempfile::tempdir().unwrap();
    let (proj, puz, out) = (d.path().join("c.project.toml"), d.path().join("c.puzzle.toml"), d.path().join("c.pbm"));
    fap().args(["new", "--width", "4", "--height", "3", "-o"]).arg(&proj).assert().success();
    fap().arg("clue-all").arg(&proj).arg("-o").arg(&puz).assert().success();
    fap().arg("info").arg(&puz).assert().success()
        .stdout(contains("puzzle 4x3")).stdout(contains("digits: 12")).stdout(contains("givens: 0"));
    fap().arg("export-pbm").arg(&puz).arg("-o").arg(&out).assert().success();
    assert!(std::fs::read(&out).unwrap().starts_with(b"P4\n4 3\n"));
}

#[test]
fn missing_file_reports_path() {
    fap().args(["info", "/nonexistent/x.project.toml"]).assert().code(1)
        .stderr(contains("error:")).stderr(contains("/nonexistent/x.project.toml"));
}
```

- [ ] **Step 3: Run to verify failure**: `nix develop -c cargo test -p fap-cli`. Expected: the tests fail (unknown subcommands / stub binary).

- [ ] **Step 4: Implement `main.rs` (clap derive `Cli` + `Command` enum, dispatch, error printing) and `commands.rs`** (one `pub fn` per subcommand returning `Result<(), String>`; every core error from reading or writing a file is mapped to `format!("{}: {}", path.display(), err)`).

- [ ] **Step 5: Run to verify pass**: `nix develop -c cargo test --workspace`. Expected: all fap-core and fap-cli tests pass (5 CLI tests).

- [ ] **Step 6: Manual smoke check**

Run: `nix develop -c cargo run -q -p fap-cli -- --help`
Expected: lists `new`, `import-pbm`, `export-pbm`, `strips`, `clue-all`, `info`.

- [ ] **Step 7: Commit**: `feat(cli): add fap CLI with file utilities and clue-all`

---

## Done when

- `nix develop -c cargo test --workspace`, `cargo clippy … -D warnings` and `cargo fmt --check` are all clean.
- `fap` can create, import, slice, fully clue and inspect files, as described in the format sections of spec §4.
- The branch is merged to the main line, ready for the three parallel worktrees (Plans 1–3).
