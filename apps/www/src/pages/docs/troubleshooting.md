---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Troubleshooting"
description: "Resolve scanning, playback, metadata, artwork, and startup problems."
---

# Troubleshooting

## Songs do not appear

Confirm that the music folder is saved under Settings → Library and is currently
accessible. Reconnect external storage if necessary, then rescan. Wait for the
scan to finish before checking counts.

A recognized extension is only the first step. Unsupported codecs, malformed
tags, and unreadable files can prevent successful indexing. Check Help → View
Logs for a specific error.

## A moved file is missing

Open File → Library → Missing Files and use the relink action to choose the file
at its new path. For an unavailable drive, reconnect and rescan first. Remove
the missing record only when you no longer want that library entry.

## A song will not play

1. Confirm that its source file still exists and can be read.
2. Try another track to distinguish a file-specific problem from an output
   problem.
3. Check the system’s audio output and volume.
4. Inspect logs for a decoding or audio-device error.
5. Include the format and codec when reporting a reproducible failure.

A matching extension does not guarantee a supported codec inside the container.

## Sound is distorted or unexpectedly quiet

Bypass the equalizer and compare. If that fixes it, reduce large boosts and
leave headroom with the preamp control. If it persists, check the source
recording and the system audio path.

## A metadata save failed

Read the failure message before retrying. Check that the source exists, is
writable, and has enough available disk space for a temporary working copy.

Try one file first. A batch can contain different formats and permission states,
so some files may save while others fail. Successful files have already changed.

If the error says a write succeeded but the library cache could not refresh, do
not assume the file remained unchanged. Reopen it or rescan to check its current
values.

## Artwork is missing or wrong

Open Get Info → Artwork and inspect the embedded images. If there are several,
check their order and picture types. If importing from online lookup, verify
that the result is the right album edition.

Unsupported image data or unavailable artwork URLs can fail independently of
audio playback.

## Lyrics are missing or out of time

Plain and synchronized lyrics are separate. Check the appropriate tab, then
verify that any online result matches the exact recording. Edit LRC timestamps
if the arrangement, intro, or track length differs.

## Online lookup fails

Check your connection, shorten the search, and try again later if the provider
is unavailable. Local playback and manual editing can still be used. Do not
repeatedly apply a result without checking whether an earlier save succeeded.

## The app cannot start

Read the startup error and use the offered relaunch action. Preserve your
app-data directory before attempting a reset or database repair. Deleting the
database can discard playlists, favorites, and listening history.

## Report a reproducible issue

Include:

- Audexis version and operating system.
- The exact action and expected result.
- What happened instead, including the full visible error.
- Whether one file or many are affected.
- File extension, codec, and tag format when known.
- Relevant log excerpts.

[Report an issue](https://github.com/adeyinkakp/audexis/issues) ·
[Privacy](/docs/privacy-and-data)
