use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use rstest::rstest;
use serde::Deserialize;
use tempfile::TempDir;
use std::{collections::HashMap, io::Read, path::{Path, PathBuf}, process::Command};

#[derive(Debug, Deserialize)]
struct TestData {
    run: Vec<TestRun>
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TestRun {
    args: Vec<String>,
    status_code: i32,
    verify_files: HashMap<PathBuf, PathBuf>,
}

fn show_file(path: &Path) {
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Can't show {:?}: {}", path, e);
            return;
        }
    };
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();
    eprintln!("{:?}", buffer);
}

#[rstest]
fn main(
    #[files("tests/data/**/*.toml")] path: PathBuf
) {
    let burt_name = {
        let mut b = path.clone();
        b.set_extension("burt");
        b
    };
    let mut f = std::fs::File::open(&path).unwrap();
    let mut toml_data = String::new();
    f.read_to_string(&mut toml_data).unwrap();
    let test: TestData = toml::from_str(&toml_data).unwrap();

    let temp_dir = TempDir::new().unwrap();
    
    if burt_name.exists() {
        let burt_filename = temp_dir.path().join("build.burt");
        std::fs::copy(burt_name, burt_filename).unwrap();
    }

    if test.run.is_empty() {
        panic!("No test runs defined");
    }

    for run in &test.run {
        let mut command = Command::cargo_bin("burt").unwrap();
        command.args(&run.args);
        command.current_dir(temp_dir.path());
        let cmd_assert = command.assert();
        cmd_assert.code(predicate::eq(run.status_code));

        for (result_name, expect_name) in &run.verify_files {
            let expect_file = path.parent().unwrap().join(expect_name);
            let result_file = temp_dir.path().join(result_name);
            let predicate_file = predicate::path::eq_file(&expect_file);
            if !predicate_file.eval(result_file.as_path()) {
                eprintln!("File contents {:?} and {:?} do not match!", &expect_file, &result_file);
                show_file(&result_file);
                panic!("Check failed!")
            }
        }
    }
}
