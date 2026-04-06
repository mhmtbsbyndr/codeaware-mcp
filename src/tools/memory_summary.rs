use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::envelope::{Envelope, ErrorCode, TrustLevel};
use crate::session::persistence::{ObservationRecord, SessionDb};

// ── Public result type ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SummarizeResult {
    pub clusters: usize,
    pub summaries_created: usize,
    pub duplicates_removed: usize,
}

// ── Internal cluster type ───────────────────────────────────────────

struct Cluster {
    observations: Vec<ObservationRecord>,
    topic: String,
}

// ── Jaccard similarity ──────────────────────────────────────────────

fn parse_files(files: &Option<String>) -> HashSet<String> {
    files
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_concepts(concepts: &Option<String>) -> HashSet<String> {
    concepts
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

// ── Clustering (Jaccard on files, sub-cluster by concepts) ──────────

fn cluster_observations(observations: Vec<ObservationRecord>) -> Vec<Cluster> {
    if observations.is_empty() {
        return vec![];
    }

    let file_sets: Vec<HashSet<String>> = observations.iter().map(|o| parse_files(&o.files)).collect();
    let concept_sets: Vec<HashSet<String>> = observations.iter().map(|o| parse_concepts(&o.concepts)).collect();

    let n = observations.len();
    let mut assigned = vec![false; n];
    let mut clusters: Vec<Vec<usize>> = Vec::new();

    // Greedy file-based clustering: O(n^2), fine for <1000
    for i in 0..n {
        if assigned[i] {
            continue;
        }
        let mut group = vec![i];
        assigned[i] = true;

        // Only cluster by files if observation has files
        if !file_sets[i].is_empty() {
            for j in (i + 1)..n {
                if assigned[j] {
                    continue;
                }
                if file_sets[j].is_empty() {
                    continue;
                }
                if jaccard(&file_sets[i], &file_sets[j]) > 0.3 {
                    group.push(j);
                    assigned[j] = true;
                }
            }
        }
        clusters.push(group);
    }

    // Sub-cluster by concepts within each file-cluster
    let mut result: Vec<Cluster> = Vec::new();
    // We need to move observations into clusters. Convert to indexed access.
    // Since we reference observations by index, we wrap them in Option for taking.
    let mut obs_slots: Vec<Option<ObservationRecord>> = observations.into_iter().map(Some).collect();

    for group in &clusters {
        if group.len() <= 1 {
            // Single-element cluster, no sub-clustering needed
            let idx = group[0];
            if let Some(obs) = obs_slots[idx].take() {
                let topic = derive_topic(&[&obs], &file_sets, &concept_sets, &[idx]);
                result.push(Cluster {
                    observations: vec![obs],
                    topic,
                });
            }
            continue;
        }

        // Sub-cluster by concepts
        let mut sub_assigned = vec![false; group.len()];
        let mut sub_clusters: Vec<Vec<usize>> = Vec::new();

        for gi in 0..group.len() {
            if sub_assigned[gi] {
                continue;
            }
            let mut sub_group = vec![gi];
            sub_assigned[gi] = true;
            let ci = group[gi];

            if !concept_sets[ci].is_empty() {
                for gj in (gi + 1)..group.len() {
                    if sub_assigned[gj] {
                        continue;
                    }
                    let cj = group[gj];
                    if !concept_sets[cj].is_empty() && jaccard(&concept_sets[ci], &concept_sets[cj]) > 0.3 {
                        sub_group.push(gj);
                        sub_assigned[gj] = true;
                    }
                }
            }
            sub_clusters.push(sub_group);
        }

        for sub in &sub_clusters {
            let indices: Vec<usize> = sub.iter().map(|&gi| group[gi]).collect();
            let obs_list: Vec<ObservationRecord> = indices
                .iter()
                .filter_map(|&idx| obs_slots[idx].take())
                .collect();
            let obs_refs: Vec<&ObservationRecord> = obs_list.iter().collect();
            let topic = derive_topic(&obs_refs, &file_sets, &concept_sets, &indices);
            result.push(Cluster {
                observations: obs_list,
                topic,
            });
        }
    }

    result
}

fn derive_topic(
    _obs: &[&ObservationRecord],
    file_sets: &[HashSet<String>],
    concept_sets: &[HashSet<String>],
    indices: &[usize],
) -> String {
    // Most common file path stem
    let mut file_freq: HashMap<String, usize> = HashMap::new();
    for &idx in indices {
        for f in &file_sets[idx] {
            let stem = std::path::Path::new(f)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(f)
                .to_string();
            *file_freq.entry(stem).or_insert(0) += 1;
        }
    }

    // Most common concept tag
    let mut concept_freq: HashMap<String, usize> = HashMap::new();
    for &idx in indices {
        for c in &concept_sets[idx] {
            *concept_freq.entry(c.clone()).or_insert(0) += 1;
        }
    }

    let top_file = file_freq
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(k, _)| k.clone());
    let top_concept = concept_freq
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(k, _)| k.clone());

    match (top_file, top_concept) {
        (Some(f), Some(c)) => format!("{} ({})", f, c),
        (Some(f), None) => f,
        (None, Some(c)) => c,
        (None, None) => "uncategorized".to_string(),
    }
}

