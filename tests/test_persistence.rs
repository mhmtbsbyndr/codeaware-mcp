use codeaware_mcp::session::persistence::SessionDb;
use tempfile::TempDir;

#[test]
fn test_open_creates_tables() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db = SessionDb::open(&db_path).unwrap();
    // Should not panic, tables should exist
    drop(db);
}

#[test]
fn test_save_and_load_session() {
    let dir = TempDir::new().unwrap();
    let db = SessionDb::open(&dir.path().join("test.db")).unwrap();

    db.save_session(
        "s-123",
        "/test/project",
        "2026-04-01T12:00:00Z",
        "Summary text",
        r#"[{"path":"src/main.rs"}]"#,
        r#"{"raw":1000}"#,
    )
    .unwrap();

    let session = db.load_latest_session("/test/project").unwrap();
    assert!(session.is_some());
    let s = session.unwrap();
    assert_eq!(s.id, "s-123");
    assert_eq!(s.summary, Some("Summary text".into()));
}

#[test]
fn test_record_file_access() {
    let dir = TempDir::new().unwrap();
    let db = SessionDb::open(&dir.path().join("test.db")).unwrap();

    db.record_file_access("/project", "src/main.rs", "skeleton")
        .unwrap();
    db.record_file_access("/project", "src/main.rs", "focused")
        .unwrap();
    db.record_file_access("/project", "src/lib.rs", "full")
        .unwrap();

    let patterns = db.get_file_access_patterns("/project").unwrap();
    assert_eq!(patterns.len(), 2);
    let main_rs = patterns
        .iter()
        .find(|p| p.file_path == "src/main.rs")
        .unwrap();
    assert_eq!(main_rs.access_count, 2);
}

#[test]
fn test_record_error_signature() {
    let dir = TempDir::new().unwrap();
    let db = SessionDb::open(&dir.path().join("test.db")).unwrap();

    db.record_error_signature("/project", "err_hash_1", None)
        .unwrap();
    db.record_error_signature(
        "/project",
        "err_hash_1",
        Some("Fix: add error handling"),
    )
    .unwrap();

    let sig = db.get_error_signature("/project", "err_hash_1").unwrap();
    assert!(sig.is_some());
    let s = sig.unwrap();
    assert_eq!(s.occurrence_count, 2);
    assert_eq!(s.typical_fix, Some("Fix: add error handling".into()));
}

#[test]
fn test_concurrent_access_wal_mode() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db1 = SessionDb::open(&db_path).unwrap();
    let db2 = SessionDb::open(&db_path).unwrap();

    // Both can read simultaneously
    let _ = db1.get_file_access_patterns("/project").unwrap();
    let _ = db2.get_file_access_patterns("/project").unwrap();
}

#[test]
fn test_fts5_event_indexing_and_search() {
    let dir = TempDir::new().unwrap();
    let db = SessionDb::open(&dir.path().join("test.db")).unwrap();

    db.index_session_event(
        "sess-001", "smart_read", Some("src/auth.rs"),
        Some("AuthManager login verify_token"),
        Some("AuthManager struct with JWT based auth"), None,
    ).unwrap();

    db.index_session_event(
        "sess-001", "smart_read", Some("src/db.rs"),
        Some("Database query"), Some("Database connection pool"), None,
    ).unwrap();

    let results = db.search_session_events("sess-001", "auth").unwrap();
    assert!(!results.is_empty(), "Should find auth-related event");

    let empty = db.search_session_events("sess-001", "nonexistent_xyz_abc_q99").unwrap();
    assert!(empty.is_empty(), "Should return empty for non-matching query");
}

#[test]
fn test_fts5_bad_query_does_not_crash() {
    let dir = TempDir::new().unwrap();
    let db = SessionDb::open(&dir.path().join("test.db")).unwrap();
    // Malformed FTS5 query — should not panic
    let _result = db.search_session_events("sess-001", "\"unterminated");
    // Just ensure no panic
}
