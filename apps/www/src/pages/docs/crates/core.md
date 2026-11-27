---
layout: ../../../layouts/MarkdownLayout.astro
title: "Audexis — Core"
description: "How Audexis stores the library, reads metadata, and keeps files in sync."
---

# Core

`audexis_core` handles the music library. It stores tracks in SQLite, watches
music folders, reads and writes tags, and syncs library changes with the server.
The app uses it for library data, artwork, lyrics, and settings.

## In this guide

- [Library and sync](/docs/crates/core/library) — Find the library modules and understand how local and cloud data fit together.
- [Editing metadata](/docs/crates/core/metadata) — Load edits, check for conflicts, and save tags safely.
- [Development](/docs/crates/core/development) — Build the crate and check changes in the desktop app.