// ── Deduplication (4-gram shingle overlap) ──────────────────────────

fn compute_shingles(text: &str, n: usize) -> HashSet<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < n {
        let mut set = HashSet::new();
        set.insert(words.join(" ").to_lowercase());
        return set;
    }
    words
        .windows(n)
        .map(|w| w.join(" ").to_lowercase())
        .collect()
}

fn shingle_overlap(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let min_len = a.len().min(b.len()) as f64;
    if min_len == 0.0 {
        0.0
    } else {
        intersection / min_len
    }
}

/// Returns IDs of duplicate observations to remove (the older one in each pair).
fn deduplicate_cluster(observations: &[ObservationRecord]) -> Vec<i64> {
    let mut duplicates: HashSet<i64> = HashSet::new();
    let shingles: Vec<HashSet<String>> = observations
        .iter()
        .map(|o| compute_shingles(&o.text, 4))
        .collect();

    for i in 0..observations.len() {
        if duplicates.contains(&observations[i].id) {
            continue;
        }
        for j in (i + 1)..observations.len() {
            if duplicates.contains(&observations[j].id) {
                continue;
            }
            if shingle_overlap(&shingles[i], &shingles[j]) > 0.7 {
                // Mark the older one (lower id) as duplicate
                let older_id = observations[i].id.min(observations[j].id);
                duplicates.insert(older_id);
            }
        }
    }

    duplicates.into_iter().collect()
}

// ── Summary generation (TF-IDF-like sentence scoring) ───────────────

fn generate_summary(observations: &[ObservationRecord]) -> String {
    if observations.is_empty() {
        return String::new();
    }

    // Split all texts into sentences
    let mut all_sentences: Vec<(String, usize)> = Vec::new(); // (sentence, obs_index)
    for (idx, obs) in observations.iter().enumerate() {
        for sentence in split_sentences(&obs.text) {
            let trimmed = sentence.trim().to_string();
            if !trimmed.is_empty() && trimmed.split_whitespace().count() >= 3 {
                all_sentences.push((trimmed, idx));
            }
        }
    }

    if all_sentences.is_empty() {
        // Fallback: just join the first few texts
        return observations
            .iter()
            .take(3)
            .map(|o| o.text.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
    }

    // Count in how many observations each word appears (document frequency)
    let total_obs = observations.len() as f64;
    let mut doc_freq: HashMap<String, HashSet<usize>> = HashMap::new();
    for (idx, obs) in observations.iter().enumerate() {
        for word in obs.text.split_whitespace() {
            let normalized = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            if normalized.len() >= 2 {
                doc_freq.entry(normalized).or_default().insert(idx);
            }
        }
    }

    // Score each sentence: words appearing in fewer observations score higher (IDF)
    let mut scored: Vec<(f64, &str)> = all_sentences
        .iter()
        .map(|(sentence, _)| {
            let words: Vec<String> = sentence
                .split_whitespace()
                .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| w.len() >= 2)
                .collect();
            if words.is_empty() {
                return (0.0, sentence.as_str());
            }
            let score: f64 = words
                .iter()
                .map(|w| {
                    let df = doc_freq.get(w).map_or(1, |s| s.len()) as f64;
                    // IDF-like: rarer words get higher scores
                    (total_obs / df).ln() + 1.0
                })
                .sum::<f64>()
                / words.len() as f64;
            (score, sentence.as_str())
        })
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Pick top 3 unique sentences
    let mut chosen: Vec<&str> = Vec::new();
    for (_, sentence) in &scored {
        if chosen.len() >= 3 {
            break;
        }
        // Avoid near-duplicate sentences in summary
        if !chosen.iter().any(|&c| sentences_similar(c, sentence)) {
            chosen.push(sentence);
        }
    }

    chosen.join(" | ")
}

fn split_sentences(text: &str) -> Vec<&str> {
    // Simple sentence splitter: split on . ! ? followed by space or end
    let mut sentences = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if (bytes[i] == b'.' || bytes[i] == b'!' || bytes[i] == b'?')
            && (i + 1 >= bytes.len() || bytes[i + 1] == b' ' || bytes[i + 1] == b'\n')
        {
            let end = i + 1;
            if end > start {
                sentences.push(&text[start..end]);
            }
            start = end;
            // Skip whitespace after sentence terminator
            while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\n') {
                start += 1;
            }
        }
    }
    // Remaining text
    if start < text.len() {
        sentences.push(&text[start..]);
    }
    sentences
}

fn sentences_similar(a: &str, b: &str) -> bool {
    let sa = compute_shingles(a, 3);
    let sb = compute_shingles(b, 3);
    shingle_overlap(&sa, &sb) > 0.6
}

