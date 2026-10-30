mod commands;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "fap", version, about = "Fill-a-Pix puzzle studio command line")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write a blank project
    New {
        #[arg(long)]
        width: usize,
        #[arg(long)]
        height: usize,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Create a project from a PBM image
    ImportPbm {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Export a project's image or a puzzle's solution as PBM
    ExportPbm {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Replace a project's regions with vertical strips
    Strips {
        input: PathBuf,
        #[arg(long)]
        strip_width: usize,
        #[arg(long = "start")]
        start_level: u8,
        #[arg(long = "end")]
        end_level: u8,
        #[arg(long)]
        margin: u8,
        /// Output file (defaults to modifying the input in place)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Write a puzzle with every cell clued
    ClueAll {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Print a summary of a project or puzzle file
    Info { input: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::New {
            width,
            height,
            output,
        } => commands::new(width, height, &output),
        Command::ImportPbm { input, output } => commands::import_pbm(&input, &output),
        Command::ExportPbm { input, output } => commands::export_pbm(&input, &output),
        Command::Strips {
            input,
            strip_width,
            start_level,
            end_level,
            margin,
            output,
        } => commands::strips(
            &input,
            strip_width,
            start_level,
            end_level,
            margin,
            output.as_deref(),
        ),
        Command::ClueAll { input, output } => commands::clue_all(&input, &output),
        Command::Info { input } => commands::info(&input),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}
