use crate::clues::{all_clues, clue_at};
use crate::error::{CoreError, Result};
use crate::grid::{Grid, Image};
use crate::regions::Regions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clue {
    Blank,
    Digit(u8),
    GivenShaded,
    GivenUnshaded,
}

/// A puzzle with its solution.
///
/// Equality compares the opaque tables too, so a NaN float in `generation` / `stats`
/// makes a value unequal to itself; producers of `[stats]` must not emit NaN.
#[derive(Clone, Debug, PartialEq)]
pub struct Puzzle {
    pub title: Option<String>,
    pub clues: Grid<Clue>,
    pub solution: Image,
    pub regions: Regions,
    pub generation: toml::Table,
    pub stats: toml::Table,
}

impl Puzzle {
    /// Requires `regions` to have the same dimensions as `image` (debug-asserted).
    pub fn fully_clued(image: &Image, regions: Regions) -> Puzzle {
        debug_assert_eq!(
            (regions.width(), regions.height()),
            (image.width(), image.height()),
            "regions must match the image dimensions"
        );
        Puzzle {
            title: None,
            clues: all_clues(image).map(|&d| Clue::Digit(d)),
            solution: image.clone(),
            regions,
            generation: toml::Table::new(),
            stats: toml::Table::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        let (w, h) = (self.solution.width(), self.solution.height());
        let dims = [
            ("clues", self.clues.width(), self.clues.height()),
            ("regions", self.regions.width(), self.regions.height()),
        ];
        for (name, cw, ch) in dims {
            if (cw, ch) != (w, h) {
                return Err(CoreError::Format(format!(
                    "{name} are {cw}x{ch} but the solution is {w}x{h}"
                )));
            }
        }
        self.regions.validate()?;
        for p in self.clues.positions() {
            let shaded = *self.solution.get(p);
            match *self.clues.get(p) {
                Clue::Blank => {}
                Clue::Digit(d) => {
                    if d > 9 {
                        return Err(CoreError::Format(format!(
                            "clue {d} at {} is out of range (0-9)",
                            p.describe()
                        )));
                    }
                    let expected = clue_at(&self.solution, p);
                    if d != expected {
                        return Err(CoreError::Format(format!(
                            "clue {d} at {} should be {expected}",
                            p.describe()
                        )));
                    }
                }
                Clue::GivenShaded if !shaded => {
                    return Err(CoreError::Format(format!(
                        "given shaded at {} but the solution cell is unshaded",
                        p.describe()
                    )));
                }
                Clue::GivenUnshaded if shaded => {
                    return Err(CoreError::Format(format!(
                        "given unshaded at {} but the solution cell is shaded",
                        p.describe()
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }
}
