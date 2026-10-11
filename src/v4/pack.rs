//! Value-per-token packing of context items under a token budget.
//!
//! Every candidate can be sent in up to three representations: the full body, a skeleton
//! (signatures only) or a one-line summary. Each (candidate, representation) pair is an
//! option with cost = estimated tokens and value = relevance * fidelity. The packer
//! picks at most one option per candidate. The pass order is:
//!
//! 1. Greedy by density (value / cost), which gives every relevant item at least its
//!    cheapest representation before any item gets a large one.
//! 2. Upgrade pass in relevance order: switch an item to a higher-fidelity option when the
//!    extra cost still fits.
//! 3. Final check: if the single most valuable option that fits beats the whole greedy
//!    selection, return only that option (the classic guard for greedy knapsack).
//!
//! All sorts use explicit tie-breaks, so output is deterministic.

use serde::{Deserialize, Serialize};

use crate::v4::tokens::estimate_tokens;

/// Value multiplier for a skeleton (signatures only) representation.
pub const SKELETON_FIDELITY: f64 = 0.6;
/// Value multiplier for a one-line summary representation.
pub const ONE_LINE_FIDELITY: f64 = 0.3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "text", rename_all = "snake_case")]
pub enum Representation {
    /// Full body of a file or symbol.
    Full(String),
    /// Signatures only.
    Skeleton(String),
    /// One-line summary.
    OneLine(String),
    /// Marker that content identical to what was already served is still in context.
    Unchanged(String),
    /// Unified diff against the version that was served earlier.
    Diff(String),
}

impl Representation {
    pub fn text(&self) -> &str {
        match self {
            Self::Full(text)
            | Self::Skeleton(text)
            | Self::OneLine(text)
            | Self::Unchanged(text)
            | Self::Diff(text) => text,
        }
    }

    pub fn estimated_tokens(&self) -> usize {
        estimate_tokens(self.text())
    }

    /// Share of the candidate's relevance that this representation keeps.
    pub fn fidelity(&self) -> f64 {
        match self {
            Self::Full(_) | Self::Diff(_) | Self::Unchanged(_) => 1.0,
            Self::Skeleton(_) => SKELETON_FIDELITY,
            Self::OneLine(_) => ONE_LINE_FIDELITY,
        }
    }
}

