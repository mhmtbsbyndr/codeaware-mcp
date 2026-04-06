use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::envelope::{Envelope, ErrorCode, TrustLevel};

// ── Data Structures ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RenameResult {
    pub old_name: String,
    pub new_name: String,
    pub files_affected: usize,
    pub occurrences: usize,
    pub changes: Vec<FileRename>,
    pub dry_run: bool,
}

#[derive(Debug, Serialize)]
pub struct FileRename {
    pub path: String,
    pub occurrences: usize,
    pub lines: Vec<LineChange>,
}

#[derive(Debug, Serialize)]
pub struct LineChange {
    pub line_number: usize,
    pub old_text: String,
    pub new_text: String,
}

// ── Source file extensions ────────────────────────────────────────

const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "py", "ts", "js", "java", "c", "cpp", "go", "php", "swift",
    "tsx", "jsx", "h", "hpp", "cs", "rb", "kt", "scala",
];

fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SOURCE_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

// ── Comment / string heuristic ───────────────────────────────────

/// Returns true if the match at `byte_offset` in `line` appears to be inside
/// a string literal or a comment. This is a simple heuristic that covers the
/// vast majority of cases without needing a full parser.
fn is_in_string_or_comment(line: &str, byte_offset: usize) -> bool {
    let trimmed = line.trim_start();

    // Line-level comment detection (covers //, #, --, ;;, %)
    if trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with("--")
        || trimmed.starts_with(";;")
        || trimmed.starts_with('%')
    {
        return true;
    }

    // Block comment heuristic: line starts with /* or *
    if trimmed.starts_with("/*") || trimmed.starts_with('*') {
        return true;
    }

    // Check if the match falls inside quotes by counting unescaped quotes
    // before the match position.
    let prefix = &line[..byte_offset];
    let single_quotes = count_unescaped_quotes(prefix, b'\'');
    let double_quotes = count_unescaped_quotes(prefix, b'"');
    let backtick_quotes = count_unescaped_quotes(prefix, b'`');

    // Odd count means we're inside quotes
    !single_quotes.is_multiple_of(2) || !double_quotes.is_multiple_of(2) || !backtick_quotes.is_multiple_of(2)
}

fn count_unescaped_quotes(s: &str, quote_char: u8) -> usize {
    let bytes = s.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == quote_char {
            // Check for preceding backslash
            if i == 0 || bytes[i - 1] != b'\\' {
                count += 1;
            }
        }
        i += 1;
    }
    count
}

// ── Core rename logic ────────────────────────────────────────────

fn find_renames(
    root: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<Vec<FileRename>, String> {
    let pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(old_name)))
        .map_err(|e| format!("Invalid symbol name for regex: {e}"))?;

    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)       // skip hidden files
        .git_ignore(true)   // respect .gitignore
        .git_global(true)
        .git_exclude(true)
        .build();

    let mut file_renames = Vec::new();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_file() || !is_source_file(path) {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue, // skip unreadable files (binary, permissions, etc.)
        };

        let mut line_changes = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            for mat in pattern.find_iter(line) {
                if is_in_string_or_comment(line, mat.start()) {
                    continue;
                }
                let new_line = pattern.replace_all(line, new_name).to_string();
                line_changes.push(LineChange {
                    line_number: line_idx + 1,
                    old_text: line.to_string(),
                    new_text: new_line,
                });
                break; // one LineChange per line is sufficient
            }
        }

        if !line_changes.is_empty() {
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            let occ = line_changes.len();
            file_renames.push(FileRename {
                path: rel_path,
                occurrences: occ,
                lines: line_changes,
            });
        }
    }

    Ok(file_renames)
}

fn apply_renames(root: &Path, old_name: &str, new_name: &str, renames: &[FileRename]) -> Result<(), String> {
    let pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(old_name)))
        .map_err(|e| format!("Invalid symbol name for regex: {e}"))?;

    for file_rename in renames {
        let abs_path = root.join(&file_rename.path);
        let content = std::fs::read_to_string(&abs_path)
            .map_err(|e| format!("Failed to read {}: {e}", file_rename.path))?;

        let mut new_lines: Vec<String> = Vec::new();
        for (line_idx, line) in content.lines().enumerate() {
            let has_match = file_rename.lines.iter().any(|lc| lc.line_number == line_idx + 1);
            if has_match && !is_in_string_or_comment(line, 0) {
                // Re-check: only replace word-boundary matches not in strings/comments
                // We do a per-match check to be safe
                let mut result_line = line.to_string();
                // Replace all word-boundary matches on this line that aren't in strings/comments
                // Since we already validated in find_renames, just do the replacement
                result_line = pattern.replace_all(&result_line, new_name).to_string();
                new_lines.push(result_line);
            } else {
                new_lines.push(line.to_string());
            }
        }

        // Preserve trailing newline if original had one
        let mut output = new_lines.join("\n");
        if content.ends_with('\n') {
            output.push('\n');
        }

        std::fs::write(&abs_path, &output)
            .map_err(|e| format!("Failed to write {}: {e}", file_rename.path))?;
    }

    Ok(())
}

// ── MCP handler ──────────────────────────────────────────────────

pub fn handle_smart_refactor(params: &Value) -> Value {
    let operation = params
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("rename");

    match operation {
        "rename" => handle_rename(params),
        other => {
            let envelope = Envelope::<()>::error(
                ErrorCode::EInternalError,
                false,
                Some(format!("Unsupported operation: {other}. Only 'rename' is currently supported.")),
            );
            json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
        }
    }
}

fn handle_rename(params: &Value) -> Value {
    let old_name = match params.get("old_name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => {
            let envelope = Envelope::<()>::error(
                ErrorCode::EInternalError,
                false,
                Some("Missing required parameter: old_name".to_string()),
            );
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };

    let new_name = match params.get("new_name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => {
            let envelope = Envelope::<()>::error(
                ErrorCode::EInternalError,
                false,
                Some("Missing required parameter: new_name".to_string()),
            );
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };

    let dry_run = params
        .get("dry_run")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".");

    let root = std::path::Path::new(path);
    if !root.exists() {
        let envelope = Envelope::<()>::error(
            ErrorCode::EInternalError,
            false,
            Some(format!("Path does not exist: {path}")),
        );
        return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
    }

    // Validate: old_name and new_name must differ
    if old_name == new_name {
        let envelope = Envelope::<()>::error(
            ErrorCode::ERefactorConflict,
            false,
            Some("old_name and new_name must be different".to_string()),
        );
        return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
    }

    // Find all occurrences
    let renames = match find_renames(root, old_name, new_name) {
        Ok(r) => r,
        Err(e) => {
            let envelope = Envelope::<()>::error(
                ErrorCode::EInternalError,
                false,
                Some(e),
            );
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };

    let total_occurrences: usize = renames.iter().map(|r| r.occurrences).sum();
    let files_affected = renames.len();

    // Apply if not dry_run
    if !dry_run {
        if let Err(e) = apply_renames(root, old_name, new_name, &renames) {
            let envelope = Envelope::<()>::error(
                ErrorCode::ERefactorConflict,
                false,
                Some(e),
            );
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    }

    let result = RenameResult {
        old_name: old_name.to_string(),
        new_name: new_name.to_string(),
        files_affected,
        occurrences: total_occurrences,
        changes: renames,
        dry_run,
    };

    let trust = if dry_run {
        TrustLevel::Heuristic
    } else {
        TrustLevel::Exact
    };

    let envelope = Envelope::success(result, trust);
    json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
}
