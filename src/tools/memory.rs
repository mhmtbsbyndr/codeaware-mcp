use serde::Serialize;
use serde_json::{json, Value};
use crate::envelope::{Envelope, ErrorCode, TrustLevel};
use crate::session::persistence::{SessionDb, SaveObservationOpts, SearchObservationsOpts};

const VALID_TYPES: &[&str] = &[
    "bugfix", "feature", "refactor", "change", "discovery", "decision",
];

const VALID_CONCEPTS: &[&str] = &[
    "how-it-works", "why-it-exists", "what-changed",
    "problem-solution", "gotcha", "pattern", "trade-off",
];

const MAX_TIMELINE_DEPTH: usize = 100;

/// Sanitize user input for FTS5 queries.
/// Strips colons (column filter syntax), escapes quotes, wraps each term.
/// Returns None if input produces no searchable terms.
fn sanitize_fts5_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|term| {
            // Strip colons to prevent column:value filter syntax
            let clean = term.replace(':', " ").replace('"', "\"\"");
            format!("\"{clean}\"")
        })
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

#[derive(Debug, Serialize)]
struct SaveMemoryResult {
    id: i64,
    message: String,
}

#[derive(Debug, Serialize)]
struct SearchMemoryResult {
    count: usize,
    observations: Vec<crate::session::persistence::ObservationRecord>,
}

#[derive(Debug, Serialize)]
struct TimelineResult {
    anchor_id: i64,
    total: usize,
    observations: Vec<crate::session::persistence::ObservationRecord>,
}

pub fn handle_save_memory(params: &Value, db: &SessionDb) -> Value {
    let text = match params.get("text").and_then(|v| v.as_str()) {
        Some(t) if !t.is_empty() => t,
        _ => {
            let env = Envelope::<()>::error(
                ErrorCode::EParseFailed,
                false,
                Some("'text' is required and must be non-empty".to_string()),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    };

    let title = params.get("title").and_then(|v| v.as_str());

    let obs_type = params
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("discovery");
    if !VALID_TYPES.contains(&obs_type) {
        let env = Envelope::<()>::error(
            ErrorCode::EParseFailed,
            false,
            Some(format!("Invalid type '{}'. Valid: {:?}", obs_type, VALID_TYPES)),
        );
        return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
    }

    let concepts = params
        .get("concepts")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|s| !s.is_empty());
    if let Some(ref c) = concepts {
        for concept in c.split(',') {
            if !concept.is_empty() && !VALID_CONCEPTS.contains(&concept) {
                let env = Envelope::<()>::error(
                    ErrorCode::EParseFailed,
                    false,
                    Some(format!("Invalid concept '{}'. Valid: {:?}", concept, VALID_CONCEPTS)),
                );
                return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
            }
        }
    }

    let project = params.get("project").and_then(|v| v.as_str());
    let files = params
        .get("files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|s| !s.is_empty());
    let facts = params
        .get("facts")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|s| !s.is_empty());

    let save_opts = SaveObservationOpts {
        title,
        text,
        observation_type: obs_type,
        concepts: concepts.as_deref(),
        project,
        files: files.as_deref(),
        facts: facts.as_deref(),
    };
    match db.save_observation(&save_opts) {
        Ok(id) => {
            let result = SaveMemoryResult {
                id,
                message: format!("Observation saved (id: {})", id),
            };
            let env = Envelope::success(result, TrustLevel::Exact);
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
        Err(e) => {
            let env = Envelope::<()>::error(
                ErrorCode::EInternalError,
                true,
                Some(format!("Failed to save: {}", e)),
            );
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
    }
}

pub fn handle_search_memory(params: &Value, db: &SessionDb) -> Value {
    let query = match params.get("query").and_then(|v| v.as_str()) {
        Some(q) if !q.is_empty() => q,
        _ => {
            let env = Envelope::<()>::error(
                ErrorCode::EParseFailed,
                false,
                Some("'query' is required and must be non-empty".to_string()),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    };

    let project = params.get("project").and_then(|v| v.as_str());
    let obs_type = params.get("type").and_then(|v| v.as_str());
    if let Some(t) = obs_type {
        if !VALID_TYPES.contains(&t) {
            let env = Envelope::<()>::error(
                ErrorCode::EParseFailed,
                false,
                Some(format!("Invalid type filter '{}'. Valid: {:?}", t, VALID_TYPES)),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    }
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .min(50) as usize;
    let offset = params
        .get("offset")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let date_start = params.get("date_start").and_then(|v| v.as_str());
    let date_end = params.get("date_end").and_then(|v| v.as_str());

    let sanitized_query = match sanitize_fts5_query(query) {
        Some(q) => q,
        None => {
            let env = Envelope::<()>::error(
                ErrorCode::EParseFailed,
                false,
                Some("Query contains no searchable terms".to_string()),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    };
    let opts = SearchObservationsOpts {
        query: &sanitized_query,
        project,
        observation_type: obs_type,
        limit,
        offset,
        date_start,
        date_end,
    };
    match db.search_observations(&opts) {
        Ok(observations) => {
            let result = SearchMemoryResult {
                count: observations.len(),
                observations,
            };
            let env = Envelope::success(result, TrustLevel::Heuristic);
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
        Err(e) => {
            let is_fts_error = e.to_string().contains("fts5");
            let env = Envelope::<()>::error(
                if is_fts_error { ErrorCode::EParseFailed } else { ErrorCode::EInternalError },
                !is_fts_error,
                Some(if is_fts_error {
                    "Search query failed. Try simpler search terms.".to_string()
                } else {
                    "Database error during search".to_string()
                }),
            );
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
    }
}

pub fn handle_memory_timeline(params: &Value, db: &SessionDb) -> Value {
    let anchor_id = match params.get("anchor_id").and_then(|v| v.as_i64()) {
        Some(id) => id,
        None => {
            let env = Envelope::<()>::error(
                ErrorCode::EParseFailed,
                false,
                Some("'anchor_id' is required (integer)".to_string()),
            );
            return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
        }
    };

    let depth_before = (params
        .get("depth_before")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize)
        .min(MAX_TIMELINE_DEPTH);
    let depth_after = (params
        .get("depth_after")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize)
        .min(MAX_TIMELINE_DEPTH);
    let project = params.get("project").and_then(|v| v.as_str());

    match db.get_observation_timeline(anchor_id, depth_before, depth_after, project) {
        Ok(observations) => {
            let result = TimelineResult {
                anchor_id,
                total: observations.len(),
                observations,
            };
            let env = Envelope::success(result, TrustLevel::Exact);
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
        Err(e) => {
            let env = Envelope::<()>::error(
                ErrorCode::EInternalError,
                false,
                Some(format!("Timeline failed: {}. Use search_memory to find valid IDs.", e)),
            );
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
    }
}
