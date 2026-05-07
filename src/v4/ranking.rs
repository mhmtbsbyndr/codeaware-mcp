use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedPath {
    pub path: String,
    pub score: usize,
    pub reason: String,
}

pub struct ContextRanker;

impl ContextRanker {
    pub fn rank_paths(goal: &str, paths: Vec<String>) -> Vec<RankedPath> {
        let goal_terms: Vec<String> = goal
            .to_lowercase()
            .split_whitespace()
            .map(|term| term.to_string())
            .collect();

        let mut ranked: Vec<RankedPath> = paths
            .into_iter()
            .map(|path| {
                let lower = path.to_lowercase();

                let score = goal_terms
                    .iter()
                    .filter(|term| term.len() > 2)
                    .filter(|term| lower.contains(term.as_str()))
                    .count();

                RankedPath {
                    path,
                    score,
                    reason: if score > 0 {
                        "Matched goal keywords".to_string()
                    } else {
                        "Fallback candidate".to_string()
                    },
                }
            })
            .collect();

        ranked.sort_by(|a, b| b.score.cmp(&a.score));
        ranked
    }
}
