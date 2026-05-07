pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let chars = text.chars().count();
    let whitespace = text.chars().filter(|ch| ch.is_whitespace()).count();
    let punctuation = text.chars().filter(|ch| ch.is_ascii_punctuation()).count();

    let rough = chars / 4;
    let structure_overhead = (whitespace + punctuation) / 12;

    rough.saturating_add(structure_overhead).max(1)
}

pub fn estimate_path_relevance(path: &str, goal: &str) -> usize {
    let path_lower = path.to_lowercase();
    let goal_lower = goal.to_lowercase();

    goal_lower
        .split_whitespace()
        .filter(|term| term.len() > 2)
        .filter(|term| path_lower.contains(*term))
        .count()
}
