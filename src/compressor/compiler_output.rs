use super::generic;
use regex::Regex;
use std::sync::OnceLock;

#[derive(PartialEq, Eq, Clone, Copy)]
enum Kind {
    Error,
    Warning,
    Summary,
}

/// Classify a line that starts a diagnostic. Covers rustc, gcc/clang and tsc header formats.
fn header_kind(line: &str) -> Option<Kind> {
    static GCC: OnceLock<Regex> = OnceLock::new();
    static TSC: OnceLock<Regex> = OnceLock::new();
    if line.starts_with("error: aborting")
        || line.starts_with("error: could not compile")
        || (line.starts_with("warning:") && line.contains("emitted"))
    {
        return Some(Kind::Summary);
    }
    if line.starts_with("error[") || line.starts_with("error:") {
        return Some(Kind::Error);
    }
    if line.starts_with("warning:") {
        return Some(Kind::Warning);
    }
    let gcc = GCC.get_or_init(|| Regex::new(r"^\S.*?:\d+(?::\d+)?: (?:fatal )?(error|warning)").expect("static regex"));
    if let Some(caps) = gcc.captures(line) {
        return Some(if &caps[1] == "error" { Kind::Error } else { Kind::Warning });
    }
    let tsc = TSC.get_or_init(|| Regex::new(r"^\S.*?\(\d+,\d+\): (error|warning) ").expect("static regex"));
    if tsc.is_match(line) {
        return Some(if line.contains(": error ") { Kind::Error } else { Kind::Warning });
    }
    None
}

/// One diagnostic: header, its `-->` location, and (for errors) the first note only.
struct Block {
    kind: Kind,
    lines: Vec<String>,
}

pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(kind) = header_kind(lines[i]) else {
            i += 1;
            continue;
        };
        let mut block = vec![lines[i].to_string()];
        let mut location_seen = false;
        let mut note_seen = false;
        i += 1;
        // Rustc blocks end at a blank line; gcc/tsc headers are single lines.
        while i < lines.len() && !lines[i].trim().is_empty() && header_kind(lines[i]).is_none() {
            let trimmed = lines[i].trim_start();
            if !location_seen && trimmed.starts_with("-->") {
                block.push(lines[i].to_string());
                location_seen = true;
            } else if kind == Kind::Error && !note_seen && trimmed.starts_with("= note:") {
                block.push(lines[i].trim().to_string());
                note_seen = true;
            }
            i += 1;
        }
        blocks.push(Block { kind, lines: block });
    }

    if blocks.is_empty() {
        return generic::compress(raw, max_lines);
    }

    // Errors and summaries are never dropped; warnings fill the remaining budget.
    let mut out: Vec<String> = Vec::new();
    for block in blocks.iter().filter(|b| b.kind == Kind::Error) {
        out.extend(block.lines.iter().cloned());
    }
    for block in blocks.iter().filter(|b| b.kind == Kind::Summary) {
        out.extend(block.lines.iter().cloned());
    }
    let warnings: Vec<&Block> = blocks.iter().filter(|b| b.kind == Kind::Warning).collect();
    // One line is kept free for the "more warnings omitted" marker.
    let mut omitted = 0usize;
    for w in &warnings {
        if out.len() + w.lines.len() < max_lines {
            out.extend(w.lines.iter().cloned());
        } else {
            omitted += 1;
        }
    }
    if omitted > 0 {
        out.push(format!("... {} more warnings omitted", omitted));
    }
    out.join("\n")
}
