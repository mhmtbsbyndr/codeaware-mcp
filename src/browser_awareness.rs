#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserConsoleEntry {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserNetworkEntry {
    pub method: String,
    pub url: String,
    pub status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSnapshotSummary {
    pub title: String,
    pub url: String,
    pub dom_summary: String,
    pub console_errors: usize,
    pub failed_requests: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserSessionSnapshot {
    pub title: String,
    pub url: String,
    pub html: String,
    pub console: Vec<BrowserConsoleEntry>,
    pub network: Vec<BrowserNetworkEntry>,
}

pub fn summarize_browser_snapshot(
    snapshot: &BrowserSessionSnapshot,
) -> BrowserSnapshotSummary {
    let console_errors = snapshot
        .console
        .iter()
        .filter(|entry| entry.level.eq_ignore_ascii_case("error"))
        .count();

    let failed_requests = snapshot
        .network
        .iter()
        .filter(|entry| entry.status >= 400)
        .count();

    BrowserSnapshotSummary {
        title: snapshot.title.clone(),
        url: snapshot.url.clone(),
        dom_summary: summarize_html(&snapshot.html),
        console_errors,
        failed_requests,
    }
}

fn summarize_html(html: &str) -> String {
    let lines = html.lines().count();
    let forms = html.matches("<form").count();
    let buttons = html.matches("<button").count();

    format!(
        "HTML summary: {} lines, {} forms, {} buttons",
        lines, forms, buttons
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_browser_snapshot() {
        let snapshot = BrowserSessionSnapshot {
            title: "Dashboard".to_string(),
            url: "https://example.com".to_string(),
            html: "<html><form></form><button>Save</button></html>".to_string(),
            console: vec![BrowserConsoleEntry {
                level: "error".to_string(),
                message: "Failed request".to_string(),
            }],
            network: vec![BrowserNetworkEntry {
                method: "GET".to_string(),
                url: "https://example.com/api".to_string(),
                status: 500,
            }],
        };

        let summary = summarize_browser_snapshot(&snapshot);

        assert_eq!(summary.console_errors, 1);
        assert_eq!(summary.failed_requests, 1);
        assert!(summary.dom_summary.contains("forms"));
    }
}
