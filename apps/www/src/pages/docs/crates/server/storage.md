---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Media storage"
description: "Choose local disk or Backblaze B2 for uploaded music."
---

# Media storage

The server stores library records in PostgreSQL and audio files in a separate
storage backend. It supports local disk and Backblaze B2.

## Local disk

Set a writable media directory and the server's public URL:

```dotenv
AUDEXIS_STORAGE_DIR=/app/data/media
AUDEXIS_PUBLIC_ORIGIN=https://music.example.com
```

The path above is for the supplied Docker image. In a source checkout, you can
use `./data/media` instead. Relative paths are resolved from the server's
working directory.

## Backblaze B2

Remove `AUDEXIS_STORAGE_DIR` from the environment to use B2. Leaving it defined,
even alongside B2 settings, selects local storage.

```dotenv
B2_ENDPOINT=https://s3.us-west-004.backblazeb2.com
B2_REGION=us-west-004
B2_BUCKET=your-private-bucket
B2_KEY_ID=your-application-key-id
B2_APPLICATION_KEY=your-application-key
```

Use your bucket's actual endpoint and region. Keep the bucket private and give
the application key the access needed to read, write, list, and delete its
files. The server returns signed media URLs to authenticated clients.

## Temporary disk space

B2 storage does not remove the need for disk space on the server. Upload
verification and AAC generation use local working directories. Check
`AUDEXIS_INGEST_DIR`, `AUDEXIS_INGEST_MAX_BYTES`,
`AUDEXIS_VARIANT_INPUT_DIR`, and `AUDEXIS_VARIANT_INPUT_CACHE_BYTES` when
planning space.

## Changing storage

Changing environment variables selects a different backend; it does not move
existing media. Back up the database and media before moving a library, and
keep the object paths the database refers to available in the new location.

See [Updates and backups](/docs/crates/server/backups) for what to preserve.
