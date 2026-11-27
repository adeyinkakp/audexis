---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Development"
description: "Build the audio engine and check playback changes."
---

# Development

## Checking playback

Run these commands from the repository root:

```sh
cargo check -p audexis_audio
cargo test -p audexis_audio
cargo doc -p audexis_audio --no-deps --open
```

To try a change with the desktop UI:

```sh
cargo run -p audexis
```

Check local playback and streaming when changing the decode path. For queue
changes, try seeking, skipping tracks, shuffle, and repeat as well.
