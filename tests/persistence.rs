//! Wrapper entry point for the `persistence` integration test group.
//!
//! Cargo treats each `tests/*.rs` file as an independent integration test
//! crate. To group submodule tests under a directory that mirrors
//! `src/persistence/`, we explicitly point each `mod` at its file with
//! `#[path]`. Without that, Rust would look for `tests/<name>.rs` siblings
//! of this file instead of `tests/persistence/<name>.rs` children.

#[path = "persistence/board_snapshot.rs"]
mod board_snapshot;
#[path = "persistence/cli.rs"]
mod cli;
#[path = "persistence/hash.rs"]
mod hash;
#[path = "persistence/magic.rs"]
mod magic;
#[path = "persistence/parser.rs"]
mod parser;
#[path = "persistence/run_id.rs"]
mod run_id;
#[path = "persistence/run_record.rs"]
mod run_record;
#[path = "persistence/scratch.rs"]
mod scratch;
#[path = "persistence/timestamps.rs"]
mod timestamps;
