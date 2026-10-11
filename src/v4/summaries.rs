use std::fs;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::v4::tokens::estimate_tokens;

/// Files larger than this are not read into memory for summaries or indexing.
pub const MAX_READ_BYTES: u64 = 512 * 1024;

/// Preview lines longer than this are truncated.
pub const MAX_PREVIEW_LINE_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSummary {
    pub path: String,
    pub summary: String,
    pub estimated_tokens: usize,
    pub source_chars: usize,
}

pub struct SummaryGenerator;

/// Reads a UTF-8 source file if it is at most `MAX_READ_BYTES` long.
/// Returns `None` for oversized, unreadable or non-UTF-8 files.
pub fn read_capped(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_READ_BYTES {
        return None;
    }
    let mut content = String::new();
    fs::File::open(path)
        .ok()?
        .take(MAX_READ_BYTES + 1)
        .read_to_string(&mut content)
        .ok()?;
    if content.len() as u64 > MAX_READ_BYTES {
        return None;
    }
    Some(content)
}

fn truncate_line(line: &str) -> String {
    if line.chars().count() <= MAX_PREVIEW_LINE_CHARS {
        return line.to_string();
    }
    let mut truncated: String = line.chars().take(MAX_PREVIEW_LINE_CHARS).collect();
    truncated.push_str(" ...");
    truncated
}

impl SummaryGenerator {
    pub fn summarize_file(path: impl Into<String>, content: &str) -> GeneratedSummary {
        let path = path.into();
        let source_chars = content.chars().count();
        let preview: Vec<String> = content.lines().take(12).map(truncate_line).collect();
        let preview = preview.join("\n");

        let summary = if source_chars == 0 {
            format!("{} is empty.", path)
        } else {
            format!(
                "Summary for {}: {} chars, {} lines. Preview:\n{}",
                path,
                source_chars,
                content.lines().count(),
                preview
            )
        };

        let estimated_tokens = estimate_tokens(&summary);

        GeneratedSummary {
            path,
            summary,
            estimated_tokens,
            source_chars,
        }
    }
}

/// Declaration keywords kept in a skeleton (after any visibility prefix is removed).
const DECLARATION_KEYWORDS: &[&str] = &[
    "fn ", "struct ", "enum ", "trait ", "impl ", "impl<", "mod ", "type ", "const ", "static ",
    "class ", "def ", "function ", "interface ", "func ",
];

/// Maximum declarations kept in a skeleton.
const MAX_SKELETON_DECLARATIONS: usize = 60;

/// Maximum characters kept from one declaration line.
const MAX_DECLARATION_CHARS: usize = 160;

fn strip_declaration_prefix(trimmed: &str) -> &str {
    let mut rest = trimmed;
    if let Some(after) = rest.strip_prefix("pub") {
        if let Some(inner) = after.strip_prefix('(') {
            if let Some(end) = inner.find(')') {
                rest = inner[end + 1..].trim_start();
            }
        } else if after.starts_with(char::is_whitespace) {
            rest = after.trim_start();
        }
    }
    for modifier in ["export ", "async ", "unsafe ", "extern "] {
        if let Some(after) = rest.strip_prefix(modifier) {
            rest = after.trim_start();
        }
    }
    rest
}

/// Returns the signature of a declaration line with its body removed, or `None` when the
/// line does not declare anything. `pub fn f(x: u8) -> u8 {` becomes `pub fn f(x: u8) -> u8 { ... }`.
pub fn declaration_skeleton(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = strip_declaration_prefix(trimmed);
    if !DECLARATION_KEYWORDS.iter().any(|keyword| rest.starts_with(keyword)) {
        return None;
    }
    let (signature, has_body) = match trimmed.find('{') {
        Some(pos) => (trimmed[..pos].trim_end(), true),
        None => (trimmed, false),
    };
    let mut out = signature.to_string();
    if has_body {
        out.push_str(" { ... }");
    }
    Some(truncate_chars(&out, MAX_DECLARATION_CHARS))
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out.push_str(" ...");
    out
}

/// Leading `///` or `//!` doc block (first one in the file), joined into one line.
/// Returns an empty string when the file has no doc block.
pub fn doc_head(content: &str, max_lines: usize) -> String {
    let mut collected: Vec<&str> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        let text = trimmed
            .strip_prefix("//!")
            .or_else(|| trimmed.strip_prefix("///"));
        match text {
            Some(text) => {
                if collected.len() < max_lines {
                    collected.push(text.trim());
                }
            }
            None if !collected.is_empty() => break,
            None => {}
        }
    }
    collected.retain(|line| !line.is_empty());
    collected.join(" ")
}

impl SummaryGenerator {
    /// Signatures only: one line per declaration, bodies replaced by `{ ... }`.
    pub fn skeleton(path: &str, content: &str) -> String {
        let declarations: Vec<String> = content
            .lines()
            .filter_map(declaration_skeleton)
            .take(MAX_SKELETON_DECLARATIONS)
            .collect();
        if declarations.is_empty() {
            return format!("{path}: no declarations");
        }
        format!("{path} skeleton:\n{}", declarations.join("\n"))
    }

    /// One line: path, line count, and the first doc line (or first declaration when there
    /// is no doc block).
    pub fn one_line(path: &str, content: &str) -> String {
        let lines = content.lines().count();
        let head = {
            let doc = doc_head(content, 2);
            if doc.is_empty() {
                content
                    .lines()
                    .find_map(declaration_skeleton)
                    .unwrap_or_default()
            } else {
                doc
            }
        };
        if head.is_empty() {
            format!("{path} ({lines} lines)")
        } else {
            format!("{path} ({lines} lines): {}", truncate_line(&head))
        }
    }
}
