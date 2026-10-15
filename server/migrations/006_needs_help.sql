CREATE TABLE tasks_next (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','working','needs_help','done')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    author_id TEXT REFERENCES participants(id),
    help_reason TEXT,
    CHECK(status != 'needs_help' OR length(trim(COALESCE(help_reason,''))) > 0)
);
INSERT INTO tasks_next(id,project_id,title,description,status,created_at,updated_at,author_id)
    SELECT id,project_id,title,description,status,created_at,updated_at,author_id FROM tasks;
DROP TABLE tasks;
ALTER TABLE tasks_next RENAME TO tasks;
CREATE INDEX tasks_author ON tasks(author_id);
PRAGMA user_version=6;
