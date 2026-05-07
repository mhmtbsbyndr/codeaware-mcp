use std::fs;
use std::path::Path;

use crate::v4::ast::AstExtractor;
use crate::v4::call_graph::HeuristicCallExtractor;
use crate::v4::discovery::{CandidateDiscovery, DiscoveryConfig};
use crate::v4::errors::{V4Error, V4Result};
use crate::v4::import_graph::RustImportExtractor;
use crate::v4::semantic_index::SemanticIndex;
use crate::v4::tests_graph::RustTestExtractor;

pub struct SemanticIndexBuilder;

impl SemanticIndexBuilder {
    pub fn build(repo_root: impl AsRef<Path>) -> V4Result<SemanticIndex> {
        let repo_root = repo_root.as_ref();

        let ranked = CandidateDiscovery::discover_ranked(
            repo_root,
            "semantic index",
            DiscoveryConfig {
                max_candidates: 500,
                ..Default::default()
            },
        )?;

        let mut index = SemanticIndex::default();

        for candidate in ranked {
            let full_path = repo_root.join(&candidate.path);

            let Ok(content) = fs::read_to_string(&full_path) else {
                continue;
            };

            if candidate.path.ends_with(".rs") {
                let symbols = AstExtractor::extract_rust_symbols(&candidate.path, &content);
                index.symbols.symbols.extend(symbols.symbols);

                let imports = RustImportExtractor::extract(&candidate.path, &content);
                index.imports.edges.extend(imports.edges);

                let calls = HeuristicCallExtractor::extract(&candidate.path, &content);
                index.calls.edges.extend(calls.edges);

                let tests = RustTestExtractor::extract(&candidate.path, &content);
                index.tests.references.extend(tests.references);
            }
        }

        Ok(index)
    }

    pub fn persist(index: &SemanticIndex, path: impl AsRef<Path>) -> V4Result<()> {
        let json = serde_json::to_string_pretty(index)
            .map_err(|err| V4Error::Serialization(err.to_string()))?;

        fs::write(path, json).map_err(|err| V4Error::Io(err.to_string()))?;

        Ok(())
    }
}
