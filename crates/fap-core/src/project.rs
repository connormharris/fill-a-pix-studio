use crate::error::{CoreError, Result};
use crate::grid::Image;
use crate::regions::{DEFAULT_LEVEL, Regions};

/// An editable project: the source image plus region layout and opaque settings.
///
/// Equality compares the opaque tables too, so a NaN float in `generation` / `pdf`
/// makes a value unequal to itself; producers must not emit NaN.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub title: Option<String>,
    pub image: Image,
    pub regions: Regions,
    pub generation: toml::Table,
    pub pdf: toml::Table,
}

impl Project {
    /// A blank project. Panics if a dimension is 0.
    pub fn new(width: usize, height: usize) -> Project {
        Project {
            title: None,
            image: Image::new(width, height, false),
            regions: Regions::new(width, height, DEFAULT_LEVEL),
            generation: toml::Table::new(),
            pdf: toml::Table::new(),
        }
    }

    /// Region dimensions must match the image, and the regions must be valid.
    pub fn validate(&self) -> Result<()> {
        let (w, h) = (self.image.width(), self.image.height());
        let (rw, rh) = (self.regions.width(), self.regions.height());
        if (rw, rh) != (w, h) {
            return Err(CoreError::Format(format!(
                "regions are {rw}x{rh} but the image is {w}x{h}"
            )));
        }
        self.regions.validate()
    }

    pub fn width(&self) -> usize {
        self.image.width()
    }

    pub fn height(&self) -> usize {
        self.image.height()
    }
}
