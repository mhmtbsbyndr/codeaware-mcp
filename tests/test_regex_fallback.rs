use codeaware_mcp::intelligence::regex_fallback::RegexFallback;

#[test]
fn test_regex_extracts_rust_functions() {
    let fallback = RegexFallback::new();
    let code = "pub fn hello(name: &str) -> String {\n    todo!()\n}\n\nfn private() {}\n";
    let symbols = fallback.extract_symbols(code);
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"hello"));
    assert!(names.contains(&"private"));
}

#[test]
fn test_regex_extracts_python_functions() {
    let fallback = RegexFallback::new();
    let code = "def greet(name):\n    pass\n\nclass MyClass:\n    pass\n";
    let symbols = fallback.extract_symbols(code);
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"MyClass"));
}

#[test]
fn test_regex_extracts_js_functions() {
    let fallback = RegexFallback::new();
    let code = "function handler(req) {}\nconst helper = () => {}\nclass App {}\n";
    let symbols = fallback.extract_symbols(code);
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"handler"));
    assert!(names.contains(&"App"));
}
