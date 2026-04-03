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
        query: "JWT token",
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
        query: "caching",
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
        query: "error handling",
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
    assert!(timeline.len() >= 5, "Should have at least 5 entries (3 before + anchor + 3 after but anchor is included in before)");

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
fn test_fts5_bad_query_graceful() {
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

    // Malformed FTS5 query should not panic
    let result = db.search_observations(&SearchObservationsOpts {
        query: "\"unterminated",
        project: None,
        observation_type: None,
        limit: 10,
        offset: 0,
        date_start: None,
        date_end: None,
    });
    // Either returns results or an error, but should not panic
    let _ = result;
}
