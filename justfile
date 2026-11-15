# will update list eventually
#
# auth_origin should be http://localhost:4321 on ur sys
auth_origin := "http://localhost:4321"

# auth_origin should be http://localhost:3000 on ur sys
cloud_api_url := "https://my-macbook-air.tail5c4e82.ts.net:8443"
default:
    @just --list

# Run the desktop app.
dev:
    AUDEXIS_AUTH_ORIGIN={{ auth_origin }} AUDEXIS_CLOUD_API_URL={{ cloud_api_url }} cargo run -p audexis
# run the server
server:
    AUDEXIS_BIND=127.0.0.1:3000 AUDEXIS_AUTH_ORIGIN="{{ auth_origin }}" AUDEXIS_PUBLIC_ORIGIN="{{ cloud_api_url }}" cargo run -p audexis_server

# run frontend dev server
frontend:
    cd apps/www && AUDEXIS_DEV_AUTH_ORIGINS="{{ auth_origin }},${AUDEXIS_DEV_AUTH_ORIGINS:-}" pnpm dev

# Build the release executable.
build:
    cargo packager --release --config apps/desktop/Packager.json
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
