# Audexis NEXT GEN

I want audexis to use least amount of memeory as possible so i'm remaking it
in freya
and also adding a server to manage library

Audexis is a local-first desktop music player for people who want to keep their
library on their own computer. It uses freya for ui

## Features

- Scan and monitor one or more local music folders
- Browse music by song, album, or artist
- Search and filter the local library
- Manage the playback queue
- Shuffle and repeat playback modes
- Control playback through native system media controls
- Keep library data locally in SQLite
- Synced and unsynced lyrics
- Batch Editing up to 200 Songs
- Custom Metadata fields
- Create, rename, and delete playlists
- Read embedded metadata and artwork from common audio formats
- - Mark songs as favorites

## Todo

- Shuffle and repeat playback modes
- View listening history and statistics in Rewind
- 10 band equalizer window
- settings window
- logs page
- missing files page
- albums page
- artists page
- favorites page
- search page
- metadta editing window

- Expand automated test coverage
- Continue expanding metadata and codec compatibility
- Implement smart playlists
- Genre page
- Normalize tags
- Custom playlist images
- Custom artist images
- Ability to organize file/folder structure based on tags
- Pin album, songs & playlists

Audexis currently recognizes MP3, MP2, MP1, FLAC, M4A, M4B, MP4, Ogg, Opus, OGA,
SPX, OGV, MOV, M4V, and QuickTime files. Support can vary depending on the codec
and metadata contained in a file.

## Project status

Audexis is pre-release software. Core library browsing, metadata-editing
and playback are in place, but testing, error handling, and documentation
still being improved before a stable release.

## Development

### Prerequisites

- [Rust & cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

- [Just](https://github.com/casey/just)

For website

- [Node.js](https://nodejs.org/) 22.18 or newer
- [pnpm](https://pnpm.io/) 9

### Run locally

From the repository root:

```sh
just dev
```

On the first run, choose the folders that contain your music. Audexis will scan
them and build a local library database.

### Build the desktop application

```sh
just build
```

## Project structure

```text
audexis/
├── apps/
│   ├── dekstop           Desktop App
│   └── www               Website
└── crates/
    ├── audexis_audio     Crate that handles audio play back
    └── audexis_core      Crate has core modules (user config, db, filewatcher)
```

## Tech Stack

- [Freya](https://freyaui.dev)
- [Rust](https://www.rust-lang.org/)
- [SQLite](https://www.sqlite.org/) through
  [SQLx](https://github.com/transact-rs/sqlx)
- [Symphonia](https://github.com/pdeljanov/Symphonia) and
  [CPAL](https://github.com/RustAudio/cpal) for audio

## License

Audexis is available under the [MIT License](./LICENSE).
