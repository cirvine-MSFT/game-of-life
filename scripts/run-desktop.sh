#!/usr/bin/env bash
# Build and launch the desktop visualizer in development mode.
#
# Wraps the documented workflow from README.md: runs `cargo tauri dev
# --no-watch` from desktop/, after making sure tauri-cli is installed
# and frontend dependencies are present in desktop/ui/. `--no-watch`
# is deliberate — Tauri's Rust-source watcher currently picks up
# changes inside desktop/ui/node_modules/ and triggers spurious
# rebuilds; the Vite dev server still hot-reloads the frontend.
#
# Usage:
#   ./scripts/run-desktop.sh                # dev mode (default)
#   ./scripts/run-desktop.sh -- --release   # forward args to `cargo tauri dev`
#
# Re-runs `npm install` only when node_modules is missing. Pass
# FORCE_INSTALL=1 to reinstall every time. Pass SKIP_TAURI_INSTALL=1
# to skip the `cargo install tauri-cli` probe (useful in CI).

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
echo "--> [desktop] launching cargo tauri dev --no-watch"
exec cargo tauri dev --no-watch "$@"
