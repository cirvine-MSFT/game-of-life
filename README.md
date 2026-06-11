# Game of Life

An interview-style Conway's Game of Life project designed to start small and grow over time.

## Project goals

- Model the rules of Conway's Game of Life clearly.
- Keep the implementation easy to extend for future interview prompts.
- Add tests and refactor as the project gains features.

## Customer lens

This project is also framed as reproducible simulation software for research, education, and interview discussion. See [CUSTOMERS.md](CUSTOMERS.md) for the personas, customer jobs, and roadmap lens that should guide future feature work.

## Current scope

This repository is intentionally runtime-neutral for now. The first implementation can choose the language, test framework, and interface that best fit the next exercise.

## Rust Prototype Implementation

The Rust implementation uses a bounded board with trait-based initialization and update algorithms. The default update algorithm uses transitional cell states and a two-pass generation flow. See [docs/design.md](docs/design.md) for detailed design rationale and [docs/decision-rust.md](docs/decision-rust.md) for the language choice record.

### Build and Run (Windows)

```powershell
# Format and lint check
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test

# Build
cargo build --release

# Run console application
.\target\release\game-of-life.exe

# Show CLI options
.\target\release\game-of-life.exe --help

# Run with a 10x10 board for 25 generations
.\target\release\game-of-life.exe --board-size 10x10 --max-iterations 25
```

### Build and Run (Linux / WSL)

```bash
# Format and lint check
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Run tests
cargo test

# Build
cargo build --release

# Run console application
./target/release/game-of-life

# Show CLI options
./target/release/game-of-life --help

# Run with a 10x10 board for 25 generations
./target/release/game-of-life --board-size 10x10 --max-iterations 25
```

The console app prints concise run information and the final board state only. Per-generation board output is intentionally omitted for now to keep runs readable.

### Command-line options

| Option | Description | Default |
|--------|-------------|---------|
| `-h`, `--help` | Print usage and supported options. | N/A |
| `-b`, `--board-size <WIDTHxHEIGHT>` | Set the bounded 2D board size, such as `5x5` or `10x20`. | `5x5` |
| `-m`, `--max-iterations <COUNT>` | Set how many generations to run. Use `0` to print the initial board as the final state. | `10` |

### Algorithm Overview

- **Board Implementation**: `InMemoryBoard` is the current finite, bounded board implementation (out-of-bounds neighbors are dead; no toroidal wrapping)
- **Board Access Traits**: Algorithms use fallible `BoardView`/`BoardEditor` traits instead of concrete board storage, including grouped coordinate reads for custom neighborhoods or future storage batching
- **Initialization Interface**: `BoardInitializer` is the trait for seeding a board; concrete implementations include `CenteredBlinkerInitializer` and seedable `RandomBoardInitializer`
- **Update Interface**: `BoardUpdater` advances a board; the default is `InPlaceTransitionalUpdater`
- **Cell States**: Dead, Alive, Dying, Resurrecting (transitional states enable single-board generation)
- **Default Generation Advancement**:
  1. **Mark Pass**: Compute each cell's next state using transitional states
  2. **Normalize Pass**: Convert Dying → Dead and Resurrecting → Alive
- **Neighbor Counting**: Alive and Dying treated as originally live; Dead and Resurrecting treated as originally dead
- **Result**: After generation, board contains only Dead and Alive states
- **Configuration**: CLI selection of alternate algorithms is intentionally deferred; current CLI behavior remains deterministic

### Architecture diagram

![Game of Life architecture and algorithm flow](docs/architecture.png)

Editable source: [docs/architecture.excalidraw](docs/architecture.excalidraw)

## Conway's Game of Life rules

For each generation:

- Any live cell with fewer than two live neighbors dies.
- Any live cell with two or three live neighbors lives.
- Any live cell with more than three live neighbors dies.
- Any dead cell with exactly three live neighbors becomes alive.

## Suggested next steps

1. Choose the initial language and test framework.
2. Add a small board representation.
3. Implement generation advancement.
4. Add tests for stable, oscillator, and edge-case patterns.

## Maintenance guidance

See [docs/product-code.md](docs/product-code.md) for product module conventions and [docs/testing.md](docs/testing.md) for test organization, `edge_case_` labels, and `negative_` labels.
