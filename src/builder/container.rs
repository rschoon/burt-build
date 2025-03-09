use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};


pub struct Container {
    container: String,
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

    pub fn export(&self, src: &Path, dest: &Path) -> anyhow::Result<()> {
        let burt = crate::current_exe();
        let mut child = Command::new("buildah")
            .arg("unshare")
            .arg("-m").arg(format!("PREFIX={}", &self.container))
            .arg("--")
            .arg(burt)
            .arg("internal-export")
            .arg(src)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()?;
        
        let stdout = child.stdout.take().unwrap();
        let mut archive = tar::Archive::new(stdout);
        archive.unpack(dest)?;

        child.wait()?;

        Ok(())
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        let _ = delete_container(&self.container);
    }
}

fn delete_container(name: &str) -> anyhow::Result<()> {
    let status = Command::new("buildah")
        .arg("rm")
        .arg(name)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Delete container {} failed", name)
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

fn internal_container_path(name: &str, suffix: &Path) -> PathBuf {
    let mut prefix = PathBuf::from(std::env::var_os(name).unwrap_or_else(|| "/".into()));
    let suffix = suffix.strip_prefix("/").unwrap_or(suffix);
    prefix.push(suffix);
    prefix
}

pub(crate) fn perform_container_copy(src: &Path, dest: &Path) -> Result<(), anyhow::Error> {
    let src = internal_container_path("PREFIX_SRC", src);
    let mut dest = internal_container_path("PREFIX_DEST", dest);

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

pub(crate) fn perform_container_export(path: &Path) -> Result<(), anyhow::Error> {
    let prefix = internal_container_path("PREFIX", path);
    let mut tarb = tar::Builder::new(std::io::stdout());
    tarb.append_dir_all("", prefix)?;
    tarb.finish()?;
    Ok(())
}

