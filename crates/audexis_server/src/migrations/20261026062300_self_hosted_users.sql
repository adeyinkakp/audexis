-- Add migration script here
CREATE TABLE self_hosted_users (
 user_id TEXT PRIMARY KEY REFERENCES cloud_accounts(user_id) ON DELETE CASCADE,
 name TEXT NOT NULL UNIQUE,
 api_key_sha256 TEXT NOT NULL UNIQUE,
 disabled BOOLEAN NOT NULL DEFAULT FALSE,
 created_at BIGINT NOT NULL DEFAULT extract(epoch FROM now())::bigint
);
