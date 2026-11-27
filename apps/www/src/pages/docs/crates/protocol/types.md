---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Shared types"
description: "Understand tracks, playlists, uploads, and metadata values."
---

# Shared types

## Main types

| Type | What it contains |
| --- | --- |
| `Metadata` | A map of tag names to lists of values, plus the favorite state. |
| `Track` | A track's ID, revision, file information, metadata, and deletion state. |
| `Playlist` | A playlist's ID, revision, name, tracks, and optional smart rules. |
| `Changes` | A page of changes, the next cursor, and whether more pages remain. |
| `UploadRequest` and `Upload` | Upload details and the current upload state. |
| `Mutation` | An operation ID, the revision being edited, and the requested edit. |
| `MediaUrl` | An audio URL, hash, size, quality, and preparation state. |
| `Usage` | Used, reserved, and allowed storage in bytes. |

## Metadata values

Tags use `BTreeMap<String, Vec<String>>`. A field can have more than one value,
so keep separate artists or genres as separate entries in the list.

`Metadata::validate` checks field counts, value sizes, null characters, and the
total size of a track's metadata. `Metadata::for_sync` removes null characters
from tag names and values before sync.
