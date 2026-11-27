---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Public access"
description: "Make a self-hosted server reachable by your clients."
---

# Public access

Clients need a URL they can reach for both API calls and media transfers.
`localhost` only points to the device making the request, so it will not work
for a phone connecting to a server on another machine.

## Set the public URL

Use an HTTPS address for a server exposed to the internet:

```dotenv
AUDEXIS_PUBLIC_ORIGIN=https://music.example.com
```

The server uses this address when generating local upload and download links.
A working API URL with the wrong public origin can let a client connect while
its uploads or playback still fail.

## Put a proxy in front

The supplied Compose file exposes the API on the host at `127.0.0.1:3000`.
Configure your reverse proxy to forward requests from your HTTPS hostname to
that address. Keep PostgreSQL private.

The proxy needs to allow uploads large enough for your music files and enough
time for the transfer to complete. Check its body-size and timeout settings
if small uploads work but large ones fail.

## Trusted proxies

`AUDEXIS_TRUSTED_PROXIES` accepts a comma-separated list of proxy IP addresses.
Use the actual source addresses the API sees. This lets the server use client
addresses forwarded by those proxies for rate limiting.

Leave it empty when connecting directly

## Check the connection

Open `$SERVER_URL/health` from another device. A successful
response is HTTP 200. Then connect a client with the same base URL and an
[API key](/docs/crates/server/users).
