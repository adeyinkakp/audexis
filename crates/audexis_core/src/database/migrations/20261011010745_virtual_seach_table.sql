-- Add migration script here
CREATE VIRTUAL TABLE media_search USING fts5(
    title,
    album_artist,
    artist,
    album,
    lyrics,
    genre,
    file_name,
    tokenize = 'unicode61 remove_diacritics 2',
    prefix = '2 3'
);

CREATE TABLE media_search_pending (
    file_id INTEGER PRIMARY KEY
);

INSERT INTO media_search_pending (file_id)
SELECT id FROM files;

CREATE TRIGGER media_search_metadata_insert
AFTER INSERT ON metadata_texts
BEGIN
    INSERT INTO media_search_pending (file_id)
    VALUES (NEW.file_id)
    ON CONFLICT (file_id) DO NOTHING;
END;

CREATE TRIGGER media_search_metadata_update
AFTER UPDATE ON metadata_texts
BEGIN
    INSERT INTO media_search_pending (file_id)
    VALUES (OLD.file_id), (NEW.file_id)
    ON CONFLICT (file_id) DO NOTHING;
END;

CREATE TRIGGER media_search_metadata_delete
AFTER DELETE ON metadata_texts
BEGIN
    INSERT INTO media_search_pending (file_id)
    VALUES (OLD.file_id)
    ON CONFLICT (file_id) DO NOTHING;
END;

CREATE TRIGGER media_search_file_insert
AFTER INSERT ON files
BEGIN
    INSERT INTO media_search_pending (file_id)
    VALUES (NEW.id)
    ON CONFLICT (file_id) DO NOTHING;
END;

CREATE TRIGGER media_search_file_rename
AFTER UPDATE OF file_name ON files
WHEN OLD.file_name IS NOT NEW.file_name
BEGIN
    INSERT INTO media_search_pending (file_id)
    VALUES (NEW.id)
    ON CONFLICT (file_id) DO NOTHING;
END;

CREATE TRIGGER media_search_file_delete
AFTER DELETE ON files
BEGIN
    DELETE FROM media_search WHERE rowid = OLD.id;
    DELETE FROM media_search_pending WHERE file_id = OLD.id;
END;
