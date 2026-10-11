use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::v4::call_graph::CallEdge;
use crate::v4::impact::{ImpactAnalyzer, ImpactResult};
use crate::v4::index_builder::SemanticIndexBuilder;
use crate::v4::retrieval::{SemanticRetrieval, SymbolSearchResult, SymbolTestResult};
use crate::v4::errors::V4Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindSymbolRequest {
    pub repo_root: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindCallersRequest {
    pub repo_root: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindCallersResponse {
    pub symbol: String,
    pub callers: Vec<CallEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindTestsRequest {
    pub repo_root: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffImpactRequest {
    pub repo_root: String,
    pub changed_path: String,
}

/// Normalises a path given by a caller to the repo-relative, forward-slash form used
/// by the semantic index. Accepts `src/x.rs`, `./src/x.rs`, and absolute paths under
/// `repo_root`. Paths that cannot be made relative are returned unchanged.
pub fn normalise_repo_path(repo_root: &str, changed: &str) -> String {
    let mut candidate = PathBuf::from(changed);
    if candidate.is_absolute() {
        let root = Path::new(repo_root);
        let stripped = candidate.strip_prefix(root).ok().map(Path::to_path_buf).or_else(|| {
            let canon_root = root.canonicalize().ok()?;
            let canon_file = candidate.canonicalize().ok()?;
            canon_file.strip_prefix(canon_root).ok().map(Path::to_path_buf)
        });
        match stripped {
            Some(relative) => candidate = relative,
            None => return changed.to_string(),
        }
    }

    let mut parts = Vec::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return changed.to_string(),
        }
    }
    if parts.is_empty() {
        changed.to_string()
    } else {
        parts.join("/")
    }
}

pub struct SemanticTools;

impl SemanticTools {
    pub fn find_symbol(req: FindSymbolRequest) -> V4Result<SymbolSearchResult> {
        let index = SemanticIndexBuilder::build(&req.repo_root)?;
        Ok(SemanticRetrieval::find_symbol(&index.symbols, &req.query))
    }

    pub fn find_callers(req: FindCallersRequest) -> V4Result<FindCallersResponse> {
        let index = SemanticIndexBuilder::build(&req.repo_root)?;
        Ok(FindCallersResponse {
            symbol: req.symbol.clone(),
            callers: index.calls.callers_of(&req.symbol),
        })
    }

    pub fn find_tests(req: FindTestsRequest) -> V4Result<SymbolTestResult> {
        let index = SemanticIndexBuilder::build(&req.repo_root)?;
        Ok(SemanticRetrieval::find_tests(&index.tests, &req.symbol))
    }

    pub fn diff_impact(req: DiffImpactRequest) -> V4Result<ImpactResult> {
        let index = SemanticIndexBuilder::build(&req.repo_root)?;
        let changed_path = normalise_repo_path(&req.repo_root, &req.changed_path);
        let affected_symbols = index
            .symbols
            .symbols
            .iter()
            .filter(|symbol| symbol.path == changed_path)
            .map(|symbol| symbol.name.clone())
            .collect::<Vec<_>>();

        let affected_imports = index
            .imports
            .edges
            .iter()
            .filter(|edge| edge.from_path == changed_path)
            .map(|edge| edge.import.clone())
            .collect::<Vec<_>>();

        let affected_tests = index
            .tests
            .references
            .iter()
            .filter(|reference| reference.test_path == changed_path)
            .map(|reference| reference.referenced_symbol.clone())
            .collect::<Vec<_>>();

        Ok(ImpactAnalyzer::build_result(
            changed_path,
            affected_symbols,
            affected_imports,
            affected_tests,
        ))
    }
}
