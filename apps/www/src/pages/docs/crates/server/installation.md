---
layout: ../../../../layouts/MarkdownLayout.astro
title: "Audexis — Run locally"
description: "Start the server with Rust and PostgreSQL."
---

# Run locally

Run the following commands from the repository root. You need Rust, Cargo,
and an existing PostgreSQL database.

```sh
cp crates/audexis_server/.env.example crates/audexis_server/.env
```

Edit the copied file:

- Set `DATABASE_URL` to your PostgreSQL connection URL.
- Set `AUDEXIS_PUBLIC_ORIGIN=http://localhost:3000`.
- Set `AUDEXIS_STORAGE_DIR=./data/media` to store audio locally.
- Leave `AUDEXIS_AUTH_ORIGIN` empty to use self-hosted accounts.

Create an account, then start the server:

```sh
cargo run -p audexis_server -- users create developer --quota 100GB
cargo run -p audexis_server
```

Save the API key printed when the account is created. You will need it to
connect a client. Run `cargo run -p audexis_server -- users --help` for the
other account commands.

Database migrations run when the server connects to PostgreSQL. The default
listener is `127.0.0.1:3000`.

```sh
curl --fail http://localhost:3000/health
```

The health endpoint returns HTTP 200. It checks that the HTTP server responds;
it does not check storage connectivity.
