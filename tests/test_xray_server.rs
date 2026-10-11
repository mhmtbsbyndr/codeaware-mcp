use codeaware_mcp::xray::metrics::MetricsState;
use codeaware_mcp::xray::server::XrayServer;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};


/// Keep the restart log out of the real per-user directory during tests.
fn isolate_state_dir() {
    static STATE: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    STATE.get_or_init(|| {
        let dir = tempfile::TempDir::new().expect("temp state dir");
        std::env::set_var("XDG_RUNTIME_DIR", dir.path());
        dir
    });
}

fn start_server() -> (XrayServer, u16) {
    isolate_state_dir();
    let metrics = Arc::new(Mutex::new(MetricsState::new()));
    let server = XrayServer::start(metrics, None).expect("server should start");
    let port = server.port();
    (server, port)
}

#[test]
fn test_server_starts_on_free_port() {
    let (_server, port) = start_server();
    assert!(port > 0);
}

#[test]
fn test_dashboard_html_served() {
    let (_server, port) = start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("200 OK"), "should return 200");
    assert!(
        response.contains("CodeAware X-Ray"),
        "should contain dashboard title"
    );
}

#[test]
fn test_metrics_api_returns_json() {
    let (_server, port) = start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .write_all(b"GET /api/metrics HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("200 OK"));
    assert!(response.contains("application/json"));
    assert!(response.contains("raw_tokens_total"));
}

#[test]
fn test_sse_stream_connects() {
    let (_server, port) = start_server();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(1)))
        .ok();
    stream
        .write_all(b"GET /events HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).unwrap_or(0);
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("text/event-stream"));
}

fn raw_request(port: u16, request: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    response
}

#[test]
fn test_foreign_host_header_is_forbidden() {
    // DNS rebinding: the browser sends the attacker's hostname as Host.
    let (_server, port) = start_server();
    let resp = raw_request(port, "GET /api/memories HTTP/1.1\r\nHost: evil.example.com\r\n\r\n");
    assert!(resp.starts_with("HTTP/1.1 403"), "got: {resp}");
    assert!(!resp.contains("\r\n\r\n[") && !resp.contains("application/json"));
}

#[test]
fn test_missing_host_header_is_forbidden() {
    let (_server, port) = start_server();
    let resp = raw_request(port, "GET /api/metrics HTTP/1.1\r\n\r\n");
    assert!(resp.starts_with("HTTP/1.1 403"), "got: {resp}");
}

#[test]
fn test_loopback_host_variants_are_allowed() {
    let (_server, port) = start_server();
    for host in [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        "127.0.0.1".to_string(),
        format!("[::1]:{port}"),
    ] {
        let req = format!("GET /api/metrics HTTP/1.1\r\nHost: {host}\r\n\r\n");
        let resp = raw_request(port, &req);
        assert!(resp.starts_with("HTTP/1.1 200"), "Host {host} should be allowed, got: {resp}");
    }
}

#[test]
fn test_host_suffix_tricks_are_forbidden() {
    let (_server, port) = start_server();
    for host in ["localhost.evil.com", "127.0.0.1.evil.com", "evil.com:80@localhost"] {
        let req = format!("GET /api/metrics HTTP/1.1\r\nHost: {host}\r\n\r\n");
        let resp = raw_request(port, &req);
        assert!(resp.starts_with("HTTP/1.1 403"), "Host {host} must be rejected, got: {resp}");
    }
}

#[test]
fn test_identity_endpoint_identifies_service() {
    let (_server, port) = start_server();
    let resp = raw_request(port, "GET /api/identity HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    assert!(resp.starts_with("HTTP/1.1 200"), "got: {resp}");
    let body = resp.split_once("\r\n\r\n").unwrap().1;
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(json["service"], "codeaware-xray");
}

#[test]
fn test_ready_endpoint_reports_ready_after_start() {
    let (_server, port) = start_server();
    let resp = raw_request(port, "GET /api/ready HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    assert!(resp.starts_with("HTTP/1.1 200"), "got: {resp}");
    let body = resp.split_once("\r\n\r\n").unwrap().1;
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(json["ready"], true);
    assert_eq!(json["phase"], "ready");
    assert_eq!(json["service"], "codeaware-xray");
    assert!(json["elapsed_ms"].is_u64(), "elapsed_ms missing: {body}");
}

#[test]
fn test_ready_endpoint_respects_host_check() {
    let (_server, port) = start_server();
    let resp = raw_request(port, "GET /api/ready HTTP/1.1\r\nHost: evil.example.com\r\n\r\n");
    assert!(resp.starts_with("HTTP/1.1 403"), "got: {resp}");
}
