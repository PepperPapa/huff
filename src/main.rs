// 引入第三方包
use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
// 标准库路径
use std::path::PathBuf;

// #[...]叫属性
#[derive(Parser)]
#[command(name = "huff", version, about="A minimal Huffman file compressor.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Compress {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Decompress {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Compress { input, output } => {
            let output = output.unwrap_or_else(|| {
                PathBuf::from(format!("{}.huff", input.display()))
            });
            if !input.exists() {
                bail!("input file not found: {}", input.display());
            }
            println!("compress {} -> {}", input.display(), output.display());
        }
        Command::Decompress { input, output } => {
            let output = output.unwrap_or_else(|| {
                let s = input.to_string_lossy();
                let stripped = s.strip_suffix(".huff").unwrap_or(&s);
                PathBuf::from(stripped)
            });
            if !input.exists() {
                bail!("input file not found: {}", input.display());
            }
            println!("decompress {} -> {}", input.display(), output.display());
        }
    }
    Ok(())
}