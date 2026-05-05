use crate::token_stats::{TokenStatsBucket, TokenStatsSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenStatsGroupBy {
    Tool,
    Category,
    None,
}

impl TokenStatsGroupBy {
    pub fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("tool") {
            "tool" => Self::Tool,
            "category" => Self::Category,
            "none" => Self::None,
            _ => Self::Tool,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Category => "category",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenStatsRequest {
    pub session_id: String,
    pub group_by: TokenStatsGroupBy,
}

impl TokenStatsRequest {
    pub fn new(session_id: impl Into<String>, group_by: TokenStatsGroupBy) -> Self {
        Self {
            session_id: session_id.into(),
            group_by,
        }
    }

    pub fn current() -> Self {
        Self::new("current", TokenStatsGroupBy::Tool)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenStatsResponse {
    pub events: u64,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
    pub group_by: String,
    pub buckets: Vec<TokenStatsBucket>,
}

impl TokenStatsResponse {
    pub fn from_summary(summary: TokenStatsSummary, group_by: TokenStatsGroupBy) -> Self {
        let buckets = match group_by {
            TokenStatsGroupBy::Tool => summary.by_tool.clone(),
            TokenStatsGroupBy::Category => summary.by_category.clone(),
            TokenStatsGroupBy::None => Vec::new(),
        };

        Self {
            events: summary.events,
            raw_tokens: summary.raw_tokens,
            compressed_tokens: summary.compressed_tokens,
            saved_tokens: summary.saved_tokens,
            savings_ratio: summary.savings_ratio,
            group_by: group_by.as_str().to_string(),
            buckets,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSavingsReportFormat {
    Markdown,
    Text,
}

impl TokenSavingsReportFormat {
    pub fn parse(value: Option<&str>) -> Self {
        match value.unwrap_or("markdown") {
            "markdown" | "md" => Self::Markdown,
            "text" | "plain" => Self::Text,
            _ => Self::Markdown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSavingsReportRequest {
    pub session_id: String,
    pub format: TokenSavingsReportFormat,
}

impl TokenSavingsReportRequest {
    pub fn new(session_id: impl Into<String>, format: TokenSavingsReportFormat) -> Self {
        Self {
            session_id: session_id.into(),
            format,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSavingsReportResponse {
    pub content: String,
}

pub fn render_token_savings_report(
    summary: &TokenStatsSummary,
    format: TokenSavingsReportFormat,
) -> TokenSavingsReportResponse {
    match format {
        TokenSavingsReportFormat::Markdown => TokenSavingsReportResponse {
            content: render_markdown_report(summary),
        },
        TokenSavingsReportFormat::Text => TokenSavingsReportResponse {
            content: render_text_report(summary),
        },
    }
}

fn render_markdown_report(summary: &TokenStatsSummary) -> String {
    let mut output = String::new();

    output.push_str("# Token Savings Report\n\n");
    output.push_str(&format!("- Events: {}\n", summary.events));
    output.push_str(&format!("- Raw tokens: {}\n", summary.raw_tokens));
    output.push_str(&format!("- Compressed tokens: {}\n", summary.compressed_tokens));
    output.push_str(&format!("- Saved tokens: {}\n", summary.saved_tokens));
    output.push_str(&format!("- Savings ratio: {:.2}%\n", summary.savings_ratio * 100.0));

    if !summary.by_tool.is_empty() {
        output.push_str("\n## By Tool\n\n");
        output.push_str("| Tool | Events | Raw | Compressed | Saved | Savings |\n");
        output.push_str("|---|---:|---:|---:|---:|---:|\n");

        for bucket in &summary.by_tool {
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} | {:.2}% |\n",
                bucket.name,
                bucket.events,
                bucket.raw_tokens,
                bucket.compressed_tokens,
                bucket.saved_tokens,
                bucket.savings_ratio * 100.0
            ));
        }
    }

    output
}

fn render_text_report(summary: &TokenStatsSummary) -> String {
    format!(
        "Token Savings Report\nEvents: {}\nRaw tokens: {}\nCompressed tokens: {}\nSaved tokens: {}\nSavings ratio: {:.2}%",
        summary.events,
        summary.raw_tokens,
        summary.compressed_tokens,
        summary.saved_tokens,
        summary.savings_ratio * 100.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_stats::{TokenStatsBucket, TokenStatsSummary};

    fn summary() -> TokenStatsSummary {
        TokenStatsSummary {
            events: 2,
            raw_tokens: 100,
            compressed_tokens: 25,
            saved_tokens: 75,
            savings_ratio: 0.75,
            by_tool: vec![TokenStatsBucket {
                name: "smart_read".to_string(),
                events: 2,
                raw_tokens: 100,
                compressed_tokens: 25,
                saved_tokens: 75,
                savings_ratio: 0.75,
            }],
            by_category: vec![],
        }
    }

    #[test]
    fn parses_group_by_defaults_to_tool() {
        assert_eq!(TokenStatsGroupBy::parse(None), TokenStatsGroupBy::Tool);
        assert_eq!(TokenStatsGroupBy::parse(Some("unknown")), TokenStatsGroupBy::Tool);
    }

    #[test]
    fn builds_response_grouped_by_tool() {
        let response = TokenStatsResponse::from_summary(summary(), TokenStatsGroupBy::Tool);

        assert_eq!(response.group_by, "tool");
        assert_eq!(response.buckets.len(), 1);
    }

    #[test]
    fn builds_response_without_buckets() {
        let response = TokenStatsResponse::from_summary(summary(), TokenStatsGroupBy::None);

        assert_eq!(response.group_by, "none");
        assert!(response.buckets.is_empty());
    }

    #[test]
    fn renders_markdown_report() {
        let report = render_token_savings_report(&summary(), TokenSavingsReportFormat::Markdown);

        assert!(report.content.contains("# Token Savings Report"));
        assert!(report.content.contains("75.00%"));
        assert!(report.content.contains("smart_read"));
    }

    #[test]
    fn renders_text_report() {
        let report = render_token_savings_report(&summary(), TokenSavingsReportFormat::Text);

        assert!(report.content.contains("Token Savings Report"));
        assert!(report.content.contains("Saved tokens: 75"));
    }
}
