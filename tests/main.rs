use assert_cmd::assert::OutputAssertExt;
use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use rstest::rstest;
use serde::Deserialize;
use tempfile::TempDir;
use std::{collections::HashMap, io::Read, path::{Path, PathBuf}, process::Command};

#[derive(Debug, Deserialize)]
struct TestData {
    #[serde(default)]
    setup: TestSetup,
    #[serde(default)]
    files: HashMap<PathBuf, String>,
    run: Vec<TestRun>
}

#[derive(Default, Debug, Deserialize)]
struct TestSetup {
    files: Vec<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TestRun {
    args: Vec<String>,
    status_code: i32,
    stderr_contains: Vec<String>,
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
    eprintln!("{}: {:?}", path.display(), buffer);
}

#[rstest]
fn main(
    #[files("tests/data/**/*.toml")] path: PathBuf
) {
    let parent = path.parent().unwrap();
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

    for add_file in &test.setup.files {
        std::fs::copy(parent.join(add_file), temp_dir.path().join(add_file)).unwrap();
    }

    if test.run.is_empty() {
        panic!("No test runs defined");
    }

    for (idx, run) in test.run.iter().enumerate() {
        eprintln!("--- {idx}");

        let mut command = Command::cargo_bin("burt").unwrap();
        command.args(&run.args);
        command.current_dir(temp_dir.path());
        let mut cmd_assert = command.assert();
        
        eprintln!("Stdout: {}", String::from_utf8_lossy(&cmd_assert.get_output().stdout));
        eprintln!("Stderr: {}", String::from_utf8_lossy(&cmd_assert.get_output().stderr));

        cmd_assert = cmd_assert.code(predicate::eq(run.status_code));
        for s in &run.stderr_contains {
            cmd_assert = cmd_assert.stderr(predicate::str::contains(s));
        }

        check_files(&path, temp_dir.path(), run, &test);
    }
}

fn check_files(config_path: &Path, temp_dir: &Path, run: &TestRun, test: &TestData) {
    for (result_name, expect_name) in &run.verify_files {
        let result_file = temp_dir.join(result_name);
        let success = if let Some(content) = test.files.get(expect_name) {
            let predicate_file = predicate::eq(content.as_ref()).from_file_path();
            predicate_file.eval(result_file.as_path())
        } else {
            let expect_file = config_path.parent().unwrap().join(expect_name);
            let predicate_file = predicate::path::eq_file(&expect_file);
            predicate_file.eval(result_file.as_path())
        };

        if !success {
            eprintln!("File contents {:?} and {:?} do not match!", expect_name, &result_file);
            show_file(&result_file);
            panic!("Check failed!")
        }
    }
}
