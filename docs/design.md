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
- `FullyAliveInitializer`: seeds every in-bounds cell as alive
- `BlinkerBoardInitializer`: seeds the deterministic centered horizontal blinker used by `--initial-board blinker`
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
   - `alive` -> `FullyAliveInitializer`
   - `blinker` -> `BlinkerBoardInitializer`
   - `random` -> `RandomBoardInitializer` with a fresh runtime-generated seed
3. Advance with `InPlaceTransitionalUpdater` for the configured iteration count

The CLI option `--initial-board <demo|alive|blinker|random>` selects the source of the initial board. The source-oriented name leaves room for future values such as `file:<PATH>` without exposing Rust trait names in the command-line interface.

The fully alive source is useful for exercising overpopulation behavior, but it is not a rich long-running demo: boards larger than `2x2` usually collapse to corner cells after one generation and then die; a `2x2` fully alive board is the standard stable block.

**Output**:
- Shows concise run information
- Advances until the configured maximum iteration count, extinction, fixed-point stability, or an exact repeated board state
- Prints the final board state only
- Reports `Stable state reached at generation N` when a no-op attempted generation confirms generation `N` was already fixed-point stable
- Uses ASCII characters (`#` for alive, `.` for dead) for platform-neutral console output

Stable-state detection deliberately means fixed-point still-life detection, not period-greater-than-1 cycle detection. Oscillators such as blinkers and toads are reported as `cyclic` when their exact board state repeats. A fully dead board is terminal under Conway's B3/S23 rule because births require exactly three live neighbors, so extinction is safe to treat as an early stop.

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
  config.rs    - SimulationConfig, BoardSize, CLI/config parsing
  lib.rs       - Public module declarations and re-exports
  main.rs      - Console application binary
  persistence/ - Run record and board snapshot file IO (zero deps)
  stats/       - Per-generation AdvanceOutcome and RunStatistics
tests/
  board_tests.rs            - Board API and Game of Life behavior
  config_tests.rs           - Configuration and parser behavior
  cli_tests.rs              - End-to-end binary behavior for the core run
  persistence_cli_tests.rs  - End-to-end save/load/replay/extract/continue
Cargo.toml     - Project manifest with library and binary targets
```

**Libraries Used**: None (no external dependencies for core logic)

**Testing**: Cargo integration tests under `tests/`
- Tests cover still-life, oscillators, edge cases, negative parser cases, CLI behavior, and state transitions
- Grid-based test construction keeps board expectations readable
- `edge_case_` labels identify valid boundary behavior
- `negative_` labels identify invalid input and actionable error-message behavior

## Persistence Design

Every successful run auto-saves a **run record** to disk. The same parser handles two related file types so users can extract, edit, and share board states without losing the audit trail.

### File types

| Type | Magic | Purpose | Hash |
|------|-------|---------|------|
| **Run record** | `GOL-RUN-RECORD v1` | Full record of one simulation: config, statistics, initial board, final board | `content_hash` trailer (mandatory) |
| **Board snapshot** | `GOL-BOARD-SNAPSHOT v1` | Standalone board with a tiny header; hand-craftable, hand-editable | None — intentionally hash-free |

The two types share one parser pair. A run record embeds two fenced board blocks (`INITIAL BOARD` and `FINAL BOARD`); a standalone snapshot is just one fenced block (`BOARD`) with a brief header. The `--extract-board` verb writes a snapshot from a run record's `INITIAL` or `FINAL` block.

### File safety and validation

Every file is sniffed before it's slurped:

- **Magic prefix.** Standard Unix-derived term for a short, fixed marker at the start of a file that identifies its format — same idea as `%PDF-` for PDF files or `#!/usr/bin/env` for shell scripts. The first non-empty line must be one of the recognized magics. Sniffing is bounded to 128 bytes (or the first newline) so it can't be a DoS vector on huge files.
- **Max file size guard.** Before reading the body, `stat()` the file; reject anything larger than `--max-input-file-bytes` (default 256 MiB).
- **Grid integrity.** Inside any board block: declared `size:` must match grid dimensions, every row must have the same width, every cell character must be `.` or `#`, and the derived `alive_count` / `dead_count` headers must match the grid.
- **Section integrity.** Required fields enforced per section; duplicate keys rejected; fence ordering and balance enforced; unknown `schema_version` rejected with a clear "supported versions: [1]" message.
- **Memory-budget validation for loaded boards.** Before allocating a grid, the declared `WxH` is checked against the configured `--max-board-memory`. Three distinct outcomes:
  1. Fits — allocate normally.
  2. Exceeds the budget but is theoretically reachable: `LoadedBoardExceedsMemoryBudget` with a concrete suggested `--max-board-memory` override embedded in the message.
  3. Exceeds anything the platform could hold: `LoadedBoardExceedsAddressableMemory`, pointing at the future streaming-board work as the planned remedy.
