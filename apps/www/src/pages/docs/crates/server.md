---
layout: ../../../layouts/MarkdownLayout.astro
title: "Audexis — Server"
description: "Run the Audexis cloud library and media server."
---

# Server

`audexis_server` stores your cloud library and serves music to connected
clients. It uses Axum for HTTP, PostgreSQL for library data, and either local
disk or Backblaze B2 for audio files.

It handles uploads, storage limits, metadata and playlist changes, and media
links. FFmpeg can create AAC copies for streaming.

## In this guide

- [Run locally](/docs/crates/server/installation) — Start the server with Rust and PostgreSQL.
- [Configuration](/docs/crates/server/configuration) — Set the database, storage, listener, and account limits.
- [HTTP API](/docs/crates/server/api) — Find the routes used by clients.
- [Development](/docs/crates/server/development) — Find the server code and run its checks.

## Hosting and maintenance

- [Docker setup](/docs/crates/server/docker)
- [Users and API keys](/docs/crates/server/users)
- [Media storage](/docs/crates/server/storage)
- [Public access](/docs/crates/server/public-access)
- [Uploads and limits](/docs/crates/server/uploads)
- [Streaming and AAC](/docs/crates/server/streaming)
- [Updates and backups](/docs/crates/server/backups)
- [Troubleshooting](/docs/crates/server/troubleshooting)
