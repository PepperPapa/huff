use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

mod app;
mod bitio;
mod codec;
mod format;

/// A minimal Huffman file compressor.
#[derive(Parser)]
#[command(name = "huff", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compress a file into .huff
    Compress {
        /// Input file
        input: PathBuf,
        /// Output file (default: <input>.huff)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Overwrite output if it exists
        #[arg(short, long)]
        force: bool,
    },
    /// Decompress a .huff file
    Decompress {
        /// Input .huff file
        input: PathBuf,
        /// Output file (default: strip .huff suffix)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Overwrite output if it exists
        #[arg(short, long)]
        force: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Compress { input, output, force } => {
            let output = output.unwrap_or_else(|| append_suffix(&input, ".huff"));
            app::compress_file(&input, &output, force)?;
        }
        Command::Decompress { input, output, force } => {
            let output = match output {
                Some(p) => p,
                None => strip_huff_suffix(&input)?,
            };
            app::decompress_file(&input, &output, force)?;
        }
    }
    Ok(())
}

fn append_suffix(input: &Path, suffix: &str) -> PathBuf {
    let mut s = input.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn strip_huff_suffix(input: &Path) -> Result<PathBuf> {
    let s = input.as_os_str().to_string_lossy();
    let stripped = s.strip_suffix(".huff").ok_or_else(|| {
        anyhow!(
            "cannot infer output name from {} (no .huff suffix); pass --output",
            input.display()
        )
    })?;
    Ok(PathBuf::from(stripped))
}