- For run records, board-block memory errors are wrapped as `RunRecordReadError::BoardBlockTooLarge { block, run_id, source }` so the message identifies which block failed and which source run it came from.

### Integrity (`content_hash`)

Run records carry a `content_hash:` trailer at the end of the file. Threat model is explicit: accidental edits, partial writes, bit flips. **Not adversarial tamper detection** — a 64-bit non-crypto FNV-1a hash is right-sized for "user made a typo in vim".

- The writer computes the hash over the canonical UTF-8 bytes of everything in the file from the magic line up to (and including) the newline preceding the trailer, then appends the trailer.
- The reader **canonicalizes** the file in-memory before hashing: LF line endings, trimmed trailing whitespace per line, exactly one trailing newline. This means a Windows editor saving the file in CRLF does not break verification.
- Mismatch → `RunRecordReadError::Corrupted { path, expected_hash, actual_hash }`. The user-facing message includes both hashes and offers two concrete remedies:
  1. `--ignore-integrity` if the edit was deliberate (prints a `Warning:` and downgrades per-grid hash mismatches to warnings).
  2. `--extract-board <path> --load-from initial|final --output snapshot.gol` to extract just the board as a freely-editable snapshot.

Board snapshots intentionally do **not** carry `content_hash`. They're designed for hand-crafting and hand-editing — enforcing integrity there would defeat the use case.

### Per-grid hashes

The `[result]` section of every run record also carries `initial_board_hash` and `final_board_hash` (also FNV-1a 64-bit, over the row-by-row ASCII grid bytes). These are cross-reference / de-dup helpers — useful for `grep final_board_hash runs/*.gol` to find runs that ended at the same state. They are verified by the reader (mismatch is a corruption error under `Enforce`; downgraded to a warning under `Ignore`).

### Run statistics

A `RunStatisticsCollector` observes one `AdvanceOutcome` per generation (births, deaths, alive count) and finalizes into a `RunStatistics` value at end of run. The updater reports `AdvanceOutcome` directly from the normalize pass, so stats are O(1) per generation with no extra full-board scan.

Recorded statistics: `status` (`extinct` / `stable` / `cyclic` / `max_iterations`), `iterations_run`, `wall_time_ms`, `initial_alive_count`, `final_alive_count`, `peak_alive_count`, `peak_alive_generation`, `min_alive_count`, `min_alive_generation`, `total_births`, `total_deaths`, and optional cycle metadata (`cycle_start_generation`, `cycle_detected_generation`, `cycle_period`) when `status: cyclic`.

### Early-stop conditions

Three terminal conditions can stop a run before `max_iterations`:

