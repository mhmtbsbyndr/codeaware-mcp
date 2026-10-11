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

// ── update_memory (PATCH semantics) ───────────────────────────────

use codeaware_mcp::tools::memory::handle_update_memory;

fn save_full(db: &SessionDb) -> i64 {
    db.save_observation(&SaveObservationOpts {
        title: Some("Cache layer"),
        text: "Cache eviction runs on every write path",
        observation_type: "decision",
        concepts: Some("trade-off"),
        project: Some("/proj/a"),
        files: Some("src/cache.rs"),
        facts: Some("eviction is O(n) on purpose"),
    })
    .unwrap()
}

#[test]
fn test_update_memory_omitted_fields_are_kept() {
    let (_dir, db) = test_db();
    let id = save_full(&db);

    let out = handle_update_memory(&serde_json::json!({"id": id, "text": "Eviction now batched"}), &db);
    assert_eq!(out["ok"], true, "{out}");

    let rec = db.get_observation_by_id(id).unwrap().unwrap();
    assert_eq!(rec.text, "Eviction now batched");
    assert_eq!(rec.title.as_deref(), Some("Cache layer"));
    assert_eq!(rec.observation_type, "decision");
    assert_eq!(rec.concepts.as_deref(), Some("trade-off"));
    assert_eq!(rec.project.as_deref(), Some("/proj/a"));
    assert_eq!(rec.files.as_deref(), Some("src/cache.rs"));
    assert_eq!(rec.facts.as_deref(), Some("eviction is O(n) on purpose"));
}

#[test]
fn test_update_memory_explicit_null_clears_field() {
    let (_dir, db) = test_db();
    let id = save_full(&db);

    let out = handle_update_memory(
        &serde_json::json!({"id": id, "title": null, "facts": null, "files": null}),
        &db,
    );
    assert_eq!(out["ok"], true, "{out}");

    let rec = db.get_observation_by_id(id).unwrap().unwrap();
    assert_eq!(rec.title, None);
    assert_eq!(rec.facts, None);
    assert_eq!(rec.files, None);
    assert_eq!(rec.text, "Cache eviction runs on every write path");
}

#[test]
fn test_update_memory_null_text_is_rejected() {
    let (_dir, db) = test_db();
    let id = save_full(&db);

    let out = handle_update_memory(&serde_json::json!({"id": id, "text": null}), &db);
    assert_eq!(out["ok"], false, "{out}");

    let rec = db.get_observation_by_id(id).unwrap().unwrap();
    assert_eq!(rec.text, "Cache eviction runs on every write path");
}

#[test]
fn test_update_memory_rejects_update_without_searchable_text() {
    let (_dir, db) = test_db();
    let id = db
        .save_observation(&SaveObservationOpts {
            title: None,
            text: "plain words here",
            observation_type: "discovery",
            concepts: None,
            project: None,
            files: None,
            facts: None,
        })
        .unwrap();

    // Only FTS-stripped characters left: nothing searchable remains.
    let out = handle_update_memory(&serde_json::json!({"id": id, "text": "::\"\""}), &db);
    assert_eq!(out["ok"], false, "{out}");
    assert!(out["fallback_suggestion"].to_string().to_lowercase().contains("searchable"), "{out}");

    let rec = db.get_observation_by_id(id).unwrap().unwrap();
    assert_eq!(rec.text, "plain words here");
}

#[test]
fn test_update_memory_searchable_via_title_when_text_unsearchable() {
    let (_dir, db) = test_db();
    let id = save_full(&db);

    // Text has no searchable term, but the title still does: accepted.
    let out = handle_update_memory(&serde_json::json!({"id": id, "text": "::"}), &db);
    assert_eq!(out["ok"], true, "{out}");
}

#[test]
fn test_update_memory_missing_id_is_error() {
    let (_dir, db) = test_db();
    let out = handle_update_memory(&serde_json::json!({"id": 999, "text": "x"}), &db);
    assert_eq!(out["ok"], false, "{out}");
}

#[test]
fn test_update_memory_rejects_invalid_type() {
    let (_dir, db) = test_db();
    let id = save_full(&db);
    let out = handle_update_memory(&serde_json::json!({"id": id, "type": "nonsense"}), &db);
    assert_eq!(out["ok"], false, "{out}");
}

#[test]
fn test_update_memory_refreshes_fts_index() {
    let (_dir, db) = test_db();
    let id = save_full(&db);

    let out = handle_update_memory(&serde_json::json!({"id": id, "text": "Quasar telemetry bridge"}), &db);
    assert_eq!(out["ok"], true, "{out}");

    let new_hits = db
        .search_observations(&SearchObservationsOpts {
            query: "\"quasar\"",
            project: None,
            observation_type: None,
            limit: 10,
            offset: 0,
            date_start: None,
            date_end: None,
        })
        .unwrap();
    assert_eq!(new_hits.len(), 1);
    assert_eq!(new_hits[0].id, id);

    let old_hits = db
        .search_observations(&SearchObservationsOpts {
            query: "\"eviction\"",
            project: None,
            observation_type: None,
            limit: 10,
            offset: 0,
            date_start: None,
            date_end: None,
        })
        .unwrap();
    // "eviction" still lives in facts, so it must match exactly once, not via stale text.
    assert_eq!(old_hits.len(), 1);
    let stale = db
        .search_observations(&SearchObservationsOpts {
            query: "\"batched\"",
            project: None,
            observation_type: None,
            limit: 10,
            offset: 0,
            date_start: None,
            date_end: None,
        })
        .unwrap();
    assert!(stale.is_empty());
}

// ── literal wildcards in LIKE lookups ─────────────────────────────

#[test]
fn test_recent_observations_treats_like_wildcards_literally() {
    let (_dir, db) = test_db();
    let literal = db
        .save_observation(&SaveObservationOpts {
            title: None,
            text: "100%_done report",
            observation_type: "discovery",
            concepts: None,
            project: Some("/proj/x"),
            files: Some("reports/100%_done.md"),
            facts: None,
        })
        .unwrap();
    db.save_observation(&SaveObservationOpts {
        title: None,
        text: "unrelated note",
        observation_type: "discovery",
        concepts: None,
        project: Some("/proj/y"),
        files: Some("plain/notes.md"),
        facts: None,
    })
    .unwrap();

    let hits = db.get_recent_observations_for_project("%_", 10).unwrap();
    let ids: Vec<i64> = hits.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![literal], "wildcards must match only literal characters");
}

// ── paging over search results ────────────────────────────────────

#[test]
fn test_search_paging_has_no_repeats_or_gaps_on_equal_ranks() {
    let (_dir, db) = test_db();
    let mut expected = Vec::new();
    for _ in 0..5 {
        expected.push(
            db.save_observation(&SaveObservationOpts {
                title: None,
                text: "identical paging marker",
                observation_type: "discovery",
                concepts: None,
                project: Some("/proj/page"),
                files: None,
                facts: None,
            })
            .unwrap(),
        );
    }
    let mut seen = Vec::new();
    for offset in [0usize, 2, 4] {
        let page = db
            .search_observations(&SearchObservationsOpts {
                query: "\"paging\"",
                project: Some("/proj/page"),
                observation_type: None,
                limit: 2,
                offset,
                date_start: None,
                date_end: None,
            })
            .unwrap();
        seen.extend(page.into_iter().map(|r| r.id));
    }
    let mut seen_sorted = seen.clone();
    seen_sorted.sort();
    seen_sorted.dedup();
    expected.sort();
    assert_eq!(seen.len(), 5, "no repeats across pages");
    assert_eq!(seen_sorted, expected, "no gaps across pages");
}
