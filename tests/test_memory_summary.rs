use codeaware_mcp::session::persistence::{SessionDb, SaveObservationOpts};
use codeaware_mcp::tools::memory_summary::handle_summarize_memory;
use tempfile::TempDir;

fn test_db() -> (TempDir, SessionDb) {
    let dir = TempDir::new().unwrap();
    let db = SessionDb::open(&dir.path().join("test.db")).unwrap();
    (dir, db)
}

#[test]
fn test_summarize_empty_project() {
    let (_dir, db) = test_db();
    let params = serde_json::json!({ "project": "nonexistent" });
    let result = handle_summarize_memory(&params, &db);
    assert_eq!(result["ok"], true);
    assert_eq!(result["data"]["clusters"], 0);
    assert_eq!(result["data"]["summaries_created"], 0);
    assert_eq!(result["data"]["duplicates_removed"], 0);
}

#[test]
fn test_summarize_clusters_by_shared_files() {
    let (_dir, db) = test_db();
    let project = "test-proj";

    // Two observations sharing files → should cluster together
    db.save_observation(&SaveObservationOpts {
        title: Some("Auth fix"),
        text: "Fixed JWT token validation in the auth middleware.",
        observation_type: "bugfix",
        concepts: Some("problem-solution"),
        project: Some(project),
        files: Some("src/auth.rs,src/middleware.rs"),
        facts: None,
    }).unwrap();

    db.save_observation(&SaveObservationOpts {
        title: Some("Auth refactor"),
        text: "Refactored auth middleware to use async handlers.",
        observation_type: "refactor",
        concepts: Some("what-changed"),
        project: Some(project),
        files: Some("src/auth.rs,src/middleware.rs,src/handlers.rs"),
        facts: None,
    }).unwrap();

    // Unrelated observation with different files
    db.save_observation(&SaveObservationOpts {
        title: Some("DB migration"),
        text: "Added new column to users table for two-factor authentication.",
        observation_type: "feature",
        concepts: Some("what-changed"),
        project: Some(project),
        files: Some("migrations/042.sql,src/models/user.rs"),
        facts: None,
    }).unwrap();

    let params = serde_json::json!({ "project": project });
    let result = handle_summarize_memory(&params, &db);
    assert_eq!(result["ok"], true);

    let data = &result["data"];
    // At least 2 clusters (auth-related and db-related)
    assert!(data["clusters"].as_u64().unwrap() >= 2,
        "Expected at least 2 clusters, got {}", data["clusters"]);
    assert!(data["summaries_created"].as_u64().unwrap() >= 2,
        "Expected at least 2 summaries, got {}", data["summaries_created"]);
}

#[test]
fn test_summarize_deduplicates_similar_entries() {
    let (_dir, db) = test_db();
    let project = "dedup-proj";

    // Two near-identical observations
    db.save_observation(&SaveObservationOpts {
        title: Some("Error handling fix"),
        text: "Fixed the error handling in the payment processing module to properly catch timeout exceptions and retry the transaction.",
        observation_type: "bugfix",
        concepts: Some("problem-solution"),
        project: Some(project),
        files: Some("src/payments.rs"),
        facts: None,
    }).unwrap();

    db.save_observation(&SaveObservationOpts {
        title: Some("Error handling fix v2"),
        text: "Fixed the error handling in the payment processing module to properly catch timeout exceptions and retry the transaction automatically.",
        observation_type: "bugfix",
        concepts: Some("problem-solution"),
        project: Some(project),
        files: Some("src/payments.rs"),
        facts: None,
    }).unwrap();

    // A distinct observation
    db.save_observation(&SaveObservationOpts {
        title: Some("New feature"),
        text: "Implemented webhook notifications for completed payment transactions with configurable retry policy.",
        observation_type: "feature",
        concepts: Some("how-it-works"),
        project: Some(project),
        files: Some("src/payments.rs,src/webhooks.rs"),
        facts: None,
    }).unwrap();

    let params = serde_json::json!({ "project": project });
    let result = handle_summarize_memory(&params, &db);
    assert_eq!(result["ok"], true);

    let data = &result["data"];
    // Should detect at least 1 duplicate
    assert!(data["duplicates_removed"].as_u64().unwrap() >= 1,
        "Expected at least 1 duplicate removed, got {}", data["duplicates_removed"]);
}

#[test]
fn test_summarize_generates_summaries_with_topic() {
    let (_dir, db) = test_db();
    let project = "summary-proj";

    for i in 0..5 {
        db.save_observation(&SaveObservationOpts {
            title: Some(&format!("Config observation {}", i)),
            text: &format!("The configuration system uses TOML format. Entry {} describes how {} config keys are validated at startup.", i, i * 3),
            observation_type: "discovery",
            concepts: Some("how-it-works"),
            project: Some(project),
            files: Some("src/config.rs"),
            facts: None,
        }).unwrap();
    }

    let params = serde_json::json!({ "project": project });
    let result = handle_summarize_memory(&params, &db);
    assert_eq!(result["ok"], true);

    let data = &result["data"];
    assert!(data["summaries_created"].as_u64().unwrap() >= 1,
        "Expected at least 1 summary created");
}

#[test]
fn test_summarize_default_project() {
    // When no project is given, should default to "."
    let (_dir, db) = test_db();

    db.save_observation(&SaveObservationOpts {
        title: None,
        text: "A standalone observation without explicit project.",
        observation_type: "discovery",
        concepts: None,
        project: Some("."),
        files: None,
        facts: None,
    }).unwrap();

    let params = serde_json::json!({});
    let result = handle_summarize_memory(&params, &db);
    assert_eq!(result["ok"], true);
    assert!(result["data"]["clusters"].as_u64().unwrap() >= 1);
}

#[test]
fn test_persistence_methods() {
    let (_dir, db) = test_db();
    let project = "persist-test";

    // save some observations
    let id1 = db.save_observation(&SaveObservationOpts {
        title: Some("Obs 1"),
        text: "First observation text",
        observation_type: "discovery",
        concepts: None,
        project: Some(project),
        files: None,
        facts: None,
    }).unwrap();

    let id2 = db.save_observation(&SaveObservationOpts {
        title: Some("Obs 2"),
        text: "Second observation text",
        observation_type: "discovery",
        concepts: None,
        project: Some(project),
        files: None,
        facts: None,
    }).unwrap();

    // get_all_observations_for_project
    let all = db.get_all_observations_for_project(project).unwrap();
    assert_eq!(all.len(), 2);

    // save_summary
    let summary_id = db.save_summary(project, "test-topic", "Summary text", &format!("{},{}", id1, id2)).unwrap();
    assert!(summary_id > 0);

    // delete_observations_by_ids
    let deleted = db.delete_observations_by_ids(&[id1]).unwrap();
    assert_eq!(deleted, 1);

    let remaining = db.get_all_observations_for_project(project).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, id2);

    // Delete empty slice should be no-op
    let deleted_empty = db.delete_observations_by_ids(&[]).unwrap();
    assert_eq!(deleted_empty, 0);
}
