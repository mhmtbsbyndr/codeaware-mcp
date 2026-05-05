#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityRating {
    Good,
    Warning,
    Bad,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompressionPipeline {
    AstDiffOnly,
    GitDiffOnly,
    FullAstContext,
    NoCompression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunSummary {
    pub command: String,
    pub exit_code: i32,
    pub passed: u64,
    pub failed: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionQuality {
    pub session_id: String,
    pub start_tokens: u64,
    pub end_tokens: u64,
    pub compression_pipeline: CompressionPipeline,
    pub quality_rating: QualityRating,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityEvaluation {
    pub rating: QualityRating,
    pub summary: String,
}

pub fn evaluate_quality(summary: &TestRunSummary) -> QualityEvaluation {
    if summary.exit_code != 0 || summary.failed > 0 {
        return QualityEvaluation {
            rating: QualityRating::Bad,
            summary: format!(
                "Tests failed: {} failed / {} passed",
                summary.failed, summary.passed
            ),
        };
    }

    if summary.duration_ms > 120_000 {
        return QualityEvaluation {
            rating: QualityRating::Warning,
            summary: format!(
                "Tests passed but execution time is high: {} ms",
                summary.duration_ms
            ),
        };
    }

    QualityEvaluation {
        rating: QualityRating::Good,
        summary: format!(
            "Tests passed successfully: {} passed / {} failed",
            summary.passed, summary.failed
        ),
    }
}

pub const TOKEN_QUALITY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS token_quality (
    session_id TEXT PRIMARY KEY,
    start_tokens INTEGER NOT NULL,
    end_tokens INTEGER NOT NULL,
    compression_pipeline TEXT NOT NULL,
    quality_rating TEXT NOT NULL,
    feedback TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS token_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    rating INTEGER NOT NULL,
    comment TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_token_quality_pipeline ON token_quality(compression_pipeline);
CREATE INDEX IF NOT EXISTS idx_token_quality_rating ON token_quality(quality_rating);
CREATE INDEX IF NOT EXISTS idx_token_feedback_session ON token_feedback(session_id);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_failed_tests_as_bad() {
        let result = evaluate_quality(&TestRunSummary {
            command: "cargo test".to_string(),
            exit_code: 1,
            passed: 10,
            failed: 2,
            duration_ms: 1000,
        });

        assert_eq!(result.rating, QualityRating::Bad);
    }

    #[test]
    fn marks_slow_tests_as_warning() {
        let result = evaluate_quality(&TestRunSummary {
            command: "cargo test".to_string(),
            exit_code: 0,
            passed: 20,
            failed: 0,
            duration_ms: 180_000,
        });

        assert_eq!(result.rating, QualityRating::Warning);
    }

    #[test]
    fn marks_clean_tests_as_good() {
        let result = evaluate_quality(&TestRunSummary {
            command: "cargo test".to_string(),
            exit_code: 0,
            passed: 42,
            failed: 0,
            duration_ms: 1500,
        });

        assert_eq!(result.rating, QualityRating::Good);
    }

    #[test]
    fn schema_contains_quality_tables() {
        assert!(TOKEN_QUALITY_SCHEMA.contains("token_quality"));
        assert!(TOKEN_QUALITY_SCHEMA.contains("token_feedback"));
    }
}
