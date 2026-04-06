use codeaware_mcp::xray::metrics::MetricsState;
use codeaware_mcp::xray::server::XrayServer;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

fn start_server() -> (XrayServer, u16) {
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
