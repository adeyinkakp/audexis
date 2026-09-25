-- Add migration script here
ALTER TABLE files
ADD COLUMN metadata_status TEXT NOT NULL DEFAULT 'ok' CHECK (metadata_status IN ('ok', 'failed'));
