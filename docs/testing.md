# Test maintenance

Tests live outside product code under `tests/` and should make coverage ownership obvious.

## Test files

| Test file | Covers |
|-----------|--------|
| `tests/board_tests.rs` | Public board API, bounded-board behavior, display behavior, and generation rules from `src/board.rs`. |
| `tests/config_tests.rs` | `SimulationConfig`, `BoardSize`, parser behavior, defaults, and typed errors from `src/config.rs`. |
| `tests/cli_tests.rs` | End-to-end behavior of the compiled `game-of-life` binary. |

If shared helpers are needed, put them under `tests/common/mod.rs` rather than `tests/common.rs` so Cargo does not compile the helper as a standalone integration test.

## Test labels

- Use `edge_case_` prefixes for valid boundary behavior, such as `1x1` boards, corner cells, out-of-bounds-safe board operations, and zero iterations.
- Use `negative_` prefixes for invalid inputs, rejected operations, non-zero exits, and actionable error messages.
- Group labeled tests in `edge_case_tests` and `negative_tests` modules where practical.
- Keep normal behavior tests separate from edge and negative tests when that improves scanability.

## Coverage expectations

- Every public module should have a matching integration test file.
- New CLI options need parser tests in `config_tests.rs` and binary tests in `cli_tests.rs`.
- New validation errors should have tests for both the typed error and the user-facing message.
- New board behavior should use readable ASCII grids with `#` for alive and `.` for dead.
