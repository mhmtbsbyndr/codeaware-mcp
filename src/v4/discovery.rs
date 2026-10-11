use std::path::Path;

use ignore::WalkBuilder;

use crate::v4::errors::{V4Error, V4Result};
use crate::v4::language_support::LanguageDetector;
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
    /// Ranks source files under `repo_root` against `goal`.
    ///
    /// The walk does not follow symlinks, honours `.gitignore` (even outside a git
    /// checkout), skips the configured directories and unreadable entries, and
    /// only considers files with a supported source extension. The candidate cap
    /// is applied after filtering and ranking, so the result is deterministic.
    pub fn discover_ranked(repo_root: impl AsRef<Path>, goal: &str, config: DiscoveryConfig) -> V4Result<Vec<RankedPath>> {
        let root = repo_root.as_ref();
        if !root.is_dir() {
            return Err(V4Error::Io(format!("not a directory: {}", root.display())));
        }

        let ignored_dirs = config.ignored_dirs.clone();
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .parents(false)
            .require_git(false)
            .git_global(false)
            .git_exclude(false)
            .follow_links(false)
            .sort_by_file_path(|a, b| a.cmp(b))
            .filter_entry(move |entry| {
                let is_dir = entry.file_type().is_some_and(|ft| ft.is_dir());
                entry.depth() == 0 || !(is_dir && ignored_dirs.iter().any(|ignored| entry.file_name() == ignored.as_str()))
            })
            .build();

        let mut paths = Vec::new();
        for result in walker {
            // Unreadable or vanished entries are skipped rather than failing the whole walk.
            let Ok(entry) = result else {
                continue;
            };
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }
            let Ok(relative) = entry.path().strip_prefix(root) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if LanguageDetector::is_supported(&relative) {
                paths.push(relative);
            }
        }

        let mut ranked = ContextRanker::rank_paths(goal, paths);
        ranked.truncate(config.max_candidates);
        Ok(ranked)
    }
}
