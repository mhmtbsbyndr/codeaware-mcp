use codeaware_mcp::intelligence::tree_sitter_provider::{TreeSitterProvider, SymbolKind};

// ── Java ─────────────────────────────────────────────────────────

#[test]
fn test_extract_java_class_and_methods() {
    let provider = TreeSitterProvider::new();
    let code = r#"
public class UserService {
    private String name;

    public UserService(String name) {
        this.name = name;
    }

    public String getName() {
        return this.name;
    }

    private void validate(String input) {
        if (input == null) {
            throw new IllegalArgumentException("null");
        }
    }
}
"#;
    let symbols = provider.extract_symbols(code, "java").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"UserService"), "Expected UserService in {:?}", names);
    assert!(names.contains(&"getName"), "Expected getName in {:?}", names);
    assert!(names.contains(&"validate"), "Expected validate in {:?}", names);

    let class = symbols.iter().find(|s| s.name == "UserService").unwrap();
    assert_eq!(class.kind, SymbolKind::Class);
    assert_eq!(class.visibility.as_deref(), Some("public"));

    let get_name = symbols.iter().find(|s| s.name == "getName").unwrap();
    assert_eq!(get_name.kind, SymbolKind::Method);
    assert_eq!(get_name.visibility.as_deref(), Some("public"));

    let validate = symbols.iter().find(|s| s.name == "validate").unwrap();
    assert_eq!(validate.kind, SymbolKind::Method);
    assert_eq!(validate.visibility.as_deref(), Some("private"));
}

#[test]
fn test_extract_java_constructor() {
    let provider = TreeSitterProvider::new();
    let code = r#"
public class Foo {
    public Foo(int x) {
        // constructor
    }
}
"#;
    let symbols = provider.extract_symbols(code, "java").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"Foo"), "Expected constructor Foo in {:?}", names);

    // Constructor should be classified as Method
    let ctors: Vec<_> = symbols.iter().filter(|s| s.name == "Foo" && s.kind == SymbolKind::Method).collect();
    assert!(!ctors.is_empty(), "Expected Foo constructor as Method");
}

#[test]
fn test_extract_java_interface_and_enum() {
    let provider = TreeSitterProvider::new();
    let code = r#"
interface Repository {
    void save(Object entity);
}

enum Status {
    ACTIVE,
    INACTIVE
}
"#;
    let symbols = provider.extract_symbols(code, "java").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"Repository"), "Expected Repository in {:?}", names);
    assert!(names.contains(&"Status"), "Expected Status in {:?}", names);

    let iface = symbols.iter().find(|s| s.name == "Repository").unwrap();
    assert_eq!(iface.kind, SymbolKind::Interface);

    let enm = symbols.iter().find(|s| s.name == "Status").unwrap();
    assert_eq!(enm.kind, SymbolKind::Enum);
}

#[test]
fn test_java_empty_parse() {
    let provider = TreeSitterProvider::new();
    let result = provider.extract_symbols("", "java");
    assert!(result.is_ok());
}

// ── C ────────────────────────────────────────────────────────────

#[test]
fn test_extract_c_functions() {
    let provider = TreeSitterProvider::new();
    let code = r#"
void greet(const char* name) {
    printf("Hello, %s\n", name);
}

int add(int a, int b) {
    return a + b;
}
"#;
    let symbols = provider.extract_symbols(code, "c").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"greet"), "Expected greet in {:?}", names);
    assert!(names.contains(&"add"), "Expected add in {:?}", names);

    let greet = symbols.iter().find(|s| s.name == "greet").unwrap();
    assert_eq!(greet.kind, SymbolKind::Function);
}

#[test]
fn test_extract_c_struct_and_enum() {
    let provider = TreeSitterProvider::new();
    let code = r#"
struct Config {
    char* name;
    int value;
};

enum Color {
    RED,
    GREEN,
    BLUE
};
"#;
    let symbols = provider.extract_symbols(code, "c").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"Config"), "Expected Config in {:?}", names);
    assert!(names.contains(&"Color"), "Expected Color in {:?}", names);

    let config = symbols.iter().find(|s| s.name == "Config").unwrap();
    assert_eq!(config.kind, SymbolKind::Struct);

    let color = symbols.iter().find(|s| s.name == "Color").unwrap();
    assert_eq!(color.kind, SymbolKind::Enum);
}

#[test]
fn test_extract_c_typedef() {
    let provider = TreeSitterProvider::new();
    let code = r#"
typedef struct {
    int x;
    int y;
} Point;

typedef int (*Callback)(int, int);
"#;
    let symbols = provider.extract_symbols(code, "c").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"Point"), "Expected Point typedef in {:?}", names);
    assert!(names.contains(&"Callback"), "Expected Callback typedef in {:?}", names);

    let point = symbols.iter().find(|s| s.name == "Point" && s.kind == SymbolKind::TypeAlias).unwrap();
    assert_eq!(point.kind, SymbolKind::TypeAlias);
}

#[test]
fn test_c_empty_parse() {
    let provider = TreeSitterProvider::new();
    let result = provider.extract_symbols("", "c");
    assert!(result.is_ok());
}

// ── C++ ──────────────────────────────────────────────────────────