// ── Handler ─────────────────────────────────────────────────────────

pub fn handle_summarize_memory(params: &Value, db: &SessionDb) -> Value {
    let project = params
        .get("project")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let _force = params
        .get("force")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // 1. Fetch all observations for project
    let observations = match db.get_all_observations_for_project(project) {
        Ok(obs) => obs,
        Err(e) => {
            let env = Envelope::<()>::error(
                ErrorCode::EInternalError,
                true,
                Some(format!("Failed to load observations: {}", e)),
            );
            return serde_json::to_value(env).unwrap_or(serde_json::json!({"ok": false}));
        }
    };

    if observations.is_empty() {
        let env = Envelope::success(
            SummarizeResult {
                clusters: 0,
                summaries_created: 0,
                duplicates_removed: 0,
            },
            TrustLevel::Exact,
        );
        return serde_json::to_value(env).unwrap_or(serde_json::json!({"ok": false}));
    }

    // 2. Cluster
    let clusters = cluster_observations(observations);
    let cluster_count = clusters.len();

    // 3. Dedup + summary for each cluster
    let mut total_duplicates_removed: usize = 0;
    let mut summaries_created: usize = 0;

    for cluster in &clusters {
        // Deduplication
        let dup_ids = deduplicate_cluster(&cluster.observations);
        if !dup_ids.is_empty() {
            match db.delete_observations_by_ids(&dup_ids) {
                Ok(count) => total_duplicates_removed += count,
                Err(e) => {
                    eprintln!("Warning: failed to delete duplicates: {}", e);
                }
            }
        }

        // Filter out duplicates for summary generation
        let non_dup_obs: Vec<&ObservationRecord> = cluster
            .observations
            .iter()
            .filter(|o| !dup_ids.contains(&o.id))
            .collect();

        if non_dup_obs.is_empty() {
            continue;
        }

        // Generate summary
        let obs_for_summary: Vec<ObservationRecord> = non_dup_obs
            .iter()
            .map(|o| ObservationRecord {
                id: o.id,
                title: o.title.clone(),
                text: o.text.clone(),
                observation_type: o.observation_type.clone(),
                concepts: o.concepts.clone(),
                project: o.project.clone(),
                files: o.files.clone(),
                facts: o.facts.clone(),
                created_at: o.created_at.clone(),
                updated_at: o.updated_at.clone(),
            })
            .collect();
        let summary_text = generate_summary(&obs_for_summary);
        let obs_ids_str = non_dup_obs
            .iter()
            .map(|o| o.id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        match db.save_summary(project, &cluster.topic, &summary_text, &obs_ids_str) {
            Ok(_) => summaries_created += 1,
            Err(e) => {
                eprintln!("Warning: failed to save summary: {}", e);
            }
        }
    }

    let result = SummarizeResult {
        clusters: cluster_count,
        summaries_created,
        duplicates_removed: total_duplicates_removed,
    };
    let env = Envelope::success(result, TrustLevel::Heuristic);
    serde_json::to_value(env).unwrap_or(serde_json::json!({"ok": false}))
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jaccard_identical() {
        let a: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert!((jaccard(&a, &a) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a: HashSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        let b: HashSet<String> = ["c", "d"].iter().map(|s| s.to_string()).collect();
        assert!((jaccard(&a, &b)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_partial() {
        let a: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let b: HashSet<String> = ["b", "c", "d"].iter().map(|s| s.to_string()).collect();
        // intersection={b,c}=2, union={a,b,c,d}=4 → 0.5
        assert!((jaccard(&a, &b) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_shingle_computation() {
        let shingles = compute_shingles("the quick brown fox jumps", 4);
        assert_eq!(shingles.len(), 2);
        assert!(shingles.contains("the quick brown fox"));
        assert!(shingles.contains("quick brown fox jumps"));
    }

    #[test]
    fn test_shingle_overlap_identical() {
        let s = compute_shingles("the quick brown fox jumps over", 4);
        assert!((shingle_overlap(&s, &s) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_split_sentences() {
        let text = "First sentence. Second sentence! Third? More text";
        let sentences = split_sentences(text);
        assert_eq!(sentences.len(), 4);
        assert_eq!(sentences[0], "First sentence.");
        assert_eq!(sentences[1], "Second sentence!");
        assert_eq!(sentences[2], "Third?");
        assert_eq!(sentences[3], "More text");
    }

    #[test]
    fn test_generate_summary_empty() {
        let obs: Vec<ObservationRecord> = vec![];
        let summary = generate_summary(&obs);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_parse_files() {
        let files = Some("src/auth.rs, src/main.rs, lib.rs".to_string());
        let set = parse_files(&files);
        assert_eq!(set.len(), 3);
        assert!(set.contains("src/auth.rs"));
        assert!(set.contains("src/main.rs"));
        assert!(set.contains("lib.rs"));
    }

    #[test]
    fn test_parse_files_empty() {
        let set = parse_files(&None);
        assert!(set.is_empty());
    }
}
