use crate::token_stats::{summarize_events, TokenEvent, TokenStatsSummary};

pub const TOKEN_EVENTS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS token_events (
    id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    tool TEXT NOT NULL,
    category TEXT NOT NULL,
    subject TEXT NOT NULL,
    raw_bytes INTEGER NOT NULL,
    compressed_bytes INTEGER NOT NULL,
    estimated_raw_tokens INTEGER NOT NULL,
    estimated_compressed_tokens INTEGER NOT NULL,
    saved_tokens INTEGER NOT NULL,
    savings_ratio REAL NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_token_events_session ON token_events(session_id);
CREATE INDEX IF NOT EXISTS idx_token_events_tool ON token_events(tool);
CREATE INDEX IF NOT EXISTS idx_token_events_category ON token_events(category);
CREATE INDEX IF NOT EXISTS idx_token_events_created_at ON token_events(created_at);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenStatsPersistenceError {
    Unavailable(String),
    WriteFailed(String),
    ReadFailed(String),
}

pub trait TokenStatsStore {
    fn init_schema(&self) -> Result<(), TokenStatsPersistenceError>;

    fn insert_event(&self, event: &TokenEvent) -> Result<(), TokenStatsPersistenceError>;

    fn list_events_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<TokenEvent>, TokenStatsPersistenceError>;

    fn summarize_session(
        &self,
        session_id: &str,
    ) -> Result<TokenStatsSummary, TokenStatsPersistenceError> {
        let events = self.list_events_for_session(session_id)?;
        Ok(summarize_events(&events))
    }
}

#[derive(Debug, Default, Clone)]
pub struct InMemoryTokenStatsStore {
    events: Vec<TokenEvent>,
}

impl InMemoryTokenStatsStore {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn events(&self) -> &[TokenEvent] {
        &self.events
    }
}

impl TokenStatsStore for InMemoryTokenStatsStore {
    fn init_schema(&self) -> Result<(), TokenStatsPersistenceError> {
        Ok(())
    }

    fn insert_event(&self, _event: &TokenEvent) -> Result<(), TokenStatsPersistenceError> {
        Err(TokenStatsPersistenceError::Unavailable(
            "InMemoryTokenStatsStore is immutable; use insert_event_mut in tests or wrap with interior mutability".to_string(),
        ))
    }

    fn list_events_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<TokenEvent>, TokenStatsPersistenceError> {
        Ok(self
            .events
            .iter()
            .filter(|event| event.session_id == session_id)
            .cloned()
            .collect())
    }
}

impl InMemoryTokenStatsStore {
    pub fn insert_event_mut(&mut self, event: TokenEvent) {
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_stats::{TokenEvent, TokenEventCategory};

    fn event(session_id: &str, tool: &str) -> TokenEvent {
        TokenEvent::new(
            format!("{session_id}-{tool}"),
            "trace-1",
            session_id,
            tool,
            TokenEventCategory::FileRead,
            "fixture",
            "aaaaaaaaaaaaaaaa",
            "aaaa",
            "2026-05-06T00:00:00Z",
        )
    }

    #[test]
    fn schema_contains_token_events_table() {
        assert!(TOKEN_EVENTS_SCHEMA.contains("CREATE TABLE IF NOT EXISTS token_events"));
        assert!(TOKEN_EVENTS_SCHEMA.contains("idx_token_events_session"));
        assert!(TOKEN_EVENTS_SCHEMA.contains("idx_token_events_tool"));
        assert!(TOKEN_EVENTS_SCHEMA.contains("idx_token_events_category"));
        assert!(TOKEN_EVENTS_SCHEMA.contains("idx_token_events_created_at"));
    }

    #[test]
    fn in_memory_store_filters_by_session() {
        let mut store = InMemoryTokenStatsStore::new();
        store.insert_event_mut(event("session-a", "smart_read"));
        store.insert_event_mut(event("session-b", "smart_run"));

        let events = store.list_events_for_session("session-a").unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].session_id, "session-a");
        assert_eq!(events[0].tool, "smart_read");
    }

    #[test]
    fn in_memory_store_summarizes_session() {
        let mut store = InMemoryTokenStatsStore::new();
        store.insert_event_mut(event("session-a", "smart_read"));
        store.insert_event_mut(event("session-a", "project_map"));
        store.insert_event_mut(event("session-b", "smart_run"));

        let summary = store.summarize_session("session-a").unwrap();

        assert_eq!(summary.events, 2);
        assert_eq!(summary.by_tool.len(), 2);
        assert!(summary.savings_ratio.is_finite());
    }
}
