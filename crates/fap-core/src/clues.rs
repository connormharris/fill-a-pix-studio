use crate::grid::{Grid, Image, Pos};

/// In-grid cells of the 3x3 square centred on `p` (including `p`), row-major.
pub fn neighborhood(width: usize, height: usize, p: Pos) -> impl Iterator<Item = Pos> {
    let rows = p.row.saturating_sub(1)..=(p.row + 1).min(height.saturating_sub(1));
    let cols = p.col.saturating_sub(1)..=(p.col + 1).min(width.saturating_sub(1));
    rows.flat_map(move |row| cols.clone().map(move |col| Pos::new(row, col)))
}

/// Number of shaded cells in the neighbourhood of `p`; off-grid counts as unshaded.
pub fn clue_at(image: &Image, p: Pos) -> u8 {
    neighborhood(image.width(), image.height(), p)
        .filter(|&q| *image.get(q))
        .count() as u8
}

pub fn all_clues(image: &Image) -> Grid<u8> {
    let mut clues = Grid::new(image.width(), image.height(), 0u8);
    for p in image.positions() {
        clues.set(p, clue_at(image, p));
    }
    clues
}