#[test]
fn test_extract_cpp_class_and_methods() {
    let provider = TreeSitterProvider::new();
    let code = r#"
class Logger {
public:
    void log(const std::string& message) {
        // implementation
    }

    int getLevel() const {
        return level_;
    }

private:
    int level_ = 0;
};
"#;
    let symbols = provider.extract_symbols(code, "cpp").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"Logger"), "Expected Logger in {:?}", names);
    assert!(names.contains(&"log"), "Expected log in {:?}", names);
    assert!(names.contains(&"getLevel"), "Expected getLevel in {:?}", names);

    let logger = symbols.iter().find(|s| s.name == "Logger").unwrap();
    assert_eq!(logger.kind, SymbolKind::Class);

    let log_method = symbols.iter().find(|s| s.name == "log").unwrap();
    assert_eq!(log_method.kind, SymbolKind::Method);
}

#[test]
fn test_extract_cpp_namespace() {
    let provider = TreeSitterProvider::new();
    let code = r#"
namespace utils {

void helper() {
    // implementation
}

} // namespace utils
"#;
    let symbols = provider.extract_symbols(code, "cpp").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"utils"), "Expected namespace utils in {:?}", names);
    assert!(names.contains(&"helper"), "Expected helper in {:?}", names);

    let ns = symbols.iter().find(|s| s.name == "utils").unwrap();
    assert_eq!(ns.kind, SymbolKind::Mod);
}

#[test]
fn test_extract_cpp_template_class() {
    let provider = TreeSitterProvider::new();
    let code = r#"
template<typename T>
class Container {
public:
    void add(const T& item) {
        items_.push_back(item);
    }
};
"#;
    let symbols = provider.extract_symbols(code, "cpp").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"Container"), "Expected Container in {:?}", names);

    let container = symbols.iter().find(|s| s.name == "Container").unwrap();
    assert_eq!(container.kind, SymbolKind::Class);
}

#[test]
fn test_extract_cpp_free_function_and_typedef() {
    let provider = TreeSitterProvider::new();
    let code = r#"
void freeFunction(int x) {
    // implementation
}

typedef int IntAlias;
"#;
    let symbols = provider.extract_symbols(code, "cpp").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"freeFunction"), "Expected freeFunction in {:?}", names);
    assert!(names.contains(&"IntAlias"), "Expected IntAlias in {:?}", names);

    let free_fn = symbols.iter().find(|s| s.name == "freeFunction").unwrap();
    assert_eq!(free_fn.kind, SymbolKind::Function);

    let alias = symbols.iter().find(|s| s.name == "IntAlias").unwrap();
    assert_eq!(alias.kind, SymbolKind::TypeAlias);
}

#[test]
fn test_extract_cpp_struct_and_enum() {
    let provider = TreeSitterProvider::new();
    let code = r#"
struct Point {
    double x;
    double y;
};

enum class Color {
    Red,
    Green,
    Blue
};
"#;
    let symbols = provider.extract_symbols(code, "cpp").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

    assert!(names.contains(&"Point"), "Expected Point in {:?}", names);
    assert!(names.contains(&"Color"), "Expected Color in {:?}", names);

    let point = symbols.iter().find(|s| s.name == "Point").unwrap();
    assert_eq!(point.kind, SymbolKind::Struct);

    let color = symbols.iter().find(|s| s.name == "Color").unwrap();
    assert_eq!(color.kind, SymbolKind::Enum);
}

#[test]
fn test_cpp_empty_parse() {
    let provider = TreeSitterProvider::new();
    let result = provider.extract_symbols("", "cpp");
    assert!(result.is_ok());
}

// ── Language detection ───────────────────────────────────────────

#[test]
fn test_detect_language_new_extensions() {
    use codeaware_mcp::tools::smart_read::detect_language;

    assert_eq!(detect_language("Main.java"), Some("java"));
    assert_eq!(detect_language("main.c"), Some("c"));
    assert_eq!(detect_language("header.h"), Some("c"));
    assert_eq!(detect_language("main.cpp"), Some("cpp"));
    assert_eq!(detect_language("main.cc"), Some("cpp"));
    assert_eq!(detect_language("main.cxx"), Some("cpp"));
    assert_eq!(detect_language("header.hpp"), Some("cpp"));
    assert_eq!(detect_language("header.hh"), Some("cpp"));
    assert_eq!(detect_language("header.hxx"), Some("cpp"));
}

#[test]
fn test_intelligence_level_new_languages() {
    use codeaware_mcp::intelligence::{select_intelligence, IntelligenceLevel};

    assert_eq!(select_intelligence("java", false), IntelligenceLevel::TreeSitter);
    assert_eq!(select_intelligence("c", false), IntelligenceLevel::TreeSitter);
    assert_eq!(select_intelligence("cpp", false), IntelligenceLevel::TreeSitter);
}

#[test]
fn test_supported_languages_count_with_new() {
    let provider = TreeSitterProvider::new();
    let languages = ["rust", "python", "typescript", "javascript", "go", "php", "swift", "java", "c", "cpp"];
    for lang in &languages {
        let result = provider.extract_symbols("", lang);
        assert!(result.is_ok(), "Language '{}' failed to parse: {:?}", lang, result);
    }
}
