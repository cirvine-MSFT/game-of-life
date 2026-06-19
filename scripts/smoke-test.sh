#!/usr/bin/env bash
# Smoke test for the game-of-life binary.
#
# Verifies the binary launches, runs a basic simulation, saves a run
# record to disk, and replays it cleanly. Used by CI (.github/workflows/ci.yml)
# and intended to also be run locally by developers when they want to
# sanity-check the binary outside of the Rust test runner.
#
# This is intentionally a thin "does the real binary do the obvious thing"
# check. Behavioral coverage of save / load / replay / extract / continue
# lives in tests/persistence/cli_tests.rs and the per-module unit tests, all
# driven through `cargo test`.
#
# Usage:
#   ./scripts/smoke-test.sh               # uses ./target/release/game-of-life
#   PROFILE=debug ./scripts/smoke-test.sh # uses ./target/debug/game-of-life
#
# Requires bash, grep, mktemp. On Windows, use Git Bash or WSL.

set -euo pipefail

PROFILE="${PROFILE:-release}"

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) BIN_SUFFIX=".exe" ;;
    *)                    BIN_SUFFIX=""    ;;
esac

BIN="./target/${PROFILE}/game-of-life${BIN_SUFFIX}"

if [ ! -x "$BIN" ]; then
    echo "Binary not found at $BIN; building with: cargo build${PROFILE:+ --$PROFILE}" >&2
    if [ "$PROFILE" = "release" ]; then
        cargo build --release
    else
        cargo build
    fi
fi

# Use a tempdir so the smoke test never touches ./runs/.
RUNS_DIR="$(mktemp -d)"
trap 'rm -rf "$RUNS_DIR"' EXIT

echo "--> [smoke] running basic simulation; record dir: $RUNS_DIR"
"$BIN" --board-size 5x5 --max-iterations 5 --runs-dir "$RUNS_DIR" \
    > "$RUNS_DIR/console.txt"

test -s "$RUNS_DIR/console.txt"
grep -q "Generation 0:"          "$RUNS_DIR/console.txt"
grep -q "Saved run record:"      "$RUNS_DIR/console.txt"
grep -q "^[.#]*$"                "$RUNS_DIR/console.txt"

# Replay round-trip catches regressions in both write and read paths.
RECORD="$(ls "$RUNS_DIR"/*.gol | head -n 1)"
echo "--> [smoke] replaying $RECORD"
"$BIN" --replay "$RECORD" > "$RUNS_DIR/replay.txt"
grep -q "Replay matched" "$RUNS_DIR/replay.txt"

echo "[smoke] PASSED"
