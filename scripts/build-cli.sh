#!/usr/bin/env bash
# Build the CLI binary.
#
# Wraps `cargo build` against the workspace's root crate (the
# `game-of-life` console binary). Honours the standard PROFILE env
# var convention used by other scripts in this folder.
#
# Usage:
#   ./scripts/build-cli.sh                      # release build (default)
#   PROFILE=debug ./scripts/build-cli.sh        # debug build
#   ./scripts/build-cli.sh -- --features foo    # forward extra args to cargo

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

PROFILE="${PROFILE:-release}"

case "$PROFILE" in
    release)
        echo "--> [cli] cargo build --release"
        cargo build --release "$@"
        BIN_DIR="target/release"
        ;;
    debug)
        echo "--> [cli] cargo build"
        cargo build "$@"
        BIN_DIR="target/debug"
        ;;
    *)
        echo "Unknown PROFILE='$PROFILE' (expected 'release' or 'debug')" >&2
        exit 2
        ;;
esac

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) EXE=".exe" ;;
    *)                     EXE="" ;;
esac

echo "--> [cli] binary: $BIN_DIR/game-of-life$EXE"
