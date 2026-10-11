-- Indexes for the lookups that filter by project and session.
-- session_events is an FTS5 table; its filtered base table is session_events_content.

CREATE INDEX IF NOT EXISTS idx_session_events_content_session_id
    ON session_events_content(session_id);

CREATE INDEX IF NOT EXISTS idx_sessions_project_path
    ON sessions(project_path);
