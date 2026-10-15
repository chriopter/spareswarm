ALTER TABLE comments ADD COLUMN reply_to TEXT REFERENCES comments(id);
CREATE INDEX comments_reply_to ON comments(reply_to);
PRAGMA user_version=3;
