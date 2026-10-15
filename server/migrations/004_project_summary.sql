ALTER TABLE projects ADD COLUMN summary TEXT NOT NULL DEFAULT '';
PRAGMA user_version=4;
