-- Add migration script here
CREATE TABLE collection_search_documents (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('album', 'artist')),
    name TEXT NOT NULL,
    artist TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_collection_search_file
ON collection_search_documents(file_id);

CREATE VIRTUAL TABLE collection_search USING fts5(
    name,
    artist,
    content = 'collection_search_documents',
    content_rowid = 'id',
    tokenize = 'unicode61 remove_diacritics 2',
    prefix = '2 3'
);

CREATE TRIGGER collection_search_insert
AFTER INSERT ON collection_search_documents
BEGIN
    INSERT INTO collection_search(rowid, name, artist)
    VALUES (NEW.id, NEW.name, NEW.artist);
END;

CREATE TRIGGER collection_search_delete
AFTER DELETE ON collection_search_documents
BEGIN
    INSERT INTO collection_search(
        collection_search, rowid, name, artist
    )
    VALUES ('delete', OLD.id, OLD.name, OLD.artist);
END;

CREATE TRIGGER collection_search_update
AFTER UPDATE ON collection_search_documents
BEGIN
    INSERT INTO collection_search(
        collection_search, rowid, name, artist
    )
    VALUES ('delete', OLD.id, OLD.name, OLD.artist);

    INSERT INTO collection_search(rowid, name, artist)
    VALUES (NEW.id, NEW.name, NEW.artist);
END;

INSERT INTO media_search_pending(file_id)
SELECT id FROM files WHERE TRUE
ON CONFLICT(file_id) DO NOTHING;
