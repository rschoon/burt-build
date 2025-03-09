
use std::collections::HashMap;

#[derive(Debug)]
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

    pub fn is_set(&self, name: &str) -> bool {
        self.vars.contains_key(name)
    }

    pub fn set<V>(&mut self, name: String, value: V)
    where 
        V: Into<minijinja::Value>
    {
        self.vars.insert(name, value.into());
    }

    pub fn render<S: AsRef<str>>(&self, s: S) -> anyhow::Result<String> {
        Ok(self.environment.render_str(s.as_ref(), &self.vars)?)
    }
}
