use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during path resolution.
#[derive(Debug, Error)]
pub enum PathError {
    #[error("path traversal blocked: {0}")]
    TraversalBlocked(String),

    #[error("symlink escapes project root: {0}")]
    SymlinkEscape(String),

    #[error("access to hidden directory blocked: {0}")]
    HiddenDirBlocked(String),

    #[error("symlink loop detected after {0} hops")]
    SymlinkLoop(usize),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Allowed exceptions inside hidden VCS directories (relative paths).
const ALLOWED_HIDDEN: &[&str] = &[".git/HEAD"];

/// Maximum symlink resolution depth before declaring a loop.
const MAX_SYMLINK_DEPTH: usize = 10;

/// Resolve `path` relative to (or inside) `project_root`, enforcing:
/// - No path traversal outside `project_root`
/// - No access to hidden VCS dirs (`.git/`, `.svn/`, `.hg/`) except `.git/HEAD`
/// - Symlinks that resolve outside `project_root` are blocked
/// - Symlink loops beyond depth 10 are blocked
pub fn resolve_path(path: &str, project_root: &Path) -> Result<PathBuf, PathError> {
    // Canonicalize the project root so comparisons are stable.
    let canonical_root = project_root
        .canonicalize()
        .map_err(PathError::Io)?;

    // Build the candidate path: if the input is absolute use it directly,
    // otherwise join with project_root.
    let raw = Path::new(path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        project_root.join(raw)
    };

    // Check hidden-dir rules BEFORE canonicalizing (so we catch
    // paths that don't exist yet, e.g. symlinks to outside).
    check_hidden_dir(path, project_root, &canonical_root)?;

    // Resolve symlinks manually to detect escapes.
    let resolved = resolve_symlinks(&candidate, &canonical_root, 0)?;

    // Final containment check.
    if !resolved.starts_with(&canonical_root) {
        return Err(PathError::TraversalBlocked(format!(
            "{} is outside project root {}",
            resolved.display(),
            canonical_root.display()
        )));
    }

    // Re-express the resolved path relative to the *original* (possibly
    // non-canonical) project_root, so callers using TempDir on macOS
    // (where /var -> /private/var) get a path that starts_with(project_root).
    let rel = resolved.strip_prefix(&canonical_root).unwrap_or(&resolved);
    let result = if rel == Path::new("") {
        project_root.to_path_buf()
    } else {
        project_root.join(rel)
    };
    Ok(result)
}

/// Walk symlinks up to `MAX_SYMLINK_DEPTH`, returning the real path.
/// Returns `SymlinkEscape` if the resolved target exits `canonical_root`,
/// `SymlinkLoop` if depth exceeds the limit.
fn resolve_symlinks(
    path: &Path,
    canonical_root: &Path,
    depth: usize,
) -> Result<PathBuf, PathError> {
    if depth > MAX_SYMLINK_DEPTH {
        return Err(PathError::SymlinkLoop(depth));
    }

    match std::fs::symlink_metadata(path) {
        Err(_) => {
            // Path does not exist; normalize lexically without resolving symlinks.
            let normalized = normalize_path(path);
            Ok(normalized)
        }
        Ok(meta) if !meta.file_type().is_symlink() => {
            // Regular file or directory — canonicalize normally.
            let canon = path.canonicalize().map_err(PathError::Io)?;
            Ok(canon)
        }
        Ok(_) => {
            // It is a symlink — read its target.
            let target = std::fs::read_link(path).map_err(PathError::Io)?;
            let resolved_target = if target.is_absolute() {
                target
            } else {
                // Relative symlink: resolve relative to the symlink's parent dir.
                let parent = path.parent().unwrap_or(Path::new("."));
                parent.join(&target)
            };

            // Recurse to resolve the target (handles chained symlinks).
            // We intentionally do NOT pass through resolve_symlinks' escape check
            // here — we check after full resolution instead.
            let final_path = match resolve_symlinks(&resolved_target, canonical_root, depth + 1) {
                Ok(p) => p,
                Err(PathError::SymlinkEscape(_)) => {
                    // Re-wrap with original symlink path for better diagnostics.
                    return Err(PathError::SymlinkEscape(format!(
                        "symlink {} resolves outside project root {}",
                        path.display(),
                        canonical_root.display()
                    )));
                }
                Err(e) => return Err(e),
            };

            // After full resolution, check containment.
            if !final_path.starts_with(canonical_root) {
                return Err(PathError::SymlinkEscape(format!(
                    "symlink {} resolves to {} which is outside project root {}",
                    path.display(),
                    final_path.display(),
                    canonical_root.display()
                )));
            }

            Ok(final_path)
        }
    }
}

/// Normalize a path without requiring it to exist (no canonicalize).
/// Resolves `.` and `..` components lexically.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components: Vec<std::path::Component> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                components.pop();
            }
            c => components.push(c),
        }
    }
    components.iter().collect()
}

/// Check whether `path_str` refers to a blocked hidden VCS directory.
/// Allowed exceptions: `.git/HEAD` (exact).
fn check_hidden_dir(
    path_str: &str,
    project_root: &Path,
    canonical_root: &Path,
) -> Result<(), PathError> {
    let raw = Path::new(path_str);
    let full = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        project_root.join(raw)
    };

    let display = full.to_string_lossy();
    let root_display = canonical_root.to_string_lossy();

    // Get a relative path from root for component analysis.
    let rel = if display.starts_with(root_display.as_ref()) {
        display[root_display.len()..].trim_start_matches('/').to_string()
    } else {
        // Absolute path outside root; hidden-dir check on raw input.
        path_str.trim_start_matches('/').to_string()
    };

    let hidden_dirs = [".git", ".svn", ".hg"];
    let rel_path = PathBuf::from(&rel);
    let mut seen: Vec<String> = Vec::new();

    for comp in rel_path.components() {
        if let std::path::Component::Normal(s) = comp {
            let s_str = s.to_string_lossy().into_owned();
            if hidden_dirs.iter().any(|&h| s_str == h) {
                seen.push(s_str);
                // Collect remaining components after the hidden dir.
                let full_rel = rel_path
                    .components()
                    .skip(seen.len() - 1)
                    .filter_map(|c| {
                        if let std::path::Component::Normal(n) = c {
                            Some(n.to_string_lossy().into_owned())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("/");

                // Check against allowed exceptions.
                let is_allowed = ALLOWED_HIDDEN.iter().any(|&allowed| full_rel == allowed);

                if !is_allowed {
                    return Err(PathError::HiddenDirBlocked(format!(
                        "access to hidden directory '{}' is blocked",
                        full_rel
                    )));
                }
                return Ok(());
            } else {
                seen.push(s_str);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn normalize_removes_dotdot() {
        let p = normalize_path(Path::new("/a/b/../c"));
        assert_eq!(p, PathBuf::from("/a/c"));
    }

    #[test]
    fn normalize_removes_dot() {
        let p = normalize_path(Path::new("/a/./b"));
        assert_eq!(p, PathBuf::from("/a/b"));
    }
}
