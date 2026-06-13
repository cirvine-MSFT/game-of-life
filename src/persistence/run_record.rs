//! Run record file IO with `content_hash` integrity protection.
//!
//! Run records have the shape:
//!
//! ```text
//! GOL-RUN-RECORD v1
//! run_id: 7b3a1f0c-4d2e-4a51-9c5e-2f8c3a1b9d77
//! schema_version: 1
//! created_at: 2026-06-12T22:55:20Z
//! tool_version: 0.1.0
//!
//! [config]
//! board_size: 10x10
//! max_iterations: 100
//! max_board_memory_bytes: 67108864
//! initial_board_source: random
//! random_seed: 14872319874213
//! updater: in_place_transitional
//! continued_from:
//!
//! [result]
//! status: extinct
//! iterations_run: 47
//! wall_time_ms: 3
//! initial_alive_count: 48
//! final_alive_count: 0
//! peak_alive_count: 53
//! peak_alive_generation: 2
//! min_alive_count: 0
//! min_alive_generation: 47
//! total_births: 312
//! total_deaths: 360
//! initial_board_hash: 0x9f2b1c4e7a5d3088
//! final_board_hash: 0x0000000000000000
//!
//! ----- BEGIN INITIAL BOARD -----
//! ... (board grid + headers)
//! ----- END INITIAL BOARD -----
//!
//! ----- BEGIN FINAL BOARD -----
//! ... (board grid + headers)
//! ----- END FINAL BOARD -----
//!
//! content_hash: 0x3f8a2c1d9e4b6075
//! ```
//!
//! The `content_hash` trailer protects the file from accidental edits,
//! truncated writes, and bit flips. It is the FNV-1a 64-bit hash of the
//! canonical UTF-8 bytes of everything in the file from the magic line up to
//! (and including) the newline preceding `content_hash:`. The reader
//! normalizes the file on read (LF endings, trimmed trailing whitespace) so
//! a Windows editor saving the file in CRLF does not break verification.

use std::fmt;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::board::InMemoryBoard;

use super::board_snapshot::{
    read_board_block, slurp_with_size_guard, write_board_block_to, BoardSnapshot,
    BoardSnapshotReadError, BoardSnapshotWriteError, LineCursor, LoadedBoardSizeError,
};
use super::errors::PersistenceIoError;
use super::hash::{fnv1a_64, format_hash, parse_hash};
use super::magic::{sniff_from_reader, FileKind, MagicError, RUN_RECORD_MAGIC, SCHEMA_VERSION};
use super::parser::{
    parse_begin_fence, parse_field_line, parse_section_header, strip_trailing_cr, ParseError,
    ParseLocation,
};
use super::run_id::{format_run_id, parse_run_id, RunId};
use super::timestamps::{format_utc, parse_utc, TimestampParseError};
use super::DEFAULT_MAX_INPUT_FILE_BYTES;

pub const INITIAL_BOARD_LABEL: &str = "INITIAL BOARD";
pub const FINAL_BOARD_LABEL: &str = "FINAL BOARD";

const CONTENT_HASH_FIELD: &str = "content_hash";

/// The tool version string written into every run record. Read from
/// `CARGO_PKG_VERSION` at compile time.
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Controls how the reader handles a `content_hash` mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentHashMode {
    Enforce,
    Ignore,
}

/// Selects which embedded board block to read when extracting from a run
/// record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractWhich {
    Initial,
    Final,
}

impl ExtractWhich {
    pub fn label(self) -> &'static str {
        match self {
            ExtractWhich::Initial => INITIAL_BOARD_LABEL,
            ExtractWhich::Final => FINAL_BOARD_LABEL,
        }
    }
}

impl std::str::FromStr for ExtractWhich {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "initial" => Ok(ExtractWhich::Initial),
            "final" => Ok(ExtractWhich::Final),
            other => Err(format!(
                "expected 'initial' or 'final' for --load-from, got '{other}'"
            )),
        }
    }
}

/// Full in-memory representation of a run record file.
#[derive(Debug, Clone)]
pub struct RunRecord {
    pub run_id: RunId,
    pub schema_version: u32,
    pub created_at: SystemTime,
    pub tool_version: String,
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
    pub continued_from: Option<RunId>,
}

#[derive(Debug, Clone)]
pub struct RunRecordResult {
    pub status: String,
    pub iterations_run: u64,
    pub wall_time_ms: u64,
    pub initial_alive_count: u64,
    pub final_alive_count: u64,
    pub peak_alive_count: u64,
    pub peak_alive_generation: u64,
    pub min_alive_count: u64,
    pub min_alive_generation: u64,
    pub total_births: u64,
    pub total_deaths: u64,
    pub initial_board_hash: u64,
    pub final_board_hash: u64,
}

pub const RECOGNIZED_STATUSES: &[&str] = &["extinct", "max_iterations", "stable", "cyclic"];

// -------- write errors ---------------------------------------------------

#[derive(Debug)]
pub enum RunRecordWriteError {
    Io(PersistenceIoError),
    OutputExists { path: PathBuf },
}

impl fmt::Display for RunRecordWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunRecordWriteError::Io(e) => write!(f, "{e}"),
            RunRecordWriteError::OutputExists { path } => write!(
                f,
                "Refusing to overwrite existing file '{}'.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RunRecordWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RunRecordWriteError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<BoardSnapshotWriteError> for RunRecordWriteError {
    fn from(value: BoardSnapshotWriteError) -> Self {
        match value {
            BoardSnapshotWriteError::Io(e) => RunRecordWriteError::Io(e),
            BoardSnapshotWriteError::OutputExists { path } => {
                RunRecordWriteError::OutputExists { path }
            }
        }
    }
}

// -------- read errors ----------------------------------------------------

