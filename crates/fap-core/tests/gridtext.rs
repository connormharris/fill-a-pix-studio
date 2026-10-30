use fap_core::format::gridtext::*;
use fap_core::{Clue, Grid, Pos, RegionId};

#[test]
fn image_round_trip() {
    let img = parse_image_rows("\n.#\n#.\n", 2, 2).unwrap();
    assert!(*img.get(Pos::new(0, 1)) && !*img.get(Pos::new(0, 0)));
    assert_eq!(image_rows(&img), ".#\n#.\n");
}

#[test]
fn crlf_and_indentation_load_identically() {
    let clean = parse_image_rows(".#\n#.\n", 2, 2).unwrap();
    assert_eq!(
        parse_image_rows("\r\n   .#  \r\n\t#.\r\n\r\n", 2, 2).unwrap(),
        clean
    );
    assert_eq!(parse_image_rows(".#\n#.", 2, 2).unwrap(), clean);
}

#[test]
fn row_errors_are_one_based_and_specific() {
    let e = parse_image_rows(".#\n#..\n", 2, 2).unwrap_err().to_string();
    assert!(e.contains("row 2: expected 2 cells, found 3"), "{e}");
    let e = parse_image_rows(".#\n", 2, 2).unwrap_err().to_string();
    assert!(e.contains("expected 2 rows, found 1"), "{e}");
    let e = parse_image_rows(".q\n..\n", 2, 2).unwrap_err().to_string();
    assert!(
        e.contains("row 1, column 2: unexpected character 'q'"),
        "{e}"
    );
}

#[test]
fn clue_chars() {
    let c = parse_clue_rows("2.#x9\n", 5, 1).unwrap();
    assert_eq!(
        c.row(0),
        &[
            Clue::Digit(2),
            Clue::Blank,
            Clue::GivenShaded,
            Clue::GivenUnshaded,
            Clue::Digit(9)
        ]
    );
    assert_eq!(clue_rows(&c), "2.#x9\n");
    assert!(parse_clue_rows("X\n", 1, 1).is_err());
}

#[test]
fn region_tokens() {
    let m = parse_region_rows(" 1   10\n.  2\n", 2, 2).unwrap();
    assert_eq!(m.row(0), &[Some(RegionId(1)), Some(RegionId(10))]);
    assert_eq!(m.row(1), &[None, Some(RegionId(2))]);
    assert_eq!(region_rows(&m), " 1 10\n .  2\n");
    let e = parse_region_rows("1 a\n", 2, 1).unwrap_err().to_string();
    assert!(
        e.contains("row 1, column 2: invalid region token 'a'"),
        "{e}"
    );
    assert!(parse_region_rows("0\n", 1, 1).is_err(), "IDs are positive");
}

#[test]
fn empty_region_map_emits_dots() {
    let m: Grid<Option<RegionId>> = Grid::new(3, 1, None);
    assert_eq!(region_rows(&m), ". . .\n");
}

#[test]
fn region_token_edge_cases() {
    assert!(parse_region_rows("4294967296\n", 1, 1).is_err());
    assert!(parse_region_rows("+1\n", 1, 1).is_err());
}
