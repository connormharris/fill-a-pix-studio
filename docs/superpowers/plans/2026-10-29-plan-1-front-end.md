# Plan 1 — Front End (Track 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The `fap-gui` desktop editor: image editing (pencil, selections, cut/copy/paste, drag-move, row/column structure edits, rescale, PBM import/export), region mode (regions, colours, labels, panel), and two undo stacks.

**Architecture:** All editing semantics live in `fap_gui::model`, pure Rust modules with no egui dependency and unit-tested against the spec's worked examples. `fap_gui::ui` holds the eframe app: drawing, input handling, dialogs and menus. It translates input into `model` calls, and its only logic is two small pure helpers (`geom`, `keymap`) that are tested too. `model::Document` is the single owner of editor state: project, selection, clipboard, history, mode, tool and region colours. Every edit goes through it, so undo recording and the dirty flag can't be bypassed. The only fap-core change is the `rescale` module (spec §3), added in Task 1.

**Tech Stack:** Rust edition 2024, `eframe`/`egui` (MIT/Apache), `rfd` (MIT) for native file dialogs, `fap-core` from Plan 0.

**Spec:** `docs/superpowers/specs/2026-10-29-fill-a-pix-design.md`, §5.1–§5.9 and the Track 1 order in §8. §5.10 (generate/export dialogs) is Plan 4. **Depends on:** Plan 0 (`docs/superpowers/plans/2026-10-29-plan-0-foundation.md`), merged. Read both before starting.

> **Plan 0 final-review additions (merged after this plan was drafted):** `fap-core` now also provides `Regions::validate()`, `Project::validate()`, `regions::MAX_REGION_ID` (= `u32::MAX / 2`; larger IDs are rejected on load) and `pub use toml` (use `fap_core::toml` instead of adding your own `toml` dependency). `Regions` equality ignores the internal `next_id` cursor, so a saved and reloaded value compares equal. `write_project_file` / `write_puzzle_file` validate before writing and return `Err` without touching the old file. `Puzzle::validate()` also validates its regions and rejects digits above 9. Opaque `[generation]` / `[pdf]` / `[stats]` tables round-trip exactly, datetimes included, but a NaN float makes `Project`/`Puzzle` compare unequal, so never write NaN into them (e.g. use 0.0 for an empty average).

## Global Constraints

- All of Plan 0's Global Constraints apply unchanged: `nix develop -c` prefix, edition 2024, no `unsafe`, `license = "GPL-3.0-or-later"`, `cargo add` for deps, conventional commits with the two attribution lines, and fmt/clippy (`-D warnings`)/test gates before every commit.
- This track runs in parallel with Plans 2 and 3. Touch `fap-core` **only** in Task 1, additively: a new module `fap_core::rescale` and its `lib.rs` line. Never change existing fap-core signatures. `Cargo.lock` conflicts at merge are resolved by re-running `cargo build`.
- `fap_gui::model` must not depend on `eframe`/`egui`.
- Undo depth is 32 per stack (`UNDO_DEPTH = 32`). There is an image stack and a region stack.
- Region colours: exactly 6, drawn translucent (alpha 0.5, i.e. 128/255). Colours are session state only and are never saved.
- Modifier key: "Cmd/Ctrl" is `egui::Modifiers::command` (Cmd on macOS, Ctrl elsewhere). The wand's remove modifier is `alt` (Option on macOS).
- Keyboard shortcuts are exactly the spec §5.8 table. Each task adds the keys for the features it builds.
- Spreadsheet notation in tests: letter = column, number = row, so `C2` is `Pos::new(1, 2)`.
- macOS is the primary platform. **Optional Linux note:** eframe on Linux needs, in `flake.nix` `mkShell`, `buildInputs = with pkgs; [ libxkbcommon libGL wayland xorg.libX11 xorg.libXcursor xorg.libXrandr xorg.libXi ]` and `LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (those same packages)`. `rfd` additionally needs `gtk3` or an xdg-desktop-portal. Add these only when someone develops on Linux; they are not part of any task.

## Review Focus

1. **Pasting or dragging partly off the grid:** cells landing outside the grid are dropped, with no panic, and the selection shrinks to what landed. Tests: `paste_clips_at_edges` (Task 6), `drag_move_clips_at_edges` (Task 7).
2. **Degenerate structure edits:** inserting 0 rows, deleting every row or column, or an insertion point past the end are refused with a message, not a panic. Tests: `zero_count_and_bad_positions_refused`, `delete_everything_is_refused` (Task 8).
3. **Region undo after a size change that was never undone:** region snapshots must already be resized, so undoing a region edit after inserting a row never restores a wrongly sized map. Test: `region_undo_after_resize_uses_resized_snapshots` (Task 5).
4. **Losing work:** closing the window, New, Open or Import with unsaved changes asks first, and a failed save keeps the document dirty. Tests: `dirty_tracking`, `failed_save_stays_dirty` (Task 3).
5. **Big images stay responsive:** at 30×300 and 300×300 only visible cells are drawn. Test: `visible_range_only_covers_clip` (Task 2), plus a manual check at 300×300 (Task 2).

---

## File Structure

```
crates/fap-core/src/rescale.rs          area-weighted image/region rescale (spec §5.5)
crates/fap-core/tests/rescale.rs
crates/fap-gui/
  Cargo.toml                            [[bin]] name = "fap-gui"
  src/main.rs                           eframe::run_native(FapApp)
  src/lib.rs                            pub mod model; pub mod ui;
  src/model/mod.rs                      Document, Mode, Tool
  src/model/history.rs                  UndoStack<S>, History (two stacks, resize entries)
  src/model/selection.rs                Selection, SetOp (spreadsheet algebra)
  src/model/image_ops.rs                fill / invert over a selection
  src/model/clipboard.rs                Clipboard, copy/cut/paste/drag_move, PBM helpers
  src/model/components.rs               Connectivity, flood fills, components
  src/model/structure.rs                Axis, ImageFill, RegionFill, ResizeOp
  src/model/region_ops.rs               new / merge / remove region
  src/model/labels.rs                   RegionLabel, labels()
  src/model/coloring.rs                 adjacency, color_graph, recolor
  src/ui/mod.rs
  src/ui/app.rs                         FapApp: menus, toolbar, status bar, modal guards
  src/ui/canvas.rs                      grid drawing, zoom/scroll, pointer → tool actions
  src/ui/geom.rs                        pure geometry (cell_at, visible_range, zoom_to_fit)
  src/ui/keymap.rs                      Action, action_for(key, mods, mode)
  src/ui/dialogs.rs                     new canvas, insert/delete, rescale, strips
  src/ui/region_panel.rs                region list, default level, property toolbar
  tests/common/mod.rs                   a1(), img(), rows_of(), cells(), regions_from(), rect()
  tests/{geom,keymap,document,selection,history,clipboard,structure,components,
         labels,coloring,region_ops}.rs
```

### Shared test helpers (`crates/fap-gui/tests/common/mod.rs`, created in Task 2)

```rust
#![allow(dead_code)]
use fap_core::{format::gridtext, Grid, Image, Pos, RegionId, RegionSettings, Regions};
use std::collections::BTreeMap;

/// Spreadsheet notation: "C2" = column C, row 2 = Pos::new(1, 2).
pub fn a1(s: &str) -> Pos {
    let letters: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let col = letters.chars().fold(0usize, |a, c| a * 26 + (c as usize - 'A' as usize + 1)) - 1;
    Pos::new(s[letters.len()..].parse::<usize>().unwrap() - 1, col)
}
/// Also used as a selection mask: '#' = true.
pub fn img(rows: &[&str]) -> Image {
    Grid::from_rows(rows.iter().map(|r| r.chars().map(|c| c == '#').collect()).collect()).unwrap()
}
pub fn rows_of(i: &Image) -> Vec<String> {
    (0..i.height()).map(|r| i.row(r).iter().map(|&b| if b { '#' } else { '.' }).collect()).collect()
}
pub fn cells(mask: &Grid<bool>) -> Vec<Pos> { mask.positions().filter(|p| *mask.get(*p)).collect() }
/// Region map from token rows; every ID gets level 2, margin 2. Goes through
/// `from_parts`, so disconnected IDs are split into new IDs.
pub fn regions_from(rows: &[&str]) -> Regions {
    let w = rows[0].split_whitespace().count();
    let map = gridtext::parse_region_rows(&rows.join("\n"), w, rows.len()).unwrap();
    let table: BTreeMap<RegionId, RegionSettings> = map.positions().filter_map(|p| *map.get(p))
        .map(|id| (id, RegionSettings { level: 2, margin: 2, name: None })).collect();
    Regions::from_parts(map, table, 2).unwrap()
}
/// Inclusive spreadsheet rectangle, row-major.
pub fn rect(a: &str, b: &str) -> Vec<Pos> {
    let (p, q) = (a1(a), a1(b));
    let mut v = vec![];
    for r in p.row.min(q.row)..=p.row.max(q.row) { for c in p.col.min(q.col)..=p.col.max(q.col) { v.push(Pos::new(r, c)); } }
    v
}
```

---

### Task 1: `fap_core::rescale` (the only fap-core change)

**Files:**
- Create: `crates/fap-core/src/rescale.rs`, `crates/fap-core/tests/rescale.rs`
- Modify: `crates/fap-core/src/lib.rs` (add `pub mod rescale;` only)

**Interfaces:**
- Consumes: Plan 0's `Image`, `Grid`, `Regions::{from_parts, map, table, default_level}`, `format::gridtext::parse_region_rows`.
- Produces:
  - `rescale::rescale_image(img: &Image, width: usize, height: usize) -> Image`. Output cell `(i, j)` covers the input area `[i·H/h, (i+1)·H/h) × [j·W/w, (j+1)·W/w)`. It is shaded iff 2·(shaded overlap area) ≥ (cell area). Compute in exact integers: scale row coordinates by `h` and columns by `w`, so input row `k` spans `[k·h, (k+1)·h)` and output row `i` spans `[i·H, (i+1)·H)`; then sum overlap products as `u64`. Never use floats.
  - `rescale::rescale_regions(r: &Regions, width: usize, height: usize) -> Regions`. Each output cell goes to the candidate (each region ID, or "no region") with the largest overlap area. Ties prefer any region over "no region", then the lower `RegionId`. Settings and default level are kept. The result is built with `Regions::from_parts`, so the split rule applies and vanished regions drop out.
  - Both panic if `width` or `height` is 0 (callers validate first).

- [ ] **Step 1: Write the failing tests** in `crates/fap-core/tests/rescale.rs`

