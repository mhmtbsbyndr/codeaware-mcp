use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use similar::TextDiff;

use crate::v4::budget::BudgetState;
use crate::v4::pack::Representation;
use crate::v4::tokens::estimate_tokens;
use crate::v4::context_items::{ContextItem, ExcludedContext};
use crate::v4::contracts::TaskContract;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPackage {
    pub task_id: String,
    pub repo_root: String,
    pub contract: TaskContract,
    pub budget: BudgetState,
    pub selected_context: Vec<ContextItem>,
    pub excluded_context: Vec<ExcludedContext>,
    pub warnings: Vec<String>,
}

pub struct ContextAssembler;

impl ContextAssembler {
    pub fn default_excluded_paths() -> Vec<&'static str> {
        vec![
            ".git/**",
            "target/**",
            "node_modules/**",
            "vendor/**",
            "dist/**",
            "build/**",
            ".cache/**",
            ".codeaware/**",
        ]
    }
}

/// Full blake3 hex digest of served content.
pub fn content_hash(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}

/// The version of a file that was last served into a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServedVersion {
    pub hash: String,
    pub content: String,
}

impl ServedVersion {
    pub fn new(content: &str) -> Self {
        Self {
            hash: content_hash(content),
            content: content.to_string(),
        }
    }
}

/// Decides how a file requested again should be sent.
///
/// * no previous version: `Full` with the whole body;
/// * same content hash: `Unchanged` one-line marker `unchanged since <hash8>`;
/// * changed: `Diff` (unified, 3 lines of context) when it is cheaper than the body, else `Full`.
pub fn delta_for(prev: Option<&ServedVersion>, now: &str) -> Representation {
    let Some(prev) = prev else {
        return Representation::Full(now.to_string());
    };
    let now_hash = content_hash(now);
    if prev.hash == now_hash {
        return Representation::Unchanged(format!("unchanged since {}", &now_hash[..8]));
    }

    let diff = TextDiff::from_lines(prev.content.as_str(), now)
        .unified_diff()
        .context_radius(3)
        .header(&format!("served {}", &prev.hash[..8]), &format!("now {}", &now_hash[..8]))
        .to_string();

    if estimate_tokens(&diff) < estimate_tokens(now) {
        Representation::Diff(diff)
    } else {
        Representation::Full(now.to_string())
    }
}

/// Per-session map of path to the last version served for it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServedMap {
    served: BTreeMap<String, ServedVersion>,
}

impl ServedMap {
    /// Returns the representation for `now` relative to what was served before, and records
    /// `now` as the newly served version.
    pub fn serve(&mut self, path: &str, now: &str) -> Representation {
        let representation = delta_for(self.served.get(path), now);
        self.served.insert(path.to_string(), ServedVersion::new(now));
        representation
    }

    pub fn get(&self, path: &str) -> Option<&ServedVersion> {
        self.served.get(path)
    }

    pub fn len(&self) -> usize {
        self.served.len()
    }

    pub fn is_empty(&self) -> bool {
        self.served.is_empty()
    }
}
