use regex::Regex;
use std::sync::OnceLock;

/// One output line, or a run of identical (after normalisation) lines.
struct Entry<'a> {
    example: &'a str,
    count: usize,
    diagnostic: bool,
}

/// Errors, panics, failures and `file.ext:line` locations.
/// Such lines are never collapsed into a run and never omitted.
pub fn is_diagnostic(line: &str) -> bool {
    static LOCATION: OnceLock<Regex> = OnceLock::new();
    let lower = line.to_ascii_lowercase();
    const KEYWORDS: [&str; 6] = ["error", "panic", "fail", "exception", "fatal", "traceback"];
    if KEYWORDS.iter().any(|k| lower.contains(k)) {
        return true;
    }
    LOCATION
        .get_or_init(|| Regex::new(r"[\w./\\-]+\.\w+:\d+").expect("static regex"))
        .is_match(line)
}

/// Grouping key: diagnostics and number-only lines match identical text only. Other lines
/// ignore digits (timestamps, counters, ids) so that "heartbeat 12ms" and "heartbeat 13ms" group.
fn group_key(line: &str, diagnostic: bool) -> String {
    // Numeric-only lines (e.g. `seq` output) carry their data in the digits: group only identical ones.
    let letters = line.chars().filter(|c| c.is_alphabetic()).count();
    if diagnostic || letters < 3 {
        format!("D{line}")
    } else {
        let digits_masked: String = line
            .chars()
            .map(|c| if c.is_ascii_digit() { '#' } else { c })
            .collect();
        format!("N{}", digits_masked.trim_end())
    }
}

/// Collapse consecutive lines with the same grouping key. The first line stays as the example.
fn collapse<'a>(lines: &[&'a str]) -> Vec<Entry<'a>> {
    let mut entries: Vec<(Entry<'a>, String)> = Vec::new();
    for &line in lines {
        let diagnostic = is_diagnostic(line);
        let key = group_key(line, diagnostic);
        match entries.last_mut() {
            Some((entry, last_key)) if *last_key == key => entry.count += 1,
            _ => entries.push((Entry { example: line, count: 1, diagnostic }, key)),
        }
    }
    entries.into_iter().map(|(e, _)| e).collect()
}

fn render_entry(entry: &Entry<'_>) -> String {
    if entry.count > 1 {
        format!("{} (x {})", entry.example, entry.count)
    } else {
        entry.example.to_string()
    }
}

/// Generic compression: collapse repeated lines, then head + tail.
/// Diagnostic lines (errors, panics, file:line) are always kept, even in the omitted middle.
pub fn compress(raw: &str, max_lines: usize) -> String {
    if max_lines == 0 {
        return raw.to_string();
    }
    let lines: Vec<&str> = raw.lines().collect();
    let entries = collapse(&lines);
    let collapsed = entries.len() < lines.len();

    if entries.len() <= max_lines {
        if !collapsed {
            return raw.to_string();
        }
        return entries.iter().map(render_entry).collect::<Vec<_>>().join("\n");
    }

    let total = entries.len();
    let diagnostic_count = entries.iter().filter(|e| e.diagnostic).count();
    let budget = max_lines.saturating_sub(diagnostic_count);
    let head_count = (budget * 2) / 3;
    let tail_count = budget - head_count;

    let keep: Vec<bool> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| e.diagnostic || i < head_count || i >= total - tail_count.min(total))
        .collect();

    let mut out: Vec<String> = Vec::new();
    let mut omitted = 0usize;
    for (entry, &kept) in entries.iter().zip(&keep) {
        if kept {
            if omitted > 0 {
                out.push(format!("... {} Zeilen ausgelassen ...", omitted));
                omitted = 0;
            }
            out.push(render_entry(entry));
        } else {
            omitted += entry.count;
        }
    }
    if omitted > 0 {
        out.push(format!("... {} Zeilen ausgelassen ...", omitted));
    }
    out.join("\n")
}
