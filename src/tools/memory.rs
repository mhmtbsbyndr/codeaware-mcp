use serde::Serialize;
use serde_json::{json, Value};
use crate::envelope::{Envelope, ErrorCode, TrustLevel};
use crate::session::persistence::{
    ObservationFields, SaveObservationOpts, SearchObservationsOpts, SessionDb,
};

const VALID_TYPES: &[&str] = &[
    "bugfix", "feature", "refactor", "change", "discovery", "decision",
];

const VALID_CONCEPTS: &[&str] = &[
    "how-it-works", "why-it-exists", "what-changed",
    "problem-solution", "gotcha", "pattern", "trade-off",
];

const MAX_TIMELINE_DEPTH: usize = 100;
const MAX_TEXT_SIZE: usize = 65536; // 64KB

/// Sanitize user input for FTS5 queries.
/// Strips colons (column filters) and double quotes (no escape in FTS5 phrases).
/// Wraps each remaining term in double quotes as a literal phrase.
/// Returns None if input produces no searchable terms.
fn sanitize_fts5_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .filter_map(|term| {
            // Strip colons (column filter syntax) and quotes (no FTS5 escape)
            let clean: String = term.chars()
                .filter(|c| *c != ':' && *c != '"')
                .collect();
            if clean.is_empty() {
                None
            } else {
                Some(format!("\"{clean}\""))
            }
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
    if text.len() > MAX_TEXT_SIZE {
        let env = Envelope::<()>::error(
            ErrorCode::EParseFailed,
            false,
            Some(format!("'text' exceeds maximum size of {} bytes", MAX_TEXT_SIZE)),
        );
        return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
    }

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
    // Validate date format (must start with YYYY- for lexicographic comparison)
    for (name, val) in [("date_start", date_start), ("date_end", date_end)] {
        if let Some(d) = val {
            if d.len() < 10 || d.as_bytes().get(4) != Some(&b'-') || d.as_bytes().get(7) != Some(&b'-') {
                let env = Envelope::<()>::error(
                    ErrorCode::EParseFailed,
                    false,
                    Some(format!("'{}' must be ISO 8601 format (YYYY-MM-DD...)", name)),
                );
                return serde_json::to_value(env).unwrap_or(json!({"ok": false}));
            }
        }
    }

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

#[derive(Debug, Serialize)]
struct UpdateMemoryResult {
    id: i64,
    message: String,
}

fn error_value(code: ErrorCode, retryable: bool, message: impl Into<String>) -> Value {
    let env = Envelope::<()>::error(code, retryable, Some(message.into()));
    serde_json::to_value(env).unwrap_or(json!({"ok": false}))
}

/// PATCH rule for a nullable text field: absent keeps the stored value,
/// explicit null clears it, a string replaces it.
fn patch_scalar(params: &Value, key: &str, current: Option<String>) -> Result<Option<String>, String> {
    match params.get(key) {
        None => Ok(current),
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("'{key}' must be a string or null")),
    }
}

/// PATCH rule for a list field, stored comma-joined like save_memory does.
/// An empty list counts as cleared.
fn patch_list(params: &Value, key: &str, current: Option<String>) -> Result<Option<String>, String> {
    match params.get(key) {
        None => Ok(current),
        Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                match item.as_str() {
                    Some(s) => parts.push(s),
                    None => return Err(format!("'{key}' must contain only strings")),
                }
            }
            let joined = parts.join(",");
            Ok(if joined.is_empty() { None } else { Some(joined) })
        }
        Some(_) => Err(format!("'{key}' must be an array of strings or null")),
    }
}

