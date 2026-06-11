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

## Algorithm Pattern Design

**Decision**: Use Rust traits to separate board storage from board initialization and update algorithms.

The core algorithm-facing traits are:

- `BoardView`: fallible read-only random-access board surface exposing dimensions, single-cell reads, and grouped coordinate reads
- `BoardEditor`: fallible mutable random-access board surface that extends `BoardView` with cell writes
- `BoardInitializer`: interface for algorithms that initialize a board with a starting state
- `BoardUpdater`: interface for algorithms that advance a board by one generation

The current concrete defaults are:

- `DemoBoardInitializer`: seeds the deterministic product/UI demo pattern used by the console app by default
- `CenteredBlinkerInitializer`: seeds a deterministic centered horizontal blinker for oscillator demos and tests
- `RandomBoardInitializer`: fills every cell from a seedable pseudo-random sequence, using an alive-cells-per-thousand density for reproducible experiments
- `InPlaceTransitionalUpdater`: applies Conway's rules using the existing single-buffer, two-pass transitional-state algorithm

**Rationale**:

- Keeps algorithms decoupled from concrete board implementations such as `InMemoryBoard`
- Preserves the current public convenience method, `InMemoryBoard::advance_generation()`, while allowing callers to use explicit updaters
- Creates a natural seam for future behavior variants, runtime/space tradeoff comparisons, and alternate board storage backends
- Allows CLI/configuration support for initial board source selection without changing board storage internals

**Grouped Reads**:

`BoardView::read_cells()` accepts a caller-provided list of `CellCoordinate` values and fills a caller-provided output collection in the same order. Board reads and writes return `Result` values, so future file-backed implementations can surface I/O, flush, or corruption errors instead of panicking or silently dropping failures. This is intentionally more flexible than only exposing single-cell reads because future algorithms may:

- Define neighborhoods differently from Conway's eight adjacent cells
- Inspect larger stencils or arbitrary cell collections
- Batch reads for file-backed, chunked, or streaming board storage

The current in-memory board still resolves grouped reads through direct coordinate lookups. Future storage implementations can preserve the same trait contract while optimizing the underlying access pattern.

These traits are still random-access board interfaces, not true streaming interfaces. A future streaming board will likely add region, row-window, or source/destination traits once that storage model is designed.

An explicit all-dead initializer is not currently needed because `InMemoryBoard::new()` already starts every cell as `Dead`. If future workflows need to reset an existing board through the initializer interface, a concrete clear/reset initializer can be added without changing the trait.

## Default Single Board Update with Transitional States

**Decision**: Keep the default update algorithm on one board buffer with four cell states (Dead, Alive, Dying, Resurrecting) instead of maintaining two full board copies.

**Rationale**:
- Reduces memory usage (single buffer instead of double buffering)
- Enables clear, observable state transitions
- Makes the algorithm's two-pass nature explicit and verifiable
- Simplifies board comparison for testing

**How It Works**:
1. **Mark Pass**: Each cell computes its next state and updates to a transitional state
   - Alive cells that survive stay Alive
   - Alive cells that die become Dying
   - Dead cells that become alive become Resurrecting
   - Dead cells that stay dead remain Dead
2. **Normalize Pass**: Convert transitional states to final states
   - Dying → Dead
   - Resurrecting → Alive
   - Final board contains only Dead and Alive

**Why Two Passes?**:
- Ensures neighbor counting treats cells correctly during the mark pass
- Allows us to distinguish "originally live" from "just became alive"
- Separates concerns: computation (mark) vs. state cleanup (normalize)

## Cell State Lifecycle

```
Dead ──[3 neighbors]──→ Resurrecting ──[normalize]──→ Alive
Alive ──[2-3 neighbors]──→ Alive
Alive ──[<2 or >3 neighbors]──→ Dying ──[normalize]──→ Dead
```

**Neighbor Counting Rule**: During the mark pass, count Alive AND Dying as originally live, Dead AND Resurrecting as originally dead. This ensures that:
- Cells that were alive at the start of the generation are counted consistently
- The one-board design doesn't corrupt neighbor calculations
- Transitional states don't interfere with the generation's outcome

## In-Memory Board Budget

**Decision**: Bound `InMemoryBoard` allocation through a configurable byte budget.

The CLI accepts `--max-board-memory <SIZE>`, where `SIZE` can be raw bytes or a whole-number `B`, `KB`, `MB`, or `GB` value. The value is stored as bytes in `SimulationConfig`.

Validation order:

1. Parse board dimensions as positive values representable by `usize`
2. Compute `width * height` with checked arithmetic
3. Compute requested cell-buffer bytes with `size_of::<CellState>()`
4. Reject byte counts above the addressable allocation limit
5. Reject byte counts above the configured memory budget

Primitive/addressability limits are not user-overridable. Users can raise the configured memory budget, but they cannot bypass dimension parsing, checked multiplication, or addressable allocation limits.

