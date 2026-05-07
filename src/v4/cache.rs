use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    pub path: String,
    pub hash: String,
    pub summary: String,
    pub estimated_tokens: usize,
}

#[derive(Debug, Default)]
pub struct ReadOnceCache {
    read_paths: HashSet<String>,
}

impl ReadOnceCache {
    pub fn mark_read(&mut self, path: impl Into<String>) {
        self.read_paths.insert(path.into());
    }

    pub fn has_read(&self, path: &str) -> bool {
        self.read_paths.contains(path)
    }

    pub fn len(&self) -> usize {
        self.read_paths.len()
    }
}
