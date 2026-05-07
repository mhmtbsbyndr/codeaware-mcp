use codeaware_mcp::v4::{
    CodeSymbol, ContextItemKind, SemanticContextAssembler, SemanticContextOptions,
    SemanticIndex, SymbolKind,
};

#[test]
fn semantic_context_contains_symbol_items() {
    let mut index = SemanticIndex::default();

    index.symbols.add(CodeSymbol {
        name: "build_context".to_string(),
        kind: SymbolKind::Function,
        path: "src/context.rs".to_string(),
        line_start: 1,
        line_end: 10,
        signature: None,
    });

    let items = SemanticContextAssembler::assemble(
        "context",
        &index,
        SemanticContextOptions::default(),
    );

    assert!(!items.is_empty());
    assert!(items.iter().any(|item| item.kind == ContextItemKind::FileExcerpt));
}
