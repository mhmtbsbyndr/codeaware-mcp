//! Tokenizer for skill metadata and goals: identifier splitting, lowercasing,
//! stopword removal and a small suffix stemmer.

const STOPWORDS: &[&str] = &[
    "a", "about", "all", "an", "and", "any", "are", "as", "at", "be", "by", "can", "do", "for",
    "from", "has", "have", "how", "i", "if", "in", "into", "is", "it", "its", "not", "of", "on",
    "or", "our", "so", "that", "the", "this", "to", "use", "used", "using", "via", "we", "when",
    "where", "which", "will", "with", "you", "your",
];

/// Lowercased, split, stopword-free, stemmed tokens of `text`.
///
/// Identifiers are split on non-alphanumeric characters, camelCase boundaries
/// and letter/digit boundaries, so `smart_read`, `smart-read` and `SmartRead`
/// all yield `["smart", "read"]`.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for run in text.split(|c: char| !c.is_alphanumeric()) {
        if run.is_empty() {
            continue;
        }
        for part in split_identifier(run) {
            let lower = part.to_lowercase();
            if lower.chars().count() < 2 || lower.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            if STOPWORDS.contains(&lower.as_str()) {
                continue;
            }
            out.push(stem(&lower));
        }
    }
    out
}

fn split_identifier(run: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut prev: Option<char> = None;
    for (idx, cur) in run.char_indices() {
        if let Some(p) = prev {
            let camel = p.is_lowercase() && cur.is_uppercase();
            let letter_digit = p.is_alphabetic() != cur.is_alphabetic();
            if camel || letter_digit {
                parts.push(&run[start..idx]);
                start = idx;
            }
        }
        prev = Some(cur);
    }
    parts.push(&run[start..]);
    parts
}

/// Minimal English suffix stripping. Applied identically to documents and
/// queries, so it only needs to be consistent, not linguistically exact.
fn stem(word: &str) -> String {
    let n = word.len();
    if n > 4 && word.ends_with("ies") {
        return format!("{}y", &word[..n - 3]);
    }
    if n > 5 && word.ends_with("ing") {
        return word[..n - 3].to_string();
    }
    if n > 4 && word.ends_with("ed") {
        return word[..n - 2].to_string();
    }
    if n > 3
        && word.ends_with('s')
        && !word.ends_with("ss")
        && !word.ends_with("us")
        && !word.ends_with("is")
    {
        return word[..n - 1].to_string();
    }
    word.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_identifiers_in_all_styles() {
        let expected = vec!["smart", "read"];
        assert_eq!(tokenize("smart_read"), expected);
        assert_eq!(tokenize("smart-read"), expected);
        assert_eq!(tokenize("SmartRead"), expected);
    }

    #[test]
    fn splits_letter_digit_boundary() {
        assert_eq!(tokenize("pdf2text"), vec!["pdf", "text"]);
    }

    #[test]
    fn drops_stopwords_and_numbers() {
        assert_eq!(tokenize("Use the 2024 report"), vec!["report"]);
    }

    #[test]
    fn stems_plurals_consistently() {
        assert_eq!(tokenize("files"), tokenize("file"));
        assert_eq!(tokenize("policies"), tokenize("policy"));
        assert_eq!(tokenize("process"), vec!["process"]);
    }
}
