# Fill-a-Pix Puzzle Studio — Design

Date: 2026-10-29
Status: draft for review

Notation used in this document: statements marked **(user)** were decided by the
project owner; statements marked **(designer's call)** were filled in during design
and may be revisited; anything under "Left to the builder" is open.

## 1. Intent

A desktop application for designing black-and-white pixel images and turning them
into Fill-a-Pix puzzles.

Fill-a-Pix rules: the solution is a grid of shaded/unshaded cells. Some cells show a
digit 0–9 giving the number of shaded cells in the 3×3 square centred on that cell
(the cell itself included; cells off the grid count as unshaded). A digit may sit in
a cell that is ultimately shaded or unshaded. Optionally, some cells are *given*
outright: drawn solid black (shaded) or with a fine X (unshaded).

**Why it exists (user):** a games convention posts a large puzzle in a corridor for
collaborative solving. Attendees with a spare hour wander up, work on a section, and
talk to whoever else is there. Staff print and assemble the poster in advance. The
model is last year's crossword: 9 squares high by several hundred long, getting
harder left to right, with a long easy "spine" (Hamlet's soliloquy as three long
clues) spanning the whole width, so there was always a way in, even in the hardest
parts. Staff design the image themselves, e.g. a collage of game and puzzle motifs
(controllers, chess pieces, tic-tac-toe boards). Future years may produce sets of
puzzles whose solutions are clues to other puzzles.

**What success looks like:** staff can draw a ~30×300 image, paint it into regions
(e.g. an easy central strip, with the parts above and below divided into sections of
increasing difficulty), generate a uniquely solvable puzzle in which every region
can be solved on its own using only clues inside it and just past its edges, at that
region's difficulty, and print it as tiles that assemble into a wall poster.

**Approach (user):** a working prototype early, even with features incomplete, is
the best way to find problems. UI polish is not a priority.

## 2. Goals, non-goals, anti-goals

### Goals
- Image editor with the operations in §5.
- Puzzle generator with a guaranteed unique solution and per-region difficulty and
  independent solvability (§6).
- PDF rendering of puzzle files as tiled multi-page posters (§7).
- Text-based, extensible file formats (§4).
- Cross-platform (macOS, Linux, Windows). (user: Unix-like first with Windows later
  is acceptable; with the chosen stack Windows is expected to come essentially free.)

### Non-goals
- Commercial distribution or UI polish.
- Solving puzzles interactively inside the app.
- Importing raster images (PNG/JPEG) or colour Fill-a-Pix.
- System clipboard integration (the clipboard is internal to the app).
- Printing directly; output is PDF.
- Multiple documents open at once in one window (designer's call; the internal
  clipboard persists across loading a different project, which covers collage work).

### Anti-goals (would make it a failure even if it "works")
- Exporting a puzzle that is not uniquely solvable. Uniqueness is guaranteed by an
  exact check, never estimated.
- Tiles that don't line up when assembled, or clue digits cut by page edges.
- Clue digits unreadable from a step back on a wall.
- Generation freezing the editor, or a pathological image consuming the CPU forever.
- Losing work: prompt on quit/new/load with unsaved changes.

## 3. Constraints and stack

- **Language:** Rust (user preference: Rust > C++ > Java).
- **GUI:** egui via eframe (designer's call): immediate-mode, cross-platform,
  good fit for a custom-drawn zoomable grid.
- **License:** GPL is acceptable (user), so any GPL-compatible crate may be used.
  The project itself is GPL-3.0-or-later (designer's call).
- **Scale (user):** images up to roughly 30 rows × 300 columns (9,000 cells) must
  work comfortably; larger should degrade gracefully rather than fail.
- **Architecture (designer's call):** one Cargo workspace:
  - `fap-core`: grid types, clue computation, project and puzzle file formats,
    PBM I/O, rescaling.
  - `fap-solver`: exact uniqueness solver, graded human-style solver, difficulty
    scoring, generator.
  - `fap-pdf`: PDF rendering of a puzzle.
  - `fap-gui`: the eframe desktop application.
  - `fap-cli`: command-line front end (`generate`, `check`, `grade`, `render`),
    so solver and PDF work can be tested without the GUI.

  Crate names are placeholders; the builder may rename.

Coordinates: internally `(row, col)`, zero-based, row 0 at the top. In the UI and
PDF, rows and columns are shown one-based, or as spreadsheet letters (A…Z, AA…AZ,
BA…, ZZ, AAA…) where the user chooses that. The user's examples use spreadsheet
notation: letter = column, number = row (e.g. C2 = column 3, row 2).

## 4. Data model and file formats

### Core types
- **Image:** width × height of booleans (shaded/unshaded).
- **Region map (user, with designer's ID scheme):** width × height of cells, each
  holding a region ID or "no region", plus a region table mapping each ID to its
  settings: target level 1–4 (§6.2), margin D (0–2, §6.4), and an optional name. A
  new project starts with no regions. Regions must be edge-connected; the editor
  enforces this (§5.6). IDs are internal and stable, so undo and files can refer to
  regions. What users see are display labels and colours, both computed (§5.6).
- **Default level:** the target level for cells in no region (project setting).
- **Puzzle:** width × height of clue cells, each one of: no clue, digit 0–9, given
  shaded, or given unshaded. A given cell never also carries a digit (designer's call,
  user agreed). The puzzle also stores its solution image, generation settings and
  stats.

### Formats (user: text-based and extensible for now; designer's call: TOML)
Grids are stored as multi-line strings, one line per row, one character per cell,
so files are readable and diffable. Each file has a `format` and `version` field.
Unknown keys are ignored on load. A newer major version is refused with a clear
message.

**Project file** (`.fapx.toml` or similar; extension is the builder's call):
```toml
format = "fill-a-pix-project"
version = 1
width = 6
height = 3

[image]          # '#' shaded, '.' unshaded
rows = """
.##...
#..#..
.##...
"""

[regions]        # one token per cell, whitespace-separated: a region ID, or '.' for none
default_level = 2
rows = """
1 1 1 2 2 .
1 1 3 3 2 .
1 1 3 3 2 2
"""

[[region]]
id = 1
level = 1
margin = 2
name = "spine"   # optional

[[region]]
id = 2
level = 3
margin = 2

[[region]]
id = 3
level = 4
margin = 1

[generation]     # last-used generation settings (§6.6), optional
[pdf]            # last-used PDF settings (§7), optional
```

**Puzzle file:**
```toml
format = "fill-a-pix-puzzle"
version = 1
width = 6
height = 3
title = "Corridor 2027"      # optional

[clues]          # digit = clue, '.' = no clue, '#' = given shaded, 'x' = given unshaded
rows = """
2.4..0
.#..x.
3..2..
"""

[solution]
rows = """ ... """

[regions]        # the region map and table used for generation, same layout as the project
[generation]     # settings and seed used, so the result can be reproduced
[stats]          # score, hardest level used, cells resolved per level, clue/given counts,
                 # and the same breakdown per region
```

**Image file:** PBM (Netpbm `P4` binary when writing; read both `P1` ASCII and
`P4`). Used for plain images, selection export, and paste-from-file.

## 5. Editor (front end)

### 5.1 Canvas and view
- New canvas: given width and height, all unshaded, no regions.
- Zoom (Cmd/Ctrl + wheel, pinch, zoom-to-fit, zoom buttons) and scroll (wheel,
  scrollbars, middle-drag or space-drag). Grid lines are visible above a minimum
  zoom. Heavier lines every 5 cells, row/column rulers along the edges (designer's
  call).
- **Problem overlay** (switchable): unforced cells (§6.5) and regions that failed
  generation (§6.4).
- The editor has two modes: **image mode** (default) and **region mode** (§5.6),
  toggled with R.

### 5.2 Tools (designer's call: resolves the clash between "click inverts a cell"
and "click starts a selection")
- **Pencil (P), image mode only:** click inverts a cell. Dragging paints: every cell
  crossed is set to the value the first cell took on.
- **Select (S):** click / shift-click / drag build selections (§5.3). In image mode,
  dragging from inside an existing selection moves it (§5.4).
- **Magic wand (W) (user, after Photoshop):** click selects a connected component
  (§5.3). In image mode, dragging from inside an existing selection moves it, as in
  the Select tool (designer's call), so a group of icons can be picked and moved
  without switching tools. In region mode the wand selects whole regions (§5.6).

### 5.3 Selections (user)
- Rectangle: click one corner, then shift-click the opposite corner, or click-and-drag.
- Non-rectangular, using spreadsheet (Apple Numbers) conventions, with "Cmd" on
  macOS and "Ctrl" elsewhere (written Cmd/Ctrl below):
  - Cmd/Ctrl-click sets the anchor corner of a new rectangle. If the anchor cell is
    unselected the rectangle will be **added**; if it is already selected the
    rectangle will be **removed**.
  - Shift-click then sets the opposite corner. The rectangle is added to, or removed
    from, the selection wherever it overlaps.
  - Example: with A1..D4 selected, Cmd/Ctrl-click E3 then shift-click C2 adds E2 and
    E3. With A1..D4 selected, Cmd/Ctrl-click C2 then shift-click D3 removes C2, C3,
    D2, D3. (Designer's reading of the rule: removal removes the overlap, so
    shift-clicking E4 instead would remove C2..D4.)
- Click (Select tool) on a single cell with no modifier selects just that cell.
- **Connected components (magic wand, image mode):**
  - Click replaces the selection with the connected component containing the
    clicked cell. Clicking a shaded cell selects its shaded component (user).
    Clicking an unshaded cell selects its unshaded component (designer's call,
    following Photoshop; handy for holes and backgrounds).
  - Shift-click adds the clicked component to the selection, so several components
    can be selected and moved together (user).
  - Option/Alt-click removes the clicked component from the selection (designer's
    call, Photoshop convention).
  - Connectivity is 4-neighbour by default, with an 8-neighbour option on the tool's
    toolbar (designer's call; 8-neighbour keeps diagonal strokes together).
- Selections made with any tool can be combined: e.g. wand-select two icons, then
  switch to the Select tool and Cmd/Ctrl-click/shift-click to add a rectangle.
- The selection is shared between image and region mode (designer's call).
- Selection changes are not undo steps (designer's call).

### 5.4 Image operations on a selection (image mode)
- Clear (C or Delete), fill (F), invert (I) all selected cells (user); also
  toolbar buttons and the right-click menu.
- **Copy:** copies the selection's bounding box plus its mask into the internal
  clipboard.
- **Cut:** copies, then immediately unshades the source cells (its own undo step).
  A later paste writes onto the grid as it is after the cut. So where source and
  destination overlap, the overlapping cells end up with the pasted values, and only
  source cells outside the destination stay unshaded.
- **Paste (Cmd/Ctrl-V):** shows a movable outline of the clipboard. Click to commit,
  Esc to cancel. On commit, every cell in the clipboard mask overwrites the
  destination, unshaded cells included.
- **Shade-only paste (Cmd/Ctrl-Shift-V) (user):** shaded clipboard cells shade the
  destination; unshaded clipboard cells leave the destination unchanged.
- Worked example (user): A1..C3 all shaded except C3. Cut A1..C3 and paste at
  B2..D4: A1, A2, A3, B1, C1 end up unshaded, and D4 is unshaded whatever it was
  before. Copy-paste the same way and A1, A2, A3, B1, C1 stay shaded; D4 is still
  unshaded.
- **Drag-move (user):** dragging a selection in the Select or wand tool behaves like
  cut-and-paste, as a single undo step, and does not change the clipboard. The order
  is the same: first unshade the source, then write the moved cells at the
  destination. Overlapping cells therefore take the moved values. The selection
  follows the moved cells.
- **Save selection as PBM (user):** writes the bounding box; cells outside the mask
  are written unshaded.
- **Paste from file (user):** loads a PBM into the clipboard and enters paste mode.
- Image operations never change the region map (designer's call).

### 5.5 Structure edits (user)
- Insert N rows at any position. Two independent choices:
  - image fill: all shaded, all unshaded, copy of the row above (not when inserting
    at the top), or copy of the row below (not when inserting at the bottom);
  - region fill: no region, inherit the region of the cell above (not at the top),
    inherit the region of the cell below (not at the bottom), or insert as a new
    region (user): all inserted cells form one new region with default settings.
  Columns work the same way (left/right).
- Inheriting never splits a region or changes which regions border each other. A
  "no region" or "new region" insertion through the middle of a region can split
  it; the split rule (§5.6) then applies. A new region creates new borders, so it
  gets a colour by the usual recolouring rule (§5.6).
- Delete rows/columns: the selected ones, or a range entered in a dialog. Deletions
  can split regions (split rule applies), delete a region's last cells (the region is
  deleted), and make two same-coloured regions touch (recolouring happens the next
  time region mode opens).
- **Rescale (user):** enter a new width × height, with an optional aspect-ratio lock.
  Each output cell takes the area-weighted fraction of input cells it covers and is
  shaded if that fraction is ≥ ½ (exactly ½ counts as shaded). For the region map,
  each output cell goes to whichever region covers the largest share of it, with
  "no region" counting as a candidate (designer's call); ties go to the
  lower-numbered region (designer's call). Small regions may vanish under drastic
  reduction, which is acceptable (user). Splits caused by rescaling follow the split
  rule. To rescale an existing *puzzle*, rescale its solution image and generate a
  new puzzle.

### 5.6 Region mode (user unless marked)
- **Entering and leaving:** R toggles region mode on and off. Leaving returns to the
  previously used image tool. In region mode the image is shown but can't be edited:
  pencil and image operations are disabled (designer's call).
- **What a region is:** an edge-connected set of cells with settings: target level
  1–4, margin D 0–2, optional name. Cells may belong to no region; they use the
  project's default level and get no independent-solvability check (§6.4).
- **Display:** each region is drawn in one of six colours, translucent (alpha about
  0.5) so the image shows through. Borders between regions are drawn.
- **Colouring:** colours are not a property of the region. They are kept in the
  session's in-memory state, so toggling region mode off and on keeps them the same.
  They are not saved. Regions that border each other (sharing an edge, or only a
  corner) must differ. Six colours always suffice for maps on a square grid where
  corners count (Ringel's conjecture, proved by Borodin), but a greedy pass is not
  guaranteed to find a six-colouring. Method (designer's call): when an edit makes
  two same-coloured regions border, recolour only the affected regions greedily.
  Where there's a free choice, prefer the colour with the fewest cells so the map
  gets variety (user). If greedy gets stuck, run a small backtracking search over
  the region graph, which has at most tens of regions, so this is instant.
- **Labels:** each region shows one integer label, placed at its centroid if that
  cell is in the region, otherwise at the region's cell closest to the centroid.
  Labels are not stored. They are recomputed whenever regions change, numbered 1, 2,
  … in this order:
  - by x + y of the centroid (x = column, y = row);
  - ties broken by the centroid's x;
  - then by the smallest x + y of any cell in the region;
  - then by the smallest x of any cell.
  Renumbering after an edit is acceptable. Internally, regions also have a stable ID
  that is never shown, and that undo and the file format use (designer's call).
- **Region panel:** lists regions by label with their settings. Clicking an entry
  scrolls the canvas to that region and selects all its cells.
- **Selection tools:** the same as image mode (Select, Cmd/Ctrl-click, shift-click,
  drag). The magic wand selects all cells of the clicked region; shift-click adds
  regions, Option/Alt-click removes them. Clicking an unassigned cell with the wand
  selects its edge-connected area of unassigned cells (designer's call).
- **Region property toolbar:** shown when the selection lies entirely within one
  region. It edits that region's level, margin D and name.
- **New region (N or button):** each edge-connected component of the selection
  becomes a new region with default settings. The cells are taken away from
  whatever regions they belonged to.
- **Merge into region (M or button):** the cursor changes; click a cell of an
  existing region to add the selected cells to it. The operation is a no-op (with a
  status-bar message) if the resulting region would not be edge-connected. Note
  that this test is on the result: a selection in two pieces on either side of the
  region is fine. Esc cancels.
- **Remove from region (Delete or button):** the selected cells become unassigned.
- **Split rule:** any operation that splits a region into several edge-connected
  pieces makes each piece an independent region with the same settings as the
  original. (Designer's note: since labels are recomputed, no piece is
  "the original"; each piece just gets a new stable ID.) Removing a region's last
  cell deletes the region. This applies in both modes, including row/column deletion
  in image mode.
- **Strip preset:** "slice into vertical strips of N columns", with any leftover
  columns folded into the rightmost strip, so it is up to 2N−1 wide (user). Each
  strip becomes a region. The dialog offers a level ramp (start and end level,
  interpolated and rounded across the strips) and a common margin D. Applying it
  replaces the region map (designer's call), as one undo step.

### 5.7 Undo / redo
- 32 levels each (user), storing full state snapshots, which are small.
- **Two stacks (user):** one for image edits, one for region edits. Cmd/Ctrl-Z in
  image mode undoes the image stack; in region mode, the region stack.
- Undoing a region operation restores the exact set of regions, with their settings,
  that existed before it (user).
- **Operations that change the grid size** (row/column insert/delete, rescale)
  affect both the image and the regions. They go on the image stack. Region history
  must survive them (user): inserting a row and then undoing the insertion leaves
  earlier region operations still undoable. Method (designer's call):
  - The size-changing operation's undo entry stores the image and current regions
    before the change, **plus a copy of the whole region undo/redo stack**. That's at
    most 64 small snapshots, well under a few MB at 30×300.
  - The same operation is then applied to every snapshot on the region stack (e.g.
    the inserted rows are inserted into each, with the same region fill), so region
    undo keeps working at the new size even if the operation is never undone.
  - Undoing the size change restores the image, the regions and the saved region
    stack exactly. Redoing it re-applies the operation the same way.
  - Region operations never change the grid size, so the image stack never needs
    the reverse treatment.
- Switching between image and region mode does not clear either stack
  (designer's call).

### 5.8 Keyboard shortcuts
| Key | Action |
|-----|--------|
| P | Pencil (user) |
| S | Select (user) |
| W | Magic wand (user) |
| R | Toggle region mode (user) |
| C or Delete | Clear selection's cells (image mode) (user) |
| F | Fill selection (image mode) (user) |
| I | Invert selection (image mode) (user) |
| N | New region from selection (region mode) (user) |
| M | Merge selection into a region (region mode) (user) |
| Delete | Remove selection from regions (region mode) (user) |
| Cmd/Ctrl-Z, Shift-Cmd/Ctrl-Z | Undo, redo |
| Cmd/Ctrl-X, -C, -V | Cut, copy, paste |
| Shift-Cmd/Ctrl-V | Shade-only paste |
| Cmd/Ctrl-A | Select all |
| Esc | Cancel paste or merge; else clear selection |
| + / − / 0 | Zoom in / out / to fit |
| Cmd/Ctrl-S, -O, -N | Save, open, new |

Other commands (insert/delete rows and columns, rescale, strip preset, generate,
export PDF) get menu entries; shortcuts for them are the builder's call. All
designer-proposed keys are the builder's to adjust.

### 5.9 Files
- New, Open, Save, Save As for project files; Import/Export PBM for the bare image.
  The unsaved-changes prompt is in §2.

### 5.10 Generate and export (integration with §6 and §7)
- **Generate dialog:** settings from §6.6. Run starts N candidates in the background
  with progress and Cancel; the editor stays usable. Finished candidates are listed
  with score and breakdown (clue count, given count, hardest level, cells per level),
  overall and per region (by label), including whether each region met its floor.
  Selecting one shows a preview in two modes: clue view, and colour-by-level (each
  cell tinted by the level that resolved it in its region's sub-puzzle), to compare
  against the region settings. Candidates that miss a floor are flagged, not hidden.
  The chosen candidate can be saved as a puzzle file and/or exported as PDF.
- **PDF dialog:** settings from §7, also available for any puzzle file opened from
  disk.

## 6. Solver and generator

This is the most algorithmically open part of the project (user). The requirements
are fixed below; the techniques are largely left to the builder, who is expected to
experiment.

### 6.1 Exact solver (uniqueness)
- Given a puzzle (clues + givens), determine whether it has 0, 1, or more solutions.
  This is the authority for "uniquely solvable".
- Must be fast at 9,000 cells, since the generator calls it repeatedly. Propagation
  plus backtracking, or a SAT solver crate, are both acceptable.
- Must be able to report the **unforced cells**: those that take different values in
  different solutions. The ideal is the full set; a subset found from a few
  alternative solutions is acceptable for a prototype.
- Must work on **sub-puzzles** (§6.4): a set of clues, and a set of target cells
  that must be forced, while every other cell those clues touch is a free unknown.
  The answer is whether all target cells are forced, and which are not.

### 6.2 Graded solver (human-style difficulty)
Solves the way a person would, always using the simplest deduction available, and
records for every cell the **level** of reasoning that first resolved it. Like the
exact solver, it runs on a whole puzzle or on a region's sub-puzzle (§6.4).
User-facing levels (designer's proposal, user agreed):
1. **Single clue:** a clue already has all its shaded cells (the rest are unshaded),
   or needs every remaining unknown (all shaded).
2. **Two overlapping clues:** comparing two nearby clues' squares forces cells
   (e.g. a 6 directly left of a 3: the three cells left of the 6 are shaded and
   the three right of the 3 unshaded).
3. **Short hypothesis:** assume a cell's value, propagate with levels 1–2, and a
   contradiction proves the opposite.
4. **Longer chains:** deeper or branching hypotheses.

Above level 4, or out of budget, the puzzle counts as "too hard": it may be unique
but humans shouldn't be asked to solve it. The builder may use finer internal
techniques but must map them onto these four bands, since the map and UI use them.

**Reasoning budget (user idea, refined by designer):** each candidate evaluation
gets its own budget of reasoning steps, as an atomic counter if several threads work
on one evaluation. Exhausting it marks the puzzle too hard. A per-candidate budget
(rather than one global pool) keeps candidates independent and results
reproducible.

**Human-like targeting (user):** heuristics for which cells a person in a partially
solved puzzle would look at first (e.g. near recent progress, extreme clues,
edges). They affect stats such as "average unsolved frontier" and possibly tie-breaking
between deductions at the same level. Exploratory.

### 6.3 Difficulty scoring
- Per puzzle: one numeric score plus a breakdown: hardest level used, cells resolved
  at each level, clue count and density, given count, and optionally the user's
  suggestion of the average number of unsolved cells during a run. The formula is
  the builder's, and it only needs to be roughly right (user). It must rise with
  harder levels and with how often they're needed (Sudoku-book style).
- Givens count toward easiness: a puzzle easy only because of givens scores easy.
- **Per-region difficulty (user):** each region is graded as its own sub-puzzle
  (§6.4): the clues inside it plus those within its margin, with everything else
  unknown. This is the region's difficulty, and it is what floors and ceilings apply
  to. Optionally also report each cell's level in one full simplest-first solve of the
  whole puzzle, for comparison (designer's call).

### 6.4 Regions and independent solvability (user)
- **Definition:** a region is *independently solvable with margin D* if every cell
  in it can be uniquely determined using only the clues (digits and givens) in cells
  of the region or within Chebyshev distance ≤ D of some cell of the region. All other
  cells, including the true values of cells outside the region, are treated as
  unknown. Cells outside the region need not be determined.
- **Guarantee:** every region in a generated puzzle is independently solvable with
  its own margin D (0–2, set per region). This replaces the earlier idea of an
  entry-point check: each region is its own way in.
- **Consequence:** if every region passes, every cell of the grid is forced, so the
  whole puzzle is uniquely solvable. The generator still re-verifies global
  uniqueness with the exact solver before export, as a safety net.
- **Difficulty targets:** each region's target level is a ceiling (its sub-puzzle
  must be solvable by the graded solver using no level above the target, within
  budget), and for target ≥ 2 also a floor (user): at least a configurable share of
  the region's cells must actually need the target level. The default share is
  tunable; the builder picks it after experimenting (designer's call).
- **Unassigned cells (user):** cells in no region use the project's default level
  and get no independent-solvability check. They are covered by the global
  uniqueness requirement. Their ceiling and floor are measured with the graded
  solver on the whole puzzle (designer's call).
- The simple "difficulty slider" is just the default level with no regions. The
  strip preset (§5.6) gives left-to-right gradients. Painted regions give
  arbitrary layouts, such as an easy central spine with harder sections above and
  below (user).
- **Failure case (designer's call):** a region with a small margin may be
  impossible to solve on its own even with every clue in it showing. For example,
  with D = 0 its edge clues involve cells outside it. The generator first checks each
  region fully clued (within its margin). Regions that fail are reported and
  highlighted in the editor. If the givens mode allows (§6.5), givens are added
  inside the region to fix it.
- Very small regions may not be able to meet a floor. The generator reports this as
  a warning rather than an error (designer's call).

### 6.5 Given cells (user)
Three modes:
- **Use freely:** givens are another clue type the generator may keep. A separate
  density slider sets the target, and the effective density decreases as the
  region's target level increases.
- **Only to disambiguate:** check whether the fully clued puzzle (every cell
  showing its digit) is unique, and whether each region is independently solvable
  when fully clued. If not, add the fewest givens the builder's method can find,
  chosen among the unforced cells. Do this before clue removal, so removal works
  around them.
- **Never:** run those fully clued checks first as a quick test. If one fails,
  generation stops, and the editor shows the unforced cells and failing regions in
  the problem overlay so staff can fix the image or the regions.

### 6.6 Generator
- Inputs: image, region map and table, givens mode + density, number of candidates
  (default 4), base seed, budget.
- **Region by region (user idea, generalised from strips).** Regions are generated
  one at a time. Within a region, clue removal works on the region's own cells and
  its margin:
  - Clues in the margin that belong to regions already generated are fixed.
  - Clues in the margin that belong to regions not yet generated may be kept, and are
    then **locked**: later regions may not remove them, since an earlier region
    depends on them.
  - When a later region is generated, removals inside it are checked against its own
    requirements. They cannot break earlier regions, because every clue those regions
    depend on is locked.
- **Region order (designer's call):** easiest target level first, ties broken left to
  right then top to bottom. The easy spine is therefore generated and locked first,
  and harder regions border its dense, easy clues, as in the Hamlet crossword. With
  the strip preset at a rising ramp, this is also left to right, which matches the
  user's original strip idea. Unassigned cells are handled last, as one final pass
  whose removals must keep the whole puzzle unique and within the default level's
  ceiling, without breaking any region (designer's call).
- Within a region (designer's sketch; the builder may improve it): start from all
  clues available (plus givens per the mode). Try removing clues in a seeded random
  order, possibly weighted by heuristics. Keep a removal only if the region stays
  independently solvable with its margin, within its ceiling and the budget. Then
  check the floor; push harder or flag it.
- Because each check covers one region plus its margin, checks stay local and cheap
  even on a 30×300 grid.
- Candidates run in parallel with different seeds derived from the base seed.
  Same inputs and seed must give the same puzzle.
- Progress reporting and cancellation are required. Performance target: a
  full-size (30×300) candidate within a few minutes on a modern laptop, faster
  being better. Incremental or local re-evaluation after each removal is likely
  necessary.
- Development order (user): (1) trivial generator: every cell gets its digit, no
  validation; (2) uniqueness tester, including sub-puzzles; (3) difficulty grader;
  (4) generator with one region at a single difficulty; (5) generator over multiple
  regions (strip preset first, then arbitrary regions; the algorithm is the same).

## 7. PDF rendering (user unless marked)

- Input: a puzzle file (the GUI passes its in-memory equivalent). A puzzle file must
  be renderable on its own via GUI or CLI.
- **Paper:** US Letter, A4, A3, 11×17 (Tabloid), from a generic paper-size table so
  more sizes are trivial to add. Portrait or landscape, with an "auto" option that
  picks the orientation giving fewer pages (designer's call). Refuse paper too small
  for the requested settings.
- **Cell size:** configurable, 10–40 mm.
- **Margins:** configurable, defaulting to a printer-safe value around 10–12 mm
  (designer's call).
- **Tiling:** every page shows a block of the grid at the same size and position.
  The last row/column of tiles may be partial. Pages assemble edge to edge after
  trimming. Crop marks at the grid edges (designer's call).
- **Drawing:** thin cell lines, heavier every 5 cells counted from the puzzle origin
  (not the page). Digits centred, sans-serif, sized relative to the cell (around 60%
  of cell height; builder tunes). Given shaded: solid fill. Given unshaded: fine X.
- **Footer on every page:** title (if any), row and column range covered (e.g.
  "Rows 41–60 · Cols U–AN"), and tile position spelled out (user), e.g. "Tile: 3rd
  from top (of 4), 8th from left (of 20)", with correct English ordinals (1st, 2nd,
  3rd, 4th, 11th, 12th, 13th, 21st, 22nd, …), using the chosen label style.
- **Row/column labels (optional):** independent choices for rows and for columns:
  none, numbers, or spreadsheet letters (A…Z, AA, AB, …, AZ, BA, …, ZZ, AAA, …).
  Printed along the outer edges of the assembled poster by default, with an option to
  print them on every tile (designer's call).
- **Printer's marks (overlap strips):** optionally print the next row/column beyond
  the tile edge in grey, with independent switches for top, bottom, left and right.
  This reduces the cells per page.
- **Staff sheets (designer's call, user agreed):** optional summary page (title,
  size, settings, stats), assembly map (tile layout with labels), and solution
  (reduced scale, may span several pages). These go at the end of the PDF or into
  a separate staff PDF.
- PDF library and font are the builder's call. Any GPL-compatible font may be
  embedded, or base-14 Helvetica used.
- Development order (user): (1) fixed-size paginated output on Letter/A4; (2)
  configurable cell size; (3) footers; (4) row/column labels; (5) printer's marks.
  Then heavier 5-cell lines, crop marks, and staff sheets (designer's addition).

## 8. Staging and parallel work (user, with designer's additions)

- **Stage 0 (sequential, first):** workspace skeleton, `fap-core` (types including
  region map and table, clue computation, TOML project/puzzle formats with round-trip
  tests, PBM I/O), and a `fap-cli` skeleton. This fixes the file formats, which the parallel tracks share.
- **Then three parallel tracks, each in its own git worktree:**
  - **Track 1, front end:** minimal app (new canvas at fixed size, pencil toggle,
    zoom/scroll, save/load project) → rectangle select + shade/unshade/invert →
    undo/redo → copy/paste → cut/paste → drag-move → row/column insert/delete →
    non-rectangular selection → connected-component selection → selection-to-PBM and
    paste-from-file → rescale → region mode (display, colouring, labels, panel,
    new/merge/remove, split rule, separate undo stack) → strip preset. Shortcut keys
    are added along with each feature.
    (Undo/redo is placed early by the designer, since every later operation needs it.)
  - **Track 2, solver/generator:** §6.6 development order, plus givens modes, all
    exposed through `fap-cli` (`generate`, `check`, `grade`). Until region mode
    exists, test region maps are written by hand in project files or made by a CLI
    strip-slicing option.
  - **Track 3, PDF:** §7 development order, exposed through `fap-cli render`.
- **Stage 4 (integration):** generate dialog with candidates and preview, problem
  overlay, and PDF export dialog in the GUI (§5.10).

Each track must keep `fap-core` changes minimal and coordinated, since it is shared.

## 9. Testing expectations
- `fap-core`: clue computation including edges and corners; format round-trips;
  PBM P1/P4 read and P4 write; rescale on known cases (including exact ½ → shaded).
- Solver: known unique and known ambiguous cases (e.g. a fully clued 2×2 grid
  is ambiguous unless it is all unshaded or all shaded, since every clue equals the
  total count); unforced-cell reporting; sub-puzzle checks with margins 0, 1 and 2;
  determinism by seed; budget exhaustion terminates.
- Generator: every exported puzzle is re-verified unique by the exact solver, and
  every region re-verified independently solvable with its margin. Locked clues are
  never removed by later regions.
- Region map: split rule, merge connectivity test (including a two-piece selection
  on either side of a region), label ordering, colouring validity (no two bordering
  regions share a colour, corners included), strip preset including the leftover fold
  (e.g. 23 columns with N = 5 gives strips of 5, 5, 5, 8), row/column insertion
  with each region fill (including new region), deletion, rescale.
- PDF: tile arithmetic (cells per page with and without overlap strips, label
  generation like Z→AA, AZ→BA, ZZ→AAA, ordinals 1st…13th, 21st…23rd, 111th), page count; a smoke test that produced PDFs
  parse.
- GUI: editing operations are implemented as pure functions on the model where
  possible, so they can be unit tested (selection algebra, cut/paste example in §5.4,
  row insertion fills).

## 10. Left to the builder
- Exact solving techniques inside each difficulty band, the scoring formula,
  floor-share default, budget size, human-targeting heuristics.
- How removal order within a region is weighted, and how hard to push for floors.
- Display colour assignment for regions (any scheme where adjacent regions differ).
- SAT crate vs hand-written exact solver.
- PDF crate and font; exact digit sizing and line weights.
- File extensions; crate names; keyboard shortcuts; toolbar layout.
- Threading approach (e.g. rayon or std threads) for candidates.
