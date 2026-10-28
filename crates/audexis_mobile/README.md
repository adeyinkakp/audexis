# audexis_mobile

The Rust library for native mobile clients. Meant to handle for both ios
and android but currently only supports iOS as I have only tested it on
iOS and I don't understand Android development that well.

## What it does

- Open a local library and expose songs and snapshots.
- Apply cloud change pages and persist the sync cursor.
- Manage queue selection and navigation.

## Todo

- Use symphonia with cpal for audio decoding and playback

## Development

Run from the repository root:

```sh
cargo build -p audexis_mobile
```

## Code map

| Path                                   | Purpose                                     |
| -------------------------------------- | ------------------------------------------- |
| `src/lib.rs`                           | Command dispatch and mobile state           |
| `src/ffi.rs`                           | C-callable interface and response envelopes |
| `src/library.rs`                       | Local library integration                   |
| `src/persistence.rs`, `src/storage.rs` | Persistence helpers                         |
| `src/mobile_schema.sql`                | Mobile database schema                      |
| `include/`                             | C header declarations                       |
