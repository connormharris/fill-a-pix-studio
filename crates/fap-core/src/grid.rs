use crate::error::{CoreError, Result};

/// A zero-based grid coordinate; row 0 is at the top. Ordering is row-major.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(row: usize, col: usize) -> Pos {
        Pos { row, col }
    }

    /// Human-facing, one-based description.
    pub fn describe(&self) -> String {
        format!("row {}, column {}", self.row + 1, self.col + 1)
    }
}

/// A rectangular grid stored row-major.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    cells: Vec<T>,
}

/// A bitmap image; `true` means shaded.
pub type Image = Grid<bool>;

impl<T> Grid<T> {
    /// Builds a grid filled with `fill`. Panics if either dimension is 0.
    pub fn new(width: usize, height: usize, fill: T) -> Grid<T>
    where
        T: Clone,
    {
        assert!(width > 0 && height > 0, "grid dimensions must be non-zero");
        Grid {
            width,
            height,
            cells: vec![fill; width * height],
        }
    }

    /// Builds a grid from rows; fails if empty, any row is empty, or rows are ragged.
    pub fn from_rows(rows: Vec<Vec<T>>) -> Result<Grid<T>> {
        let height = rows.len();
        let width = rows.first().map_or(0, Vec::len);
        if height == 0 || width == 0 {
            return Err(CoreError::Format("grid must not be empty".into()));
        }
        if let Some(i) = rows.iter().position(|r| r.len() != width) {
            return Err(CoreError::Format(format!(
                "row {} has {} cells, expected {}",
                i + 1,
                rows[i].len(),
                width
            )));
        }
        Ok(Grid {
            width,
            height,
            cells: rows.into_iter().flatten().collect(),
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn index(&self, p: Pos) -> usize {
        assert!(
            p.row < self.height && p.col < self.width,
            "position out of bounds: {}",
            p.describe()
        );
        p.row * self.width + p.col
    }

    pub fn get(&self, p: Pos) -> &T {
        &self.cells[self.index(p)]
    }

    pub fn get_mut(&mut self, p: Pos) -> &mut T {
        let i = self.index(p);
        &mut self.cells[i]
    }

    pub fn set(&mut self, p: Pos, v: T) {
        *self.get_mut(p) = v;
    }

    /// Bounds-checked conversion from signed coordinates.
    pub fn pos_at(&self, row: isize, col: isize) -> Option<Pos> {
        let row = usize::try_from(row).ok()?;
        let col = usize::try_from(col).ok()?;
        (row < self.height && col < self.width).then_some(Pos { row, col })
    }

    /// All positions in row-major order.
    pub fn positions(&self) -> impl Iterator<Item = Pos> + use<T> {
        let (w, h) = (self.width, self.height);
        (0..h).flat_map(move |row| (0..w).map(move |col| Pos { row, col }))
    }

    pub fn row(&self, r: usize) -> &[T] {
        assert!(r < self.height, "row {} out of bounds", r + 1);
        &self.cells[r * self.width..(r + 1) * self.width]
    }

    pub fn map<U>(&self, f: impl FnMut(&T) -> U) -> Grid<U> {
        Grid {
            width: self.width,
            height: self.height,
            cells: self.cells.iter().map(f).collect(),
        }
    }
}
