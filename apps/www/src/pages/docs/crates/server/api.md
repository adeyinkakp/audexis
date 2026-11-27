---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — HTTP API"
description: "Find the routes used by clients."
---

# HTTP API

## HTTP routes

The client API is under `/api` and requires authentication.

| Route | What it does |
| --- | --- |
| `GET /api/session` | Reads the current session. |
| `GET /api/usage` | Reads account storage usage. |
| `GET /api/sync/changes` | Gets a page of library changes. |
| `POST /api/sync/mutations` | Applies a library edit. |
| `POST /api/uploads` | Starts an upload. |
| `GET /api/uploads/{id}` | Checks an upload. |
| `DELETE /api/uploads/{id}` | Cancels an upload. |
| `POST /api/uploads/{id}/complete` | Submits an upload for completion. |
| `GET /api/tracks/{id}/{kind}` | Requests a media link. |

See [Protocol](/docs/crates/protocol) for the shared request and response types.

## Authentication

Send the account's API key as a bearer token. For example, with your key stored
in the shell variable `AUDEXIS_API_KEY`:

```sh
curl --fail \
  -H "Authorization: Bearer $AUDEXIS_API_KEY" \
  https://music.example.com/api/session
```

Use the same header for the other `/api` routes. Upload and media URLs returned
by the API have their own authorization in the URL; use the URL the server
returns for that transfer.

## Reading changes

Request `/api/sync/changes?after=0` for the first page. Apply the returned changes
and save `next_cursor`. If `has_more` is true, request another page using that
cursor as `after`.

Pages are limited by both change count and response size. Do not assume that a
short page means sync is finished; use `has_more`.

## Sending edits

A mutation contains an operation ID, the item's `base_revision`, and the edit.
If another device has already changed the item, the server returns a conflict.
Fetch the current version and resolve the edit before submitting it again.

See [Sync and compatibility](/docs/crates/protocol/sync) for the shared types.