#[derive(Debug)]
pub enum RunRecordReadError {
    Io(PersistenceIoError),
    Magic(MagicError),
    UnexpectedFileKind {
        path: PathBuf,
        expected: FileKind,
        actual: FileKind,
    },
    InvalidTimestamp(TimestampParseError),
    Parse(ParseError),
    LoadedBoardSize(LoadedBoardSizeError),
    BoardBlockTooLarge {
        block: &'static str,
        run_id: RunId,
        source: LoadedBoardSizeError,
    },
    FileTooLarge {
        path: PathBuf,
        actual_bytes: u64,
        limit_bytes: usize,
    },
    MalformedSizeHeader {
        location: ParseLocation,
        value: String,
    },
    Corrupted {
        path: PathBuf,
        expected_hash: u64,
        actual_hash: u64,
    },
    MissingContentHash {
        path: PathBuf,
    },
    UnrecognizedStatus {
        location: ParseLocation,
        value: String,
    },
    MalformedRunId {
        location: ParseLocation,
        value: String,
    },
    MalformedField {
        location: ParseLocation,
        field: String,
        value: String,
    },
    MissingField {
        section: String,
        field: String,
    },
}

impl fmt::Display for RunRecordReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunRecordReadError::Io(e) => write!(f, "{e}"),
            RunRecordReadError::Magic(e) => write!(f, "{e}"),
            RunRecordReadError::UnexpectedFileKind { path, expected, actual } => write!(
                f,
                "File '{}' is a {actual}, but expected a {expected}.",
                path.display()
            ),
            RunRecordReadError::InvalidTimestamp(e) => write!(f, "{e}"),
            RunRecordReadError::Parse(e) => write!(f, "{e}"),
            RunRecordReadError::LoadedBoardSize(e) => write!(f, "{e}"),
            RunRecordReadError::BoardBlockTooLarge {
                block,
                run_id,
                source,
            } => write!(
                f,
                "While loading the {block} board of run {}: {source}",
                format_run_id(run_id)
            ),
            RunRecordReadError::FileTooLarge {
                path,
                actual_bytes,
                limit_bytes,
            } => write!(
                f,
                "File '{}' is {actual_bytes} bytes which exceeds the {limit_bytes}-byte input file limit. Raise the limit with --max-input-file-bytes if the file is trustworthy.",
                path.display()
            ),
            RunRecordReadError::MalformedSizeHeader { location, value } => write!(
                f,
                "Malformed board 'size' header at {location}: '{value}'. Expected WIDTHxHEIGHT, for example 10x10."
            ),
            RunRecordReadError::Corrupted {
                path,
                expected_hash,
                actual_hash,
            } => write!(
                f,
                "Run record '{}' failed integrity check: expected content_hash {}, computed {}. If you intentionally edited this file, re-run with --ignore-integrity to bypass this check (the loaded data will be used as-is). To craft a new board from this run, use --extract-board <path> --load-from initial|final --output snapshot.gol instead -- board snapshots have no integrity hash and are freely editable.",
                path.display(),
                format_hash(*expected_hash),
                format_hash(*actual_hash),
            ),
            RunRecordReadError::MissingContentHash { path } => write!(
                f,
                "Run record '{}' is missing the integrity trailer 'content_hash:' line. Regenerate the file or pass --ignore-integrity to bypass this check.",
                path.display()
            ),
            RunRecordReadError::UnrecognizedStatus { location, value } => write!(
                f,
                "Unknown run status '{value}' at {location}; recognized statuses: {}.",
                RECOGNIZED_STATUSES.join(", ")
            ),
            RunRecordReadError::MalformedRunId { location, value } => write!(
                f,
                "Malformed run id '{value}' at {location}; expected xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx."
            ),
            RunRecordReadError::MalformedField {
                location,
                field,
                value,
            } => write!(
                f,
                "Malformed value '{value}' for field '{field}' at {location}."
            ),
            RunRecordReadError::MissingField { section, field } => write!(
                f,
                "Section [{section}] is missing required field '{field}'."
            ),
        }
    }
}

impl std::error::Error for RunRecordReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RunRecordReadError::Io(e) => Some(e),
            RunRecordReadError::Magic(e) => Some(e),
            RunRecordReadError::InvalidTimestamp(e) => Some(e),
            RunRecordReadError::Parse(e) => Some(e),
            RunRecordReadError::LoadedBoardSize(e) => Some(e),
            RunRecordReadError::BoardBlockTooLarge { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<PersistenceIoError> for RunRecordReadError {
    fn from(value: PersistenceIoError) -> Self {
        RunRecordReadError::Io(value)
    }
}

impl From<MagicError> for RunRecordReadError {
    fn from(value: MagicError) -> Self {
        RunRecordReadError::Magic(value)
    }
}

impl From<ParseError> for RunRecordReadError {
    fn from(value: ParseError) -> Self {
        RunRecordReadError::Parse(value)
    }
}

impl From<TimestampParseError> for RunRecordReadError {
    fn from(value: TimestampParseError) -> Self {
        RunRecordReadError::InvalidTimestamp(value)
    }
}

impl From<BoardSnapshotReadError> for RunRecordReadError {
    fn from(value: BoardSnapshotReadError) -> Self {
        match value {
            BoardSnapshotReadError::Io(e) => RunRecordReadError::Io(e),
            BoardSnapshotReadError::Magic(e) => RunRecordReadError::Magic(e),
            BoardSnapshotReadError::UnexpectedFileKind { path, expected, actual } => {
                RunRecordReadError::UnexpectedFileKind { path, expected, actual }
            }
            BoardSnapshotReadError::InvalidTimestamp(e) => RunRecordReadError::InvalidTimestamp(e),
            BoardSnapshotReadError::Parse(e) => RunRecordReadError::Parse(e),
            BoardSnapshotReadError::LoadedBoardSize(e) => RunRecordReadError::LoadedBoardSize(e),
            BoardSnapshotReadError::FileTooLarge {
                path,
                actual_bytes,
                limit_bytes,
            } => RunRecordReadError::FileTooLarge {
                path,
                actual_bytes,
                limit_bytes,
            },
            BoardSnapshotReadError::MalformedSizeHeader { location, value } => {
                RunRecordReadError::MalformedSizeHeader { location, value }
            }
        }
    }
}

// -------- writing --------------------------------------------------------

pub fn write_run_record(
    path: impl AsRef<Path>,
    record: &RunRecord,
) -> Result<(), RunRecordWriteError> {
    let path = path.as_ref();
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                RunRecordWriteError::OutputExists {
                    path: path.to_path_buf(),
                }
            } else {
                RunRecordWriteError::Io(PersistenceIoError::new(
                    path,
                    "creating run record file",
                    e,
                ))
            }
        })?;

    let mut body_buf = Vec::new();
    write_run_record_body(&mut body_buf, record).map_err(|e| {
        RunRecordWriteError::Io(PersistenceIoError::new(path, "encoding run record", e))
    })?;
    let content_hash = fnv1a_64(&body_buf);

    let mut writer = io::BufWriter::new(file);
    writer.write_all(&body_buf).map_err(|e| {
        RunRecordWriteError::Io(PersistenceIoError::new(path, "writing run record body", e))
    })?;
    writeln!(writer).map_err(|e| {
        RunRecordWriteError::Io(PersistenceIoError::new(
            path,
            "writing run record trailer",
            e,
        ))
    })?;
    writeln!(writer, "{CONTENT_HASH_FIELD}: {}", format_hash(content_hash)).map_err(|e| {
        RunRecordWriteError::Io(PersistenceIoError::new(path, "writing content_hash", e))
    })?;
    writer.flush().map_err(|e| {
        RunRecordWriteError::Io(PersistenceIoError::new(path, "flushing run record", e))
    })?;
    Ok(())
}

