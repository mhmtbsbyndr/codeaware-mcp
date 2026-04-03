use codeaware_mcp::tools::smart_read::{smart_read, SmartReadInput, ReadMode};
use tempfile::TempDir;
use std::fs;

#[test]
fn test_skeleton_uses_tree_sitter_for_rust() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("auth.rs");
    let code = r#"use std::io;

/// Auth manager for JWT tokens
pub struct AuthManager {
    secret: String,
    duration: i64,
}

impl AuthManager {
    pub fn new(secret: String) -> Self {
        Self { secret, duration: 24 }
    }

    pub fn verify_token(&self, token: &str) -> Result<bool, String> {
        if token.len() < 10 {
            return Err("Token too short".into());
        }
        Ok(true)
    }

    fn private_helper(&self) -> i64 {
        self.duration * 3600
    }
}

pub enum Role {
    Admin,
    User,
    ReadOnly,
}

fn standalone_function(x: i32) -> i32 {
    x * 2
}
"#;
    fs::write(&file, code).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Skeleton,
        focus: None,
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "skeleton");
    assert!(!result.symbols.is_empty());

    let sym_names: Vec<&str> = result.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(sym_names.contains(&"AuthManager"));
    assert!(sym_names.contains(&"new"));
    assert!(sym_names.contains(&"verify_token"));
    assert!(sym_names.contains(&"Role"));
    assert!(sym_names.contains(&"standalone_function"));

    // Intelligence level should be "tree-sitter" not "heuristic"
    assert_eq!(result.intelligence_level, "tree-sitter");
}

#[test]
fn test_skeleton_falls_back_for_unknown_language() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("code.rb");
    let code = "def hello\n  puts 'hi'\nend\n\nclass Foo\n  def bar\n    1\n  end\nend\n";
    // Pad to > 100 lines
    let mut content = code.to_string();
    for i in 0..100 { content.push_str(&format!("# comment {i}\n")); }
    fs::write(&file, &content).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Skeleton,
        focus: None,
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "skeleton");
    // Should fall back to heuristic
    assert!(result.intelligence_level == "heuristic" || result.intelligence_level == "regex");
}

#[test]
fn test_focused_with_symbol_name() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("large.rs");
    let mut code = String::new();
    for i in 0..50 { code.push_str(&format!("fn padding_{i}() {{ }}\n")); }
    code.push_str("pub fn target_function(x: i32, y: i32) -> i32 {\n    let sum = x + y;\n    let product = x * y;\n    sum + product\n}\n");
    for i in 0..50 { code.push_str(&format!("fn more_padding_{i}() {{ }}\n")); }
    fs::write(&file, &code).unwrap();

    let input = SmartReadInput {
        path: file.to_string_lossy().to_string(),
        mode: ReadMode::Auto,
        focus: Some("target_function".into()),
        lines: None,
        scope: None,
    };

    let result = smart_read(&input, dir.path()).unwrap();
    assert_eq!(result.mode_used, "focused");
    let content = result.content.unwrap();
    assert!(content.contains("target_function"));
    assert!(content.contains("let sum"));
}

#[test]
fn test_project_map_includes_symbols() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.rs"), "pub fn main() {}\nfn helper() {}\n").unwrap();

    use codeaware_mcp::tools::project_map::{project_map, ProjectMapInput};
    let input = ProjectMapInput {
        path: dir.path().to_string_lossy().to_string(),
        depth: 3,
        include_symbols: true,
        filter_language: None,
        task_context: None,
    };

    let result = project_map(&input).unwrap();
    let main_file = result.tree.iter().find(|f| f.path.contains("main.rs")).unwrap();
    assert!(!main_file.symbols.is_empty());
    assert!(main_file.symbols.contains(&"main".to_string()));
    assert!(main_file.symbols.contains(&"helper".to_string()));
}
