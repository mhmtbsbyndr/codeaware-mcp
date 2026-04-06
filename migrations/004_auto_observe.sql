-- Migration 004: Auto-observe support
-- New columns (dedup_hash, auto_generated, source_tool, session_id) are added
-- via Rust code (try_add_column) since SQLite lacks ALTER TABLE ADD COLUMN IF NOT EXISTS.
-- This file serves as the migration marker.
CREATE TABLE IF NOT EXISTS _migration_004_done (id INTEGER PRIMARY KEY);
