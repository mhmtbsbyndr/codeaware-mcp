use codeaware_mcp::intelligence::tree_sitter_provider::{TreeSitterProvider, SymbolKind};

#[test]
fn test_extract_python_functions_and_classes() {
    let provider = TreeSitterProvider::new();
    let code = r#"
def hello(name: str) -> str:
    return f"Hello, {name}"

class AuthManager:
    def __init__(self, secret: str):
        self.secret = secret

    def verify_token(self, token: str) -> bool:
        return len(token) > 10

    async def async_method(self):
        pass
"#;
    let symbols = provider.extract_symbols(code, "python").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"hello"));
    assert!(names.contains(&"AuthManager"));
    assert!(names.contains(&"__init__"));
    assert!(names.contains(&"verify_token"));
    assert!(names.contains(&"async_method"));

    let class = symbols.iter().find(|s| s.name == "AuthManager").unwrap();
    assert_eq!(class.kind, SymbolKind::Class);

    let hello = symbols.iter().find(|s| s.name == "hello").unwrap();
    assert_eq!(hello.kind, SymbolKind::Function);
}

#[test]
fn test_extract_typescript_symbols() {
    let provider = TreeSitterProvider::new();
    let code = r#"
export function greet(name: string): string {
    return `Hello, ${name}`;
}

interface AuthConfig {
    secret: string;
    duration: number;
}

class AuthManager {
    private secret: string;

    constructor(config: AuthConfig) {
        this.secret = config.secret;
    }

    verify(token: string): boolean {
        return token.length > 10;
    }
}

type UserId = string;

const DEFAULT_TIMEOUT = 5000;
"#;
    let symbols = provider.extract_symbols(code, "typescript").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"AuthConfig"));
    assert!(names.contains(&"AuthManager"));
    assert!(names.contains(&"verify"));

    let iface = symbols.iter().find(|s| s.name == "AuthConfig").unwrap();
    assert_eq!(iface.kind, SymbolKind::Interface);
}

#[test]
fn test_extract_javascript_symbols() {
    let provider = TreeSitterProvider::new();
    let code = r#"
function hello(name) {
    return `Hello, ${name}`;
}

class Router {
    constructor() {
        this.routes = [];
    }

    addRoute(path, handler) {
        this.routes.push({ path, handler });
    }
}
"#;
    let symbols = provider.extract_symbols(code, "javascript").unwrap();
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"hello"));
    assert!(names.contains(&"Router"));
}
