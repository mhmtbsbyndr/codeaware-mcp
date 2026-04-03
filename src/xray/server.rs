use crate::xray::metrics::MetricsState;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const DASHBOARD_HTML: &str = include_str!("dashboard.html");

pub struct XrayServer {
    port: u16,
    _handle: thread::JoinHandle<()>,
}

impl XrayServer {
    pub fn start(metrics: Arc<Mutex<MetricsState>>) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();

        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    let metrics = Arc::clone(&metrics);
                    thread::spawn(move || handle_connection(stream, &metrics));
                }
            }
        });

        Ok(XrayServer {
            port,
            _handle: handle,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

fn handle_connection(mut stream: TcpStream, metrics: &Arc<Mutex<MetricsState>>) {
    let mut buf = [0u8; 2048];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    match path {
        "/" => serve_html(&mut stream),
        "/api/metrics" => serve_metrics_json(&mut stream, metrics),
        "/events" => serve_sse(&mut stream, metrics),
        _ => serve_404(&mut stream),
    }
}

fn serve_html(stream: &mut TcpStream) {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        DASHBOARD_HTML.len(),
        DASHBOARD_HTML
    );
    let _ = stream.write_all(response.as_bytes());
}

fn serve_metrics_json(stream: &mut TcpStream, metrics: &Arc<Mutex<MetricsState>>) {
    let json = metrics
        .lock()
        .ok()
        .and_then(|m| serde_json::to_string(&m.snapshot()).ok())
        .unwrap_or_else(|| "{}".to_string());
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        json.len(),
        json
    );
    let _ = stream.write_all(response.as_bytes());
}

fn serve_sse(stream: &mut TcpStream, metrics: &Arc<Mutex<MetricsState>>) {
    let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";
    if stream.write_all(header.as_bytes()).is_err() {
        return;
    }
    loop {
        let json = metrics
            .lock()
            .ok()
            .and_then(|m| serde_json::to_string(&m.snapshot()).ok())
            .unwrap_or_else(|| "{}".to_string());
        if stream
            .write_all(format!("data: {json}\n\n").as_bytes())
            .is_err()
        {
            break;
        }
        let _ = stream.flush();
        thread::sleep(Duration::from_secs(2));
    }
}

fn serve_404(stream: &mut TcpStream) {
    let body = "Not Found";
    let response = format!(
        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}
