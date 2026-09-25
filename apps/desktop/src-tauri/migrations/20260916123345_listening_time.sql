-- Add migration script here
CREATE TABLE
    listening_time_daily (
        file_id INTEGER NOT NULL REFERENCES files (id) ON DELETE CASCADE,
        source_playlist_id INTEGER NOT NULL DEFAULT 0,
        day TEXT NOT NULL,
        heard_us INTEGER NOT NULL DEFAULT 0 CHECK (heard_us >= 0),
        PRIMARY KEY (file_id, source_playlist_id, day)
    );

CREATE INDEX idx_listening_time_day ON listening_time_daily (day);

CREATE TABLE
    listening_time_checkpoint (
        slot INTEGER PRIMARY KEY CHECK (slot = 1),
        segment_id TEXT NOT NULL,
        heard_us INTEGER NOT NULL
    );
