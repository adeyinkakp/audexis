---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Users and API keys"
description: "Create accounts, replace keys, and set storage limits."
---

# Users and API keys

Self-hosted accounts are managed with the server's `users` command. Leave
`AUDEXIS_AUTH_ORIGIN` empty when using only these accounts.

The examples below run from `deploy` with Compose. For a source checkout,
replace `docker compose exec api audexis_server` with
`cargo run -p audexis_server --` and run from the repository root.

## Create an account

```sh
docker compose exec api audexis_server users create kp --quota 100GB
```

Names can contain letters, numbers, dots, hyphens, and underscores, with a
maximum of 64 characters. The command prints the API key once. Save it before
closing the terminal; the server stores its hash and cannot show it again.

Enter the server URL and key in the client's custom-server settings.

## Replace a lost key

```sh
docker compose exec api audexis_server users reset-key kp
```

This replaces the old key. Update every client using that account with the new
one. Replacing a key does not create a new library.

## Change the quota

```sh
docker compose exec api audexis_server users quota kp 200GB
```

Sizes accept values such as `100GB`, `1TB`, or a number of bytes. New accounts
use `AUDEXIS_QUOTA_BYTES` unless you pass `--quota` when creating them. Change
an existing account's quota with the command above.

## Disable or enable access

```sh
docker compose exec api audexis_server users disable kp
docker compose exec api audexis_server users enable kp
```

Disabling an account blocks authentication; it does not delete its library.
Account changes do not require a server restart.

For the available commands:

```sh
docker compose exec api audexis_server users --help
```
