# Copilot Instructions for Rust Game of Life

This document provides guidance for GitHub Copilot when working on the Rust Game of Life prototype branch.

## Branch Context

- **Branch**: `cirvine-msft/fictional-train`
- **Project**: Game of Life (Rust prototype)
- **Purpose**: Single-language prototype demonstrating Conway's Game of Life with bounded board and transitional cell states

## Project Structure

```
src/
  lib.rs          Library with Board, CellState, generation logic, and tests
  main.rs         Console application (5x5 blinker demo)
Cargo.toml        Project manifest with library + binary targets
.github/
  workflows/
    rust-ci.yml   CI workflow for format/lint/test/build on Windows/Linux
  copilot-instructions.md  This file
docs/
  design.md       Full design rationale and architecture notes
  architecture.excalidraw  Flow diagram of board/generation algorithm
```

## Rust Conventions Used

- **Edition**: 2021
- **Dependencies**: Zero external crates (std library only)
- **Testing**: Built-in `#[cfg(test)]` framework with ASCII grid helpers
- **Code Style**: Enforced by `cargo fmt` and `cargo clippy`

## Key Design Decisions

1. **Single Board + Transitional States**: One buffer with Dead/Alive/Dying/Resurrecting states
   - No double-buffering; all computation in-place
   - Two-pass generation: Mark (compute) → Normalize (finalize)
   - Enables clear neighbor counting that treats "originally live" consistently

2. **Bounded Board**: Out-of-bounds neighbors are dead; no toroidal wrapping
   - Simpler semantics for interview context
   - Make edge/corner cases explicit in tests

3. **No External Dependencies**: All logic in std library
   - Simpler deployment
   - Easier to review and extend

4. **Console Application**: Hardcoded 5×5 blinker pattern for deterministic output
   - Ideal for smoke testing in CI
   - Ready to accept user input or patterns as future enhancement

## Development Workflow

### Adding a New Test

Create an ASCII grid helper in `src/lib.rs`:

```rust
fn board_from_grid(lines: &[&str]) -> Board {
    let height = lines.len();
    let width = if height > 0 { lines[0].len() } else { 0 };
    let mut board = Board::new(width, height);
    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let state = match ch {
                '#' => CellState::Alive,
                '.' => CellState::Dead,
                _ => CellState::Dead,
            };
            board.set(x, y, state);
        }
    }
    board
}

#[test]
fn test_my_pattern() {
    let mut board = board_from_grid(&["...", ".#.", "..."]);
    board.advance_generation();
    let expected = board_from_grid(&["...", "...", "..."]);
    assert_eq!(board, expected);
}
```

### Modifying Board Logic

When changing `Board::advance_generation()`:
1. Run full test suite: `cargo test`
2. Check lints: `cargo clippy --all-targets -- -D warnings`
3. Format: `cargo fmt`
4. Verify console app still runs: `cargo build && ./target/debug/game-of-life`

### Extending Console App

Current pattern is a 5×5 blinker (hardcoded). To add features:
- **Pattern generation**: Refactor into helper functions or patterns module
- **Board size**: Make `WIDTH` and `HEIGHT` configurable
- **Iteration count**: Add command-line argument parsing
- **File input**: Consider pattern file format (RLE, plaintext)

When modifying `main.rs`, ensure the binary still builds and runs deterministically for CI.

## Testing Requirements

Every feature must have corresponding tests:
- **Still-life patterns**: Should not change
- **Oscillators**: Should return to initial state after N generations
- **Edge semantics**: Out-of-bounds neighbors are dead
- **Transitional states**: Never remain after generation completes
- **Neighbor counting**: During mark pass, treats Alive/Dying as live

All tests must use readable ASCII grids with `#` for alive, `.` for dead.

## CI/CD Pipeline

Workflow: `.github/workflows/rust-ci.yml`

**Runs on**: `windows-latest` and `ubuntu-latest`

**Checks**:
1. `cargo fmt --check` – Format verification
2. `cargo clippy --all-targets -- -D warnings` – Lints as errors
3. `cargo test --verbose` – Unit tests
4. `cargo build --release` – Production build
5. Console binary smoke test – Runs built binary and verifies no crashes

If CI fails, review the workflow logs and ensure all local checks pass before pushing.

## Performance Considerations

Current design is suitable for small boards (e.g., up to 100×100):
- Single buffer passes: O(width × height × neighbors) per generation
- No optimization premature applied

For very large boards (1000+×1000+), consider:
- Bit-packing cell states (saves 75% memory)
- Sparse board representation (only track live cells)
- SIMD operations for neighbor counting

Keep the current simple design until profiling shows it's a bottleneck.

## Documentation

- `docs/design.md`: Full design rationale, tradeoffs, future work
- `docs/architecture.excalidraw`: Flow diagram of board and generation steps
- `README.md`: Build/test/run commands for Windows and Linux

Update docs when:
- Adding significant new features
- Changing generation algorithm or board representation
- Making performance optimizations
- Deciding to defer future work

## Known Limitations

1. **Board size**: Hardcoded in console app (5×5). Future enhancement: make configurable.
2. **Patterns**: Only blinker in console app. Future: pattern library or file input.
3. **Interactivity**: No step-through or interactive mode. Future: add if needed.

## Extending to C++ (Sibling Branch)

A parallel C++ implementation exists on `cirvine-msft/prototype-spike-plan`. The design spec is shared:
- Bounded board, single buffer, transitional states, two-pass generation
- Same test cases and patterns
- Both must demonstrate the same core algorithm

Do not merge or cross-pollinate between branches unless explicitly directed.

## Debugging Tips

### Test Failures
- Use `RUST_BACKTRACE=full cargo test` to see full stack traces
- Add `println!` statements in tests to inspect board state
- Print boards using `println!("{}", board)` to visualize state

### Runtime Crashes
- Run `cargo miri` if available (detects UB, though not all issues)
- Check bounds in `Board::get()` and `Board::set()` for off-by-one errors
- Ensure `advance_generation()` handles edge cells correctly

### Performance Issues
- Use `cargo flamegraph` (or `perf` on Linux) to profile hot paths
- Current implementation is O(width × height) per generation—acceptable for benchmarks

## Questions?

Refer to:
- `docs/design.md` for architectural decisions
- `src/lib.rs` for implementation details and test examples
- `README.md` for build and run commands
