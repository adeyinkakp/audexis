---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Formats and compatibility"
description:
  "Distinguish recognized containers, audio decoding, and editable metadata
  formats."
---

# Formats and compatibility

A filename extension, an audio codec, and a metadata format describe different
things. For example, an `.m4a` container can contain audio and iTunes-style
metadata, while an `.mp3` file can carry different ID3 tag versions.

## Recognized files

The library recognizes MP3, MP2, MP1, FLAC, M4A, M4B, MP4, Ogg, Opus, OGA, SPX,
OGV, MOV, M4V, and QuickTime files.

Recognition means the scanner considers the file. Successful playback still
depends on the actual contained codec and file structure. A recognized video
container does not imply that Audexis provides a video player.

Do not infer complete compatibility from a filename alone. A damaged file,
unsupported codec, encrypted media, or unusual tag structure can still fail.

## Editable tag families

- **ID3v2.2, v2.3, and v2.4:** regular metadata, custom text and URL frames,
  artwork, and supported lyrics editing.
- **FLAC:** Vorbis-style comments and embedded artwork, with supported lyrics
  fields.
- **Ogg/Vorbis and Ogg/Opus:** comment-based metadata and supported embedded
  image/lyrics conventions.
- **MP4/iTunes:** recognized atoms and freeform text fields, cover artwork, and
  supported lyrics fields.
- **Legacy ID3v1:** a small fixed set of basic fields; no general custom-field
  support.

Available editing controls and save errors are the authority for an individual
file. A container may be playable while a particular tag operation is
unavailable.

## Regular fields without a native equivalent

Audexis prefers the format’s native field mapping. For supported formats, some
regular fields without a usable native mapping are stored as named custom text
entries and exposed again as regular fields when read.

This preserves a place to store the value, but another player may show it as
custom metadata or ignore it. A field being editable in Audexis does not
guarantee identical interpretation by every application.

## Custom URL fields

ID3 supports distinct user-defined URL frames. FLAC, Ogg, and MP4 store those
custom URL values as text. The URL type in a saved Audexis definition does not
add a new native URL-frame type to those formats.

## Multiple values, artwork, and binary frames

A field can permit multiple logical values, but the native representation
differs between formats and tag versions. Artwork descriptions and picture types
are also not equally expressive in every container.

Binary frame types are not interchangeable with text. Their presence in the
field catalog does not make them editable through an ordinary text box. Use the
specialized editor where one exists.

other player you rely on. Keep backups of files you cannot replace. Audexis
performs write verification, but this is not an automatic version history.

[Metadata reference](/docs/metadata-reference) ·
[Custom fields](/docs/custom-fields)
