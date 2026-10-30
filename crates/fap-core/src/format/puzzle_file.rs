use super::common::{
    RawRegion, RawRegions, check_dims, check_header, from_text, in_section, parse_table,
    regions_from_raw, regions_to_raw, write_atomic,
};
use super::{FORMAT_VERSION, PUZZLE_FORMAT, gridtext};
use crate::error::Result;
use crate::puzzle::Puzzle;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct RawRows {
    rows: String,
}

#[derive(Serialize, Deserialize)]
struct RawPuzzle {
    format: String,
    version: u32,
    width: i64,
    height: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    clues: RawRows,
    solution: RawRows,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    regions: Option<RawRegions>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    region: Vec<RawRegion>,
    #[serde(default, skip_serializing_if = "toml::Table::is_empty")]
    generation: toml::Table,
    #[serde(default, skip_serializing_if = "toml::Table::is_empty")]
    stats: toml::Table,
}

pub fn load_puzzle(text: &str) -> Result<Puzzle> {
    let table = parse_table(text)?;
    check_header(&table, PUZZLE_FORMAT)?;
    let raw: RawPuzzle = from_text(text)?;
    let (w, h) = check_dims(raw.width, raw.height)?;
    let clues =
        gridtext::parse_clue_rows(&raw.clues.rows, w, h).map_err(|e| in_section("[clues]", e))?;
    let solution = gridtext::parse_image_rows(&raw.solution.rows, w, h)
        .map_err(|e| in_section("[solution]", e))?;
    let regions = regions_from_raw(raw.regions, raw.region, w, h)?;
    let puzzle = Puzzle {
        title: raw.title,
        clues,
        solution,
        regions,
        generation: raw.generation,
        stats: raw.stats,
    };
    puzzle.validate()?;
    Ok(puzzle)
}

pub fn save_puzzle(p: &Puzzle) -> String {
    let (regions, region) = regions_to_raw(&p.regions);
    let raw = RawPuzzle {
        format: PUZZLE_FORMAT.to_string(),
        version: FORMAT_VERSION,
        width: p.solution.width() as i64,
        height: p.solution.height() as i64,
        title: p.title.clone(),
        clues: RawRows {
            rows: gridtext::clue_rows(&p.clues),
        },
        solution: RawRows {
            rows: gridtext::image_rows(&p.solution),
        },
        regions: Some(regions),
        region,
        generation: p.generation.clone(),
        stats: p.stats.clone(),
    };
    toml::to_string(&raw).expect("puzzle is always serializable")
}

pub fn read_puzzle_file(path: &Path) -> Result<Puzzle> {
    let text = std::fs::read_to_string(path)?;
    load_puzzle(&text)
}

/// Validates first (the file is left untouched on error). `save_puzzle` does not validate.
pub fn write_puzzle_file(path: &Path, p: &Puzzle) -> Result<()> {
    p.validate()?;
    write_atomic(path, save_puzzle(p).as_bytes())
}
