//! Netpbm bitmap (PBM) reading (P1, P4) and writing (P4). `1` is black, i.e. shaded.

use std::path::Path;

use crate::{CoreError, Grid, Image, Pos, Result};

fn err<T>(msg: impl Into<String>) -> Result<T> {
    Err(CoreError::Pbm(msg.into()))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    /// Skips whitespace and `#` comments (to end of line).
    fn skip_blank(&mut self) {
        while let Some(&b) = self.bytes.get(self.at) {
            if b.is_ascii_whitespace() {
                self.at += 1;
            } else if b == b'#' {
                while self.bytes.get(self.at).is_some_and(|&c| c != b'\n') {
                    self.at += 1;
                }
            } else {
                break;
            }
        }
    }

    fn token(&mut self) -> Option<&[u8]> {
        self.skip_blank();
        let start = self.at;
        while self
            .bytes
            .get(self.at)
            .is_some_and(|b| !b.is_ascii_whitespace() && *b != b'#')
        {
            self.at += 1;
        }
        (self.at > start).then(|| &self.bytes[start..self.at])
    }

    fn dimension(&mut self, what: &str) -> Result<usize> {
        let Some(tok) = self.token() else {
            return err(format!("missing {what}"));
        };
        let text = std::str::from_utf8(tok)
            .ok()
            .filter(|t| t.bytes().all(|b| b.is_ascii_digit()));
        match text.and_then(|t| t.parse::<usize>().ok()) {
            Some(0) => err(format!("{what} must be positive")),
            Some(n) => Ok(n),
            None => err(format!("invalid {what}")),
        }
    }
}

/// Parses a P1 or P4 PBM image.
pub fn read_pbm(bytes: &[u8]) -> Result<Image> {
    let mut cur = Cursor { bytes, at: 0 };
    let magic = cur.token().map(|t| String::from_utf8_lossy(t).into_owned());
    let p4 = match magic.as_deref() {
        Some("P1") => false,
        Some("P4") => true,
        Some(other) => return err(format!("unsupported PBM type {other}")),
        None => return err("empty PBM data"),
    };
    let width = cur.dimension("width")?;
    let height = cur.dimension("height")?;
    let row_bytes = width.div_ceil(8);
    let too_big = || CoreError::Pbm("dimensions too large".into());
    let raster = row_bytes.checked_mul(height).ok_or_else(too_big)?;
    let pixels = width.checked_mul(height).ok_or_else(too_big)?;
    // Each P4 byte / P1 pixel needs input, so check before allocating.
    let remaining = bytes.len() - cur.at;
    if remaining < if p4 { raster + 1 } else { pixels } {
        return err("truncated PBM data");
    }

    let mut img = Grid::new(width, height, false);
    if p4 {
        // Exactly one whitespace byte follows the height.
        if !cur.bytes.get(cur.at).is_some_and(u8::is_ascii_whitespace) {
            return err("missing separator after header");
        }
        let data = &bytes[cur.at + 1..];
        if data.len() < row_bytes * height {
            return err("truncated P4 data");
        }
        for r in 0..height {
            for c in 0..width {
                let byte = data[r * row_bytes + c / 8];
                if byte & (0x80 >> (c % 8)) != 0 {
                    img.set(Pos::new(r, c), true);
                }
            }
        }
    } else {
        let mut n = 0;
        let total = width * height;
        loop {
            cur.skip_blank();
            let Some(&b) = cur.bytes.get(cur.at) else {
                break;
            };
            cur.at += 1;
            if b != b'0' && b != b'1' {
                return err(format!("invalid character {:?} in P1 data", b as char));
            }
            if n < total {
                img.set(Pos::new(n / width, n % width), b == b'1');
            }
            n += 1;
        }
        if n < total {
            return err("truncated P1 data");
        }
    }
    Ok(img)
}

/// Serialises as binary P4.
pub fn write_pbm(img: &Image) -> Vec<u8> {
    let mut out = format!("P4\n{} {}\n", img.width(), img.height()).into_bytes();
    for r in 0..img.height() {
        let mut row = vec![0u8; img.width().div_ceil(8)];
        for (c, &on) in img.row(r).iter().enumerate() {
            if on {
                row[c / 8] |= 0x80 >> (c % 8);
            }
        }
        out.extend(row);
    }
    out
}

pub fn read_pbm_file(path: &Path) -> Result<Image> {
    read_pbm(&std::fs::read(path)?)
}

pub fn write_pbm_file(path: &Path, img: &Image) -> Result<()> {
    Ok(std::fs::write(path, write_pbm(img))?)
}
