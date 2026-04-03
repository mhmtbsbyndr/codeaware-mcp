use crate::intelligence::tree_sitter_provider::TreeSitterProvider;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefetchedFile {
    pub path: String,
    pub mode: String,
    pub symbols: Vec<String>,
    pub reason: String,
}

pub fn prefetch_for_file(
    file_path: &str,
    session_files: &std::collections::HashSet<String>,
    project_root: &Path,
) -> Vec<PrefetchedFile> {
    let mut results = Vec::new();
    let max_prefetch = 3;

    // Derive base name: "src/auth.rs" -> "auth"
    let base_name = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    if base_name.is_empty() {
        return results;
    }

    // Strategy 1: Find matching test file
    let test_patterns = vec![
        format!("tests/test_{}.rs", base_name),
        format!("tests/{}_test.rs", base_name),
        format!("tests/test_{}.py", base_name),
    ];

    for pattern in &test_patterns {
        if results.len() >= max_prefetch {
            break;
        }
        let test_path = project_root.join(pattern);
        if test_path.exists() && !session_files.contains(pattern.as_str()) {
            if let Some(pf) = try_prefetch(
                &test_path,
                pattern,
                &format!("test file for {}", base_name),
            ) {
                results.push(pf);
            }
        }
    }

    // Strategy 2: Co-access from session -- files in the same parent directory
    // Suggest mod.rs or lib.rs from same dir as likely related
    let parent = Path::new(file_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    if !parent.is_empty() {
        let parent_abs = project_root.join(parent);
        if parent_abs.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&parent_abs) {
                for entry in entries.flatten() {
                    if results.len() >= max_prefetch {
                        break;
                    }
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if !["rs", "py", "ts", "js", "go"].contains(&ext) {
                        continue;
                    }

                    let rel = path
                        .strip_prefix(project_root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();

                    // Skip the file itself and already-read files
                    if rel == file_path || session_files.contains(rel.as_str()) {
                        continue;
                    }

                    // Check if this sibling was co-accessed in previous session reads
                    // For now, just suggest mod.rs or lib.rs from same dir as likely related
                    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if fname == "mod.rs" || fname == "lib.rs" {
                        if let Some(pf) = try_prefetch(
                            &path,
                            &rel,
                            &format!("sibling module in {}/", parent),
                        ) {
                            results.push(pf);
                        }
                    }
                }
            }
        }
    }

    results
}

fn try_prefetch(abs_path: &Path, rel_path: &str, reason: &str) -> Option<PrefetchedFile> {
    let content = std::fs::read_to_string(abs_path).ok()?;
    let lang = match abs_path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("go") => "go",
        _ => return None,
    };

    let provider = TreeSitterProvider::new();
    let symbols = provider
        .extract_symbols(&content, lang)
        .unwrap_or_default()
        .iter()
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();

    Some(PrefetchedFile {
        path: rel_path.to_string(),
        mode: "skeleton".to_string(),
        symbols,
        reason: reason.to_string(),
    })
}
