use std::path::Path;

#[derive(Debug, PartialEq)]
pub struct WorkspaceInfo {
    pub kind: String,
    pub packages: Vec<String>,
}

pub fn detect_workspace(root: &Path) -> Result<Option<WorkspaceInfo>, std::io::Error> {
    // Check Cargo.toml for [workspace]
    if let Ok(content) = std::fs::read_to_string(root.join("Cargo.toml")) {
        if content.contains("[workspace]") {
            let packages = find_cargo_workspace_members(root, &content);
            return Ok(Some(WorkspaceInfo {
                kind: "cargo".into(),
                packages,
            }));
        }
    }

    // Check package.json for workspaces
    if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if json.get("workspaces").is_some() {
                let packages = find_npm_workspace_packages(root, &json);
                return Ok(Some(WorkspaceInfo {
                    kind: "npm".into(),
                    packages,
                }));
            }
        }
    }

    // Check go.work
    if root.join("go.work").exists() {
        return Ok(Some(WorkspaceInfo {
            kind: "go".into(),
            packages: Vec::new(),
        }));
    }

    Ok(None)
}

fn find_cargo_workspace_members(root: &Path, content: &str) -> Vec<String> {
    // Parse the TOML so multi-line `members = [ ... ]` arrays are handled.
    // Invalid TOML yields no members rather than guessing.
    let patterns: Vec<String> = toml::from_str::<toml::Value>(content)
        .ok()
        .and_then(|v| v.get("workspace")?.get("members")?.as_array().cloned())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    // Expand glob patterns by scanning the filesystem
    let mut packages = Vec::new();
    for pattern in &patterns {
        if pattern.ends_with("/*") {
            let dir = pattern.trim_end_matches("/*");
            let full_dir = root.join(dir);
            if let Ok(entries) = std::fs::read_dir(&full_dir) {
                for entry in entries.flatten() {
                    if entry.path().join("Cargo.toml").exists() {
                        packages.push(
                            entry
                                .path()
                                .strip_prefix(root)
                                .unwrap_or(&entry.path())
                                .to_string_lossy()
                                .to_string(),
                        );
                    }
                }
            }
        } else {
            let full_path = root.join(pattern);
            if full_path.join("Cargo.toml").exists() {
                packages.push(pattern.clone());
            }
        }
    }

    packages
}

fn find_npm_workspace_packages(root: &Path, json: &serde_json::Value) -> Vec<String> {
    let mut packages = Vec::new();

    let workspaces = match json.get("workspaces").and_then(|w| w.as_array()) {
        Some(arr) => arr.clone(),
        None => return packages,
    };

    for ws in &workspaces {
        if let Some(pattern) = ws.as_str() {
            if pattern.ends_with("/*") {
                let dir = pattern.trim_end_matches("/*");
                let full_dir = root.join(dir);
                if let Ok(entries) = std::fs::read_dir(&full_dir) {
                    for entry in entries.flatten() {
                        if entry.path().join("package.json").exists() {
                            packages.push(
                                entry
                                    .path()
                                    .strip_prefix(root)
                                    .unwrap_or(&entry.path())
                                    .to_string_lossy()
                                    .to_string(),
                            );
                        }
                    }
                }
            } else {
                let full_path = root.join(pattern);
                if full_path.join("package.json").exists() {
                    packages.push(pattern.to_string());
                }
            }
        }
    }

    packages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn test_cargo_members_multiline_array() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[workspace]\nmembers = [\n    \"core\",\n    \"tools/*\",\n]\n\n[package]\nname = \"x\"\n",
        );
        write(root, "core/Cargo.toml", "[package]\nname = \"core\"\n");
        write(root, "tools/a/Cargo.toml", "[package]\nname = \"a\"\n");
        write(root, "tools/b/Cargo.toml", "[package]\nname = \"b\"\n");
        write(root, "other/Cargo.toml", "[package]\nname = \"other\"\n");

        let info = detect_workspace(root).unwrap().unwrap();
        assert_eq!(info.kind, "cargo");
        let mut packages = info.packages.clone();
        packages.sort();
        assert_eq!(
            packages,
            vec!["core".to_string(), "tools/a".to_string(), "tools/b".to_string()]
        );
    }

    #[test]
    fn test_cargo_members_single_line_array() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        write(root, "Cargo.toml", "[workspace]\nmembers = [\"core\"]\n");
        write(root, "core/Cargo.toml", "[package]\nname = \"core\"\n");
        let info = detect_workspace(root).unwrap().unwrap();
        assert_eq!(info.packages, vec!["core".to_string()]);
    }
}
