# Agent Operating Instructions

This document provides concise guidance for automated agents (Copilot, sub-agents, CI/CD) working on the Rust Game of Life prototype.

## Quick Facts

- **Repository**: cirvine-MSFT/game-of-life
- **Branch**: cirvine-msft/fictional-train
- **Language**: Rust 2021, zero dependencies
- **Project Type**: Library + console binary
- **Core Feature**: Conway's Game of Life with bounded board, transitional cell states, two-pass generation

## Test Case Checklist

Every change must satisfy:

- [ ] `cargo fmt --check` passes (no formatting issues)
- [ ] `cargo clippy --all-targets -- -D warnings` passes (no lint warnings as errors)
- [ ] `cargo test` passes (unit and integration tests)
- [ ] `cargo build --release` succeeds
- [ ] Console binary runs without crash: `./target/release/game-of-life`

## What Can Be Changed

✅ **Safe to modify:**
- Core algorithm in `src/board.rs` (Board, CellState, generation logic)
- Configuration model and parsing in `src/config.rs`
- Console app pattern, output format, or iterations in `src/main.rs`
- Integration tests and helpers in `tests/`
- Documentation in `docs/` and README
- CI workflow in `.github/workflows/ci.yml`

❌ **Do NOT change without explicit direction:**
- Repository hygiene checks in `.github/workflows/ci.yml`
- Repository startup files (CODEOWNERS, PR template, etc.)
- Cargo.toml edition or package name without explicit direction

## Common Tasks

### Add a Test

1. Choose the matching integration test file under `tests/`
2. Write or reuse an ASCII grid helper for board scenarios
3. Use grid patterns with `#` = Alive, `.` = Dead
4. Prefix valid boundary tests with `edge_case_`
5. Prefix invalid input or error-message tests with `negative_`
6. Run `cargo test` to verify

**Example:**
```rust
#[test]
fn test_beacon_pattern() {
    let mut board = board_from_grid(&[
        ".##",
        "##.",
        "...",
    ]);
    board.advance_generation();
    // Assert expected state
}
```

### Fix a Failing Test

1. Identify failing test in `cargo test` output
2. Run just that test: `RUST_BACKTRACE=full cargo test test_name -- --nocapture`
3. Add `println!("{}", board)` statements to see state at each step
4. Review generation logic in `Board::advance_generation()` and `count_live_neighbors()`
5. Verify neighbor counting treats Alive/Dying as "originally live"

### Optimize Generation Performance

Do not optimize prematurely. Current implementation is correct and suitable for small/medium boards.

If profiling shows `advance_generation()` is a bottleneck:
1. Use `cargo flamegraph` or profiler to identify hot loops
2. Consider: bit-packing cells, sparse representation, SIMD
3. Preserve correctness and test coverage during refactoring
4. Update `docs/design.md` with new optimization rationale

### Extend Console App

Current: configurable board size and max iterations with defaults of 5×5 and 10 generations. The initial pattern is a deterministic blinker.

Future enhancements (if requested):
- Accept pattern from file or stdin
- Make board size configurable
- Add interactive step-through mode
- Implement pattern library (gliders, beacons, etc.)

Any change must:
1. Maintain deterministic output for CI smoke test
2. Build and run on both Windows and Linux
3. Not introduce external dependencies
4. Include updated docs if behavior changes

## CI Pipeline

Workflow: `.github/workflows/ci.yml`

**On push or PR to this branch**, CI:
1. Runs on windows-latest and ubuntu-latest
2. Checks format, clippy, tests, build
3. Smoke tests console binary on each platform
4. Fails if any check fails

**Local pre-commit**: Always run all checks locally before pushing:
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
./target/release/game-of-life  # Verify runs
```

## Design Constraints

**Do not violate:**
1. **Bounded board**: Out-of-bounds = dead. No toroidal wrapping.
2. **Single buffer**: One board copy with transitional states. No double-buffering.
3. **Two-pass generation**: Mark pass (compute) + Normalize pass (finalize).
4. **Neighbor counting rule**: Alive + Dying = "originally live" for counting.
5. **Final state**: After generation, only Dead/Alive exist (no transitional states).
6. **No external crates**: Keep dependency-free for simplicity and portability.

## Files at a Glance

| File | Purpose |
|------|---------|
| `src/board.rs` | Board, CellState, display, and generation logic |
| `src/config.rs` | SimulationConfig, BoardSize, CLI/config parsing, and typed errors |
| `src/lib.rs` | Library module declarations and public re-exports |
| `src/main.rs` | Console app and process-level CLI behavior |
| `tests/board_tests.rs` | Board API and generation behavior tests |
| `tests/config_tests.rs` | Configuration and parser tests |
| `tests/cli_tests.rs` | End-to-end binary CLI tests |
| `Cargo.toml` | Project manifest |
| `.github/workflows/ci.yml` | CI: repository hygiene, format, lint, test, build, smoke test |
| `docs/design.md` | Full design rationale and tradeoffs |
| `docs/product-code.md` | Product module maintenance guidance |
| `docs/testing.md` | Test organization and labeling guidance |
| `docs/architecture.excalidraw` | Flow diagram of algorithm |
| `.github/copilot-instructions.md` | Detailed Copilot guidance |
| `AGENTS.md` | This file |

## When to Escalate

Ask for human review if:
- Changing the generation algorithm in fundamental ways
- Adding external dependencies
- Modifying CI in a way that breaks existing workflows
- Unsure whether a change violates design constraints
- Need to update branch strategy or merge targets

## Success Criteria

A complete task:
- ✅ All tests pass
- ✅ All lints/format checks pass
- ✅ Console binary builds and runs deterministically
- ✅ Changes documented (README, design.md, or inline comments)
- ✅ Committed with descriptive message + Co-authored-by trailer

## Speed Tips

- Batch format and lint fixes: `cargo fmt && cargo clippy --fix`
- Run tests in parallel: `cargo test -- --test-threads=4`
- Use release build for console app: `cargo build --release` (faster execution)
- Reference `docs/design.md` before modifying core algorithm (saves investigation time)
