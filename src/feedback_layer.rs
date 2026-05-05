#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackEntry {
    pub session_id: String,
    pub rating: u8,
    pub comment: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackSummary {
    pub total_feedback: usize,
    pub positive: usize,
    pub neutral: usize,
    pub negative: usize,
}

pub fn validate_feedback_rating(rating: u8) -> bool {
    matches!(rating, 1..=3)
}

pub fn summarize_feedback(entries: &[FeedbackEntry]) -> FeedbackSummary {
    let positive = entries.iter().filter(|e| e.rating == 1).count();
    let neutral = entries.iter().filter(|e| e.rating == 2).count();
    let negative = entries.iter().filter(|e| e.rating == 3).count();

    FeedbackSummary {
        total_feedback: entries.len(),
        positive,
        neutral,
        negative,
    }
}

pub fn feedback_prompt() -> &'static str {
    "Was the session result correct?\n1. Good\n2. Partial issues\n3. Incorrect"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_feedback_range() {
        assert!(validate_feedback_rating(1));
        assert!(validate_feedback_rating(2));
        assert!(validate_feedback_rating(3));
        assert!(!validate_feedback_rating(0));
        assert!(!validate_feedback_rating(4));
    }

    #[test]
    fn summarizes_feedback_entries() {
        let entries = vec![
            FeedbackEntry {
                session_id: "s1".to_string(),
                rating: 1,
                comment: None,
                created_at: "2026-05-06".to_string(),
            },
            FeedbackEntry {
                session_id: "s2".to_string(),
                rating: 2,
                comment: None,
                created_at: "2026-05-06".to_string(),
            },
            FeedbackEntry {
                session_id: "s3".to_string(),
                rating: 3,
                comment: None,
                created_at: "2026-05-06".to_string(),
            },
        ];

        let summary = summarize_feedback(&entries);

        assert_eq!(summary.total_feedback, 3);
        assert_eq!(summary.positive, 1);
        assert_eq!(summary.neutral, 1);
        assert_eq!(summary.negative, 1);
    }
}
