# Game of Life

An interview-style Conway's Game of Life project designed to start small and grow over time.

## Project goals

- Model the rules of Conway's Game of Life clearly.
- Keep the implementation easy to extend for future interview prompts.
- Add tests and refactor as the project gains features.

## Current scope

This repository is intentionally runtime-neutral for now. The first implementation can choose the language, test framework, and interface that best fit the next exercise.

## Rust Prototype Implementation

The Rust implementation uses a bounded board with transitional cell states and a two-pass generation algorithm. See [docs/design.md](docs/design.md) for detailed design rationale.

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
```

### Algorithm Overview

- **Board**: Finite, bounded (out-of-bounds neighbors are dead; no toroidal wrapping)
- **Cell States**: Dead, Alive, Dying, Resurrecting (transitional states enable single-board generation)
- **Generation Advancement**:
  1. **Mark Pass**: Compute each cell's next state using transitional states
  2. **Normalize Pass**: Convert Dying → Dead and Resurrecting → Alive
- **Neighbor Counting**: Alive and Dying treated as originally live; Dead and Resurrecting treated as originally dead
- **Result**: After generation, board contains only Dead and Alive states

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

