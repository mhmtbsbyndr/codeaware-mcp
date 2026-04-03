use crate::xray::metrics::MetricsState;
use crate::xray::server::XrayServer;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

static BROWSER_OPENED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Serialize)]
pub struct XrayResult {
    pub url: String,
    pub port: u16,
    pub message: String,
}

pub fn handle_xray(
    metrics: Arc<Mutex<MetricsState>>,
    existing_server: &Mutex<Option<XrayServer>>,
) -> Result<XrayResult, String> {
    let mut guard = existing_server.lock().map_err(|e| format!("lock error: {e}"))?;

    if let Some(ref server) = *guard {
        return Ok(XrayResult {
            url: server.url(),
            port: server.port(),
            message: "Dashboard already running. Reuse existing browser tab.".to_string(),
        });
    }

    let server = XrayServer::start(metrics).map_err(|e| format!("server start error: {e}"))?;
    let is_reused = server.is_reused();
    let result = XrayResult {
        url: server.url(),
        port: server.port(),
        message: if is_reused {
            format!("Dashboard already running at {}", server.url())
        } else {
            format!("Dashboard started at {}", server.url())
        },
    };

    // Only open browser ONCE per process lifetime, and never for reused servers
    if !is_reused && !BROWSER_OPENED.swap(true, Ordering::SeqCst) {
        let url = server.url();
        #[cfg(target_os = "macos")]
        { let _ = std::process::Command::new("open").arg(&url).spawn(); }
        #[cfg(target_os = "linux")]
        { let _ = std::process::Command::new("xdg-open").arg(&url).spawn(); }
    }

    *guard = Some(server);
    Ok(result)
}
