
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Args {
    #[clap(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Command
}

impl Args {
    pub fn parse() -> Self {
        let result = Parser::try_parse();
        let err = match result {
            Ok(r) => return r,
            Err(e) => e,
        };

        if let Ok(r) = BuildDefaultCommand::try_parse() {
            return Self {
                global: r.global,
                command: Command::Build(r.args)
            }
        }

        err.exit()
    }
}

#[derive(Subcommand)]
pub enum Command {
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
}

#[derive(Parser)]
struct BuildDefaultCommand {
    #[clap(flatten)]
    global: GlobalArgs,

    #[clap(flatten)]
    args: BuildArgs
}

#[derive(Parser)]
pub struct GlobalArgs {
    /// export artifacts
    #[clap(long, short('a'), global=true)]
    pub artifact: bool,

    /// set build value
    #[clap(long, short('D'), global=true)]
    pub define: Vec<String>,

    /// source file
    #[clap(short, long, default_value="build.burt", global=true)]
    pub file: PathBuf,
}

#[derive(Parser)]
pub struct BuildArgs {
    pub targets: Vec<String>, 
}
