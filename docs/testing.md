# Test maintenance

Tests live outside product code and should make coverage ownership obvious. Rust tests use separate Cargo integration-test directories that roughly mirror the product-code modules or behavior they cover.

## File naming

Rust integration tests use the `_tests.rs` suffix so test files are distinguishable from product files by filename alone. Test filenames should identify both the product module or behavior under test and that the file is a test. Top-level Rust files under `tests/` and `desktop/tests/` are Cargo integration-test binaries. Grouped root test modules keep a suffixed wrapper and suffixed child files.

Desktop UI tests keep the TypeScript/Vitest convention and stay near UI source files as `.test.ts` or `.test.tsx`.

## Test files

| Test file | Covers |
|-----------|--------|
| `tests/algorithms_tests.rs` | Algorithm initializers, updater behavior, stabilization, and seeded board patterns. |
| `tests/board_tests.rs` | Public board API, bounded-board behavior, display behavior, and generation rules. |
| `tests/cli_tests.rs` | End-to-end behavior of the compiled `game-of-life` binary for core run flows. |
| `tests/config_tests.rs` | `SimulationConfig`, `BoardSize`, parser behavior, defaults, and typed errors. |
| `tests/pattern_analysis_tests.rs` | Pattern analyzer cycle and terminal-state detection. |
| `tests/persistence_tests.rs` | Wrapper entry point for persistence integration tests. |
| `tests/persistence/*_tests.rs` | Persistence hash, parser, run ID, run record, board snapshot, scratch path, timestamp, and persistence CLI flows. |
| `tests/stats_tests.rs` | Wrapper entry point for statistics integration tests. |
| `tests/stats/run_statistics_tests.rs` | `RunStatistics` aggregation and status behavior. |
| `tests/streaming_tests.rs` | Wrapper entry point for streaming board integration tests. |
| `tests/streaming/streaming_board_tests.rs` | File-backed streaming board behavior. |
| `desktop/tests/ipc_types_tests.rs` | Desktop IPC wire-format conversions and payload helpers. |
| `desktop/tests/run_commands_tests.rs` | Desktop run-command helper behavior. |
| `desktop/tests/session_tests.rs` | Desktop `RunSession` state-machine behavior. |
| `desktop/ui/src/**/*.test.ts(x)` | Desktop UI behavior through Vitest and Testing Library. |

If shared helpers are needed, put them under `tests/common/mod.rs` rather than `tests/common.rs` so Cargo does not compile the helper as a standalone integration test.

## Wrapper pattern

Cargo's integration-test discovery only picks up top-level `tests/*.rs` files as test binaries. Files inside `tests/<module>/` need a suffixed wrapper such as `tests/persistence_tests.rs` that declares each child with an explicit path:

```rust
#[path = "persistence/hash_tests.rs"]
mod hash_tests;
```

Use the same suffix in both the child filename and module name.

## Test labels

- Use `edge_case_` prefixes for valid boundary behavior, such as `1x1` boards, corner cells, out-of-bounds-safe board operations, and zero iterations.
- Use `negative_` prefixes for invalid inputs, rejected operations, non-zero exits, and actionable error messages.
- Group labeled tests in `edge_case_tests` and `negative_tests` modules where practical.
- Keep normal behavior tests separate from edge and negative tests when that improves scanability.

## Coverage expectations

- Every public Rust module should have a matching `_tests.rs` integration test file or grouped child file.
- New CLI options need parser tests in `config_tests.rs` and binary tests in `cli_tests.rs`.
- New desktop IPC or state-machine behavior needs matching tests under `desktop/tests/*_tests.rs`; UI behavior needs `.test.ts` / `.test.tsx`.
- New validation errors should have tests for both the typed error and the user-facing message.
- New board behavior should use readable ASCII grids with `#` for alive and `.` for dead.
