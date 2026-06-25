#!/usr/bin/env bash
# Produce a release build of the desktop visualizer.
#
# Mirrors the README workflow: runs `cargo tauri build` from
# desktop\, which builds the React frontend (vite build) and then
# compiles + bundles the Rust shell. Output bundles land under
# desktop/target/release/bundle/.
#
# Usage:
#   ./scripts/build-desktop.sh                # release bundle (default)
#   ./scripts/build-desktop.sh -- --debug     # forward args to `cargo tauri build`
#
# Re-runs `npm install` only when node_modules is missing. Pass
# FORCE_INSTALL=1 to reinstall every time. Pass SKIP_TAURI_INSTALL=1
# to skip the `cargo install tauri-cli` probe.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DESKTOP_DIR="$REPO_ROOT/desktop"
UI_DIR="$DESKTOP_DIR/ui"

if [ ! -d "$DESKTOP_DIR" ] || [ ! -d "$UI_DIR" ]; then
    echo "Expected desktop/ and desktop/ui/ under $REPO_ROOT" >&2
    exit 1
fi

if [ "${SKIP_TAURI_INSTALL:-0}" != "1" ] && ! cargo tauri --version >/dev/null 2>&1; then
    echo "--> [desktop] installing tauri-cli (one-time, ~couple of minutes)"
    cargo install tauri-cli --version "^2.0" --locked
fi

if [ "${FORCE_INSTALL:-0}" = "1" ] || [ ! -d "$UI_DIR/node_modules" ]; then
    echo "--> [desktop] installing UI dependencies"
    (cd "$UI_DIR" && npm install)
fi

cd "$DESKTOP_DIR"
echo "--> [desktop] running cargo tauri build"
cargo tauri build "$@"
echo "--> [desktop] bundles written to desktop/target/release/bundle/"