**Future seam**: The memory budget is intentionally expressed in bytes rather than cells or dimensions. Future file-backed boards can use the same setting to decide when to prefer streaming storage or how much of a file-backed board may be staged in memory. File-backed storage will still need separate disk, offset, and metadata validation.

## Console Application Design

**Default initial board**: Curated deterministic demo pattern that adapts to the configured board size.

The demo initializer uses isolated 10x10 tiles on boards large enough to hold them. Larger boards receive repeated tiles separated by dead gutters so each tile evolves independently and settles within 20 generations. Smaller boards receive compact motifs that stay in bounds and settle quickly.

The console app now uses the algorithm abstractions internally:

1. Create an `InMemoryBoard`
2. Apply the selected initial board source:
   - `demo` -> `DemoBoardInitializer`
   - `blinker` -> `CenteredBlinkerInitializer`
   - `random` -> `RandomBoardInitializer` with a fresh runtime-generated seed
3. Advance with `InPlaceTransitionalUpdater` for the configured iteration count

The CLI option `--initial-board <demo|blinker|random>` selects the source of the initial board. The source-oriented name leaves room for future values such as `file:<PATH>` without exposing Rust trait names in the command-line interface.

**Output**:
- Shows concise run information
- Advances to the configured maximum iteration count
- Prints the final board state only
- Uses ASCII characters (`#` for alive, `.` for dead) for platform-neutral console output

**Determinism**: The default demo and blinker patterns are deterministic, ensuring consistent smoke-test output. The `random` source intentionally generates a fresh random board each run; future save/resume work should persist the generated initial state when reproducibility is needed.

**Extensibility**: The design is ready for:
- File-based initial board patterns
- Saved run snapshots that restore both board cells and run metadata such as generation index
- File-backed board storage and streaming windows bounded by the memory budget
- Variable board sizes
- Configurable iteration counts
- Optional per-generation, interactive, or step-through output modes

## Rust/Cargo Project Structure

```
src/
  algorithms/  - Algorithm traits and concrete initializer/update implementations
  board/       - Board traits, cell model, and concrete board implementations
  config.rs    - SimulationConfig, BoardSize, and CLI/config parsing
  lib.rs       - Public module declarations and re-exports
  main.rs      - Console application binary
tests/
  board_tests.rs   - Board API and Game of Life behavior
  config_tests.rs  - Configuration and parser behavior
  cli_tests.rs     - End-to-end binary behavior
Cargo.toml     - Project manifest with library and binary targets
```

**Libraries Used**: None (no external dependencies for core logic)

**Testing**: Cargo integration tests under `tests/`
- Tests cover still-life, oscillators, edge cases, negative parser cases, CLI behavior, and state transitions
- Grid-based test construction keeps board expectations readable
- `edge_case_` labels identify valid boundary behavior
- `negative_` labels identify invalid input and actionable error-message behavior

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

**Workflow File**: `.github/workflows/ci.yml`

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
| Trait-based algorithms | Decouples behavior from storage and enables future variants | Adds a small abstraction layer |
| Grouped coordinate reads | Supports custom neighborhoods and future batched storage reads | Current in-memory implementation still resolves individual cells |
| ASCII console output | Portable across Windows, Linux, and CI logs | Less visually rich than Unicode output |
| Final-state-only default output | Keeps CLI runs readable | Requires a future option for generation-by-generation viewing |

## Future Enhancements

Future work should be guided by the customer lens in [../CUSTOMERS.md](../CUSTOMERS.md). The current bounded 2D implementation remains the baseline, but research and education scenarios point toward reproducible experiment configuration, richer visualization, and analyzable outputs.

1. **User Input**: Accept board patterns via file or command-line
2. **Board Sizes**: Make dimensions configurable
3. **Experiment Configuration**: Record initial state, rules, boundary behavior, update mode, random seed, iteration limit, and software version so findings can be reproduced
4. **Batch Runs**: Run many simulations across initial states and configuration variables, then aggregate outcomes
5. **Pattern Analysis**: Detect still lifes, oscillators, periods, spaceships, extinction, and long transients
6. **Visualization and Replay**: Provide views that explain board evolution better than console board dumps
7. **Interactive Mode**: Step through generations interactively
8. **Performance and Storage**: Profile and optimize for large boards or many independent runs. Consider bit-packing, sparse representations, chunked file-backed storage, and streaming reads/writes when justified.
9. **Telemetry**: Expose operations, timing, and memory-relevant dimensions for educator-facing algorithm comparisons
10. **Patterns Library**: Pre-built patterns (gliders, oscillators, spaceships)

## References

- [Conway's Game of Life - Wikipedia](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life)
- [Patterns and behaviors](https://www.conwaylife.com/)

## Architecture Diagram

The editable architecture diagram is maintained in [architecture.excalidraw](architecture.excalidraw). A static PNG export is linked from the repository README for easier Markdown viewing.
