use fap_core::{Clue, CoreError, Grid, Image, Pos, Puzzle, Regions, clues};

fn img(rows: &[&str]) -> Image {
    Grid::from_rows(
        rows.iter()
            .map(|r| r.chars().map(|c| c == '#').collect())
            .collect(),
    )
    .unwrap()
}

#[test]
fn neighborhood_sizes_corner_edge_interior() {
    assert_eq!(clues::neighborhood(3, 3, Pos::new(0, 0)).count(), 4);
    assert_eq!(clues::neighborhood(3, 3, Pos::new(0, 1)).count(), 6);
    assert_eq!(clues::neighborhood(3, 3, Pos::new(1, 1)).count(), 9);
    assert!(clues::neighborhood(3, 3, Pos::new(1, 1)).any(|p| p == Pos::new(1, 1)));
    assert_eq!(clues::neighborhood(1, 1, Pos::new(0, 0)).count(), 1);
}

#[test]
fn off_grid_counts_as_unshaded() {
    let full = img(&["###", "###", "###"]);
    assert_eq!(clues::clue_at(&full, Pos::new(0, 0)), 4);
    assert_eq!(clues::clue_at(&full, Pos::new(0, 1)), 6);
    assert_eq!(clues::clue_at(&full, Pos::new(1, 1)), 9);
    assert_eq!(clues::clue_at(&img(&["#"]), Pos::new(0, 0)), 1);
}

#[test]
fn clue_counts_own_cell() {
    let i = img(&["...", ".#.", "..."]);
    let c = clues::all_clues(&i);
    assert!(i.positions().all(|p| *c.get(p) == 1));
}

#[test]
fn two_by_two_clues_all_equal_total() {
    let c = clues::all_clues(&img(&["#.", ".#"]));
    assert!(c.positions().all(|p| *c.get(p) == 2));
}

#[test]
fn fully_clued_puzzle_validates() {
    let i = img(&["#..#", ".##.", "#..#"]);
    let p = Puzzle::fully_clued(&i, Regions::new(4, 3, 2));
    assert_eq!(
        *p.clues.get(Pos::new(1, 1)),
        Clue::Digit(clues::clue_at(&i, Pos::new(1, 1)))
    );
    assert!(p.validate().is_ok());
}

#[test]
fn validate_rejects_wrong_digit_and_wrong_given() {
    let i = img(&["#.", ".."]);
    let mut p = Puzzle::fully_clued(&i, Regions::new(2, 2, 2));
    p.clues.set(Pos::new(0, 1), Clue::Digit(3));
    let e = p.validate().unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(e.to_string().contains("row 1, column 2"), "{e}");

    let mut p = Puzzle::fully_clued(&i, Regions::new(2, 2, 2));
    p.clues.set(Pos::new(1, 1), Clue::GivenShaded);
    assert!(
        p.validate()
            .unwrap_err()
            .to_string()
            .contains("row 2, column 2")
    );
    p.clues.set(Pos::new(1, 1), Clue::GivenUnshaded);
    assert!(p.validate().is_ok());
}

#[test]
fn validate_rejects_dimension_mismatch() {
    let mut p = Puzzle::fully_clued(&img(&["#.", ".."]), Regions::new(2, 2, 2));
    p.solution = img(&["#..", "..."]);
    assert!(matches!(p.validate(), Err(CoreError::Format(_))));
}
