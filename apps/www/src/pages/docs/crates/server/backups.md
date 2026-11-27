---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Updates and backups"
description: "Preserve the database and music when updating the server."
---

# Updates and backups

Back up both PostgreSQL and the media storage. The database contains accounts,
playlists, metadata, and references to the audio files. A database dump alone
does not contain the music.

## Back up PostgreSQL

From `deploy`, write a dump to a file on the host:

```sh
docker compose exec -T postgres pg_dump -U audexis -d audexis -Fc > audexis.dump
```

For a backup that matches a copy of local media, stop the API while taking both:

```sh
docker compose stop api
```

Keep PostgreSQL running for the dump. Copy the media from the `server_data`
volume using your backup tool, then bring the API back with
`docker compose up -d api`.

Keep a copy of `deploy/.env` somewhere private too. Store backups away from the
server and check that you can restore them to a separate installation.

## Update the API

After backing up, run from `deploy`:

```sh
docker compose up -d --build --no-deps api
docker compose logs --tail=100 api
```

Database migrations run on startup. Check the logs, the health endpoint, and
playback from a client after the update.

An older binary may not work with a database that has newer migrations. Keep
the matching database and media backup if you need to return to an earlier
version.

## Keep volumes

`docker compose down` keeps named volumes. Adding `-v` deletes them, including
the database and locally stored media. Do not use that option for a normal
restart or update.

Editing `POSTGRES_PASSWORD` in the environment does not change the password
inside an already initialized PostgreSQL volume.
