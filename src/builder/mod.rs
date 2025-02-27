use crate::file::{Command, RootSection};

pub struct Build {
    container: Option<String>,
}


impl Build {
    pub fn new() -> Self {
        Self {
            container: None,
        }
    }

    pub fn build(&mut self, root_config: &RootSection, target: &str) -> anyhow::Result<()> {
        let target_def = root_config.targets.get(target).ok_or_else(|| anyhow::anyhow!("No such target"))?;

        for command in &target_def.commands {
            self.build_command(command)?;
        }

        println!("Result: {}", self.container.as_deref().unwrap_or("--"));

        Ok(())
    }

    fn build_command(&mut self, cmd: &Command) -> anyhow::Result<()> {
        match cmd {
            Command::From(f) => self.cmd_from(f),
            Command::Run(r) => self.cmd_run(r)
        }?;

        Ok(())
    }

    fn cmd_from(&mut self, f: &crate::file::FromCommand) -> anyhow::Result<()> {
        let out = std::process::Command::new("buildah").arg("from").arg(&f.src).output()?;
        self.container = Some(String::from_utf8(out.stdout.trim_ascii_end().to_vec())?);
        Ok(())
    }

    fn cmd_run(&mut self, r: &crate::file::RunCommand) -> anyhow::Result<()> {
        let Some(container) = self.container.as_deref() else { return Err(anyhow::anyhow!("No container set yet")) };

        match &r.cmd {
            crate::file::RunCommandArgs::List(items) => { std::process::Command::new("buildah").arg("run").arg(container).args(items).status()?; }
            crate::file::RunCommandArgs::String(script) => { std::process::Command::new("buildah").arg("run").arg("--").arg(container).arg("/bin/sh").arg("-c").arg(script).status()?; }
        }

        Ok(())
    }
}
