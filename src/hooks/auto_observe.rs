use crate::session::persistence::{AutoObservationOpts, SessionDb};

/// Extract a compact observation from a tool call result and persist it.
pub fn record_auto_observation(
    db: &SessionDb,
    tool_name: &str,
    file_path: Option<&str>,
    session_id: &str,
    result_text: &str,
) {
    // Skip tools that don't produce useful observations
    if matches!(tool_name, "xray" | "session_status" | "validate_config") {
        return;
    }

    // Build dedup hash from tool_name + file_path
    let dedup_input = format!("{}:{}", tool_name, file_path.unwrap_or(""));
    let dedup_hash = blake3::hash(dedup_input.as_bytes()).to_hex().to_string();

    // Extract summary (first 200 chars of result, truncated at word boundary)
    let summary = truncate_at_word(result_text, 200);

    // Auto-determine observation type
    let obs_type = match tool_name {
        "smart_edit" => "change",
        "smart_run" => {
            if result_text.contains("fail") {
                "bugfix"
            } else {
                "discovery"
            }
        }
        _ => "discovery",
    };

    let title = format!("{} → {}", tool_name, file_path.unwrap_or("(no file)"));

    // Use INSERT OR IGNORE with dedup_hash unique constraint
    let _ = db.save_auto_observation(&AutoObservationOpts {
        title: &title,
        text: &summary,
        observation_type: obs_type,
        file_path,
        dedup_hash: &dedup_hash,
        source_tool: tool_name,
        session_id,
    });
}

fn truncate_at_word(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    match s[..max].rfind(' ') {
        Some(pos) => format!("{}…", &s[..pos]),
        None => format!("{}…", &s[..max]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_at_word_short() {
        assert_eq!(truncate_at_word("hello world", 200), "hello world");
    }

    #[test]
    fn test_truncate_at_word_long() {
        let input = "the quick brown fox jumps over the lazy dog";
        let result = truncate_at_word(input, 20);
        assert!(result.len() <= 22); // 20 + "…" (multi-byte)
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_at_word_no_space() {
        let input = "abcdefghijklmnopqrstuvwxyz";
        let result = truncate_at_word(input, 10);
        assert_eq!(result, "abcdefghij…");
    }
}
