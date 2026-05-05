#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRoot {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceManifest {
    pub workspace_name: String,
    pub roots: Vec<WorkspaceRoot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRepoMatch {
    pub repo_name: String,
    pub file_path: String,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMap {
    pub workspace_name: String,
    pub repositories: Vec<String>,
    pub total_roots: usize,
}

impl WorkspaceManifest {
    pub fn workspace_map(&self) -> WorkspaceMap {
        WorkspaceMap {
            workspace_name: self.workspace_name.clone(),
            repositories: self
                .roots
                .iter()
                .map(|root| root.name.clone())
                .collect(),
            total_roots: self.roots.len(),
        }
    }

    pub fn cross_repo_search(&self, query: &str) -> Vec<CrossRepoMatch> {
        self.roots
            .iter()
            .filter(|root| root.name.to_lowercase().contains(&query.to_lowercase()))
            .map(|root| CrossRepoMatch {
                repo_name: root.name.clone(),
                file_path: format!("{}/README.md", root.path),
                symbol: query.to_string(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> WorkspaceManifest {
        WorkspaceManifest {
            workspace_name: "platform".to_string(),
            roots: vec![
                WorkspaceRoot {
                    name: "api".to_string(),
                    path: "../api".to_string(),
                },
                WorkspaceRoot {
                    name: "frontend".to_string(),
                    path: "../frontend".to_string(),
                },
            ],
        }
    }

    #[test]
    fn builds_workspace_map() {
        let map = manifest().workspace_map();

        assert_eq!(map.total_roots, 2);
        assert!(map.repositories.contains(&"api".to_string()));
    }

    #[test]
    fn supports_cross_repo_search() {
        let results = manifest().cross_repo_search("api");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].repo_name, "api");
    }
}
