use codeaware_mcp::server::McpServer;
use std::io::{self, BufRead, Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "hook" {
        let event = args.get(2).map(|s| s.as_str()).unwrap_or("unknown");
        // Read raw bytes and decode lossily once: read_to_string would silently
        // drop the whole hook input on a single invalid UTF-8 byte.
        let input = codeaware_mcp::hooks::stdin_utf8::read_hook_input(std::io::stdin()).unwrap_or_default();
        match codeaware_mcp::hooks::dispatch_hook(event, &input) {
            Ok(output) => {
                println!("{output}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Hook error: {e}");
                std::process::exit(1);
            }
        }
    }

    eprintln!("CodeAware MCP Server v{} starting (stdio)...", env!("CARGO_PKG_VERSION"));
    run_stdio_server();
}

fn run_stdio_server() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let server = McpServer::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        if let Some(response) = server.handle_message(&line) {
            let _ = writeln!(stdout, "{response}");
            let _ = stdout.flush();
        }
    }
}
