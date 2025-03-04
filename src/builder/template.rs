use std::collections::HashMap;

pub struct Environment {
    environment: minijinja::Environment<'static>,
    vars: HashMap<String, minijinja::Value>
}

impl Environment {
    pub fn new() -> Self {
        Self {
            environment: minijinja::Environment::empty(),
            vars: HashMap::new(),
        }
    }

    pub fn render<S: AsRef<str>>(&self, s: S) -> anyhow::Result<String> {
        Ok(self.environment.render_str(s.as_ref(), &self.vars)?)
    }
}
