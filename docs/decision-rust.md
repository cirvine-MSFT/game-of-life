# Decision: choose Rust for the Game of Life project

## Status

Accepted for the next implementation phase.

## Context

This project started with two comparable Conway's Game of Life spikes: one in Rust and one in C++. Both prototypes implemented the same bounded-board rules, single-board transitional cell states, reusable core logic, console application, tests, CI, agent instructions, and design artifacts.

The goal was not only to pick a language for the first implementation, but also to evaluate which ecosystem is easier for Casey and AI agents to maintain as the project grows.

Future project directions are expected to include:

- OpenTelemetry-based runtime and application telemetry.
- Developer-facing health and performance metrics, potentially visualized in Grafana.
- File-system integration for loading, streaming, or persisting boards.
- Non-console UIs, possibly including a web UI, desktop shell, or service-backed frontend.
- Web services and WebSocket APIs for streaming board state to clients.
- A learning path for Casey to get more familiar with modern Rust through real implementation work.

## Decision

Use Rust as the primary implementation language for the project going forward.

Keep the current Rust prototype as the baseline branch for future work. Preserve the architecture where the Game of Life rules live in a reusable core library and the console application is only one frontend over that core.

## Why Rust

### Lower build and dependency friction

Cargo gave the Rust prototype a straightforward build, test, lint, format, and release workflow:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo build --release`

The C++ prototype was workable, but required more setup and hardening around CMake, GoogleTest, CTest, compiler warning scopes, multi-config generators, and Windows-specific CI behavior. That complexity is useful to practice, but it is not the lowest-friction path for building this project.

### Strong agent maintainability signal

Rust's standard tooling gives agents a consistent set of commands and compiler feedback. The compiler and Clippy tend to produce actionable diagnostics, and the package/build/test model is more uniform than C++.

The spike showed that the agent could still make a design mistake in Rust, but it was easy to detect, correct, validate, and document. The C++ branch required more toolchain-specific review to catch CI and build-system edge cases.

### Good fit for learning goals

Rust supports Casey's goal of getting refamiliar with the language through a real project rather than isolated exercises. The project has enough depth to exercise practical Rust topics over time:

- ownership and borrowing around board state,
- enums and explicit state modeling,
- error handling,
- package and module organization,
- async services,
- telemetry instrumentation,
- file and stream processing,
- cross-platform distribution.

### Strong path for telemetry

Rust has good support for OpenTelemetry and adjacent observability patterns. A likely direction is:

- use `tracing` for structured application events and spans,
- use `opentelemetry` and `opentelemetry-otlp` to export traces and metrics,
- run an OpenTelemetry Collector beside the application,
- forward metrics/traces to a backend that Grafana can visualize, such as Prometheus, Tempo, or another OTLP-compatible service.

The initial console app does not need telemetry yet, but Rust can support the planned architecture without changing the core Game of Life model.

### Good fit for web services and streaming

Rust has mature async and web-service options for future frontends:

- `tokio` for async runtime support,
- `axum`, `actix-web`, or `warp` for HTTP APIs,
- WebSocket support through those frameworks or lower-level crates such as `tokio-tungstenite`,
- `serde` for JSON or structured board serialization.

That gives the project a clear path to expose board state over HTTP and stream generations to browser or desktop clients.

### Flexible UI options

Choosing Rust does not lock the project into a console UI. The core library can remain UI-agnostic while new frontends are added around it:

- keep the current console app for deterministic smoke tests and CLI workflows,
- add a Rust web service that streams board updates to a browser UI,
- add a Tauri desktop app with a Rust backend and web frontend,
- experiment with Rust-native UI frameworks such as `egui` if a native UI becomes interesting.

This keeps the implementation flexible while preserving a single source of truth for Game of Life rules.

### Native and cross-platform trajectory

Rust is increasingly common for native, cross-platform application foundations where memory safety, performance, and packaging matter. The GitHub Copilot app's Rust/Tauri direction is a useful reference point for the kind of native-plus-web hybrid this project may eventually explore.

## Consequences

### Positive

- Simpler default build and test workflow.
- Easier local validation in the current environment.
- Strong compiler and lint feedback for both humans and agents.
- Good ecosystem support for telemetry, async services, WebSockets, file I/O, and cross-platform applications.
- Supports Casey's learning goal directly.
- Keeps frontend options open by separating core logic from UI.

### Tradeoffs

- Casey is currently more familiar with C++, so Rust may slow some early work while language patterns become familiar again.
- Rust async and lifetime patterns can add complexity once services and streaming are introduced.
- Some UI choices may still be better implemented with web technologies, using Rust as the backend/service layer.
- The Rust ecosystem is strong, but crate selection matters; telemetry and web stacks should be introduced deliberately rather than all at once.

## Near-term architecture guidance

Keep the project layered:

```text
game_of_life core library
  owns board state, rules, transitions, serialization-ready model

console frontend
  deterministic demo and CI smoke testing

future service layer
  HTTP/WebSocket APIs, telemetry, runtime controls

future UI layer
  browser UI, Tauri app, or native Rust UI consuming the service/core
```

The next implementation phase should keep the core dependency-light, then add telemetry and service dependencies at the boundary layers.

## Telemetry direction

When telemetry is added, prefer an incremental approach:

1. Add `tracing` spans/events around generation advancement, board loading, and console/service commands.
2. Add counters and histograms for generation count, board size, live-cell count, step duration, and memory-relevant dimensions.
3. Export via OpenTelemetry OTLP to an OpenTelemetry Collector.
4. Add a local container-based observability stack only after the application emits useful telemetry.
5. Document the local Grafana/collector setup in `docs/`.

## UI direction

Do not replace the console app immediately. Keep it as the simplest executable and CI smoke target.

For richer UI work, prefer one of these paths:

- **Web-first**: Rust service with HTTP/WebSocket streaming, browser frontend consumes board updates.
- **Desktop hybrid**: Tauri app using Rust for backend commands and a web frontend for rendering.
- **Native Rust UI**: evaluate only if the project specifically wants a Rust-native rendering experiment.

## Decision review triggers

Revisit this decision if:

- Rust telemetry or UI crates become a major blocker.
- C++ becomes necessary for integration with a required native library.
- Performance requirements exceed what the straightforward Rust model can achieve after profiling.
- The project shifts from learning/exploration to a domain where another ecosystem is clearly better.

For the current goals, Rust is the recommended choice.
