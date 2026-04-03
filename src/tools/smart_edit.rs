use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

// ─── Public Types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPair {
    pub old: String,
    pub new: String,
}

/// Input to smart_edit.  `strategy` defaults to "text".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartEditInput {
    pub path: String,
    pub strategy: String,
    pub symbol: Option<String>,
    pub line_range: Option<String>,
    pub edits: Option<Vec<EditPair>>,
    pub new_content: Option<String>,
    pub dry_run: bool,
    pub expected_hash: Option<String>,
}

impl Default for SmartEditInput {
    fn default() -> Self {
        SmartEditInput {
            path: String::new(),
            strategy: "text".into(),
            symbol: None,
            line_range: None,
            edits: None,
            new_content: None,
            dry_run: false,
            expected_hash: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditApplied {
    pub lines: (usize, usize),
    pub summary: String,
    pub diff_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditImpact {
    pub callers_affected: usize,
    pub tests_affected: usize,
    pub test_file_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditEnforcement {
    pub tdd_warning: bool,
    pub uncommitted_edits_in_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartEditResult {
    pub path: String,
    pub applied: bool,
    pub dry_run: bool,
    pub strategy_used: String,
    pub new_file_hash: String,
    pub edits_applied: Vec<EditApplied>,
    pub syntax_check: Option<String>,
    pub impact: EditImpact,
    pub enforcement: EditEnforcement,
}

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SmartEditError {
    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("ambiguous match for '{text}': {occurrences} occurrence(s) at lines {line_numbers:?}")]
    AmbiguousMatch {
        text: String,
        occurrences: usize,
        line_numbers: Vec<usize>,
    },

    #[error("hash mismatch: expected {expected}, actual {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("invalid strategy: {0}")]
    InvalidStrategy(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn file_hash(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

/// Find all line numbers (1-based) where `needle` appears in `haystack`.
fn find_occurrences(haystack: &str, needle: &str) -> Vec<usize> {
    let mut result = Vec::new();
    for (idx, line) in haystack.lines().enumerate() {
        if line.contains(needle) {
            result.push(idx + 1);
        }
    }
    result
}

/// Count non-overlapping occurrences of `needle` in `haystack`.
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        count += 1;
        start += pos + needle.len();
    }
    count
}

/// Write `content` to `path` atomically (temp → fsync → rename).
fn atomic_write(path: &Path, content: &[u8]) -> Result<(), SmartEditError> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp_path = dir.join(format!(".smart_edit_{}.tmp", uuid::Uuid::new_v4()));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp_path)?;
        f.write_all(content)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Parse "start-end" or "start" line range (1-based).
fn parse_line_range(spec: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = spec.splitn(2, '-').collect();
    match parts.as_slice() {
        [s, e] => {
            let start = s.trim().parse::<usize>().ok()?;
            let end = e.trim().parse::<usize>().ok()?;
            Some((start, end))
        }
        [s] => {
            let n = s.trim().parse::<usize>().ok()?;
            Some((n, n))
        }
        _ => None,
    }
}

// ─── Core function ────────────────────────────────────────────────────────────

pub fn smart_edit(
    input: &SmartEditInput,
    project_root: &Path,
) -> Result<SmartEditResult, SmartEditError> {
    // Resolve absolute path
    let abs_path = if Path::new(&input.path).is_absolute() {
        std::path::PathBuf::from(&input.path)
    } else {
        project_root.join(&input.path)
    };

    if !abs_path.exists() {
        return Err(SmartEditError::FileNotFound(input.path.clone()));
    }

    let original_bytes = std::fs::read(&abs_path)?;
    let original_content = String::from_utf8_lossy(&original_bytes).into_owned();

    // Hash check
    if let Some(expected) = &input.expected_hash {
        let actual = file_hash(&original_bytes);
        if *expected != actual {
            return Err(SmartEditError::HashMismatch {
                expected: expected.clone(),
                actual,
            });
        }
    }

    let (final_content, edits_applied) = match input.strategy.as_str() {
        "text" => apply_text_strategy(&original_content, input)?,
        "lines" => apply_lines_strategy(&original_content, input)?,
        "symbol" => {
            return Err(SmartEditError::NotImplemented(
                "symbol strategy reserved for Phase 5".into(),
            ));
        }
        other => return Err(SmartEditError::InvalidStrategy(other.into())),
    };

    let new_bytes = final_content.as_bytes();
    let new_hash = file_hash(new_bytes);

    let applied = if input.dry_run {
        false
    } else {
        atomic_write(&abs_path, new_bytes)?;
        true
    };

    Ok(SmartEditResult {
        path: input.path.clone(),
        applied,
        dry_run: input.dry_run,
        strategy_used: input.strategy.clone(),
        new_file_hash: new_hash,
        edits_applied,
        syntax_check: None,
        impact: EditImpact {
            callers_affected: 0,
            tests_affected: 0,
            test_file_exists: false,
        },
        enforcement: EditEnforcement {
            tdd_warning: false,
            uncommitted_edits_in_file: false,
        },
    })
}

// ─── Strategy: text ──────────────────────────────────────────────────────────

fn apply_text_strategy(
    original: &str,
    input: &SmartEditInput,
) -> Result<(String, Vec<EditApplied>), SmartEditError> {
    let edits = input.edits.as_deref().unwrap_or(&[]);
    let mut current = original.to_owned();
    let mut applied = Vec::new();

    for pair in edits {
        let occurrences = count_occurrences(&current, &pair.old);
        if occurrences == 0 {
            return Err(SmartEditError::AmbiguousMatch {
                text: pair.old.clone(),
                occurrences: 0,
                line_numbers: vec![],
            });
        }
        if occurrences > 1 {
            let lines = find_occurrences(&current, &pair.old);
            return Err(SmartEditError::AmbiguousMatch {
                text: pair.old.clone(),
                occurrences,
                line_numbers: lines,
            });
        }

        // Exactly one match — find line number for reporting
        let line_numbers = find_occurrences(&current, &pair.old);
        let line = line_numbers.first().copied().unwrap_or(0);

        let diff = format!("-{}\n+{}", pair.old, pair.new);
        current = current.replacen(&pair.old, &pair.new, 1);

        applied.push(EditApplied {
            lines: (line, line),
            summary: format!("replaced '{}' with '{}'", pair.old, pair.new),
            diff_preview: diff,
        });
    }

    Ok((current, applied))
}

// ─── Strategy: lines ─────────────────────────────────────────────────────────

fn apply_lines_strategy(
    original: &str,
    input: &SmartEditInput,
) -> Result<(String, Vec<EditApplied>), SmartEditError> {
    let spec = input
        .line_range
        .as_deref()
        .ok_or_else(|| SmartEditError::InvalidStrategy("lines strategy requires line_range".into()))?;

    let (start, end) = parse_line_range(spec)
        .ok_or_else(|| SmartEditError::InvalidStrategy(format!("invalid line_range: {}", spec)))?;

    let new_content = input
        .new_content
        .as_deref()
        .ok_or_else(|| SmartEditError::InvalidStrategy("lines strategy requires new_content".into()))?;

    let lines: Vec<&str> = original.lines().collect();
    let total = lines.len();

    if start == 0 || start > total + 1 || end < start || end > total {
        return Err(SmartEditError::InvalidStrategy(format!(
            "line_range {}-{} out of bounds (file has {} lines)",
            start, end, total
        )));
    }

    // Replace lines [start..=end] (1-based) with new_content lines
    let new_lines: Vec<&str> = new_content.lines().collect();
    let before = &lines[..start - 1];
    let after = &lines[end..];

    let mut result_lines: Vec<&str> = Vec::new();
    result_lines.extend_from_slice(before);
    result_lines.extend_from_slice(&new_lines);
    result_lines.extend_from_slice(after);

    // Preserve trailing newline if original had one
    let mut result = result_lines.join("\n");
    if original.ends_with('\n') {
        result.push('\n');
    }

    let applied = vec![EditApplied {
        lines: (start, end),
        summary: format!("replaced lines {}-{}", start, end),
        diff_preview: format!("lines {}-{} replaced", start, end),
    }];

    Ok((result, applied))
}
