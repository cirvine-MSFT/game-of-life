//! Wrapper entry point for the `streaming` integration test group.
//!
//! Cargo treats each `tests/*.rs` file as an independent integration
//! test crate. To group submodule tests under a directory that mirrors
//! `src/board/streaming_board.rs`, we explicitly point each `mod` at
//! its file with `#[path]`. See AGENTS.md > Test Style for the same
//! pattern used elsewhere.

#[path = "streaming/streaming_board_tests.rs"]
mod streaming_board_tests;
