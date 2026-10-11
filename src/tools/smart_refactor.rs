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

/// Byte ranges of string literals in a line, plus the byte offset where a
/// trailing `//` line comment starts (if any). Heuristic, not a parser.
struct LineMask {
    strings: Vec<(usize, usize)>,
    comment_start: Option<usize>,
}

impl LineMask {
    fn new(line: &str) -> Self {
        let bytes = line.as_bytes();
        let mut strings = Vec::new();
        let mut comment_start = None;
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'/' if bytes.get(i + 1) == Some(&b'/') => {
                    comment_start = Some(i);
                    break;
                }
                q @ (b'"' | b'\'' | b'`') => {
                    // Find the closing quote, honoring backslash escapes.
                    let mut j = i + 1;
                    let mut closed = None;
                    while j < bytes.len() {
                        if bytes[j] == b'\\' {
                            j += 2;
                            continue;
                        }
                        if bytes[j] == q {
                            closed = Some(j);
                            break;
                        }
                        j += 1;
                    }
                    match closed {
                        Some(end) => {
                            strings.push((i, end + 1));
                            i = end + 1;
                        }
                        // Unclosed `"` or backtick: treat the rest of the line as string.
                        None if q != b'\'' => {
                            strings.push((i, bytes.len()));
                            break;
                        }
                        // Unclosed `'` is a lifetime or similar, not a string.
                        None => i += 1,
                    }
                }
                _ => i += 1,
            }
        }
        LineMask { strings, comment_start }
    }

    fn contains(&self, offset: usize) -> bool {
        self.comment_start.is_some_and(|c| offset >= c)
            || self.strings.iter().any(|&(s, e)| offset >= s && offset < e)
    }
}

/// Returns true if the match at `byte_offset` in `line` appears to be inside
/// a string literal or a comment.
fn is_in_string_or_comment(line: &str, byte_offset: usize) -> bool {
    let trimmed = line.trim_start();

    // Whole-line comment detection (covers //, #, --, ;;, %)
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

    LineMask::new(line).contains(byte_offset)
}

/// Rename every whole-word match on `line` that is not inside a string literal
/// or comment. Returns the new line and the number of replaced matches, or
/// None if nothing on the line was replaced.
fn rename_line(line: &str, pattern: &regex::Regex, new_name: &str) -> Option<(String, usize)> {
    let mut out = String::with_capacity(line.len());
    let mut last = 0;
    let mut count = 0;
    for mat in pattern.find_iter(line) {
        if is_in_string_or_comment(line, mat.start()) {
            continue;
        }
        out.push_str(&line[last..mat.start()]);
        out.push_str(new_name);
        last = mat.end();
        count += 1;
    }
    if count == 0 {
        return None;
    }
    out.push_str(&line[last..]);
    Some((out, count))
}

// ── Core rename logic ────────────────────────────────────────────

/// Find the lines that `rename` would change, across all source files under `root`.
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
        let mut occurrences = 0;

        for (line_idx, line) in content.lines().enumerate() {
            if let Some((new_line, count)) = rename_line(line, &pattern, new_name) {
                occurrences += count;
                line_changes.push(LineChange {
                    line_number: line_idx + 1,
                    old_text: line.to_string(),
                    new_text: new_line,
                });
            }
        }

        if !line_changes.is_empty() {
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();
            file_renames.push(FileRename {
                path: rel_path,
                occurrences,
                lines: line_changes,
            });
        }
    }

    Ok(file_renames)
}

/// Write the renames to disk. Line terminators (LF or CRLF) and the final
/// newline are kept byte-for-byte; only the replaced identifiers change.
fn apply_renames(root: &Path, old_name: &str, new_name: &str, renames: &[FileRename]) -> Result<(), String> {
    let pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(old_name)))
        .map_err(|e| format!("Invalid symbol name for regex: {e}"))?;

    for file_rename in renames {
        let abs_path = root.join(&file_rename.path);
        let content = std::fs::read_to_string(&abs_path)
            .map_err(|e| format!("Failed to read {}: {e}", file_rename.path))?;

        let mut output = String::with_capacity(content.len());
        for segment in content.split_inclusive('\n') {
            let body = segment.strip_suffix('\n').unwrap_or(segment);
            let (body, cr) = match body.strip_suffix('\r') {
                Some(b) => (b, "\r"),
                None => (body, ""),
            };
            let terminator = &segment[body.len() + cr.len()..];
            match rename_line(body, &pattern, new_name) {
                Some((new_body, _)) => output.push_str(&new_body),
                None => output.push_str(body),
            }
            output.push_str(cr);
            output.push_str(terminator);
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

/// Rename a symbol (whole-word matches only).
///
/// There is no `replace_all` flag: every whole-word occurrence is renamed,
/// except occurrences inside string literals or comments (line comments and
/// `//` trailing comments are recognized; `#`, `--`, `%` comments only when
/// they start the line). `dry_run` defaults to true and only reports changes.
/// `occurrences` counts matches; `lines` lists each changed line.
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
