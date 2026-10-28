# audexis_audio

The desktop audio engine.

## What it does

- Decode audio with Symphonia and send output through CPAL.
- Manage playback position, seeking, shuffle, repeat, and queue changes.
- Publish playback and queue state for the UI.
- Integrate with system media controls through Souvlaki.
- equalizer, play counts, and listening time tracking.

## Development

Run from the repository root:

```sh
cargo check -p audexis_audio
cargo test -p audexis_audio
```

Audio output depends on the host platform's audio backend. Run the
[desktop app](../../apps/desktop/README.md) to test playback with a UI
(mobile coming soon).

## Code map

| Path               | Purpose                                       |
| ------------------ | --------------------------------------------- |
| `src/lib.rs`       | Public exports                                |
| `src/playback.rs`  | `AudioPlayer` and `PlayerHandle`              |
| `src/worker/`      | Decode loop and playback command handling     |
| `src/queue/`       | Queue navigation, shuffle, and reconciliation |
| `src/events.rs`    | Playback state, queue information, and events |
| `src/streaming.rs` | Streaming audio sources                       |
| `src/equalizer.rs` | Equalizer processing                          |
| `src/media.rs`     | System media controls                         |