/// One item to pack. `one_line` is mandatory; `full` and `skeleton` are optional.
#[derive(Debug, Clone)]
pub struct PackCandidate {
    pub id: String,
    pub relevance: f64,
    pub full: Option<String>,
    pub skeleton: Option<String>,
    pub one_line: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackMode {
    /// Relevance order, full (or best available) representation, stop at the first item that
    /// does not fit. This mirrors the prefix fill used before v4 packing.
    #[default]
    Legacy,
    /// Value-per-token knapsack over all representations.
    Knapsack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackConfig {
    pub mode: PackMode,
    pub budget_tokens: usize,
    /// Stop after the first candidate whose relevance is below `fraction * top relevance`.
    /// `None` disables the score-gap cut.
    pub early_stop_fraction: Option<f64>,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            mode: PackMode::Legacy,
            budget_tokens: 8_000,
            early_stop_fraction: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedItem {
    pub id: String,
    pub relevance: f64,
    pub representation: Representation,
    pub value: f64,
    pub tokens: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackResult {
    /// Selected items, by relevance (desc) then id (asc).
    pub items: Vec<PackedItem>,
    pub total_tokens: usize,
    pub total_value: f64,
    /// Candidates removed by the score-gap cut before packing.
    pub cut_by_score_gap: usize,
    /// Candidates that were eligible but did not fit the budget.
    pub skipped: Vec<String>,
}

/// Number of leading items to keep when `scores_desc` is sorted best first. Keeps items while
/// their score is at least `fraction * top`. A non-positive top score keeps nothing.
pub fn score_gap_cutoff(scores_desc: &[f64], fraction: f64) -> usize {
    let Some(&top) = scores_desc.first() else {
        return 0;
    };
    if top <= 0.0 || !top.is_finite() {
        return 0;
    }
    let threshold = top * fraction.clamp(0.0, 1.0);
    scores_desc.iter().take_while(|score| **score >= threshold).count()
}

/// Packs candidates under `config.budget_tokens`.
///
/// Candidates with a non-positive or non-finite relevance are never selected.
pub fn pack(config: &PackConfig, candidates: &[PackCandidate]) -> PackResult {
    let mut eligible: Vec<&PackCandidate> = candidates
        .iter()
        .filter(|candidate| candidate.relevance.is_finite() && candidate.relevance > 0.0)
        .collect();
    eligible.sort_by(|a, b| b.relevance.total_cmp(&a.relevance).then_with(|| a.id.cmp(&b.id)));

    let mut cut_by_score_gap = 0;
    if let Some(fraction) = config.early_stop_fraction {
        let scores: Vec<f64> = eligible.iter().map(|candidate| candidate.relevance).collect();
        let keep = score_gap_cutoff(&scores, fraction);
        cut_by_score_gap = eligible.len() - keep;
        eligible.truncate(keep);
    }

    let mut result = match config.mode {
        PackMode::Legacy => pack_legacy(&eligible, config.budget_tokens),
        PackMode::Knapsack => pack_knapsack(&eligible, config.budget_tokens),
    };
    result.cut_by_score_gap = cut_by_score_gap;
    result.skipped = eligible
        .iter()
        .filter(|candidate| !result.items.iter().any(|item| item.id == candidate.id))
        .map(|candidate| candidate.id.clone())
        .collect();
    result
}

/// Representations available for a candidate, best fidelity first.
fn options_for(candidate: &PackCandidate) -> Vec<Representation> {
    let mut options = Vec::with_capacity(3);
    if let Some(full) = candidate.full.as_ref().filter(|text| !text.is_empty()) {
        options.push(Representation::Full(full.clone()));
    }
    if let Some(skeleton) = candidate.skeleton.as_ref().filter(|text| !text.is_empty()) {
        options.push(Representation::Skeleton(skeleton.clone()));
    }
    options.push(Representation::OneLine(candidate.one_line.clone()));
    options
}

fn build(candidate: &PackCandidate, representation: Representation) -> PackedItem {
    let tokens = representation.estimated_tokens();
    PackedItem {
        id: candidate.id.clone(),
        relevance: candidate.relevance,
        value: candidate.relevance * representation.fidelity(),
        tokens,
        representation,
    }
}

/// Relevance order; full (else skeleton, else one-line); stop at the first that does not fit.
fn pack_legacy(eligible: &[&PackCandidate], budget: usize) -> PackResult {
    let mut items = Vec::new();
    let mut used = 0;
    for candidate in eligible {
        let representation = options_for(candidate).remove(0);
        let item = build(candidate, representation);
        if used + item.tokens > budget {
            break;
        }
        used += item.tokens;
        items.push(item);
    }
    finish(items)
}

#[derive(Clone)]
struct PackOption {
    candidate: usize,
    item: PackedItem,
    density: f64,
    fidelity_rank: usize,
}

fn pack_knapsack(eligible: &[&PackCandidate], budget: usize) -> PackResult {
    let mut options: Vec<PackOption> = Vec::new();
    for (ci, candidate) in eligible.iter().enumerate() {
        for (rank, representation) in options_for(candidate).into_iter().enumerate() {
            let item = build(candidate, representation);
            let cost = item.tokens.max(1) as f64;
            options.push(PackOption {
                candidate: ci,
                density: item.value / cost,
                fidelity_rank: rank,
                item,
            });
        }
    }
    // Density desc, value desc, candidate order, then fidelity (best first).
    options.sort_by(|a, b| {
        b.density
            .total_cmp(&a.density)
            .then_with(|| b.item.value.total_cmp(&a.item.value))
            .then_with(|| a.candidate.cmp(&b.candidate))
            .then_with(|| a.fidelity_rank.cmp(&b.fidelity_rank))
    });

    // Pass 1: greedy by density. An option that does not fit now never fits later, because
    // used tokens only grow.
    let mut chosen: Vec<Option<usize>> = vec![None; eligible.len()];
    let mut used = 0usize;
    for (oi, option) in options.iter().enumerate() {
        if chosen[option.candidate].is_some() {
            continue;
        }
        if used + option.item.tokens <= budget {
            used += option.item.tokens;
            chosen[option.candidate] = Some(oi);
        }
    }

    // Pass 2: upgrade in relevance order (candidates are already sorted that way).
    for ci in 0..eligible.len() {
        let Some(current) = chosen[ci] else {
            continue;
        };
        let current_tokens = options[current].item.tokens;
        let current_value = options[current].item.value;
        let mut upgrades: Vec<usize> = (0..options.len())
            .filter(|&oi| options[oi].candidate == ci && options[oi].item.value > current_value)
            .collect();
        upgrades.sort_by(|&a, &b| options[b].item.value.total_cmp(&options[a].item.value));
        for oi in upgrades {
            let tokens = options[oi].item.tokens;
            if used - current_tokens + tokens <= budget {
                used = used - current_tokens + tokens;
                chosen[ci] = Some(oi);
                break;
            }
        }
    }

    let greedy: Vec<PackedItem> = chosen
        .iter()
        .flatten()
        .map(|&oi| options[oi].item.clone())
        .collect();
    let greedy_value: f64 = greedy.iter().map(|item| item.value).sum();

    // Pass 3: final check against the best single option that fits on its own.
    let mut best_single: Option<&PackOption> = None;
    for option in &options {
        if option.item.tokens > budget {
            continue;
        }
        let better = match best_single {
            None => true,
            Some(best) => option.item.value > best.item.value,
        };
        if better {
            best_single = Some(option);
        }
    }
    if let Some(single) = best_single.filter(|single| single.item.value > greedy_value) {
        return finish(vec![single.item.clone()]);
    }

    finish(greedy)
}

fn finish(mut items: Vec<PackedItem>) -> PackResult {
    items.sort_by(|a, b| b.relevance.total_cmp(&a.relevance).then_with(|| a.id.cmp(&b.id)));
    let total_tokens = items.iter().map(|item| item.tokens).sum();
    let total_value = items.iter().map(|item| item.value).sum();
    PackResult {
        items,
        total_tokens,
        total_value,
        cut_by_score_gap: 0,
        skipped: Vec::new(),
    }
}
