CREATE TABLE IF NOT EXISTS mobile_library_state (
    slot INTEGER PRIMARY KEY CHECK(slot=1),
    user_id TEXT,
    current_index INTEGER CHECK(current_index IS NULL OR current_index >= 0),
    generation INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS mobile_queue (
    position INTEGER PRIMARY KEY CHECK(position >= 0),
    track_id TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mobile_downloads (
    track_id TEXT PRIMARY KEY,
    file TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size INTEGER NOT NULL CHECK(size > 0)
);
