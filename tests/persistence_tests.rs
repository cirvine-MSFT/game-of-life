//! Wrapper entry point for the `persistence` integration test group.
//!
//! Cargo treats each `tests/*.rs` file as an independent integration test
//! crate. To group submodule tests under a directory that mirrors
//! `src/persistence/`, we explicitly point each `mod` at its file with
//! `#[path]`. Without that, Rust would look for `tests/<name>.rs` siblings
//! of this file instead of `tests/persistence/<name>.rs` children.

#[path = "persistence/board_snapshot_tests.rs"]
mod board_snapshot_tests;
#[path = "persistence/cli_tests.rs"]
mod cli_tests;
#[path = "persistence/hash_tests.rs"]
mod hash_tests;
#[path = "persistence/magic_tests.rs"]
mod magic_tests;
#[path = "persistence/parser_tests.rs"]
mod parser_tests;
#[path = "persistence/run_id_tests.rs"]
mod run_id_tests;
#[path = "persistence/run_record_tests.rs"]
mod run_record_tests;
#[path = "persistence/scratch_tests.rs"]
mod scratch_tests;
#[path = "persistence/timestamps_tests.rs"]
mod timestamps_tests;
