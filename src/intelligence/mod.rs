pub mod tree_sitter_provider;
pub mod regex_fallback;
pub mod lsp_client;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelligenceLevel {
    LSP,
    TreeSitter,
    Regex,
}

const TREE_SITTER_LANGUAGES: &[&str] = &["rust", "python", "typescript", "javascript", "go", "php", "swift"];

pub fn select_intelligence(lang: &str, lsp_available: bool) -> IntelligenceLevel {
    if lsp_available {
        IntelligenceLevel::LSP
    } else if TREE_SITTER_LANGUAGES.contains(&lang) {
        IntelligenceLevel::TreeSitter
    } else {
        IntelligenceLevel::Regex
    }
}
