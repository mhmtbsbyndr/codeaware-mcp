use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SessionStatusResult {
    pub session_id: String,
    pub steps_completed: u32,
    pub token_metrics: TokenMetrics,
    pub files_read: Vec<FileReadInfo>,
    pub edits_made: Vec<EditInfo>,
    pub last_test: Option<serde_json::Value>,
    pub error_loops: Vec<String>,
    pub verification_checklist: VerificationChecklist,
}

#[derive(Debug, Serialize)]
pub struct TokenMetrics {
    pub estimated_raw: usize,
    pub estimated_compressed: usize,
    pub estimated_saved: usize,
    pub compression_ratio: f32,
}

#[derive(Debug, Serialize)]
pub struct FileReadInfo {
    pub path: String,
    pub step: u32,
    pub mode: String,
    pub stale: bool,
}

#[derive(Debug, Serialize)]
pub struct EditInfo {
    pub path: String,
    pub step: u32,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct VerificationChecklist {
    pub files_modified: usize,
    pub files_with_tests: usize,
    pub uncommitted_changes: bool,
    pub compiler_errors: usize,
    pub open_failures: usize,
}
