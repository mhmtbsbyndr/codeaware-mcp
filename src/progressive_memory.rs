#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryIndexEntry {
    pub id: u64,
    pub title: String,
    pub memory_type: String,
    pub project: String,
    pub created_at: String,
    pub token_estimate: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryTimelineWindow {
    pub anchor_id: u64,
    pub before: Vec<MemoryIndexEntry>,
    pub after: Vec<MemoryIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryObservationDetail {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub citations: Vec<String>,
    pub private: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressiveMemoryPlan {
    pub layer_1_search: String,
    pub layer_2_timeline: String,
    pub layer_3_details: String,
    pub expected_savings_note: String,
}

pub fn progressive_memory_plan(query: &str) -> ProgressiveMemoryPlan {
    ProgressiveMemoryPlan {
        layer_1_search: format!("search compact index for '{query}'"),
        layer_2_timeline: "fetch timeline around selected observation IDs".to_string(),
        layer_3_details: "fetch full observations only for final filtered IDs".to_string(),
        expected_savings_note: "Progressive disclosure avoids loading full memories before filtering".to_string(),
    }
}

pub fn filter_private_observations(
    observations: &[MemoryObservationDetail],
) -> Vec<MemoryObservationDetail> {
    observations
        .iter()
        .filter(|observation| !observation.private && !contains_private_tag(&observation.body))
        .cloned()
        .collect()
}

pub fn contains_private_tag(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("<private>") && lower.contains("</private>")
}

pub fn build_memory_citation(id: u64) -> String {
    format!("memory://observation/{id}")
}

pub fn compact_memory_index(entries: &[MemoryIndexEntry], limit: usize) -> Vec<MemoryIndexEntry> {
    entries.iter().take(limit).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_progressive_plan() {
        let plan = progressive_memory_plan("auth bug");
        assert!(plan.layer_1_search.contains("auth bug"));
        assert!(plan.expected_savings_note.contains("Progressive"));
    }

    #[test]
    fn filters_private_observations() {
        let observations = vec![
            MemoryObservationDetail {
                id: 1,
                title: "Public".to_string(),
                body: "safe".to_string(),
                citations: vec![],
                private: false,
            },
            MemoryObservationDetail {
                id: 2,
                title: "Secret".to_string(),
                body: "<private>secret</private>".to_string(),
                citations: vec![],
                private: false,
            },
        ];
        let filtered = filter_private_observations(&observations);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, 1);
    }

    #[test]
    fn builds_memory_citation() {
        assert_eq!(build_memory_citation(42), "memory://observation/42");
    }
}
