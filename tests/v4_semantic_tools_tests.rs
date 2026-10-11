use codeaware_mcp::v4::{FindCallersRequest, FindSymbolRequest, SemanticTools};
use tempfile::tempdir;

#[test]
fn semantic_tools_find_symbol_runs() {
    let dir = tempdir().unwrap();

    std::fs::create_dir_all(dir.path().join("src")).unwrap();

    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn build_context() {}",
    )
    .unwrap();

    let result = SemanticTools::find_symbol(FindSymbolRequest {
        repo_root: dir.path().to_string_lossy().to_string(),
        query: "build_context".to_string(),
    })
    .expect("find_symbol should succeed on a valid repo");

    assert!(!result.matches.is_empty());
}

#[test]
fn semantic_tools_find_callers_runs() {
    let dir = tempdir().unwrap();

    std::fs::create_dir_all(dir.path().join("src")).unwrap();

    std::fs::write(
        dir.path().join("src/lib.rs"),
        "fn caller() { target(); } fn target() {}",
    )
    .unwrap();

    let result = SemanticTools::find_callers(FindCallersRequest {
        repo_root: dir.path().to_string_lossy().to_string(),
        symbol: "target".to_string(),
    })
    .expect("find_callers should succeed on a valid repo");

    assert!(!result.callers.is_empty());
}
