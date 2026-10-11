use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use thiserror::Error;
use crate::intelligence::tree_sitter_provider::TreeSitterProvider;
use crate::tools::prefetch::PrefetchedFile;
use crate::v4::context::ServedMap;
use crate::v4::pack::Representation;

/// Largest file smart_read will load into memory (8 MiB).
pub const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;
/// Skeleton text budget (about 4k tokens). Larger skeletons are cut and flagged.
pub const MAX_SKELETON_BYTES: usize = 16 * 1024;
/// Symbol entries returned per response.
pub const MAX_SYMBOLS: usize = 200;

/// Cut `s` to at most `max` bytes without splitting a UTF-8 character.
/// Returns the text and whether it was cut.
fn cap_at_char_boundary(s: String, max: usize) -> (String, bool) {
    if s.len() <= max {
        return (s, false);
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    (s[..end].to_string(), true)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReadMode {
    #[default]
    Auto,
    Skeleton,
    Focused,
    Full,
    Diff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartReadInput {
    pub path: String,
    pub mode: ReadMode,
    pub focus: Option<String>,
    pub lines: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerInfo {
    pub name: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartReadResult {
    pub path: String,
    pub mode_used: String,
    pub file_hash: String,
    pub loc: usize,
    pub stale: bool,
    pub truncated: bool,
    pub intelligence_level: String,
    pub summary: Option<String>,
    pub symbols: Vec<SymbolInfo>,
    pub imports: Vec<String>,
    pub callers: Vec<CallerInfo>,
    pub relevant_tests: Vec<String>,
    pub content: Option<String>,
    pub suggested_next: Vec<String>,
    pub prefetched: Vec<PrefetchedFile>,
}

#[derive(Debug, Error)]
pub enum SmartReadError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Binary file detected: {0}")]
    BinaryFile(String),
    #[error("Invalid line range: {0}")]
    InvalidRange(String),
    #[error("Access denied: {0}")]
    Denied(String),
    #[error("File too large (limit {limit} bytes): {path}. Read a part instead with mode \"focused\" and lines (e.g. \"1-200\") or focus (symbol or search term).")]
    TooLarge { path: String, limit: u64 },
}

/// Reads at most `limit + 1` bytes so a file that grows during the read cannot exhaust memory.
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, std::io::Error> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn detect_language(path: &str) -> Option<&str> {
    let ext = std::path::Path::new(path).extension()?.to_str()?;
    match ext {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" => Some("javascript"),
        "go" => Some("go"),
        "php" => Some("php"),
        "swift" => Some("swift"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some("cpp"),
        _ => None,
    }
}

pub fn smart_read(input: &SmartReadInput, project_root: &Path) -> Result<SmartReadResult, SmartReadError> {
    // Resolve against the project root, reject escapes, and apply the Deny-Read policy.
    let resolved = crate::security::deny_list::check_read_allowed(&input.path, project_root)
        .map_err(SmartReadError::Denied)?;
    if std::fs::metadata(&resolved)?.len() > MAX_READ_BYTES {
        return Err(SmartReadError::TooLarge { path: input.path.clone(), limit: MAX_READ_BYTES });
    }
    let raw = read_bounded(&resolved, MAX_READ_BYTES)?;
    if raw.len() as u64 > MAX_READ_BYTES {
        // The file grew between the metadata check and the read.
        return Err(SmartReadError::TooLarge { path: input.path.clone(), limit: MAX_READ_BYTES });
    }

    // Binary detection: check first 8KB for null bytes
    let check_len = raw.len().min(8192);
    if raw[..check_len].contains(&0u8) {
        return Err(SmartReadError::BinaryFile(input.path.clone()));
    }

    let content_str = String::from_utf8_lossy(&raw).into_owned();
    let loc = content_str.lines().count();

    // Compute blake3 hash
    let hash = blake3::hash(&raw);
    let file_hash = hash.to_hex().to_string();

    // Try tree-sitter symbol extraction
    let (ts_symbols, intelligence_level) = {
        let lang = detect_language(&input.path);
        match lang {
            Some(lang_str) => {
                let provider = TreeSitterProvider::new();
                match provider.extract_symbols(&content_str, lang_str) {
                    Ok(syms) => {
                        let converted: Vec<SymbolInfo> = syms.iter().map(|s| SymbolInfo {
                            name: s.name.clone(),
                            kind: format!("{:?}", s.kind).to_lowercase(),
                            line: s.start_line,
                            start_line: s.start_line,
                            end_line: s.end_line,
                        }).collect();
                        (converted, "tree-sitter".to_string())
                    }
                    Err(_) => (vec![], "heuristic".to_string()),
                }
            }
            None => (vec![], "heuristic".to_string()),
        }
    };

    // Determine mode
    let effective_mode = match &input.mode {
        ReadMode::Auto => {
            if input.lines.is_some() || input.focus.is_some() {
                ReadMode::Focused
            } else if loc < 100 {
                ReadMode::Full
            } else {
                ReadMode::Skeleton
            }
        }
        other => other.clone(),
    };

    let (mode_used, content, truncated) = match &effective_mode {
        ReadMode::Full => {
            ("full".to_string(), Some(content_str.clone()), false)
        }
        ReadMode::Skeleton => {
            let skeleton = if !ts_symbols.is_empty() {
                build_skeleton_from_symbols(&content_str, &ts_symbols)
            } else {
                build_skeleton(&content_str)
            };
            let trunc = skeleton.lines().count() < loc;
            let (skeleton, cut) = cap_at_char_boundary(skeleton, MAX_SKELETON_BYTES);
            let note = "\n... [skeleton truncated; use focus or lines for more]";
            let skeleton = if cut { format!("{skeleton}{note}") } else { skeleton };
            ("skeleton".to_string(), Some(skeleton), trunc || cut)
        }
        ReadMode::Focused => {
            let focused = build_focused_with_symbols(&content_str, input.focus.as_deref(), input.lines.as_deref(), &ts_symbols)?;
            ("focused".to_string(), Some(focused), true)
        }
        ReadMode::Auto => {
            // Should not reach here after normalization
            ("full".to_string(), Some(content_str.clone()), false)
        }
        ReadMode::Diff => {
            ("diff".to_string(), None, false)
        }
    };

    // Build suggested_next: related files the caller might want to read next
    let suggested_next = suggest_next_files(&input.path, project_root);

    // Keep the symbol list bounded too: a file with thousands of items must not
    // cost more tokens than the file itself.
    let mut ts_symbols = ts_symbols;
    let symbols_cut = ts_symbols.len() > MAX_SYMBOLS;
    ts_symbols.truncate(MAX_SYMBOLS);

    Ok(SmartReadResult {
        path: input.path.clone(),
        mode_used,
        file_hash,
        loc,
        stale: false,
        truncated: truncated || symbols_cut,
        intelligence_level,
        summary: None,
        symbols: ts_symbols,
        imports: vec![],
        callers: vec![],
        relevant_tests: vec![],
        content,
        suggested_next,
        prefetched: vec![],
    })
}

/// Session-aware smart_read: the body is compared with the version this session was last served
/// for the same path and view. Unchanged content becomes a one-line `unchanged since <hash8>`
/// marker (mode_used "unchanged"); changed content becomes a unified diff (mode_used "diff") when
/// that is smaller than the body; otherwise the full body is returned.
pub fn smart_read_in_session(
    input: &SmartReadInput,
    project_root: &Path,
    served: &mut ServedMap,
) -> Result<SmartReadResult, SmartReadError> {
    let mut result = smart_read(input, project_root)?;
    let Some(body) = result.content.take() else {
        return Ok(result);
    };
    let key = format!(
        "{}|{}|{}|{}",
        input.path,
        result.mode_used,
        input.focus.as_deref().unwrap_or(""),
        input.lines.as_deref().unwrap_or("")
    );
    let (mode_used, text) = match served.serve(&key, &body) {
        Representation::Unchanged(marker) => ("unchanged".to_string(), marker),
        Representation::Diff(diff) => ("diff".to_string(), diff),
        other => (result.mode_used.clone(), other.text().to_string()),
    };
    result.mode_used = mode_used;
    result.content = Some(text);
    Ok(result)
}

/// Build a skeleton using tree-sitter symbol information (signature lines only).
/// Skips trivial one-liner symbols (start_line == end_line) to ensure compression.
fn build_skeleton_from_symbols(content: &str, symbols: &[SymbolInfo]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();

    for sym in symbols {
        let start = sym.start_line.saturating_sub(1);
        // Skip trivial one-liners (entire symbol on a single line, e.g. `fn foo() {}`)
        if sym.start_line == sym.end_line {
            continue;
        }
        if start < lines.len() {
            result.push(format!("{}: {}", sym.start_line, lines[start]));
        }
        // Add closing brace if present
        let end = sym.end_line.saturating_sub(1);
        if end < lines.len() && end > start {
            let end_line_text = lines[end].trim();
            if end_line_text == "}" || end_line_text == "}," {
                result.push(format!("{}: {}", sym.end_line, lines[end]));
            }
        }
    }

    result.join("\n")
}

/// Build focused content, using symbol line ranges when a focus term matches a symbol name.
fn build_focused_with_symbols(
    content: &str,
    focus: Option<&str>,
    lines: Option<&str>,
    symbols: &[SymbolInfo],
) -> Result<String, SmartReadError> {
    let all_lines: Vec<&str> = content.lines().collect();

    if let Some(range_str) = lines {
        return extract_line_range(&all_lines, range_str);
    }

    if let Some(term) = focus {
        // Try to find a symbol matching the focus term exactly
        if let Some(sym) = symbols.iter().find(|s| s.name == term) {
            let start = sym.start_line.saturating_sub(1);
            let end = sym.end_line.min(all_lines.len());
            let extracted: Vec<String> = all_lines[start..end]
                .iter()
                .enumerate()
                .map(|(i, l)| format!("{}: {}", start + i + 1, l))
                .collect();
            return Ok(extracted.join("\n"));
        }
        // Fall back to grep-based extraction
        return Ok(extract_around_match(&all_lines, term));
    }

    Ok(content.to_string())
}

/// Build a skeleton of the file: keep structurally significant lines.
fn build_skeleton(content: &str) -> String {
    let mut result = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if is_skeleton_line(trimmed) {
            result.push(format!("{}: {}", i + 1, line));
        }
    }
    result.join("\n")
}

fn is_skeleton_line(trimmed: &str) -> bool {
    // Exclude trivial one-liner definitions with empty bodies on the same line
    // e.g. `fn foo() {}` or `struct Foo {}` — they have no interesting body
    if (trimmed.ends_with("{}") || trimmed.ends_with("{ }"))
        && !trimmed.starts_with("//")
        && !trimmed.starts_with("#[")
    {
        return false;
    }

    let prefixes = [
        "pub fn ", "fn ", "pub struct ", "struct ", "pub enum ", "enum ",
        "pub impl ", "impl ", "pub mod ", "mod ", "use ", "pub use ",
        "pub trait ", "trait ", "pub type ", "type ", "//!", "///",
        "#[", "async fn ", "pub async fn ", "class ", "def ", "interface ",
        "export ", "import ",
    ];
    for prefix in &prefixes {
        if trimmed.starts_with(prefix) {
            return true;
        }
    }
    // closing braces
    if trimmed == "}" || trimmed == "})" || trimmed == "}," {
        return true;
    }
    false
}

/// Build focused content from line range or search term (no symbol context).
#[allow(dead_code)]
fn build_focused(content: &str, focus: Option<&str>, lines: Option<&str>) -> Result<String, SmartReadError> {
    build_focused_with_symbols(content, focus, lines, &[])
}

/// Extract a 1-based line range "start-end".
fn extract_line_range(lines: &[&str], range: &str) -> Result<String, SmartReadError> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        return Err(SmartReadError::InvalidRange(range.to_string()));
    }
    let start: usize = parts[0].trim().parse().map_err(|_| SmartReadError::InvalidRange(range.to_string()))?;
    let end: usize = parts[1].trim().parse().map_err(|_| SmartReadError::InvalidRange(range.to_string()))?;

    if start == 0 || start > end || end > lines.len() + 1 {
        return Err(SmartReadError::InvalidRange(range.to_string()));
    }

    let start_idx = start - 1;
    let end_idx = end.min(lines.len());
    Ok(lines[start_idx..end_idx].join("\n"))
}

/// Suggest related files the caller might want to read next.
/// Looks for test/source counterparts and sibling files in the same directory.
fn suggest_next_files(file_path: &str, project_root: &Path) -> Vec<String> {
    let path = Path::new(file_path);
    let base_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    if base_name.is_empty() {
        return vec![];
    }

    let mut suggestions: Vec<String> = Vec::new();
    let max_suggestions = 3;

    // Strategy 1: If this is a test file, suggest the source file (and vice versa)
    let is_test = base_name.starts_with("test_")
        || base_name.ends_with("_test")
        || base_name.ends_with("_tests")
        || file_path.contains("/tests/");

    if is_test {
        // Strip test prefix/suffix to find source file name
        let source_name = base_name
            .strip_prefix("test_")
            .or_else(|| base_name.strip_suffix("_test"))
            .or_else(|| base_name.strip_suffix("_tests"))
            .unwrap_or(base_name);
        // Look for source in src/ directory
        let candidates = [
            format!("src/{}.{}", source_name, ext),
            format!("src/{}/mod.{}", source_name, ext),
        ];
        for candidate in &candidates {
            if suggestions.len() >= max_suggestions {
                break;
            }
            let abs = project_root.join(candidate);
            if abs.exists() {
                suggestions.push(candidate.clone());
            }
        }
    } else {
        // Look for test counterpart
        let test_candidates = [
            format!("tests/test_{}.{}", base_name, ext),
            format!("tests/{}_test.{}", base_name, ext),
        ];
        for candidate in &test_candidates {
            if suggestions.len() >= max_suggestions {
                break;
            }
            let abs = project_root.join(candidate);
            if abs.exists() {
                suggestions.push(candidate.clone());
            }
        }
    }

    // Strategy 2: Suggest other files in the same directory (excluding self)
    if let Some(parent) = path.parent() {
        let parent_abs = if parent.is_absolute() {
            parent.to_path_buf()
        } else {
            project_root.join(parent)
        };
        if let Ok(entries) = std::fs::read_dir(&parent_abs) {
            for entry in entries.flatten() {
                if suggestions.len() >= max_suggestions {
                    break;
                }
                let entry_path = entry.path();
                if !entry_path.is_file() {
                    continue;
                }
                let entry_ext = entry_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if entry_ext != ext {
                    continue;
                }
                let entry_name = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let self_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if entry_name == self_name {
                    continue;
                }
                let rel = entry_path
                    .strip_prefix(project_root)
                    .unwrap_or(&entry_path)
                    .to_string_lossy()
                    .to_string();
                if !suggestions.contains(&rel) {
                    suggestions.push(rel);
                }
            }
        }
    }

    suggestions.truncate(max_suggestions);
    suggestions
}

/// Extract lines around each match of `term`, with 5 lines context on each side. Merges overlapping ranges.
fn extract_around_match(lines: &[&str], term: &str) -> String {
    let context = 5usize;
    let mut ranges: Vec<(usize, usize)> = vec![];

    for (i, line) in lines.iter().enumerate() {
        if line.contains(term) {
            let start = i.saturating_sub(context);
            let end = (i + context + 1).min(lines.len());
            ranges.push((start, end));
        }
    }

    if ranges.is_empty() {
        return String::new();
    }

    // Merge overlapping ranges
    ranges.sort_by_key(|r| r.0);
    let mut merged: Vec<(usize, usize)> = vec![ranges[0]];
    for &(s, e) in &ranges[1..] {
        let last = merged.last_mut().unwrap();
        if s <= last.1 {
            last.1 = last.1.max(e);
        } else {
            merged.push((s, e));
        }
    }

    let mut result = Vec::new();
    for (start, end) in merged {
        for (i, line) in lines.iter().enumerate().take(end).skip(start) {
            result.push(format!("{}: {}", i + 1, line));
        }
    }
    result.join("\n")
}
