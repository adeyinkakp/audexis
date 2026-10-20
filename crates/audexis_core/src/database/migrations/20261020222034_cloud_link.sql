-- Add migration script here
PRAGMA foreign_keys = ON;

CREATE TABLE connections (
    id TEXT PRIMARY KEY NOT NULL,
    server_url TEXT NOT NULL,
    server_user_id TEXT NOT NULL,
    UNIQUE (server_url, server_user_id)
);

CREATE TABLE file_cloud_links (
    file_id INTEGER NOT NULL
        REFERENCES files(id) ON DELETE CASCADE,
    connection_id TEXT NOT NULL
        REFERENCES connections(id) ON DELETE CASCADE,
    cloud_track_id TEXT NOT NULL,
    PRIMARY KEY (file_id, connection_id)
);

CREATE INDEX idx_file_cloud_links_cloud_track
    ON file_cloud_links(connection_id, cloud_track_id);
