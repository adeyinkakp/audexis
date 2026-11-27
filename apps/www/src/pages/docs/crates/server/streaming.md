---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Streaming and AAC"
description: "Serve original audio or generate smaller AAC copies."
---

# Streaming and AAC

The server uses original audio by default. Clients can also request AAC at
128 or 256 kbps when FFmpeg is configured.

## Enable AAC generation

For a source installation, install FFmpeg and set its executable path:

```dotenv
AUDEXIS_FFMPEG_PATH=/usr/bin/ffmpeg
```

The supplied Docker image includes FFmpeg. Check the container configuration
if AAC requests keep returning the original file.

## Requesting audio

The media endpoint accepts these quality values:

- `original`
- `aac_128`
- `aac_256`

For example, a client can request
`/api/tracks/{id}/audio?quality=aac_128`. The response contains a URL, the file
size and hash, the actual `quality`, and a `preparing` flag.

If the AAC copy is still queued or processing, the response can contain the
original audio URL with `preparing: true`. Clients should use the returned
quality and hash rather than assuming the requested copy is ready.

## Why the first request takes longer

AAC generation has to read the original file and encode another copy. A track
that has not been converted yet can take longer than one with a ready variant.
Original playback avoids waiting for this conversion.

The server keeps input files in `AUDEXIS_VARIANT_INPUT_DIR` to reuse during
conversion. `AUDEXIS_VARIANT_INPUT_CACHE_BYTES` limits that cache. This working
space is separate from account storage quotas.

## Media URLs

B2 downloads use signed storage URLs. Local downloads use the server's
`/transfer/{token}` route. Request a fresh media URL if an old one stops
working; do not treat it as a permanent track address.
