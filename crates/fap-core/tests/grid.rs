use fap_core::{CoreError, Grid, Pos};

#[test]
fn new_grid_has_dimensions_and_fill() {
    let g = Grid::new(3, 2, false);
    assert_eq!((g.width(), g.height()), (3, 2));
    assert_eq!(g.positions().count(), 6);
    assert!(g.positions().all(|p| !*g.get(p)));
}

#[test]
fn positions_are_row_major() {
    let g = Grid::new(2, 2, 0u8);
    let v: Vec<Pos> = g.positions().collect();
    assert_eq!(
        v,
        vec![
            Pos::new(0, 0),
            Pos::new(0, 1),
            Pos::new(1, 0),
            Pos::new(1, 1)
        ]
    );
}

#[test]
fn set_get_and_row() {
    let mut g = Grid::new(3, 2, 0u8);
    g.set(Pos::new(1, 2), 7);
    *g.get_mut(Pos::new(0, 0)) = 1;
    assert_eq!(*g.get(Pos::new(1, 2)), 7);
    assert_eq!(g.row(0), &[1, 0, 0]);
    assert_eq!(g.row(1), &[0, 0, 7]);
}

#[test]
fn from_rows_builds_and_rejects_bad_shapes() {
    let g = Grid::from_rows(vec![vec![1, 2, 3], vec![4, 5, 6]]).unwrap();
    assert_eq!((g.width(), g.height()), (3, 2));
    assert_eq!(*g.get(Pos::new(1, 0)), 4);
    assert!(matches!(
        Grid::from_rows(vec![vec![1, 2], vec![3]]),
        Err(CoreError::Format(_))
    ));
    assert!(matches!(
        Grid::<u8>::from_rows(vec![]),
        Err(CoreError::Format(_))
    ));
    assert!(matches!(
        Grid::<u8>::from_rows(vec![vec![]]),
        Err(CoreError::Format(_))
    ));
}

#[test]
fn pos_at_checks_bounds() {
    let g = Grid::new(3, 2, ());
    assert_eq!(g.pos_at(1, 2), Some(Pos::new(1, 2)));
    assert_eq!(g.pos_at(-1, 0), None);
    assert_eq!(g.pos_at(0, -1), None);
    assert_eq!(g.pos_at(2, 0), None);
    assert_eq!(g.pos_at(0, 3), None);
}

#[test]
fn map_preserves_shape() {
    let g = Grid::new(2, 3, 5u8).map(|v| *v as u32 * 2);
    assert_eq!((g.width(), g.height()), (2, 3));
    assert_eq!(*g.get(Pos::new(2, 1)), 10u32);
}

#[test]
fn pos_describe_is_one_based() {
    assert_eq!(Pos::new(0, 4).describe(), "row 1, column 5");
}

#[test]
#[should_panic]
fn zero_size_panics() {
    let _ = Grid::new(0, 3, false);
}
