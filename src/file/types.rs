
use std::collections::HashMap;

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
}

#[derive(Debug)]
pub struct FromCommand {
    pub src: String,
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
