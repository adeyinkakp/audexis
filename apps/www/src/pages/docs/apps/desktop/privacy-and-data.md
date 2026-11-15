---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Privacy"
description: "Understand local files, library data, online requests, and backup boundaries."
---

# Privacy and storage

## Your music stays where you put it

Audexis reads music from the folders you select. Library browsing and playback
use those local files. Optional [Cloud sync](/docs/apps/desktop/cloud-sync)
uploads music and library data to the connected server and supports streaming
and downloads.

Saving tags, artwork, or lyrics changes the source file. Playlists, favorites,
listening history, and view preferences are stored separately as app data.

## Local app data

The app stores its library in SQLite and preferences through its settings store
in the platform’s application-data location. The exact location depends on the
operating system and app identifier.

Library records include paths, indexed metadata, artwork, playlists, favorites,
and listening activity. Settings include choices such as watched folders and
display preferences.

The current desktop source uses `user_config.json` for its settings store and
`audexis_test.db` for the library database. These are implementation
names, not a stable export format; they can change between builds.

## Features that use the internet

- Metadata lookup searches iTunes using the search terms you submit.
- Cover lookup downloads artwork from the provider’s image servers.
- Lyrics lookup searches LRCLIB using the entered song information.
- Update checks contact the release service to look for new versions.
- Cloud sign-in and sync contact audexis Cloud or your configured server.
  Uploads send audio files and library metadata; streaming and downloads retrieve
  cloud audio and artwork.

Cloud credentials use the operating system’s keyring when available. If it is
unavailable, the connection lasts for the current session only. Logging out
stops access through the app but does not erase cached library data, downloads,
or the server-side library.

## Logs and support

Error logs can contain local paths and diagnostic information. Review the
material you choose to share with a bug report. You usually do not need to share
a full library database or audio collection to describe an issue.

[Settings](/docs/apps/desktop/settings) · [Troubleshooting](/docs/apps/desktop/troubleshooting)
