# will update list eventually
#
# auth_origin should be http://localhost:4321 on ur sys
auth_origin := "https://my-macbook-air.tail5c4e82.ts.net"
# auth_origin should be http://localhost:3000 on ur sys
cloud_api_url := "https://my-macbook-air.tail5c4e82.ts.net:8443"
default:
    @just --list

# Run the desktop app.
dev:
    AUDEXIS_AUTH_ORIGIN={{ auth_origin }} AUDEXIS_CLOUD_API_URL={{ cloud_api_url }} cargo run -p audexis
# run the server
server:
    cargo run -p audexis_server

# run frontend dev server
frontend:
    cd apps/www && pnpm dev

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
