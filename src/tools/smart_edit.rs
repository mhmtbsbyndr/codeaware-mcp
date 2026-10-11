use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

use crate::tools::confidence::ConfidenceScore;
use crate::tools::semantic_diff::SemanticChange;
use crate::tools::test_selector::TestSelection;

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
    pub confidence: Option<ConfidenceScore>,
    pub semantic_changes: Option<Vec<SemanticChange>>,
    pub suggested_tests: Option<TestSelection>,
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

    #[error("file is not valid UTF-8, refusing to edit: {0}")]
    NotUtf8(String),

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
/// An empty needle has no meaningful occurrences and is reported as 0
/// (it would otherwise match at every position and never advance).
fn count_occurrences(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
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

/// Line terminator used by the file: CRLF if the file contains any CRLF, else LF.
fn detect_eol(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Normalize `s` to the given line terminator.
fn normalize_eol(s: &str, eol: &str) -> String {
    let lf = s.replace("\r\n", "\n");
    if eol == "\n" {
        lf
    } else {
        lf.replace('\n', eol)
    }
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
    // Refuse non-UTF-8 input instead of lossy conversion, which would permanently
    // replace the invalid bytes with U+FFFD when the file is written back.
    let original_content = std::str::from_utf8(&original_bytes)
        .map_err(|_| SmartEditError::NotUtf8(input.path.clone()))?
        .to_owned();

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
        confidence: None,
        semantic_changes: None,
        suggested_tests: None,
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
    let eol = detect_eol(original);

    for pair in edits {
        if pair.old.is_empty() {
            return Err(SmartEditError::InvalidStrategy(
                "edit 'old' text must not be empty".into(),
            ));
        }
        // Match and insert using the file's own line terminators.
        let old_text = normalize_eol(&pair.old, eol);
        let new_text = normalize_eol(&pair.new, eol);
        let occurrences = count_occurrences(&current, &old_text);
        if occurrences == 0 {
            return Err(SmartEditError::AmbiguousMatch {
                text: pair.old.clone(),
                occurrences: 0,
                line_numbers: vec![],
            });
        }
        if occurrences > 1 {
            let lines = find_occurrences(&current, &old_text);
            return Err(SmartEditError::AmbiguousMatch {
                text: pair.old.clone(),
                occurrences,
                line_numbers: lines,
            });
        }

        // Exactly one match — find line number for reporting
        let line_numbers = find_occurrences(&current, &old_text);
        let line = line_numbers.first().copied().unwrap_or(0);

        let diff = format!("-{}\n+{}", pair.old, pair.new);
        current = current.replacen(&old_text, &new_text, 1);

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

    // Segments keep their original terminators, so untouched lines are byte-identical.
    let segments: Vec<&str> = original.split_inclusive('\n').collect();
    let total = segments.len();

    if start == 0 || start > total + 1 || end < start || end > total {
        return Err(SmartEditError::InvalidStrategy(format!(
            "line_range {}-{} out of bounds (file has {} lines)",
            start, end, total
        )));
    }

    let eol = detect_eol(original);
    let new_lines: Vec<&str> = new_content.lines().collect();
    // If the file's last line has no terminator, the replacement block must not add one.
    let ends_without_newline = end == total && !original.ends_with('\n');

    let mut result = String::with_capacity(original.len() + new_content.len());
    for seg in &segments[..start - 1] {
        result.push_str(seg);
    }
    for (i, line) in new_lines.iter().enumerate() {
        result.push_str(line);
        if !(ends_without_newline && i + 1 == new_lines.len()) {
            result.push_str(eol);
        }
    }
    for seg in &segments[end..] {
        result.push_str(seg);
    }

    let applied = vec![EditApplied {
        lines: (start, end),
        summary: format!("replaced lines {}-{}", start, end),
        diff_preview: format!("lines {}-{} replaced", start, end),
    }];

    Ok((result, applied))
}
