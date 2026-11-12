-- Add migration script here
ALTER TABLE files ADD COLUMN hash TEXT;
CREATE INDEX idx_files_hash ON files(hash);
