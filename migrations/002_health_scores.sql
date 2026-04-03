CREATE TABLE IF NOT EXISTS code_health (
    project_path TEXT NOT NULL,
    file_path TEXT NOT NULL,
    health_score INTEGER NOT NULL DEFAULT 50,
    test_coverage_score INTEGER NOT NULL DEFAULT 50,
    stability_score INTEGER NOT NULL DEFAULT 50,
    error_score INTEGER NOT NULL DEFAULT 50,
    complexity_score INTEGER NOT NULL DEFAULT 50,
    doc_score INTEGER NOT NULL DEFAULT 50,
    last_updated TEXT NOT NULL,
    PRIMARY KEY (project_path, file_path)
);
