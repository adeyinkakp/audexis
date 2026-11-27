---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Development"
description: "Build the mobile library and check native integration."
---

# Development

Run these commands from the repository root:

```sh
cargo build -p audexis_mobile
cargo check -p audexis_mobile
cargo doc -p audexis_mobile --no-deps --open
```

A host build checks the Rust library. Changes to iOS media controls or native
integration also need to be checked in the iOS app.
