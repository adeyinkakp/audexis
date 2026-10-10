---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Privacy"
description: "Understand local files, library data, online requests, and backup boundaries."
---

# Privacy and storage

## Your music stays where you put it

Audexis reads music from the folders you select. Library browsing and playback
use those local files; adding a folder does not upload the collection to a
hosted library.

Saving tags, artwork, or lyrics changes the source file. Playlists, favorites,
listening history, and view preferences are stored separately as app data.

## Local app data

The app stores its library in SQLite and preferences through its settings store
in the platform’s application-data location. The exact location depends on the
operating system and app identifier.

Library records include paths, indexed metadata, artwork, playlists, favorites,
and listening activity. Settings include choices such as watched folders and
display preferences.

The current desktop source uses `settings.json` for its settings store and
`audexis_main.db` for the library database. These are implementation
names, not a stable export format; they can change between builds.

## Features that use the internet

- Metadata lookup searches iTunes using the search terms you submit.
- Cover lookup downloads artwork from the provider’s image servers.
- Lyrics lookup searches LRCLIB using the entered song information.
- Update check to see if there is a new verision of audexis available.
-

## Logs and support

Error logs can contain local paths and diagnostic information. Review the
material you choose to share with a bug report. You usually do not need to share
a full library database or audio collection to describe an issue.

[Settings](/docs/settings) · [Troubleshooting](/docs/troubleshooting)
