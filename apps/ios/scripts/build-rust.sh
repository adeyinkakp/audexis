#!/bin/sh
set -eu

export PATH="$HOME/.cargo/bin:$PATH"
ROOT=$(CDPATH= cd -- "$SRCROOT/../../.." && pwd)

case "$PLATFORM_NAME" in
    iphoneos) TARGET=aarch64-apple-ios ;;
    iphonesimulator)
        case "$CURRENT_ARCH" in
            x86_64) TARGET=x86_64-apple-ios ;;
            *) TARGET=aarch64-apple-ios-sim ;;
        esac
        ;;
    *) echo "Unsupported Rust platform: $PLATFORM_NAME" >&2; exit 1 ;;
esac

PROFILE=debug
RELEASE_FLAG=
if [ "$CONFIGURATION" = Release ]; then
    PROFILE=release
    RELEASE_FLAG=--release
fi

export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}"
export CARGO_TARGET_DIR="$ROOT/target/mobile"
cd "$ROOT"
cargo build --locked --package audexis_mobile --target "$TARGET" $RELEASE_FLAG
mkdir -p "$BUILT_PRODUCTS_DIR"
cp "$CARGO_TARGET_DIR/$TARGET/$PROFILE/libaudexis_mobile.a" "$BUILT_PRODUCTS_DIR/libaudexis_mobile.a"
