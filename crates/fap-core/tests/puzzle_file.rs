use fap_core::format::{self, FileKind};
use fap_core::{Clue, CoreError, Grid, Pos, Puzzle, Regions};

fn sample() -> Puzzle {
    let img: fap_core::Image =
        Grid::from_rows(vec![vec![true, false, true], vec![false, true, false]]).unwrap();
    let mut p = Puzzle::fully_clued(&img, Regions::strips(3, 2, 3, 2, 2, 1, 2));
    p.title = Some("T".into());
    p.clues.set(Pos::new(0, 1), Clue::Blank);
    p.clues.set(Pos::new(0, 0), Clue::GivenShaded);
    p.clues.set(Pos::new(1, 0), Clue::GivenUnshaded);
    p.stats.insert("score".into(), toml::Value::Float(3.5));
    p
}

#[test]
fn round_trip() {
    let p = sample();
    let text = format::save_puzzle(&p);
    assert!(text.contains("format = \"fill-a-pix-puzzle\""));
    assert!(text.contains("rows = \"\"\"\n#.2\nx32\n"), "{text}");
    assert!(matches!(
        format::detect_kind(&text).unwrap(),
        FileKind::Puzzle
    ));
    assert_eq!(format::load_puzzle(&text).unwrap(), p);
}

#[test]
fn mismatched_clue_is_rejected_on_load() {
    let text = format::save_puzzle(&sample());
    let bad = text.replacen("#.2", "#.5", 1);
    assert_ne!(bad, text, "fixture must contain the clue row");
    let e = format::load_puzzle(&bad).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().contains("row 1, column 3"), "{e}");
}

#[test]
fn project_loaded_as_puzzle_is_rejected_by_kind() {
    let text = format::save_project(&fap_core::Project::new(2, 2));
    let e = format::load_puzzle(&text).unwrap_err();
    assert!(e.to_string().contains("found fill-a-pix-project"), "{e}");
}

#[test]
fn regions_section_is_optional() {
    let text = "format = \"fill-a-pix-puzzle\"\nversion = 1\nwidth = 1\nheight = 1\n\
                [clues]\nrows = \"\"\"\n1\n\"\"\"\n[solution]\nrows = \"\"\"\n#\n\"\"\"\n";
    let p = format::load_puzzle(text).unwrap();
    assert!(p.regions.table().is_empty());
}

#[test]
fn stats_datetime_survives_round_trip() {
    let text = format::save_puzzle(&sample())
        .replace("[stats]\n", "[stats]\nwhen = 2026-10-29T10:00:00Z\n");
    let p = format::load_puzzle(&text).unwrap();
    assert!(matches!(
        p.stats.get("when"),
        Some(toml::Value::Datetime(_))
    ));
    let again = format::load_puzzle(&format::save_puzzle(&p)).unwrap();
    assert!(matches!(
        again.stats.get("when"),
        Some(toml::Value::Datetime(_))
    ));
    assert_eq!(again, p);
}

#[test]
fn write_rejects_out_of_range_digit() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("a.puzzle.toml");
    let mut p = sample();
    format::write_puzzle_file(&path, &p).unwrap();
    let before = std::fs::read_to_string(&path).unwrap();
    p.clues.set(Pos::new(0, 1), Clue::Digit(12));
    assert!(format::write_puzzle_file(&path, &p).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}
