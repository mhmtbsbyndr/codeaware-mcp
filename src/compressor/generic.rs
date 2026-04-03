/// Generic head+tail compression.
/// If lines <= max_lines, returns as-is.
/// Otherwise: head (66%) + "... N Zeilen ausgelassen ..." + tail (33%).
pub fn compress(raw: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let total = lines.len();

    if max_lines == 0 || total <= max_lines {
        return raw.to_string();
    }

    let head_count = (max_lines * 2) / 3;
    let tail_count = max_lines - head_count;
    let omitted = total - head_count - tail_count;

    let head: Vec<&str> = lines[..head_count].to_vec();
    let tail: Vec<&str> = lines[total - tail_count..].to_vec();

    let mut result = head.join("\n");
    result.push('\n');
    result.push_str(&format!("... {} Zeilen ausgelassen ...\n", omitted));
    result.push_str(&tail.join("\n"));

    result
}
