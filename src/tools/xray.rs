use crate::xray::metrics::MetricsState;
use crate::xray::server::XrayServer;
use serde::Serialize;
use std::sync::{Arc, Mutex};

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
            message: "Dashboard already running".to_string(),
        });
    }

    let server = XrayServer::start(metrics).map_err(|e| format!("server start error: {e}"))?;
    let result = XrayResult {
        url: server.url(),
        port: server.port(),
        message: "Dashboard started. Opening browser...".to_string(),
    };

    let url = server.url();
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    }

    *guard = Some(server);
    Ok(result)
}
