---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Library and sync"
description: "Find the library modules and understand how local and cloud data fit together."
---

# Library and sync

## Where to start

The crate lives in `crates/audexis_core`. Its public modules are listed in
`src/lib.rs`.

| Module | What it does |
| --- | --- |
| `database` | Stores and queries songs, albums, artists, and playlists. |
| `file_watcher` | Scans music folders and picks up file changes. |
| `library_files` | Keeps track of file locations and availability. |
| `tag_manager` | Reads and writes tags for each supported format. |
| `metadata_editor` | Loads editable metadata, checks changes, and saves files. |
| `cloud_sync` | Handles cloud library changes, uploads, and downloads. |
| `artwork` and `lyrics` | Loads artwork and lyrics for the app. |
| `config` | Loads and saves user settings. |

## Library changes

The database holds the library's indexed metadata. The audio files remain in
the folders the user added. The file watcher updates the index when those files
change, and the app subscribes to library updates to refresh its views.

Cloud sync uses the shared types in [Protocol](/docs/crates/protocol). The
[Server](/docs/crates/server) stores the remote library; this crate handles the
client side.
