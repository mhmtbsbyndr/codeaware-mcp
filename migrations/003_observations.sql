-- Semantic memory: persistent observations across sessions
CREATE TABLE IF NOT EXISTS observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT,
    text TEXT NOT NULL,
    observation_type TEXT NOT NULL DEFAULT 'discovery',
    concepts TEXT,
    project TEXT,
    files TEXT,
    facts TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- FTS5 virtual table for BM25-ranked full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS observations_fts USING fts5(
    title,
    text,
    facts,
    concepts,
    observation_type,
    project UNINDEXED,
    content='observations',
    content_rowid='id'
);

-- Sync triggers (3-table FTS5 pattern)
CREATE TRIGGER IF NOT EXISTS observations_ai AFTER INSERT ON observations BEGIN
    INSERT INTO observations_fts(rowid, title, text, facts, concepts, observation_type, project)
    VALUES (new.id, new.title, new.text, new.facts, new.concepts, new.observation_type, new.project);
END;

CREATE TRIGGER IF NOT EXISTS observations_ad AFTER DELETE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, rowid, title, text, facts, concepts, observation_type, project)
    VALUES ('delete', old.id, old.title, old.text, old.facts, old.concepts, old.observation_type, old.project);
END;

CREATE TRIGGER IF NOT EXISTS observations_au AFTER UPDATE ON observations BEGIN
    INSERT INTO observations_fts(observations_fts, rowid, title, text, facts, concepts, observation_type, project)
    VALUES ('delete', old.id, old.title, old.text, old.facts, old.concepts, old.observation_type, old.project);
    INSERT INTO observations_fts(rowid, title, text, facts, concepts, observation_type, project)
    VALUES (new.id, new.title, new.text, new.facts, new.concepts, new.observation_type, new.project);
END;

-- Indexes for timeline and filtering
CREATE INDEX IF NOT EXISTS idx_observations_created ON observations(created_at);
CREATE INDEX IF NOT EXISTS idx_observations_project ON observations(project);
CREATE INDEX IF NOT EXISTS idx_observations_type ON observations(observation_type);
