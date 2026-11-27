---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Playback"
description: "Create a player and keep the UI in step with playback."
---

# Playback

## Where to start

The crate lives in `crates/audexis_audio`. `AudioPlayer` owns the player, and
`PlayerHandle` provides access to playback commands. `PlayerCmd` describes the
commands the player accepts.

Creating an `AudioPlayer` requires a SQLite connection pool, a Tokio runtime
handle, equalizer settings, and optional system media controls. It opens the
default output device, so creation can fail if no usable device is available.
Handle that error in the app rather than assuming playback always starts.

| File or directory | What it does |
| --- | --- |
| `src/playback.rs` | Creates the player and exposes its handle. |
| `src/worker/` | Decodes audio and processes playback commands. |
| `src/queue/` | Tracks queue order, shuffle, and the current song. |
| `src/events.rs` | Defines playback state, queue updates, and events. |
| `src/streaming.rs` | Provides streaming audio sources. |
| `src/equalizer.rs` | Applies equalizer settings to playback. |
| `src/media.rs` | Connects playback to system media controls. |

## Playback updates

The engine publishes `PlaybackState`, `QueueInfo`, and `AudioEvent` updates.
Use these to keep the UI in step with the player, including changes made through
system media controls.

The engine also tracks play counts and listening time. Library data comes from
[Core](/docs/crates/core), while shared equalizer settings are defined in
[Protocol](/docs/crates/protocol).
