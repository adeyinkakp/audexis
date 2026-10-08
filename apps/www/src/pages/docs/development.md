---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Development guide"
description:
  "Repository structure, development commands, validation, and documentation
  maintenance."
---

# Development guide

This page is for working on Audexis itself. For everyday use, start with
[Getting started](/docs/getting-started).

## Repository structure

- `apps/desktop`: React and TypeScript desktop interface.
- `apps/desktop/src-tauri`: Rust backend, Tauri application, audio playback,
  metadata codecs, library scanning, and database migrations.
- `apps/www`: Astro website and these Markdown documentation pages.
- `packages/shared`: shared styles and assets.
- `.github/workflows`: validation and release automation.

The desktop frontend communicates with Rust through Tauri commands and listens
for library and playback events. SQLite stores the local library. React Query
maintains frontend query state, and TanStack Router supplies application routes.

Install dependencies from the repository root:

```sh
pnpm install
```

## Run the desktop app

```sh
pnpm --filter desktop tauri dev
```

To create a desktop bundle:

```sh
pnpm --filter desktop tauri build
```

Build availability and signing depend on the host platform and release
configuration.

## Run the website

```sh
pnpm --filter www dev
```

Build and preview it with:

```sh
pnpm --filter www build
pnpm --filter www preview
```

## Validate changes

From the repository root:

```sh
pnpm --filter desktop run validate:frontend
pnpm --filter desktop run validate:rust
```

The frontend validation checks TypeScript, runs frontend tests, and builds the
application. Rust validation covers formatting, compilation, Clippy, and tests.

## Important source areas

- `src-tauri/src/tag_manager`: format detection, frame mappings, reads, writes,
  and write verification.
- `src-tauri/src/commands/library`: frontend-facing library and editing
  commands.
- `src-tauri/src/file_watcher`: file monitoring and indexed metadata.
- `src-tauri/src/audio_player`: decoding, queue, playback state, equalizer, and
  listening tracking.
- `src/components/info`: metadata, artwork, and lyrics interfaces.
- `src/hooks/libraryEvents.ts`: invalidation after library changes.
- `src-tauri/migrations`: ordered database schema and indexing migrations.

Avoid changing stored keys or existing migrations casually. UI display names,
serialized command keys, native tag names, and indexed metadata keys are related
but not always identical.
