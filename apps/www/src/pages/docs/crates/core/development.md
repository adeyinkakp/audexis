---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Development"
description: "Build the crate and check changes in the desktop app."
---

# Development

Run these commands from the repository root:

```sh
cargo check -p audexis_core
cargo test -p audexis_core
cargo doc -p audexis_core --no-deps --open
```

For a change that affects the UI, also try it in the desktop app. Use copies of
music files when checking metadata writes.