/// Partial update of an observation with PATCH semantics:
/// - omitted fields keep their stored value
/// - explicit null clears a nullable field (title, project, facts, files, concepts)
/// - `text` and `type` are required columns, so null is rejected
/// - the result must keep at least one searchable term in title, text, facts or concepts
pub fn handle_update_memory(params: &Value, db: &SessionDb) -> Value {
    let id = match params.get("id").and_then(|v| v.as_i64()) {
        Some(id) => id,
        None => return error_value(ErrorCode::EParseFailed, false, "'id' is required (integer)"),
    };
    let current = match db.get_observation_by_id(id) {
        Ok(Some(rec)) => rec,
        Ok(None) => {
            return error_value(ErrorCode::EParseFailed, false, format!("No observation with id {id}"))
        }
        Err(e) => {
            return error_value(ErrorCode::EInternalError, true, format!("Failed to load observation: {e}"))
        }
    };

    let text: String = match params.get("text") {
        None => current.text.clone(),
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::String(_)) => {
            return error_value(ErrorCode::EParseFailed, false, "'text' must be non-empty")
        }
        Some(Value::Null) => {
            return error_value(ErrorCode::EParseFailed, false, "'text' cannot be cleared")
        }
        Some(_) => return error_value(ErrorCode::EParseFailed, false, "'text' must be a string"),
    };
    if text.len() > MAX_TEXT_SIZE {
        return error_value(
            ErrorCode::EParseFailed,
            false,
            format!("'text' exceeds maximum size of {} bytes", MAX_TEXT_SIZE),
        );
    }

    let obs_type: String = match params.get("type") {
        None => current.observation_type.clone(),
        Some(Value::String(t)) if VALID_TYPES.contains(&t.as_str()) => t.clone(),
        Some(Value::String(t)) => {
            return error_value(
                ErrorCode::EParseFailed,
                false,
                format!("Invalid type '{}'. Valid: {:?}", t, VALID_TYPES),
            )
        }
        Some(_) => return error_value(ErrorCode::EParseFailed, false, "'type' must be a string"),
    };

    let title = match patch_scalar(params, "title", current.title.clone()) {
        Ok(v) => v,
        Err(m) => return error_value(ErrorCode::EParseFailed, false, m),
    };
    let project = match patch_scalar(params, "project", current.project.clone()) {
        Ok(v) => v,
        Err(m) => return error_value(ErrorCode::EParseFailed, false, m),
    };
    let facts = match patch_list(params, "facts", current.facts.clone()) {
        Ok(v) => v,
        Err(m) => return error_value(ErrorCode::EParseFailed, false, m),
    };
    let files = match patch_list(params, "files", current.files.clone()) {
        Ok(v) => v,
        Err(m) => return error_value(ErrorCode::EParseFailed, false, m),
    };
    let concepts = match patch_list(params, "concepts", current.concepts.clone()) {
        Ok(v) => v,
        Err(m) => return error_value(ErrorCode::EParseFailed, false, m),
    };
    if let Some(ref c) = concepts {
        for concept in c.split(',') {
            if !VALID_CONCEPTS.contains(&concept) {
                return error_value(
                    ErrorCode::EParseFailed,
                    false,
                    format!("Invalid concept '{}'. Valid: {:?}", concept, VALID_CONCEPTS),
                );
            }
        }
    }

    // The type is an enum value, not user text, so it does not count as searchable content.
    let has_searchable_text = [title.as_deref(), Some(text.as_str()), facts.as_deref(), concepts.as_deref()]
        .into_iter()
        .flatten()
        .any(|s| sanitize_fts5_query(s).is_some());
    if !has_searchable_text {
        return error_value(
            ErrorCode::EParseFailed,
            false,
            "Update would leave no searchable text in title, text, facts or concepts",
        );
    }

    let fields = ObservationFields {
        title: title.as_deref(),
        text: &text,
        observation_type: &obs_type,
        concepts: concepts.as_deref(),
        project: project.as_deref(),
        files: files.as_deref(),
        facts: facts.as_deref(),
    };
    match db.update_observation_fields(id, &fields) {
        Ok(true) => {
            let result = UpdateMemoryResult {
                id,
                message: format!("Observation updated (id: {})", id),
            };
            let env = Envelope::success(result, TrustLevel::Exact);
            serde_json::to_value(env).unwrap_or(json!({"ok": false}))
        }
        Ok(false) => error_value(ErrorCode::EParseFailed, false, format!("No observation with id {id}")),
        Err(e) => error_value(ErrorCode::EInternalError, true, format!("Failed to update: {e}")),
    }
}