fn write_run_record_body<W: Write>(writer: &mut W, record: &RunRecord) -> io::Result<()> {
    writeln!(writer, "{RUN_RECORD_MAGIC}")?;
    writeln!(writer, "run_id: {}", format_run_id(&record.run_id))?;
    writeln!(writer, "schema_version: {}", record.schema_version)?;
    writeln!(writer, "created_at: {}", format_utc(record.created_at))?;
    writeln!(writer, "tool_version: {}", record.tool_version)?;
    writeln!(writer)?;

    writeln!(writer, "[config]")?;
    writeln!(
        writer,
        "board_size: {}x{}",
        record.config.board_size.0, record.config.board_size.1
    )?;
    writeln!(writer, "max_iterations: {}", record.config.max_iterations)?;
    writeln!(
        writer,
        "max_board_memory_bytes: {}",
        record.config.max_board_memory_bytes
    )?;
    writeln!(
        writer,
        "initial_board_source: {}",
        record.config.initial_board_source
    )?;
    writeln!(writer, "random_seed: {}", record.config.random_seed)?;
    writeln!(writer, "updater: {}", record.config.updater)?;
    match &record.config.continued_from {
        Some(id) => writeln!(writer, "continued_from: {}", format_run_id(id))?,
        None => writeln!(writer, "continued_from:")?,
    }
    writeln!(writer)?;

    writeln!(writer, "[result]")?;
    writeln!(writer, "status: {}", record.result.status)?;
    writeln!(writer, "iterations_run: {}", record.result.iterations_run)?;
    writeln!(writer, "wall_time_ms: {}", record.result.wall_time_ms)?;
    writeln!(
        writer,
        "initial_alive_count: {}",
        record.result.initial_alive_count
    )?;
    writeln!(
        writer,
        "final_alive_count: {}",
        record.result.final_alive_count
    )?;
    writeln!(
        writer,
        "peak_alive_count: {}",
        record.result.peak_alive_count
    )?;
    writeln!(
        writer,
        "peak_alive_generation: {}",
        record.result.peak_alive_generation
    )?;
    writeln!(writer, "min_alive_count: {}", record.result.min_alive_count)?;
    writeln!(
        writer,
        "min_alive_generation: {}",
        record.result.min_alive_generation
    )?;
    writeln!(writer, "total_births: {}", record.result.total_births)?;
    writeln!(writer, "total_deaths: {}", record.result.total_deaths)?;
    writeln!(
        writer,
        "initial_board_hash: {}",
        format_hash(record.result.initial_board_hash)
    )?;
    writeln!(
        writer,
        "final_board_hash: {}",
        format_hash(record.result.final_board_hash)
    )?;
    writeln!(writer)?;

    write_board_block_to(writer, INITIAL_BOARD_LABEL, &record.initial_board)?;
    writeln!(writer)?;
    write_board_block_to(writer, FINAL_BOARD_LABEL, &record.final_board)?;

    Ok(())
}

/// Computes the FNV-1a 64-bit hash of an `InMemoryBoard`'s grid using the
/// same ASCII row-by-row encoding the writer uses.
pub fn board_grid_hash(board: &InMemoryBoard) -> u64 {
    let mut buffer = Vec::with_capacity(
        board
            .width()
            .saturating_mul(board.height() + 1)
            .saturating_mul(2),
    );
    for y in 0..board.height() {
        for x in 0..board.width() {
            let ch = match board.get(x, y) {
                crate::board::CellState::Alive => b'#',
                _ => b'.',
            };
            buffer.push(ch);
        }
        buffer.push(b'\n');
    }
    fnv1a_64(&buffer)
}

// -------- reading --------------------------------------------------------

pub fn read_run_record(
    path: impl AsRef<Path>,
    max_board_memory_bytes: usize,
    max_input_file_bytes: usize,
    content_hash_mode: ContentHashMode,
) -> Result<RunRecord, RunRecordReadError> {
    read_run_record_with_warnings(
        path,
        max_board_memory_bytes,
        max_input_file_bytes,
        content_hash_mode,
    )
    .map(|loaded| loaded.record)
}

