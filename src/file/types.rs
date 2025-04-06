
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
    Set(SetCommand),
    Copy(CopyCommand),
    ReadFile(ReadFileCommand),
    ReadRun(ReadRunCommand),
}

#[derive(Debug)]
pub struct FromCommand {
    pub src: FromImage,
}

#[derive(Debug)]
pub enum FromImage {
    Image(String),
    Target(TargetRef),
}

#[derive(Debug)]
pub struct TargetRef {
    pub path: Option<PathBuf>,
    pub target: String,
    pub artifact: Option<String>,
    // pub args: HashMap<String, String>
}

#[derive(Debug)]
pub struct SetCommand {
    pub name: String,
    pub value: Option<String>,
    pub default: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RunCommand {
    pub cmd: RunCommandArgs,
}

#[derive(Debug)]
pub struct WorkDirCommand {
    pub path: String
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

#[derive(Debug)]
pub enum CopySource {
    LocalPath(String),
    Artifact(TargetRef),
}

#[derive(Debug)]
pub struct CopyCommand {
    pub src: Vec<CopySource>,
    pub dest: String,
}

#[derive(Debug)]
pub struct ReadFileCommand {
    pub src: String,
    pub dest: String,
}

#[derive(Debug)]
pub struct ReadRunCommand {
    pub src: RunCommandArgs,
    pub dest: String,
}
