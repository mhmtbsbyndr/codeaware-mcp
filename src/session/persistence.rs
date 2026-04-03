use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

pub struct SessionDb {
    conn: Connection,
}

pub struct SessionRecord {
    pub id: String,
    pub project_path: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub summary: Option<String>,
    pub files_touched: Option<String>,
    pub token_stats: Option<String>,
}

pub struct FileAccessRecord {
    pub file_path: String,
    pub access_count: u32,
    pub last_accessed: String,
    pub co_accessed_with: Option<String>,
    pub avg_read_mode: Option<String>,
}

pub struct ErrorSignatureRecord {
    pub signature_hash: String,
    pub occurrence_count: u32,
    pub last_seen: String,
    pub typical_fix: Option<String>,
}

impl SessionDb {
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        // Create parent dirs if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=100;")?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<(), PersistenceError> {
        let schema = include_str!("../../migrations/001_initial.sql");
        self.conn.execute_batch(schema)?;
        Ok(())
    }

    pub fn save_session(
        &self,
        id: &str,
        project_path: &str,
        started_at: &str,
        summary: &str,
        files_touched: &str,
        token_stats: &str,
    ) -> Result<(), PersistenceError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sessions (id, project_path, started_at, ended_at, summary, files_touched, token_stats) VALUES (?1, ?2, ?3, datetime('now'), ?4, ?5, ?6)",
            params![id, project_path, started_at, summary, files_touched, token_stats],
        )?;
        Ok(())
    }

    pub fn load_latest_session(
        &self,
        project_path: &str,
    ) -> Result<Option<SessionRecord>, PersistenceError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_path, started_at, ended_at, summary, files_touched, token_stats FROM sessions WHERE project_path = ?1 ORDER BY started_at DESC LIMIT 1",
        )?;
        let record = stmt
            .query_row(params![project_path], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    project_path: row.get(1)?,
                    started_at: row.get(2)?,
                    ended_at: row.get(3)?,
                    summary: row.get(4)?,
                    files_touched: row.get(5)?,
                    token_stats: row.get(6)?,
                })
            })
            .optional()?;
        Ok(record)
    }

    pub fn record_file_access(
        &self,
        project_path: &str,
        file_path: &str,
        read_mode: &str,
    ) -> Result<(), PersistenceError> {
        self.conn.execute(
            "INSERT INTO file_access_patterns (project_path, file_path, access_count, last_accessed, avg_read_mode) VALUES (?1, ?2, 1, datetime('now'), ?3) ON CONFLICT(project_path, file_path) DO UPDATE SET access_count = access_count + 1, last_accessed = datetime('now'), avg_read_mode = ?3",
            params![project_path, file_path, read_mode],
        )?;
        Ok(())
    }

    pub fn get_file_access_patterns(
        &self,
        project_path: &str,
    ) -> Result<Vec<FileAccessRecord>, PersistenceError> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path, access_count, last_accessed, co_accessed_with, avg_read_mode FROM file_access_patterns WHERE project_path = ?1 ORDER BY access_count DESC",
        )?;
        let records = stmt
            .query_map(params![project_path], |row| {
                Ok(FileAccessRecord {
                    file_path: row.get(0)?,
                    access_count: row.get(1)?,
                    last_accessed: row.get(2)?,
                    co_accessed_with: row.get(3)?,
                    avg_read_mode: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn record_error_signature(
        &self,
        project_path: &str,
        sig_hash: &str,
        typical_fix: Option<&str>,
    ) -> Result<(), PersistenceError> {
        self.conn.execute(
            "INSERT INTO error_signatures (project_path, signature_hash, occurrence_count, last_seen, typical_fix) VALUES (?1, ?2, 1, datetime('now'), ?3) ON CONFLICT(project_path, signature_hash) DO UPDATE SET occurrence_count = occurrence_count + 1, last_seen = datetime('now'), typical_fix = COALESCE(?3, typical_fix)",
            params![project_path, sig_hash, typical_fix],
        )?;
        Ok(())
    }

    pub fn get_error_signature(
        &self,
        project_path: &str,
        sig_hash: &str,
    ) -> Result<Option<ErrorSignatureRecord>, PersistenceError> {
        let mut stmt = self.conn.prepare(
            "SELECT signature_hash, occurrence_count, last_seen, typical_fix FROM error_signatures WHERE project_path = ?1 AND signature_hash = ?2",
        )?;
        let record = stmt
            .query_row(params![project_path, sig_hash], |row| {
                Ok(ErrorSignatureRecord {
                    signature_hash: row.get(0)?,
                    occurrence_count: row.get(1)?,
                    last_seen: row.get(2)?,
                    typical_fix: row.get(3)?,
                })
            })
            .optional()?;
        Ok(record)
    }

    /// Index a tool call result as a session event for FTS5 search
    pub fn index_session_event(
        &self,
        session_id: &str,
        tool_name: &str,
        file_path: Option<&str>,
        symbols: Option<&str>,
        summary: Option<&str>,
        error_signature: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO session_events_content
             (session_id, tool_name, file_path, symbols, summary, error_signature, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![session_id, tool_name, file_path, symbols, summary, error_signature, timestamp],
        )?;
        Ok(())
    }

    /// Search session events via FTS5 BM25 ranking for compaction recovery
    pub fn search_session_events(
        &self,
        session_id: &str,
        query: &str,
    ) -> Result<Vec<SessionEventRow>, PersistenceError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.session_id, c.tool_name, c.file_path, c.symbols, c.summary, c.error_signature, c.timestamp
             FROM session_events_content c
             INNER JOIN session_events e ON c.rowid = e.rowid
             WHERE e.session_id = ?1 AND session_events MATCH ?2
             ORDER BY rank
             LIMIT 20",
        )?;
        let rows = stmt
            .query_map(params![session_id, query], |row| {
                Ok(SessionEventRow {
                    session_id: row.get(0)?,
                    tool_name: row.get(1)?,
                    file_path: row.get(2)?,
                    symbols: row.get(3)?,
                    summary: row.get(4)?,
                    error_signature: row.get(5)?,
                    timestamp: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

pub struct SessionEventRow {
    pub session_id: String,
    pub tool_name: String,
    pub file_path: Option<String>,
    pub symbols: Option<String>,
    pub summary: Option<String>,
    pub error_signature: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
