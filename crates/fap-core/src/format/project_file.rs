use super::common::{
    RawRegion, RawRegions, check_dims, check_header, from_table, in_section, parse_table,
    regions_from_raw, regions_to_raw, write_atomic,
};
use super::{FORMAT_VERSION, PROJECT_FORMAT, gridtext};
use crate::error::Result;
use crate::project::Project;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct RawImage {
    rows: String,
}

#[derive(Serialize, Deserialize)]
struct RawProject {
    format: String,
    version: u32,
    width: i64,
    height: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    image: RawImage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    regions: Option<RawRegions>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    region: Vec<RawRegion>,
    #[serde(default, skip_serializing_if = "toml::Table::is_empty")]
    generation: toml::Table,
    #[serde(default, skip_serializing_if = "toml::Table::is_empty")]
    pdf: toml::Table,
}

pub fn load_project(text: &str) -> Result<Project> {
    let table = parse_table(text)?;
    check_header(&table, PROJECT_FORMAT)?;
    let raw: RawProject = from_table(table)?;
    let (w, h) = check_dims(raw.width, raw.height)?;
    let image =
        gridtext::parse_image_rows(&raw.image.rows, w, h).map_err(|e| in_section("[image]", e))?;
    let regions = regions_from_raw(raw.regions, raw.region, w, h)?;
    Ok(Project {
        title: raw.title,
        image,
        regions,
        generation: raw.generation,
        pdf: raw.pdf,
    })
}

pub fn save_project(p: &Project) -> String {
    let (regions, region) = regions_to_raw(&p.regions);
    let raw = RawProject {
        format: PROJECT_FORMAT.to_string(),
        version: FORMAT_VERSION,
        width: p.width() as i64,
        height: p.height() as i64,
        title: p.title.clone(),
        image: RawImage {
            rows: gridtext::image_rows(&p.image),
        },
        regions: Some(regions),
        region,
        generation: p.generation.clone(),
        pdf: p.pdf.clone(),
    };
    toml::to_string(&raw).expect("project is always serializable")
}

pub fn read_project_file(path: &Path) -> Result<Project> {
    let text = std::fs::read_to_string(path)?;
    load_project(&text)
}

pub fn write_project_file(path: &Path, p: &Project) -> Result<()> {
    write_atomic(path, &save_project(p))
}
