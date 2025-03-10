
use base64::prelude::*;
use sha2::Digest;
use std::{borrow::Cow, ffi::OsStr, path::Path};
use std::io::{self, Seek};

use crate::file::{Command, RootSection};

mod artifact;
mod container;
mod template;
mod hashfile;

use container::Container;

pub(crate) use container::{
    perform_container_copy,
    perform_container_export,
    perform_container_import_tar
};

macro_rules! ensure_container {
    ($b:expr) => {
        if let Some(c) = $b.container.as_ref() {
            c
        } else if let Some(s) = $b.container_src.as_ref() {
            $b.container = Some(Container::create(&s.from)?);
            $b.container.as_ref().unwrap()
        } else {
            return Err(anyhow::anyhow!("No container"));
        }
    }
}

pub struct Build {
    container_src: Option<ContainerSrc>,
    container: Option<container::Container>,
    artifact_output: artifact::ArtifactStore,
    environment: template::Environment
}

impl Build {
    pub fn new() -> Self {
        Self {
            container_src: None,
            container: None,
            artifact_output: artifact::ArtifactStore::default(),
            environment: template::Environment::new()
        }
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.environment.set(name.to_owned(), value);
    }

    pub fn export_artifact<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.artifact_output.export(path.as_ref())
    }

    pub fn build(&mut self, root_config: &RootSection, target: &str) -> anyhow::Result<()> {
        let target_def = root_config.targets.get(target).ok_or_else(|| anyhow::anyhow!("No such target"))?;

        for command in &target_def.commands {
            self.build_command(command)?;
        }

        Ok(())
    }

    fn track_changes<F, T, K>(&mut self, key: K, func: F) -> anyhow::Result<T>
    where 
        F: FnOnce(&Container) -> anyhow::Result<T>,
        K: ToString
    {    
        let Some(parent) = self.container_src.as_ref() else {
            return Err(anyhow::anyhow!("No container from"));
        };
    
        let mut combine_key = sha2::Sha256::new();
        combine_key.update(parent.key.as_bytes());
        combine_key.update(b"\0");
        combine_key.update(key.to_string().as_bytes());
        let combine_key = format!("burt-{}", BASE64_STANDARD.encode(combine_key.finalize()));

        let src = container::get_cached_image(&combine_key);
        let src = src.as_deref().unwrap_or(&parent.from);
        let container = Container::create(src)?;
        let rv = func(&container);

        if rv.is_ok() {
            self.container_src = Some(container.commit(combine_key)?);
        }
        self.container = Some(container);
        
        rv
    }

    fn build_command(&mut self, cmd: &Command) -> anyhow::Result<()> {
        match cmd {
            Command::From(f) => self.cmd_from(f),
            Command::Run(r) => self.cmd_run(r),
            Command::WorkDir(w) => self.cmd_work_dir(w),
            Command::SaveArtifact(c) => self.cmd_save(c),
            Command::Set(s) => self.cmd_set(s),
            Command::Copy(c) => self.cmd_copy(c),
        }?;

        Ok(())
    }

    fn cmd_from(&mut self, f: &crate::file::FromCommand) -> anyhow::Result<()> {
        let src = self.environment.render(&f.src)?;
        self.container_src = Some(ContainerSrc::from(src)?);
        Ok(())
    }

    fn cmd_run(&mut self, r: &crate::file::RunCommand) -> anyhow::Result<()> {
        let cmd_args: Vec<Cow<'_, str>> = match &r.cmd {
            crate::file::RunCommandArgs::List(args) => {
                args.iter().map(|a| self.environment.render(a).map(Cow::Owned)).collect::<Result<Vec<_>, _>>()?
            },
            crate::file::RunCommandArgs::String(script) => {
                let script = self.environment.render(script)?;
                vec!["/bin/sh".into(), "-c".into(), script.into()]
            }
        };

        let key = cmd_args.join("\0");
        self.track_changes(
            format!("cmd:{key}"),
            move |c| {
                let cmd = c.run()
                    .args(cmd_args.iter().map(|a| OsStr::new(a.as_ref())));
        
                let result = cmd.status()?;
                if !result.success() {
                    Err(anyhow::anyhow!("{}", result.code().unwrap_or(-1)))
                } else {
                    Ok(())
                }
            }
        )
    }

    fn cmd_work_dir(&mut self, r: &crate::file::WorkDirCommand) -> anyhow::Result<()> {
        let path = self.environment.render(&r.path)?;
        self.track_changes(format!("workdir:{}", &path), move |c| {
            c.set_work_dir(Path::new(&path))
        })
    }

    fn cmd_save(&mut self, r: &crate::file::SaveArtifactCommand) -> anyhow::Result<()> {
        let container = ensure_container!(self);
        let src = self.environment.render(&r.src)?;
        let dest = r.dest.as_deref().map(|p| self.environment.render(p)).transpose()?;
        self.artifact_output.save(container, &src, dest.as_deref().unwrap_or("/"))?;
        Ok(())
    }
    
    fn cmd_set(&mut self, s: &crate::file::SetCommand) -> Result<(), anyhow::Error> {
        if s.default && self.environment.is_set(&s.name) {
            return Ok(())
        }
        
        if let Some(v) = s.value.as_deref() {
            let value = self.environment.render(&v)?;
            self.environment.set(s.name.clone(), value);
        } else {
            self.environment.set(s.name.clone(), minijinja::Value::default());
        }

        Ok(())
    }

    fn cmd_copy(&mut self, c: &crate::file::CopyCommand) -> anyhow::Result<()> {
        let mut tarfile = tar::Builder::new(hashfile::HashedFile::new(tempfile::tempfile()?));

        let dest = self.environment.render(&c.dest)?;
        for inp in &c.src {
            let inp = self.environment.render(inp)?;
            tarfile.append_path(inp)?;
        }

        tarfile.finish()?;
        let (mut tarfile, hash) = tarfile.into_inner()?.finish()?;

        tarfile.seek(io::SeekFrom::Start(0))?;
        self.track_changes(
            format!("copy-tar:{}:{}", BASE64_STANDARD.encode(hash), dest),
            move |c| {
                c.import_tar(tarfile, &dest)
            }
        )
    }
}

struct ContainerSrc {
    from: String,
    key: String
}

impl ContainerSrc {
    fn from(name: String) -> anyhow::Result<Self> {
        let key = container::fetch_image(&name)?;
        Ok(Self {
            from: name,
            key: format!("from-{key}")
        })
    }
}


