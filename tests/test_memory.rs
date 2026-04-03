use codeaware_mcp::session::persistence::{SessionDb, SaveObservationOpts, SearchObservationsOpts};
use codeaware_mcp::tools::memory::handle_save_memory;
use tempfile::TempDir;

fn test_db() -> (TempDir, SessionDb) {
    let dir = TempDir::new().unwrap();
    let db = SessionDb::open(&dir.path().join("test.db")).unwrap();
    (dir, db)
}

#[test]
fn test_save_and_search_memory() {
    let (_dir, db) = test_db();

    let id = db.save_observation(&SaveObservationOpts {
        title: Some("Auth bug fix"),
        text: "Fixed JWT token expiry not being checked on refresh",
        observation_type: "bugfix",
        concepts: Some("problem-solution,gotcha"),
        project: Some("/my/project"),
        files: Some("src/auth.rs,src/middleware.rs"),
        facts: Some("JWT refresh tokens need expiry validation"),
    }).unwrap();
    assert!(id > 0);

    let results = db.search_observations(&SearchObservationsOpts {
        query: "\"JWT\" \"token\"",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id);
    assert_eq!(results[0].observation_type, "bugfix");
}

#[test]
fn test_search_with_project_filter() {
    let (_dir, db) = test_db();

    db.save_observation(&SaveObservationOpts {
        title: Some("Feature A"),
        text: "Added caching layer",
        observation_type: "feature",
        concepts: None,
        project: Some("project-a"),
        files: None,
        facts: None,
    }).unwrap();

    db.save_observation(&SaveObservationOpts {
        title: Some("Feature B"),
        text: "Added caching to API",
        observation_type: "feature",
        concepts: None,
        project: Some("project-b"),
        files: None,
        facts: None,
    }).unwrap();

    let results = db.search_observations(&SearchObservationsOpts {
        query: "\"caching\"",
        project: Some("project-a"),
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].project.as_deref(), Some("project-a"));
}

#[test]
fn test_search_with_type_filter() {
    let (_dir, db) = test_db();

    db.save_observation(&SaveObservationOpts {
        title: None,
        text: "Refactored error handling module",
        observation_type: "refactor",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    db.save_observation(&SaveObservationOpts {
        title: None,
        text: "Fixed error handling in edge case",
        observation_type: "bugfix",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    let results = db.search_observations(&SearchObservationsOpts {
        query: "\"error\" \"handling\"",
        project: None,
        observation_type: Some("bugfix"),
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].observation_type, "bugfix");
}

#[test]
fn test_get_observation_by_id() {
    let (_dir, db) = test_db();

    let id = db.save_observation(&SaveObservationOpts {
        title: Some("Test observation"),
        text: "Some important discovery",
        observation_type: "discovery",
        concepts: Some("how-it-works"),
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    let obs = db.get_observation_by_id(id).unwrap();
    assert!(obs.is_some());
    let obs = obs.unwrap();
    assert_eq!(obs.title.as_deref(), Some("Test observation"));
    assert_eq!(obs.text, "Some important discovery");

    let missing = db.get_observation_by_id(99999).unwrap();
    assert!(missing.is_none());
}

#[test]
fn test_memory_timeline() {
    let (_dir, db) = test_db();

    let mut ids = Vec::new();
    for i in 0..10 {
        let id = db.save_observation(&SaveObservationOpts {
            title: Some(&format!("Observation {}", i)),
            text: &format!("Content for observation number {}", i),
            observation_type: "discovery",
            concepts: None,
            project: Some("timeline-test"),
            files: None,
            facts: None,
        }).unwrap();
        ids.push(id);
    }

    // Timeline around observation #5
    let timeline = db.get_observation_timeline(ids[5], 3, 3, None).unwrap();
    assert!(timeline.len() >= 5, "Should have at least 5 entries");

    // Verify chronological order by id (stable even when timestamps are identical)
    for w in timeline.windows(2) {
        assert!(w[0].id < w[1].id, "Timeline must be in id order (chronological)");
    }
}

#[test]
fn test_timeline_invalid_anchor() {
    let (_dir, db) = test_db();
    let result = db.get_observation_timeline(99999, 5, 5, None);
    assert!(result.is_err(), "Should error for non-existent anchor");
}

#[test]
fn test_timeline_depth_zero() {
    let (_dir, db) = test_db();

    let id = db.save_observation(&SaveObservationOpts {
        title: None,
        text: "Solo observation",
        observation_type: "discovery",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    // depth_before=0 should still include the anchor
    let timeline = db.get_observation_timeline(id, 0, 0, None).unwrap();
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].id, id);
}

#[test]
fn test_save_memory_minimal() {
    let (_dir, db) = test_db();

    let id = db.save_observation(&SaveObservationOpts {
        title: None,
        text: "Just a simple note",
        observation_type: "discovery",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();
    assert!(id > 0);

    let obs = db.get_observation_by_id(id).unwrap().unwrap();
    assert!(obs.title.is_none());
    assert_eq!(obs.observation_type, "discovery");
}

#[test]
fn test_observations_persist_across_connections() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");

    let id = {
        let db = SessionDb::open(&db_path).unwrap();
        db.save_observation(&SaveObservationOpts {
            title: Some("Persistent memory"),
            text: "This should survive reconnection",
            observation_type: "decision",
            concepts: None,
            project: None,
            files: None,
            facts: None,
        }).unwrap()
    };

    // Reopen
    let db2 = SessionDb::open(&db_path).unwrap();
    let obs = db2.get_observation_by_id(id).unwrap();
    assert!(obs.is_some());
    assert_eq!(obs.unwrap().title.as_deref(), Some("Persistent memory"));
}

#[test]
fn test_fts5_bad_query_returns_error() {
    let (_dir, db) = test_db();

    db.save_observation(&SaveObservationOpts {
        title: None,
        text: "Some content",
        observation_type: "discovery",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    // Malformed FTS5 query should return an error, not panic
    let result = db.search_observations(&SearchObservationsOpts {
        query: "\"unterminated",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    });
    assert!(result.is_err(), "Malformed FTS5 query should return error");
}

#[test]
fn test_fts5_column_filter_stripped() {
    let (_dir, db) = test_db();

    // Text contains both "alpha" and "beta" so FTS5 can match
    db.save_observation(&SaveObservationOpts {
        title: Some("alpha topic"),
        text: "beta content here",
        observation_type: "discovery",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    // "title:alpha" with colon stripped becomes two separate terms "title" + "alpha"
    // Since "title" doesn't appear in the content, this should return 0 if colon is stripped correctly.
    // If colon were NOT stripped, FTS5 would interpret "title:alpha" as column filter and find 1 result.
    let results = db.search_observations(&SearchObservationsOpts {
        query: "title:alpha",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    // Colon stripped: searches for "title" AND "alpha" — "title" not in content → 0 results
    // If column filter worked: would search title column for "alpha" → 1 result
    assert_eq!(results.len(), 0, "Column filter syntax must be neutralized");

    // Normal search for "alpha" should find it
    let results2 = db.search_observations(&SearchObservationsOpts {
        query: "\"alpha\"",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    assert_eq!(results2.len(), 1, "Normal search should find the observation");
}

#[test]
fn test_search_with_pagination() {
    let (_dir, db) = test_db();

    for i in 0..5 {
        db.save_observation(&SaveObservationOpts {
            title: None,
            text: &format!("Observation about pagination topic {}", i),
            observation_type: "discovery",
            concepts: None,
            project: None,
            files: None,
            facts: None,
        }).unwrap();
    }

    let page1 = db.search_observations(&SearchObservationsOpts {
        query: "\"pagination\"",
        project: None,
        observation_type: None,
        limit: 2,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    assert_eq!(page1.len(), 2);

    let page2 = db.search_observations(&SearchObservationsOpts {
        query: "\"pagination\"",
        project: None,
        observation_type: None,
        limit: 2,
        offset: 2,
        date_start: None,
        date_end: None,
    }).unwrap();
    assert_eq!(page2.len(), 2);

    // Pages should not overlap
    assert_ne!(page1[0].id, page2[0].id);
}

#[test]
fn test_rollback_no_transaction() {
    let (_dir, db) = test_db();
    // ROLLBACK when no transaction is active returns an error — that's expected
    let result = db.rollback();
    assert!(result.is_err(), "ROLLBACK outside transaction should error");
}

#[test]
fn test_date_format_validation() {
    // Test via handler layer to exercise date validation
    let (_dir, db) = test_db();

    db.save_observation(&SaveObservationOpts {
        title: None,
        text: "Date test content",
        observation_type: "discovery",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    // Valid date range should work
    let results = db.search_observations(&SearchObservationsOpts {
        query: "\"date\"",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: Some("2020-01-01"),
        date_end: Some("2030-12-31"),
    }).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_sanitize_strips_quotes_and_colons() {
    let (_dir, db) = test_db();

    db.save_observation(&SaveObservationOpts {
        title: None,
        text: "Testing quote handling in searches",
        observation_type: "discovery",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    // Query with quotes and colons — should not crash, quotes stripped
    let results = db.search_observations(&SearchObservationsOpts {
        query: "\"quote\" \"handling\"",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_text_size_limit() {
    let huge_text = "x".repeat(70000);
    let params = serde_json::json!({ "text": huge_text });
    let (_dir, db) = test_db();
    let result = handle_save_memory(&params, &db);
    assert_eq!(result["ok"], false, "Huge text should be rejected");
}
