//! Run record file IO. Stub — see `docs/design.md` for the format spec; full
//! implementation lands in a follow-up commit in this PR.

#![allow(dead_code, unused_variables, unused_imports)]

use std::fmt;
use std::path::PathBuf;

use crate::board::InMemoryBoard;

use super::errors::PersistenceIoError;

pub const INITIAL_BOARD_LABEL: &str = "INITIAL BOARD";
pub const FINAL_BOARD_LABEL: &str = "FINAL BOARD";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentHashMode {
    Enforce,
    Ignore,
}

#[derive(Debug, Clone)]
pub struct RunRecord {
    pub run_id: super::run_id::RunId,
    pub config: RunRecordConfig,
    pub result: RunRecordResult,
    pub initial_board: InMemoryBoard,
    pub final_board: InMemoryBoard,
}

#[derive(Debug, Clone)]
pub struct RunRecordConfig {
    pub board_size: (usize, usize),
    pub max_iterations: usize,
    pub max_board_memory_bytes: usize,
    pub initial_board_source: String,
    pub random_seed: u64,
    pub updater: String,
    pub continued_from: Option<super::run_id::RunId>,
}

#[derive(Debug, Clone)]
pub struct RunRecordResult {
    pub status: String,
    pub iterations_run: usize,
    pub wall_time_ms: u64,
    pub initial_alive_count: usize,
    pub final_alive_count: usize,
    pub peak_alive_count: usize,
    pub peak_alive_generation: usize,
    pub min_alive_count: usize,
    pub min_alive_generation: usize,
    pub total_births: u64,
    pub total_deaths: u64,
    pub initial_board_hash: u64,
    pub final_board_hash: u64,
}

#[derive(Debug)]
pub enum RunRecordWriteError {
    Io(PersistenceIoError),
}

impl fmt::Display for RunRecordWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunRecordWriteError::Io(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for RunRecordWriteError {}

#[derive(Debug)]
pub enum RunRecordReadError {
    Io(PersistenceIoError),
}

impl fmt::Display for RunRecordReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunRecordReadError::Io(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for RunRecordReadError {}

pub fn write_run_record(_path: &std::path::Path, _record: &RunRecord) -> Result<(), RunRecordWriteError> {
    unimplemented!("run record writer lands in a follow-up commit in this PR")
}

pub fn read_run_record(
    _path: &std::path::Path,
    _max_board_memory_bytes: usize,
    _max_input_file_bytes: usize,
    _content_hash_mode: ContentHashMode,
) -> Result<RunRecord, RunRecordReadError> {
    unimplemented!("run record reader lands in a follow-up commit in this PR")
}

pub fn extract_board_from_run(
    _path: &std::path::Path,
    _which: ExtractWhich,
    _output_path: &std::path::Path,
    _max_board_memory_bytes: usize,
    _max_input_file_bytes: usize,
    _content_hash_mode: ContentHashMode,
) -> Result<(), RunRecordReadError> {
    unimplemented!("extract-board lands in a follow-up commit in this PR")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractWhich {
    Initial,
    Final,
}
