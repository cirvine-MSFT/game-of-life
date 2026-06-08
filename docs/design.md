# Game of Life Design Document

## Overview

This document describes the design of the Rust implementation of Conway's Game of Life, including architectural decisions, implementation rationale, and tradeoffs.

## Bounded Board Design

**Decision**: Use a finite, bounded board with no toroidal wrapping.

**Rationale**:
- Simpler to reason about and implement
- Matches typical interview problem constraints
- Makes edge/corner behavior explicit and testable
- Out-of-bounds cells are treated as dead, which provides natural boundary conditions

**Tradeoff**: Patterns are not preserved at edges when they would wrap in toroidal boards, but this is the intended behavior.

## Single Board with Transitional States

**Decision**: Use one board buffer with four cell states (Dead, Alive, Dying, BecomingAlive) instead of maintaining two full board copies.

**Rationale**:
- Reduces memory usage (single buffer instead of double buffering)
- Enables clear, observable state transitions
- Makes the algorithm's two-pass nature explicit and verifiable
- Simplifies board comparison for testing

**How It Works**:
1. **Mark Pass**: Each cell computes its next state and updates to a transitional state
   - Alive cells that survive stay Alive
   - Alive cells that die become Dying
   - Dead cells that become alive become BecomingAlive
   - Dead cells that stay dead remain Dead
2. **Normalize Pass**: Convert transitional states to final states
   - Dying → Dead
   - BecomingAlive → Alive
   - Final board contains only Dead and Alive

**Why Two Passes?**:
- Ensures neighbor counting treats cells correctly during the mark pass
- Allows us to distinguish "originally live" from "just became alive"
- Separates concerns: computation (mark) vs. state cleanup (normalize)

## Cell State Lifecycle

```
Dead ──[3 neighbors]──→ BecomingAlive ──[normalize]──→ Alive
Alive ──[2-3 neighbors]──→ Alive
Alive ──[<2 or >3 neighbors]──→ Dying ──[normalize]──→ Dead
```

**Neighbor Counting Rule**: During the mark pass, count Alive AND Dying as originally live, Dead AND BecomingAlive as originally dead. This ensures that:
- Cells that were alive at the start of the generation are counted consistently
- The one-board design doesn't corrupt neighbor calculations
- Transitional states don't interfere with the generation's outcome

## Console Application Design

**Pattern**: Horizontal blinker (3 cells in a row) at the center of a 5×5 board

**Output**:
- Shows initial state (generation 0)
- Advances and prints each generation (0-10)
- Uses Unicode block characters (█ for alive, · for dead) for clarity

**Determinism**: The pattern and iteration count are hardcoded, ensuring consistent output for smoke tests

**Extensibility**: The design is ready for:
- User input patterns
- Variable board sizes
- Interactive or step-through modes
- Configurable iteration counts

## Rust/Cargo Project Structure

```
src/
  lib.rs       - Core Game of Life library (Board, CellState, generation logic)
  main.rs      - Console application binary
Cargo.toml     - Project manifest with library and binary targets
```

**Libraries Used**: None (no external dependencies for core logic)

**Testing**: Built-in Rust test framework
- Tests cover still-life, oscillators, edge cases, and state transitions
- Grid-based test construction for readability

## Cross-Platform Considerations

**Windows (Native)**:
- Uses native MSVC toolchain
- Console output works natively
- Binary name: `game-of-life.exe`

**Linux/WSL**:
- Uses GNU toolchain
- Console output via standard stdout
- Binary name: `game-of-life`

**Platform-Agnostic Code**:
- All code uses standard library only (no platform-specific APIs)
- CI runs on both windows-latest and ubuntu-latest to catch issues
- Shell detection in CI workflow handles platform differences in binary paths

## CI/CD Design

**Workflow File**: `.github/workflows/rust-ci.yml`

**Checks** (run on matrix: windows-latest, ubuntu-latest):
1. `cargo fmt --check` - Ensures code formatting
2. `cargo clippy --all-targets -- -D warnings` - Catches common mistakes and style issues
3. `cargo test --verbose` - Runs all tests
4. `cargo build --release` - Ensures production build succeeds
5. Smoke test of console binary - Verifies runtime behavior

**Trigger**: Push or pull request on the feature branch or main

**Benefit**: Catches platform-specific issues early and ensures consistent code quality

## Testing Strategy

**Test Cases**:
1. **Still-Life Block** (2×2 alive square): Remains unchanged after one generation
2. **Blinker Oscillator** (3 cells in a row): Alternates between horizontal and vertical every generation
3. **Edge Cells**: Out-of-bounds neighbors are treated as dead
4. **Corner Cells**: Verify bounded semantics at all edges
5. **No Transitional States Remain**: After generation, only Dead and Alive exist
6. **Neighbor Counting Preserve**: Transitional states don't corrupt calculations

**Test Pattern**: ASCII grid construction for readability and maintainability

## Design Tradeoffs

| Decision | Benefit | Cost |
|----------|---------|------|
| Single board with transitional states | Memory efficient, clear algorithm | Requires two passes per generation |
| Bounded board (no wrapping) | Simpler edge semantics | Patterns don't preserve at edges |
| No external dependencies | Lightweight, portable | Must implement everything from scratch |
| Hardcoded console pattern | Deterministic, easy to test | Less flexible for exploration |
| Unicode block output | Beautiful, readable | May not render on all terminals |

## Future Enhancements

1. **User Input**: Accept board patterns via file or command-line
2. **Board Sizes**: Make dimensions configurable
3. **Interactive Mode**: Step through generations interactively
4. **Performance**: Profile and optimize for large boards (consider bit-packing for cells)
5. **Visualization**: Integrate with GUI library for real-time animation
6. **Patterns Library**: Pre-built patterns (gliders, oscillators, spaceships)

## References

- [Conway's Game of Life - Wikipedia](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life)
- [Patterns and behaviors](https://www.conwaylife.com/)
