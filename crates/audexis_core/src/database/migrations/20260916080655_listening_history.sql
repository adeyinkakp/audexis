-- Add migration script here
CREATE TABLE counted_plays (
    session_id TEXT PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    played_at INTEGER,
     playlist_id INTEGER REFERENCES playlists(id) ON DELETE SET NULL
);
CREATE INDEX idx_counted_plays_file ON counted_plays(file_id);
CREATE INDEX idx_media_info_plays ON media_info(plays DESC) WHERE plays > 0;


CREATE INDEX idx_counted_plays_date ON counted_plays(played_at, file_id);
CREATE TABLE listening_history (
    session_id TEXT PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    playlist_id INTEGER REFERENCES playlists(id) ON DELETE SET NULL,
    played_at INTEGER NOT NULL
);
CREATE INDEX idx_listening_history_date ON listening_history(played_at DESC);
CREATE INDEX idx_listening_history_file ON listening_history(file_id);
CREATE INDEX idx_listening_history_playlist ON listening_history(playlist_id);
