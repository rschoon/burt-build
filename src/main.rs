use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};

mod builder;
mod file;

fn read_burt_file(path: &Path) -> anyhow::Result<file::RootSection> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?;
    let result = file::parse_reader(file)
        .with_context(|| format!("Failed to read data from {}", path.display()))?;
    Ok(result)
}

#[derive(Parser)]
struct Args {
    /// source file
    #[clap(short, long, default_value="build.burt")]
    file: PathBuf,

    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand)]
enum Command {
    Build {
        targets: Vec<String>,
    }
}

fn build_targets(burtfile: file::RootSection, targets: Vec<String>) -> anyhow::Result<()> {
    for target in targets {
        if target.starts_with("+") {
            let mut build = builder::Build::new();
            build.build(&burtfile, &target[1..])?;
        } else {
            anyhow::bail!("Unknown target {}", target);
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let burtfile = read_burt_file(&args.file)?; 
    match args.command {
        Command::Build { targets } => build_targets(burtfile, targets)
    }
}

