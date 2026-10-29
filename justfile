# will update list eventually
default:
    @just --list

# Run the desktop app.
dev:
    cargo run -p audexis
hotreload:
    dx serve --hot-patch --features hotreload

# run frontend dev server
frontend:
    cd apps/www && pnpm dev

# Build the release executable.
build:
    cargo packager --release --config apps/desktop/Packager.toml
# Check every Rust crate.
check:
    cargo check --workspace

# Run tests.
test:
    cargo test --workspace

# format rust code
fmt:
    cargo fmt --all ; swift-format format --in-place --configuration swift-format.json --recursive apps/ios
# lint checks
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# run all checks
validate:
    cargo fmt --all -- --check
    cargo check --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
