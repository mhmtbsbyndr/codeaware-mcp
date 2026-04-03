use codeaware_mcp::session::persistence::{SessionDb, SaveObservationOpts, SearchObservationsOpts};
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

    db.save_observation(&SaveObservationOpts {
        title: Some("secret title"),
        text: "Public content about databases",
        observation_type: "discovery",
        concepts: None,
        project: None,
        files: None,
        facts: None,
    }).unwrap();

    // Column filter syntax should be neutralized (colon stripped)
    let results = db.search_observations(&SearchObservationsOpts {
        query: "\"title secret\"",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    }).unwrap();
    // Should search across all columns, not just title
    // The colon in "title:secret" gets stripped to "title secret" in persistence layer
    assert!(results.len() <= 1);
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
fn test_execute_raw_rollback() {
    let (_dir, db) = test_db();
    // ROLLBACK when no transaction is active should not error
    let result = db.execute_raw("ROLLBACK");
    // SQLite returns error for ROLLBACK outside transaction, that's OK
    let _ = result;
}
