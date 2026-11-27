---
layout: ../../../layouts/MarkdownLayout.astro
title: "Audexis — Mobile"
description: "The Rust library behind the native mobile clients."
---

# Mobile

`audexis_mobile` gives the native mobile app access to the library, cloud data,
downloads, queue, and player. It uses [Core](/docs/crates/core) for library data
and [Audio](/docs/crates/audio) for playback.

The current mobile work is focused on iOS. Android support has not been tested
to the same extent.

## In this guide

- [Requests and library state](/docs/crates/mobile/requests) — Open the library and send requests from the mobile app.
- [Native integration](/docs/crates/mobile/native-integration) — Call the Rust library and manage handles and responses.
- [Development](/docs/crates/mobile/development) — Build the mobile library and check native integration.
