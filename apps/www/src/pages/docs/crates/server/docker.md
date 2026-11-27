---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Docker setup"
description: "Run the server and PostgreSQL with Docker Compose."
---

# Docker setup

The repository includes a Compose setup for the API and PostgreSQL. Run the
first commands from the repository root with Docker Engine and the Compose
plugin installed.

## Create the configuration

```sh
cp deploy/.env.example deploy/.env
openssl rand -hex 32
```

Put the generated value in `POSTGRES_PASSWORD` in `deploy/.env`. The template
uses local storage at `/app/data/media`, which is inside the persistent
`server_data` volume. PostgreSQL uses its own `postgres_data` volume.

Set `AUDEXIS_PUBLIC_ORIGIN` to the URL your clients will use. For a local trial,
`http://localhost:3000` works on the same machine. A phone or another computer
needs an address it can reach.

## Start the services

```sh
cd deploy
docker compose build api
docker compose up -d --wait
curl --fail http://localhost:3000/health
```

Compose creates the database and starts the API after PostgreSQL is ready.
The server applies database migrations on startup.

Create your first account:

```sh
docker compose exec api audexis_server users create developer --quota 100GB
```

Save the printed API key. See [Users and API keys](/docs/crates/server/users)
for the other account commands.

## Ports and public access

The API is published at `127.0.0.1:3000` on the host. PostgreSQL has no
published port. For remote clients, put a reverse proxy in front of the API
and follow [Public access](/docs/crates/server/public-access).

## Everyday commands

Run these from `deploy`:

```sh
docker compose ps
docker compose logs --tail=100 -f api
docker compose restart api
docker compose stop
docker compose up -d --wait
```

After changing `deploy/.env`, recreate the API with `docker compose up -d api`.
A plain restart does not apply new container environment settings.

Before upgrading, follow [Updates and backups](/docs/crates/server/backups).
