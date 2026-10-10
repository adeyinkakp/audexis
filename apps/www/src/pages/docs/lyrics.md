---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Lyrics and synchronization"
description:
  "Edit plain lyrics, work with LRC timestamps, and find lyrics online."
---

# Lyrics and synchronization

Audexis supports both plain lyrics and synchronized lyrics. They are separate
fields: editing one does not automatically replace the other.

## Read lyrics during playback

Open Lyrics from the playback bar, View menu, or expanded player. Synchronized
lyrics can follow the active playback position. Availability depends on the
embedded lyrics in the current song.

A song may have no lyrics, plain lyrics only, synchronized lyrics only, or both.
An empty panel does not mean the audio file is broken.

## Edit plain lyrics

1. Open **Get Info → Lyrics**.
2. Enter or paste the text, preserving the line breaks you want.
3. Choose **Save Lyrics**.

Clear the text and save to remove that lyrics value. Reset discards your unsaved
draft. Lyrics edits are saved into supported audio files, not only into the
visible panel.

## Synchronized lyrics

Open **Get Info → Synchronized Lyrics**. The editor provides timing controls and
a raw LRC view.

LRC associates lyric lines with timestamps:

```text
[00:12.500]The first line begins here
[00:17.250]The next line begins here
[00:23.000]Another line follows
```

The first component is minutes, followed by seconds and a fractional part. A
line’s timestamp describes when it begins; it is not the duration of that line.

Use the line timing controls to adjust timestamps, move a line earlier or later
by 250 milliseconds, or set its time from the preview position. Use the preview
to check alignment against the correct recording. A live version or alternate
edit often needs different timestamps.

Invalid synchronized text must be corrected before saving. The raw text is
useful for pasting an existing LRC document; the line editor is useful for
adjusting timing.

## Find lyrics online

Choose **Find lyrics online** to search LRCLIB by title and artist. Inspect the
song name, artist, album, duration, and available lyric types. A result can have
plain lyrics, synchronized lyrics, both, or neither; instrumental results may
have no lyric text.

Review the preview before applying. Applying a result saves the chosen lyrics to
the selected files, so confirm that the recording and selection are correct.
Search requires internet access.

## Multiple files and entries

A mixed selection means the songs do not share the same lyric content. Saving
applies the edited text to the selected songs. If a file contains multiple
entries for the edited lyrics field, this editor saves one replacement entry
rather than maintaining a language-by-language collection.

## Format limits

Lyrics editing is available for supported ID3v2, FLAC, Ogg, and MP4/iTunes tag
formats. Other applications may interpret synchronized lyrics differently,
especially outside ID3. See [Formats and compatibility](/docs/formats).

[Playback](/docs/playback) · [Metadata editing](/docs/metadata)
