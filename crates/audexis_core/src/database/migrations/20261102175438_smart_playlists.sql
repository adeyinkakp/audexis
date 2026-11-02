-- Add migration script here
ALTER TABLE playlists
ADD COLUMN smart_rules TEXT
CHECK (smart_rules IS NULL OR json_valid(smart_rules));

ALTER TABLE playlists
ADD COLUMN cloud_enabled INTEGER NOT NULL DEFAULT 1
CHECK (cloud_enabled IN (0, 1));
