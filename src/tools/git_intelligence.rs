use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

use crate::envelope::{Envelope, ErrorCode, TrustLevel};

// ── Data Structures ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FileChange {
    pub path: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Serialize)]
pub struct DiffResult {
    pub files_changed: usize,
    pub total_additions: u32,
    pub total_deletions: u32,
    pub files: Vec<FileChange>,
}

#[derive(Debug, Serialize)]
pub struct BlameLine {
    pub author: String,
    pub date: String,
    pub line_number: u32,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct BlameResult {
    pub file: String,
    pub lines: Vec<BlameLine>,
}

#[derive(Debug, Serialize)]
pub struct ChangelogEntry {
    pub category: String,
    pub message: String,
    pub hash: String,
    pub author: String,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ChangelogResult {
    pub entries: Vec<ChangelogEntry>,
    pub total_commits: usize,
}

// ── Helpers ──────────────────────────────────────────────────────

fn run_git(args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git error: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extract an optional working directory from the "cwd" param.
fn extract_cwd(params: &Value) -> Option<std::path::PathBuf> {
    params
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
}

fn categorize_commit(message: &str) -> String {
    let lower = message.to_lowercase();
    if lower.starts_with("feat") {
        "feature".to_string()
    } else if lower.starts_with("fix") {
        "bugfix".to_string()
    } else if lower.starts_with("refactor") {
        "refactor".to_string()
    } else if lower.starts_with("docs") {
        "docs".to_string()
    } else if lower.starts_with("chore") {
        "chore".to_string()
    } else if lower.starts_with("test") {
        "test".to_string()
    } else if lower.starts_with("perf") {
        "performance".to_string()
    } else if lower.starts_with("ci") {
        "ci".to_string()
    } else if lower.starts_with("style") {
        "style".to_string()
    } else {
        "other".to_string()
    }
}

// ── git_diff handler ─────────────────────────────────────────────

pub fn handle_git_diff(params: &Value) -> Value {
    let base = params
        .get("base")
        .and_then(|v| v.as_str())
        .unwrap_or("HEAD~1");
    let head = params
        .get("head")
        .and_then(|v| v.as_str())
        .unwrap_or("HEAD");
    let cwd = extract_cwd(params);

    let numstat = match run_git(&["diff", "--numstat", base, head], cwd.as_deref()) {
        Ok(output) => output,
        Err(e) => {
            let envelope =
                Envelope::<()>::error(ErrorCode::EGitError, false, Some(e));
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };

    let mut files = Vec::new();
    let mut total_additions: u32 = 0;
    let mut total_deletions: u32 = 0;

    for line in numstat.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            // Binary files show "-" for additions/deletions
            let additions = parts[0].parse::<u32>().unwrap_or(0);
            let deletions = parts[1].parse::<u32>().unwrap_or(0);
            let path = parts[2].to_string();

            let status = if additions > 0 && deletions > 0 {
                "modified"
            } else if additions > 0 {
                "added"
            } else if deletions > 0 {
                "deleted"
            } else {
                "binary"
            };

            total_additions += additions;
            total_deletions += deletions;

            files.push(FileChange {
                path,
                status: status.to_string(),
                additions,
                deletions,
            });
        }
    }

    let result = DiffResult {
        files_changed: files.len(),
        total_additions,
        total_deletions,
        files,
    };

    let envelope = Envelope::success(result, TrustLevel::Exact);
    json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
}

// ── git_blame handler ────────────────────────────────────────────

pub fn handle_git_blame(params: &Value) -> Value {
    let file = match params.get("file").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => {
            let envelope = Envelope::<()>::error(
                ErrorCode::EGitError,
                false,
                Some("Missing required parameter: file".to_string()),
            );
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };
    let cwd = extract_cwd(params);

    let start_line = params
        .get("start_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    let end_line = params
        .get("end_line")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let mut args = vec!["blame", "--porcelain"];

    let line_range;
    if let Some(start) = start_line {
        let end = end_line.unwrap_or(start);
        line_range = format!("{start},{end}");
        args.push("-L");
        args.push(&line_range);
    }

    args.push(file);

    let output = match run_git(&args, cwd.as_deref()) {
        Ok(o) => o,
        Err(e) => {
            let envelope =
                Envelope::<()>::error(ErrorCode::EGitError, false, Some(e));
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };

    let lines = parse_blame_porcelain(&output);

    let result = BlameResult {
        file: file.to_string(),
        lines,
    };

    let envelope = Envelope::success(result, TrustLevel::Exact);
    json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
}

fn parse_blame_porcelain(output: &str) -> Vec<BlameLine> {
    let mut lines = Vec::new();
    let mut current_author = String::new();
    let mut current_date = String::new();
    let mut current_line_number: u32 = 0;

    for line in output.lines() {
        if line.starts_with("author ") {
            current_author = line.strip_prefix("author ").unwrap_or("").to_string();
        } else if line.starts_with("author-time ") {
            let timestamp = line
                .strip_prefix("author-time ")
                .unwrap_or("")
                .trim();
            // Convert Unix timestamp to ISO date
            if let Ok(ts) = timestamp.parse::<i64>() {
                current_date = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| timestamp.to_string());
            } else {
                current_date = timestamp.to_string();
            }
        } else if line.len() >= 40
            && line.chars().take(40).all(|c| c.is_ascii_hexdigit())
        {
            // This is a commit hash line: <hash> <orig_line> <final_line> [<num_lines>]
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                current_line_number = parts[2].parse::<u32>().unwrap_or(0);
            }
        } else if let Some(content) = line.strip_prefix('\t') {
            // Content line starts with a tab
            lines.push(BlameLine {
                author: current_author.clone(),
                date: current_date.clone(),
                line_number: current_line_number,
                content: content.to_string(),
            });
        }
    }

    lines
}

// ── git_changelog handler ────────────────────────────────────────

pub fn handle_git_changelog(params: &Value) -> Value {
    let base = params.get("base").and_then(|v| v.as_str());
    let head = params
        .get("head")
        .and_then(|v| v.as_str())
        .unwrap_or("HEAD");
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;
    let cwd = extract_cwd(params);

    let range = match base {
        Some(b) => format!("{b}..{head}"),
        None => head.to_string(),
    };

    let limit_str = format!("-{limit}");

    let log_args = if base.is_some() {
        vec![
            "log",
            "--pretty=format:COMMIT:%H|%an|%s",
            "--name-only",
            &limit_str,
            &range,
        ]
    } else {
        vec![
            "log",
            "--pretty=format:COMMIT:%H|%an|%s",
            "--name-only",
            &limit_str,
        ]
    };

    let output = match run_git(&log_args, cwd.as_deref()) {
        Ok(o) => o,
        Err(e) => {
            let envelope =
                Envelope::<()>::error(ErrorCode::EGitError, false, Some(e));
            return json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]});
        }
    };

    let entries = parse_changelog_output(&output);
    let total_commits = entries.len();

    let result = ChangelogResult {
        entries,
        total_commits,
    };

    let envelope = Envelope::success(result, TrustLevel::Exact);
    json!({"content": [{"type": "text", "text": serde_json::to_string(&envelope).unwrap_or_default()}]})
}

