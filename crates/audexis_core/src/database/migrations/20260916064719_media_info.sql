-- Add migration script here
CREATE TABLE media_info (
    file_id INTEGER PRIMARY KEY,
    plays INTEGER NOT NULL DEFAULT 0,
    loved BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
);

CREATE INDEX idx_media_info_loved ON media_info(file_id) WHERE loved = 1;
