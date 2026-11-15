---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Development guide"
description:
  "Repository structure, development commands, validation, and documentation
  maintenance."
---

# Development guide

This page is for working on audexis itself. For everyday use, start with
[Getting started](/docs/apps/desktop).

## Repository structure

- `apps/desktop`: Rust desktop application and Freya interface.
- `crates/audexis_core`: library database, metadata, file watching, and cloud sync.
- `crates/audexis_audio`: decoding, audio output, playback state, and equalizer.
- `crates/audexis_protocol`: shared cloud records and wire types.
- `crates/audexis_server`: cloud API and media storage.
- `apps/www`: Astro website and these Markdown documentation pages.
- `.github/workflows`: validation and release automation.

The desktop interface calls shared Rust services directly. SQLite stores the
local library, and library and playback events keep the interface up to date.

## Run the desktop app

Install a Rust toolchain with support for edition 2024. From the repository
root, run:

```sh
cargo run -p audexis
```

Use copies of your music when developing features that write tags or change
files.

Build the release executable with:

```sh
cargo build -p audexis --release
```

To create desktop packages, install Cargo Packager and run:

```sh
cargo packager --release --config apps/desktop/Packager.json
```

Build availability and signing depend on the host platform and release
configuration.

## Run the website

Install the website dependencies and start its development server:

```sh
cd apps/www
pnpm install
pnpm dev
```

From the same directory, build and preview it with:

```sh
pnpm build
pnpm preview
```

Account and billing features need the environment configuration described in
`apps/www/README.md`.

## Validate changes

From the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Use `cargo check -p audexis` for a focused desktop compilation check. Build the
website after changing documentation to catch Markdown and layout errors.

## Important source areas

- `apps/desktop/src/ui`: pages, settings, menus, and metadata editors.
- `apps/desktop/src/services`: account credentials and playback integration.
- `apps/desktop/src/cloud_auth`: browser sign-in callback handling.
- `crates/audexis_core/src/tag_manager`: format detection and tag reads and writes.
- `crates/audexis_core/src/database`: library queries and data operations.
- `crates/audexis_core/src/cloud_sync`: uploads, downloads, and library sync.
- `crates/audexis_audio/src`: playback and audio processing.

Avoid changing stored keys or existing migrations casually. UI display names,
serialized keys, native tag names, and indexed metadata keys are related but
not always identical.
