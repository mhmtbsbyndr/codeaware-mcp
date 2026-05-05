#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspPosition {
    pub line: u64,
    pub character: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspLocation {
    pub file_path: String,
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub file_path: String,
    pub range_start: LspPosition,
    pub range_end: LspPosition,
    pub severity: String,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspHoverResponse {
    pub file_path: String,
    pub position: LspPosition,
    pub contents: String,
    pub trust: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspBridgeCapabilities {
    pub hover: bool,
    pub definitions: bool,
    pub references: bool,
    pub diagnostics: bool,
    pub rename_preview: bool,
}

impl Default for LspBridgeCapabilities {
    fn default() -> Self {
        Self {
            hover: true,
            definitions: true,
            references: true,
            diagnostics: true,
            rename_preview: false,
        }
    }
}

pub trait LspBridge {
    fn capabilities(&self) -> LspBridgeCapabilities;
    fn hover(&self, file_path: &str, position: LspPosition) -> Option<LspHoverResponse>;
    fn definitions(&self, file_path: &str, position: LspPosition) -> Vec<LspLocation>;
    fn references(&self, file_path: &str, position: LspPosition) -> Vec<LspLocation>;
    fn diagnostics(&self, file_path: &str) -> Vec<LspDiagnostic>;
}

#[derive(Debug, Default)]
pub struct NullLspBridge;

impl LspBridge for NullLspBridge {
    fn capabilities(&self) -> LspBridgeCapabilities {
        LspBridgeCapabilities {
            hover: false,
            definitions: false,
            references: false,
            diagnostics: false,
            rename_preview: false,
        }
    }

    fn hover(&self, _file_path: &str, _position: LspPosition) -> Option<LspHoverResponse> {
        None
    }

    fn definitions(&self, _file_path: &str, _position: LspPosition) -> Vec<LspLocation> {
        Vec::new()
    }

    fn references(&self, _file_path: &str, _position: LspPosition) -> Vec<LspLocation> {
        Vec::new()
    }

    fn diagnostics(&self, _file_path: &str) -> Vec<LspDiagnostic> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_bridge_reports_no_capabilities() {
        let bridge = NullLspBridge;
        let capabilities = bridge.capabilities();

        assert!(!capabilities.hover);
        assert!(bridge.definitions("src/main.rs", LspPosition { line: 1, character: 1 }).is_empty());
    }
}
