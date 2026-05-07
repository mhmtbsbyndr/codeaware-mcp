use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelTier {
    Cheap,
    Balanced,
    Premium,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub complexity_score: usize,
    pub recommended_tier: ModelTier,
    pub reason: String,
}

pub struct SemanticRouter;

impl SemanticRouter {
    pub fn route(
        symbol_count: usize,
        call_count: usize,
        impact_score: usize,
    ) -> RoutingDecision {
        let complexity_score = symbol_count * 2 + call_count * 2 + impact_score;

        let recommended_tier = if complexity_score < 20 {
            ModelTier::Cheap
        } else if complexity_score < 80 {
            ModelTier::Balanced
        } else {
            ModelTier::Premium
        };

        let reason = match recommended_tier {
            ModelTier::Cheap => "Low semantic complexity.".to_string(),
            ModelTier::Balanced => "Moderate semantic complexity.".to_string(),
            ModelTier::Premium => "High semantic complexity and impact.".to_string(),
        };

        RoutingDecision {
            complexity_score,
            recommended_tier,
            reason,
        }
    }
}