pub fn read_run_record_with_warnings(
    path: impl AsRef<Path>,
    max_board_memory_bytes: usize,
    max_input_file_bytes: usize,
    content_hash_mode: ContentHashMode,
) -> Result<LoadedRunRecord, RunRecordReadError> {
    let path = path.as_ref();
    let body = slurp_with_size_guard_into_run_error(path, max_input_file_bytes)?;
    let kind = sniff_from_reader(path, &mut body.as_bytes())?;
    if kind != FileKind::RunRecord {
        return Err(RunRecordReadError::UnexpectedFileKind {
            path: path.to_path_buf(),
            expected: FileKind::RunRecord,
            actual: kind,
        });
    }

    let canonical = canonicalize_body(&body);
    let (canonical_without_trailer, found_trailer) = split_off_trailer(&canonical);
    let mut warnings: Vec<String> = Vec::new();
    match (found_trailer, content_hash_mode) {
        (Some(expected_hash), ContentHashMode::Enforce) => {
            let computed = fnv1a_64(canonical_without_trailer.as_bytes());
            if expected_hash != computed {
                return Err(RunRecordReadError::Corrupted {
                    path: path.to_path_buf(),
                    expected_hash,
                    actual_hash: computed,
                });
            }
        }
        (Some(expected_hash), ContentHashMode::Ignore) => {
            let computed = fnv1a_64(canonical_without_trailer.as_bytes());
            if expected_hash != computed {
                warnings.push(format!(
                    "Warning: integrity check bypassed for '{}': expected content_hash {}, computed {}; data may have been edited.",
                    path.display(),
                    format_hash(expected_hash),
                    format_hash(computed),
                ));
            }
        }
        (None, ContentHashMode::Enforce) => {
            return Err(RunRecordReadError::MissingContentHash {
                path: path.to_path_buf(),
            });
        }
        (None, ContentHashMode::Ignore) => {
            warnings.push(format!(
                "Warning: integrity check bypassed for '{}': no content_hash trailer found.",
                path.display()
            ));
        }
    }

    let mut cursor = LineCursor::new(path.display().to_string(), &canonical_without_trailer);
    cursor_skip_magic(&mut cursor, RUN_RECORD_MAGIC)?;

    let header = parse_run_header(&mut cursor)?;
    let config = parse_config_section(&mut cursor)?;
    let result = parse_result_section(&mut cursor)?;

    let initial_board = read_embedded_board_block(
        &mut cursor,
        INITIAL_BOARD_LABEL,
        max_board_memory_bytes,
        header.run_id,
        "INITIAL",
    )?;
    let final_board = read_embedded_board_block(
        &mut cursor,
        FINAL_BOARD_LABEL,
        max_board_memory_bytes,
        header.run_id,
        "FINAL",
    )?;

    let initial_actual = board_grid_hash(&initial_board);
    let final_actual = board_grid_hash(&final_board);
    if initial_actual != result.initial_board_hash {
        let msg = format!(
            "initial_board_hash mismatch: expected {}, computed {}",
            format_hash(result.initial_board_hash),
            format_hash(initial_actual)
        );
        match content_hash_mode {
            ContentHashMode::Enforce => {
                return Err(RunRecordReadError::Corrupted {
                    path: path.to_path_buf(),
                    expected_hash: result.initial_board_hash,
                    actual_hash: initial_actual,
                });
            }
            ContentHashMode::Ignore => warnings.push(format!("Warning: {msg}")),
        }
    }
    if final_actual != result.final_board_hash {
        let msg = format!(
            "final_board_hash mismatch: expected {}, computed {}",
            format_hash(result.final_board_hash),
            format_hash(final_actual)
        );
        match content_hash_mode {
            ContentHashMode::Enforce => {
                return Err(RunRecordReadError::Corrupted {
                    path: path.to_path_buf(),
                    expected_hash: result.final_board_hash,
                    actual_hash: final_actual,
                });
            }
            ContentHashMode::Ignore => warnings.push(format!("Warning: {msg}")),
        }
    }

    Ok(LoadedRunRecord {
        record: RunRecord {
            run_id: header.run_id,
            schema_version: header.schema_version,
            created_at: header.created_at,
            tool_version: header.tool_version,
            config,
            result,
            initial_board,
            final_board,
        },
        warnings,
    })
}

#[derive(Debug)]
pub struct LoadedRunRecord {
    pub record: RunRecord,
    pub warnings: Vec<String>,
}

fn slurp_with_size_guard_into_run_error(
    path: &Path,
    max_bytes: usize,
) -> Result<String, RunRecordReadError> {
    match slurp_with_size_guard(path, max_bytes) {
        Ok(body) => Ok(body),
        Err(BoardSnapshotReadError::Io(e)) => Err(RunRecordReadError::Io(e)),
        Err(BoardSnapshotReadError::FileTooLarge {
            path,
            actual_bytes,
            limit_bytes,
        }) => Err(RunRecordReadError::FileTooLarge {
            path,
            actual_bytes,
            limit_bytes,
        }),
        Err(other) => Err(other.into()),
    }
}

fn cursor_skip_magic(
    cursor: &mut LineCursor<'_>,
    expected: &str,
) -> Result<(), RunRecordReadError> {
    let line = cursor.next_logical().map_err(RunRecordReadError::from)?;
    let line = line.ok_or_else(|| ParseError::UnexpectedEnd {
        location: cursor.eof_location(),
        expected: format!("'{expected}'"),
    })?;
    if line.trim() != expected {
        return Err(RunRecordReadError::Parse(ParseError::MalformedFieldLine {
            location: cursor.last_consumed_location(),
            line,
        }));
    }
    Ok(())
}

#[derive(Debug)]
struct RunHeader {
    run_id: RunId,
    schema_version: u32,
    created_at: SystemTime,
    tool_version: String,
}

