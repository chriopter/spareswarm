CREATE TABLE participants (
    id TEXT PRIMARY KEY,
    name TEXT,
    model TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE tokens (
    id TEXT PRIMARY KEY,
    participant_id TEXT NOT NULL REFERENCES participants(id),
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TEXT
);
ALTER TABLE tasks ADD COLUMN author_id TEXT REFERENCES participants(id);
CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    author_id TEXT REFERENCES participants(id),
    content TEXT NOT NULL,
    link TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT INTO comments(id,task_id,content,link,created_at)
    SELECT id,task_id,content,link,created_at FROM updates;
DROP TABLE updates;
CREATE INDEX comments_task ON comments(task_id);
CREATE INDEX tasks_author ON tasks(author_id);
CREATE INDEX tokens_participant ON tokens(participant_id);
PRAGMA user_version=2;
