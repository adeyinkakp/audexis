-- Add migration script here
CREATE TABLE playlists (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE playlist_tracks (
    playlist_id INTEGER NOT NULL,
    file_id INTEGER NOT NULL,
    ord INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, ord),
    FOREIGN KEY (playlist_id) REFERENCES playlists (id) ON DELETE CASCADE,
    FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
);


CREATE INDEX idx_playlist_tracks_playlist_id ON playlist_tracks (playlist_id, ord);
CREATE INDEX idx_playlist_tracks_file_id ON playlist_tracks (file_id);
