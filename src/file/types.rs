
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub struct RootSection {
    pub targets: HashMap<String, TargetSection>
}

#[derive(Debug)]
pub struct TargetSection {
    pub commands: Vec<Command>,
}

#[derive(Debug)]
pub enum Command {
    From(FromCommand),
    Run(RunCommand),
    WorkDir(WorkDirCommand),
    SaveArtifact(SaveArtifactCommand),
}

#[derive(Debug)]
pub struct FromCommand {
    pub src: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RunCommand {
    pub cmd: RunCommandArgs,
}

#[derive(Debug)]
pub struct WorkDirCommand {
    pub path: PathBuf
}

#[derive(Debug, Eq, PartialEq)]
pub enum RunCommandArgs {
    List(Vec<String>),
    String(String),
}

#[derive(Debug)]
pub struct SaveArtifactCommand {
    pub src: String,
    pub dest: Option<String>,
}
