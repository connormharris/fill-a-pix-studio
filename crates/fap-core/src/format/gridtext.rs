//! Text codecs for the multi-line row strings used in project and puzzle files.

use crate::{Clue, CoreError, Grid, Image, Pos, RegionId, Result};

fn fmt_err<T>(msg: String) -> Result<T> {
    Err(CoreError::Format(msg))
}

/// Split into trimmed, non-blank lines.
fn lines(text: &str) -> Vec<&str> {
    text.split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect()
}

fn check_rows(n: usize, height: usize) -> Result<()> {
    if n != height {
        return fmt_err(format!("expected {height} rows, found {n}"));
    }
    Ok(())
}

fn check_cells(r: usize, n: usize, width: usize) -> Result<()> {
    if n != width {
        return fmt_err(format!("row {}: expected {width} cells, found {n}", r + 1));
    }
    Ok(())
}

fn parse_char_grid<T>(
    text: &str,
    width: usize,
    height: usize,
    cell: impl Fn(char) -> Option<T>,
) -> Result<Grid<T>> {
    let ls = lines(text);
    check_rows(ls.len(), height)?;
    let mut rows = Vec::with_capacity(height);
    for (r, line) in ls.iter().enumerate() {
        check_cells(r, line.chars().count(), width)?;
        let mut row = Vec::with_capacity(width);
        for (c, ch) in line.chars().enumerate() {
            match cell(ch) {
                Some(v) => row.push(v),
                None => {
                    return fmt_err(format!(
                        "row {}, column {}: unexpected character '{ch}'",
                        r + 1,
                        c + 1
                    ));
                }
            }
        }
        rows.push(row);
    }
    Grid::from_rows(rows)
}

fn emit_char_grid<T>(g: &Grid<T>, ch: impl Fn(&T) -> char) -> String {
    let mut s = String::new();
    for r in 0..g.height() {
        s.extend(g.row(r).iter().map(&ch));
        s.push('\n');
    }
    s
}

pub fn parse_image_rows(text: &str, width: usize, height: usize) -> Result<Image> {
    parse_char_grid(text, width, height, |ch| match ch {
        '.' => Some(false),
        '#' => Some(true),
        _ => None,
    })
}

pub fn image_rows(img: &Image) -> String {
    emit_char_grid(img, |&b| if b { '#' } else { '.' })
}

pub fn parse_clue_rows(text: &str, width: usize, height: usize) -> Result<Grid<Clue>> {
    parse_char_grid(text, width, height, |ch| match ch {
        '.' => Some(Clue::Blank),
        '#' => Some(Clue::GivenShaded),
        'x' => Some(Clue::GivenUnshaded),
        '0'..='9' => Some(Clue::Digit(ch as u8 - b'0')),
        _ => None,
    })
}

pub fn clue_rows(c: &Grid<Clue>) -> String {
    emit_char_grid(c, |clue| match clue {
        Clue::Blank => '.',
        Clue::GivenShaded => '#',
        Clue::GivenUnshaded => 'x',
        Clue::Digit(d) => (b'0' + d) as char,
    })
}

pub fn parse_region_rows(
    text: &str,
    width: usize,
    height: usize,
) -> Result<Grid<Option<RegionId>>> {
    let ls = lines(text);
    check_rows(ls.len(), height)?;
    let mut rows = Vec::with_capacity(height);
    for (r, line) in ls.iter().enumerate() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        check_cells(r, toks.len(), width)?;
        let mut row = Vec::with_capacity(width);
        for (c, tok) in toks.iter().enumerate() {
            if *tok == "." {
                row.push(None);
                continue;
            }
            match tok.parse::<u32>() {
                Ok(n) if n > 0 && tok.bytes().all(|b| b.is_ascii_digit()) => {
                    row.push(Some(RegionId(n)))
                }
                _ => {
                    return fmt_err(format!(
                        "row {}, column {}: invalid region token '{tok}'",
                        r + 1,
                        c + 1
                    ));
                }
            }
        }
        rows.push(row);
    }
    Grid::from_rows(rows)
}

pub fn region_rows(m: &Grid<Option<RegionId>>) -> String {
    let token = |p: Pos| match m.get(p) {
        Some(id) => id.0.to_string(),
        None => ".".to_string(),
    };
    let w = m.positions().map(|p| token(p).len()).max().unwrap_or(1);
    let mut s = String::new();
    for r in 0..m.height() {
        let row: Vec<String> = (0..m.width())
            .map(|c| format!("{:>w$}", token(Pos::new(r, c))))
            .collect();
        s.push_str(&row.join(" "));
        s.push('\n');
    }
    s
}
