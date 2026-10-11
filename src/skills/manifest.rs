//! `skills/MANIFEST.toml`: the pinned, reviewed inventory of vendored skills.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::SkillError;

/// File name of the manifest inside the skills directory.
pub const MANIFEST_FILE: &str = "MANIFEST.toml";
/// Current manifest schema version.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    #[serde(default, rename = "sources")]
    pub sources: Vec<SourceEntry>,
    #[serde(default, rename = "skills")]
    pub skills: Vec<SkillEntry>,
    #[serde(default, rename = "skipped")]
    pub skipped: Vec<SkippedEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    pub id: String,
    pub repo: String,
    pub commit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_license: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEntry {
    pub id: String,
    pub source: String,
    /// Path of the original SKILL.md relative to the source repository root.
    pub source_path: String,
    /// Path of the vendored SKILL.md relative to the skills directory.
    pub path: String,
    pub license: String,
    /// Vendored copy of the license text that governs this skill, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_file: Option<String>,
    pub sha256: String,
    /// Vendored files the SKILL.md references, relative to the skill directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedEntry {
    pub source: String,
    pub source_path: String,
    /// Stable reason code, e.g. `security:pipe-to-shell` or `license-conflict`.
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
}

impl Manifest {
    /// Reads and parses a manifest file.
    pub fn read(path: &Path) -> Result<Self, SkillError> {
        let text = std::fs::read_to_string(path).map_err(|source| SkillError::Io {
            path: path.display().to_string(),
            source,
        })?;
        toml::from_str(&text).map_err(|e| SkillError::Manifest {
            path: path.display().to_string(),
            message: e.to_string(),
        })
    }

    /// Deterministic TOML serialization.
    pub fn to_toml(&self) -> Result<String, SkillError> {
        toml::to_string(self).map_err(|e| SkillError::Other(format!("manifest encode: {e}")))
    }
}
