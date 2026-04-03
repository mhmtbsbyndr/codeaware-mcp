use codeaware_mcp::tools::project_map::{project_map, ProjectMapInput};
use tempfile::TempDir;
use std::fs;

#[test]
fn test_project_map_basic() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\nfn helper() {}\n").unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub mod auth;\n").unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();

    let input = ProjectMapInput {
        path: dir.path().to_string_lossy().to_string(),
        depth: 3,
        include_symbols: false,
        filter_language: None,
        task_context: None,
    };

    let result = project_map(&input).unwrap();
    assert_eq!(result.total_files, 3);
    assert!(result.total_loc > 0);
    assert!(!result.tree.is_empty());
}

#[test]
fn test_project_map_respects_ignore() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::create_dir_all(dir.path().join("target/debug")).unwrap();
    fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();
    fs::write(dir.path().join("target/debug/app"), "binary").unwrap();
    fs::write(dir.path().join("node_modules/pkg/index.js"), "module.exports = {}").unwrap();

    let input = ProjectMapInput {
        path: dir.path().to_string_lossy().to_string(),
        depth: 3,
        include_symbols: false,
        filter_language: None,
        task_context: None,
    };

    let result = project_map(&input).unwrap();
    let paths: Vec<&str> = result.tree.iter().map(|e| e.path.as_str()).collect();
    assert!(paths.iter().any(|p| p.contains("main.rs")));
    assert!(!paths.iter().any(|p| p.contains("target")));
    assert!(!paths.iter().any(|p| p.contains("node_modules")));
}

#[test]
fn test_project_map_depth_limit() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b/c/d/e")).unwrap();
    fs::write(dir.path().join("a/file.rs"), "x").unwrap();
    fs::write(dir.path().join("a/b/c/d/e/deep.rs"), "y").unwrap();

    let input = ProjectMapInput {
        path: dir.path().to_string_lossy().to_string(),
        depth: 2,
        include_symbols: false,
        filter_language: None,
        task_context: None,
    };

    let result = project_map(&input).unwrap();
    let paths: Vec<&str> = result.tree.iter().map(|e| e.path.as_str()).collect();
    assert!(paths.iter().any(|p| p.contains("file.rs")));
    assert!(!paths.iter().any(|p| p.contains("deep.rs")));
}