fn parse_changelog_output(output: &str) -> Vec<ChangelogEntry> {
    let mut entries = Vec::new();
    let mut current_entry: Option<ChangelogEntry> = None;

    for line in output.lines() {
        if let Some(commit_line) = line.strip_prefix("COMMIT:") {
            // Save previous entry if exists
            if let Some(entry) = current_entry.take() {
                entries.push(entry);
            }

            let parts: Vec<&str> = commit_line.splitn(3, '|').collect();
            if parts.len() == 3 {
                let hash = parts[0].to_string();
                let author = parts[1].to_string();
                let message = parts[2].to_string();
                let category = categorize_commit(&message);

                current_entry = Some(ChangelogEntry {
                    category,
                    message,
                    hash,
                    author,
                    files: Vec::new(),
                });
            }
        } else if !line.trim().is_empty() {
            // This is a filename
            if let Some(ref mut entry) = current_entry {
                entry.files.push(line.trim().to_string());
            }
        }
    }

    // Don't forget the last entry
    if let Some(entry) = current_entry {
        entries.push(entry);
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_commit() {
        assert_eq!(categorize_commit("feat: add git tools"), "feature");
        assert_eq!(categorize_commit("fix: resolve crash"), "bugfix");
        assert_eq!(categorize_commit("refactor: clean up"), "refactor");
        assert_eq!(categorize_commit("docs: update readme"), "docs");
        assert_eq!(categorize_commit("chore: bump version"), "chore");
        assert_eq!(categorize_commit("test: add tests"), "test");
        assert_eq!(categorize_commit("random commit"), "other");
    }

    #[test]
    fn test_parse_blame_porcelain() {
        let sample = "\
abcdef1234567890abcdef1234567890abcdef12 1 1 1\nauthor John Doe\nauthor-mail <john@example.com>\nauthor-time 1700000000\nauthor-tz +0000\ncommitter John Doe\ncommitter-mail <john@example.com>\ncommitter-time 1700000000\ncommitter-tz +0000\nsummary Initial commit\nfilename test.rs\n\tlet x = 42;\n";

        let lines = parse_blame_porcelain(sample);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].author, "John Doe");
        assert_eq!(lines[0].line_number, 1);
        assert_eq!(lines[0].content, "let x = 42;");
        assert_eq!(lines[0].date, "2023-11-14");
    }

    #[test]
    fn test_parse_changelog_output() {
        let sample = "\
COMMIT:abc123|Alice|feat: add new feature\nfile1.rs\nfile2.rs\n\nCOMMIT:def456|Bob|fix: resolve bug\nfile3.rs\n";

        let entries = parse_changelog_output(sample);
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].hash, "abc123");
        assert_eq!(entries[0].author, "Alice");
        assert_eq!(entries[0].category, "feature");
        assert_eq!(entries[0].files, vec!["file1.rs", "file2.rs"]);

        assert_eq!(entries[1].hash, "def456");
        assert_eq!(entries[1].author, "Bob");
        assert_eq!(entries[1].category, "bugfix");
        assert_eq!(entries[1].files, vec!["file3.rs"]);
    }
}
