# audexis_server

The audexis cloud library and media server. Built with Axum and PostgreSQL,
with local disk or Backblaze B2 storage for uploaded audio. (inspired by navidrome)

## WHat it does

- Authenticate clients and enforce account storage quotas.
- Accept and verify uploads.
- Synchronize revisioned tracks, metadata, and playlists.
- Serve media links and optional AAC streaming variants.
- Manage self-hosted users and API keys from cli.

## Docker deployment

The image includes the Rust server, FFmpeg for AAC streaming copies, and a
health check.
Install Docker Engine with the Compose plugin on the host, then run from the
repository root:

```sh
cp deploy/.env.example deploy/.env
openssl rand -hex 32
```

Put the generated password in `POSTGRES_PASSWORD` in `deploy/.env`. Configure
the B2 settings and `AUDEXIS_PUBLIC_ORIGIN`. To use the server disk,
uncomment `AUDEXIS_STORAGE_DIR` instead of configuring B2.

```sh
cd deploy
docker compose build api
docker compose up -d --wait
curl --fail http://localhost:3000/health
docker compose exec api audexis_server users create developer --quota 100GB
```

Save the API key printed by the last command. Use `users --help` for account
management. Database migrations run automatically at startup. The health
endpoint returns HTTP 204; it checks HTTP availability, not B2 connectivity.

The API is published on host loopback port 3000. PostgreSQL has no published
port. For public access, put a host reverse proxy in front of
`127.0.0.1:3000`, enable HTTPS, and set `AUDEXIS_PUBLIC_ORIGIN` to that HTTPS
address. Set `AUDEXIS_TRUSTED_PROXIES` only to the proxy's actual source IP as
seen by the container.

To build just the image, use the repository root as the build context:

```sh
docker build -f crates/audexis_server/Dockerfile -t audexis-server:local .
docker run --rm audexis-server:local users --help
```

Build on Linux CI or another build machine if the VPS is short on RAM. On an
Apple Silicon Mac targeting an x86 VPS, use:

```sh
docker buildx build --platform linux/amd64 --load \
  -f crates/audexis_server/Dockerfile -t audexis-server:local .
```

Transfer the image through a private registry or
`docker save`/`docker load`; with the image loaded on the VPS, start Compose
with `docker compose up -d --no-build --wait`.

For updates, rebuild or load the new image and recreate only the API service;
Older binaries may be
incompatible with newer migrations, so take a database backup before updates.

```sh
docker compose logs --tail=100 -f api
docker compose exec -T postgres pg_dump -U audexis -d audexis -Fc > audexis.dump
docker compose up -d --build --no-deps api
```

Copy database backups off the VPS and test restores. Volumes survive
`docker compose down`; `docker compose down -v` deletes them. Existing database
passwords are not changed by editing `POSTGRES_PASSWORD` after init.
Compose restarts exited processes, but an unhealthy health check alone does
not restart a running process.

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
