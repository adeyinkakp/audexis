---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Requests and library state"
description: "Open the library and send requests from the mobile app."
---

# Requests and library state

## Where to start

The crate lives in `crates/audexis_mobile`. `MobileCore` holds the current
library and playback state. Its `call` method accepts a JSON request and
returns a JSON result or an error.

Open the library before sending other requests:

```json
{
  "op": "open",
  "directory": "/path/to/app-data"
}
```

Use a writable app-data directory. Opening it creates the audio directory and
loads the saved library state.

## Requests

Each request has an `op` field. The request handlers are in `src/requests.rs`
and `src/playback.rs`.

| Operation | What it does |
| --- | --- |
| `songs` | Reads songs from the local library. |
| `snapshot` | Returns the current library and queue state. |
| `changes` | Applies a page of cloud changes. |
| `enqueue`, `select`, `advance` | Updates or moves through the queue. |
| `media_plan` | Works out how to access a track's audio. |
| `commit_download`, `remove_download` | Updates saved download records. |
| `player_load` | Loads an audio source into the player. |
| `player_pause`, `player_resume`, `player_seek` | Controls playback. |

These operations have their own request fields. Check the handler when adding
a call from the native app.
