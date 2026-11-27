---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Development"
description: "Check the crate and generate its Rust API docs."
---

# Development

Run these commands from the repository root:

```sh
cargo check -p audexis_protocol
cargo test -p audexis_protocol
cargo doc -p audexis_protocol --no-deps --open
```
