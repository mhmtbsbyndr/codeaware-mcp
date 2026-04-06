use std::collections::BTreeMap;

/// Maximum matches shown per file before collapsing.
const MAX_MATCHES_PER_FILE: usize = 2;

pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    // Group results by file path.
    // Search tools (rg, grep) typically output "file:line:content" or "file:content".
    let mut file_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();

    for line in &lines {
        let file_key = extract_file_prefix(line);
        if !file_groups.contains_key(&file_key) {
            order.push(file_key.clone());
        }
        file_groups.entry(file_key).or_default().push(line.to_string());
    }

    let mut result: Vec<String> = Vec::new();
    let mut total_lines_used = 0usize;

    for file_key in &order {
        if total_lines_used >= max_lines {
            break;
        }
        let matches = &file_groups[file_key];
        let shown = matches.len().min(MAX_MATCHES_PER_FILE);
        for m in &matches[..shown] {
            if total_lines_used >= max_lines {
                break;
            }
            result.push(m.clone());
            total_lines_used += 1;
        }
        let remaining_in_file = matches.len().saturating_sub(shown);
        if remaining_in_file > 0 && total_lines_used < max_lines {
            result.push(format!("  ... and {} more matches in {}", remaining_in_file, file_key));
            total_lines_used += 1;
        }
    }

    // If we hit the limit, show how many files were skipped
    let files_shown = order.iter().take_while(|f| {
        // count files that got at least one line in result
        file_groups[f.as_str()].iter().any(|m| result.contains(m))
    }).count();
    let files_remaining = order.len().saturating_sub(files_shown);
    if files_remaining > 0 {
        result.push(format!("... {} more files with matches", files_remaining));
    }

    result.join("\n")
}

/// Extract the file path prefix from a search result line.
/// Handles "file:line:col:content", "file:line:content", and "file:content".
fn extract_file_prefix(line: &str) -> String {
    // Try to split on ':' — file paths may contain ':' on Windows but this is Unix-focused.
    // Pattern: the file prefix is everything before the first ':' that is followed by a digit.
    if let Some(colon_pos) = line.find(':') {
        let after = &line[colon_pos + 1..];
        // If what follows the colon starts with a digit, this is likely file:line:...
        if after.starts_with(|c: char| c.is_ascii_digit()) {
            return line[..colon_pos].to_string();
        }
        // Otherwise treat everything before the first colon as the file key
        return line[..colon_pos].to_string();
    }
    // No colon — use the whole line as key (unusual for search output)
    line.to_string()
}
