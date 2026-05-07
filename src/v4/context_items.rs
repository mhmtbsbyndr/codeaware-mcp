use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextItemKind {
    FileSummary,
    FileExcerpt,
    ArchitectureRule,
    TestHint,
    RecentChange,
    DecisionRecord,
    Contract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub kind: ContextItemKind,
    pub path: Option<String>,
    pub symbol: Option<String>,
    pub content: String,
    pub reason: String,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedContext {
    pub path: String,
    pub reason: String,
}
