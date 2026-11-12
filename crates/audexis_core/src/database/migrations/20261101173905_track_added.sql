-- Add migration script here
ALTER TABLE files
ADD COLUMN added_at INTEGER NOT NULL DEFAULT 0;

UPDATE files SET added_at = unixepoch() WHERE added_at <= 0;

CREATE TRIGGER files_added_at_default
AFTER INSERT ON files
WHEN NEW.added_at <= 0
BEGIN
    UPDATE files SET added_at = unixepoch() WHERE id = NEW.id;
END;

CREATE INDEX idx_files_added_at ON files(added_at);
