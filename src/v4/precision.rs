use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecisionMetrics {
    pub symbol_precision: f32,
    pub caller_precision: f32,
    pub import_precision: f32,
    pub semantic_context_precision: f32,
}

impl Default for PrecisionMetrics {
    fn default() -> Self {
        Self {
            symbol_precision: 0.0,
            caller_precision: 0.0,
            import_precision: 0.0,
            semantic_context_precision: 0.0,
        }
    }
}

pub struct PrecisionEvaluator;

impl PrecisionEvaluator {
    pub fn score(total: usize, correct: usize) -> f32 {
        if total == 0 {
            return 0.0;
        }

        correct as f32 / total as f32
    }
}