1. **Extinction**: if every cell is dead at generation 0 or after a generation, the run stops with `status: extinct`. A dead board cannot resurrect under B3/S23 because every dead cell has zero live neighbors, not the three required for birth.
2. **Fixed-point stability**: if an attempted non-extinct generation reports `births == 0` and `deaths == 0`, the run stops with `status: stable`. That no-op attempt is a confirmation, not useful simulation work, so `iterations_run` records the previous generation `N` whose board was confirmed stable. A still-life initial board therefore reports `iterations_run: 0`.
3. **Exact cycle detection**: in-memory runs retain exact bit-packed board signatures in a per-run pattern analyzer. If generation `M` has the same dimensions and live/dead cell bits as a previously observed generation `N`, the run stops with `status: cyclic`, `iterations_run: M`, and `cycle_period: M - N`. Extinction and fixed-point stability keep priority because they are cheaper and more specific terminal outcomes.

Streaming-sized runs keep extinction and stability detection but do not retain historical exact signatures yet; doing so could violate the configured memory budget.

### Pattern analysis and board signatures

Pattern analysis is stateful per run/session. A `PatternAnalyzer` owns detector instances and observes generation 0 plus each completed non-terminal generation. The first detector is exact cycle detection; future detectors can report non-terminal notable patterns through the same `PatternMatch` shape without changing the run loop.

The shared `BoardSignature` value is exact, not hash-only: board dimensions, alive-cell count, and row-major bit-packed live/dead cells. In-memory cycle detection stores `BoardSignature -> first_seen_generation` in a hash map, but equality still compares the full signature before reporting a cycle. This avoids false positives while keeping expected lookup O(1).

Signature construction is fused with existing board passes where possible. Generation advancement can return a `GenerationSummary` containing both the existing `AdvanceOutcome` and the post-generation signature built during normalization. Callers that explicitly need a signature outside the hot path can request one through the board signature interface, which may scan the board when no precomputed signature is available.

### CLI surface

- `--runs-dir`, `--save-run`, `--no-save` — control where (and whether) the auto-save lands.
- `--load-board`, `--load-from` — start a new run from a snapshot or a recorded board.
- `--continue`, `--additional-iterations` — load a prior run's FINAL board and run further. Records `continued_from: <source-run-id>` for provenance. The iteration budget can be specified two ways: `--additional-iterations N` means "run for N more steps"; `--max-iterations M` (when paired with `--continue`) means "target a cumulative total of M steps across the chain" and runs for `M - source.iterations_run` more. The two budget flags are mutually exclusive; cumulative `M <= source.iterations_run` is rejected with a clear error.
- `--replay <PATH>` — re-execute a run record and diff final board + key stats.
- `--extract-board <PATH> --output <PATH>` — write a snapshot from a run record's `INITIAL` or `FINAL` block.
- `--ignore-integrity` — opt-in bypass of the `content_hash` check (warns on stderr).
- `--max-input-file-bytes` — per-invocation override of the input file size guard.

### Deferred (future PRs)

