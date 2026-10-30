use fap_core::format::{self, FileKind};
use fap_core::{CoreError, Pos, Project, RegionId};

const SAMPLE: &str = r#"
format = "fill-a-pix-project"
version = 1
width = 3
height = 2
title = "Sample"
future_key = 1

[image]
rows = """
#.#
.#.
"""
unknown_inner = true

[regions]
default_level = 3
rows = """
1 1 .
. 2 2
"""

[[region]]
id = 1
level = 1
margin = 2
name = "spine"

[[region]]
id = 2
level = 4
margin = 0

[generation]
seed = 42
"#;

#[test]
fn loads_sample_and_ignores_unknown_keys() {
    let p = format::load_project(SAMPLE).unwrap();
    assert_eq!((p.width(), p.height()), (3, 2));
    assert_eq!(p.title.as_deref(), Some("Sample"));
    assert!(*p.image.get(Pos::new(0, 0)) && !*p.image.get(Pos::new(0, 1)));
    assert_eq!(p.regions.default_level(), 3);
    assert_eq!(
        p.regions.settings(RegionId(1)).unwrap().name.as_deref(),
        Some("spine")
    );
    assert_eq!(p.regions.region_at(Pos::new(1, 2)), Some(RegionId(2)));
    assert_eq!(
        p.generation.get("seed").and_then(|v| v.as_integer()),
        Some(42)
    );
    assert!(p.pdf.is_empty());
    assert!(matches!(
        format::detect_kind(SAMPLE).unwrap(),
        FileKind::Project
    ));
}

#[test]
fn round_trip_and_multiline_rows() {
    let p = format::load_project(SAMPLE).unwrap();
    let text = format::save_project(&p);
    assert!(text.contains("format = \"fill-a-pix-project\""));
    assert!(text.contains("version = 1"));
    assert!(text.contains("rows = \"\"\"\n#.#\n.#.\n"), "{text}");
    assert_eq!(format::load_project(&text).unwrap(), p);
}

#[test]
fn new_project_round_trips_without_regions_section() {
    let p = Project::new(4, 2);
    let back = format::load_project(&format::save_project(&p)).unwrap();
    assert_eq!(back, p);
    let minimal = "format = \"fill-a-pix-project\"\nversion = 1\nwidth = 2\nheight = 1\n[image]\nrows = \"\"\"\n#.\n\"\"\"\n";
    let m = format::load_project(minimal).unwrap();
    assert!(m.regions.table().is_empty());
    assert_eq!(m.regions.default_level(), 2);
}

#[test]
fn disconnected_region_in_file_is_split() {
    let text = SAMPLE.replace("1 1 .\n. 2 2", "1 . 1\n. 2 2");
    let p = format::load_project(&text).unwrap();
    assert_eq!(p.regions.table().len(), 3);
}

#[test]
fn undefined_region_id_names_row_and_id() {
    let text = SAMPLE.replace(". 2 2", ". 2 9");
    let e = format::load_project(&text).unwrap_err().to_string();
    assert!(
        e.contains("region 9") && e.contains("row 2, column 3"),
        "{e}"
    );
}

#[test]
fn row_length_mismatch_names_section_and_row() {
    let text = SAMPLE.replace(".#.\n", ".#\n");
    let e = format::load_project(&text).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().contains("[image] row 2"), "{e}");
}

#[test]
fn puzzle_file_loaded_as_project_is_rejected_by_kind() {
    let text = SAMPLE.replace("fill-a-pix-project", "fill-a-pix-puzzle");
    let e = format::load_project(&text).unwrap_err();
    assert!(matches!(e, CoreError::Unsupported(_)));
    assert!(e.to_string().contains("found fill-a-pix-puzzle"), "{e}");
}

#[test]
fn newer_version_is_rejected() {
    let e = format::load_project(&SAMPLE.replace("version = 1", "version = 2")).unwrap_err();
    assert!(
        e.to_string()
            .contains("version 2 is newer than supported version 1"),
        "{e}"
    );
}

#[test]
fn toml_syntax_error() {
    assert!(matches!(
        format::load_project("format = "),
        Err(CoreError::Toml(_))
    ));
}

#[test]
fn file_helpers_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.project.toml");
    let mut p = Project::new(2, 2);
    p.regions = fap_core::Regions::strips(2, 2, 1, 1, 2, 1, 2);
    format::write_project_file(&path, &p).unwrap();
    assert_eq!(format::read_project_file(&path).unwrap(), p);
    assert!(!dir.path().join(".a.project.toml.tmp").exists());
}

#[test]
fn zero_dimensions_and_bad_region_settings_are_format_errors() {
    let e = format::load_project(&SAMPLE.replace("width = 3", "width = 0")).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)), "{e}");
    let e = format::load_project(&SAMPLE.replace("level = 4", "level = 9")).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().starts_with("[regions] "), "{e}");
}
