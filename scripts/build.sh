#!/usr/bin/env bash
# Build everything: CLI binary first, then the desktop bundle.
#
# Delegates to the per-component scripts so this stays a thin
# orchestrator. The CLI builds first because:
#   1. It's the fastest path to failure if the workspace is broken.
#   2. `cargo tauri build` shares the same Cargo target dir, so any
#      shared dependencies are already compiled by the time the
#      desktop step starts.
#
# Usage:
#   ./scripts/build.sh                 # release everything (default)
#   PROFILE=debug ./scripts/build.sh   # debug CLI; desktop is always release
#
# Environment variables forwarded to child scripts:
#   PROFILE              -> build-cli.sh (release|debug)
#   FORCE_INSTALL        -> build-desktop.sh (re-run npm install)
#   SKIP_TAURI_INSTALL   -> build-desktop.sh (skip cargo install tauri-cli probe)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== [build] step 1/2: CLI ==="
"$SCRIPT_DIR/build-cli.sh"

echo
echo "=== [build] step 2/2: desktop ==="
"$SCRIPT_DIR/build-desktop.sh"

echo
echo "=== [build] done ==="
