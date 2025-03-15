
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::LazyLock;

use anyhow::Context;
use builder::BurtCache;
use clap::{Parser, Subcommand};

mod builder;
mod file;

fn current_exe() -> &'static Path {
    static CE: LazyLock<PathBuf> = LazyLock::new(|| {
        if let Ok(p) = std::env::current_exe() {
            p
        } else {
            PathBuf::from("burt")
        }
    });
    &CE
}


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
    #[clap(short, long, default_value="build.burt", global=true)]
    file: PathBuf,

    /// export artifacts
    #[clap(long, short('a'), global=true)]
    artifact: bool,

    /// set build value
    #[clap(long, short('D'), global=true)]
    define: Vec<String>,

    #[command(subcommand)]
    command: Command
}

#[derive(Subcommand)]
enum Command {
    Build(BuildArgs),
    #[clap(hide(true))]
    InternalContainerCopy {
        src: PathBuf,
        dest: PathBuf,
    },
    #[clap(hide(true))]
    InternalExport {
        path: PathBuf,
    },
    #[clap(hide(true))]
    InternalImportTar {
        path: PathBuf,
    },
    // alias for build
    #[clap(external_subcommand)]
    TopDefault(Vec<OsString>)
}

#[derive(Debug, Parser)]
struct BuildArgs {
    targets: Vec<String>,  
}

fn build_targets(path: &Path, targets: Vec<String>, export_artifacts: bool, defines: Vec<String>) -> anyhow::Result<()> {
    let burt_cache = Rc::new(BurtCache::default());

    for target in targets {
        if let Some(target) = target.strip_prefix('+') {
            let mut build = builder::Build::new(burt_cache.clone());

            for define in &defines {
                if let Some((k, v)) = define.split_once('=') {
                    build.set(k, v);
                } else {
                    build.set(&define, "");
                }
            }

            build.build(path, target)?;

            if export_artifacts {
                build.export_artifact(".")?;
            }
        } else {
            anyhow::bail!("Unknown target {}", target);
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Build(build_args) => build_targets(&args.file, build_args.targets, args.artifact, args.define),
        Command::TopDefault(mut build_args) => {
            build_args.insert(0, OsString::new());
            let build_args = BuildArgs::parse_from(build_args);
            build_targets(&args.file, build_args.targets, args.artifact, args.define)
        },
        Command::InternalContainerCopy { src, dest } => {
            builder::perform_container_copy(&src, &dest)
        },
        Command::InternalExport { path } => {
            builder::perform_container_export(&path)
        },
        Command::InternalImportTar { path } => {
            builder::perform_container_import_tar(std::io::stdin().lock(), &path)
        }
    }
}

