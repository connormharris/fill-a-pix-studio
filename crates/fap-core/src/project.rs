use crate::grid::Image;
use crate::regions::{DEFAULT_LEVEL, Regions};

/// An editable project: the source image plus region layout and opaque settings.
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

    pub fn width(&self) -> usize {
        self.image.width()
    }

    pub fn height(&self) -> usize {
        self.image.height()
    }
}
