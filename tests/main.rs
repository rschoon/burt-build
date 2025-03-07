use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use rstest::rstest;
use serde::Deserialize;
use std::{io::{Read, Write}, path::PathBuf, process::Command};

#[derive(Debug, Deserialize)]
struct TestData {
    run: Vec<TestRun>
}

#[derive(Debug, Deserialize)]
struct TestRun {
    args: Vec<String>,
    #[serde(default)]
    status_code: i32
}

#[rstest]
fn main(
    #[files("tests/data/**/*.txt")] path: PathBuf
) {
    use std::io::Seek;
    use tempfile::NamedTempFile;

    let mut f = std::fs::File::open(path).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();

    let pos = buffer.find("\n###\n").unwrap();
    let toml_data = &buffer[..pos];
    let burt_data = &buffer[pos+5..];

    let test: TestData = toml::from_str(toml_data).unwrap();

    let mut burt_file = NamedTempFile::new().unwrap();
    burt_file.write(burt_data.as_bytes()).unwrap();
    burt_file.seek(std::io::SeekFrom::Start(0)).unwrap();

    if test.run.is_empty() {
        panic!("No test runs defined");
    }

    for run in &test.run {
        let mut command = Command::cargo_bin("burt").unwrap();
        command.arg("-f").arg(burt_file.path()).args(&run.args);
        let cmd_assert = command.assert();
        cmd_assert.code(predicate::eq(run.status_code));
    }
}
