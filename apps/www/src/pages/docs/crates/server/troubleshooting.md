---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Troubleshooting"
description: "Find common setup, upload, and playback problems."
---

# Troubleshooting

Start with the API logs. With Compose, run from `deploy`:

```sh
docker compose ps
docker compose logs api
docker compose logs postgres
```

## The server will not start

Check `DATABASE_URL` and confirm PostgreSQL is running. For Compose, make sure
`POSTGRES_PASSWORD` is set in `deploy/.env` before starting the services.

For local storage, set `AUDEXIS_STORAGE_DIR` to a writable directory. For B2,
remove that variable and provide all the bucket settings. A partial B2
configuration will not start a working storage backend.

## A client cannot connect

Try the public `/health` URL from the client device. If it fails, check the
hostname, HTTPS proxy, and network access before changing the API key.

If health works but authentication fails, check the key and whether the
account is disabled. Resetting a key invalidates the previous one.

## Uploads stall or fail

Check free disk space, the ingestion limit, the account quota, and the maximum
file size. Pending uploads reserve storage and count toward the three-upload
limit. A reverse proxy can also reject a file before it reaches the API.

For B2 errors, check the bucket, region, key permissions, and storage-provider
limits. A storage cooldown needs time to expire; repeated retries do not clear
it.

## Playback links do not work

Check `AUDEXIS_PUBLIC_ORIGIN`. An address containing `localhost` will not work
from another device. Ask the server for a fresh media URL instead of retrying
an old signed link indefinitely.

If original audio plays but an AAC copy is not ready, check FFmpeg and the
variant worker logs. See [Streaming and AAC](/docs/crates/server/streaming).

## An edit reports a conflict

Another device may have changed the same item. Pull the latest library changes
and resolve the edit against that version. Retrying the old revision does not
resolve the conflict.

## Health is OK but something still fails

`/health` only confirms that HTTP requests reach the server. It does not check
B2, account authentication, or whether a particular audio file is available.
Use the log entry for the failing operation to narrow it down.
