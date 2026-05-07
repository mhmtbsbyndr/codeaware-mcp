use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResult {
    pub changed_path: String,
    pub affected_symbols: Vec<String>,
    pub affected_imports: Vec<String>,
    pub affected_tests: Vec<String>,
    pub risk_score: usize,
}

pub struct ImpactAnalyzer;

impl ImpactAnalyzer {
    pub fn estimate_risk(
        symbol_count: usize,
        import_count: usize,
        test_count: usize,
    ) -> usize {
        symbol_count * 3 + import_count * 2 + test_count
    }

    pub fn build_result(
        changed_path: impl Into<String>,
        affected_symbols: Vec<String>,
        affected_imports: Vec<String>,
        affected_tests: Vec<String>,
    ) -> ImpactResult {
        let risk_score = Self::estimate_risk(
            affected_symbols.len(),
            affected_imports.len(),
            affected_tests.len(),
        );

        ImpactResult {
            changed_path: changed_path.into(),
            affected_symbols,
            affected_imports,
            affected_tests,
            risk_score,
        }
    }
}
