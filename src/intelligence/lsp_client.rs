use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const DEFAULT_TIMEOUT_MS: u64 = 2000;
const DEFAULT_MAX_TIMEOUTS: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    #[serde(default)]
    pub servers: HashMap<String, LspServerConfig>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_max_timeouts")]
    pub max_consecutive_timeouts: u32,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}
fn default_max_timeouts() -> u32 {
    DEFAULT_MAX_TIMEOUTS
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            servers: HashMap::new(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_consecutive_timeouts: DEFAULT_MAX_TIMEOUTS,
        }
    }
}

impl LspConfig {
    pub fn from_json(json: &str) -> Result<Self, LspError> {
        serde_json::from_str(json).map_err(|e| LspError::ConfigError(e.to_string()))
    }

    pub fn from_file(path: &Path) -> Result<Self, LspError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| LspError::ConfigError(e.to_string()))?;
        Self::from_json(&content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LspCapability {
    Definition,
    References,
    Hover,
    Symbols,
    CallHierarchy,
}

/// LSP Client — manages connections to language servers.
/// In this MVP, the client is a structural placeholder with the correct interfaces.
/// Full process-spawning LSP communication will be added when LSP servers are available.
pub struct LspClient {
    connected: bool,
    consecutive_timeouts: u32,
    degraded: bool,
    capabilities: Vec<LspCapability>,
}

impl LspClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            consecutive_timeouts: 0,
            degraded: false,
            capabilities: Vec::new(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded
    }

    pub fn capabilities(&self) -> &[LspCapability] {
        &self.capabilities
    }

    /// Attempt to get definition location. Returns error if not connected.
    pub fn get_definition(
        &self,
        _lang: &str,
        _file: &str,
        _line: u32,
        _col: u32,
    ) -> Result<Location, LspError> {
        if !self.connected {
            return Err(LspError::NotConnected);
        }
        if self.degraded {
            return Err(LspError::Degraded);
        }
        Err(LspError::NotImplemented)
    }

    /// Attempt to get references. Returns error if not connected.
    pub fn get_references(
        &self,
        _lang: &str,
        _file: &str,
        _line: u32,
        _col: u32,
    ) -> Result<Vec<Location>, LspError> {
        if !self.connected {
            return Err(LspError::NotConnected);
        }
        Err(LspError::NotImplemented)
    }

    /// Record a timeout. After max_consecutive_timeouts, mark as degraded.
    pub fn record_timeout(&mut self, max: u32) {
        self.consecutive_timeouts += 1;
        if self.consecutive_timeouts >= max {
            self.degraded = true;
        }
    }

    pub fn reset_timeouts(&mut self) {
        self.consecutive_timeouts = 0;
    }
}

impl Default for LspClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Location {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("LSP not connected")]
    NotConnected,
    #[error("LSP in degraded mode (too many timeouts)")]
    Degraded,
    #[error("LSP timeout after {0}ms")]
    Timeout(u64),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Not implemented")]
    NotImplemented,
}
