
use std::path::Path;

use container::Container;

use crate::file::{Command, RootSection};

mod artifact;
mod container;
mod template;

pub(crate) use container::{perform_container_copy, perform_container_export};

macro_rules! ensure_container {
    ($b:expr) => {
        if let Some(c) = $b.container.as_ref() {
            c
        } else {
            return Err(anyhow::anyhow!("No container"));
        }
    }
}

pub struct Build {
    container: Option<container::Container>,
    artifact_output: artifact::ArtifactStore,
    environment: template::Environment
}

impl Build {
    pub fn new() -> Self {
        Self {
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

    fn build_command(&mut self, cmd: &Command) -> anyhow::Result<()> {
        match cmd {
            Command::From(f) => self.cmd_from(f),
            Command::Run(r) => self.cmd_run(r),
            Command::WorkDir(w) => self.cmd_work_dir(w),
            Command::SaveArtifact(c) => self.cmd_save(c),
            Command::Set(s) => self.cmd_set(s),
        }?;

        Ok(())
    }

    fn cmd_from(&mut self, f: &crate::file::FromCommand) -> anyhow::Result<()> {
        let src = self.environment.render(&f.src)?;
        self.container = Some(Container::create(&src)?);
        Ok(())
    }

    fn cmd_run(&mut self, r: &crate::file::RunCommand) -> anyhow::Result<()> {
        let container = ensure_container!(self);

        let mut cmd = container.run();
        cmd = match &r.cmd {
            crate::file::RunCommandArgs::List(args) => {
                let args = args.iter().map(|a| self.environment.render(a)).collect::<Result<Vec<_>, _>>()?;
                cmd.args(args)
            },
            crate::file::RunCommandArgs::String(script) => {
                let script = self.environment.render(script)?;
                cmd.arg("/bin/sh").arg("-c").arg(script)
            }
        };

        let result = cmd.status()?;
        if !result.success() {
            Err(anyhow::anyhow!("{}", result.code().unwrap_or(-1)))
        } else {
            Ok(())
        }
    }

    fn cmd_work_dir(&mut self, r: &crate::file::WorkDirCommand) -> anyhow::Result<()> {
        let container = ensure_container!(self);
        let path = self.environment.render(&r.path)?;
        container.set_work_dir(Path::new(&path))?;
        Ok(())
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
}
