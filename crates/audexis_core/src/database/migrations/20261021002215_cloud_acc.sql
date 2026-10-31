-- Add migration script here
CREATE TABLE cloud_sync_cursors (
    connection_id TEXT PRIMARY KEY
        REFERENCES connections(id) ON DELETE CASCADE,
    cursor INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE cloud_tracks (
    connection_id TEXT NOT NULL
        REFERENCES connections(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    document TEXT NOT NULL,
    PRIMARY KEY (connection_id, id)
);

CREATE TABLE cloud_playlists (
    connection_id TEXT NOT NULL
        REFERENCES connections(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    document TEXT NOT NULL,
    PRIMARY KEY (connection_id, id)
);

CREATE TABLE cloud_local_state (
    connection_id TEXT NOT NULL
        REFERENCES connections(id) ON DELETE CASCADE,
    file_id INTEGER NOT NULL
        REFERENCES files(id) ON DELETE CASCADE,
    metadata TEXT NOT NULL,
    revision INTEGER NOT NULL,
    PRIMARY KEY (connection_id, file_id)
);

CREATE TABLE cloud_playlist_links (
    connection_id TEXT NOT NULL
        REFERENCES connections(id) ON DELETE CASCADE,
    playlist_id INTEGER NOT NULL
        REFERENCES playlists(id) ON DELETE CASCADE,
    cloud_id TEXT NOT NULL,
    snapshot TEXT NOT NULL,
    revision INTEGER NOT NULL,
    PRIMARY KEY (connection_id, playlist_id)
);

CREATE TABLE cloud_outbox (
    operation_id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL
        REFERENCES connections(id) ON DELETE CASCADE,
    entity_key TEXT NOT NULL,
    payload TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    retry_at INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'pending',
    error TEXT,
    UNIQUE (connection_id, entity_key)
);

CREATE INDEX cloud_outbox_due
    ON cloud_outbox(connection_id, state, retry_at);

CREATE INDEX cloud_playlist_links_remote
    ON cloud_playlist_links(connection_id, cloud_id);
