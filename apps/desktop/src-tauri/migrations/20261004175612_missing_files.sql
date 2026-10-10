-- Add migration script here
ALTER TABLE files ADD COLUMN missing_since INTEGER;

CREATE INDEX idx_files_missing_since ON files(missing_since);
