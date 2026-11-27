---
layout: ../../../layouts/MarkdownLayout.astro
title: "Audexis — Protocol"
description: "The data types shared by Audexis clients and cloud sync."
---

# Protocol

`audexis_protocol` defines the data that Audexis clients use for cloud sync.
It contains the track, playlist, upload, and edit types, along with shared
equalizer settings and smart playlist rules.

The crate lives in `crates/audexis_protocol`. It uses Serde to serialize and
deserialize its types. It does not make network requests or open a database.

## In this guide

- [Shared types](/docs/crates/protocol/types) — Understand tracks, playlists, uploads, and metadata values.
- [Sync and compatibility](/docs/crates/protocol/sync) — Work with revisions and keep clients compatible.
- [Development](/docs/crates/protocol/development) — Check the crate and generate its Rust API docs.
