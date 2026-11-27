---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Development"
description: "Find the server code and run its checks."
---

# Development

The crate lives in `crates/audexis_server`. Start with `src/router.rs` for HTTP
routes, `src/config.rs` for configuration, or `src/users.rs` for account
commands. Upload processing is in `src/ingest.rs`, storage backends are in
`src/storage/`, and AAC generation is in `src/variants.rs`.

```sh
cargo check -p audexis_server
cargo test -p audexis_server
```

The crate's README also includes Docker deployment and backup commands.
