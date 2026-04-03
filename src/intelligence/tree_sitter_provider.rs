use tree_sitter::{Parser, Language, Node};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    TypeAlias,
    Const,
    Mod,
    Class,       // Python/TS
    Interface,   // TS
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
    pub start_line: usize, // 1-based
    pub end_line: usize,   // 1-based
    pub doc_comment: Option<String>,
    pub visibility: Option<String>,
}

pub struct TreeSitterProvider {}

impl TreeSitterProvider {
    pub fn new() -> Self {
        Self {}
    }

    pub fn extract_symbols(&self, code: &str, language: &str) -> Result<Vec<SymbolInfo>, TreeSitterError> {
        let lang = self.get_language(language)?;
        let mut parser = Parser::new();
        parser.set_language(&lang).map_err(|e| TreeSitterError::LanguageError(e.to_string()))?;

        let tree = parser.parse(code, None)
            .ok_or(TreeSitterError::ParseFailed)?;

        let root = tree.root_node();
        let mut symbols = Vec::new();

        match language {
            "rust" => self.extract_rust_symbols(&root, code, &mut symbols),
            "python" => self.extract_python_symbols(&root, code, &mut symbols),
            "typescript" | "tsx" => self.extract_ts_symbols(&root, code, &mut symbols, true),
            "javascript" | "jsx" => self.extract_ts_symbols(&root, code, &mut symbols, false),
            "go" => self.extract_go_symbols(&root, code, &mut symbols),
            "php" => self.extract_php_symbols(&root, code, &mut symbols),
            "swift" => self.extract_swift_symbols(&root, code, &mut symbols),
            _ => return Err(TreeSitterError::UnsupportedLanguage(language.into())),
        }

        Ok(symbols)
    }

    pub fn build_skeleton(&self, code: &str, language: &str) -> Result<String, TreeSitterError> {
        let symbols = self.extract_symbols(code, language)?;
        let lines: Vec<&str> = code.lines().collect();
        let mut skeleton_lines = Vec::new();

        for sym in &symbols {
            // Add the signature line(s) — convert from 1-based to 0-based index
            let start = sym.start_line.saturating_sub(1);
            if start < lines.len() {
                skeleton_lines.push(format!("{:>4}| {}", sym.start_line, lines[start]));
            }
            // Add closing brace line if distinct from start
            let end = sym.end_line.saturating_sub(1);
            if end < lines.len() && end > start {
                let end_line = lines[end].trim();
                if end_line == "}" || end_line == "}," {
                    skeleton_lines.push(format!("{:>4}| {}", sym.end_line, lines[end]));
                }
            }
        }

        Ok(skeleton_lines.join("\n"))
    }

    fn get_language(&self, name: &str) -> Result<Language, TreeSitterError> {
        match name {
            "rust" => Ok(tree_sitter_rust::LANGUAGE.into()),
            "python" => Ok(tree_sitter_python::LANGUAGE.into()),
            "typescript" | "tsx" => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            "javascript" | "jsx" => Ok(tree_sitter_javascript::LANGUAGE.into()),
            "go" => Ok(tree_sitter_go::LANGUAGE.into()),
            "php" => Ok(tree_sitter_php::LANGUAGE_PHP.into()),
            "swift" => Ok(tree_sitter_swift::LANGUAGE.into()),
            _ => Err(TreeSitterError::UnsupportedLanguage(name.into())),
        }
    }

