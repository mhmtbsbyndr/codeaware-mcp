use std::fs;
use std::path::{Path, PathBuf};

use crate::v4::errors::{V4Error, V4Result};
use crate::v4::ranking::{ContextRanker, RankedPath};

#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub max_candidates: usize,
    pub ignored_dirs: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_candidates: 64,
            ignored_dirs: vec![
                ".git".to_string(),
                ".codeaware".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                "vendor".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
        }
    }
}

pub struct CandidateDiscovery;

impl CandidateDiscovery {
    pub fn discover_ranked(repo_root: impl AsRef<Path>, goal: &str, config: DiscoveryConfig) -> V4Result<Vec<RankedPath>> {
        let mut paths = Vec::new();
        Self::walk(repo_root.as_ref(), repo_root.as_ref(), &config, &mut paths)?;
        paths.truncate(config.max_candidates);
        Ok(ContextRanker::rank_paths(goal, paths))
    }

    fn walk(root: &Path, current: &Path, config: &DiscoveryConfig, out: &mut Vec<String>) -> V4Result<()> {
        let entries = match fs::read_dir(current) {
            Ok(entries) => entries,
            Err(err) => return Err(V4Error::Io(err.to_string())),
        };

        for entry in entries.flatten() {
            let path: PathBuf = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                if config.ignored_dirs.iter().any(|ignored| ignored == &name) {
                    continue;
                }
                Self::walk(root, &path, config, out)?;
                continue;
            }

            if path.is_file() {
                if let Ok(relative) = path.strip_prefix(root) {
                    out.push(relative.to_string_lossy().to_string());
                }
            }

            if out.len() >= config.max_candidates {
                break;
            }
        }

        Ok(())
    }
}