```rust
use fap_core::format::gridtext::parse_region_rows;
use fap_core::rescale::{rescale_image, rescale_regions};
use fap_core::{Grid, Image, Pos, RegionId, RegionSettings, Regions};
use std::collections::BTreeMap;

fn img(rows: &[&str]) -> Image {
    Grid::from_rows(rows.iter().map(|r| r.chars().map(|c| c == '#').collect()).collect()).unwrap()
}
fn regions(row: &str, levels: &[(u32, u8)]) -> Regions {
    let w = row.split_whitespace().count();
    let table: BTreeMap<_, _> = levels.iter()
        .map(|&(id, l)| (RegionId(id), RegionSettings { level: l, margin: 2, name: None })).collect();
    Regions::from_parts(parse_region_rows(row, w, 1).unwrap(), table, 2).unwrap()
}

#[test]
fn identity_and_upscale() {
    let i = img(&["#.", ".#"]);
    assert_eq!(rescale_image(&i, 2, 2), i);
    assert_eq!(rescale_image(&img(&["#."]), 4, 2), img(&["##..", "##.."]));
}

#[test]
fn exactly_half_is_shaded() {
    assert_eq!(rescale_image(&img(&["#."]), 1, 1), img(&["#"]));
    assert_eq!(rescale_image(&img(&["#.", ".."]), 1, 1), img(&["."]));
}

#[test]
fn fractional_overlap() {
    assert_eq!(rescale_image(&img(&["#.."]), 2, 1), img(&["#."])); // 1/1.5 shaded, then 0
    assert_eq!(rescale_image(&img(&[".#."]), 2, 1), img(&[".."])); // 1/3 each
}

#[test]
fn regions_largest_share_wins_and_settings_kept() {
    let r = rescale_regions(&regions("1 1 2", &[(1, 1), (2, 4)]), 2, 1);
    assert_eq!(r.region_at(Pos::new(0, 0)), Some(RegionId(1)));
    assert_eq!(r.region_at(Pos::new(0, 1)), Some(RegionId(2)));
    assert_eq!(r.settings(RegionId(2)).unwrap().level, 4);
}

#[test]
fn region_ties_prefer_region_then_lower_id() {
    assert_eq!(rescale_regions(&regions("1 .", &[(1, 2)]), 1, 1).region_at(Pos::new(0, 0)), Some(RegionId(1)));
    assert_eq!(rescale_regions(&regions("1 2", &[(1, 2), (2, 2)]), 1, 1).region_at(Pos::new(0, 0)), Some(RegionId(1)));
}

#[test]
fn small_region_can_vanish() {
    let r = rescale_regions(&regions("1 1 1 2", &[(1, 2), (2, 3)]), 1, 1);
    assert_eq!(r.table().keys().copied().collect::<Vec<_>>(), vec![RegionId(1)]);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-core --test rescale`. Expected: compile error, `rescale` not found.
- [ ] **Step 3: Implement `rescale.rs`** per Interfaces.
- [ ] **Step 4: Run to verify pass**: same command. Expected: 6 passed.
- [ ] **Step 5: Commit**: `feat(core): add area-weighted image and region rescale`

---

### Task 2: GUI scaffold: document, canvas, zoom/scroll, pencil

**Files:**
- Create: `crates/fap-gui/{Cargo.toml, src/main.rs, src/lib.rs, src/model/mod.rs, src/ui/mod.rs, src/ui/app.rs, src/ui/canvas.rs, src/ui/geom.rs, src/ui/keymap.rs}`, `crates/fap-gui/tests/{common/mod.rs, geom.rs, keymap.rs, document.rs}`

