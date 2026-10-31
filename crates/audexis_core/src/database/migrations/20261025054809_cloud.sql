-- Add migration script here
CREATE TRIGGER IF NOT EXISTS cloud_playlist_delete_outbox
BEFORE DELETE ON playlists
BEGIN
 INSERT INTO cloud_outbox(operation_id,connection_id,entity_key,payload)
 SELECT lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
              substr(hex(randomblob(2)),2) || '-8' ||
              substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
        connection_id, 'playlist-delete:' || cloud_id,
        json_object('job','mutate','file_id',NULL,'playlist_id',NULL,
          'request',json_object('operation_id','','base_revision',revision,
            'edit',json_object('kind','delete_playlist','playlist_id',cloud_id)))
 FROM cloud_playlist_links WHERE playlist_id=OLD.id
 ON CONFLICT(connection_id,entity_key) DO NOTHING;

 UPDATE cloud_outbox SET payload=json_set(payload,'$.request.operation_id',operation_id)
 WHERE connection_id IN (SELECT connection_id FROM cloud_playlist_links WHERE playlist_id=OLD.id)
   AND json_extract(payload,'$.request.operation_id')='';
END;
ALTER TABLE files ADD COLUMN cloud_connection_id TEXT REFERENCES connections(id) ON DELETE CASCADE;
CREATE INDEX files_cloud_connection ON files(cloud_connection_id) WHERE cloud_connection_id IS NOT NULL;
CREATE TABLE cloud_library_session (
    slot INTEGER PRIMARY KEY CHECK(slot=1),
    connection_id TEXT NOT NULL REFERENCES connections(id) ON DELETE CASCADE
);

CREATE TABLE cloud_library_pending (slot INTEGER PRIMARY KEY CHECK(slot=1));
INSERT INTO cloud_library_pending VALUES(1);
CREATE TRIGGER cloud_library_track_insert AFTER INSERT ON cloud_tracks BEGIN
    INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
END;
CREATE TRIGGER cloud_library_track_update AFTER UPDATE ON cloud_tracks BEGIN
    INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
END;
CREATE TRIGGER cloud_library_local_insert AFTER INSERT ON files WHEN NEW.cloud_connection_id IS NULL BEGIN
    INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
END;
CREATE TRIGGER cloud_library_local_update AFTER UPDATE OF hash,status,missing_since,path ON files WHEN NEW.cloud_connection_id IS NULL BEGIN
    INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
END;
CREATE TRIGGER cloud_library_local_delete AFTER DELETE ON files WHEN OLD.cloud_connection_id IS NULL BEGIN
    INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
END;
CREATE TABLE cloud_downloads (
    connection_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    size INTEGER NOT NULL,
    PRIMARY KEY(connection_id,track_id),
    FOREIGN KEY(connection_id,track_id) REFERENCES cloud_tracks(connection_id,id) ON DELETE CASCADE
);
CREATE TABLE cloud_downloads_scanned (
    connection_id TEXT PRIMARY KEY REFERENCES connections(id) ON DELETE CASCADE
);
ALTER TABLE playlists ADD COLUMN cloud_connection_id TEXT REFERENCES connections(id) ON DELETE CASCADE;
CREATE INDEX playlists_cloud_connection ON playlists(cloud_connection_id) WHERE cloud_connection_id IS NOT NULL;
CREATE TRIGGER cloud_library_playlist_insert AFTER INSERT ON cloud_playlists BEGIN
 INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
END;
CREATE TRIGGER cloud_library_playlist_update AFTER UPDATE ON cloud_playlists BEGIN
 INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
END;
INSERT INTO cloud_library_pending VALUES(1) ON CONFLICT(slot) DO NOTHING;
CREATE VIEW library_playlists AS SELECT p.* FROM playlists p WHERE p.cloud_connection_id IS NULL OR p.cloud_connection_id=(SELECT connection_id FROM cloud_library_session WHERE slot=1);
