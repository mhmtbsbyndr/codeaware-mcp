use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::path::Path;

pub struct SessionDb {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservationRecord {
    pub id: i64,
    pub title: Option<String>,
    pub text: String,
    pub observation_type: String,
    pub concepts: Option<String>,
    pub project: Option<String>,
    pub files: Option<String>,
    pub facts: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct SaveObservationOpts<'a> {
    pub title: Option<&'a str>,
    pub text: &'a str,
    pub observation_type: &'a str,
    pub concepts: Option<&'a str>,
    pub project: Option<&'a str>,
    pub files: Option<&'a str>,
    pub facts: Option<&'a str>,
}

pub struct SearchObservationsOpts<'a> {
    pub query: &'a str,
    pub project: Option<&'a str>,
    pub observation_type: Option<&'a str>,
    pub limit: usize,
    pub offset: usize,
    pub date_start: Option<&'a str>,
    pub date_end: Option<&'a str>,
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
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Execute a raw SQL statement (used for recovery, e.g. ROLLBACK)
    pub fn execute_raw(&self, sql: &str) -> Result<(), PersistenceError> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), PersistenceError> {
        let schema = include_str!("../../migrations/001_initial.sql");
        self.conn.execute_batch(schema)?;
        let health_schema = include_str!("../../migrations/002_health_scores.sql");
        self.conn.execute_batch(health_schema)?;
        let observations_schema = include_str!("../../migrations/003_observations.sql");
        self.conn.execute_batch(observations_schema)?;
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

    /// Upsert a file's health factors and computed score
    pub fn upsert_health(
        &self,
        project_path: &str,
        file_path: &str,
        factors: &crate::session::health::HealthFactors,
    ) -> Result<(), PersistenceError> {
        let score = crate::session::health::compute_health(factors);
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO code_health (project_path, file_path, health_score, test_coverage_score, stability_score, error_score, complexity_score, doc_score, last_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(project_path, file_path) DO UPDATE SET
               health_score = ?3, test_coverage_score = ?4, stability_score = ?5,
               error_score = ?6, complexity_score = ?7, doc_score = ?8, last_updated = ?9",
            params![project_path, file_path, score, factors.test_coverage, factors.stability, factors.error_rate, factors.complexity, factors.documentation, now],
        )?;
        Ok(())
    }

    /// Get health factors for a specific file
    pub fn get_health(
        &self,
        project_path: &str,
        file_path: &str,
    ) -> Option<crate::session::health::HealthFactors> {
        self.conn
            .query_row(
                "SELECT test_coverage_score, stability_score, error_score, complexity_score, doc_score FROM code_health WHERE project_path = ?1 AND file_path = ?2",
                params![project_path, file_path],
                |row| {
                    Ok(crate::session::health::HealthFactors {
                        test_coverage: row.get::<_, u32>(0)?,
                        stability: row.get::<_, u32>(1)?,
                        error_rate: row.get::<_, u32>(2)?,
                        complexity: row.get::<_, u32>(3)?,
                        documentation: row.get::<_, u32>(4)?,
                    })
                },
            )
            .ok()
    }

    // ── Observation / Memory methods ──────────────────────────────────

    pub fn save_observation(&self, obs: &SaveObservationOpts<'_>) -> Result<i64, PersistenceError> {
        self.conn.execute(
            "INSERT INTO observations (title, text, observation_type, concepts, project, files, facts, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))",
            params![obs.title, obs.text, obs.observation_type, obs.concepts, obs.project, obs.files, obs.facts],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn search_observations(
        &self,
        opts: &SearchObservationsOpts<'_>,
    ) -> Result<Vec<ObservationRecord>, PersistenceError> {
        // Defense-in-depth: strip colons (column filters) even if caller sanitized
        let query: String = opts.query.replace(':', " ");
        let project = opts.project;
        let observation_type = opts.observation_type;
        let limit = opts.limit;
        let offset = opts.offset;
        let date_start = opts.date_start;
        let date_end = opts.date_end;
        let mut conditions = vec!["observations_fts MATCH ?1".to_string()];
        let mut param_idx = 2u32;

        if project.is_some() {
            conditions.push(format!("c.project = ?{param_idx}"));
            param_idx += 1;
        }
        if observation_type.is_some() {
            conditions.push(format!("c.observation_type = ?{param_idx}"));
            param_idx += 1;
        }
        if date_start.is_some() {
            conditions.push(format!("c.created_at >= ?{param_idx}"));
            param_idx += 1;
        }
        if date_end.is_some() {
            conditions.push(format!("c.created_at <= ?{param_idx}"));
            param_idx += 1;
        }

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT c.id, c.title, c.text, c.observation_type, c.concepts, c.project, c.files, c.facts, c.created_at, c.updated_at
             FROM observations_fts f
             INNER JOIN observations c ON c.id = f.rowid
             WHERE {where_clause}
             ORDER BY rank
             LIMIT ?{param_idx} OFFSET ?{}",
            param_idx + 1
        );

        let mut stmt = self.conn.prepare(&sql)?;

        // Build dynamic params
        let mut bound: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        bound.push(Box::new(query.to_string()));
        if let Some(p) = project { bound.push(Box::new(p.to_string())); }
        if let Some(t) = observation_type { bound.push(Box::new(t.to_string())); }
        if let Some(ds) = date_start { bound.push(Box::new(ds.to_string())); }
        if let Some(de) = date_end { bound.push(Box::new(de.to_string())); }
        bound.push(Box::new(limit as i64));
        bound.push(Box::new(offset as i64));

        let param_refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|b| b.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(ObservationRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    text: row.get(2)?,
                    observation_type: row.get(3)?,
                    concepts: row.get(4)?,
                    project: row.get(5)?,
                    files: row.get(6)?,
                    facts: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_observation_by_id(
        &self,
        id: i64,
    ) -> Result<Option<ObservationRecord>, PersistenceError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, text, observation_type, concepts, project, files, facts, created_at, updated_at
             FROM observations WHERE id = ?1",
        )?;
        let record = stmt
            .query_row(params![id], |row| {
                Ok(ObservationRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    text: row.get(2)?,
                    observation_type: row.get(3)?,
                    concepts: row.get(4)?,
                    project: row.get(5)?,
                    files: row.get(6)?,
                    facts: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .optional()?;
        Ok(record)
    }

    pub fn get_observation_timeline(
        &self,
        anchor_id: i64,
        depth_before: usize,
        depth_after: usize,
        project: Option<&str>,
    ) -> Result<Vec<ObservationRecord>, PersistenceError> {
        // Verify anchor exists
        self.get_observation_by_id(anchor_id)?
            .ok_or_else(|| PersistenceError::Sqlite(
                rusqlite::Error::QueryReturnedNoRows,
            ))?;

        let project_filter = if project.is_some() { " AND project = ?3" } else { "" };

        // Before (including anchor), use id for stable ordering
        let sql_before = format!(
            "SELECT id, title, text, observation_type, concepts, project, files, facts, created_at, updated_at
             FROM observations WHERE id <= ?1{project_filter} ORDER BY id DESC LIMIT ?2"
        );
        let mut stmt_before = self.conn.prepare(&sql_before)?;
        let before: Vec<ObservationRecord> = if let Some(p) = project {
            stmt_before.query_map(params![anchor_id, depth_before as i64 + 1, p], |row| {
                Self::row_to_observation(row)
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt_before.query_map(params![anchor_id, depth_before as i64 + 1], |row| {
                Self::row_to_observation(row)
            })?.collect::<Result<Vec<_>, _>>()?
        };

        // After anchor
        let sql_after = format!(
            "SELECT id, title, text, observation_type, concepts, project, files, facts, created_at, updated_at
             FROM observations WHERE id > ?1{project_filter} ORDER BY id ASC LIMIT ?2"
        );
        let mut stmt_after = self.conn.prepare(&sql_after)?;
        let after: Vec<ObservationRecord> = if let Some(p) = project {
            stmt_after.query_map(params![anchor_id, depth_after as i64, p], |row| {
                Self::row_to_observation(row)
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt_after.query_map(params![anchor_id, depth_after as i64], |row| {
                Self::row_to_observation(row)
            })?.collect::<Result<Vec<_>, _>>()?
        };

        // Combine: before (reversed to chronological) + after
        let mut result: Vec<ObservationRecord> = before.into_iter().rev().collect();
        result.extend(after);
        Ok(result)
    }

    fn row_to_observation(row: &rusqlite::Row) -> rusqlite::Result<ObservationRecord> {
        Ok(ObservationRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            text: row.get(2)?,
            observation_type: row.get(3)?,
            concepts: row.get(4)?,
            project: row.get(5)?,
            files: row.get(6)?,
            facts: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }

    /// Get the unhealthiest files in a project, ordered by ascending health score
    pub fn get_unhealthiest(
        &self,
        project_path: &str,
        limit: usize,
    ) -> Vec<crate::session::health::CodeHealth> {
        let mut stmt = match self.conn.prepare(
            "SELECT file_path, health_score, test_coverage_score, stability_score, error_score, complexity_score, doc_score, last_updated
             FROM code_health WHERE project_path = ?1 ORDER BY health_score ASC LIMIT ?2",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        stmt.query_map(params![project_path, limit as i64], |row| {
            Ok(crate::session::health::CodeHealth {
                file_path: row.get(0)?,
                health_score: row.get(1)?,
                factors: crate::session::health::HealthFactors {
                    test_coverage: row.get(2)?,
                    stability: row.get(3)?,
                    error_rate: row.get(4)?,
                    complexity: row.get(5)?,
                    documentation: row.get(6)?,
                },
                trend: "stable".to_string(),
                last_updated: row.get(7)?,
            })
        })
        .ok()
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
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
