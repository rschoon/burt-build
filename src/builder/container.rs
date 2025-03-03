use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;


pub struct Container {
    container: String,
    // ui: Ui
}

impl Container {
    pub fn create(from: &str) -> anyhow::Result<Self> {
        let out = Command::new("buildah").arg("from").arg(from).output()?;
        let container = String::from_utf8(out.stdout.trim_ascii_end().to_vec())?;
        Ok(Self {
            container
        })
    }

    pub fn run(&self) -> CommandRun {
        let mut command = Command::new("buildah");
        command.arg("run").arg("--").arg(&self.container);
        CommandRun {
            command
        }
    }

    pub fn copy_to_container(&self, src: &str, container: &Container, dest: &str) -> anyhow::Result<()> {
        let burt = crate::current_exe();
        Command::new("buildah")
            .arg("unshare")
            .arg("-m").arg(format!("PREFIX_SRC={}", &self.container))
            .arg("-m").arg(format!("PREFIX_DEST={}", &container.container))
            .arg("--")
            .arg(burt)
            .arg("internal-container-copy")
            .arg(src)
            .arg(dest).status()?;
        Ok(())
    }

    pub fn set_work_dir(&self, path: &Path) -> anyhow::Result<()> {
        Command::new("buildah")
            .arg("config")
            .arg("--workingdir")
            .arg(path)
            .arg(&self.container).status()?;
        Ok(())
    }
}

pub struct CommandRun {
    command: Command
}

impl CommandRun {
    pub fn arg<S>(mut self, arg: S) -> Self
    where
        S: std::convert::AsRef<std::ffi::OsStr>,
    {
        self.command.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item=S>,
        S: std::convert::AsRef<std::ffi::OsStr>,
    {
        self.command.args(args);
        self
    }

    pub fn status(mut self) -> anyhow::Result<std::process::ExitStatus> {
        Ok(self.command.status()?)
    }
}

fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else if ty.is_file() {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else if ty.is_symlink() {
            std::os::unix::fs::symlink(dst.as_ref().join(entry.file_name()), entry.path())?;
        }
        // TODO: chown, chdir
    }
    Ok(())
}

pub(crate) fn perform_container_copy(src: &Path, dest: &Path) -> Result<(), anyhow::Error> {
    fn make_path(name: &str, suffix: &Path) -> PathBuf {
        let mut prefix = PathBuf::from(std::env::var_os(name).unwrap_or_else(|| "/".into()));
        let suffix = suffix.strip_prefix("/").unwrap_or(suffix);
        prefix.push(suffix);
        prefix
    }

    let src = make_path("PREFIX_SRC", src);
    let mut dest = make_path("PREFIX_DEST", dest);

    if dest.as_os_str().as_encoded_bytes().ends_with(b"/") {
        if let Some(filename) = src.file_name() {
            dest.push(filename);
        }
    }

    if src.is_dir() {
        fs::remove_dir_all(&dest)?;
        copy_dir(&src, &dest)?;
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dest)?;
    }

    Ok(())
}

