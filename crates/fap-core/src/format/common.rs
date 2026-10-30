//! Pieces shared by the project and puzzle file formats.

use crate::error::{CoreError, Result};
use crate::format::gridtext;
use crate::grid::Grid;
use crate::regions::{DEFAULT_LEVEL, MAX_REGION_ID, RegionId, RegionSettings, Regions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

pub const PROJECT_FORMAT: &str = "fill-a-pix-project";
pub const PUZZLE_FORMAT: &str = "fill-a-pix-puzzle";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Project,
    Puzzle,
}

#[derive(Deserialize)]
struct Header {
    format: String,
    version: i64,
}

pub(crate) fn parse_table(text: &str) -> Result<toml::Table> {
    text.parse::<toml::Table>()
        .map_err(|e| CoreError::Toml(e.to_string()))
}

pub(crate) fn from_table<T: serde::de::DeserializeOwned>(table: toml::Table) -> Result<T> {
    table
        .try_into()
        .map_err(|e: toml::de::Error| CoreError::Format(e.to_string()))
}

/// Deserializes straight from text (keeps TOML datetimes typed, gives line context).
pub(crate) fn from_text<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    toml::from_str(text)
        .map_err(|e: toml::de::Error| CoreError::Format(e.to_string().trim_end().to_string()))
}

/// Checks `format` (against `expected`) and `version`.
pub(crate) fn check_header(table: &toml::Table, expected: &str) -> Result<()> {
    let h: Header = from_table(table.clone())?;
    if h.format != expected {
        return Err(CoreError::Unsupported(format!(
            "expected {expected}, found {}",
            h.format
        )));
    }
    if h.version > i64::from(FORMAT_VERSION) {
        return Err(CoreError::Unsupported(format!(
            "version {} is newer than supported version {FORMAT_VERSION}",
            h.version
        )));
    }
    if h.version < 1 {
        return Err(CoreError::Format(format!("invalid version {}", h.version)));
    }
    Ok(())
}

/// Reads only the `format` key to tell project and puzzle files apart.
pub fn detect_kind(text: &str) -> Result<FileKind> {
    let table = parse_table(text)?;
    let format = table
        .get("format")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CoreError::Format("missing 'format' key".into()))?;
    match format {
        PROJECT_FORMAT => Ok(FileKind::Project),
        PUZZLE_FORMAT => Ok(FileKind::Puzzle),
        other => Err(CoreError::Unsupported(format!("unknown format {other}"))),
    }
}

/// Validates the `width` / `height` keys.
pub(crate) fn check_dims(width: i64, height: i64) -> Result<(usize, usize)> {
    if width < 1 || height < 1 {
        return Err(CoreError::Format(format!(
            "width and height must be at least 1, found {width}x{height}"
        )));
    }
    Ok((width as usize, height as usize))
}

#[derive(Deserialize, Serialize)]
pub(crate) struct RawRegions {
    pub default_level: u8,
    pub rows: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct RawRegion {
    pub id: u32,
    pub level: u8,
    pub margin: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Prefixes the message of a `Format` error with a section name.
pub(crate) fn in_section(section: &str, e: CoreError) -> CoreError {
    match e {
        CoreError::Format(m) => CoreError::Format(format!("{section} {m}")),
        other => other,
    }
}

fn too_large(id: u32) -> CoreError {
    CoreError::Format(format!("[regions] region id {id} is too large"))
}

/// Builds `Regions` from the optional `[regions]` section and the `[[region]]` entries.
pub(crate) fn regions_from_raw(
    raw: Option<RawRegions>,
    entries: Vec<RawRegion>,
    width: usize,
    height: usize,
) -> Result<Regions> {
    let (default_level, map) = match raw {
        Some(r) => (
            r.default_level,
            gridtext::parse_region_rows(&r.rows, width, height)
                .map_err(|e| in_section("[regions]", e))?,
        ),
        None => (DEFAULT_LEVEL, Grid::new(width, height, None)),
    };
    let mut table = BTreeMap::new();
    for e in entries {
        let s = RegionSettings {
            level: e.level,
            margin: e.margin,
            name: e.name,
        };
        if e.id > MAX_REGION_ID {
            return Err(too_large(e.id));
        }
        if table.insert(RegionId(e.id), s).is_some() {
            return Err(CoreError::Format(format!(
                "[regions] region {} is defined more than once",
                e.id
            )));
        }
    }
    Regions::from_parts(map, table, default_level).map_err(|e| in_section("[regions]", e))
}

pub(crate) fn regions_to_raw(r: &Regions) -> (RawRegions, Vec<RawRegion>) {
    let raw = RawRegions {
        default_level: r.default_level(),
        rows: gridtext::region_rows(r.map()),
    };
    let entries = r
        .table()
        .iter()
        .map(|(id, s)| RawRegion {
            id: id.0,
            level: s.level,
            margin: s.margin,
            name: s.name.clone(),
        })
        .collect();
    (raw, entries)
}

/// Writes to a temp file in the same directory, then renames over `path`.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| CoreError::Format(format!("invalid file path {}", path.display())))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(result?)
}
