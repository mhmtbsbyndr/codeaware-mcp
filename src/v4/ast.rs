use tree_sitter::{Node, Parser, Tree};

use crate::v4::symbols::{CodeSymbol, SymbolIndex, SymbolKind};

pub struct AstExtractor;

impl AstExtractor {
    pub fn parse_rust(source: &str) -> Option<Tree> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .ok()?;
        parser.parse(source, None)
    }

    pub fn extract_rust_symbols(path: &str, source: &str) -> SymbolIndex {
        let mut index = SymbolIndex::default();

        let Some(tree) = Self::parse_rust(source) else {
            return index;
        };

        let root = tree.root_node();
        Self::walk_rust(root, source, path, &mut index);

        index
    }

    fn walk_rust(node: Node, source: &str, path: &str, index: &mut SymbolIndex) {
        let kind = node.kind();

        let symbol_kind = match kind {
            "function_item" => Some(SymbolKind::Function),
            "struct_item" => Some(SymbolKind::Struct),
            "enum_item" => Some(SymbolKind::Enum),
            "trait_item" => Some(SymbolKind::Trait),
            "mod_item" => Some(SymbolKind::Module),
            "const_item" => Some(SymbolKind::Constant),
            _ => None,
        };

        if let Some(symbol_kind) = symbol_kind {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                    index.add(CodeSymbol {
                        name: name.to_string(),
                        kind: symbol_kind,
                        path: path.to_string(),
                        line_start: node.start_position().row + 1,
                        line_end: node.end_position().row + 1,
                        signature: None,
                    });
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            Self::walk_rust(child, source, path, index);
        }
    }
}
