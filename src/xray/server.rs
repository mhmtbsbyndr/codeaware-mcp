use crate::xray::metrics::MetricsState;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const DASHBOARD_HTML: &str = include_str!("dashboard.html");
const DEFAULT_PORT: u16 = 9847;

pub struct XrayServer {
    port: u16,
    _handle: Option<thread::JoinHandle<()>>,
}

/// Check if an existing XRay server is already responding on the default port.
fn probe_existing_server() -> bool {
    use std::net::TcpStream as TcpClient;
    if let Ok(mut stream) = TcpClient::connect_timeout(
        &format!("127.0.0.1:{DEFAULT_PORT}").parse().unwrap(),
        Duration::from_millis(200),
    ) {
        let _ = stream.write_all(b"GET /api/metrics HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        let mut buf = [0u8; 64];
        if let Ok(n) = stream.read(&mut buf) {
            let resp = String::from_utf8_lossy(&buf[..n]);
            return resp.contains("200 OK");
        }
    }
    false
}

/// Create a TcpListener with SO_REUSEADDR so the port is available immediately
/// after a previous process exits.
fn bind_with_reuseaddr(port: u16) -> Result<TcpListener, std::io::Error> {
    use std::os::unix::io::FromRawFd;

    unsafe {
        let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let optval: libc::c_int = 1;
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &optval as *const _ as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );

        let addr = libc::sockaddr_in {
            sin_len: std::mem::size_of::<libc::sockaddr_in>() as u8,
            sin_family: libc::AF_INET as u8,
            sin_port: port.to_be(),
            sin_addr: libc::in_addr { s_addr: u32::from_be_bytes([127, 0, 0, 1]).to_be() },
            sin_zero: [0; 8],
        };

        if libc::bind(fd, &addr as *const libc::sockaddr_in as *const libc::sockaddr,
                       std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t) < 0 {
            let err = std::io::Error::last_os_error();
            libc::close(fd);
            return Err(err);
        }

        if libc::listen(fd, 128) < 0 {
            let err = std::io::Error::last_os_error();
            libc::close(fd);
            return Err(err);
        }

        Ok(TcpListener::from_raw_fd(fd))
    }
}

impl XrayServer {
    pub fn start(metrics: Arc<Mutex<MetricsState>>) -> Result<Self, std::io::Error> {
        // If there's already a server on DEFAULT_PORT, reuse it (no new tab)
        if probe_existing_server() {
            return Ok(XrayServer {
                port: DEFAULT_PORT,
                _handle: None,
            });
        }

        // Bind with SO_REUSEADDR so port is immediately reusable after process exit
        let listener = bind_with_reuseaddr(DEFAULT_PORT)
            .or_else(|_| TcpListener::bind("127.0.0.1:0"))?;
        let port = listener.local_addr()?.port();

        let handle = thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let metrics = Arc::clone(&metrics);
                thread::spawn(move || handle_connection(stream, &metrics));
            }
        });

        Ok(XrayServer {
            port,
            _handle: Some(handle),
        })
    }

    /// Returns true if this instance reuses an already-running server
    /// (no thread was spawned).
    pub fn is_reused(&self) -> bool {
        self._handle.is_none()
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

fn merge_shared_metrics(snapshot: &mut crate::xray::metrics::MetricsSnapshot) {
    if let Some(shared) = crate::xray::shared::read_shared() {
        snapshot.raw_tokens_total += shared.raw_tokens_total;
        snapshot.compressed_tokens_total += shared.compressed_tokens_total;
        snapshot.tool_calls += shared.tool_calls;
        for (file, tokens) in &shared.file_tokens {
            *snapshot.file_tokens.entry(file.clone()).or_insert(0) += tokens;
        }
        for ev in &shared.timeline {
            snapshot.timeline.push(crate::xray::metrics::TimelineEvent {
                timestamp: ev.timestamp.clone(),
                tool: ev.tool.clone(),
                file: ev.file.clone(),
                raw_tokens: ev.raw_tokens,
                compressed_tokens: 0,
                duration_ms: 0,
                phase: String::new(),
            });
        }
        // Sort timeline by timestamp
        snapshot.timeline.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    }
}

fn get_merged_snapshot(metrics: &Arc<Mutex<MetricsState>>) -> String {
    let mut snapshot = metrics.lock().ok().map(|m| m.snapshot()).unwrap_or_else(|| {
        crate::xray::metrics::MetricsSnapshot {
            raw_tokens_total: 0,
            compressed_tokens_total: 0,
            tool_calls: 0,
            file_tokens: std::collections::HashMap::new(),
            edit_scores: vec![],
            timeline: vec![],
            phase: "Idle".to_string(),
            session_id: String::new(),
            error_loops: vec![],
        }
    });
    merge_shared_metrics(&mut snapshot);
    serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string())
}

fn serve_metrics_json(stream: &mut TcpStream, metrics: &Arc<Mutex<MetricsState>>) {
    let json = get_merged_snapshot(metrics);
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
        let json = get_merged_snapshot(metrics);
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
