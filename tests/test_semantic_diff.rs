use codeaware_mcp::tools::semantic_diff::compute_semantic_diff;

#[test]
fn test_no_changes() {
    let code = "pub fn hello() {}\n";
    let changes = compute_semantic_diff(code, code, "rust");
    assert!(changes.is_empty());
}

#[test]
fn test_signature_change_detected() {
    let old = "fn verify(token: &str) -> bool {\n    true\n}\n";
    let new = "fn verify(token: &str, key: &str) -> bool {\n    true\n}\n";
    let changes = compute_semantic_diff(old, new, "rust");
    assert!(changes.iter().any(|c| c.change_type == "signature_changed" && c.symbol == "verify"));
    assert!(changes.iter().any(|c| c.breaking));
}

#[test]
fn test_symbol_added() {
    let old = "fn existing() {}\n";
    let new = "fn existing() {}\nfn new_func() {}\n";
    let changes = compute_semantic_diff(old, new, "rust");
    assert!(changes.iter().any(|c| c.change_type == "symbol_added" && c.symbol == "new_func"));
}

#[test]
fn test_symbol_removed() {
    let old = "fn keep() {}\nfn remove_me() {}\n";
    let new = "fn keep() {}\n";
    let changes = compute_semantic_diff(old, new, "rust");
    let removed = changes.iter().find(|c| c.change_type == "symbol_removed").unwrap();
    assert_eq!(removed.symbol, "remove_me");
    assert!(removed.breaking);
}

#[test]
fn test_visibility_change() {
    let old = "fn private_fn() {}\n";
    let new = "pub fn private_fn() {}\n";
    let changes = compute_semantic_diff(old, new, "rust");
    assert!(changes.iter().any(|c| c.change_type == "visibility_changed" && c.symbol == "private_fn"));
}

#[test]
fn test_body_only_change() {
    let old = "fn foo() {\n    1\n}\n";
    let new = "fn foo() {\n    1\n    2\n    3\n}\n";
    let changes = compute_semantic_diff(old, new, "rust");
    assert!(changes.iter().any(|c| c.change_type == "body_changed" && c.symbol == "foo"));
    // body changes are not breaking
    assert!(!changes.iter().filter(|c| c.change_type == "body_changed").any(|c| c.breaking));
}

#[test]
fn test_breaking_flag() {
    let old = "pub fn api() -> i32 {\n    42\n}\n";
    let new = "pub fn api() -> String {\n    String::new()\n}\n";
    let changes = compute_semantic_diff(old, new, "rust");
    let sig_change = changes.iter().find(|c| c.change_type == "signature_changed").unwrap();
    assert!(sig_change.breaking);
}

#[test]
fn test_multiple_changes() {
    let old = "fn a() {}\nfn b() {}\nfn c() {}\n";
    let new = "fn a(x: i32) {}\nfn c() {}\nfn d() {}\n";
    let changes = compute_semantic_diff(old, new, "rust");
    // a: signature changed, b: removed, d: added
    assert!(changes.iter().any(|c| c.change_type == "signature_changed" && c.symbol == "a"));
    assert!(changes.iter().any(|c| c.change_type == "symbol_removed" && c.symbol == "b"));
    assert!(changes.iter().any(|c| c.change_type == "symbol_added" && c.symbol == "d"));
}