**Interfaces:**
- Consumes: `fap_core::{Project, Image, Pos, Grid}`.
- Produces:
  - `model::Mode { Image, Region }`, `model::Tool { Pencil, Select, Wand }` (derive `Clone, Copy, Debug, PartialEq, Eq`).
  - `model::Document` with public fields `project: Project`, `path: Option<PathBuf>`, `dirty: bool`, `mode: Mode`, `tool: Tool`. Later tasks add `selection`, `clipboard`, `history`, `colors`, `connectivity`, `problem_cells`.
  - `Document::new(project: Project) -> Document`: clean, image mode, Pencil.
  - `Document::stroke_begin(&mut self, p: Pos)`: inverts `p` and remembers that value. `stroke_continue(&mut self, p: Pos)` sets `p` to the remembered value. `stroke_end(&mut self)`. A stroke is one edit (Task 5 makes it one undo step). It does nothing in region mode or when the tool isn't Pencil.
  - `ui::geom`: constants `MIN_CELL_PX: f32 = 2.0`, `MAX_CELL_PX: f32 = 64.0`, `GRID_LINES_MIN_PX: f32 = 6.0`, `HEAVY_LINE_EVERY: usize = 5`.
    - `cell_at(local: egui::Pos2, cell_px: f32, width: usize, height: usize) -> Option<Pos>` (`local` is relative to the grid's top-left).
    - `cell_at_unclamped(local: egui::Pos2, cell_px: f32) -> (isize, isize)` (row, col; used for paste and drag offsets).
    - `visible_range(clip: egui::Rect, cell_px: f32, width: usize, height: usize) -> (Range<usize>, Range<usize>)` (rows, cols; `clip` in grid-local coords, clamped to the grid).
    - `zoom_to_fit(avail: egui::Vec2, width: usize, height: usize) -> f32` (largest px fitting both axes, clamped to the min/max).
  - `ui::keymap`: `enum Action { ToolPencil, ToolSelect, ToolWand, ToggleRegionMode, Clear, Fill, Invert, NewRegion, MergeRegion, RemoveFromRegion, Undo, Redo, Cut, Copy, Paste, PasteShadeOnly, SelectAll, Escape, ZoomIn, ZoomOut, ZoomFit, New, Open, Save, SaveAs }` and `action_for(key: egui::Key, mods: egui::Modifiers, mode: Mode) -> Option<Action>`. This task maps `P` (image mode only), `+`/`=` → ZoomIn, `-` → ZoomOut, `0` → ZoomFit, and Cmd/Ctrl-`N`/`O`/`S`, Shift-Cmd/Ctrl-`S` → SaveAs.
  - `ui::app::FapApp` implementing `eframe::App`. It holds a `Document` and view state (`cell_px`). The canvas is a `ScrollArea::both()` sized `width·cell_px × height·cell_px` that paints only `visible_range`. Cells are dark grey when shaded and white when unshaded. Thin grid lines appear when `cell_px ≥ GRID_LINES_MIN_PX`, with heavier lines every 5 cells counted from the origin. Row/column rulers (1-based numbers) run along the top and left edges. Cmd/Ctrl+wheel and pinch zoom around the pointer; plain wheel scrolls.
  - Startup: a blank 30×30 project. The New dialog arrives in Task 3.

- [ ] **Step 1: Create the crate**: `nix develop -c cargo new --bin crates/fap-gui && nix develop -c cargo add -p fap-gui eframe rfd && nix develop -c cargo add -p fap-gui --path crates/fap-core`. Add `src/lib.rs` (`pub mod model; pub mod ui;`), and have `main.rs` call `eframe::run_native("Fill-a-Pix", NativeOptions::default(), Box::new(|_| Ok(Box::new(FapApp::new()))))`. Set the workspace package fields.

- [ ] **Step 2: Write the failing tests**: `tests/common/mod.rs` (above), plus:

`tests/geom.rs`
```rust
use eframe::egui::{pos2, vec2, Rect};
use fap_core::Pos;
use fap_gui::ui::geom::*;

#[test]
fn cell_at_maps_and_rejects_outside() {
    assert_eq!(cell_at(pos2(25.0, 5.0), 10.0, 4, 3), Some(Pos::new(0, 2)));
    assert_eq!(cell_at(pos2(-1.0, 5.0), 10.0, 4, 3), None);
    assert_eq!(cell_at(pos2(40.0, 5.0), 10.0, 4, 3), None);
    assert_eq!(cell_at_unclamped(pos2(-1.0, 35.0), 10.0), (3, -1));
}

#[test]
fn visible_range_only_covers_clip() {
    let (rows, cols) = visible_range(Rect::from_min_size(pos2(95.0, 15.0), vec2(30.0, 20.0)), 10.0, 300, 30);
    assert_eq!((rows, cols), (1..4, 9..13));
    let (rows, cols) = visible_range(Rect::from_min_size(pos2(-50.0, -50.0), vec2(1e6, 1e6)), 10.0, 300, 30);
    assert_eq!((rows, cols), (0..30, 0..300));
}

#[test]
fn zoom_to_fit_fits_and_clamps() {
    assert_eq!(zoom_to_fit(vec2(600.0, 300.0), 300, 30), 2.0);
    assert_eq!(zoom_to_fit(vec2(10000.0, 10000.0), 2, 2), MAX_CELL_PX);
    assert_eq!(zoom_to_fit(vec2(10.0, 10.0), 300, 300), MIN_CELL_PX);
}
```

`tests/keymap.rs`
```rust
use eframe::egui::{Key, Modifiers};
use fap_gui::model::Mode;
use fap_gui::ui::keymap::{action_for, Action};

const NONE: Modifiers = Modifiers::NONE;
const CMD: Modifiers = Modifiers::COMMAND;

#[test]
fn base_keys() {
    assert_eq!(action_for(Key::P, NONE, Mode::Image), Some(Action::ToolPencil));
    assert_eq!(action_for(Key::P, NONE, Mode::Region), None);
    assert_eq!(action_for(Key::Num0, NONE, Mode::Image), Some(Action::ZoomFit));
    assert_eq!(action_for(Key::S, CMD, Mode::Image), Some(Action::Save));
    assert_eq!(action_for(Key::S, CMD | Modifiers::SHIFT, Mode::Region), Some(Action::SaveAs));
}
```

`tests/document.rs`
```rust
mod common;
use common::*;
use fap_core::{Pos, Project};
use fap_gui::model::{Document, Mode, Tool};

fn doc(rows: &[&str]) -> Document {
    let mut p = Project::new(rows[0].len(), rows.len());
    p.image = img(rows);
    Document::new(p)
}

#[test]
fn pencil_stroke_paints_value_of_first_cell() {
    let mut d = doc(&["#.#."]);
    d.stroke_begin(Pos::new(0, 0));
    d.stroke_continue(Pos::new(0, 1));
    d.stroke_continue(Pos::new(0, 2));
    d.stroke_end();
    assert_eq!(rows_of(&d.project.image), vec!["...."]);
    assert!(d.dirty);
}

#[test]
fn pencil_ignored_in_region_mode() {
    let mut d = doc(&["...."]);
    d.mode = Mode::Region;
    d.stroke_begin(Pos::new(0, 0));
    d.stroke_end();
    assert_eq!(rows_of(&d.project.image), vec!["...."]);
    assert_eq!(Document::new(Project::new(1, 1)).tool, Tool::Pencil);
}
```

- [ ] **Step 3: Run to verify failure**: `nix develop -c cargo test -p fap-gui`. Expected: compile errors.
- [ ] **Step 4: Implement** `model/mod.rs` (Document, Mode, Tool, strokes), `ui/geom.rs`, `ui/keymap.rs`, `ui/app.rs`, `ui/canvas.rs`, `main.rs`.
- [ ] **Step 5: Run to verify pass**: `nix develop -c cargo test -p fap-gui`. Expected: 6 passed.
- [ ] **Step 6: Manual check**: `nix develop -c cargo run -p fap-gui`. Expected:
  - a 30×30 white grid with rulers;
  - click toggles a cell, and dragging paints;
  - Cmd+wheel zooms around the pointer, and grid lines vanish when zoomed far out;
  - heavier lines appear every 5 cells;
  - `0` fits the grid to the window.

  Temporarily change the startup size to 300×300, confirm scrolling and zooming stay smooth, then revert.
- [ ] **Step 7: Commit**: `feat(gui): add editor scaffold with canvas, zoom and pencil`

---

### Task 3: Files: new, open, save, PBM import/export, unsaved-changes guard

**Files:**
- Modify: `crates/fap-gui/src/model/mod.rs`, `src/ui/app.rs`
- Create: `crates/fap-gui/src/ui/dialogs.rs`
- Test: `crates/fap-gui/tests/document.rs` (append)

**Interfaces:**
- Consumes: `fap_core::format::{read_project_file, write_project_file}`, `fap_core::pbm::{read_pbm_file, write_pbm_file}`.
- Produces:
  - `Document::save_to(&mut self, path: &Path) -> Result<(), String>`: writes the project. On success sets `path` and clears `dirty`. On failure leaves both unchanged and returns `"{path}: {err}"`.
  - `Document::open(path: &Path) -> Result<Document, String>` and `Document::import_pbm(path: &Path) -> Result<Document, String>` (a new, untitled, dirty project from the image, with no regions). `Document::export_pbm(&self, path: &Path) -> Result<(), String>`.
  - UI:
    - File menu: New… (dialog: width and height, 1–1000, default 30×30), Open…, Save, Save As…, Import PBM…, Export PBM….
    - Native dialogs come from `rfd::FileDialog`, with filters `*.project.toml` and `*.pbm`.
    - A modal "Save changes?" (Save / Discard / Cancel) guards New, Open, Import and window close. Window close is intercepted via `ctx.input(|i| i.viewport().close_requested())` + `ViewportCommand::CancelClose`.
    - Errors appear in a modal error window.
    - The title bar shows the file name, plus `•` when dirty.

- [ ] **Step 1: Write the failing tests** (append to `tests/document.rs`)

```rust
#[test]
fn dirty_tracking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("d.project.toml");
    let mut d = doc(&["...."]);
    assert!(!d.dirty);
    d.stroke_begin(Pos::new(0, 0)); d.stroke_end();
    assert!(d.dirty);
    d.save_to(&path).unwrap();
    assert!(!d.dirty);
    assert_eq!(d.path.as_deref(), Some(path.as_path()));
    let back = Document::open(&path).unwrap();
    assert_eq!(rows_of(&back.project.image), vec!["#..."]);
    assert!(!back.dirty);
}

#[test]
fn failed_save_stays_dirty() {
    let mut d = doc(&["...."]);
    d.stroke_begin(Pos::new(0, 0)); d.stroke_end();
    let e = d.save_to(std::path::Path::new("/nonexistent-dir/x.project.toml")).unwrap_err();
    assert!(e.contains("/nonexistent-dir/x.project.toml"), "{e}");
    assert!(d.dirty && d.path.is_none());
}

#[test]
fn pbm_import_export() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("i.pbm");
    fap_core::pbm::write_pbm_file(&p, &img(&["#.", ".#"])).unwrap();
    let d = Document::import_pbm(&p).unwrap();
    assert!(d.dirty && d.path.is_none());
    let out = dir.path().join("o.pbm");
    d.export_pbm(&out).unwrap();
    assert_eq!(rows_of(&fap_core::pbm::read_pbm_file(&out).unwrap()), vec!["#.", ".#"]);
}
```

- [ ] **Step 2: Add the dev dep and run to verify failure**: `nix develop -c cargo add -p fap-gui --dev tempfile && nix develop -c cargo test -p fap-gui --test document`. Expected: compile errors (`save_to` etc.).
- [ ] **Step 3: Implement** the Document file methods, `dialogs.rs` (New dialog) and the app's File menu, modal guard and error window.
- [ ] **Step 4: Run to verify pass**: same command. Expected: 5 passed.
- [ ] **Step 5: Manual check**: `nix develop -c cargo run -p fap-gui`. Do each of the following and confirm:
  - draw something, then Save As `t.project.toml`, and the title shows the name without `•`;
  - edit again (the title shows `•`), then close the window, and the "Save changes?" modal appears; Cancel keeps the window open;
  - Open the saved file, and the drawing reappears;
  - Import a PBM made by `fap export-pbm`, and the image matches.
- [ ] **Step 6: Commit**: `feat(gui): add file commands and unsaved-changes guard`

---

### Task 4: Rectangle selection, Select tool, clear/fill/invert

**Files:**
- Create: `crates/fap-gui/src/model/selection.rs`, `src/model/image_ops.rs`
- Modify: `src/model/mod.rs`, `src/ui/canvas.rs`, `src/ui/app.rs`, `src/ui/keymap.rs`
- Test: `crates/fap-gui/tests/selection.rs`, append `tests/keymap.rs`

**Interfaces:**
- Produces:
  - `selection::SetOp { Replace, Add, Remove }`.
  - `selection::Selection` (fields private: `mask: Grid<bool>`, `base: Grid<bool>`, `anchor: Option<Pos>`, `mode: SetOp`; derive `Clone, Debug, PartialEq`):
    - `new(width, height)`, `mask(&self) -> &Grid<bool>`, `is_empty()`, `contains(p)`, `cells(&self) -> Vec<Pos>` (row-major).
    - `bounding_box(&self) -> Option<(Pos, Pos)>` (inclusive corners), `row_span(&self) -> Option<Range<usize>>`, `col_span(&self) -> Option<Range<usize>>`.
    - `click(&mut self, p)`: selects only `p`; anchor = `p`, mode Replace, base = empty.
    - `shift_click(&mut self, p)`: result = `base` ⊕ rect(anchor, p), where ⊕ is replace/union/difference per mode. Without an anchor it behaves as `click`.
    - `drag_to(&mut self, p)`: same as `shift_click` (a drag is `click(start)` then `drag_to` each frame).
    - `apply_cells(&mut self, cells: &[Pos], op: SetOp)`: clears the anchor. `select_all()`, `clear()`, `set_mask(&mut self, mask: Grid<bool>)`.
  - `image_ops::fill(img: &mut Image, sel: &Selection, value: bool)` and `image_ops::invert(img: &mut Image, sel: &Selection)`.
  - `Document.selection: Selection`.
  - `Document::image_edit(&mut self, f: impl FnOnce(&mut Image, &mut Selection)) -> bool`: returns `false` and does nothing in region mode. Otherwise applies `f`; if the image changed, sets dirty (Task 5 adds recording) and returns `true`.
  - Select tool (canvas): mouse-down = `click`, shift-mouse-down = `shift_click`, drag = `drag_to`. The selection is drawn as a translucent blue overlay with an outline.
  - Toolbar buttons Clear / Fill / Invert, plus the same three items in a right-click context menu on the canvas.
  - Keys: `S` → ToolSelect; `C` and `Delete`/`Backspace` → Clear (image mode); `F` → Fill; `I` → Invert; Cmd/Ctrl-`A` → SelectAll; `Escape` → Escape (clears the selection).

- [ ] **Step 1: Write the failing tests**: `tests/selection.rs`

```rust
mod common;
use common::*;
use fap_gui::model::{image_ops, selection::{Selection, SetOp}};

#[test]
fn click_then_shift_click_selects_rectangle() {
    let mut s = Selection::new(6, 6);
    s.click(a1("A1"));
    s.shift_click(a1("D4"));
    assert_eq!(s.cells(), rect("A1", "D4"));
    s.shift_click(a1("B2"));
    assert_eq!(s.cells(), rect("A1", "B2"), "shift-click moves the opposite corner");
}

#[test]
fn drag_selects_rectangle_any_direction() {
    let mut s = Selection::new(6, 6);
    s.click(a1("C5"));
    s.drag_to(a1("B2"));
    assert_eq!(s.cells(), rect("B2", "C5"));
}

#[test]
fn bounding_box_and_spans() {
    let mut s = Selection::new(6, 6);
    s.apply_cells(&[a1("B2"), a1("D3")], SetOp::Replace);
    assert_eq!(s.bounding_box(), Some((a1("B2"), a1("D3"))));
    assert_eq!((s.row_span(), s.col_span()), (Some(1..3), Some(1..4)));
    s.apply_cells(&[a1("B2")], SetOp::Remove);
    assert_eq!(s.cells(), vec![a1("D3")]);
    s.apply_cells(&[a1("A1")], SetOp::Add);
    assert_eq!(s.cells(), vec![a1("A1"), a1("D3")]);
    s.clear();
    assert!(s.is_empty() && s.bounding_box().is_none());
}

#[test]
fn fill_clear_invert_affect_only_selection() {
    let mut i = img(&["#..", "..."]);
    let mut s = Selection::new(3, 2);
    s.click(a1("A1"));
    s.shift_click(a1("B2"));
    image_ops::invert(&mut i, &s);
    assert_eq!(rows_of(&i), vec![".#.", "##."]);
    image_ops::fill(&mut i, &s, false);
    assert_eq!(rows_of(&i), vec!["...", "..."]);
    image_ops::fill(&mut i, &s, true);
    assert_eq!(rows_of(&i), vec!["##.", "##."]);
}
```

Append to `tests/keymap.rs`:
```rust
#[test]
fn selection_keys() {
    assert_eq!(action_for(Key::S, NONE, Mode::Image), Some(Action::ToolSelect));
    assert_eq!(action_for(Key::C, NONE, Mode::Image), Some(Action::Clear));
    assert_eq!(action_for(Key::Delete, NONE, Mode::Image), Some(Action::Clear));
    assert_eq!(action_for(Key::F, NONE, Mode::Image), Some(Action::Fill));
    assert_eq!(action_for(Key::I, NONE, Mode::Image), Some(Action::Invert));
    assert_eq!(action_for(Key::F, NONE, Mode::Region), None);
    assert_eq!(action_for(Key::A, CMD, Mode::Region), Some(Action::SelectAll));
    assert_eq!(action_for(Key::Escape, NONE, Mode::Image), Some(Action::Escape));
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui`. Expected: compile errors.
- [ ] **Step 3: Implement** `selection.rs`, `image_ops.rs`, `Document::image_edit`, the Select tool in the canvas, toolbar, context menu and keys.
- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test -p fap-gui`. Expected: all pass.
- [ ] **Step 5: Manual check**: press `S`, drag a rectangle, press `F`, and the rectangle fills; right-click → Invert works; shift-click extends from the first corner.
- [ ] **Step 6: Commit**: `feat(gui): add rectangle selection and fill/clear/invert`

---

### Task 5: Undo/redo, two stacks

**Files:**
- Create: `crates/fap-gui/src/model/history.rs`
- Modify: `src/model/mod.rs`, `src/ui/app.rs`, `src/ui/keymap.rs`
- Test: `crates/fap-gui/tests/history.rs`, append `tests/document.rs`, `tests/keymap.rs`

**Interfaces:**
- Produces:
  - `history::UNDO_DEPTH: usize = 32`.
  - `history::UndoStack<S>` (derive `Clone, Debug, Default`):
    - `new()`;
    - `push(&mut self, before: S)` clears redo and drops the oldest entry beyond 32;
    - `step_back(&mut self, current: S) -> Result<S, S>` returns `Ok(previous)` and moves `current` to redo, or `Err(current)` when empty;
    - `step_forward(&mut self, current: S) -> Result<S, S>`, symmetric;
    - `map_all(&mut self, f: impl FnMut(&S) -> S)` applies to every undo and redo entry;
    - `can_undo()`, `can_redo()`.
  - `history::ImageEntry { Image(Image), Resize { image: Image, regions: Regions, region_history: UndoStack<Regions> } }`.
  - `history::History { image: UndoStack<ImageEntry>, regions: UndoStack<Regions> }` (derive `Default`):
    - `record_image(&mut self, before: &Image)`, `record_regions(&mut self, before: &Regions)`.
    - `record_resize(&mut self, before: &Project, transform_regions: impl FnMut(&Regions) -> Regions)`: pushes `Resize { image, regions, region_history: <clone of self.regions> }`, **then** `self.regions.map_all(transform_regions)` (spec §5.7).
    - `undo_image(&mut self, p: &mut Project) -> bool` / `redo_image`. An `Image` entry swaps the image. A `Resize` entry swaps image, regions **and** the whole region stack with the saved ones; the current trio goes to the opposite stack as a `Resize` entry. Swapping the saved post-resize state back in is how "redo re-applies the operation".
    - `undo_regions(&mut self, p: &mut Project) -> bool` / `redo_regions`.
  - `Document.history: History`. `image_edit` and strokes record the image on the image stack only if the image actually changed (a stroke records at `stroke_begin` and discards the entry at `stroke_end` if nothing changed). `Document::undo(&mut self) -> bool` / `redo` use the image stack in image mode and the region stack in region mode, set dirty, and reset the selection if the grid size changed.
  - Keys: Cmd/Ctrl-`Z` → Undo, Shift-Cmd/Ctrl-`Z` → Redo. Edit menu: Undo/Redo, greyed when unavailable.

- [ ] **Step 1: Write the failing tests**: `tests/history.rs`

```rust
use fap_core::{Grid, Pos, Project, RegionId, Regions};
use fap_gui::model::history::{History, UndoStack, UNDO_DEPTH};

fn drain_back(s: &mut UndoStack<i32>, mut cur: i32) -> (usize, i32) {
    let mut n = 0;
    loop { match s.step_back(cur) { Ok(p) => { cur = p; n += 1 } Err(c) => return (n, c) } }
}

#[test]
fn stack_caps_at_32_and_push_clears_redo() {
    let mut s = UndoStack::new();
    for i in 0..40 { s.push(i); }
    let (n, cur) = drain_back(&mut s, 40);
    assert_eq!((n, cur), (UNDO_DEPTH, 8));
    assert_eq!(s.step_forward(cur), Ok(9));
    s.push(99);
    assert!(!s.can_redo());
}

fn with_region(r: &Regions) -> Regions {
    let mut r = r.clone();
    let id = r.create_region(r.default_settings());
    r.assign(Pos::new(0, 0), Some(id));
    r
}
/// Stand-in for "insert one unassigned row at the bottom" (the real op arrives in Task 8).
fn grow(r: &Regions) -> Regions {
    let mut rows: Vec<Vec<Option<RegionId>>> = (0..r.height()).map(|i| r.map().row(i).to_vec()).collect();
    rows.push(vec![None; r.width()]);
    Regions::from_parts(Grid::from_rows(rows).unwrap(), r.table().clone(), r.default_level()).unwrap()
}
fn resize(h: &mut History, p: &mut Project) {
    h.record_resize(p, grow);
    let mut img: Vec<Vec<bool>> = (0..p.height()).map(|i| p.image.row(i).to_vec()).collect();
    img.push(vec![false; p.width()]);
    p.image = Grid::from_rows(img).unwrap();
    p.regions = grow(&p.regions);
}

#[test]
fn image_and_region_stacks_are_independent() {
    let (mut p, mut h) = (Project::new(3, 2), History::default());
    h.record_image(&p.image); p.image.set(Pos::new(0, 0), true);
    h.record_regions(&p.regions); p.regions = with_region(&p.regions);
    assert!(h.undo_image(&mut p));
    assert!(!*p.image.get(Pos::new(0, 0)));
    assert_eq!(p.regions.table().len(), 1, "region edit survives image undo");
    assert!(h.undo_regions(&mut p));
    assert!(p.regions.table().is_empty());
    assert!(h.redo_image(&mut p));
    assert!(*p.image.get(Pos::new(0, 0)));
}

#[test]
fn insert_then_undo_keeps_earlier_region_ops_undoable() {
    let (mut p, mut h) = (Project::new(3, 2), History::default());
    h.record_regions(&p.regions); p.regions = with_region(&p.regions);
    resize(&mut h, &mut p);
    assert_eq!(p.height(), 3);
    assert!(h.undo_image(&mut p));
    assert_eq!((p.height(), p.regions.table().len()), (2, 1));
    assert!(h.undo_regions(&mut p));
    assert!(p.regions.table().is_empty());
}

#[test]
fn region_undo_after_resize_uses_resized_snapshots() {
    let (mut p, mut h) = (Project::new(3, 2), History::default());
    h.record_regions(&p.regions); p.regions = with_region(&p.regions);
    resize(&mut h, &mut p);
    assert!(h.undo_regions(&mut p));
    assert_eq!((p.regions.height(), p.regions.table().len()), (3, 0));
    assert!(h.redo_regions(&mut p));
    assert_eq!((p.regions.height(), p.regions.table().len()), (3, 1));
}

#[test]
fn redo_resize_restores_post_resize_state() {
    let (mut p, mut h) = (Project::new(3, 2), History::default());
    resize(&mut h, &mut p);
    let after = p.clone();
    assert!(h.undo_image(&mut p));
    assert!(h.redo_image(&mut p));
    assert_eq!(p, after);
}
```

Append to `tests/document.rs`:
```rust
#[test]
fn pencil_stroke_is_one_undo_step_and_undo_follows_mode() {
    let mut d = doc(&["...."]);
    d.stroke_begin(Pos::new(0, 0)); d.stroke_continue(Pos::new(0, 1)); d.stroke_end();
    assert_eq!(rows_of(&d.project.image), vec!["##.."]);
    d.mode = Mode::Region;
    assert!(!d.undo(), "region stack is empty");
    d.mode = Mode::Image;
    assert!(d.undo());
    assert_eq!(rows_of(&d.project.image), vec!["...."]);
    assert!(d.redo());
    assert_eq!(rows_of(&d.project.image), vec!["##.."]);
}

#[test]
fn no_op_edit_records_nothing() {
    let mut d = doc(&["...."]);
    assert!(!d.image_edit(|_, _| {}));
    assert!(!d.undo());
}
```

Append to `tests/keymap.rs`:
```rust
#[test]
fn undo_keys() {
    assert_eq!(action_for(Key::Z, CMD, Mode::Image), Some(Action::Undo));
    assert_eq!(action_for(Key::Z, CMD | Modifiers::SHIFT, Mode::Region), Some(Action::Redo));
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui`. Expected: compile errors.
- [ ] **Step 3: Implement** `history.rs` and the Document wiring.
- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test -p fap-gui`. Expected: all pass.
- [ ] **Step 5: Manual check**: draw three strokes, Cmd-Z three times, Shift-Cmd-Z twice, and confirm each step matches.
- [ ] **Step 6: Commit**: `feat(gui): add image and region undo stacks`

---

### Task 6: Copy, cut, paste, shade-only paste

**Files:**
- Create: `crates/fap-gui/src/model/clipboard.rs`
- Modify: `src/model/mod.rs`, `src/ui/canvas.rs`, `src/ui/app.rs`, `src/ui/keymap.rs`
- Test: `crates/fap-gui/tests/clipboard.rs`, append `tests/keymap.rs`

**Interfaces:**
- Produces:
  - `clipboard::Clipboard { pub cells: Image, pub mask: Grid<bool> }` (bounding-box sized; derive `Clone, Debug, PartialEq`).
  - `clipboard::PasteMode { Overwrite, ShadeOnly }`.
  - `clipboard::copy(img: &Image, sel: &Selection) -> Option<Clipboard>` (`None` for an empty selection).
  - `clipboard::cut(img: &mut Image, sel: &Selection) -> Option<Clipboard>` (copy, then unshade the selected cells).
  - `clipboard::paste(img: &mut Image, clip: &Clipboard, top_left: (isize, isize), mode: PasteMode) -> Selection`. Writes every masked cell that lands in the grid: Overwrite writes the value; ShadeOnly writes only shaded values. Returns the selection of the landed masked cells.
  - `Document.clipboard: Option<Clipboard>`. When Open/New/Import replaces the Document, `FapApp` moves the old clipboard into the new one, so the clipboard persists.
  - UI paste mode:
    - Cmd/Ctrl-V (or Shift-Cmd/Ctrl-V) enters paste mode. The clipboard outline follows the pointer, with its top-left at the cell under the pointer.
    - A click commits as one image undo step, via `image_edit`, and the selection becomes the pasted cells.
    - `Escape` cancels.
  - Cut is its own undo step.
  - Keys: Cmd/Ctrl-`X`/`C`/`V`, and Shift-Cmd/Ctrl-`V` → PasteShadeOnly. The Edit menu has the same items.

- [ ] **Step 1: Write the failing tests**: `tests/clipboard.rs`

```rust
mod common;
use common::*;
use fap_gui::model::clipboard::{self, PasteMode};
use fap_gui::model::selection::{Selection, SetOp};

fn select(w: usize, h: usize, a: &str, b: &str) -> Selection {
    let mut s = Selection::new(w, h);
    s.click(a1(a));
    s.shift_click(a1(b));
    s
}
// Spec §5.4 worked example: A1..C3 shaded except C3; D4 starts shaded.
fn example() -> fap_core::Image { img(&["###..", "###..", "##...", "...#.", "....."]) }

#[test]
fn cut_paste_user_example() {
    let mut i = example();
    let clip = clipboard::cut(&mut i, &select(5, 5, "A1", "C3")).unwrap();
    clipboard::paste(&mut i, &clip, (1, 1), PasteMode::Overwrite);
    for c in ["A1", "A2", "A3", "B1", "C1", "D4"] { assert!(!*i.get(a1(c)), "{c} should be unshaded"); }
    for c in ["B2", "C2", "D2", "B3", "C3", "D3", "B4", "C4"] { assert!(*i.get(a1(c)), "{c} should be shaded"); }
}

#[test]
fn copy_paste_user_example() {
    let mut i = example();
    let clip = clipboard::copy(&i, &select(5, 5, "A1", "C3")).unwrap();
    clipboard::paste(&mut i, &clip, (1, 1), PasteMode::Overwrite);
    for c in ["A1", "A2", "A3", "B1", "C1"] { assert!(*i.get(a1(c)), "{c} should stay shaded"); }
    assert!(!*i.get(a1("D4")));
}

#[test]
fn shade_only_paste_never_unshades() {
    let mut i = img(&["#.", "##"]);
    let clip = clipboard::copy(&i, &select(2, 2, "A1", "B1")).unwrap(); // "#."
    clipboard::paste(&mut i, &clip, (1, 0), PasteMode::ShadeOnly);
    assert_eq!(rows_of(&i), vec!["#.", "##"]);
}

#[test]
fn non_rectangular_mask_only_writes_masked_cells() {
    let mut i = img(&["##", "#.", ".#"]);
    let mut s = Selection::new(2, 3);
    s.apply_cells(&[a1("A1"), a1("B1"), a1("A2")], SetOp::Replace); // L shape, all shaded
    let clip = clipboard::copy(&i, &s).unwrap();
    clipboard::paste(&mut i, &clip, (1, 0), PasteMode::Overwrite); // B3 is outside the mask
    assert_eq!(rows_of(&i), vec!["##", "##", "##"]);
}

#[test]
fn paste_clips_at_edges() {
    let mut i = img(&["...", "...", "..."]);
    let clip = clipboard::copy(&img(&["##", "##"]), &select(2, 2, "A1", "B2")).unwrap();
    let landed = clipboard::paste(&mut i, &clip, (-1, 2), PasteMode::Overwrite);
    assert_eq!(rows_of(&i), vec!["..#", "...", "..."]);
    assert_eq!(cells(landed.mask()), vec![a1("C1")]);
    clipboard::paste(&mut i, &clip, (5, 5), PasteMode::Overwrite); // entirely off-grid
    assert_eq!(rows_of(&i), vec!["..#", "...", "..."]);
}

#[test]
fn empty_selection_copies_nothing() {
    assert!(clipboard::copy(&img(&[".."]), &Selection::new(2, 1)).is_none());
}
```

Append to `tests/keymap.rs`:
```rust
#[test]
fn clipboard_keys() {
    assert_eq!(action_for(Key::X, CMD, Mode::Image), Some(Action::Cut));
    assert_eq!(action_for(Key::C, CMD, Mode::Image), Some(Action::Copy));
    assert_eq!(action_for(Key::V, CMD, Mode::Image), Some(Action::Paste));
    assert_eq!(action_for(Key::V, CMD | Modifiers::SHIFT, Mode::Image), Some(Action::PasteShadeOnly));
    assert_eq!(action_for(Key::V, CMD, Mode::Region), None);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui`. Expected: compile errors.
- [ ] **Step 3: Implement** `clipboard.rs`, the paste-mode canvas state and keys.
- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test -p fap-gui`. Expected: all pass.
- [ ] **Step 5: Manual check**: reproduce the spec §5.4 example on screen with cut, then with copy. Shift-Cmd-V over a shaded area leaves it shaded. Esc cancels a pending paste.
- [ ] **Step 6: Commit**: `feat(gui): add copy, cut, paste and shade-only paste`

---

### Task 7: Drag-move

**Files:**
- Modify: `crates/fap-gui/src/model/clipboard.rs`, `src/ui/canvas.rs`
- Test: `crates/fap-gui/tests/clipboard.rs` (append)

**Interfaces:**
- Produces: `clipboard::drag_move(img: &mut Image, sel: &Selection, d_row: isize, d_col: isize) -> Selection`. It equals `copy` → unshade the source → `paste(Overwrite)` at the offset, and returns the landed selection. The clipboard is not touched. In the Select tool (and, from Task 10, the Wand), a mouse-down inside the selection starts a move instead of a new selection. A ghost outline follows the drag, and release commits as one `image_edit`. Only image mode.

- [ ] **Step 1: Write the failing tests** (append)

```rust
#[test]
fn drag_move_overlapping_takes_moved_values() {
    let mut i = img(&["##.", "..."]);
    let moved = clipboard::drag_move(&mut i, &select(3, 2, "A1", "B1"), 0, 1);
    assert_eq!(rows_of(&i), vec![".##", "..."]);
    assert_eq!(cells(moved.mask()), vec![a1("B1"), a1("C1")]);
}

#[test]
fn drag_move_clips_at_edges() {
    let mut i = img(&["##.", "..."]);
    let moved = clipboard::drag_move(&mut i, &select(3, 2, "A1", "B1"), 1, 2);
    assert_eq!(rows_of(&i), vec!["...", "..#"]);
    assert_eq!(cells(moved.mask()), vec![a1("C2")]);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui --test clipboard`. Expected: compile error, `drag_move` not found.
- [ ] **Step 3: Implement** `drag_move` and the canvas move gesture.
- [ ] **Step 4: Run to verify pass**: same command. Expected: all pass.
- [ ] **Step 5: Manual check**: select an icon and drag it 3 cells right: it moves, the source clears, one Cmd-Z restores it, and Cmd-V still pastes the previous clipboard.
- [ ] **Step 6: Commit**: `feat(gui): add drag-move of selections`

---

### Task 8: Row/column insert and delete

**Files:**
- Create: `crates/fap-gui/src/model/structure.rs`
- Modify: `src/model/mod.rs`, `src/ui/dialogs.rs`, `src/ui/app.rs`
- Test: `crates/fap-gui/tests/structure.rs`, append `tests/document.rs`

**Interfaces:**
- Produces:
  - `structure::Axis { Rows, Cols }`. "Before" means above for rows and left for columns.
  - `structure::ImageFill { Shaded, Unshaded, CopyBefore, CopyAfter }` and `structure::RegionFill { NoRegion, InheritBefore, InheritAfter, NewRegion }`.
  - `structure::ResizeOp { Insert { axis: Axis, at: usize, count: usize, image_fill: ImageFill, region_fill: RegionFill }, Delete { axis: Axis, start: usize, end: usize }, Rescale { width: usize, height: usize } }` (`end` exclusive; all derive `Clone, Copy, Debug, PartialEq, Eq`).
  - `ResizeOp::apply_image(&self, img: &Image) -> Result<Image, String>` and `ResizeOp::apply_regions(&self, r: &Regions) -> Result<Regions, String>`. Both run the same validation:
    - count 0 → `"count must be at least 1"`;
    - `at > len` → `"insertion point out of range"`;
    - CopyBefore/InheritBefore at 0 → `"nothing before the insertion point to copy"`;
    - CopyAfter/InheritAfter at `len` → `"nothing after the insertion point to copy"`;
    - empty or out-of-range delete → `"nothing to delete"`;
    - deleting all → `"cannot delete every row"` / `"cannot delete every column"`;
    - rescale to 0 → `"size must be at least 1×1"`.

    Copies take the single line adjacent to the insertion point. NewRegion makes all inserted cells one region with `default_settings()`. Region results go through `Regions::from_parts` (split rule; regions with no cells left are deleted). `Rescale` delegates to `fap_core::rescale` (dialog in Task 12).
  - `Document::resize(&mut self, op: &ResizeOp) -> Result<(), String>`: computes both results first, so a failure changes nothing. Then calls `history.record_resize(&project, |r| op.apply_regions(r).expect("same-size snapshot"))`, applies, resets the selection to the new size and sets dirty. It works in either mode.
  - UI:
    - Structure menu: Insert Rows…, Insert Columns…, Delete Rows…, Delete Columns….
    - The insert dialog has: count, position as a 1-based "before row/column N" (N = len+1 means at the end), image fill and region fill. Options that aren't allowed at the chosen position are greyed out.
    - The delete dialog is pre-filled from `selection.row_span()`/`col_span()`, or asks for an inclusive 1-based range.
    - Errors show in the error window.

- [ ] **Step 1: Write the failing tests**: `tests/structure.rs`

```rust
mod common;
use common::*;
use fap_core::{Pos, RegionId};
use fap_gui::model::structure::{Axis, ImageFill, RegionFill, ResizeOp};

fn ins(axis: Axis, at: usize, count: usize, i: ImageFill, r: RegionFill) -> ResizeOp {
    ResizeOp::Insert { axis, at, count, image_fill: i, region_fill: r }
}

#[test]
fn insert_rows_with_each_image_fill() {
    let i = img(&["#.", ".#"]);
    let op = |f| ins(Axis::Rows, 1, 2, f, RegionFill::NoRegion);
    assert_eq!(rows_of(&op(ImageFill::CopyBefore).apply_image(&i).unwrap()), vec!["#.", "#.", "#.", ".#"]);
    assert_eq!(rows_of(&op(ImageFill::CopyAfter).apply_image(&i).unwrap()), vec!["#.", ".#", ".#", ".#"]);
    assert_eq!(rows_of(&op(ImageFill::Shaded).apply_image(&i).unwrap()), vec!["#.", "##", "##", ".#"]);
    assert_eq!(rows_of(&op(ImageFill::Unshaded).apply_image(&i).unwrap()), vec!["#.", "..", "..", ".#"]);
}

#[test]
fn insert_columns_is_symmetric() {
    let i = img(&["#.", ".#"]);
    let out = ins(Axis::Cols, 2, 1, ImageFill::CopyBefore, RegionFill::NoRegion).apply_image(&i).unwrap();
    assert_eq!(rows_of(&out), vec!["#..", ".##"]);
}

#[test]
fn zero_count_and_bad_positions_refused() {
    let i = img(&["#.", ".#"]);
    let e = |op: ResizeOp| op.apply_image(&i).unwrap_err();
    assert!(e(ins(Axis::Rows, 0, 0, ImageFill::Shaded, RegionFill::NoRegion)).contains("at least 1"));
    assert!(e(ins(Axis::Rows, 3, 1, ImageFill::Shaded, RegionFill::NoRegion)).contains("out of range"));
    assert!(e(ins(Axis::Rows, 0, 1, ImageFill::CopyBefore, RegionFill::NoRegion)).contains("nothing before"));
    assert!(e(ins(Axis::Cols, 2, 1, ImageFill::CopyAfter, RegionFill::NoRegion)).contains("nothing after"));
    let r = regions_from(&["1 1", "1 1"]);
    assert!(ins(Axis::Rows, 0, 1, ImageFill::Shaded, RegionFill::InheritBefore).apply_regions(&r).is_err());
}

#[test]
fn inherit_never_splits_but_no_region_and_new_region_can() {
    let r = regions_from(&["1 1", "1 1"]);
    let inherit = ins(Axis::Rows, 1, 1, ImageFill::Unshaded, RegionFill::InheritBefore).apply_regions(&r).unwrap();
    assert_eq!(inherit.table().len(), 1);
    assert_eq!(inherit.region_at(Pos::new(1, 0)), inherit.region_at(Pos::new(0, 0)));
    let none = ins(Axis::Rows, 1, 1, ImageFill::Unshaded, RegionFill::NoRegion).apply_regions(&r).unwrap();
    assert_eq!(none.table().len(), 2);
    assert_eq!(none.region_at(Pos::new(1, 1)), None);
    let new = ins(Axis::Rows, 1, 2, ImageFill::Unshaded, RegionFill::NewRegion).apply_regions(&r).unwrap();
    assert_eq!(new.table().len(), 3);
    let band = new.region_at(Pos::new(1, 0)).unwrap();
    assert_eq!(new.cells(band).len(), 4);
    assert_eq!(new.settings(band), Some(&new.default_settings()));
}

#[test]
fn delete_removes_regions_that_lose_all_cells() {
    // Region 2 is a single cell; the two halves of "1" are split by from_parts but region 2 keeps its ID.
    let r = regions_from(&["1 1 1", ". 2 .", "1 1 1"]);
    let out = ResizeOp::Delete { axis: Axis::Rows, start: 1, end: 2 }.apply_regions(&r).unwrap();
    assert_eq!(out.height(), 2);
    assert!(out.settings(RegionId(2)).is_none());
    assert!(ResizeOp::Delete { axis: Axis::Cols, start: 0, end: 3 }.apply_regions(&r).unwrap_err().contains("every column"));
}

#[test]
fn delete_everything_is_refused() {
    let i = img(&["#.", ".#"]);
    assert!(ResizeOp::Delete { axis: Axis::Rows, start: 0, end: 2 }.apply_image(&i).unwrap_err().contains("every row"));
    assert!(ResizeOp::Delete { axis: Axis::Rows, start: 1, end: 1 }.apply_image(&i).unwrap_err().contains("nothing to delete"));
    assert!(ResizeOp::Rescale { width: 0, height: 2 }.apply_image(&i).unwrap_err().contains("1×1"));
}
```

Append to `tests/document.rs`:
```rust
#[test]
fn resize_records_one_image_undo_step_and_resets_selection() {
    use fap_gui::model::structure::{Axis, ImageFill, RegionFill, ResizeOp};
    let mut d = doc(&["#.", ".#"]);
    d.selection.select_all();
    d.resize(&ResizeOp::Insert { axis: Axis::Rows, at: 2, count: 1, image_fill: ImageFill::CopyBefore, region_fill: RegionFill::NoRegion }).unwrap();
    assert_eq!(rows_of(&d.project.image), vec!["#.", ".#", ".#"]);
    assert!(d.selection.is_empty() && d.selection.mask().height() == 3);
    assert!(d.undo());
    assert_eq!(rows_of(&d.project.image), vec!["#.", ".#"]);
    assert!(d.resize(&ResizeOp::Delete { axis: Axis::Rows, start: 0, end: 2 }).is_err());
    assert_eq!(d.project.height(), 2, "failed resize changes nothing");
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui`. Expected: compile errors.
- [ ] **Step 3: Implement** `structure.rs`, `Document::resize` and the dialogs and menu.
- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test -p fap-gui`. Expected: all pass.
- [ ] **Step 5: Manual check**: insert 3 rows before row 1 with "copy below", and the top rows duplicate; "copy above" is greyed at row 1; deleting rows from a selection works; Cmd-Z restores.
- [ ] **Step 6: Commit**: `feat(gui): add row and column insertion and deletion`

---

### Task 9: Non-rectangular selection (Cmd/Ctrl-click)

**Files:**
- Modify: `crates/fap-gui/src/model/selection.rs`, `src/ui/canvas.rs`
- Test: `crates/fap-gui/tests/selection.rs` (append)

**Interfaces:**
- Produces:
  - `Selection::command_click(&mut self, p: Pos)`: sets `base` = the current mask and anchor = `p`, with mode Add if `p` is unselected, otherwise Remove. It then applies rect(p, p) at once, so the cell toggles. A following `shift_click`/`drag_to` re-applies `base ⊕ rect(anchor, q)`.
  - Canvas (Select tool): Cmd/Ctrl-mouse-down → `command_click`. Cmd/Ctrl-drag → `command_click(start)` then `drag_to`.

- [ ] **Step 1: Write the failing tests** (append; spec §5.3 examples)

```rust
fn a1_d4() -> Selection {
    let mut s = Selection::new(6, 6);
    s.click(a1("A1"));
    s.shift_click(a1("D4"));
    s
}
fn minus(a: Vec<fap_core::Pos>, b: Vec<fap_core::Pos>) -> Vec<fap_core::Pos> {
    a.into_iter().filter(|p| !b.contains(p)).collect()
}

#[test]
fn command_click_adds_rectangle_user_example() {
    let mut s = a1_d4();
    s.command_click(a1("E3"));
    s.shift_click(a1("C2"));
    let mut expected = rect("A1", "D4");
    expected.extend([a1("E2"), a1("E3")]);
    expected.sort();
    assert_eq!(s.cells(), expected);
}

#[test]
fn command_click_on_selected_cell_removes_overlap() {
    let mut s = a1_d4();
    s.command_click(a1("C2"));
    s.shift_click(a1("D3"));
    assert_eq!(s.cells(), minus(rect("A1", "D4"), rect("C2", "D3")));
    s.shift_click(a1("E4"));
    assert_eq!(s.cells(), minus(rect("A1", "D4"), rect("C2", "D4")));
}

#[test]
fn command_click_alone_toggles_cell() {
    let mut s = Selection::new(4, 4);
    s.click(a1("A1"));
    s.command_click(a1("C3"));
    assert_eq!(s.cells(), vec![a1("A1"), a1("C3")]);
    s.command_click(a1("C3"));
    assert_eq!(s.cells(), vec![a1("A1")]);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui --test selection`. Expected: compile error, `command_click` not found.
- [ ] **Step 3: Implement** `command_click` and the canvas modifiers.
- [ ] **Step 4: Run to verify pass**: same command. Expected: all pass.
- [ ] **Step 5: Manual check**: reproduce both spec §5.3 examples on screen.
- [ ] **Step 6: Commit**: `feat(gui): add spreadsheet-style non-rectangular selection`

---

### Task 10: Magic wand

**Files:**
- Create: `crates/fap-gui/src/model/components.rs`
- Modify: `src/model/mod.rs`, `src/ui/canvas.rs`, `src/ui/app.rs`, `src/ui/keymap.rs`
- Test: `crates/fap-gui/tests/components.rs`, append `tests/keymap.rs`

**Interfaces:**
- Produces:
  - `components::Connectivity { Four, Eight }`.
  - `components::image_component(img: &Image, p: Pos, conn: Connectivity) -> Vec<Pos>`: the cells with the same value as `p`, connected to it, row-major.
  - `components::region_component(r: &Regions, p: Pos) -> Vec<Pos>`: the region's cells if `p` is assigned, otherwise the 4-connected unassigned area containing `p`.
  - `components::components(mask: &Grid<bool>) -> Vec<Vec<Pos>>`: 4-connected components of the true cells, ordered by each one's first cell, each row-major.
  - `Document.connectivity: Connectivity` (default `Four`).
  - Wand tool (image mode): a click calls `selection.apply_cells(image_component(..), Replace)`; Shift-click uses `Add` and Alt-click uses `Remove`. Dragging from inside the selection moves it (Task 7's gesture). The toolbar has a 4/8 toggle shown while the wand is active. Region-mode behaviour comes in Task 15.
  - Key: `W` → ToolWand (both modes).

- [ ] **Step 1: Write the failing tests**: `tests/components.rs`

```rust
mod common;
use common::*;
use fap_gui::model::components::{components, image_component, region_component, Connectivity};

#[test]
fn shaded_component_four_vs_eight() {
    let i = img(&["#..", ".#.", "..#"]);
    assert_eq!(image_component(&i, a1("A1"), Connectivity::Four), vec![a1("A1")]);
    assert_eq!(image_component(&i, a1("A1"), Connectivity::Eight), vec![a1("A1"), a1("B2"), a1("C3")]);
}

#[test]
fn unshaded_component() {
    let i = img(&["#.", "#."]);
    assert_eq!(image_component(&i, a1("B1"), Connectivity::Four), vec![a1("B1"), a1("B2")]);
}

#[test]
fn region_component_is_region_or_unassigned_area() {
    let r = regions_from(&["1 1 .", ". . .", "2 . 2"]); // the two 2-cells get split; irrelevant here
    assert_eq!(region_component(&r, a1("A1")), vec![a1("A1"), a1("B1")]);
    assert_eq!(region_component(&r, a1("C1")), vec![a1("C1"), a1("A2"), a1("B2"), a1("C2"), a1("B3")]);
}

#[test]
fn components_of_selection() {
    let mask = img(&["#.#", "#.."]);
    assert_eq!(components(&mask), vec![vec![a1("A1"), a1("A2")], vec![a1("C1")]]);
}
```

Append to `tests/keymap.rs`:
```rust
#[test]
fn wand_key() {
    assert_eq!(action_for(Key::W, NONE, Mode::Image), Some(Action::ToolWand));
    assert_eq!(action_for(Key::W, NONE, Mode::Region), Some(Action::ToolWand));
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui`. Expected: compile errors.
- [ ] **Step 3: Implement** `components.rs` and the wand tool.
- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test -p fap-gui`. Expected: all pass.
- [ ] **Step 5: Manual check**: press `W`, click an icon, shift-click a second icon, and drag both 2 cells down. Alt-click removes one. The 8-neighbour toggle picks up a diagonal line.
- [ ] **Step 6: Commit**: `feat(gui): add magic wand selection`

---

### Task 11: Save selection as PBM, paste from file

**Files:**
- Modify: `crates/fap-gui/src/model/clipboard.rs`, `src/ui/app.rs`
- Test: `crates/fap-gui/tests/clipboard.rs` (append)

**Interfaces:**
- Produces:
  - `clipboard::selection_image(img: &Image, sel: &Selection) -> Option<Image>`: bounding box, with cells outside the mask unshaded.
  - `clipboard::from_image(img: Image) -> Clipboard`: every mask cell true.
  - File menu: "Save Selection as PBM…" (disabled when the selection is empty) and "Paste from File…" (loads a PBM, sets `Document.clipboard`, then enters paste mode as in Task 6).

- [ ] **Step 1: Write the failing tests** (append)

```rust
#[test]
fn selection_image_blanks_unselected_bbox_cells() {
    let i = img(&["##", "##"]);
    let mut s = Selection::new(2, 2);
    s.apply_cells(&[a1("A1"), a1("B1"), a1("A2")], SetOp::Replace);
    assert_eq!(rows_of(&clipboard::selection_image(&i, &s).unwrap()), vec!["##", "#."]);
    assert!(clipboard::selection_image(&i, &Selection::new(2, 2)).is_none());
}

#[test]
fn from_image_masks_everything() {
    let c = clipboard::from_image(img(&["#.", ".."]));
    assert!(c.mask.positions().all(|p| *c.mask.get(p)));
    let mut target = img(&["##", "##"]);
    clipboard::paste(&mut target, &c, (0, 0), PasteMode::Overwrite);
    assert_eq!(rows_of(&target), vec!["#.", ".."]);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui --test clipboard`. Expected: compile errors.
- [ ] **Step 3: Implement** both functions and the two menu items.
- [ ] **Step 4: Run to verify pass**: same command. Expected: all pass.
- [ ] **Step 5: Manual check**: save a wand-selected icon as `icon.pbm`, open a different project, choose Paste from File, place it, and it appears.
- [ ] **Step 6: Commit**: `feat(gui): add selection export and paste from file`

---

### Task 12: Rescale dialog

**Files:**
- Modify: `crates/fap-gui/src/ui/dialogs.rs`, `src/ui/app.rs`
- Test: `crates/fap-gui/tests/structure.rs` (append)

**Interfaces:**
- Consumes: `ResizeOp::Rescale` (Task 8), which delegates to `fap_core::rescale` (Task 1).
- Produces: Structure → "Rescale…" dialog with width, height, and an aspect-lock checkbox (on by default). With the lock on, editing one field sets the other to `max(1, round(other_original · new / original))`. OK calls `Document::resize(&ResizeOp::Rescale { .. })`.

- [ ] **Step 1: Write the test** (append)

```rust
#[test]
fn rescale_op_maps_image_and_regions() {
    let op = ResizeOp::Rescale { width: 4, height: 2 };
    assert_eq!(rows_of(&op.apply_image(&img(&["#."])).unwrap()), vec!["##..", "##.."]);
    let r = op.apply_regions(&regions_from(&["1 ."])).unwrap();
    assert_eq!((r.width(), r.height(), r.cells(RegionId(1)).len()), (4, 2, 4));
}
```

- [ ] **Step 2: Run** `nix develop -c cargo test -p fap-gui --test structure`. Expected: passes if Task 8 fully wired `Rescale`; if it fails, finish the `Rescale` arms of `apply_image`/`apply_regions` now.
- [ ] **Step 3: Implement** the dialog and menu item.
- [ ] **Step 4: Run to verify pass**: same command. Expected: all pass.
- [ ] **Step 5: Manual check**: rescale a 30×30 drawing to 60×60 (shapes double) and back to 30×30 (the original returns); Cmd-Z undoes each step.
- [ ] **Step 6: Commit**: `feat(gui): add rescale dialog`

---

### Task 13: Region labels and six-colouring (pure)

**Files:**
- Create: `crates/fap-gui/src/model/labels.rs`, `src/model/coloring.rs`
- Modify: `src/model/mod.rs`
- Test: `crates/fap-gui/tests/labels.rs`, `tests/coloring.rs`

**Interfaces:**
- Produces:
  - `labels::RegionLabel { pub id: RegionId, pub number: usize, pub at: Pos }`.
  - `labels::labels(r: &Regions) -> Vec<RegionLabel>`, sorted by `number` (1..=n). The ordering key (spec §5.6), with x = col and y = row:
    1. centroid x + y (as f64);
    2. centroid x;
    3. the minimum x + y over the region's cells;
    4. the minimum x;
    5. the `RegionId` (for determinism).

    `at` is the cell `(round(cy), round(cx))` if it belongs to the region. Otherwise it's the region cell nearest the centroid by Euclidean distance, with ties going to the row-major first.
  - `coloring::COLORS: u8 = 6` and `coloring::Adjacency = BTreeMap<RegionId, BTreeSet<RegionId>>`.
  - `coloring::adjacency(r: &Regions) -> Adjacency`: every region is a key, and two regions border if any of their cells are 8-neighbours.
  - `coloring::color_graph(adj: &Adjacency, sizes: &BTreeMap<RegionId, usize>, previous: &HashMap<RegionId, u8>) -> Option<HashMap<RegionId, u8>>`:
    1. Keep each `previous` colour (< 6) that doesn't clash with an already-kept neighbour, processing regions in ID order.
    2. Colour the rest greedily in DSatur order (most distinct neighbour colours first, ties to the larger region, then lower ID). Each picks the allowed colour with the fewest total cells so far, ties to the lower index.
    3. If greedy gets stuck, run a backtracking search over all regions. It tries each region's current colour first, then colours by ascending usage.
    4. Return `None` only if no 6-colouring exists.
  - `coloring::recolor(r: &Regions, previous: &HashMap<RegionId, u8>) -> HashMap<RegionId, u8>`: calls `color_graph`. If that returns `None`, which can't happen for grid maps (spec §5.6), it falls back to `id % 6`.

- [ ] **Step 1: Write the failing tests**

`tests/labels.rs`
```rust
mod common;
use common::*;
use fap_gui::model::labels::labels;

#[test]
fn numbered_top_left_to_bottom_right() {
    let r = regions_from(&["1 1 . .", "1 1 . .", ". . 2 2", ". . 2 2"]);
    let l = labels(&r);
    assert_eq!(l.iter().map(|x| (x.id.0, x.number)).collect::<Vec<_>>(), vec![(1, 1), (2, 2)]);
}

#[test]
fn equal_sums_tie_broken_by_centroid_x() {
    let r = regions_from(&[". . 1", ". . .", "2 . ."]); // centroids (x2,y0) and (x0,y2)
    let l = labels(&r);
    assert_eq!((l[0].id.0, l[0].number), (2, 1));
}

#[test]
fn concave_region_label_on_nearest_cell() {
    // U shape: centroid (x1, y8/7) rounds to B2, which is not in the region; nearest region cell is B3.
    let r = regions_from(&["1 . 1", "1 . 1", "1 1 1"]);
    assert_eq!(labels(&r)[0].at, a1("B3"));
}
```

`tests/coloring.rs`
```rust
mod common;
use common::*;
use fap_core::{RegionId, Regions};
use fap_gui::model::coloring::{adjacency, color_graph, recolor, Adjacency, COLORS};
use std::collections::{BTreeMap, BTreeSet, HashMap};

fn valid(adj: &Adjacency, c: &HashMap<RegionId, u8>) -> bool {
    adj.iter().all(|(a, ns)| c[a] < COLORS && ns.iter().all(|b| c[a] != c[b]))
}
fn complete(n: u32) -> Adjacency {
    (1..=n).map(|i| (RegionId(i), (1..=n).filter(|&j| j != i).map(RegionId).collect())).collect()
}
fn ones(adj: &Adjacency) -> BTreeMap<RegionId, usize> { adj.keys().map(|k| (*k, 1)).collect() }

#[test]
fn corner_contact_counts_as_bordering() {
    let r = regions_from(&["1 .", ". 2"]);
    assert!(adjacency(&r)[&RegionId(1)].contains(&RegionId(2)));
}

#[test]
fn strips_colour_validly_and_keep_valid_previous_colours() {
    let r = Regions::strips(30, 3, 3, 1, 4, 2, 2);
    let c = recolor(&r, &HashMap::new());
    assert!(valid(&adjacency(&r), &c));
    assert_eq!(recolor(&r, &c), c);
}

#[test]
fn prefers_least_used_colour() {
    let adj: Adjacency = [(RegionId(1), BTreeSet::new()), (RegionId(2), BTreeSet::new())].into();
    let sizes = BTreeMap::from([(RegionId(1), 10), (RegionId(2), 1)]);
    let c = color_graph(&adj, &sizes, &HashMap::from([(RegionId(1), 0)])).unwrap();
    assert_ne!(c[&RegionId(2)], 0, "colour 0 already covers 10 cells");
}

#[test]
fn backtracking_rescues_stuck_greedy() {
    // Hub 7 borders six mutually non-adjacent leaves pre-coloured 0..5.
    let mut adj: Adjacency = (1..=6).map(|i| (RegionId(i), BTreeSet::from([RegionId(7)]))).collect();
    adj.insert(RegionId(7), (1..=6).map(RegionId).collect());
    let prev: HashMap<_, _> = (1..=6).map(|i| (RegionId(i), (i - 1) as u8)).collect();
    assert!(valid(&adj, &color_graph(&adj, &ones(&adj), &prev).unwrap()));
}

#[test]
fn k6_colours_and_k7_does_not() {
    let k6 = complete(6);
    assert!(valid(&k6, &color_graph(&k6, &ones(&k6), &HashMap::new()).unwrap()));
    let k7 = complete(7);
    assert!(color_graph(&k7, &ones(&k7), &HashMap::new()).is_none());
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui --test labels --test coloring`. Expected: compile errors.
- [ ] **Step 3: Implement** `labels.rs` and `coloring.rs`.
- [ ] **Step 4: Run to verify pass**: same command. Expected: 8 passed.
- [ ] **Step 5: Commit**: `feat(gui): add region labels and six-colouring`

---

### Task 14: Region operations (pure)

**Files:**
- Create: `crates/fap-gui/src/model/region_ops.rs`
- Modify: `src/model/mod.rs`
- Test: `crates/fap-gui/tests/region_ops.rs`

**Interfaces:**
- Consumes: `components::components` (Task 10), Plan 0's `Regions::{create_region, assign, normalize, cells, default_settings}`.
- Produces:
  - `region_ops::new_regions(r: &mut Regions, sel: &Grid<bool>) -> Vec<RegionId>`: one new region per 4-connected component of `sel`, with `default_settings()`. The cells are taken from their previous regions, then `normalize()` runs (donors may split). Returns the new IDs in component order.
  - `region_ops::merge_into(r: &mut Regions, sel: &Grid<bool>, target: RegionId) -> Result<(), String>`. If `target`'s cells ∪ `sel` isn't 4-connected, it returns `Err("merged region would not be connected")` and leaves `r` untouched. Otherwise it assigns the cells and runs `normalize()`.
  - `region_ops::remove_from_regions(r: &mut Regions, sel: &Grid<bool>)`: the cells become unassigned, then `normalize()` runs.

- [ ] **Step 1: Write the failing tests**: `tests/region_ops.rs`

```rust
mod common;
use common::*;
use fap_core::RegionId;
use fap_gui::model::region_ops::{merge_into, new_regions, remove_from_regions};

#[test]
fn new_region_per_component() {
    let mut r = regions_from(&[". . . ."]);
    let ids = new_regions(&mut r, &img(&["#.##"]));
    assert_eq!(ids.len(), 2);
    assert_eq!(r.cells(ids[1]), vec![a1("C1"), a1("D1")]);
    assert_eq!(r.settings(ids[0]), Some(&r.default_settings()));
}

#[test]
fn new_region_can_split_donor() {
    let mut r = regions_from(&["1 1 1"]);
    new_regions(&mut r, &img(&[".#."]));
    assert_eq!(r.table().len(), 3);
}

#[test]
fn merge_accepts_pieces_on_both_sides() {
    let mut r = regions_from(&[". 1 .", ". . ."]);
    merge_into(&mut r, &img(&["#.#", "..."]), RegionId(1)).unwrap();
    assert_eq!(r.cells(RegionId(1)).len(), 3);
}

#[test]
fn merge_refuses_disconnected_result() {
    let mut r = regions_from(&[". 1 .", ". . .", ". . ."]);
    let before = r.clone();
    let e = merge_into(&mut r, &img(&["...", "...", "#.."]), RegionId(1)).unwrap_err();
    assert!(e.contains("not be connected"));
    assert_eq!(r, before, "failed merge is a no-op");
}

#[test]
fn remove_last_cell_deletes_region() {
    let mut r = regions_from(&["1 2"]);
    remove_from_regions(&mut r, &img(&[".#"]));
    assert_eq!(r.table().keys().copied().collect::<Vec<_>>(), vec![RegionId(1)]);
    assert_eq!(r.region_at(a1("B1")), None);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui --test region_ops`. Expected: compile errors.
- [ ] **Step 3: Implement** `region_ops.rs`.
- [ ] **Step 4: Run to verify pass**: same command. Expected: 5 passed.
- [ ] **Step 5: Commit**: `feat(gui): add new, merge and remove region operations`

---

### Task 15: Region mode UI

**Files:**
- Create: `crates/fap-gui/src/ui/region_panel.rs`
- Modify: `src/model/mod.rs`, `src/ui/canvas.rs`, `src/ui/app.rs`, `src/ui/keymap.rs`
- Test: append `tests/document.rs`, `tests/keymap.rs`

**Interfaces:**
- Consumes: Tasks 10, 13, 14.
- Produces:
  - `Document.colors: HashMap<RegionId, u8>`, refreshed with `coloring::recolor(&regions, &colors)` on entering region mode and after every region edit, region undo/redo, or resize.
  - `Document.problem_cells: Option<Grid<bool>>`, drawn as a red hatch overlay when View → "Problems" is on. It's populated by Plan 4, and this task only draws it.
  - `Document::toggle_region_mode(&mut self)`: entering remembers the image tool, switching Pencil to Select. Leaving restores the remembered tool.
  - `Document::region_edit(&mut self, f: impl FnOnce(&mut Regions, &Selection) -> Result<(), String>) -> Result<(), String>`: works in either mode. On `Ok` with a change it records on the region stack, sets dirty and recolours; on `Err` the regions are unchanged.
  - Canvas in region mode:
    - each assigned cell is tinted `PALETTE[color]` at alpha 128; the palette is six distinct hues, the builder's choice;
    - borders between different regions are drawn 2 px dark;
    - each label number is drawn at `RegionLabel.at`;
    - the image stays visible underneath and can't be edited.
  - Wand in region mode: `region_component` with Replace/Add (Shift)/Remove (Alt).
  - Region panel (right side panel, visible in region mode):
    - the project default level (1–4);
    - a list of `labels()` in number order, showing "N: level L, margin D, name". Clicking a row selects the region's cells and scrolls the canvas so the label cell is centred.
  - Property toolbar: shown when the selection is non-empty and all of it lies in one region. It edits that region's level (1–4), margin (0–2) and name; each committed change is one `region_edit`.
  - Buttons and keys:
    - New Region (`N`) → `new_regions`;
    - Merge (`M`) enters merge mode (crosshair cursor, status "click a region to merge into"): the next click on an assigned cell calls `merge_into`, and an `Err` shows in the status bar; `Escape` cancels;
    - Remove (`Delete`/`Backspace` in region mode) → `remove_from_regions`;
    - `R` → ToggleRegionMode.

- [ ] **Step 1: Write the failing tests** (append)

`tests/document.rs`:
```rust
#[test]
fn region_mode_tool_switch_and_separate_undo() {
    use fap_gui::model::{region_ops::new_regions, selection::SetOp};
    let mut d = doc(&["...."]);
    d.toggle_region_mode();
    assert_eq!((d.mode, d.tool), (Mode::Region, Tool::Select));
    d.selection.apply_cells(&[a1("A1"), a1("B1")], SetOp::Replace);
    d.region_edit(|r, s| { new_regions(r, s.mask()); Ok(()) }).unwrap();
    assert_eq!((d.project.regions.table().len(), d.colors.len()), (1, 1));
    assert!(!d.image_edit(|i, _| i.set(a1("A1"), true)), "image is read-only in region mode");
    assert!(d.undo());
    assert!(d.project.regions.table().is_empty());
    d.toggle_region_mode();
    assert_eq!((d.mode, d.tool), (Mode::Image, Tool::Pencil));
    assert!(!d.undo(), "image stack untouched by region edits");
}

#[test]
fn failed_region_edit_changes_nothing() {
    let mut d = doc(&["...."]);
    d.toggle_region_mode();
    assert!(d.region_edit(|_, _| Err("nope".into())).is_err());
    assert!(!d.undo());
}
```

`tests/keymap.rs`:
```rust
#[test]
fn region_keys() {
    assert_eq!(action_for(Key::R, NONE, Mode::Image), Some(Action::ToggleRegionMode));
    assert_eq!(action_for(Key::R, NONE, Mode::Region), Some(Action::ToggleRegionMode));
    assert_eq!(action_for(Key::N, NONE, Mode::Region), Some(Action::NewRegion));
    assert_eq!(action_for(Key::M, NONE, Mode::Region), Some(Action::MergeRegion));
    assert_eq!(action_for(Key::Delete, NONE, Mode::Region), Some(Action::RemoveFromRegion));
    assert_eq!(action_for(Key::N, NONE, Mode::Image), None);
    assert_eq!(action_for(Key::C, NONE, Mode::Region), None);
}
```

- [ ] **Step 2: Run to verify failure**: `nix develop -c cargo test -p fap-gui`. Expected: compile errors.
- [ ] **Step 3: Implement** the Document methods, region drawing, the panel, property toolbar, merge mode and keys.
- [ ] **Step 4: Run to verify pass**: `nix develop -c cargo test -p fap-gui`. Expected: all pass.
- [ ] **Step 5: Manual check**: `nix develop -c cargo run -p fap-gui`. Then confirm each of the following:
  1. On a drawn image, press `R`, and the image is still visible.
  2. Select a band and press `N`: a tinted region with label 1 appears.
  3. Select cells on both sides of it, press `M` and click the region: it grows.
  4. Select a column through it and press Delete: it splits into two labelled regions of different colours.
  5. Press `R` twice: the colours are unchanged.
  6. Clicking a panel row selects and scrolls to that region.
  7. Cmd-Z in region mode undoes region edits only.
  8. Inserting a row with "new region" in image mode, then pressing `R`, shows the band with a colour of its own.
- [ ] **Step 6: Commit**: `feat(gui): add region mode with panel, colours and labels`

---

### Task 16: Strip preset dialog

**Files:**
- Modify: `crates/fap-gui/src/ui/dialogs.rs`, `src/ui/app.rs`
- Test: append `tests/document.rs`

**Interfaces:**
- Consumes: Plan 0 `Regions::strips`, `Document::region_edit` (Task 15).
- Produces: Regions → "Slice into Strips…" dialog (available in both modes) with strip width N (≥ 1, default 20), start level, end level, and margin D. A live note shows "K strips; last strip W columns". OK runs `region_edit(|r, _| { *r = Regions::strips(w, h, n, start, end, margin, r.default_level()); Ok(()) })` as one region undo step.

- [ ] **Step 1: Write the test** (append)

```rust
#[test]
fn strip_preset_is_one_region_undo_step() {
    let mut d = Document::new(Project::new(23, 2));
    d.region_edit(|r, _| { *r = fap_core::Regions::strips(23, 2, 5, 1, 4, 2, r.default_level()); Ok(()) }).unwrap();
    assert_eq!(d.project.regions.table().len(), 4);
    d.mode = Mode::Region;
    assert!(d.undo());
    assert!(d.project.regions.table().is_empty());
}
```

- [ ] **Step 2: Run** `nix develop -c cargo test -p fap-gui --test document`. Expected: passes (it pins the behaviour the dialog relies on).
- [ ] **Step 3: Implement** the dialog and menu item.
- [ ] **Step 4: Manual check**: on a 30×300 project, Slice into Strips with N = 40, levels 1→4 and D = 2. The note reads "7 strips; last strip 60 columns", and region mode shows 7 coloured strips labelled 1–7 left to right, with levels rising in the panel.
- [ ] **Step 5: Commit**: `feat(gui): add strip preset dialog`

---

## Done when

- `nix develop -c cargo test --workspace`, clippy with `-D warnings` and `cargo fmt --check` are all clean.
- Every manual check above was performed on macOS.
- All of spec §5.1–§5.9 is reachable from the UI. Shortcuts match the §5.8 table, including Cmd/Ctrl-A/S/O/N and Shift-Cmd/Ctrl-S.
- The branch is ready to merge alongside Plans 2 and 3. Plan 4 adds the generate/export dialogs and fills `problem_cells`.
