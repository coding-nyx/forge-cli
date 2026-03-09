use crate::providers::base::ForgeProvider;
use std::collections::HashMap;

pub struct ProviderFactory {
    providers: HashMap<String, Box<dyn ForgeProvider>>,
}

impl ProviderFactory {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, provider: Box<dyn ForgeProvider>) {
        self.providers.insert(name.to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<&dyn ForgeProvider> {
        self.providers.get(name).map(|b| b.as_ref())
    }
}
