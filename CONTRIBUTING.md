# Contributing to Audexis

Thanks for helping improve Audexis. The project is under active development,
and contributions of code, tests, documentation, bug reports, and design
feedback are welcome.

## Ways to contribute

- Report a reproducible bug
- Improve documentation
- Fix an existing issue
- Improve accessibility or usability
- Propose a feature or architectural change

Pls keep each contribution focused. Small pull requests are easier to
review, test, and merge than changes that combine unrelated work.

## Development setup

### Prerequisites

- [Node.js](https://nodejs.org/) 24 or newer
- [pnpm](https://pnpm.io/) 11
- A stable [Rust toolchain](https://www.rust-lang.org/tools/install)
- The [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
  for your operating system

### Install and run

Fork and clone the repository, then run the following commands from its root:

```sh
pnpm install
pnpm --filter desktop tauri dev
```

Audexis stores its library database in the operating system's application data
directory. When testing library changes, use a folder containing disposable
copies of media files. Do not use the only copy of a music library when testing
metadata writing or file-watcher behavior.

## Repository layout

```text
audexis/
├── apps/desktop/
│   ├── src/                 React and TypeScript interface
│   └── src-tauri/           Tauri Backend
│       ├── migrations/      SQLite migrations
│       └── src/             Rust backend
├── packages/shared/         Shared styles
└── .github/workflows/       GitHub Actions workflows
```

Important Rust modules include:

- `audio_player`: playback, queue, decoding, and listening state
- `commands`: the interface between the frontend and Rust backend
- `database`: SQLite setup and database types
- `file_watcher`: library discovery, indexing, and filesystem events
- `tag_manager`: metadata parsing and writing

## Making changes

### TypeScript and React

- Keep Tauri calls in focused hooks or utilities instead of scattering them
  through presentation components.
- Follow the formatting and naming style of the surrounding files.
- Pls use [ARIA](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference)
- Keep wording clear

Run the frontend build before submitting a change:

```sh
pnpm --filter desktop build
```

### Rust

- Return useful errors instead of panicking on recoverable input or I/O errors.
- Avoid adding `unwrap()` unless checked for error or is none previously
- Avoid using `expect()` where a malformed media file, poisoned lock, missing
  path, or database failure could reach the code.
- Keep Tauri commands thin. Put reusable behavior in ordinary Rust functions so
  it can be tested without launching the application.
- Use parameter binding for values included in SQL queries.

Format and check Rust code with:

```sh
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets
```

### Database migrations

- Do not change, edit or delate any file in migrations folder.
- Add a new migration for schema changes.
- Make migrations safe for existing libraries whenever possible.
- Test both a fresh database and an upgrade from the previous schema.

### Audio and metadata changes

Media files are untrusted input. Code that parses or writes metadata should
handle truncated, malformed, and unusual files without crashing.

## Use of AI tools

AI-assisted contributions are allowed, but contributors remain responsible for
everything they submit.

Using AI tools is acceptable for frontend work such as exploring designs,
refining interface copy, drafting documentation, or assisting with routine UI
implementation. All generated work must still be reviewed, understood, and
tested by the contributor.

AI tools are **not recommended for backend changes**. Audexis handles audio
decoding, metadata writes, filesystem events, and database state;
plausible-looking mistakes in these areas can corrupt files, lose library data,
or introduce difficult-to-reproduce playback bugs. Backend contributions should
be written and reviewed by someone who understands the affected Rust code and
can explain it.

## Reporting bugs

A useful bug report includes:

- Audexis version or commit
- Operating system and version
- Steps to reproduce the problem
- Expected and actual behavior
- Relevant logs or screenshots
- The affected file format or codec, when applicable

## Pull requests

Before opening a pull request:

1. Rebase or merge the latest target branch as appropriate.
2. Remove debugging output and unrelated changes.
3. Run the frontend build and relevant Rust checks.
4. Add or update tests and documentation.
5. Review the final diff yourself.

The pull request description should explain:

- What changed and why
- How the change was tested
- Any known limitations or follow-up work

A pull request does not need to solve every related problem, but it should leave the
project in a buildable and understandable state.

## License

By contributing to Audexis, you agree that your contribution will be licensed
under the project's [MIT License](LICENSE).
