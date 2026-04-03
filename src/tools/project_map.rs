use std::path::Path;
use std::time::SystemTime;
use ignore::WalkBuilder;
use thiserror::Error;
use crate::intelligence::tree_sitter_provider::TreeSitterProvider;
use crate::tools::smart_read::detect_language;

#[derive(Debug, Clone)]
pub struct ProjectMapInput {
    pub path: String,
    pub depth: usize,
    pub include_symbols: bool,
    pub filter_language: Option<String>,
    pub task_context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub loc: usize,
    pub access_frequency: f64,
    pub symbols: Vec<String>,
    pub last_modified: String,
}

#[derive(Debug, Clone)]
pub struct ProjectMapResult {
    pub project_name: String,
    pub total_files: usize,
    pub total_loc: usize,
    pub total_symbols: usize,
    pub intelligence_level: String,
    pub tree: Vec<FileEntry>,
    pub dependencies: Vec<String>,
    pub suggested_start: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ProjectMapError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path does not exist: {0}")]
    PathNotFound(String),
    #[error("Walk error: {0}")]
    Walk(String),
}

const IGNORED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    "dist",
    "build",
    "__pycache__",
    ".next",
    ".cache",
    "vendor",
    ".idea",
    ".vscode",
];

const BINARY_EXTENSIONS: &[&str] = &[
    "wasm", "so", "dylib", "exe", "dll", "a", "o",
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp",
    "pdf", "zip", "tar", "gz", "bz2", "xz", "7z",
    "mp3", "mp4", "wav", "flac", "ogg",
    "ttf", "woff", "woff2", "eot",
    "class", "pyc", "pyo",
    "db", "sqlite", "lock",
];

fn is_ignored_dir(path: &Path, root: &Path) -> bool {
    // Check each component against ignored dirs list
    if let Ok(rel) = path.strip_prefix(root) {
        for component in rel.components() {
            let name = component.as_os_str().to_string_lossy();
            if IGNORED_DIRS.contains(&name.as_ref()) {
                return true;
            }
        }
    }
    false
}

fn is_binary_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_lower = ext.to_string_lossy().to_lowercase();
        return BINARY_EXTENSIONS.contains(&ext_lower.as_str());
    }
    false
}

fn count_loc(path: &Path) -> usize {
    match std::fs::read_to_string(path) {
        Ok(content) => content.lines().count(),
        Err(_) => 0,
    }
}

fn format_relative_time(modified: SystemTime) -> String {
    let now = SystemTime::now();
    match now.duration_since(modified) {
        Ok(elapsed) => {
            let secs = elapsed.as_secs();
            if secs < 60 {
                format!("{}s ago", secs)
            } else if secs < 3600 {
                format!("{}m ago", secs / 60)
            } else if secs < 86400 {
                format!("{}h ago", secs / 3600)
            } else {
                format!("{}d ago", secs / 86400)
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

fn get_language_from_ext(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let lang = match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "typescript",
        "jsx" => "javascript",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "toml" => "toml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "md" => "markdown",
        "sh" | "bash" => "shell",
        _ => return None,
    };
    Some(lang.to_string())
}

fn depth_from_root(path: &Path, root: &Path) -> usize {
    if let Ok(rel) = path.strip_prefix(root) {
        rel.components().count()
    } else {
        0
    }
}

pub fn project_map(input: &ProjectMapInput) -> Result<ProjectMapResult, ProjectMapError> {
    let root = Path::new(&input.path);
    if !root.exists() {
        return Err(ProjectMapError::PathNotFound(input.path.clone()));
    }

    let project_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut entries: Vec<FileEntry> = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false)
        .build();

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                // Log and skip walk errors
                let _ = err;
                continue;
            }
        };

        let path = entry.path();

        // Skip root itself
        if path == root {
            continue;
        }

        // Skip ignored directories (check path components)
        if is_ignored_dir(path, root) {
            continue;
        }

        // Only process files
        if !path.is_file() {
            continue;
        }

        // Check depth limit
        let depth = depth_from_root(path, root);
        if depth > input.depth {
            continue;
        }

        // Skip binary extensions
        if is_binary_extension(path) {
            continue;
        }

        // Language filter
        if let Some(ref lang_filter) = input.filter_language {
            match get_language_from_ext(path) {
                Some(ref lang) if lang == lang_filter => {}
                _ => continue,
            }
        }

        let loc = count_loc(path);

        let last_modified = match std::fs::metadata(path) {
            Ok(meta) => match meta.modified() {
                Ok(mtime) => format_relative_time(mtime),
                Err(_) => "unknown".to_string(),
            },
            Err(_) => "unknown".to_string(),
        };

        let rel_path = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        let symbols = if input.include_symbols {
            let path_str = path.to_string_lossy();
            if let Some(lang) = detect_language(path_str.as_ref()) {
                if let Ok(content) = std::fs::read_to_string(path) {
                    let provider = TreeSitterProvider::new();
                    match provider.extract_symbols(&content, lang) {
                        Ok(syms) => syms.into_iter().map(|s| s.name).collect(),
                        Err(_) => Vec::new(),
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        entries.push(FileEntry {
            path: rel_path,
            loc,
            access_frequency: 0.0,
            symbols,
            last_modified,
        });
    }

    // Sort by LOC descending
    entries.sort_by(|a, b| b.loc.cmp(&a.loc));

    let total_files = entries.len();
    let total_loc: usize = entries.iter().map(|e| e.loc).sum();
    let total_symbols: usize = entries.iter().map(|e| e.symbols.len()).sum();

    let intelligence_level = if input.include_symbols {
        "symbols+loc".to_string()
    } else {
        "loc".to_string()
    };

    // Suggested start: top 3 files by LOC
    let suggested_start: Vec<String> = entries
        .iter()
        .take(3)
        .map(|e| e.path.clone())
        .collect();

    // Basic dependency detection (look for Cargo.toml, package.json, go.mod, etc.)
    let mut dependencies: Vec<String> = Vec::new();
    for e in &entries {
        let filename = Path::new(&e.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match filename.as_str() {
            "cargo.toml" => dependencies.push("cargo".to_string()),
            "package.json" => dependencies.push("npm".to_string()),
            "go.mod" => dependencies.push("go".to_string()),
            "requirements.txt" | "pyproject.toml" => dependencies.push("pip".to_string()),
            "gemfile" => dependencies.push("bundler".to_string()),
            _ => {}
        }
    }
    dependencies.dedup();

    Ok(ProjectMapResult {
        project_name,
        total_files,
        total_loc,
        total_symbols,
        intelligence_level,
        tree: entries,
        dependencies,
        suggested_start,
    })
}
