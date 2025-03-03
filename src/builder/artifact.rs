
use super::container::Container;

#[derive(Default)]
pub struct ArtifactStore {
    container: Option<Container>
}

impl ArtifactStore {
    pub fn save(&mut self, container: &Container, src: &str, dest: &str) -> anyhow::Result<()> {
        let dest_container = self.ensure_container()?;
        container.copy_to_container(src, dest_container, dest)
    }

    fn ensure_container(&mut self) -> anyhow::Result<&Container> {
        if self.container.is_none() {
            self.container = Some(Container::create("scratch")?);
        }
        Ok(self.container.as_ref().unwrap())
    }
}
