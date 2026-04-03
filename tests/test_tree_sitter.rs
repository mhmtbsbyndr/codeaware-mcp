use codeaware_mcp::intelligence::tree_sitter_provider::{TreeSitterProvider, SymbolInfo, SymbolKind};
use codeaware_mcp::intelligence::IntelligenceLevel;

#[test]
fn test_select_intelligence_with_tree_sitter() {
    use codeaware_mcp::intelligence::select_intelligence;
    let level = select_intelligence("rust", false);
    assert_eq!(level, IntelligenceLevel::TreeSitter);
}

#[test]
fn test_select_intelligence_unknown_language() {
    use codeaware_mcp::intelligence::select_intelligence;
    let level = select_intelligence("cobol", false);
    assert_eq!(level, IntelligenceLevel::Regex);
}

#[test]
fn test_extract_rust_functions() {
    let provider = TreeSitterProvider::new();
    let code = r#"
pub fn hello(name: &str) -> String {
    format!("Hello, {name}")
}

fn private_helper() -> i32 {
    42
}

pub async fn async_handler(req: Request) -> Response {
    Response::ok()
}
"#;
    let symbols = provider.extract_symbols(code, "rust").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"hello"));
    assert!(names.contains(&"private_helper"));
    assert!(names.contains(&"async_handler"));
    assert_eq!(symbols.len(), 3);

    let hello = symbols.iter().find(|s| s.name == "hello").unwrap();
    assert_eq!(hello.kind, SymbolKind::Function);
    assert!(hello.signature.contains("pub fn hello"));
    assert!(hello.signature.contains("-> String"));
}

#[test]
fn test_extract_rust_structs_and_enums() {
    let provider = TreeSitterProvider::new();
    let code = r#"
pub struct AuthManager {
    secret: String,
    duration: i64,
}

pub enum Role {
    Admin,
    User,
    ReadOnly,
}

pub trait Authenticator {
    fn verify(&self, token: &str) -> bool;
}
"#;
    let symbols = provider.extract_symbols(code, "rust").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"AuthManager"));
    assert!(names.contains(&"Role"));
    assert!(names.contains(&"Authenticator"));

    let auth = symbols.iter().find(|s| s.name == "AuthManager").unwrap();
    assert_eq!(auth.kind, SymbolKind::Struct);

    let role = symbols.iter().find(|s| s.name == "Role").unwrap();
    assert_eq!(role.kind, SymbolKind::Enum);

    let trait_sym = symbols.iter().find(|s| s.name == "Authenticator").unwrap();
    assert_eq!(trait_sym.kind, SymbolKind::Trait);
}

#[test]
fn test_extract_rust_impl_methods() {
    let provider = TreeSitterProvider::new();
    let code = r#"
impl AuthManager {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AuthError> {
        todo!()
    }

    fn private_method(&self) {}
}
"#;
    let symbols = provider.extract_symbols(code, "rust").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"new"));
    assert!(names.contains(&"verify_token"));
    assert!(names.contains(&"private_method"));

    // Methods should have SymbolKind::Method
    let new_sym = symbols.iter().find(|s| s.name == "new").unwrap();
    assert_eq!(new_sym.kind, SymbolKind::Method);
}

#[test]
fn test_extract_symbols_returns_line_ranges() {
    let provider = TreeSitterProvider::new();
    let code = "fn foo() {\n    1\n}\n\nfn bar() {\n    2\n}\n";
    let symbols = provider.extract_symbols(code, "rust").unwrap();

    let foo = symbols.iter().find(|s| s.name == "foo").unwrap();
    assert_eq!(foo.start_line, 1); // 1-based
    assert_eq!(foo.end_line, 3);

    let bar = symbols.iter().find(|s| s.name == "bar").unwrap();
    assert_eq!(bar.start_line, 5);
    assert_eq!(bar.end_line, 7);
}

#[test]
fn test_build_skeleton_from_symbols() {
    let provider = TreeSitterProvider::new();
    let code = r#"use std::io;

/// Documentation comment
pub fn important_function(x: i32) -> String {
    let internal = x * 2;
    let another = internal + 1;
    format!("{another}")
}

struct Config {
    name: String,
    value: i32,
}

fn helper() {
    println!("hi");
}
"#;
    let skeleton = provider.build_skeleton(code, "rust").unwrap();
    // Skeleton should contain signatures but not function bodies
    assert!(skeleton.contains("pub fn important_function"));
    assert!(skeleton.contains("struct Config"));
    assert!(skeleton.contains("fn helper"));
    // Should NOT contain internal variable declarations
    assert!(!skeleton.contains("let internal"));
    assert!(!skeleton.contains("let another"));
}

#[test]
fn test_unsupported_language_returns_error() {
    let provider = TreeSitterProvider::new();
    let result = provider.extract_symbols("some code", "brainfuck");
    assert!(result.is_err());
}

#[test]
fn test_extract_php_class_and_methods() {
    let provider = TreeSitterProvider::new();
    let code = r#"<?php
namespace App\Http\Controllers;

class AuthController {
    public function login() {
        return "login";
    }

    private function validateToken(string $token): bool {
        return strlen($token) > 0;
    }
}

function globalHelper() {
    return true;
}
"#;
    let symbols = provider.extract_symbols(code, "php").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(!symbols.is_empty(), "No PHP symbols extracted, got: {:?}", names);
    // At minimum, the class or a method/function should be found
    assert!(
        names.iter().any(|n| *n == "AuthController" || *n == "login" || *n == "globalHelper"),
        "Expected AuthController, login, or globalHelper in {:?}", names
    );
}

#[test]
fn test_extract_swift_functions() {
    let provider = TreeSitterProvider::new();
    let code = r#"
import Foundation

func greet(name: String) -> String {
    return "Hello, \(name)"
}

class UserManager {
    func fetchUser(id: Int) -> User? {
        return nil
    }
}

struct Point {
    var x: Double
    var y: Double
}

protocol Drawable {
    func draw()
}
"#;
    let symbols = provider.extract_symbols(code, "swift").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(!symbols.is_empty(), "No Swift symbols extracted, got: {:?}", names);
    // Should find at least greet function or UserManager class
    assert!(
        names.iter().any(|n| *n == "greet" || *n == "UserManager" || *n == "Point" || *n == "Drawable"),
        "Expected greet, UserManager, Point, or Drawable in {:?}", names
    );
}

#[test]
fn test_supported_languages_count() {
    // V22 requires 6 languages: Rust, Python, TypeScript, JavaScript, Go, PHP, Swift
    // (TypeScript and JavaScript count as separate but use same grammar family)
    let provider = TreeSitterProvider::new();
    let languages = ["rust", "python", "typescript", "javascript", "go", "php", "swift"];
    for lang in &languages {
        // Each should at least parse without error (even if empty result)
        let result = provider.extract_symbols("", lang);
        assert!(result.is_ok(), "Language '{}' failed to parse: {:?}", lang, result);
    }
}
