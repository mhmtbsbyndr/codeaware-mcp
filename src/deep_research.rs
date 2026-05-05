use crate::symbol_index::{IndexedSymbol, SymbolIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchEvidence {
    pub file_path: String,
    pub symbol: String,
    pub trust: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepResearchRequest {
    pub question: String,
    pub scope: Option<String>,
    pub include_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepResearchResponse {
    pub answer: String,
    pub evidence: Vec<ResearchEvidence>,
    pub open_questions: Vec<String>,
    pub suggested_next: Vec<String>,
}

pub fn run_deep_research(
    index: &SymbolIndex,
    request: &DeepResearchRequest,
) -> DeepResearchResponse {
    let matches = find_relevant_symbols(index, &request.question);

    let evidence: Vec<ResearchEvidence> = matches
        .iter()
        .map(build_evidence)
        .collect();

    let answer = if evidence.is_empty() {
        format!(
            "No direct structural evidence found for query: {}",
            request.question
        )
    } else {
        format!(
            "Found {} structurally relevant symbols for query: {}",
            evidence.len(),
            request.question
        )
    };

    DeepResearchResponse {
        answer,
        evidence,
        open_questions: vec![
            "Semantic runtime behavior is not yet inferred".to_string(),
            "LSP enrichment is not yet enabled".to_string(),
        ],
        suggested_next: vec![
            "query_symbols".to_string(),
            "get_references".to_string(),
            "smart_read".to_string(),
        ],
    }
}

fn find_relevant_symbols(index: &SymbolIndex, query: &str) -> Vec<IndexedSymbol> {
    index.query_symbols(query)
}

fn build_evidence(symbol: &IndexedSymbol) -> ResearchEvidence {
    ResearchEvidence {
        file_path: symbol.file_path.clone(),
        symbol: symbol.qualified_name.clone(),
        trust: format!("{:?}", symbol.trust),
        summary: format!(
            "{} defined in {} lines {}-{}",
            symbol.name, symbol.file_path, symbol.start_line, symbol.end_line
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol_index::{IndexedSymbol, SymbolKind, SymbolTrust};

    #[test]
    fn returns_structural_research_result() {
        let mut index = SymbolIndex::new();

        index.upsert_symbol(IndexedSymbol {
            id: "s1".to_string(),
            file_path: "src/auth.rs".to_string(),
            name: "authenticate".to_string(),
            qualified_name: "crate::auth::authenticate".to_string(),
            kind: SymbolKind::Function,
            start_line: 10,
            end_line: 40,
            trust: SymbolTrust::Structural,
        });

        let response = run_deep_research(
            &index,
            &DeepResearchRequest {
                question: "auth".to_string(),
                scope: None,
                include_evidence: true,
            },
        );

        assert_eq!(response.evidence.len(), 1);
        assert!(response.answer.contains("structurally relevant symbols"));
    }
}
