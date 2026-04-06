CREATE TABLE IF NOT EXISTS observation_summaries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL,
    topic TEXT NOT NULL,
    summary TEXT NOT NULL,
    observation_ids TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_summaries_project ON observation_summaries(project);
