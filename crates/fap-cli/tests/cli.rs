use assert_cmd::Command;
use predicates::str::contains;

fn fap() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("fap"))
}

#[test]
fn new_then_info() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("a.project.toml");
    fap()
        .args(["new", "--width", "4", "--height", "3", "-o"])
        .arg(&p)
        .assert()
        .success();
    fap()
        .arg("info")
        .arg(&p)
        .assert()
        .success()
        .stdout(contains("project 4x3"))
        .stdout(contains("shaded: 0"))
        .stdout(contains("regions: 0"));
}

#[test]
fn pbm_import_export_round_trip() {
    let d = tempfile::tempdir().unwrap();
    let (pbm, proj, out) = (
        d.path().join("i.pbm"),
        d.path().join("i.project.toml"),
        d.path().join("o.pbm"),
    );
    let mut bytes = b"P4\n10 2\n".to_vec();
    bytes.extend_from_slice(&[0x80, 0x40, 0x00, 0x00]);
    std::fs::write(&pbm, &bytes).unwrap();
    fap()
        .arg("import-pbm")
        .arg(&pbm)
        .arg("-o")
        .arg(&proj)
        .assert()
        .success();
    fap()
        .arg("info")
        .arg(&proj)
        .assert()
        .stdout(contains("shaded: 2"));
    fap()
        .arg("export-pbm")
        .arg(&proj)
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    assert_eq!(std::fs::read(&out).unwrap(), bytes);
}

#[test]
fn strips_in_place() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("s.project.toml");
    fap()
        .args(["new", "--width", "23", "--height", "2", "-o"])
        .arg(&p)
        .assert()
        .success();
    fap()
        .arg("strips")
        .arg(&p)
        .args([
            "--strip-width",
            "5",
            "--start",
            "1",
            "--end",
            "4",
            "--margin",
            "2",
        ])
        .assert()
        .success();
    fap()
        .arg("info")
        .arg(&p)
        .assert()
        .success()
        .stdout(contains("regions: 4"))
        .stdout(contains("region 1: 10 cells, level 1, margin 2"))
        .stdout(contains("region 4: 16 cells, level 4, margin 2"));
}

#[test]
fn clue_all_produces_valid_puzzle_and_solution_exports() {
    let d = tempfile::tempdir().unwrap();
    let (proj, puz, out) = (
        d.path().join("c.project.toml"),
        d.path().join("c.puzzle.toml"),
        d.path().join("c.pbm"),
    );
    fap()
        .args(["new", "--width", "4", "--height", "3", "-o"])
        .arg(&proj)
        .assert()
        .success();
    fap()
        .arg("clue-all")
        .arg(&proj)
        .arg("-o")
        .arg(&puz)
        .assert()
        .success();
    fap()
        .arg("info")
        .arg(&puz)
        .assert()
        .success()
        .stdout(contains("puzzle 4x3"))
        .stdout(contains("digits: 12"))
        .stdout(contains("givens: 0"));
    fap()
        .arg("export-pbm")
        .arg(&puz)
        .arg("-o")
        .arg(&out)
        .assert()
        .success();
    assert!(std::fs::read(&out).unwrap().starts_with(b"P4\n4 3\n"));
}

#[test]
fn missing_file_reports_path() {
    fap()
        .args(["info", "/nonexistent/x.project.toml"])
        .assert()
        .code(1)
        .stderr(contains("error:"))
        .stderr(contains("/nonexistent/x.project.toml"));
}

#[test]
fn strips_rejects_out_of_range_margin() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("s.project.toml");
    fap()
        .args(["new", "--width", "10", "--height", "2", "-o"])
        .arg(&p)
        .assert()
        .success();
    fap()
        .arg("strips")
        .arg(&p)
        .args([
            "--strip-width",
            "5",
            "--start",
            "1",
            "--end",
            "4",
            "--margin",
            "3",
        ])
        .assert()
        .code(1)
        .stderr(contains("error:"));
}
