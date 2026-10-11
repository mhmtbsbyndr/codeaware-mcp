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
