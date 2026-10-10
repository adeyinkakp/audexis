---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Playback and queue"
description:
  "Use playback controls, the queue, shuffle, repeat, and system media controls."
---

# Playback and queue

## Main playback controls

The bottom playback bar shows the current track and provides play/pause,
previous, next, and a seek bar. Drag or click the seek bar to choose a position
within the current song.

Some controls are unavailable when there is no active track. If the bar says
nothing is playing, start a song from the library before seeking or changing
track-specific options.

The **Playback** menu also provides Play, Pause, Previous Track, Next Track,
shuffle, repeat, the equalizer, and the two player views.

## Build the queue

The playback queue is the sequence of songs selected for the current listening
session. It is distinct from a saved playlist.

Right-click a song and choose:

- **Play Next** to place it after the current track.
- **Play Last** to append it to the end of the queue.

Open **Queue** from the playback bar or **View → Show Queue** to inspect the
queue. Choose a queued item to jump to it. Starting playback from a different
collection can replace the current queue.

Adding songs to the queue does not move audio files or create a saved playlist.
Use [Playlists](/docs/playlists) when you want a collection you can return to
later.

## Shuffle

Shuffle changes the playback order. Toggle it from the playback bar or Playback
menu. Turning it off returns to the queue’s underlying order rather than
permanently rewriting a playlist’s saved order.

If the same file appears more than once in a queue, those are separate queue
entries; shuffle does not turn them into a single entry.

## Repeat

- **Off:** playback stops when the queue reaches its end.
- **All:** the queue wraps to its beginning.
- **One:** the current track repeats when it finishes.

Repeat One affects what happens at the end of the track. Explicitly choosing
Next still advances to another queue entry.

## Lyrics and system controls

Open **Lyrics** from the playback bar or **View → Show Lyrics**. Available
embedded lyrics are shown for the current song; synchronized lyrics can follow
the playback position.

Audexis connects playback to native system media controls. Their appearance and
available controls depend on the operating system. These controls operate the
same playback session as the app.

## Other player views

The [fullscreen player and mini player](/docs/players) share the same active
track and queue. Opening another view does not start an independent audio
session.

[Equalizer](/docs/equalizer) · [Lyrics](/docs/lyrics) ·
[Troubleshooting](/docs/troubleshooting)