    fn extract_rust_symbols(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_item" => {
                    if let Some(sym) = self.parse_rust_function(&child, code, SymbolKind::Function) {
                        symbols.push(sym);
                    }
                }
                "struct_item" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.rust_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Struct,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "enum_item" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.rust_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Enum,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "trait_item" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.rust_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Trait,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "impl_item" => {
                    // Extract methods from impl blocks
                    self.extract_rust_impl_methods(&child, code, symbols);
                }
                _ => {
                    // Recurse into children for nested items
                    self.extract_rust_symbols(&child, code, symbols);
                }
            }
        }
    }

    fn extract_rust_impl_methods(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "declaration_list" {
                let mut inner_cursor = child.walk();
                for inner in child.children(&mut inner_cursor) {
                    if inner.kind() == "function_item" {
                        if let Some(sym) = self.parse_rust_function(&inner, code, SymbolKind::Method) {
                            symbols.push(sym);
                        }
                    }
                }
            }
        }
    }

    fn parse_rust_function(&self, node: &Node, code: &str, kind: SymbolKind) -> Option<SymbolInfo> {
        let name = self.get_child_by_field(node, "name", code)?;
        let start_byte = node.start_byte();
        let text = &code[start_byte..node.end_byte()];
        let sig = text.lines().next().unwrap_or("").to_string();
        let visibility = self.rust_visibility(node, code);

        Some(SymbolInfo {
            name,
            kind,
            signature: sig.trim_end_matches('{').trim().to_string(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            doc_comment: None,
            visibility,
        })
    }

    fn extract_python_symbols(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" | "decorated_definition" => {
                    // For decorated_definition, look for the inner function/class
                    let actual = if child.kind() == "decorated_definition" {
                        // find the function_definition or class_definition inside
                        let mut c2 = child.walk();
                        let mut found = None;
                        for inner in child.children(&mut c2) {
                            if inner.kind() == "function_definition" || inner.kind() == "class_definition" {
                                found = Some(inner);
                                break;
                            }
                        }
                        found
                    } else {
                        Some(child)
                    };
                    if let Some(n) = actual {
                        if n.kind() == "function_definition" {
                            if let Some(name) = self.get_child_by_field(&n, "name", code) {
                                let visibility = self.python_visibility(&name);
                                symbols.push(SymbolInfo {
                                    name,
                                    kind: SymbolKind::Function,
                                    signature: self.get_first_line(&n, code),
                                    start_line: n.start_position().row + 1,
                                    end_line: n.end_position().row + 1,
                                    doc_comment: None,
                                    visibility,
                                });
                            }
                        } else if n.kind() == "class_definition" {
                            if let Some(name) = self.get_child_by_field(&n, "name", code) {
                                let visibility = self.python_visibility(&name);
                                symbols.push(SymbolInfo {
                                    name,
                                    kind: SymbolKind::Class,
                                    signature: self.get_first_line(&n, code),
                                    start_line: n.start_position().row + 1,
                                    end_line: n.end_position().row + 1,
                                    doc_comment: None,
                                    visibility,
                                });
                                self.extract_python_class_methods(&n, code, symbols);
                            }
                        }
                    }
                }
                "class_definition" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.python_visibility(&name);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Class,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                        self.extract_python_class_methods(&child, code, symbols);
                    }
                }
                _ => {
                    self.extract_python_symbols(&child, code, symbols);
                }
            }
        }
    }

    fn extract_python_class_methods(&self, class_node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = class_node.walk();
        for child in class_node.children(&mut cursor) {
            if child.kind() == "block" {
                let mut block_cursor = child.walk();
                for item in child.children(&mut block_cursor) {
                    let actual = if item.kind() == "decorated_definition" {
                        let mut c2 = item.walk();
                        let mut found = None;
                        for inner in item.children(&mut c2) {
                            if inner.kind() == "function_definition" {
                                found = Some(inner);
                                break;
                            }
                        }
                        found
                    } else if item.kind() == "function_definition" {
                        Some(item)
                    } else {
                        None
                    };
                    if let Some(n) = actual {
                        if let Some(name) = self.get_child_by_field(&n, "name", code) {
                            let visibility = self.python_visibility(&name);
                            symbols.push(SymbolInfo {
                                name,
                                kind: SymbolKind::Method,
                                signature: self.get_first_line(&n, code),
                                start_line: n.start_position().row + 1,
                                end_line: n.end_position().row + 1,
                                doc_comment: None,
                                visibility,
                            });
                        }
                    }
                }
            }
        }
    }

    fn extract_ts_symbols(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>, _is_ts: bool) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" | "generator_function_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.ts_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Function,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "class_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.ts_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Class,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                        self.extract_ts_class_methods(&child, code, symbols);
                    }
                }
                "interface_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.ts_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Interface,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "type_alias_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.ts_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::TypeAlias,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "export_statement" | "lexical_declaration" | "variable_declaration" => {
                    // Recurse for export wrapping function/class declarations
                    self.extract_ts_symbols(&child, code, symbols, _is_ts);
                }
                _ => {
                    // Skip shallow recursion for most nodes to avoid deeply nested false positives
                }
            }
        }
    }

    fn extract_ts_class_methods(&self, class_node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = class_node.walk();
        for child in class_node.children(&mut cursor) {
            if child.kind() == "class_body" {
                let mut body_cursor = child.walk();
                for item in child.children(&mut body_cursor) {
                    if item.kind() == "method_definition" {
                        if let Some(name) = self.get_child_by_field(&item, "name", code) {
                            let visibility = self.ts_visibility(&item, code);
                            symbols.push(SymbolInfo {
                                name,
                                kind: SymbolKind::Method,
                                signature: self.get_first_line(&item, code),
                                start_line: item.start_position().row + 1,
                                end_line: item.end_position().row + 1,
                                doc_comment: None,
                                visibility,
                            });
                        }
                    }
                }
            }
        }
    }

    fn extract_go_symbols(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.go_visibility(&name);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Function,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "method_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.go_visibility(&name);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Method,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "type_declaration" => {
                    self.extract_go_type(&child, code, symbols);
                }
                _ => {}
            }
        }
    }

    fn extract_go_type(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_spec" {
                if let Some(name) = self.get_child_by_field(&child, "name", code) {
                    let visibility = self.go_visibility(&name);
                    symbols.push(SymbolInfo {
                        name,
                        kind: SymbolKind::TypeAlias,
                        signature: self.get_first_line(node, code),
                        start_line: node.start_position().row + 1,
                        end_line: node.end_position().row + 1,
                        doc_comment: None,
                        visibility,
                    });
                }
            }
        }
    }

    fn extract_php_symbols(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        // Top-level PHP functions default to public
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Function,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility: Some("public".to_string()),
                        });
                    }
                }
                "class_declaration" | "interface_declaration" | "trait_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let kind = if child.kind() == "interface_declaration" {
                            SymbolKind::Interface
                        } else {
                            SymbolKind::Class
                        };
                        let visibility = self.php_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                        self.extract_php_class_methods(&child, code, symbols);
                    }
                }
                "program" => {
                    self.extract_php_symbols(&child, code, symbols);
                }
                _ => {
                    self.extract_php_symbols(&child, code, symbols);
                }
            }
        }
    }

    fn extract_php_class_methods(&self, class_node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = class_node.walk();
        for child in class_node.children(&mut cursor) {
            if child.kind() == "declaration_list" {
                let mut body_cursor = child.walk();
                for item in child.children(&mut body_cursor) {
                    if item.kind() == "method_declaration" {
                        if let Some(name) = self.get_child_by_field(&item, "name", code) {
                            let visibility = self.php_visibility(&item, code);
                            symbols.push(SymbolInfo {
                                name,
                                kind: SymbolKind::Method,
                                signature: self.get_first_line(&item, code),
                                start_line: item.start_position().row + 1,
                                end_line: item.end_position().row + 1,
                                doc_comment: None,
                                visibility,
                            });
                        }
                    }
                }
            }
        }
    }

    fn extract_swift_symbols(&self, node: &Node, code: &str, symbols: &mut Vec<SymbolInfo>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.swift_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Function,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "class_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.swift_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Class,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                        self.extract_swift_symbols(&child, code, symbols);
                    }
                }
                "struct_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "type_name", code)
                        .or_else(|| self.get_child_by_field(&child, "name", code)) {
                        let visibility = self.swift_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Struct,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "protocol_declaration" => {
                    if let Some(name) = self.get_child_by_field(&child, "name", code) {
                        let visibility = self.swift_visibility(&child, code);
                        symbols.push(SymbolInfo {
                            name,
                            kind: SymbolKind::Trait,
                            signature: self.get_first_line(&child, code),
                            start_line: child.start_position().row + 1,
                            end_line: child.end_position().row + 1,
                            doc_comment: None,
                            visibility,
                        });
                    }
                }
                "class_body" | "struct_body" | "protocol_body" | "extension_declaration" => {
                    self.extract_swift_symbols(&child, code, symbols);
                }
                _ => {}
            }
        }
    }

    fn get_child_by_field(&self, node: &Node, field: &str, code: &str) -> Option<String> {
        node.child_by_field_name(field)
            .map(|n| code[n.start_byte()..n.end_byte()].to_string())
    }

    fn get_first_line(&self, node: &Node, code: &str) -> String {
        let text = &code[node.start_byte()..node.end_byte()];
        text.lines().next().unwrap_or("").to_string()
    }

    fn get_node_text<'a>(&self, node: &Node, code: &'a str) -> &'a str {
        &code[node.start_byte()..node.end_byte()]
    }

    fn rust_visibility(&self, node: &Node, code: &str) -> Option<String> {
        let text = self.get_node_text(node, code).trim_start();
        if text.starts_with("pub ") || text.starts_with("pub(") {
            Some("public".to_string())
        } else {
            Some("private".to_string())
        }
    }

    fn python_visibility(&self, name: &str) -> Option<String> {
        if name.starts_with('_') {
            Some("private".to_string())
        } else {
            Some("public".to_string())
        }
    }

    fn ts_visibility(&self, node: &Node, code: &str) -> Option<String> {
        // Check if the node itself or its parent is an export_statement
        if let Some(parent) = node.parent() {
            if parent.kind() == "export_statement" {
                return Some("public".to_string());
            }
        }
        let text = self.get_node_text(node, code).trim_start();
        if text.starts_with("export ") {
            Some("public".to_string())
        } else {
            Some("private".to_string())
        }
    }

    fn go_visibility(&self, name: &str) -> Option<String> {
        if name.starts_with(|c: char| c.is_uppercase()) {
            Some("public".to_string())
        } else {
            Some("private".to_string())
        }
    }

    fn php_visibility(&self, node: &Node, code: &str) -> Option<String> {
        let text = self.get_node_text(node, code).trim_start();
        if text.starts_with("private ") {
            Some("private".to_string())
        } else if text.starts_with("protected ") {
            Some("protected".to_string())
        } else {
            Some("public".to_string())
        }
    }

    fn swift_visibility(&self, node: &Node, code: &str) -> Option<String> {
        let text = self.get_node_text(node, code).trim_start();
        if text.starts_with("public ") {
            Some("public".to_string())
        } else if text.starts_with("private ") {
            Some("private".to_string())
        } else if text.starts_with("fileprivate ") {
            Some("fileprivate".to_string())
        } else if text.starts_with("open ") {
            Some("open".to_string())
        } else {
            Some("internal".to_string())
        }
    }
}

impl Default for TreeSitterProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TreeSitterError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("Parse failed")]
    ParseFailed,
    #[error("Language error: {0}")]
    LanguageError(String),
}