- **Streaming board** ✅ — shipped in the streaming PR. See [Streaming Board for Very Large Boards](#streaming-board-for-very-large-boards) below.
- **Cycle detection** — adds `status: cyclic` to the writer for period-greater-than-1 repeats. Format already reserves it.
- **Cryptographic signing** — separate from the integrity check above; would need adversarial threat model.

## Streaming Board for Very Large Boards

**Decision**: When an `--initial-board` (initializer-based) run would
allocate a board larger than `--max-board-memory`, auto-promote to a
file-streaming backend instead of failing.

### Motivating constraint

`InMemoryBoard` is a `Vec<CellState>` whose allocation is bounded by
`--max-board-memory`. The original behavior was: if the board doesn't fit,
surface `MemoryBudgetExceeded` and stop. For very large boards (e.g., a
researcher running 10⁹-cell experiments) or very constrained devices
(e.g., a few-KB cap on small hardware), that's a real wall.

### Algorithm: 2D chunked sliding window with stencil halo

`StreamingBoard` keeps a small rectangular **chunk** of cells in memory
at a time, sliding across the board as the updater scans. The chunk is
itself an `InMemoryBoard` sized to the maximum loaded extent; the
streaming wrapper tracks **which absolute rectangle is currently
loaded** and **which sub-rectangle the chunk position owns** for the
current update step.

Two rectangles per chunk position, both in `usize` global coordinates:

- **Owned**: cells this chunk position will update during the current
  pass. Owned rectangles partition the board — every in-board cell
  belongs to exactly one chunk position's owned region.
- **Loaded**: owned cells + 1-cell halo on each non-board-edge side
  so the 3×3 Conway stencil never has to cross into adjacent chunks.

Out-of-board halo (e.g., would-be left halo when owned starts at
`x = 0`) is **never stored**. The streaming board's own bounds check in
`cell_state(x, y)` returns `Dead` for any `(x, y)` outside
`[0, width) × [0, height)` directly — never via the chunk's
`InMemoryBoard`. This is the critical invariant that prevents the
chunk's artificial edges from being confused with the real board edges.

Width-first chunk dimensioning prefers the **row-band fast path** when
the budget allows a chunk spanning the full board width
(`owned_cols == width`); horizontal sliding is then unnecessary. When
the budget can't fit a full-width row-band, the streaming board falls
back to the general 2D path with horizontal sliding inside each
row-band.

### Cell update is unchanged

`StreamingBoard` implements `BoardView` + `BoardEditor`. Update logic
runs via the new `CellRule` trait + `BoardEditor::advance_with_rule`:

- `CellRule::next_state(currently_alive: bool, live_neighbors: usize) -> bool`
  is a pure decision function. Rules never see or return transitional
  states.
- `InPlaceTransitionalUpdater` is a `CellRule` impl encoding Conway
  B3/S23. The same rule drives both backends.
- The board owns iteration. `InMemoryBoard` runs the existing two-pass
  mark+normalize logic in place; `StreamingBoard` does the same logic
  chunk-by-chunk.
- The shared `CellState::is_originally_alive` helper is the **single
  source of truth** for "did this neighbor cell start the generation
  alive?" during mark passes (Alive | Dying = originally live). Both
  backends call it; rules never need to know about it.

The trait surface stays object-safe (`advance_with_rule(&dyn CellRule)`,
not generic) so existing `&mut dyn BoardEditor` consumers keep working.

### Scratch file format

The backing store is a private binary file with magic
`GOL-SCRATCH v1`:

```
Header (64 bytes):
  bytes  0..16: magic                = "GOL-SCRATCH v1\n\0"
  bytes 16..20: schema_version       = u32 LE
  bytes 20..28: created_at_unix_secs = u64 LE
  bytes 28..36: width                = u64 LE
  bytes 36..44: height               = u64 LE
  bytes 44..52: row_bytes            = u64 LE (= ceil(width * 2 / 8))
  bytes 52..64: reserved             = 12 × 0

Row payload (height rows × row_bytes each, fixed stride):
  2 bits per cell, packed little-endian within each byte.
  4 cells per byte. Codes: 00=Dead, 01=Alive, 10=Dying, 11=Resurrecting.
```

Random-access reads/writes for cells `[cstart, cend)` in row `y`:

- Row file offset: `64 + y * row_bytes`.
- Byte range within row: `[cstart / 4, ceil(cend / 4))`.
- **Writes** of unaligned ranges use **read-modify-write** for the two
  boundary bytes so bits belonging to neighboring cells outside the
  range are preserved. Interior bytes are written wholesale.

The format is internal-only. User-facing `.gol-snapshot` files stay
exactly as before: ASCII, hand-editable, hash-free, two-state.

### File lifecycle

- **Scratch filename**: `gol-scratch-<run_id>-<random_suffix>.bin` so
  concurrent runs in the same `--working-dir` never collide.
- **Default location**: `TMPDIR` / `%TEMP%`. Override via
  `--working-dir <PATH>`.
- **Auto-delete on success**: yes.
- **Preserve on failure**: yes. Panics, Ctrl-C, SIGKILL, or any I/O
  error leaves the scratch file on disk for inspection. We don't
  attempt panic-safe cleanup because SIGKILL can't run destructors
  anyway, so guaranteed cleanup is impossible; we prefer the simpler
  "intentional debug artifact" semantics.

### Memory cap scope

`--max-board-memory` governs the streaming board's **chunk RAM**, using
`size_of::<CellState>()` per cell (the in-memory cost). The 2-bit disk
packing is irrelevant to RAM accounting — only the scratch file's size.

The streaming floor is the cost of a minimum-size 3×3 loaded chunk plus
the per-row dirty-bit overhead: ~10–12 bytes with the current
single-byte `CellState`. Below that we surface a clear suggested-override
error matching the existing in-memory budget UX.

### What's deferred

- **Row-band fusion**: the row-band fast path currently runs mark +
  normalize as two separate chunked iterations. Fusing them — after
  marking row `y`, normalize row `y-1` (whose last neighbor consumer
  was `y`'s mark pass), with a final-row drain — would halve I/O. The
  invariant is documented in the code so this is straightforward to
  add when profiling justifies it.
- **Snapshot streaming reader**: `--load-board` keeps the in-memory
  path; loading a snapshot larger than the cap still fails with the
  existing error. The streaming snapshot **writer** is implemented and
  used by `--save-board` so streaming runs can persist their final
  state.
- **Run-record external board references**: in streaming mode, a
  requested `--save-run` produces a warning and skips. Embedding
  external references (`EXTERNAL_INITIAL_BOARD` / `EXTERNAL_FINAL_BOARD`)
  alongside the run record requires non-trivial persistence refactors;
  ships in a follow-up.
- **Snapshot streaming reader** + **run-record external boards**
  together would close the loop on streaming-sized loading and saving.
- **Full 2D fusion**, **time-skewing**, **trapezoidal tiling**, and
  other temporal-blocking optimizations are real techniques for
  out-of-core stencil computations but well above the bar of an
  interview prototype.

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
5. **Pattern Analysis**: Extend the current exact in-memory cycle detector with named oscillators, spaceships, and long-transient classifiers
6. **Visualization and Replay**: Provide views that explain board evolution better than console board dumps
7. **Interactive Mode**: Step through generations interactively
8. **Performance and Storage**: Profile and optimize for large boards or many independent runs. Consider bit-packing, sparse representations, chunked file-backed storage, and streaming reads/writes when justified.
9. **Telemetry**: Expose operations, timing, and memory-relevant dimensions for educator-facing algorithm comparisons
10. **Patterns Library**: Pre-built patterns (gliders, oscillators, spaceships)

## References

- [Conway's Game of Life - Wikipedia](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life)
- [Patterns and behaviors](https://www.conwaylife.com/)

## Architecture Diagrams

Four Excalidraw diagrams document the system, all in `docs/`:

- [`architecture.excalidraw`](architecture.excalidraw) — high-level overview
  (algorithm pattern, board storage, runtime wiring). Also exported as
  `architecture.png` and linked from the repository README.
- [`streaming-architecture.excalidraw`](streaming-architecture.excalidraw)
  — the StreamingBoard, owned-vs-loaded chunk rectangles, GOL-SCRATCH
  file layout, and lifecycle.
- [`persistence-architecture.excalidraw`](persistence-architecture.excalidraw)
  — board snapshot vs run record file kinds, the shared read pipeline
  (sniff → size guard → parser → content_hash check), and CLI verbs.
- [`board-memory-architecture.excalidraw`](board-memory-architecture.excalidraw)
  — `--max-board-memory` enforcement, the three allocation outcomes,
  and `--initial-board` source routing into the four built-in seeders.

Edit these in the Microsoft internal Excalidraw instance
(https://aka.ms/excalidraw). The streaming, persistence, and
board-memory diagrams are referenced inline from this design document.
