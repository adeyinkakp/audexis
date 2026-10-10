---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Frequently asked questions"
description: "Answers about files, offline use, playlists, metadata, and player views."
---

# Frequently asked questions

## Does Audexis copy my music into its own folder?

No. It indexes the music folders you choose and plays those source files. Keep
the originals available, including any external drives that hold them.

## Can I use it offline?

Yes, for local library browsing, playback, and manual editing. Online metadata,
artwork, and lyrics lookup require internet access.

## Does editing metadata change the audio file?

Yes. Save/apply actions write supported tags, artwork, or lyrics into the source
file. They are not just display overrides in the library database.

## Does hiding a field remove its value?

No. Settings → Metadata controls which regular fields are shown. Songs column
settings control what the table displays. Deleting a stored value requires an
actual metadata edit and save.

## What is the difference between a title and a key for a custom field?

The title is the friendly label shown in the editor and table. The key is the
name stored in the audio file. The per-song value is separate from both.

## Does removing a playlist delete the songs?

No. It removes the saved playlist. Removing a song from a playlist removes that
entry, not its audio file. Removing library folders or missing-file records
affects library membership and can also affect playlist references.

## Where should I report a problem?

Use the project’s [issue tracker](https://github.com/adeyinkakp/audexis/issues).
Include your operating system, app version, steps to reproduce, and any relevant
log message. See [Troubleshooting](/docs/troubleshooting).
