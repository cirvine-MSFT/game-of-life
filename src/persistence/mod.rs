//! Persistence layer for Game of Life runs and board states.
//!
//! Two file types share one parser pair:
//!
//! - **Run record** (`GOL-RUN-RECORD v1`): the full file written at the end of
//!   a run, capturing the configuration, run statistics, the initial board, and
//!   the final board. Protected by a `content_hash` trailer.
//! - **Board snapshot** (`GOL-BOARD-SNAPSHOT v1`): a standalone, hash-free,
//!   freely-editable file containing just one board block. Users can craft
//!   these by hand or extract them from a run record via the `--extract-board`
//!   command.
//!
//! See `docs/design.md` for the format spec.

pub mod board_snapshot;
pub mod errors;
pub mod hash;
pub mod magic;
pub mod parser;
pub mod run_id;
pub mod run_record;
pub mod timestamps;

pub use board_snapshot::{
    read_board_snapshot, read_board_snapshot_default, suggest_memory_override,
    validate_loaded_board_size, write_board_snapshot, write_board_snapshot_to, BoardSnapshot,
    BoardSnapshotReadError, BoardSnapshotWriteError, LoadedBoardSizeError, SuggestedMemoryDisplay,
    SuggestedMemoryOverride, BOARD_BLOCK_LABEL, SUGGESTED_MEMORY_OVERRIDE_FLOOR_BYTES,
};
pub use errors::PersistenceIoError;
pub use hash::{fnv1a_64, format_hash, parse_hash, HashParseError};
pub use magic::{
    sniff_file_kind, sniff_from_reader, FileKind, MagicError, BOARD_SNAPSHOT_MAGIC,
    RUN_RECORD_MAGIC, SCHEMA_VERSION,
};
pub use parser::{ParseError, ParseLocation};
pub use run_id::{format_run_id, parse_run_id, short_run_id, RunId, RunIdParseError};
pub use run_record::{
    board_grid_hash, extract_board_from_run, read_run_record, read_run_record_default,
    read_run_record_with_warnings, write_run_record, ContentHashMode, ExtractBoardError,
    ExtractWhich, LoadedRunRecord, RunRecord, RunRecordConfig, RunRecordReadError, RunRecordResult,
    RunRecordWriteError, FINAL_BOARD_LABEL, INITIAL_BOARD_LABEL, RECOGNIZED_STATUSES, TOOL_VERSION,
};
pub use timestamps::{format_utc, parse_utc, TimestampParseError};

/// Maximum number of bytes the magic-sniff routine will read from a file
/// before deciding whether it recognizes the file type. Bounded to keep the
/// sniff cheap and prevent a DoS on huge files.
pub const MAX_MAGIC_PEEK_BYTES: usize = 128;

/// Default ceiling on the size of input files we'll read into memory.
/// Configurable per-invocation via `--max-input-file-bytes`.
pub const DEFAULT_MAX_INPUT_FILE_BYTES: usize = 256 * 1024 * 1024;
