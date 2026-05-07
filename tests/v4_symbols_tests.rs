use codeaware_mcp::v4::{AstExtractor, SymbolKind};

#[test]
fn rust_symbol_extraction_finds_function_and_struct() {
    let source = r#"
        pub struct ContextPackage {
            pub id: String,
        }

        pub fn build_context() {}
    "#;

    let index = AstExtractor::extract_rust_symbols("src/context.rs", source);

    let struct_symbol = index
        .symbols
        .iter()
        .find(|symbol| symbol.kind == SymbolKind::Struct);

    let fn_symbol = index
        .symbols
        .iter()
        .find(|symbol| symbol.kind == SymbolKind::Function);

    assert!(struct_symbol.is_some());
    assert!(fn_symbol.is_some());
}
