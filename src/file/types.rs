
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
}

#[derive(Debug)]
pub struct FromCommand {
    pub src: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RunCommand {
    pub cmd: RunCommandArgs,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RunCommandArgs {
    List(Vec<String>),
    String(String),
}
