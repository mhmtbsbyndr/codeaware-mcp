pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.len() <= max_lines {
        return raw.to_string();
    }

    // Extract error blocks and warning blocks
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut summary_lines: Vec<String> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Summary lines (e.g. "error: aborting due to N previous errors")
        if line.starts_with("error: aborting") || line.starts_with("error: could not compile") {
            summary_lines.push(line.to_string());
            i += 1;
            continue;
        }

        // error[EXXXX]: message
        if line.starts_with("error[") || (line.starts_with("error") && line.contains("]:")) {
            let mut block = vec![line.to_string()];
            i += 1;
            // Grab location line (  --> file:line)
            if i < lines.len() && lines[i].trim_start().starts_with("-->") {
                block.push(lines[i].to_string());
                i += 1;
            }
            errors.push(block.join("\n"));
            continue;
        }

        // warning: message
        if line.starts_with("warning:") {
            let mut block = vec![line.to_string()];
            i += 1;
            if i < lines.len() && lines[i].trim_start().starts_with("-->") {
                block.push(lines[i].to_string());
                i += 1;
            }
            warnings.push(block.join("\n"));
            continue;
        }

        i += 1;
    }

    // Budget: errors first, then summary, then warnings if space
    let mut result: Vec<String> = Vec::new();
    let mut used = 0usize;

    for e in &errors {
        let lc = e.lines().count();
        if used + lc <= max_lines {
            result.push(e.clone());
            used += lc;
        }
    }

    for s in &summary_lines {
        if used < max_lines {
            result.push(s.clone());
            used += 1;
        }
    }

    for w in &warnings {
        let lc = w.lines().count();
        if used + lc <= max_lines {
            result.push(w.clone());
            used += lc;
        } else {
            break;
        }
    }

    result.join("\n")
}
