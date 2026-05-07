use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::v4::errors::{V4Error, V4Result};

#[derive(Debug, Clone)]
pub struct V4Storage {
    root: PathBuf,
}

impl V4Storage {
    pub fn new(repo_root: impl AsRef<Path>) -> Self {
        Self {
            root: repo_root.as_ref().join(".codeaware").join("v4"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure_layout(&self) -> V4Result<()> {
        for dir in ["cache", "traces", "contracts", "decisions"] {
            fs::create_dir_all(self.root.join(dir)).map_err(|err| V4Error::Io(err.to_string()))?;
        }
        Ok(())
    }

    pub fn append_jsonl(&self, relative_path: &str, line: &str) -> V4Result<()> {
        self.ensure_layout()?;
        let path = self.root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| V4Error::Io(err.to_string()))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| V4Error::Io(err.to_string()))?;

        writeln!(file, "{}", line).map_err(|err| V4Error::Io(err.to_string()))?;
        Ok(())
    }
}
