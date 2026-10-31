
CREATE TABLE cloud_accounts (
 user_id TEXT PRIMARY KEY, quota_bytes BIGINT NOT NULL CHECK(quota_bytes >= 0)
);
CREATE TABLE cloud_objects (
 sha256 TEXT PRIMARY KEY, object_key TEXT NOT NULL UNIQUE,
 size BIGINT NOT NULL CHECK(size >= 0), format TEXT NOT NULL,
 duration_ms BIGINT, artwork_key TEXT, unreferenced_at BIGINT,
 created_at BIGINT NOT NULL DEFAULT extract(epoch FROM now())::bigint,
 artwork_size BIGINT CHECK (artwork_size >= 0),
 artwork_sha256 TEXT
);
CREATE TABLE cloud_tracks (
 id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_accounts(user_id),
 object_hash TEXT REFERENCES cloud_objects(sha256) ON DELETE SET NULL,
 document JSONB NOT NULL, deleted BOOLEAN NOT NULL DEFAULT FALSE,
 UNIQUE(user_id,object_hash)
);
CREATE TABLE cloud_playlists (
 id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_accounts(user_id),
 document JSONB NOT NULL, deleted BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE TABLE cloud_changes (
 seq BIGSERIAL PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_accounts(user_id),
 value JSONB NOT NULL
);
CREATE INDEX cloud_changes_user_seq ON cloud_changes(user_id,seq);
CREATE TABLE cloud_uploads (
 id TEXT PRIMARY KEY, user_id TEXT NOT NULL REFERENCES cloud_accounts(user_id),
 operation_id TEXT NOT NULL, request JSONB NOT NULL, stage_key TEXT NOT NULL,
 size BIGINT NOT NULL, state TEXT NOT NULL DEFAULT 'created',
 track_id TEXT REFERENCES cloud_tracks(id), error TEXT,
 expires_at BIGINT NOT NULL, lease_at BIGINT NOT NULL DEFAULT 0,
 UNIQUE(user_id,operation_id)
);
CREATE INDEX cloud_uploads_work ON cloud_uploads(state,lease_at);
CREATE TABLE cloud_object_writes (
 object_key TEXT PRIMARY KEY,
 created_at BIGINT NOT NULL DEFAULT extract(epoch FROM now())::bigint
);
CREATE TABLE cloud_mutations (
 user_id TEXT NOT NULL REFERENCES cloud_accounts(user_id), operation_id TEXT NOT NULL,
 request_hash TEXT NOT NULL, result JSONB NOT NULL,
 PRIMARY KEY(user_id,operation_id)
);
CREATE TABLE cloud_media_variants (
 object_hash TEXT NOT NULL REFERENCES cloud_objects(sha256) ON DELETE CASCADE,
 quality TEXT NOT NULL CHECK(quality IN ('aac_128','aac_256')),
 state TEXT NOT NULL DEFAULT 'queued', object_key TEXT UNIQUE,
 size BIGINT, sha256 TEXT, lease_at BIGINT NOT NULL DEFAULT 0,
 attempts INTEGER NOT NULL DEFAULT 0, retry_at BIGINT NOT NULL DEFAULT 0,
 PRIMARY KEY(object_hash,quality)
);
CREATE INDEX cloud_media_variants_work ON cloud_media_variants(state,retry_at);
