# Contributing to Audexis

Contributions of code, tests, documentation, bug reports, and design feedback
are welcome.

Keep contributions focused. Discuss substantial features, dependency additions,
and architectural changes in an issue before investing in a large implementation.
Follow the [code of conduct](CODE_OF_CONDUCT.md) in project spaces.

## Crates/Apps

| Area                                | Starting point                                        |
| ----------------------------------- | ----------------------------------------------------- |
| Desktop app                         | [apps/desktop](apps/desktop/README.md)                |
| Website and documentation           | [apps/www](apps/www/README.md)                        |
| iOS client                          | [apps/ios](apps/ios/README.md)                        |
| Android client                      | [apps/android](apps/android/README.md)                |
| Library, tags, database, and sync   | [audexis_core](crates/audexis_core/README.md)         |
| Audio decoding and playback         | [audexis_audio](crates/audexis_audio/README.md)       |
| Cloud data types                    | [audexis_protocol](crates/audexis_protocol/README.md) |
| Shared mobile state and C interface | [audexis_mobile](crates/audexis_mobile/README.md)     |
| Cloud API and media storage         | [audexis_server](crates/audexis_server/README.md)     |

## Development setup

Fork and clone the repository, then create a branch for your change.

For Rust work, install a toolchain supporting edition 2024 with Rustfmt and
Clippy. From the repository root:

```sh
cargo run -p audexis
```

Just is optional; the root `justfile` provides shortcuts for development and
validation. Desktop packaging also requires Cargo Packager.

For website work, use Node.js 22.12 or newer and pnpm, then run from `apps/www`:

```sh
pnpm install
pnpm dev
```

Account and billing routes require environment configuration and PostgreSQL.

The desktop stores its database and user configuration in the platform's app
data directory. Use disposable copies of media files when testing metadata
writing, file watching, downloads, or library migrations.

## Making changes

### Rust

- Follow the surrounding code's formatting and naming conventions.
- Return useful errors for failures.
- Avoid `unwrap()` and `expect()` where malformed media or external state can reach them.
- Keep reusable behavior separate from UI components and platform glue.
- Use parameter binding for SQL values.
- Keep blocking work away from UI.

### Interfaces and accessibility

- Keep components focused and reuse existing styles and controls.
- Preserve keyboard navigation, readable contrast, labels, and focus behavior.
- Use semantic HTML and appropriate ARIA attributes on the website.
- Check loading, empty, and error states when changing a screen.

### Database changes

Do not edit or delete existing migration files. Add a new migration for schema
changes, and test both a fresh database and an upgrade from the previous schema.
Desktop SQLite migrations are under `crates/audexis_core/src/database/migrations`;
server PostgreSQL migrations are under `crates/audexis_server/src/migrations`.
The mobile schema is in `crates/audexis_mobile/src/mobile_schema.sql`.

### Audio and metadata

Treat media files as untrusted input. Readers and writers should handle
truncated, malformed, and unusual files without crashing or damaging the source.
For fixes in these areas, include a focused regression test or explain the
manual reproduction and verification. Share only fixtures you have permission
to distribute; a minimal synthetic example is preferable to a full music file.

## Validation

Run Rust checks from the repository root:

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`just validate` runs the same checks. During development, use `-p <package>` to
check or test the component you are changing, then run the workspace checks
before submitting Rust changes.

For website changes, run `pnpm build` from `apps/www` and inspect the affected
pages. For iOS and Android changes, build and exercise the affected flow in the
native project; record the simulator or device used.

## Use of AI tools

AI-assisted contributions are allowed, but contributors remain responsible for
everything they submit. Review, understand, and test generated work yourself.

AI tools can help explore interface designs, refine copy, draft documentation,
and assist with routine UI implementation.

AI tools are **not recommended for backend changes**. Audexis handles audio
decoding, metadata writes, filesystem events, and database state; mistakes can
corrupt files, lose library data, or cause subtle playback bugs. Backend changes
should be written and reviewed by someone who understands the affected Rust
code and can explain it.

## Bugs and feature requests

Search existing issues before opening a new one. Include the version or commit,
platform, affected component, reproduction steps, expected result, actual
result, and relevant redacted logs or screenshots. For media issues, include
the container and codec when known.

For a feature request, describe the problem, an example workflow, and any
alternatives you have tried. See [Support](SUPPORT.md) for more reporting help.
Report vulnerabilities through [Security](SECURITY.md), rather than a public issue.

## Pull requests

Before submitting:

1. Bring your branch up to date with the target branch.
2. Remove unrelated changes and temporary debugging output.
3. Add relevant tests and update documentation when behavior changes.
4. Run the applicable checks and review the final diff.

Explain the problem, what changes for the user or caller, and how you verified
it. Link related issues and include screenshots for visible UI changes. Call
out compatibility changes, migrations, and known limitations.

## License

By contributing, you agree that your contribution will be licensed under the
[MIT License](LICENSE).
