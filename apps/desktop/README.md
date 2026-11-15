# Audexis Desktop app

Audexis is a Rust and Freya music player for local libraries, with optional
cloud sync. Browse music, edit metadata, manage playlists, and play local or
cloud songs from the same library views.

## Run the app

Install a Rust toolchain with support for edition 2024, then run from the
repository root:

```sh
cargo run -p audexis
```

Choose your music folders on first launch. Use copies of your music when
developing features that change tags or files.

## Build

Build the release executable from the repository root:

```sh
cargo build -p audexis --release
```

To create installers, install Cargo Packager and run:

```sh
cargo packager --release --config apps/desktop/Packager.json
```

Packaging and signing depend on the host platform and release configuration.

## Cloud sync

Open **Settings → Cloud** to sign in to audexis Cloud or connect a custom server
with its URL and API key. Choose **Sync now** to upload eligible library music.
**Automatically sync library** is off by default; enabling it uploads new music
automatically. Cloud-library updates and edits can still sync while it is off.

Use **Cloud Sync** in the sidebar to inspect queued operations, check storage,
and resolve conflicts. Right-click a cloud song and choose **Download for
Offline** before listening without a connection.

See the [cloud sync guide](../www/src/pages/docs/apps/desktop/cloud-sync.md)
for sign-in, downloads, retries, and storage details.

## Documentation and checks

- [Getting started](../www/src/pages/docs/apps/desktop/index.md).
- [Development guide](../www/src/pages/docs/apps/desktop/development.md).
- [Troubleshooting](../www/src/pages/docs/apps/desktop/troubleshooting.md).

Run `cargo check -p audexis` for a focused compilation check. The development
guide lists workspace formatting, lint, and test commands.
