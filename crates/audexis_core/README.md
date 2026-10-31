# audexis_core

Shared library and metadata functionality for Audexis.

## What it does

- Store and query the local music library in SQLite.
- Watch music folders and track library file changes.
- Read and edit metadata across supported file formats.
- Manage artwork, lyrics, and user configuration.
- Synchronize tracks and playlists with an Audexis server.

## Development

Run from the repository root:

```sh
cargo check -p audexis_core
cargo test -p audexis_core
```

## Code map

| Path                              | Purpose                                      |
| --------------------------------- | -------------------------------------------- |
| `src/lib.rs`                      | Public modules                               |
| `src/database/`                   | SQLite queries, data types, and migrations   |
| `src/file_watcher/`               | File Watcher for music                       |
| `src/tag_manager/`                | Format-specific metadata readers and writers |
| `src/metadata_editor.rs`          | Metadata editing operations                  |
| `src/cloud_sync/`                 | Cloud sync engine and DOwnload management    |
| `src/config/`                     | User configuration                           |
| `src/artwork.rs`, `src/lyrics.rs` | Artwork and lyrics helpers                   |
