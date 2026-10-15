ALTER TABLE projects ADD COLUMN category TEXT NOT NULL DEFAULT 'development';
PRAGMA user_version=8;
