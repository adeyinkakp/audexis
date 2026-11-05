# audexis_server

The Audexis cloud library and media server. Built with Axum and PostgreSQL,
with local disk or Backblaze B2 storage for uploaded audio. (inspired by navidrome)

## WHat it does

- Authenticate clients and enforce account storage quotas.
- Accept and verify uploads.
- Synchronize revisioned tracks, metadata, and playlists.
- Serve media links and optional AAC streaming variants.
- Manage self-hosted users and API keys from cli.

# TODO

- ADD dockerfile

## Development

Run these commands from the repository root with Rust and Cargo installed.
Create a PostgreSQL database, then copy the configuration template:

```sh
cp crates/audexis_server/.env.example crates/audexis_server/.env
```

Edit the copied file for your database and storage. For local development,
set `DATABASE_URL`, `AUDEXIS_PUBLIC_ORIGIN=http://localhost:3000`, and
`AUDEXIS_STORAGE_DIR=./data/media`. Leave `AUDEXIS_AUTH_ORIGIN` empty to use
self-hosted accounts. Database migrations run automatically on connection.

```sh
cargo run -p audexis_server -- users --help
cargo run -p audexis_server -- users create developer --quota 100GB
cargo run -p audexis_server
```

The default listener is `127.0.0.1:3000`; `GET /health` returns HTTP 204.
Keep the API key returned when creating a user for connecting a client.

## Configuration

See [.env.example](.env.example) for the full configuration template.

| Setting                                         | Purpose                                                  |
| ----------------------------------------------- | -------------------------------------------------------- |
| `DATABASE_URL`                                  | PostgreSQL connection                                    |
| `AUDEXIS_BIND`, `AUDEXIS_PUBLIC_ORIGIN`         | Listener and client-facing server URL                    |
| `AUDEXIS_STORAGE_DIR`                           | Local media storage instead of B2                        |
| `B2_*`                                          | Private bucket endpoint, region, bucket, and credentials |
| `AUDEXIS_AUTH_ORIGIN`                           | Optional website-based account verification              |
| `AUDEXIS_UPLOAD_MODE`                           | Server ingestion or direct uploads                       |
| `AUDEXIS_QUOTA_BYTES`, `AUDEXIS_MAX_FILE_BYTES` | Account and upload limits                                |
| `AUDEXIS_FFMPEG_PATH`                           | Optional FFmpeg executable for AAC variants              |

The server tris to load `crates/audexis_server/.env` first, falling back to
env on device when that file is absent.

## Code map

| Path                               | Purpose                                    |
| ---------------------------------- | ------------------------------------------ |
| `src/main.rs`, `src/config.rs`     | Startup and environment configuration      |
| `src/router.rs`, `src/routes/`     | HTTP routing and API handlers              |
| `src/auth.rs`, `src/users.rs`      | Request authentication and user management |
| `src/db.rs`, `src/migrations/`     | PostgreSQL operations and migrations       |
| `src/storage/`                     | Storage backends                           |
| `src/ingest.rs`, `src/transfer.rs` | Upload processing and media transfer       |
| `src/variants.rs`                  | Streaming variant generation               |

## Checks

```sh
cargo check -p audexis_server
cargo test -p audexis_server
```
