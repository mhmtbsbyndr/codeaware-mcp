use serde::{Deserialize, Serialize};

use crate::v4::tokens::estimate_tokens;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedSummary {
    pub path: String,
    pub summary: String,
    pub estimated_tokens: usize,
    pub source_chars: usize,
}

pub struct SummaryGenerator;

impl SummaryGenerator {
    pub fn summarize_file(path: impl Into<String>, content: &str) -> GeneratedSummary {
        let path = path.into();
        let source_chars = content.chars().count();
        let first_lines: Vec<&str> = content.lines().take(12).collect();
        let preview = first_lines.join("\n");

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
