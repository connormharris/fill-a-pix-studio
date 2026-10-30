use fap_core::format::{
    FileKind, detect_kind, read_project_file, read_puzzle_file, write_project_file,
    write_puzzle_file,
};
use fap_core::pbm::{read_pbm_file, write_pbm_file};
use fap_core::{Clue, Image, Project, Puzzle, Regions};
use std::fmt::{Display, Write as _};
use std::path::Path;

type CmdResult = Result<(), String>;

fn at<T, E: Display>(path: &Path, r: Result<T, E>) -> Result<T, String> {
    r.map_err(|e| format!("{}: {}", path.display(), e))
}

fn read_text(path: &Path) -> Result<String, String> {
    at(path, std::fs::read_to_string(path))
}

pub fn new(width: usize, height: usize, output: &Path) -> CmdResult {
    if width == 0 || height == 0 {
        return Err("width and height must be at least 1".into());
    }
    at(
        output,
        write_project_file(output, &Project::new(width, height)),
    )
}

pub fn import_pbm(input: &Path, output: &Path) -> CmdResult {
    let image = at(input, read_pbm_file(input))?;
    let mut project = Project::new(image.width(), image.height());
    project.image = image;
    at(output, write_project_file(output, &project))
}

pub fn export_pbm(input: &Path, output: &Path) -> CmdResult {
    let kind = at(input, detect_kind(&read_text(input)?))?;
    let image = match kind {
        FileKind::Project => at(input, read_project_file(input))?.image,
        FileKind::Puzzle => at(input, read_puzzle_file(input))?.solution,
    };
    at(output, write_pbm_file(output, &image))
}

pub fn strips(
    input: &Path,
    strip_width: usize,
    start_level: u8,
    end_level: u8,
    margin: u8,
    output: Option<&Path>,
) -> CmdResult {
    if strip_width == 0 {
        return Err("strip width must be at least 1".into());
    }
    for (level, what) in [(start_level, "start"), (end_level, "end")] {
        if !(1..=4).contains(&level) {
            return Err(format!("{what} level must be between 1 and 4"));
        }
    }
    if margin > 2 {
        return Err("margin must be between 0 and 2".into());
    }
    let mut project = at(input, read_project_file(input))?;
    project.regions = Regions::strips(
        project.width(),
        project.height(),
        strip_width,
        start_level,
        end_level,
        margin,
        project.regions.default_level(),
    );
    let out = output.unwrap_or(input);
    at(out, write_project_file(out, &project))
}

pub fn clue_all(input: &Path, output: &Path) -> CmdResult {
    let project = at(input, read_project_file(input))?;
    let mut puzzle = Puzzle::fully_clued(&project.image, project.regions);
    puzzle.title = project.title;
    at(output, write_puzzle_file(output, &puzzle))
}

pub fn info(input: &Path) -> CmdResult {
    let kind = at(input, detect_kind(&read_text(input)?))?;
    let text = match kind {
        FileKind::Project => {
            let p = at(input, read_project_file(input))?;
            summary("project", p.title.as_deref(), &p.image, &p.regions, None)
        }
        FileKind::Puzzle => {
            let p = at(input, read_puzzle_file(input))?;
            let digits = p
                .clues
                .positions()
                .filter(|&q| matches!(p.clues.get(q), Clue::Digit(_)))
                .count();
            let givens = p
                .clues
                .positions()
                .filter(|&q| matches!(p.clues.get(q), Clue::GivenShaded | Clue::GivenUnshaded))
                .count();
            summary(
                "puzzle",
                p.title.as_deref(),
                &p.solution,
                &p.regions,
                Some((digits, givens)),
            )
        }
    };
    print!("{text}");
    Ok(())
}

fn summary(
    label: &str,
    title: Option<&str>,
    image: &Image,
    regions: &Regions,
    clues: Option<(usize, usize)>,
) -> String {
    let shaded = image.positions().filter(|&q| *image.get(q)).count();
    let mut s = String::new();
    let _ = writeln!(s, "{label} {}x{}", image.width(), image.height());
    let _ = writeln!(s, "title: {}", title.unwrap_or("(none)"));
    let _ = writeln!(s, "shaded: {shaded}");
    if let Some((digits, givens)) = clues {
        let _ = writeln!(s, "digits: {digits}");
        let _ = writeln!(s, "givens: {givens}");
    }
    let _ = writeln!(s, "default level: {}", regions.default_level());
    let _ = writeln!(s, "regions: {}", regions.table().len());
    for (id, r) in regions.table() {
        let _ = write!(
            s,
            "  region {}: {} cells, level {}, margin {}",
            id.0,
            regions.cells(*id).len(),
            r.level,
            r.margin
        );
        if let Some(name) = &r.name {
            let _ = write!(s, ", name \"{name}\"");
        }
        s.push('\n');
    }
    s
}
