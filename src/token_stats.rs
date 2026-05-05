use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenEventCategory {
    FileRead,
    CommandOutput,
    GitDiff,
    SearchOutput,
    MemoryResume,
    ToolSchema,
    Other,
}

impl TokenEventCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileRead => "file_read",
            Self::CommandOutput => "command_output",
            Self::GitDiff => "git_diff",
            Self::SearchOutput => "search_output",
            Self::MemoryResume => "memory_resume",
            Self::ToolSchema => "tool_schema",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenEvent {
    pub id: String,
    pub trace_id: String,
    pub session_id: String,
    pub tool: String,
    pub category: TokenEventCategory,
    pub subject: String,
    pub raw_bytes: u64,
    pub compressed_bytes: u64,
    pub estimated_raw_tokens: u64,
    pub estimated_compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
    pub created_at: String,
}

impl TokenEvent {
    pub fn new(
        id: impl Into<String>,
        trace_id: impl Into<String>,
        session_id: impl Into<String>,
        tool: impl Into<String>,
        category: TokenEventCategory,
        subject: impl Into<String>,
        raw: &str,
        compressed: &str,
        created_at: impl Into<String>,
    ) -> Self {
        let estimated_raw_tokens = estimate_tokens(raw);
        let estimated_compressed_tokens = estimate_tokens(compressed);
        let saved_tokens = estimated_raw_tokens as i64 - estimated_compressed_tokens as i64;
        let savings_ratio = calculate_savings_ratio(estimated_raw_tokens, estimated_compressed_tokens);

        Self {
            id: id.into(),
            trace_id: trace_id.into(),
            session_id: session_id.into(),
            tool: tool.into(),
            category,
            subject: subject.into(),
            raw_bytes: raw.len() as u64,
            compressed_bytes: compressed.len() as u64,
            estimated_raw_tokens,
            estimated_compressed_tokens,
            saved_tokens,
            savings_ratio,
            created_at: created_at.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenStatsSummary {
    pub events: u64,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
    pub by_tool: Vec<TokenStatsBucket>,
    pub by_category: Vec<TokenStatsBucket>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenStatsBucket {
    pub name: String,
    pub events: u64,
    pub raw_tokens: u64,
    pub compressed_tokens: u64,
    pub saved_tokens: i64,
    pub savings_ratio: f64,
}

pub fn estimate_tokens(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }

    let chars = text.chars().count() as u64;
    let words = text.split_whitespace().count() as u64;
    let by_chars = chars.div_ceil(4);
    let by_words = (words * 4).div_ceil(3);

    by_chars.max(by_words).max(1)
}

pub fn calculate_savings_ratio(raw_tokens: u64, compressed_tokens: u64) -> f64 {
    if raw_tokens == 0 {
        return 0.0;
    }

    let ratio = (raw_tokens as f64 - compressed_tokens as f64) / raw_tokens as f64;

    if ratio.is_finite() {
        ratio
    } else {
        0.0
    }
}

pub fn summarize_events(events: &[TokenEvent]) -> TokenStatsSummary {
    let raw_tokens = events.iter().map(|event| event.estimated_raw_tokens).sum();
    let compressed_tokens = events
        .iter()
        .map(|event| event.estimated_compressed_tokens)
        .sum();
    let saved_tokens = raw_tokens as i64 - compressed_tokens as i64;
    let savings_ratio = calculate_savings_ratio(raw_tokens, compressed_tokens);

    TokenStatsSummary {
        events: events.len() as u64,
        raw_tokens,
        compressed_tokens,
        saved_tokens,
        savings_ratio,
        by_tool: group_by_tool(events),
        by_category: group_by_category(events),
    }
}

pub fn group_by_tool(events: &[TokenEvent]) -> Vec<TokenStatsBucket> {
    let mut buckets: BTreeMap<String, Vec<&TokenEvent>> = BTreeMap::new();

    for event in events {
        buckets.entry(event.tool.clone()).or_default().push(event);
    }

    buckets
        .into_iter()
        .map(|(name, bucket_events)| build_bucket(name, &bucket_events))
        .collect()
}

pub fn group_by_category(events: &[TokenEvent]) -> Vec<TokenStatsBucket> {
    let mut buckets: BTreeMap<String, Vec<&TokenEvent>> = BTreeMap::new();

    for event in events {
        buckets
            .entry(event.category.as_str().to_string())
            .or_default()
            .push(event);
    }

    buckets
        .into_iter()
        .map(|(name, bucket_events)| build_bucket(name, &bucket_events))
        .collect()
}

fn build_bucket(name: String, events: &[&TokenEvent]) -> TokenStatsBucket {
    let raw_tokens = events.iter().map(|event| event.estimated_raw_tokens).sum();
    let compressed_tokens = events
        .iter()
        .map(|event| event.estimated_compressed_tokens)
        .sum();
    let saved_tokens = raw_tokens as i64 - compressed_tokens as i64;
    let savings_ratio = calculate_savings_ratio(raw_tokens, compressed_tokens);

    TokenStatsBucket {
        name,
        events: events.len() as u64,
        raw_tokens,
        compressed_tokens,
        saved_tokens,
        savings_ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(tool: &str, category: TokenEventCategory, raw: &str, compressed: &str) -> TokenEvent {
        TokenEvent::new(
            format!("event-{tool}"),
            "trace-1",
            "session-1",
            tool,
            category,
            "fixture",
            raw,
            compressed,
            "2026-05-06T00:00:00Z",
        )
    }

    #[test]
    fn estimates_empty_string_as_zero() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimates_prose_deterministically() {
        assert_eq!(estimate_tokens("hello world from codeaware"), 6);
    }

    #[test]
    fn estimates_code_deterministically() {
        let code = "fn main() { println!(\"hello\"); }";
        assert_eq!(estimate_tokens(code), estimate_tokens(code));
        assert!(estimate_tokens(code) > 0);
    }

    #[test]
    fn calculates_positive_savings() {
        let ratio = calculate_savings_ratio(100, 25);
        assert_eq!(ratio, 0.75);
    }

    #[test]
    fn calculates_zero_savings() {
        let ratio = calculate_savings_ratio(100, 100);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn supports_negative_savings() {
        let ratio = calculate_savings_ratio(100, 125);
        assert_eq!(ratio, -0.25);
    }

    #[test]
    fn zero_raw_tokens_returns_safe_ratio() {
        let ratio = calculate_savings_ratio(0, 10);
        assert_eq!(ratio, 0.0);
        assert!(ratio.is_finite());
    }

    #[test]
    fn summarizes_events() {
        let events = vec![
            event("smart_read", TokenEventCategory::FileRead, "aaaaaaaaaaaaaaaa", "aaaa"),
            event("smart_run", TokenEventCategory::CommandOutput, "bbbbbbbb", "bbbb"),
        ];

        let summary = summarize_events(&events);

        assert_eq!(summary.events, 2);
        assert_eq!(summary.by_tool.len(), 2);
        assert_eq!(summary.by_category.len(), 2);
        assert!(summary.savings_ratio.is_finite());
    }

    #[test]
    fn groups_by_tool() {
        let events = vec![
            event("smart_read", TokenEventCategory::FileRead, "aaaaaaaa", "aa"),
            event("smart_read", TokenEventCategory::FileRead, "bbbbbbbb", "bb"),
            event("smart_run", TokenEventCategory::CommandOutput, "cccccccc", "cc"),
        ];

        let grouped = group_by_tool(&events);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].name, "smart_read");
        assert_eq!(grouped[0].events, 2);
    }

    #[test]
    fn groups_by_category() {
        let events = vec![
            event("smart_read", TokenEventCategory::FileRead, "aaaaaaaa", "aa"),
            event("project_map", TokenEventCategory::FileRead, "bbbbbbbb", "bb"),
            event("smart_run", TokenEventCategory::CommandOutput, "cccccccc", "cc"),
        ];

        let grouped = group_by_category(&events);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].name, "command_output");
        assert_eq!(grouped[1].name, "file_read");
    }
}
