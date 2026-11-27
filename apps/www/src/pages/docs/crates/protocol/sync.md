---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Sync and compatibility"
description: "Work with revisions and keep clients compatible."
---

# Sync and compatibility

## Sync changes

Tracks and playlists have revisions. A `Mutation` includes the revision the
client edited so the server can check it against the current version.

A `Changes` response includes `next_cursor` and `has_more`. Clients use these
to request the next page and save their place in the change stream.

The client sync code lives in [Core](/docs/crates/core). The HTTP handlers live
in [Server](/docs/crates/server).

## Changing a shared type

Check the clients and server together when changing these types. A renamed
field or enum variant can change the JSON they exchange. For a new optional
field, consider how an older saved record or response will be read.