fn parse_run_header(cursor: &mut LineCursor<'_>) -> Result<RunHeader, RunRecordReadError> {
    let mut run_id: Option<RunId> = None;
    let mut schema_version: Option<u32> = None;
    let mut created_at: Option<SystemTime> = None;
    let mut tool_version: Option<String> = None;

    loop {
        let peek = cursor.peek_logical().map_err(RunRecordReadError::from)?;
        let line = match peek {
            Some(l) => l,
            None => break,
        };
        if parse_section_header(line).is_some() {
            break;
        }
        let location = cursor.current_location();
        let owned = line.to_string();
        cursor.consume();
        let (key, value) = parse_field_line(location.clone(), &owned)?;
        match key {
            "run_id" => {
                if run_id.is_some() {
                    return Err(ParseError::DuplicateField {
                        location,
                        field: key.to_string(),
                    }
                    .into());
                }
                run_id = Some(parse_run_id(value).map_err(|_| {
                    RunRecordReadError::MalformedRunId {
                        location: location.clone(),
                        value: value.to_string(),
                    }
                })?);
            }
            "schema_version" => {
                if schema_version.is_some() {
                    return Err(ParseError::DuplicateField {
                        location,
                        field: key.to_string(),
                    }
                    .into());
                }
                let parsed: u32 =
                    value
                        .parse()
                        .map_err(|_| RunRecordReadError::MalformedField {
                            location: location.clone(),
                            field: key.to_string(),
                            value: value.to_string(),
                        })?;
                if parsed != SCHEMA_VERSION {
                    return Err(ParseError::UnsupportedSchemaVersion {
                        location,
                        version: parsed,
                    }
                    .into());
                }
                schema_version = Some(parsed);
            }
            "created_at" => {
                if created_at.is_some() {
                    return Err(ParseError::DuplicateField {
                        location,
                        field: key.to_string(),
                    }
                    .into());
                }
                created_at = Some(parse_utc(value)?);
            }
            "tool_version" => {
                if tool_version.is_some() {
                    return Err(ParseError::DuplicateField {
                        location,
                        field: key.to_string(),
                    }
                    .into());
                }
                tool_version = Some(value.to_string());
            }
            _ => {
                return Err(ParseError::MalformedFieldLine {
                    location,
                    line: owned,
                }
                .into());
            }
        }
    }

    Ok(RunHeader {
        run_id: run_id.ok_or_else(|| RunRecordReadError::MissingField {
            section: "header".to_string(),
            field: "run_id".to_string(),
        })?,
        schema_version: schema_version.ok_or_else(|| RunRecordReadError::MissingField {
            section: "header".to_string(),
            field: "schema_version".to_string(),
        })?,
        created_at: created_at.ok_or_else(|| RunRecordReadError::MissingField {
            section: "header".to_string(),
            field: "created_at".to_string(),
        })?,
        tool_version: tool_version.ok_or_else(|| RunRecordReadError::MissingField {
            section: "header".to_string(),
            field: "tool_version".to_string(),
        })?,
    })
}

