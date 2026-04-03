use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
    #[error("Hook error: {0}")]
    Other(String),
}

pub fn handle_tool_failure(input: &str) -> Result<String, HookError> {
    let parsed: serde_json::Value = serde_json::from_str(input)?;
    let error = parsed["error"].as_str().unwrap_or("unknown");
    let tool_name = parsed["tool_name"].as_str().unwrap_or("unknown");

    let hint = match error {
        "E_AMBIGUOUS_MATCH" => "Try strategy=lines or strategy=symbol instead of strategy=text",
        "E_STALE_READ" => "File changed since last read. Call smart_read again.",
        "E_HASH_MISMATCH" => "Concurrent edit detected. Re-read the file first.",
        "E_LSP_TIMEOUT" => "LSP is slow. tree-sitter fallback will be used.",
        "E_PARSE_FAILED" => "tree-sitter parse failed. Regex fallback active.",
        _ => "Check the error code and retry with adjusted parameters.",
    };

    let suggest_retry = matches!(error, "E_STALE_READ" | "E_HASH_MISMATCH" | "E_LSP_TIMEOUT");

    Ok(serde_json::json!({
        "tool_name": tool_name,
        "recovery_hint": hint,
        "suggest_retry": suggest_retry
    })
    .to_string())
}
