use fap_core::pbm::{read_pbm, read_pbm_file, write_pbm, write_pbm_file};
use fap_core::{CoreError, Grid, Image, Pos};

fn ten_by_two() -> Image {
    let mut g = Grid::new(10, 2, false);
    g.set(Pos::new(0, 0), true);
    g.set(Pos::new(0, 9), true);
    g
}

#[test]
fn writes_p4_exact_bytes() {
    let mut expected = b"P4\n10 2\n".to_vec();
    expected.extend_from_slice(&[0x80, 0x40, 0x00, 0x00]);
    assert_eq!(write_pbm(&ten_by_two()), expected);
}

#[test]
fn p4_round_trip() {
    assert_eq!(read_pbm(&write_pbm(&ten_by_two())).unwrap(), ten_by_two());
}

#[test]
fn p4_padding_bits_ignored() {
    let mut bytes = b"P4\n# made elsewhere\n10 2\n".to_vec();
    bytes.extend_from_slice(&[0x80, 0x7F, 0x00, 0x3F]);
    assert_eq!(read_pbm(&bytes).unwrap(), ten_by_two());
}

#[test]
fn p1_comments_and_unseparated_digits() {
    let img = read_pbm(b"P1\n# comment\n3 2\n101\n0 1 # trailing\n 0\n").unwrap();
    let expect: Image =
        Grid::from_rows(vec![vec![true, false, true], vec![false, true, false]]).unwrap();
    assert_eq!(img, expect);
}

#[test]
fn errors() {
    let e = read_pbm(b"P5\n1 1\n255\n\0").unwrap_err();
    assert!(
        matches!(e, CoreError::Pbm(_)) && e.to_string().contains("P5"),
        "{e}"
    );
    assert!(matches!(
        read_pbm(b"P4\n10 2\n\x80"),
        Err(CoreError::Pbm(_))
    ));
    assert!(matches!(read_pbm(b"P1\n0 2\n"), Err(CoreError::Pbm(_))));
    assert!(matches!(read_pbm(b"P1\n2 1\n1"), Err(CoreError::Pbm(_))));
}

#[test]
fn rejects_overflow_bad_chars_and_ignores_trailing() {
    let huge = format!("P4\n{} {}\n", usize::MAX, usize::MAX);
    assert!(matches!(read_pbm(huge.as_bytes()), Err(CoreError::Pbm(_))));
    assert!(matches!(read_pbm(b"P1\n2 1\n1x"), Err(CoreError::Pbm(_))));
    let mut bytes = write_pbm(&ten_by_two());
    bytes.extend_from_slice(b"junk");
    assert_eq!(read_pbm(&bytes).unwrap(), ten_by_two());
}

#[test]
fn file_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.pbm");
    write_pbm_file(&path, &ten_by_two()).unwrap();
    assert_eq!(read_pbm_file(&path).unwrap(), ten_by_two());
}

#[test]
fn huge_dimensions_without_data_error_instead_of_allocating() {
    assert!(matches!(
        read_pbm(b"P4\n100000 100000\n"),
        Err(CoreError::Pbm(_))
    ));
    assert!(matches!(
        read_pbm(b"P1\n100000 100000\n"),
        Err(CoreError::Pbm(_))
    ));
}

#[test]
fn write_pbm_file_leaves_no_temp_file() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("a.pbm");
    write_pbm_file(&path, &ten_by_two()).unwrap();
    assert_eq!(read_pbm_file(&path).unwrap(), ten_by_two());
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 1);
}
