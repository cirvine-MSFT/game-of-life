#!/usr/bin/env bash
# End-to-end smoke test for every bundled example board.
#
# Loads every file in examples/patterns/*.gol through the game-of-life binary
# with --load-board, runs a sensible number of iterations, and verifies that
# the binary exits cleanly and emits the expected lifecycle markers. Catches
# regressions where a board file drifts out of sync with the parser (bad
# header counts, bad grid characters, unexpected size, etc.).
#
# Negative-case files live in examples/negative/ and are exercised by
# tests/persistence/cli_tests.rs; this script only covers happy-path patterns.
#
# Usage:
#   ./scripts/examples-smoke-test.sh
#   PROFILE=debug ./scripts/examples-smoke-test.sh

set -euo pipefail

PROFILE="${PROFILE:-release}"

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) BIN_SUFFIX=".exe" ;;
    *)                    BIN_SUFFIX=""    ;;
esac

BIN="./target/${PROFILE}/game-of-life${BIN_SUFFIX}"

if [ ! -x "$BIN" ]; then
    echo "Binary not found at $BIN; building" >&2
    if [ "$PROFILE" = "release" ]; then
        cargo build --release
    else
        cargo build
    fi
fi

# Iteration counts per file. Defaults to 20 for any file not listed here.
# Methuselahs and the gun get more iterations so they actually do something
# interesting; the 1000x1000 random board gets only a handful so CI stays fast.
iters_for() {
    case "$1" in
        r-pentomino-methuselah.gol) echo 50 ;;
        acorn-methuselah.gol)       echo 50 ;;
        gosper-glider-gun.gol)      echo 50 ;;
        random-1000x1000.gol)       echo 5  ;;
        *)                          echo 20 ;;
    esac
}

shopt -s nullglob
BOARDS=(examples/patterns/*.gol)
if [ ${#BOARDS[@]} -eq 0 ]; then
    echo "No example boards found under examples/patterns/" >&2
    exit 1
fi

# Run each board through the binary in a private --runs-dir so the smoke
# test never pollutes the working tree. --no-save would skip save entirely,
# but we deliberately exercise the full save path here as part of "prove
# the file can drive a real run".
RUNS_DIR="$(mktemp -d)"
trap 'rm -rf "$RUNS_DIR"' EXIT

failures=0
total=0
for board in "${BOARDS[@]}"; do
    name="$(basename "$board")"
    iters="$(iters_for "$name")"
    total=$((total + 1))
    log="$RUNS_DIR/$name.log"

    echo "--> [examples] $name (iters=$iters)"
    if ! "$BIN" --load-board "$board" --max-iterations "$iters" \
            --runs-dir "$RUNS_DIR" > "$log" 2>&1; then
        echo "FAIL: $name exited non-zero" >&2
        sed 's/^/    /' "$log" >&2
        failures=$((failures + 1))
        continue
    fi

    # Required lifecycle markers. These are the same shape the existing
    # smoke-test.sh asserts, plus a load-specific Initial board line.
    if ! grep -q "^Initial board: load:" "$log" \
        || ! grep -q "^Generation 0:" "$log" \
        || ! grep -q "^Simulation complete:" "$log" \
        || ! grep -q "^Saved run record:" "$log"; then
        echo "FAIL: $name missing expected output marker" >&2
        sed 's/^/    /' "$log" >&2
        failures=$((failures + 1))
        continue
    fi
done

if [ "$failures" -ne 0 ]; then
    echo "[examples] FAILED: $failures of $total boards" >&2
    exit 1
fi

echo "[examples] PASSED: all $total boards loaded and ran"
