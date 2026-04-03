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
    // Extract members patterns from TOML content (simple heuristic)
    let mut patterns: Vec<String> = Vec::new();

    let mut in_workspace = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[workspace]" {
            in_workspace = true;
            continue;
        }
        if in_workspace && trimmed.starts_with('[') {
            break;
        }
        if in_workspace && trimmed.starts_with("members") {
            // Extract quoted strings from the members line(s)
            for part in trimmed.split('"') {
                let p = part.trim();
                if !p.is_empty()
                    && !p.contains('=')
                    && !p.starts_with('[')
                    && !p.starts_with(']')
                    && p != "members"
                    && !p.contains(',')
                {
                    patterns.push(p.to_string());
                }
            }
        }
    }

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
