use std::collections::HashMap;
use std::path::Path;

pub struct SeenFiles {
    files: HashMap<String, SeenFileEntry>,
}

struct SeenFileEntry {
    hash: String,
    step: u32,
    pre_compact: bool,
}

impl Default for SeenFiles {
    fn default() -> Self {
        Self::new()
    }
}

impl SeenFiles {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    pub fn mark_seen(&mut self, path: &str, hash: &str, step: u32) {
        self.files.insert(path.to_string(), SeenFileEntry {
            hash: hash.to_string(),
            step,
            pre_compact: false,
        });
    }

    pub fn is_seen(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    pub fn is_stale(&self, path: &str, current_hash: &str) -> bool {
        self.files.get(path)
            .map(|e| e.hash != current_hash)
            .unwrap_or(true)
    }

    pub fn update_hash(&mut self, path: &str, new_hash: &str, step: u32) {
        if let Some(entry) = self.files.get_mut(path) {
            entry.hash = new_hash.to_string();
            entry.step = step;
        }
    }

    pub fn last_seen_step(&self, path: &str) -> Option<u32> {
        self.files.get(path).map(|e| e.step)
    }

    pub fn invalidate(&mut self, path: &str) {
        self.files.remove(path);
    }

    pub fn invalidate_all(&mut self) {
        self.files.clear();
    }

    pub fn mark_all_pre_compact(&mut self) {
        for entry in self.files.values_mut() {
            entry.pre_compact = true;
        }
    }

    pub fn is_pre_compact(&self, path: &str) -> bool {
        self.files.get(path).map(|e| e.pre_compact).unwrap_or(false)
    }

    pub fn all_seen(&self) -> Vec<(&str, &str, u32, bool)> {
        self.files.iter()
            .map(|(path, e)| (path.as_str(), e.hash.as_str(), e.step, e.pre_compact))
            .collect()
    }

    pub fn hash_file(path: &Path) -> Result<String, std::io::Error> {
        let content = std::fs::read(path)?;
        Ok(blake3::hash(&content).to_hex().to_string())
    }
}
