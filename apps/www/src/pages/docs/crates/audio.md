---
layout: ../../../layouts/MarkdownLayout.astro
title: "Audexis — Audio"
description: "The playback engine used by the desktop app and mobile library."
---

# Audio

`audexis_audio` plays music. It decodes audio with Symphonia and sends the
samples to the output device through CPAL. The desktop app and mobile library
use it for playback.

It also handles the queue, seeking, shuffle, repeat, equalizer processing, and
playback updates for the UI.

## In this guide

- [Playback](/docs/crates/audio/playback) — Create a player and keep the UI in step with playback.
- [Development](/docs/crates/audio/development) — Build the audio engine and check playback changes.
