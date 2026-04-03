use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
}

fn default_max_file_lines_full() -> u32 {
    100
}

fn default_max_command_output() -> u32 {
    50
}

fn default_max_search_results() -> u32 {
    20
}

fn default_true() -> bool {
    true
}

fn default_intelligence_strategy() -> String {
    "auto".to_string()
}

fn default_persistence_path() -> String {
    "~/.codeaware/sessions.db".to_string()
}

fn default_pattern_confidence_decay() -> f64 {
    0.05
}

fn default_pattern_prune_threshold() -> f64 {
    0.2
}

fn default_pattern_prune_after_days() -> u32 {
    30
}

fn default_confidence_threshold() -> u32 {
    60
}

fn default_confidence_mode() -> String {
    "warn".to_string()
}

fn default_error_loop_threshold() -> u32 {
    3
}

fn default_max_iterations_per_task() -> u32 {
    5
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ProjectConfig {
    pub name: String,
    pub languages: Vec<String>,
    pub ignore: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct IntelligenceConfig {
    #[serde(default = "default_intelligence_strategy")]
    pub strategy: String,
}

impl Default for IntelligenceConfig {
    fn default() -> Self {
        Self {
            strategy: default_intelligence_strategy(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct CompressionConfig {
    #[serde(default = "default_max_file_lines_full")]
    pub max_file_lines_full: u32,
    #[serde(default = "default_max_command_output")]
    pub max_command_output: u32,
    #[serde(default = "default_max_search_results")]
    pub max_search_results: u32,
    #[serde(default = "default_true")]
    pub include_callers: bool,
    #[serde(default = "default_true")]
    pub include_tests: bool,
    #[serde(default = "default_true")]
    pub scan_secrets: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_file_lines_full: default_max_file_lines_full(),
            max_command_output: default_max_command_output(),
            max_search_results: default_max_search_results(),
            include_callers: true,
            include_tests: true,
            scan_secrets: true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    #[serde(default = "default_true")]
    pub track_seen_files: bool,
    #[serde(default = "default_true")]
    pub persistence: bool,
    #[serde(default = "default_persistence_path")]
    pub persistence_path: String,
    #[serde(default = "default_true")]
    pub pattern_learning: bool,
    #[serde(default = "default_pattern_confidence_decay")]
    pub pattern_confidence_decay: f64,
    #[serde(default = "default_pattern_prune_threshold")]
    pub pattern_prune_threshold: f64,
    #[serde(default = "default_pattern_prune_after_days")]
    pub pattern_prune_after_days: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            track_seen_files: true,
            persistence: true,
            persistence_path: default_persistence_path(),
            pattern_learning: true,
            pattern_confidence_decay: default_pattern_confidence_decay(),
            pattern_prune_threshold: default_pattern_prune_threshold(),
            pattern_prune_after_days: default_pattern_prune_after_days(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct EnforcementConfig {
    pub tdd_warning: bool,
    #[serde(default = "default_error_loop_threshold")]
    pub error_loop_threshold: u32,
    #[serde(default = "default_max_iterations_per_task")]
    pub max_iterations_per_task: u32,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: u32,
    #[serde(default = "default_confidence_mode")]
    pub confidence_mode: String,
}

impl Default for EnforcementConfig {
    fn default() -> Self {
        Self {
            tdd_warning: false,
            error_loop_threshold: default_error_loop_threshold(),
            max_iterations_per_task: default_max_iterations_per_task(),
            confidence_threshold: default_confidence_threshold(),
            confidence_mode: default_confidence_mode(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct LanguageConfig {
    pub test_command: String,
    pub build_command: String,
    pub lint_command: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    #[serde(default = "default_true")]
    pub detect: bool,
    pub packages: Vec<String>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            detect: true,
            packages: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct CodeAwareConfig {
    pub project: ProjectConfig,
    pub intelligence: IntelligenceConfig,
    pub compression: CompressionConfig,
    pub session: SessionConfig,
    pub enforcement: EnforcementConfig,
    pub languages: HashMap<String, LanguageConfig>,
    pub workspace: WorkspaceConfig,
}

impl CodeAwareConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let config: CodeAwareConfig = toml::from_str(&content)?;
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(CodeAwareConfig::default()),
            Err(e) => Err(ConfigError::ReadError(e)),
        }
    }
}
