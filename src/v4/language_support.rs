use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SupportedLanguage {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Php,
    Go,
    Swift,
    Java,
    Unknown,
}

pub struct LanguageDetector;

impl LanguageDetector {
    pub fn detect(path: &str) -> SupportedLanguage {
        if path.ends_with(".rs") {
            SupportedLanguage::Rust
        } else if path.ends_with(".ts") || path.ends_with(".tsx") {
            SupportedLanguage::TypeScript
        } else if path.ends_with(".js") || path.ends_with(".jsx") {
            SupportedLanguage::JavaScript
        } else if path.ends_with(".py") {
            SupportedLanguage::Python
        } else if path.ends_with(".php") {
            SupportedLanguage::Php
        } else if path.ends_with(".go") {
            SupportedLanguage::Go
        } else if path.ends_with(".swift") {
            SupportedLanguage::Swift
        } else if path.ends_with(".java") {
            SupportedLanguage::Java
        } else {
            SupportedLanguage::Unknown
        }
    }

    pub fn is_supported(path: &str) -> bool {
        Self::detect(path) != SupportedLanguage::Unknown
    }
}
