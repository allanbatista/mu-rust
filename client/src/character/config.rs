use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct PlayerActionsConfig {
    pub actions: Vec<ActionDef>,
    pub class_defaults: HashMap<String, ClassDefaults>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ActionDef {
    pub index: usize,
    pub name: String,
    pub category: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClassDefaults {
    pub idle: usize,
    pub walk: usize,
    pub run: usize,
}

impl PlayerActionsConfig {
    pub fn action_name(&self, index: usize) -> Option<&str> {
        self.actions
            .iter()
            .find(|a| a.index == index)
            .map(|a| a.name.as_str())
    }

    pub fn class_idle(&self, class_name: &str) -> usize {
        self.class_defaults
            .get(class_name)
            .map(|d| d.idle)
            .unwrap_or(1)
    }

    pub fn class_walk(&self, class_name: &str) -> usize {
        self.class_defaults
            .get(class_name)
            .map(|d| d.walk)
            .unwrap_or(15)
    }

    pub fn class_run(&self, class_name: &str) -> usize {
        self.class_defaults
            .get(class_name)
            .map(|d| d.run)
            .unwrap_or(25)
    }
}
