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

## Comment Style

Comments must earn their place. The rules:

1. **No comments that just restate what the code or signature already says.** If the doc paraphrases the function name, types, or body, delete it — Rust signatures and well-named symbols carry that information by themselves.
2. **Do comment WHY.** Design rationale, threat models, performance tradeoffs, historical decisions, and non-obvious gotchas all belong in comments. The reader can see *what* the code does; you're there to explain *why it's that way*.
3. **Do document non-obvious contracts.** Preconditions, postconditions, panic conditions, ordering requirements, and accepted input shapes are not always inferable from the signature and are worth a comment.
4. **If the code is hard to read, fix the code first.** Reach for comments only when restructuring/renaming wouldn't have made the code self-explanatory.

Examples that pass: explaining why a `--continue` cumulative max is rejected when `<= source.iterations_run`; documenting that `parse_run_id` deliberately skips v4-bit enforcement so synthetic test IDs round-trip; noting that the file integrity hash is for accidental edits and bit flips, not adversarial tampering.

Examples that fail and should be removed: `/// Builds a snapshot wrapping the given board.` over `fn for_board(board) -> Self`; `/// Number of cells that transitioned from dead to alive` over a `pub births: u64` field.

## Test Style

All tests live in `tests/` (Cargo integration test style). No inline `#[cfg(test)] mod tests {}` blocks inside `src/` modules — they're idiomatic in Rust but **not used in this repo**.

Two consequences worth being explicit about:

1. **Tests can only see the public API.** Integration tests sit outside the crate, so they cannot reach private or `pub(crate)` items. Private implementation details are free to refactor without breaking tests.
2. **Tests verify contracts, not implementations.** If a behavior is worth testing, the corresponding function/type/value should be public. If you find yourself wanting to test a private helper, either:
   - Test the public function that calls it (the helper is implementation detail), OR
   - Promote the helper to public if it really is a meaningful library surface.

File naming — Rust integration tests use the `_tests.rs` suffix so test files are distinguishable from product files by filename alone. Top-level `tests/*_tests.rs` files are Cargo test binaries. Grouped child modules under `tests/<module>/` also use `_tests.rs` and are included by a suffixed wrapper.

```
src/                            tests/
  persistence/  ───────────►      persistence_tests.rs    ← wrapper (cargo test entry)
    hash.rs                       persistence/
    magic.rs                        hash_tests.rs         ← tests for src/persistence/hash.rs
    ...                             magic_tests.rs
                                    ...
  stats/        ───────────►      stats_tests.rs
    run_statistics.rs               stats/
                                    run_statistics_tests.rs
  config.rs     ───────────►      config_tests.rs        ← flat module → flat test file
  algorithms/   ───────────►      algorithms_tests.rs
  pattern_analysis/ ───────►      pattern_analysis_tests.rs
desktop/src/
  session.rs    ───────────►      desktop/tests/session_tests.rs
```

Why the wrapper file: cargo's integration-test discovery only picks up top-level `tests/*.rs` files as test binaries. Files inside `tests/<module>/` need a `tests/<module>_tests.rs` wrapper that declares each submodule with `#[path = "<module>/<file>_tests.rs"] mod <file>_tests;`. The `#[path]` is required because each `tests/*.rs` is its own crate root, so the default `mod foo;` lookup looks for `tests/foo.rs` (sibling) instead of `tests/<wrapper>/foo.rs` (child). See `tests/persistence_tests.rs` and `tests/stats_tests.rs` for the pattern.

End-to-end binary-driven tests (e.g. CLI tests that drive the actual binary via `Command::new(env!("CARGO_BIN_EXE_game-of-life"))`) live alongside their module — `tests/persistence/cli_tests.rs` covers binary-driven persistence behavior.

## What Can Be Changed

✅ **Safe to modify:**
- Board interfaces and implementation in `src/board/` (BoardView, BoardEditor, InMemoryBoard, CellState)
- Core update algorithms in `src/algorithms/`
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
4. Review generation logic in `InMemoryBoard::advance_generation()` and the active updater implementation
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
| `src/board/` | Board traits, CellState, InMemoryBoard, display, and generation convenience |
| `src/algorithms/` | BoardInitializer, BoardUpdater, and concrete initializer/update implementations |
| `src/config.rs` | SimulationConfig, BoardSize, CLI/config parsing, and typed errors |
| `src/lib.rs` | Library module declarations and public re-exports |
| `src/main.rs` | Console app and process-level CLI behavior |
| `tests/algorithms_tests.rs` | Algorithm initializer and updater behavior tests |
| `tests/board_tests.rs` | Board API and generation behavior tests |
| `tests/config_tests.rs` | Configuration and parser tests |
| `tests/cli_tests.rs` | End-to-end binary CLI tests |
| `tests/pattern_analysis_tests.rs` | Pattern analyzer behavior tests |
| `tests/persistence_tests.rs`, `tests/persistence/*_tests.rs` | Persistence behavior and CLI flow tests |
| `tests/stats_tests.rs`, `tests/stats/*_tests.rs` | Statistics behavior tests |
| `tests/streaming_tests.rs`, `tests/streaming/*_tests.rs` | Streaming board behavior tests |
| `desktop/tests/*_tests.rs` | Desktop Rust integration tests |
| `desktop/ui/src/**/*.test.ts(x)` | Desktop UI Vitest tests |
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
