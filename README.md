# Audexis

Audexis is a local-first desktop music player for people who want to keep their
library on their own computer. It uses react for the frontend and rust for the backend.

## Features

- Scan and monitor one or more local music folders
- Browse music by song, album, or artist
- Search and filter the local library
- Create, rename, reorder, and delete playlists
- Mark songs as favorites
- Manage the playback queue
- Shuffle and repeat playback modes
- View listening history and statistics in Rewind
- Read embedded metadata and artwork from common audio formats
- Control playback through native system media controls
- Keep library data locally in SQLite

Audexis currently recognizes MP3, MP2, MP1, FLAC, M4A, M4B, MP4, Ogg, Opus,
OGA, SPX, OGV, MOV, M4V, and QuickTime files. Support can vary depending on the
codec and metadata contained in a file.

## Project status

Audexis is pre-release software. Core library browsing and playback are in
place, but testing, error handling, documentation, and metadata editing are
still being improved before a stable release.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 20 or newer
- [pnpm](https://pnpm.io/) 9
- A stable [Rust toolchain](https://www.rust-lang.org/tools/install)
- The [Tauri 2 system prerequisites](https://v2.tauri.app/start/prerequisites/)
  for your operating system

### Run locally

From the repository root:

```sh
pnpm install
pnpm --filter desktop tauri dev
```

On the first run, choose the folders that contain your music. Audexis will scan
them and build a local library database.

### Build the frontend

```sh
pnpm --filter desktop build
```

### Build the desktop application

```sh
pnpm --filter desktop tauri build
```

## Project structure

```text
audexis/
├── apps/
│   ├── dekstop        Desktop App
│   └── www            Website
└── packages/
    └── shared         Shared Styles
```

## Tech Stack

- [Tauri](https://tauri.app/)
- [React](https://react.dev/) and TypeScript
- [TanStack Router](https://tanstack.com/router) and TanStack Query
- [Rust](https://www.rust-lang.org/)
- [SQLite](https://www.sqlite.org/) through [SQLx](https://github.com/transact-rs/sqlx)
- [Symphonia](https://github.com/pdeljanov/Symphonia)
  and [CPAL](https://github.com/RustAudio/cpal) for audio

## Todo

- Finish in-app metadata editing
- Expand automated test coverage
- Improve handling of renamed, moved, and missing files
- Add lyrics and synchronized lyrics
- Continue expanding metadata and codec compatibility

## License

Audexis is available under the [MIT License](./LICENSE).
