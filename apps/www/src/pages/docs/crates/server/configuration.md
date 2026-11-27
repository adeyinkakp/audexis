---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Configuration"
description: "Set the database, storage, listener, and account limits."
---

# Configuration

The configuration template is `crates/audexis_server/.env.example`.

| Setting | What it controls |
| --- | --- |
| `DATABASE_URL` | The PostgreSQL connection. |
| `AUDEXIS_BIND` | The address and port the server listens on. |
| `AUDEXIS_PUBLIC_ORIGIN` | The server URL clients can reach. |
| `AUDEXIS_STORAGE_DIR` | A local media directory, used instead of B2. |
| `B2_*` | The bucket, endpoint, region, and credentials for B2 storage. |
| `AUDEXIS_AUTH_ORIGIN` | Optional account verification through the Audexis website. |
| `AUDEXIS_UPLOAD_MODE` | Whether uploads go through the server or directly to storage. |
| `AUDEXIS_QUOTA_BYTES` | The default account storage limit. |
| `AUDEXIS_MAX_FILE_BYTES` | The maximum size of one uploaded file. |
| `AUDEXIS_FFMPEG_PATH` | The FFmpeg executable used for AAC copies. |

For public access, put an HTTPS reverse proxy in front of the server and set
`AUDEXIS_PUBLIC_ORIGIN` to the public HTTPS address. Keep the database and media
storage persistent, and back them up before updates.

## Working directories

Uploads and AAC conversion need local disk space even when the finished media
lives in B2.

| Setting | What it controls |
| --- | --- |
| `AUDEXIS_INGEST_DIR` | Files waiting for upload verification. |
| `AUDEXIS_INGEST_MAX_BYTES` | The ingestion space limit. |
| `AUDEXIS_VARIANT_INPUT_DIR` | Original files kept for AAC conversion. |
| `AUDEXIS_VARIANT_INPUT_CACHE_BYTES` | The size limit for that input cache. |

Use writable paths. For Compose, keep persistent working data under `/app/data`
unless you add another volume. For source runs, relative paths are based on
the directory where you start the server.

## Loading and applying settings

The server first tries `crates/audexis_server/.env`. If that file is absent, it
uses dotenv's normal `.env` lookup. Values already set in the process
environment take precedence.

Restart a source-run server after editing its environment. For Compose,
recreate the API with `docker compose up -d api` from `deploy` so the container
receives the updated values.

`AUDEXIS_QUOTA_BYTES` sets the default for new self-hosted accounts. Use the
[quota command](/docs/crates/server/users#change-the-quota) to update an existing
account.
