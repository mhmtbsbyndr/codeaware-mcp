use chrono::Utc;
use uuid::Uuid;
use std::collections::{HashMap, HashSet};
use crate::tools::workspace_state::WorkspaceSlots;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Idle,
    Analyzing,
    Editing,
    Verifying,
    Complete,
    Compacting,
}

pub struct SessionState {
    session_id: String,
    project_path: String,
    phase: SessionPhase,
    phase_before_compact: Option<SessionPhase>,
    steps: u32,
    files_read: HashSet<String>,
    files_edited: HashSet<String>,
    error_signatures: HashMap<String, u32>,
    pub workspace_slots: WorkspaceSlots,
    /// Memories injected from previous sessions at startup
    injected_context: Option<String>,
    // These fields are specified in the design and will be used in later phases
    #[allow(dead_code)]
    last_test_result: Option<TestResult>,
    #[allow(dead_code)]
    started_at: chrono::DateTime<Utc>,
}

pub struct TestResult {
    pub passed: u32,
    pub failed: u32,
    pub command: String,
}

impl SessionState {
    pub fn new(project_path: &str) -> Self {
        Self {
            session_id: format!("s-{}", Uuid::new_v4()),
            project_path: project_path.to_string(),
            phase: SessionPhase::Idle,
            phase_before_compact: None,
            steps: 0,
            files_read: HashSet::new(),
            files_edited: HashSet::new(),
            error_signatures: HashMap::new(),
            workspace_slots: WorkspaceSlots::new(),
            injected_context: None,
            last_test_result: None,
            started_at: Utc::now(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn phase(&self) -> SessionPhase {
        self.phase
    }

    pub fn project_path(&self) -> &str {
        &self.project_path
    }

    pub fn steps_completed(&self) -> u32 {
        self.steps
    }

    pub fn files_read(&self) -> &HashSet<String> {
        &self.files_read
    }

    pub fn files_edited(&self) -> &HashSet<String> {
        &self.files_edited
    }

    pub fn on_smart_read(&mut self, path: &str) {
        self.files_read.insert(path.to_string());
        self.phase = SessionPhase::Analyzing;
        self.steps += 1;
    }

    pub fn on_smart_edit(&mut self, path: &str) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.phase == SessionPhase::Idle {
            warnings.push("Kein vorheriger Read".to_string());
        }
        self.files_edited.insert(path.to_string());
        self.phase = SessionPhase::Editing;
        self.steps += 1;
        warnings
    }

    pub fn on_smart_run(&mut self, _command: &str) {
        self.phase = SessionPhase::Verifying;
        self.steps += 1;
    }

    pub fn on_task_complete(&mut self) {
        self.phase = SessionPhase::Complete;
    }

    pub fn on_pre_compact(&mut self) {
        self.phase_before_compact = Some(self.phase);
        self.phase = SessionPhase::Compacting;
    }

    pub fn on_post_compact(&mut self) {
        self.phase = self.phase_before_compact.unwrap_or(SessionPhase::Idle);
        self.phase_before_compact = None;
    }

    pub fn on_cwd_changed(&mut self, new_path: &str) {
        self.project_path = new_path.to_string();
        self.phase = SessionPhase::Idle;
        self.files_read.clear();
        self.files_edited.clear();
    }

    pub fn record_error_signature(&mut self, sig: &str) {
        *self.error_signatures.entry(sig.to_string()).or_insert(0) += 1;
    }

    pub fn is_error_loop(&self, sig: &str, threshold: u32) -> bool {
        self.error_signatures.get(sig).copied().unwrap_or(0) >= threshold
    }

    pub fn set_injected_context(&mut self, ctx: String) {
        self.injected_context = Some(ctx);
    }

    pub fn injected_context(&self) -> Option<&str> {
        self.injected_context.as_deref()
    }
}
