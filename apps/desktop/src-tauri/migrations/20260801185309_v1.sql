-- Add migration script here
CREATE TABLE
    import_roots (
        id INTEGER PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        last_scanned INTEGER NOT NULL DEFAULT 0
    );

CREATE TABLE
    files (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        path TEXT NOT NULL UNIQUE,
        file_name TEXT NOT NULL,
        last_validated INTEGER NOT NULL DEFAULT 0,
        status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'ok')),
        duration_ms INTEGER,
        format TEXT,
        modified_at INTEGER NOT NULL DEFAULT 0
    );

CREATE TABLE
    metadata_texts (
        file_id INTEGER NOT NULL,
        key TEXT NOT NULL,
        value TEXT NOT NULL,
        ord INTEGER NOT NULL DEFAULT 0,
        PRIMARY KEY (file_id, key, ord),
        FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
    );

CREATE TABLE
    metadata_pictures (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        file_id INTEGER NOT NULL,
        data BLOB NOT NULL,
        mime_type TEXT NOT NULL,
        picture_type INTEGER NOT NULL DEFAULT 3,
        description TEXT NOT NULL DEFAULT '',
        FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
    );

CREATE INDEX idx_files_status ON files (status);

CREATE INDEX idx_files_format ON files (format);

CREATE INDEX idx_files_modified_at ON files (modified_at);

CREATE INDEX idx_meta_text_key_value ON metadata_texts (key, value);

CREATE INDEX idx_meta_text_file_key ON metadata_texts (file_id, key);

CREATE INDEX idx_pic_file ON metadata_pictures (file_id);