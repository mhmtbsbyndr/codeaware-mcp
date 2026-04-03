use crate::intelligence::tree_sitter_provider::TreeSitterProvider;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticChange {
    pub change_type: String,      // "signature_changed", "visibility_changed", "symbol_added", "symbol_removed", "body_changed"
    pub symbol: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub breaking: bool,
}

pub fn compute_semantic_diff(old_content: &str, new_content: &str, language: &str) -> Vec<SemanticChange> {
    let provider = TreeSitterProvider::new();
    let old_symbols = match provider.extract_symbols(old_content, language) {
        Ok(syms) => syms,
        Err(_) => return Vec::new(),
    };
    let new_symbols = match provider.extract_symbols(new_content, language) {
        Ok(syms) => syms,
        Err(_) => return Vec::new(),
    };

    let mut changes = Vec::new();

    // Find removed symbols (in old but not in new)
    for old_sym in &old_symbols {
        if !new_symbols.iter().any(|s| s.name == old_sym.name && s.kind == old_sym.kind) {
            changes.push(SemanticChange {
                change_type: "symbol_removed".to_string(),
                symbol: old_sym.name.clone(),
                from: Some(old_sym.signature.clone()),
                to: None,
                breaking: true,
            });
        }
    }

    // Find added symbols (in new but not in old)
    for new_sym in &new_symbols {
        if !old_symbols.iter().any(|s| s.name == new_sym.name && s.kind == new_sym.kind) {
            changes.push(SemanticChange {
                change_type: "symbol_added".to_string(),
                symbol: new_sym.name.clone(),
                from: None,
                to: Some(new_sym.signature.clone()),
                breaking: false,
            });
        }
    }

    // Find changed symbols (same name+kind in both)
    for old_sym in &old_symbols {
        if let Some(new_sym) = new_symbols.iter().find(|s| s.name == old_sym.name && s.kind == old_sym.kind) {
            // Check signature change
            if old_sym.signature != new_sym.signature {
                changes.push(SemanticChange {
                    change_type: "signature_changed".to_string(),
                    symbol: old_sym.name.clone(),
                    from: Some(old_sym.signature.clone()),
                    to: Some(new_sym.signature.clone()),
                    breaking: true,
                });
            }

            // Check visibility change
            if old_sym.visibility != new_sym.visibility {
                let narrowed = matches!(
                    (old_sym.visibility.as_deref(), new_sym.visibility.as_deref()),
                    (Some("public"), Some("private")) | (Some("public"), Some("protected"))
                );
                changes.push(SemanticChange {
                    change_type: "visibility_changed".to_string(),
                    symbol: old_sym.name.clone(),
                    from: old_sym.visibility.clone(),
                    to: new_sym.visibility.clone(),
                    breaking: narrowed,
                });
            }

            // Check body change (line count difference, signature same)
            if old_sym.signature == new_sym.signature {
                let old_lines = old_sym.end_line - old_sym.start_line;
                let new_lines = new_sym.end_line - new_sym.start_line;
                if old_lines != new_lines {
                    changes.push(SemanticChange {
                        change_type: "body_changed".to_string(),
                        symbol: old_sym.name.clone(),
                        from: Some(format!("{} lines", old_lines)),
                        to: Some(format!("{} lines", new_lines)),
                        breaking: false,
                    });
                }
            }
        }
    }

    changes
}
