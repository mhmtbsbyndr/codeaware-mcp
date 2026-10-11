use codeaware_mcp::hooks::compact::{handle_post_compact, handle_pre_compact};
use codeaware_mcp::hooks::post_tool_use::handle_post_tool_use;
use codeaware_mcp::hooks::session_stop::handle_session_stop;
use codeaware_mcp::hooks::subagent_stop::handle_subagent_stop;
use codeaware_mcp::hooks::tool_failure::handle_tool_failure;

#[test]
fn test_post_tool_use_metrics() {
    let input = serde_json::json!({
        "event": "PostToolUse",
        "tool_name": "codeaware__smart_read",
        "tool_input": {"path": "src/main.rs", "mode": "skeleton"},
        "tool_output_size": 847,
        "session_id": "abc123",
        "timestamp": "2026-04-01T12:34:56Z"
    });
    let result = handle_post_tool_use(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
    assert!(parsed["metrics_logged"].as_bool().unwrap());
}

#[test]
fn test_tool_failure_recovery_hint() {
    let input = serde_json::json!({
        "event": "PostToolUseFailure",
        "tool_name": "codeaware__smart_edit",
        "tool_input": {"path": "src/main.rs", "strategy": "text"},
        "error": "E_AMBIGUOUS_MATCH",
        "session_id": "abc123"
    });
    let result = handle_tool_failure(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("recovery_hint").is_some());
}

#[test]
fn test_pre_compact_produces_summary() {
    let input = serde_json::json!({
        "event": "PreCompact",
        "session_id": "abc123"
    });
    let result = handle_pre_compact(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
}

#[test]
fn test_post_compact_marks_stale() {
    let input = serde_json::json!({
        "event": "PostCompact",
        "session_id": "abc123"
    });
    let result = handle_post_compact(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
}

#[test]
fn test_subagent_stop_normalizes() {
    let input = serde_json::json!({
        "event": "SubagentStop",
        "agent_name": "code-analyzer",
        "result_size": 2500,
        "session_id": "abc123"
    });
    let result = handle_subagent_stop(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["decision"], "approve");
}

#[test]
fn test_session_stop_bilanz() {
    let input = serde_json::json!({
        "event": "Stop",
        "session_id": "abc123"
    });
    let result = handle_session_stop(&input.to_string()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("reason").is_some());
}

// ── Spool, context cache, bounded transcript reader, UTF-8 stdin ──────────

use codeaware_mcp::hooks::post_tool_use::is_file_edit_tool;
use codeaware_mcp::hooks::spool;
use codeaware_mcp::hooks::stdin_utf8::{read_hook_input, Utf8ChunkDecoder};
use codeaware_mcp::hooks::tail_reader::open_backward;
use std::io::{Read, Write};

#[test]
fn test_spool_event_written_before_drain_survives_restart() {
    // Simulates a crash: the event is enqueued, then the process "dies"
    // before draining. A later process must still see and apply it.
    let dir = tempfile::tempdir().unwrap();
    let path = spool::enqueue_in(dir.path(), "session_stop", &serde_json::json!({"session_id": "s9"})).unwrap();
    assert!(path.exists());
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("json"));

    let mut applied = Vec::new();
    let report = spool::drain_in(dir.path(), |kind, payload| {
        applied.push(format!("{kind}:{}", payload["session_id"].as_str().unwrap()));
        Ok(())
    });
    assert_eq!(report.processed, 1);
    assert_eq!(applied, vec!["session_stop:s9"]);
    assert!(!path.exists());
}

#[test]
fn test_spool_unknown_kind_is_not_written() {
    let dir = tempfile::tempdir().unwrap();
    assert!(spool::enqueue_in(dir.path(), "../../etc", &serde_json::json!({})).is_err());
    let entries: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert!(entries.is_empty());
}

#[test]
fn test_file_edit_classification() {
    assert!(is_file_edit_tool("codeaware__smart_edit"));
    assert!(is_file_edit_tool("Edit"));
    assert!(is_file_edit_tool("Write"));
    assert!(!is_file_edit_tool("codeaware__smart_read"));
    assert!(!is_file_edit_tool("Bash"));
}

#[test]
fn test_tail_reader_on_file_reads_newest_first_and_bounds_memory() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let line = "{\"type\":\"assistant\",\"text\":\"Größe €😀\"}\n";
    for _ in 0..50_000 {
        file.write_all(line.as_bytes()).unwrap();
    }
    file.flush().unwrap();

    let mut lines = open_backward(file.path(), 64 * 1024, 4096).unwrap();
    let first = lines.next_line().unwrap().unwrap();
    assert_eq!(first, line.trim_end().as_bytes());
    let mut count = 1usize;
    while lines.next_line().unwrap().is_some() {
        count += 1;
    }
    assert_eq!(count, 50_000);
    assert_eq!(lines.skipped_oversize(), 0);
    assert!(lines.peak_buffer_bytes() <= 64 * 1024 + 4096);
}

#[test]
fn test_stdin_multibyte_split_across_reads_is_decoded_whole() {
    struct OneByte<'a>(&'a [u8]);
    impl Read for OneByte<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0.is_empty() || buf.is_empty() {
                return Ok(0);
            }
            buf[0] = self.0[0];
            self.0 = &self.0[1..];
            Ok(1)
        }
    }
    let payload = r#"{"tool_name":"smart_read","note":"Größe — 😀"}"#;
    let out = read_hook_input(OneByte(payload.as_bytes())).unwrap();
    assert_eq!(out, payload);
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["note"], "Größe — 😀");
}

#[test]
fn test_utf8_decoder_splits_two_byte_char_between_chunks() {
    let bytes = "ä".as_bytes(); // C3 A4
    let mut dec = Utf8ChunkDecoder::new();
    let mut out = dec.push(&bytes[..1]);
    assert_eq!(out, "");
    out.push_str(&dec.push(&bytes[1..]));
    out.push_str(&dec.finish());
    assert_eq!(out, "ä");
}
