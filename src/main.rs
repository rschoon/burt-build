use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Parser;

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
    #[clap(short, long, default_value="build.burt")]
    file: PathBuf
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let burtfile = read_burt_file(&args.file)?; 

    let mut build = builder::Build::new();
    build.build(&burtfile, "default")?;

    Ok(())
}
