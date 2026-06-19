# Desktop visualizer — design notes

This document mirrors [`docs/design.md`](design.md) for the Tauri v2 desktop
app under [`desktop/`](../desktop/). Read [`design.md`](design.md) first for
the core algorithm; this file only covers what's specific to the visualizer.

## Rationale

The CLI is optimized for batch runs and replay correctness. A user who
wants to **watch** the simulation needs a different shape of tool —
animated transitions, click-to-paint setup, playback controls, live stats,
and pause/jump affordances.

Rather than fork the core, the visualizer wraps it through a narrow IPC
surface. This keeps the core's zero-dependency posture intact while letting
the desktop crate pull in the React + Tauri toolchain it needs.

## Architecture

```
+---------------------------+        +-------------------------------+
| desktop/ui (React + TS)   |        | desktop/src (Rust + Tauri 2) |
|                           | invoke |                               |
|   App / Shell layout      | -----> |   commands/{setup,run,...}.rs |
|   BoardCanvas (Canvas 2D) |        |   session.rs (RunSession)     |
|   PlaybackControls        | <----- |   ipc_types.rs (serde wires)  |
|   StatsPanel (Recharts)   | events |   events.rs (event names)     |
|   Zustand store           |        +-------------------------------+
+---------------------------+                       |
                                                    | path = ".."
                                                    v
                                          +------------------+
                                          | game-of-life     |
                                          | (root library)   |
                                          +------------------+
```

## Workspace separation

`desktop/Cargo.toml` declares its own `[workspace]` so cargo's upward search
stops there. Consequences:

- Root `Cargo.toml`, root `Cargo.lock`, `src/`, `tests/`, and
  `.github/workflows/ci.yml` are bit-identical to `main`.
- The desktop crate path-depends on the root crate (`game-of-life = { path = ".." }`)
  so it always gets the same library bits the CLI does.
- Pulling Tauri's ~300 transitive crates only churns
  `desktop/Cargo.lock`. The root lockfile stays empty.

## State machine

```
[Setup] ─start_run→ [Running:Paused] ⇄ [Running:Playing]
              ▲                │
              │                ├── restart      → [Running:Paused @ iter 0]
              │                ├── jump_to(n)   → [Running:JumpingTo n] → [Running:Paused @ n]
              └── edit_board ──┘
```

- **Setup** is the only mode that allows cell edits, pattern apply, randomize,
  clear, and board-size change.
- **Running:*** locks the canvas. The cursor switches to `not-allowed` and
  pointer events are no-ops.
- **JumpingTo** is a transient state used while a background worker is
  fast-forwarding (or replaying-from-initial when going backward).

## IPC surface (Rust → frontend)

Commands live in [`desktop/src/commands/`](../desktop/src/commands/) and are
listed in [`desktop/src/lib.rs`](../desktop/src/lib.rs)'s `invoke_handler`.
Names match the Tauri command names, which match the frontend wrapper
functions in [`desktop/ui/src/ipc/commands.ts`](../desktop/ui/src/ipc/commands.ts).

| Category | Commands |
|---|---|
| Read-only | `get_session`, `get_board`, `get_alive_history`, `get_final_stats`, `default_save_dir` |
| Setup | `create_run`, `set_cell`, `paint_cells`, `apply_pattern`, `randomize`, `clear_board` |
| Run | `start_run`, `restart`, `edit_board`, `extend_max_iterations`, `step`, `play`, `pause`, `jump_to` |

Events emitted from the Rust side and listened-to in the frontend's
[`desktop/ui/src/ipc/events.ts`](../desktop/ui/src/ipc/events.ts):

| Event | Payload | When |
|---|---|---|
| `gol://board-tick` | `{ stats: AdvanceTick, board: BoardPayload }` | After every generation during `play` and on `step` |
| `gol://jump-progress` | `{ current, target }` | Every ~100ms during a `jump_to` |
| `gol://run-completed` | `{ iteration, status, stats }` | When the simulation reaches `max_iterations` or extinction |
| `gol://session-changed` | `SessionInfo` | When mode transitions (Paused ↔ Playing, etc.) |

## Concurrency model

`RunSession` holds a `parking_lot::Mutex<SessionData>` with **short**
critical sections (a single `advance_generation` call) plus an
`AtomicBool` cancel flag. The `play` and `jump_to` commands spawn tokio
tasks that loop:

1. `if session.cancel_requested() { break }`
2. `let tick = session.advance_one()?`
3. emit `board-tick` event
4. `tokio::time::sleep(period)` (play) or `tokio::task::yield_now()` (jump)

This is the fix for the critic's CRITICAL "long advance(N) freezes the UI"
finding — Pause latency is bounded by one generation, not by the whole
batch.

## Animation

Transitional cell states (`Dying`, `Resurrecting`) exist only inside the
core updater's mark phase and are never returned across IPC. The fade
animation is therefore a **desktop-side diff**: BoardCanvas captures the
pre-state and post-state grids on each `board-tick` and interpolates
opacity over `~8` frames via `requestAnimationFrame`. The full fade
implementation lands with the `animation` follow-up; the scaffold ships
with hard cuts.

In High Contrast theme, fades are disabled entirely and cells use border
styling in addition to fill — color-only signalling fails screen readers
and high-contrast settings.

## Save semantics

| Action | When enabled | What it writes |
|---|---|---|
| Save board snapshot | Any mode, any iteration | A `GOL-BOARD-SNAPSHOT v1` `.gol` with just the current grid |
| Save run | Only after the run completes (extinct or hit max-iterations) | A full `GOL-RUN-RECORD v1` `.gol` matching the CLI's format |

The split exists because `RunRecord` requires a terminal `RunStatus`
(`max_iterations` or `extinct`). Mid-run saves would have to invent a
status, which would prevent CLI replay round-tripping. The visualizer
sidesteps this by writing board snapshots for in-flight captures and
full run records only once the simulation has terminated.

## Tests

Desktop Rust integration tests live under `desktop/tests/` and use the same
`_tests.rs` suffix as the root Rust crate:

| Test file | Covers |
|---|---|
| `desktop/tests/ipc_types_tests.rs` | IPC wire-format conversions and payload helpers |
| `desktop/tests/run_commands_tests.rs` | Pure run-command helper behavior |
| `desktop/tests/session_tests.rs` | `RunSession` state-machine behavior |

Desktop UI tests keep the UI-native Vitest convention and stay near source
files as `.test.ts` or `.test.tsx`, such as
`desktop/ui/src/App.test.tsx` and `desktop/ui/src/ipc/types.test.ts`.

## Release artifacts

See the **Releases** section of the [top-level README](../README.md) for
the artifact matrix, the SmartScreen warning, the AppImage glibc floor,
and the `.gol` round-trip caveats.
