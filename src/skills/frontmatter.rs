//! YAML-like frontmatter and markdown structure extraction for SKILL.md files.

/// Key/value pairs from a `---` frontmatter block, in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frontmatter {
    fields: Vec<(String, String)>,
}

impl Frontmatter {
    /// First non-empty value stored under `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, v)| k == key && !v.is_empty())
            .map(|(_, v)| v.as_str())
    }

    /// All parsed fields in file order.
    pub fn fields(&self) -> &[(String, String)] {
        &self.fields
    }
}

/// Splits a SKILL.md document into its frontmatter and body.
///
/// Returns empty frontmatter and the whole text when no well-formed block
/// (opening `---` line and a closing `---` line) is present.
pub fn split(text: &str) -> (Frontmatter, &str) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.split_inclusive('\n');
    let first = match lines.next() {
        Some(first) if first.trim_end() == "---" => first,
        _ => return (Frontmatter::default(), text),
    };
    let mut offset = first.len();
    let mut yaml: Vec<&str> = Vec::new();
    for line in lines {
        offset += line.len();
        if line.trim_end() == "---" {
            return (parse_fields(&yaml), &text[offset..]);
        }
        yaml.push(line.trim_end_matches(['\r', '\n']));
    }
    // Unterminated block: treat the whole document as body.
    (Frontmatter::default(), text)
}

fn parse_fields(lines: &[&str]) -> Frontmatter {
    let mut fields = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if line.trim().is_empty()
            || line.trim_start().starts_with('#')
            || line.starts_with(char::is_whitespace)
        {
            continue;
        }
        let Some((key, raw)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_string();
        let raw = raw.trim();
        if matches!(raw, ">" | ">-" | ">+" | "|" | "|-" | "|+") {
            let folded = raw.starts_with('>');
            let mut parts: Vec<&str> = Vec::new();
            while i < lines.len()
                && (lines[i].trim().is_empty() || lines[i].starts_with(char::is_whitespace))
            {
                parts.push(lines[i].trim());
                i += 1;
            }
            let joined = if folded {
                parts
                    .iter()
                    .filter(|p| !p.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                parts.join("\n")
            };
            fields.push((key, joined.trim().to_string()));
        } else {
            fields.push((key, unquote(raw).to_string()));
        }
    }
    Frontmatter { fields }
}

fn unquote(raw: &str) -> &str {
    let bytes = raw.as_bytes();
    if raw.len() >= 2
        && ((bytes[0] == b'"' && bytes[raw.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[raw.len() - 1] == b'\''))
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    }
}

fn is_fence(trimmed_start: &str) -> bool {
    trimmed_start.starts_with("```") || trimmed_start.starts_with("~~~")
}

/// Text of an ATX heading (`# Title` through `###### Title`), if the line is one.
fn heading_text(trimmed_start: &str) -> Option<&str> {
    let hashes = trimmed_start.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &trimmed_start[hashes..];
    if !(rest.starts_with(' ') || rest.starts_with('\t')) {
        return None;
    }
    let text = rest.trim();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// All ATX headings in the body, outside fenced code blocks, in order.
pub fn headings(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        let t = line.trim_start();
        if is_fence(t) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(h) = heading_text(t) {
            out.push(h.to_string());
        }
    }
    out
}

/// First plain-text paragraph after the first heading (or from the top when
/// the body has no heading). Fenced code and headings are never included.
pub fn first_paragraph(body: &str) -> Option<String> {
    let mut ready = headings(body).is_empty();
    let mut in_fence = false;
    let mut para: Vec<&str> = Vec::new();
    for line in body.lines() {
        let t = line.trim_start();
        if is_fence(t) {
            in_fence = !in_fence;
            if !para.is_empty() {
                break;
            }
            continue;
        }
        if in_fence {
            continue;
        }
        if heading_text(t).is_some() {
            if !para.is_empty() {
                break;
            }
            ready = true;
            continue;
        }
        if !ready {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !para.is_empty() {
                break;
            }
            continue;
        }
        para.push(trimmed);
    }
    if para.is_empty() {
        None
    } else {
        Some(para.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalar_and_folded_fields() {
        let text = "---\nname: pdf-tools\ndescription: >-\n  Extract text\n  from PDFs.\nversion: 1.0\n---\n# Body\n";
        let (fm, body) = split(text);
        assert_eq!(fm.get("name"), Some("pdf-tools"));
        assert_eq!(fm.get("description"), Some("Extract text from PDFs."));
        assert_eq!(fm.get("version"), Some("1.0"));
        assert_eq!(body, "# Body\n");
    }

    #[test]
    fn strips_quotes_and_keeps_colons_in_value() {
        let (fm, _) = split("---\ndescription: \"Use when: needed\"\n---\nx");
        assert_eq!(fm.get("description"), Some("Use when: needed"));
    }

    #[test]
    fn no_frontmatter_returns_whole_text() {
        let text = "# Title\n\nBody";
        let (fm, body) = split(text);
        assert!(fm.fields().is_empty());
        assert_eq!(body, text);
    }

    #[test]
    fn unterminated_frontmatter_is_body() {
        let text = "---\nname: x\nno closing";
        let (fm, body) = split(text);
        assert!(fm.fields().is_empty());
        assert_eq!(body, text);
    }

    #[test]
    fn headings_skip_code_fences() {
        let body = "# Top\ntext\n```\n# not a heading\n```\n## Second\n";
        assert_eq!(headings(body), vec!["Top", "Second"]);
    }

    #[test]
    fn first_paragraph_follows_first_heading() {
        let body = "# Title\n\nFirst line\nsecond line.\n\nLater para.\n";
        assert_eq!(
            first_paragraph(body).as_deref(),
            Some("First line second line.")
        );
    }

    #[test]
    fn first_paragraph_without_heading_uses_top() {
        assert_eq!(
            first_paragraph("Intro text.\n\n# H\nmore").as_deref(),
            Some("Intro text.")
        );
    }
}
