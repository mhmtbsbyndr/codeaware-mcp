CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    project_path TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    summary TEXT,
    files_touched TEXT,
    patterns TEXT,
    token_stats TEXT
);

CREATE TABLE IF NOT EXISTS file_access_patterns (
    project_path TEXT NOT NULL,
    file_path TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed TEXT NOT NULL,
    co_accessed_with TEXT,
    avg_read_mode TEXT,
    PRIMARY KEY (project_path, file_path)
);

CREATE TABLE IF NOT EXISTS error_signatures (
    project_path TEXT NOT NULL,
    signature_hash TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 0,
    last_seen TEXT NOT NULL,
    typical_fix TEXT,
    PRIMARY KEY (project_path, signature_hash)
);

-- Session events for compaction recovery (FTS5 BM25 ranking)
CREATE TABLE IF NOT EXISTS session_events_content (
    rowid INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    file_path TEXT,
    symbols TEXT,
    summary TEXT,
    error_signature TEXT,
    timestamp TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS session_events USING fts5(
    session_id UNINDEXED,
    tool_name,
    file_path,
    symbols,
    summary,
    error_signature,
    content='session_events_content',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS session_events_ai AFTER INSERT ON session_events_content BEGIN
    INSERT INTO session_events(rowid, session_id, tool_name, file_path, symbols, summary, error_signature)
    VALUES (new.rowid, new.session_id, new.tool_name, new.file_path, new.symbols, new.summary, new.error_signature);
END;

CREATE TRIGGER IF NOT EXISTS session_events_ad AFTER DELETE ON session_events_content BEGIN
    INSERT INTO session_events(session_events, rowid, session_id, tool_name, file_path, symbols, summary, error_signature)
    VALUES ('delete', old.rowid, old.session_id, old.tool_name, old.file_path, old.symbols, old.summary, old.error_signature);
END;