fn parse_config_section(
    cursor: &mut LineCursor<'_>,
) -> Result<RunRecordConfig, RunRecordReadError> {
    expect_section_header(cursor, "config")?;
    let mut board_size: Option<(usize, usize)> = None;
    let mut max_iterations: Option<usize> = None;
    let mut max_board_memory_bytes: Option<usize> = None;
    let mut initial_board_source: Option<String> = None;
    let mut random_seed: Option<u64> = None;
    let mut updater: Option<String> = None;
    let mut continued_from: Option<Option<RunId>> = None;

    loop {
        let peek = cursor.peek_logical().map_err(RunRecordReadError::from)?;
        let line = match peek {
            Some(l) => l,
            None => break,
        };
        if parse_section_header(line).is_some() || parse_begin_fence(line).is_some() {
            break;
        }
        let location = cursor.current_location();
        let owned = line.to_string();
        cursor.consume();
        let (key, value) = parse_field_line(location.clone(), &owned)?;
        match key {
            "board_size" => {
                board_size = Some(parse_size_value(value).ok_or_else(|| {
                    RunRecordReadError::MalformedField {
                        location: location.clone(),
                        field: key.to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "max_iterations" => {
                max_iterations = Some(parse_usize_field(value, key, &location)?);
            }
            "max_board_memory_bytes" => {
                max_board_memory_bytes = Some(parse_usize_field(value, key, &location)?);
            }
            "initial_board_source" => {
                initial_board_source = Some(value.to_string());
            }
            "random_seed" => {
                random_seed = Some(parse_u64_field(value, key, &location)?);
            }
            "updater" => {
                updater = Some(value.to_string());
            }
            "continued_from" => {
                continued_from = Some(if value.is_empty() {
                    None
                } else {
                    Some(parse_run_id(value).map_err(|_| {
                        RunRecordReadError::MalformedRunId {
                            location: location.clone(),
                            value: value.to_string(),
                        }
                    })?)
                });
            }
            _ => {
                return Err(ParseError::MalformedFieldLine {
                    location,
                    line: owned,
                }
                .into());
            }
        }
    }

    let board_size = board_size.ok_or_else(|| RunRecordReadError::MissingField {
        section: "config".to_string(),
        field: "board_size".to_string(),
    })?;
    let max_iterations = max_iterations.ok_or_else(|| RunRecordReadError::MissingField {
        section: "config".to_string(),
        field: "max_iterations".to_string(),
    })?;
    let max_board_memory_bytes =
        max_board_memory_bytes.ok_or_else(|| RunRecordReadError::MissingField {
            section: "config".to_string(),
            field: "max_board_memory_bytes".to_string(),
        })?;
    let initial_board_source =
        initial_board_source.ok_or_else(|| RunRecordReadError::MissingField {
            section: "config".to_string(),
            field: "initial_board_source".to_string(),
        })?;
    let random_seed = random_seed.ok_or_else(|| RunRecordReadError::MissingField {
        section: "config".to_string(),
        field: "random_seed".to_string(),
    })?;
    let updater = updater.ok_or_else(|| RunRecordReadError::MissingField {
        section: "config".to_string(),
        field: "updater".to_string(),
    })?;
    let continued_from = continued_from.ok_or_else(|| RunRecordReadError::MissingField {
        section: "config".to_string(),
        field: "continued_from".to_string(),
    })?;

    Ok(RunRecordConfig {
        board_size,
        max_iterations,
        max_board_memory_bytes,
        initial_board_source,
        random_seed,
        updater,
        continued_from,
    })
}

fn parse_result_section(
    cursor: &mut LineCursor<'_>,
) -> Result<RunRecordResult, RunRecordReadError> {
    expect_section_header(cursor, "result")?;
    let mut status: Option<String> = None;
    let mut iterations_run: Option<u64> = None;
    let mut wall_time_ms: Option<u64> = None;
    let mut initial_alive_count: Option<u64> = None;
    let mut final_alive_count: Option<u64> = None;
    let mut peak_alive_count: Option<u64> = None;
    let mut peak_alive_generation: Option<u64> = None;
    let mut min_alive_count: Option<u64> = None;
    let mut min_alive_generation: Option<u64> = None;
    let mut total_births: Option<u64> = None;
    let mut total_deaths: Option<u64> = None;
    let mut initial_board_hash: Option<u64> = None;
    let mut final_board_hash: Option<u64> = None;

    loop {
        let peek = cursor.peek_logical().map_err(RunRecordReadError::from)?;
        let line = match peek {
            Some(l) => l,
            None => break,
        };
        if parse_begin_fence(line).is_some() {
            break;
        }
        let location = cursor.current_location();
        let owned = line.to_string();
        cursor.consume();
        let (key, value) = parse_field_line(location.clone(), &owned)?;
        match key {
            "status" => {
                if !RECOGNIZED_STATUSES.contains(&value) {
                    return Err(RunRecordReadError::UnrecognizedStatus {
                        location,
                        value: value.to_string(),
                    });
                }
                status = Some(value.to_string());
            }
            "iterations_run" => iterations_run = Some(parse_u64_field(value, key, &location)?),
            "wall_time_ms" => wall_time_ms = Some(parse_u64_field(value, key, &location)?),
            "initial_alive_count" => {
                initial_alive_count = Some(parse_u64_field(value, key, &location)?)
            }
            "final_alive_count" => {
                final_alive_count = Some(parse_u64_field(value, key, &location)?)
            }
            "peak_alive_count" => peak_alive_count = Some(parse_u64_field(value, key, &location)?),
            "peak_alive_generation" => {
                peak_alive_generation = Some(parse_u64_field(value, key, &location)?)
            }
            "min_alive_count" => min_alive_count = Some(parse_u64_field(value, key, &location)?),
            "min_alive_generation" => {
                min_alive_generation = Some(parse_u64_field(value, key, &location)?)
            }
            "total_births" => total_births = Some(parse_u64_field(value, key, &location)?),
            "total_deaths" => total_deaths = Some(parse_u64_field(value, key, &location)?),
            "initial_board_hash" => {
                initial_board_hash = Some(parse_hash(value).map_err(|_| {
                    RunRecordReadError::MalformedField {
                        location: location.clone(),
                        field: key.to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            "final_board_hash" => {
                final_board_hash = Some(parse_hash(value).map_err(|_| {
                    RunRecordReadError::MalformedField {
                        location: location.clone(),
                        field: key.to_string(),
                        value: value.to_string(),
                    }
                })?);
            }
            _ => {
                return Err(ParseError::MalformedFieldLine {
                    location,
                    line: owned,
                }
                .into());
            }
        }
    }

    fn required_field<T>(opt: Option<T>, name: &str) -> Result<T, RunRecordReadError> {
        opt.ok_or_else(|| RunRecordReadError::MissingField {
            section: "result".to_string(),
            field: name.to_string(),
        })
    }

    Ok(RunRecordResult {
        status: required_field(status, "status")?,
        iterations_run: required_field(iterations_run, "iterations_run")?,
        wall_time_ms: required_field(wall_time_ms, "wall_time_ms")?,
        initial_alive_count: required_field(initial_alive_count, "initial_alive_count")?,
        final_alive_count: required_field(final_alive_count, "final_alive_count")?,
        peak_alive_count: required_field(peak_alive_count, "peak_alive_count")?,
        peak_alive_generation: required_field(peak_alive_generation, "peak_alive_generation")?,
        min_alive_count: required_field(min_alive_count, "min_alive_count")?,
        min_alive_generation: required_field(min_alive_generation, "min_alive_generation")?,
        total_births: required_field(total_births, "total_births")?,
        total_deaths: required_field(total_deaths, "total_deaths")?,
        initial_board_hash: required_field(initial_board_hash, "initial_board_hash")?,
        final_board_hash: required_field(final_board_hash, "final_board_hash")?,
    })
}

fn expect_section_header(
    cursor: &mut LineCursor<'_>,
    expected: &str,
) -> Result<(), RunRecordReadError> {
    let line = cursor
        .next_logical()
        .map_err(RunRecordReadError::from)?
        .ok_or_else(|| ParseError::UnexpectedEnd {
            location: cursor.eof_location(),
            expected: format!("[{expected}]"),
        })?;
    let location = cursor.last_consumed_location();
    let actual =
        parse_section_header(&line).ok_or_else(|| ParseError::MalformedSectionHeader {
            location: location.clone(),
            line: line.clone(),
        })?;
    if actual != expected {
        return Err(ParseError::UnexpectedSection {
            location,
            section: actual.to_string(),
        }
        .into());
    }
    Ok(())
}

fn parse_size_value(value: &str) -> Option<(usize, usize)> {
    let trimmed = value.trim();
    let mut parts = trimmed.split(['x', 'X']);
    let w = parts.next()?.trim().parse::<usize>().ok()?;
    let h = parts.next()?.trim().parse::<usize>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((w, h))
}

fn parse_usize_field(
    value: &str,
    field: &str,
    location: &ParseLocation,
) -> Result<usize, RunRecordReadError> {
    value
        .trim()
        .parse::<usize>()
        .map_err(|_| RunRecordReadError::MalformedField {
            location: location.clone(),
            field: field.to_string(),
            value: value.to_string(),
        })
}

fn parse_u64_field(
    value: &str,
    field: &str,
    location: &ParseLocation,
) -> Result<u64, RunRecordReadError> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| RunRecordReadError::MalformedField {
            location: location.clone(),
            field: field.to_string(),
            value: value.to_string(),
        })
}

fn read_embedded_board_block(
    cursor: &mut LineCursor<'_>,
    expected_label: &str,
    max_board_memory_bytes: usize,
    run_id: RunId,
    block_tag: &'static str,
) -> Result<InMemoryBoard, RunRecordReadError> {
    match read_board_block(cursor, expected_label, max_board_memory_bytes) {
        Ok(board) => Ok(board),
        Err(BoardSnapshotReadError::LoadedBoardSize(source)) => {
            Err(RunRecordReadError::BoardBlockTooLarge {
                block: block_tag,
                run_id,
                source,
            })
        }
        Err(other) => Err(other.into()),
    }
}

// -------- canonical form + trailer extraction ----------------------------

fn canonicalize_body(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    for (idx, line) in body.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        let stripped = strip_trailing_cr(line).trim_end();
        out.push_str(stripped);
    }
    // Ensure exactly one trailing newline.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn split_off_trailer(canonical: &str) -> (String, Option<u64>) {
    let lines: Vec<&str> = canonical.split('\n').collect();
    let mut idx = lines.len();
    while idx > 0 && lines[idx - 1].is_empty() {
        idx -= 1;
    }
    if idx == 0 {
        return (canonical.to_string(), None);
    }
    let last = lines[idx - 1];
    let trailer_hash = last
        .strip_prefix(CONTENT_HASH_FIELD)
        .and_then(|rest| rest.trim_start().strip_prefix(':'))
        .and_then(|rest| parse_hash(rest.trim()).ok());
    let Some(hash) = trailer_hash else {
        return (canonical.to_string(), None);
    };
    let body_lines = &lines[..idx - 1];
    let mut trimmed_end = body_lines.len();
    while trimmed_end > 0 && body_lines[trimmed_end - 1].is_empty() {
        trimmed_end -= 1;
    }
    let mut rebuilt = String::with_capacity(canonical.len());
    for line in &body_lines[..trimmed_end] {
        rebuilt.push_str(line);
        rebuilt.push('\n');
    }
    (rebuilt, Some(hash))
}

// -------- extract-board --------------------------------------------------

pub fn extract_board_from_run(
    path: impl AsRef<Path>,
    which: ExtractWhich,
    output_path: impl AsRef<Path>,
    max_board_memory_bytes: usize,
    max_input_file_bytes: usize,
    content_hash_mode: ContentHashMode,
) -> Result<(), ExtractBoardError> {
    let path = path.as_ref();
    let output_path = output_path.as_ref();
    let loaded = read_run_record_with_warnings(
        path,
        max_board_memory_bytes,
        max_input_file_bytes,
        content_hash_mode,
    )
    .map_err(ExtractBoardError::Read)?;
    let board = match which {
        ExtractWhich::Initial => loaded.record.initial_board,
        ExtractWhich::Final => loaded.record.final_board,
    };
    let snapshot = BoardSnapshot::for_board(board);
    super::board_snapshot::write_board_snapshot(output_path, &snapshot)
        .map_err(ExtractBoardError::Write)?;
    Ok(())
}

#[derive(Debug)]
pub enum ExtractBoardError {
    Read(RunRecordReadError),
    Write(BoardSnapshotWriteError),
}

impl fmt::Display for ExtractBoardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractBoardError::Read(e) => write!(f, "{e}"),
            ExtractBoardError::Write(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ExtractBoardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExtractBoardError::Read(e) => Some(e),
            ExtractBoardError::Write(e) => Some(e),
        }
    }
}

pub fn read_run_record_default(
    path: impl AsRef<Path>,
    max_board_memory_bytes: usize,
    content_hash_mode: ContentHashMode,
) -> Result<RunRecord, RunRecordReadError> {
    read_run_record(
        path,
        max_board_memory_bytes,
        DEFAULT_MAX_INPUT_FILE_BYTES,
        content_hash_mode,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::CellState;
    use crate::persistence::BOARD_SNAPSHOT_MAGIC;
    use std::time::UNIX_EPOCH;

    fn small_board(width: usize, height: usize, alive_at: &[(usize, usize)]) -> InMemoryBoard {
        let mut board = InMemoryBoard::new(width, height);
        for (x, y) in alive_at {
            board.set(*x, *y, CellState::Alive);
        }
        board
    }

    fn fixture_record() -> RunRecord {
        let run_id = RunId::from_bytes([
            0x7b, 0x3a, 0x1f, 0x0c, 0x4d, 0x2e, 0x4a, 0x51, 0x9c, 0x5e, 0x2f, 0x8c, 0x3a, 0x1b,
            0x9d, 0x77,
        ]);
        let initial = small_board(3, 3, &[(0, 0), (1, 1), (2, 2)]);
        let final_ = small_board(3, 3, &[(1, 1)]);
        let initial_hash = board_grid_hash(&initial);
        let final_hash = board_grid_hash(&final_);
        RunRecord {
            run_id,
            schema_version: SCHEMA_VERSION,
            created_at: UNIX_EPOCH + std::time::Duration::from_secs(1_780_000_000),
            tool_version: TOOL_VERSION.to_string(),
            config: RunRecordConfig {
                board_size: (3, 3),
                max_iterations: 10,
                max_board_memory_bytes: 64 * 1024 * 1024,
                initial_board_source: "random".to_string(),
                random_seed: 42,
                updater: "in_place_transitional".to_string(),
                continued_from: None,
            },
            result: RunRecordResult {
                status: "max_iterations".to_string(),
                iterations_run: 10,
                wall_time_ms: 3,
                initial_alive_count: 3,
                final_alive_count: 1,
                peak_alive_count: 3,
                peak_alive_generation: 0,
                min_alive_count: 1,
                min_alive_generation: 5,
                total_births: 4,
                total_deaths: 6,
                initial_board_hash: initial_hash,
                final_board_hash: final_hash,
            },
            initial_board: initial,
            final_board: final_,
        }
    }

    fn write_to_temp(path_label: &str, record: &RunRecord) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gol_run_record_{}_{}",
            path_label,
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{path_label}.gol"));
        let _ = std::fs::remove_file(&path);
        write_run_record(&path, record).unwrap();
        path
    }

    #[test]
    fn round_trip_writes_then_reads_identical_record() {
        let original = fixture_record();
        let path = write_to_temp("round_trip", &original);
        let loaded = read_run_record(
            &path,
            64 * 1024 * 1024,
            DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Enforce,
        )
        .unwrap();
        assert_eq!(loaded.run_id, original.run_id);
        assert_eq!(loaded.config.board_size, original.config.board_size);
        assert_eq!(loaded.config.random_seed, original.config.random_seed);
        assert_eq!(loaded.config.continued_from, original.config.continued_from);
        assert_eq!(loaded.result.status, original.result.status);
        assert_eq!(loaded.result.iterations_run, original.result.iterations_run);
        assert_eq!(loaded.initial_board, original.initial_board);
        assert_eq!(loaded.final_board, original.final_board);
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn round_trip_writes_then_reads_with_continued_from() {
        let mut original = fixture_record();
        original.config.continued_from = Some(RunId::from_bytes([0xaa; 16]));
        let path = write_to_temp("continued", &original);
        let loaded = read_run_record(
            &path,
            64 * 1024 * 1024,
            DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Enforce,
        )
        .unwrap();
        assert_eq!(loaded.config.continued_from, original.config.continued_from);
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn negative_read_detects_corruption() {
        let original = fixture_record();
        let path = write_to_temp("corruption", &original);
        let body = std::fs::read_to_string(&path).unwrap();
        // Replace a '.' on the final board with '#' to corrupt content.
        let corrupted = body.replacen("...\n.#.\n...", "...\n##.\n...", 1);
        assert_ne!(corrupted, body, "test fixture must produce a non-trivial mutation");
        std::fs::write(&path, corrupted).unwrap();
        let err = read_run_record(
            &path,
            64 * 1024 * 1024,
            DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Enforce,
        )
        .unwrap_err();
        assert!(matches!(err, RunRecordReadError::Corrupted { .. }));
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn ignore_integrity_downgrades_corruption_to_warning() {
        let original = fixture_record();
        let path = write_to_temp("ignore_integrity", &original);
        let body = std::fs::read_to_string(&path).unwrap();
        let corrupted = body.replacen("...\n.#.\n...", "...\n##.\n...", 1);
        // alive_count was 1 -> 2, dead_count was 8 -> 7; mismatch breaks
        // reading. Fix the counts so we test integrity, not header parsing.
        let corrupted = corrupted
            .replacen("alive_count: 1\ndead_count: 8", "alive_count: 2\ndead_count: 7", 1);
        std::fs::write(&path, corrupted).unwrap();
        let loaded = read_run_record_with_warnings(
            &path,
            64 * 1024 * 1024,
            DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Ignore,
        )
        .unwrap();
        assert!(!loaded.warnings.is_empty());
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn negative_read_missing_content_hash_under_enforce() {
        let original = fixture_record();
        let path = write_to_temp("missing_trailer", &original);
        let body = std::fs::read_to_string(&path).unwrap();
        let truncated = body
            .lines()
            .filter(|l| !l.starts_with("content_hash:"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, format!("{truncated}\n")).unwrap();
        let err = read_run_record(
            &path,
            64 * 1024 * 1024,
            DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Enforce,
        )
        .unwrap_err();
        assert!(matches!(err, RunRecordReadError::MissingContentHash { .. }));
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn crlf_rewrite_does_not_break_integrity_check() {
        let original = fixture_record();
        let path = write_to_temp("crlf", &original);
        let body = std::fs::read_to_string(&path).unwrap();
        let crlf = body.replace('\n', "\r\n");
        std::fs::write(&path, crlf).unwrap();
        let loaded = read_run_record(
            &path,
            64 * 1024 * 1024,
            DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Enforce,
        )
        .unwrap();
        assert_eq!(loaded.initial_board, original.initial_board);
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(path.parent().unwrap()).ok();
    }

    #[test]
    fn extract_board_round_trip_via_disk() {
        let original = fixture_record();
        let run_path = write_to_temp("extract_source", &original);
        let out_dir = run_path.parent().unwrap().to_path_buf();
        let out_path = out_dir.join("extracted.gol");
        let _ = std::fs::remove_file(&out_path);
        extract_board_from_run(
            &run_path,
            ExtractWhich::Final,
            &out_path,
            64 * 1024 * 1024,
            DEFAULT_MAX_INPUT_FILE_BYTES,
            ContentHashMode::Enforce,
        )
        .unwrap();
        let body = std::fs::read_to_string(&out_path).unwrap();
        assert!(body.starts_with(BOARD_SNAPSHOT_MAGIC));
        assert!(!body.contains("content_hash:"));
        std::fs::remove_file(&run_path).ok();
        std::fs::remove_file(&out_path).ok();
        std::fs::remove_dir(out_dir).ok();
    }

    #[test]
    fn negative_write_refuses_to_overwrite_existing() {
        let dir = std::env::temp_dir().join(format!(
            "gol_run_record_overwrite_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("collision.gol");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, b"existing").unwrap();
        let original = fixture_record();
        let err = write_run_record(&path, &original).unwrap_err();
        assert!(matches!(err, RunRecordWriteError::OutputExists { .. }));
        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
