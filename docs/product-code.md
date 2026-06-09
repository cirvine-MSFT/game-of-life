# Product code maintenance

Keep product code module-centric and small enough that ownership is obvious.

## Module ownership

| Module | Owns | Should not own |
|--------|------|----------------|
| `src/board.rs` | `Board`, `CellState`, display formatting, and generation advancement. | CLI parsing, file loading, or simulation orchestration. |
| `src/config.rs` | `SimulationConfig`, `BoardSize`, CLI/config parsing helpers, and typed configuration errors. | Board mutation or console output. |
| `src/lib.rs` | Public module declarations and re-exports. | Product logic. |
| `src/main.rs` | Process entry point, help text, stderr/stdout behavior, exit codes, and wiring config into a run. | Board rules or reusable parser logic. |

## Change guidance

- Preserve the public API re-exports unless there is an explicit breaking-change decision.
- Keep reusable logic in the library, not in `main.rs`.
- Add a new source module when a new concern grows beyond a few focused functions.
- Update `docs/testing.md` and matching tests whenever a public module or behavior changes.
- Keep the project dependency-free unless a feature explicitly justifies a dependency.
