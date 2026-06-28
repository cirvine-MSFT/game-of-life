//! Board snapshot file IO and board-block IO shared with run records.
//!
//! A standalone snapshot file has the shape:
//!
//! ```text
//! GOL-BOARD-SNAPSHOT v1
//! schema_version: 1
//! created_at: 2026-06-12T22:55:20Z
//!
//! ----- BEGIN BOARD -----
//! size: 10x10
//! encoding: ascii
//! alive_count: 3
//! dead_count: 97
//! ..........
//! ....#.....
//! ....#.....
//! ....#.....
//! ..........
//! ..........
//! ..........
//! ..........
//! ..........
//! ..........
//! ----- END BOARD -----
//! ```
//!
//! Standalone snapshots are intentionally hash-free and freely editable; that
//! is the supported workflow for users who want to craft new boards
//! from existing ones.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::board::{BoardView, CellCoordinate, CellState, InMemoryBoard, StreamingBoard};

use super::errors::PersistenceIoError;
use super::magic::{
    sniff_from_reader, FileKind, MagicError, BOARD_SNAPSHOT_MAGIC, BOARD_SNAPSHOT_SCHEMA_VERSION,
};
use super::parser::{
    format_begin_fence, format_end_fence, parse_begin_fence, parse_end_fence, parse_field_line,
    strip_trailing_cr, ParseError, ParseLocation,
};
use super::timestamps::{format_utc, parse_utc, TimestampParseError};
use super::DEFAULT_MAX_INPUT_FILE_BYTES;

/// Label used for the (single) fenced block inside a standalone board
/// snapshot.
pub const BOARD_BLOCK_LABEL: &str = "BOARD";

/// Minimum value we will ever suggest for `--max-board-memory` overrides.
/// Keeps suggestions readable.
pub const SUGGESTED_MEMORY_OVERRIDE_FLOOR_BYTES: usize = 1024;

const ENCODING_ASCII: &str = "ascii";

/// A standalone board snapshot file.
#[derive(Debug, Clone)]
pub struct BoardSnapshot {
    pub schema_version: u32,
    pub created_at: SystemTime,
    pub board: InMemoryBoard,
}

impl BoardSnapshot {
    pub fn for_board(board: InMemoryBoard) -> Self {
        Self {
            schema_version: BOARD_SNAPSHOT_SCHEMA_VERSION,
            created_at: SystemTime::now(),
            board,
        }
    }
}

#[derive(Debug)]
pub enum BoardSnapshotWriteError {
    Io(PersistenceIoError),
    OutputExists { path: PathBuf },
}

impl fmt::Display for BoardSnapshotWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoardSnapshotWriteError::Io(e) => write!(f, "{e}"),
            BoardSnapshotWriteError::OutputExists { path } => write!(
                f,
                "Refusing to overwrite existing file '{}'.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for BoardSnapshotWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BoardSnapshotWriteError::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum BoardSnapshotReadError {
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
    FileTooLarge {
        path: PathBuf,
        actual_bytes: u64,
        limit_bytes: usize,
    },
    MalformedSizeHeader {
        location: ParseLocation,
        value: String,
    },
}

impl fmt::Display for BoardSnapshotReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoardSnapshotReadError::Io(e) => write!(f, "{e}"),
            BoardSnapshotReadError::Magic(e) => write!(f, "{e}"),
            BoardSnapshotReadError::UnexpectedFileKind { path, expected, actual } => write!(
                f,
                "File '{}' is a {actual}, but expected a {expected}.",
                path.display()
            ),
            BoardSnapshotReadError::InvalidTimestamp(e) => write!(f, "{e}"),
            BoardSnapshotReadError::Parse(e) => write!(f, "{e}"),
            BoardSnapshotReadError::LoadedBoardSize(e) => write!(f, "{e}"),
            BoardSnapshotReadError::FileTooLarge {
                path,
                actual_bytes,
                limit_bytes,
            } => write!(
                f,
                "File '{}' is {actual_bytes} bytes which exceeds the {limit_bytes}-byte input file limit. Raise the limit with --max-input-file-bytes if the file is trustworthy.",
                path.display()
            ),
            BoardSnapshotReadError::MalformedSizeHeader { location, value } => write!(
                f,
                "Malformed board 'size' header at {location}: '{value}'. Expected WIDTHxHEIGHT, for example 10x10."
            ),
        }
    }
}

impl std::error::Error for BoardSnapshotReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BoardSnapshotReadError::Io(e) => Some(e),
            BoardSnapshotReadError::Magic(e) => Some(e),
            BoardSnapshotReadError::InvalidTimestamp(e) => Some(e),
            BoardSnapshotReadError::Parse(e) => Some(e),
            BoardSnapshotReadError::LoadedBoardSize(e) => Some(e),
            _ => None,
        }
    }
}

impl From<PersistenceIoError> for BoardSnapshotReadError {
    fn from(value: PersistenceIoError) -> Self {
        BoardSnapshotReadError::Io(value)
    }
}

impl From<MagicError> for BoardSnapshotReadError {
    fn from(value: MagicError) -> Self {
        BoardSnapshotReadError::Magic(value)
    }
}

impl From<ParseError> for BoardSnapshotReadError {
    fn from(value: ParseError) -> Self {
        BoardSnapshotReadError::Parse(value)
    }
}

impl From<TimestampParseError> for BoardSnapshotReadError {
    fn from(value: TimestampParseError) -> Self {
        BoardSnapshotReadError::InvalidTimestamp(value)
    }
}

impl From<LoadedBoardSizeError> for BoardSnapshotReadError {
    fn from(value: LoadedBoardSizeError) -> Self {
        BoardSnapshotReadError::LoadedBoardSize(value)
    }
}

/// Failure modes when validating that a board declared by a header would fit
/// within the configured memory budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadedBoardSizeError {
    /// The declared board would fit in some memory configuration, but not the
    /// currently-configured `--max-board-memory`.
    ExceedsMemoryBudget {
        width: usize,
        height: usize,
        required_bytes: usize,
        max_budget_bytes: usize,
        suggested_override: SuggestedMemoryOverride,
    },
    /// The declared board cannot fit in memory on this platform regardless of
    /// the configured budget (cell-count overflow, allocation overflow, or
    /// address-space exceeded).
    ExceedsAddressableMemory {
        width: usize,
        height: usize,
        required_bytes: u128,
        max_addressable_bytes: usize,
    },
}

impl fmt::Display for LoadedBoardSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadedBoardSizeError::ExceedsMemoryBudget {
                width,
                height,
                required_bytes,
                max_budget_bytes,
                suggested_override,
            } => write!(
                f,
                "Loaded board '{width}x{height}' needs {required_bytes} bytes, which exceeds the configured --max-board-memory of {max_budget_bytes} bytes. Try --max-board-memory {suggested_override} (supported suffixes: B, KB, MB, GB)."
            ),
            LoadedBoardSizeError::ExceedsAddressableMemory {
                width,
                height,
                required_bytes,
                max_addressable_bytes,
            } => write!(
                f,
                "Loaded board '{width}x{height}' needs {required_bytes} bytes, which cannot fit in memory on this platform (max addressable allocation is {max_addressable_bytes} bytes; pointer width is {pointer_bits}-bit). A streaming board impl is planned; see docs/design.md \u{2192} Deferred work.",
                pointer_bits = usize::BITS,
            ),
        }
    }
}

impl std::error::Error for LoadedBoardSizeError {}

/// A human-readable suggested value for `--max-board-memory`.
///
/// Rendered as `<integer><suffix>` (e.g. `256MB`, `4GB`). Always rounds up
/// past the required size to a familiar-looking number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuggestedMemoryOverride {
    pub bytes: usize,
    pub display: SuggestedMemoryDisplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuggestedMemoryDisplay {
    pub amount: usize,
    pub suffix: &'static str,
}

impl fmt::Display for SuggestedMemoryOverride {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.display.amount, self.display.suffix)
    }
}

/// Computes a memory-override suggestion guaranteed to be >= `required_bytes`,
/// rounded up to a familiar `<n>KB`/`<n>MB`/`<n>GB` boundary.
pub fn suggest_memory_override(required_bytes: usize) -> SuggestedMemoryOverride {
    let target = required_bytes.max(SUGGESTED_MEMORY_OVERRIDE_FLOOR_BYTES);

    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;
    const GB: usize = 1024 * 1024 * 1024;

    let (unit_bytes, suffix) = if target < MB {
        (KB, "KB")
    } else if target < GB {
        (MB, "MB")
    } else {
        (GB, "GB")
    };

    let mut amount = target.div_ceil(unit_bytes);
    // Round up to a friendly amount: powers of two within reason.
    amount = round_up_to_friendly(amount);

    let bytes = amount.saturating_mul(unit_bytes);
    SuggestedMemoryOverride {
        bytes,
        display: SuggestedMemoryDisplay { amount, suffix },
    }
}

fn round_up_to_friendly(amount: usize) -> usize {
    // Snap to common power-of-two-ish values: 1, 2, 4, 8, 16, ..., 1024.
    if amount <= 1 {
        return 1;
    }
    let mut snap = 1usize;
    while snap < amount {
        if snap >= 1024 {
            // Above 1024 we let the caller carry forward as-is, rounded up to
            // the next 256 to stay readable.
            return amount.div_ceil(256) * 256;
        }
        snap *= 2;
    }
    snap
}

/// Validates whether a board with the given dimensions would fit in the given
/// memory budget. Returns the required allocation size on success.
pub fn validate_loaded_board_size(
    width: usize,
    height: usize,
    max_budget_bytes: usize,
) -> Result<usize, LoadedBoardSizeError> {
    match InMemoryBoard::allocation_bytes(width, height) {
        Ok(required_bytes) => {
            if required_bytes > max_budget_bytes {
                Err(LoadedBoardSizeError::ExceedsMemoryBudget {
                    width,
                    height,
                    required_bytes,
                    max_budget_bytes,
                    suggested_override: suggest_memory_override(required_bytes),
                })
            } else {
                Ok(required_bytes)
            }
        }
        Err(crate::board::InMemoryBoardCreationError::AllocationAddressSpaceExceeded {
            requested_memory_bytes,
            max_addressable_bytes,
            ..
        }) => Err(LoadedBoardSizeError::ExceedsAddressableMemory {
            width,
            height,
            required_bytes: requested_memory_bytes as u128,
            max_addressable_bytes,
        }),
        Err(crate::board::InMemoryBoardCreationError::AllocationSizeOverflow {
            cell_count,
            cell_size,
            ..
        }) => Err(LoadedBoardSizeError::ExceedsAddressableMemory {
            width,
            height,
            required_bytes: (cell_count as u128).saturating_mul(cell_size as u128),
            max_addressable_bytes: isize::MAX as usize,
        }),
        Err(crate::board::InMemoryBoardCreationError::CellCountOverflow { width, height }) => {
            Err(LoadedBoardSizeError::ExceedsAddressableMemory {
                width,
                height,
                required_bytes: (width as u128).saturating_mul(height as u128),
                max_addressable_bytes: isize::MAX as usize,
            })
        }
        Err(crate::board::InMemoryBoardCreationError::MemoryBudgetExceeded { .. }) => {
            unreachable!("allocation_bytes does not enforce the configured memory budget")
        }
    }
}

// -------- writing --------------------------------------------------------

/// Writes a standalone board snapshot file at `path`. Refuses to overwrite an
/// existing file.
pub fn write_board_snapshot(
    path: impl AsRef<Path>,
    snapshot: &BoardSnapshot,
) -> Result<(), BoardSnapshotWriteError> {
    let path = path.as_ref();
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                BoardSnapshotWriteError::OutputExists {
                    path: path.to_path_buf(),
                }
            } else {
                BoardSnapshotWriteError::Io(PersistenceIoError::new(
                    path,
                    "creating snapshot file",
                    e,
                ))
            }
        })?;
    let mut writer = io::BufWriter::new(file);
    write_board_snapshot_to(&mut writer, snapshot).map_err(|e| {
        BoardSnapshotWriteError::Io(PersistenceIoError::new(path, "writing snapshot", e))
    })?;
    writer.flush().map_err(|e| {
        BoardSnapshotWriteError::Io(PersistenceIoError::new(path, "flushing snapshot", e))
    })?;
    Ok(())
}

/// Writes a standalone board snapshot to an arbitrary writer.
pub fn write_board_snapshot_to<W: Write>(
    writer: &mut W,
    snapshot: &BoardSnapshot,
) -> io::Result<()> {
    writeln!(writer, "{BOARD_SNAPSHOT_MAGIC}")?;
    writeln!(writer, "schema_version: {}", snapshot.schema_version)?;
    writeln!(writer, "created_at: {}", format_utc(snapshot.created_at))?;
    writeln!(writer)?;
    write_board_block_to(writer, BOARD_BLOCK_LABEL, &snapshot.board)?;
    Ok(())
}

/// Writes a fenced board block (label + size header + counts + grid) to the
/// writer. Used both for standalone snapshots and for embedded `INITIAL`/
/// `FINAL` blocks inside run records.
pub fn write_board_block_to<W: Write>(
    writer: &mut W,
    label: &str,
    board: &InMemoryBoard,
) -> io::Result<()> {
    // Render the grid into a buffer in one pass, counting alive cells while
    // we go. We need the count to emit the header *before* the grid, so we
    // buffer the grid bytes rather than walking the board twice.
    let width = board.width();
    let height = board.height();
    let mut grid_buf = Vec::with_capacity(width.saturating_mul(height + 1));
    let mut alive: usize = 0;
    for y in 0..height {
        for x in 0..width {
            let ch = match board.get(x, y) {
                CellState::Alive => {
                    alive += 1;
                    b'#'
                }
                _ => b'.',
            };
            grid_buf.push(ch);
        }
        grid_buf.push(b'\n');
    }
    let total = width.saturating_mul(height);
    let dead = total - alive;
    writeln!(writer, "{}", format_begin_fence(label))?;
    writeln!(writer, "size: {width}x{height}")?;
    writeln!(writer, "encoding: {ENCODING_ASCII}")?;
    writeln!(writer, "alive_count: {alive}")?;
    writeln!(writer, "dead_count: {dead}")?;
    writer.write_all(&grid_buf)?;
    writeln!(writer, "{}", format_end_fence(label))?;
    Ok(())
}

// -------- streaming snapshot writing -------------------------------------

/// Write a standalone board snapshot file from a `StreamingBoard`,
/// streaming row-by-row from the scratch file so the host never has to
/// hold the entire grid in memory.
///
/// Refuses to overwrite an existing file at `path` (matching
/// `write_board_snapshot`'s semantics).
pub fn write_streaming_board_snapshot(
    path: impl AsRef<Path>,
    board: &mut StreamingBoard,
) -> Result<(), BoardSnapshotWriteError> {
    let path = path.as_ref();
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists {
                BoardSnapshotWriteError::OutputExists {
                    path: path.to_path_buf(),
                }
            } else {
                BoardSnapshotWriteError::Io(PersistenceIoError::new(
                    path,
                    "creating streaming snapshot file",
                    e,
                ))
            }
        })?;
    let mut writer = BufWriter::new(file);
    write_streaming_board_snapshot_to(&mut writer, board).map_err(|e| {
        BoardSnapshotWriteError::Io(PersistenceIoError::new(
            path,
            "writing streaming snapshot",
            e,
        ))
    })?;
    writer.flush().map_err(|e| {
        BoardSnapshotWriteError::Io(PersistenceIoError::new(
            path,
            "flushing streaming snapshot",
            e,
        ))
    })?;
    Ok(())
}

/// Streaming counterpart of `write_board_snapshot_to`. Writes the standard
/// snapshot file format to an arbitrary writer, but pulls cell data from
/// a `StreamingBoard` one row at a time via `peek_cell` so peak host
/// memory stays bounded by the streaming board's chunk, not the board
/// size.
///
/// The output is byte-identical to what `write_board_snapshot_to` would
/// produce for the equivalent in-memory board — same magic header, same
/// `size` / `encoding` / `alive_count` / `dead_count` fields, same
/// `#`/`.` grid, same fences.
///
/// **Cost**: two full scans over the board via `peek_cell` — one to
/// total `alive_count` so the header can be emitted before the grid,
/// one to write the grid. A seek-and-patch single-pass variant would
/// halve the scratch I/O, but only at the cost of writing fixed-width
/// zero-padded count fields (e.g. `alive_count: 00000000000000000004`),
/// which would diverge from the hand-edit-friendly in-memory writer's
/// output format. The byte-identical contract is worth one extra scan;
/// I/O efficiency is a deferred follow-up.
pub fn write_streaming_board_snapshot_to<W: Write>(
    writer: &mut W,
    board: &mut StreamingBoard,
) -> io::Result<()> {
    writeln!(writer, "{BOARD_SNAPSHOT_MAGIC}")?;
    writeln!(writer, "schema_version: {BOARD_SNAPSHOT_SCHEMA_VERSION}")?;
    writeln!(writer, "created_at: {}", format_utc(SystemTime::now()))?;
    writeln!(writer)?;

    let width = board.width();
    let height = board.height();
    let mut alive_count: u64 = 0;
    for y in 0..height {
        for x in 0..width {
            if matches!(
                board
                    .peek_cell(CellCoordinate::new(x, y))
                    .map_err(io::Error::other)?,
                CellState::Alive
            ) {
                alive_count += 1;
            }
        }
    }
    let total: u64 = (width as u64).saturating_mul(height as u64);
    let dead_count = total - alive_count;

    writeln!(writer, "{}", format_begin_fence(BOARD_BLOCK_LABEL))?;
    writeln!(writer, "size: {width}x{height}")?;
    writeln!(writer, "encoding: {ENCODING_ASCII}")?;
    writeln!(writer, "alive_count: {alive_count}")?;
    writeln!(writer, "dead_count: {dead_count}")?;

    let mut row_buf = Vec::with_capacity(width + 1);
    for y in 0..height {
        row_buf.clear();
        for x in 0..width {
            let ch = match board
                .peek_cell(CellCoordinate::new(x, y))
                .map_err(io::Error::other)?
            {
                CellState::Alive => b'#',
                _ => b'.',
            };
            row_buf.push(ch);
        }
        row_buf.push(b'\n');
        writer.write_all(&row_buf)?;
    }
    writeln!(writer, "{}", format_end_fence(BOARD_BLOCK_LABEL))?;
    Ok(())
}

// -------- reading --------------------------------------------------------

/// Reads a standalone board snapshot file at `path`, validating it against
/// the configured memory budget and file-size limit.
pub fn read_board_snapshot(
    path: impl AsRef<Path>,
    max_board_memory_bytes: usize,
    max_input_file_bytes: usize,
) -> Result<BoardSnapshot, BoardSnapshotReadError> {
    let path = path.as_ref();
    let body = slurp_with_size_guard(path, max_input_file_bytes)?;
    let kind = sniff_from_reader(path, &mut body.as_bytes())?;
    if kind != FileKind::BoardSnapshot {
        return Err(BoardSnapshotReadError::UnexpectedFileKind {
            path: path.to_path_buf(),
            expected: FileKind::BoardSnapshot,
            actual: kind,
        });
    }

    let mut cursor = LineCursor::new(path.display().to_string(), &body);
    // The magic line itself was line 1; advance past it.
    cursor.expect_magic_line(BOARD_SNAPSHOT_MAGIC)?;

    // Optional ordering of header lines: schema_version, created_at. Both
    // required. We accept them in either order, but reject duplicates and
    // unknown keys (until we see the BEGIN fence).
    let mut schema_version: Option<u32> = None;
    let mut created_at: Option<SystemTime> = None;

    loop {
        match cursor.peek_logical()? {
            Some(line) => {
                if parse_begin_fence(line).is_some() {
                    break;
                }
                let location = cursor.current_location();
                cursor.consume();
                let (key, value) = parse_field_line(location.clone(), line)?;
                match key {
                    "schema_version" => {
                        if schema_version.is_some() {
                            return Err(BoardSnapshotReadError::Parse(
                                ParseError::DuplicateField {
                                    location,
                                    field: key.to_string(),
                                },
                            ));
                        }
                        let parsed =
                            parse_u32(value).ok_or_else(|| ParseError::MalformedFieldLine {
                                location: location.clone(),
                                line: format!("{key}: {value}"),
                            })?;
                        if parsed != BOARD_SNAPSHOT_SCHEMA_VERSION {
                            return Err(BoardSnapshotReadError::Parse(
                                ParseError::UnsupportedSchemaVersion {
                                    location,
                                    version: parsed,
                                },
                            ));
                        }
                        schema_version = Some(parsed);
                    }
                    "created_at" => {
                        if created_at.is_some() {
                            return Err(BoardSnapshotReadError::Parse(
                                ParseError::DuplicateField {
                                    location,
                                    field: key.to_string(),
                                },
                            ));
                        }
                        created_at = Some(parse_utc(value)?);
                    }
                    _ => {
                        return Err(BoardSnapshotReadError::Parse(
                            ParseError::MalformedFieldLine {
                                location,
                                line: format!("{key}: {value}"),
                            },
                        ));
                    }
                }
            }
            None => {
                return Err(BoardSnapshotReadError::Parse(ParseError::UnexpectedEnd {
                    location: cursor.eof_location(),
                    expected: format!("'{}'", format_begin_fence(BOARD_BLOCK_LABEL)),
                }));
            }
        }
    }

    let schema_version = schema_version.ok_or_else(|| ParseError::MissingRequiredField {
        section: "header".to_string(),
        field: "schema_version".to_string(),
    })?;
    let created_at = created_at.ok_or_else(|| ParseError::MissingRequiredField {
        section: "header".to_string(),
        field: "created_at".to_string(),
    })?;

    let board = read_board_block(&mut cursor, BOARD_BLOCK_LABEL, max_board_memory_bytes)?;
    Ok(BoardSnapshot {
        schema_version,
        created_at,
        board,
    })
}

/// Reads a fenced board block, asserting its label matches `expected_label`.
/// The cursor must be positioned at the BEGIN fence line.
pub(crate) fn read_board_block(
    cursor: &mut LineCursor<'_>,
    expected_label: &str,
    max_board_memory_bytes: usize,
) -> Result<InMemoryBoard, BoardSnapshotReadError> {
    let begin_line = cursor
        .next_logical()?
        .ok_or_else(|| ParseError::UnexpectedEnd {
            location: cursor.eof_location(),
            expected: format!("'{}'", format_begin_fence(expected_label)),
        })?;
    let begin_location = cursor.last_consumed_location();
    let label = parse_begin_fence(&begin_line).ok_or_else(|| ParseError::MalformedFence {
        location: begin_location.clone(),
        line: begin_line.clone(),
    })?;
    if label != expected_label {
        return Err(BoardSnapshotReadError::Parse(
            ParseError::UnexpectedFenceLabel {
                location: begin_location,
                expected: expected_label.to_string(),
                actual: label.to_string(),
            },
        ));
    }

    let mut header_size: Option<(usize, usize)> = None;
    let mut encoding_seen = false;
    let mut header_alive: Option<usize> = None;
    let mut header_dead: Option<usize> = None;
    let size_header_location;

    // Read header lines until we hit the first non-header content (the grid).
    loop {
        let location = cursor.current_location();
        let raw = cursor
            .next_raw()?
            .ok_or_else(|| ParseError::UnexpectedEnd {
                location: cursor.eof_location(),
                expected: format!(
                    "board header lines followed by '{}'",
                    format_end_fence(expected_label)
                ),
            })?;
        let line = strip_trailing_cr(&raw).trim_end();
        if line.is_empty() {
            // Blank inside a board block is not allowed; treat as a malformed
            // grid row.
            return Err(BoardSnapshotReadError::Parse(ParseError::RaggedBoardRow {
                location,
                expected_width: header_size.map(|(w, _)| w).unwrap_or(0),
                actual_width: 0,
            }));
        }
        if let Some((key, value)) = line.find(':').map(|i| (&line[..i], line[i + 1..].trim())) {
            let key = key.trim();
            match key {
                "size" => {
                    if header_size.is_some() {
                        return Err(BoardSnapshotReadError::Parse(ParseError::DuplicateField {
                            location,
                            field: "size".to_string(),
                        }));
                    }
                    let (w, h) = parse_size_value(value).ok_or_else(|| {
                        BoardSnapshotReadError::MalformedSizeHeader {
                            location: location.clone(),
                            value: value.to_string(),
                        }
                    })?;
                    if w == 0 || h == 0 {
                        return Err(BoardSnapshotReadError::MalformedSizeHeader {
                            location,
                            value: value.to_string(),
                        });
                    }
                    header_size = Some((w, h));
                    continue;
                }
                "encoding" => {
                    if encoding_seen {
                        return Err(BoardSnapshotReadError::Parse(ParseError::DuplicateField {
                            location,
                            field: "encoding".to_string(),
                        }));
                    }
                    if value != ENCODING_ASCII {
                        return Err(BoardSnapshotReadError::Parse(ParseError::UnknownEncoding {
                            location,
                            encoding: value.to_string(),
                        }));
                    }
                    encoding_seen = true;
                    continue;
                }
                "alive_count" => {
                    if header_alive.is_some() {
                        return Err(BoardSnapshotReadError::Parse(ParseError::DuplicateField {
                            location,
                            field: "alive_count".to_string(),
                        }));
                    }
                    header_alive =
                        Some(
                            parse_usize(value).ok_or_else(|| ParseError::MalformedFieldLine {
                                location: location.clone(),
                                line: format!("alive_count: {value}"),
                            })?,
                        );
                    continue;
                }
                "dead_count" => {
                    if header_dead.is_some() {
                        return Err(BoardSnapshotReadError::Parse(ParseError::DuplicateField {
                            location,
                            field: "dead_count".to_string(),
                        }));
                    }
                    header_dead =
                        Some(
                            parse_usize(value).ok_or_else(|| ParseError::MalformedFieldLine {
                                location: location.clone(),
                                line: format!("dead_count: {value}"),
                            })?,
                        );
                    continue;
                }
                _ => {
                    // Unknown header line: hand back to grid-parsing path,
                    // since a grid line starting with `.` or `#` will never
                    // contain a `:` (unless it's malformed, which falls
                    // through to the grid character check below anyway).
                    return Err(BoardSnapshotReadError::Parse(
                        ParseError::MalformedFieldLine {
                            location,
                            line: line.to_string(),
                        },
                    ));
                }
            }
        }

        // First non-header line: this is the first grid row. We need a size to
        // have been declared before now.
        let (width, height) = header_size.ok_or_else(|| ParseError::MissingRequiredField {
            section: format!("{expected_label} block"),
            field: "size".to_string(),
        })?;
        size_header_location = location.clone();

        // Validate memory budget *before* allocating.
        validate_loaded_board_size(width, height, max_board_memory_bytes)?;
        let mut board = InMemoryBoard::new(width, height);

        // First grid row is `line`; subsequent rows come from the cursor.
        parse_grid_row(&mut board, 0, line, width, &location)?;
        let mut alive_count = count_alive_in_row(line);

        for y in 1..height {
            let line_location = cursor.current_location();
            let raw_row = cursor
                .next_raw()?
                .ok_or_else(|| ParseError::UnexpectedEnd {
                    location: cursor.eof_location(),
                    expected: format!(
                        "additional board rows followed by '{}'",
                        format_end_fence(expected_label)
                    ),
                })?;
            let row = strip_trailing_cr(&raw_row).trim_end();
            parse_grid_row(&mut board, y, row, width, &line_location)?;
            alive_count += count_alive_in_row(row);
        }

        // Expect the END fence next.
        let end_location = cursor.current_location();
        let end_line = cursor
            .next_raw()?
            .ok_or_else(|| ParseError::UnexpectedEnd {
                location: cursor.eof_location(),
                expected: format!("'{}'", format_end_fence(expected_label)),
            })?;
        let end_line = strip_trailing_cr(&end_line).trim_end();
        let end_label = parse_end_fence(end_line).ok_or_else(|| ParseError::MalformedFence {
            location: end_location.clone(),
            line: end_line.to_string(),
        })?;
        if end_label != expected_label {
            return Err(BoardSnapshotReadError::Parse(
                ParseError::UnexpectedFenceLabel {
                    location: end_location,
                    expected: expected_label.to_string(),
                    actual: end_label.to_string(),
                },
            ));
        }

        // All header fields required.
        let header_alive = header_alive.ok_or_else(|| ParseError::MissingRequiredField {
            section: format!("{expected_label} block"),
            field: "alive_count".to_string(),
        })?;
        let header_dead = header_dead.ok_or_else(|| ParseError::MissingRequiredField {
            section: format!("{expected_label} block"),
            field: "dead_count".to_string(),
        })?;
        if !encoding_seen {
            return Err(BoardSnapshotReadError::Parse(
                ParseError::MissingRequiredField {
                    section: format!("{expected_label} block"),
                    field: "encoding".to_string(),
                },
            ));
        }

        // Verify derived counts match the grid.
        let total = width.saturating_mul(height);
        let computed_dead = total - alive_count;
        if header_alive != alive_count {
            return Err(BoardSnapshotReadError::Parse(
                ParseError::BoardCountMismatch {
                    location: size_header_location.clone(),
                    field: "alive_count",
                    header_value: header_alive,
                    grid_value: alive_count,
                },
            ));
        }
        if header_dead != computed_dead {
            return Err(BoardSnapshotReadError::Parse(
                ParseError::BoardCountMismatch {
                    location: size_header_location,
                    field: "dead_count",
                    header_value: header_dead,
                    grid_value: computed_dead,
                },
            ));
        }

        return Ok(board);
    }
}

fn parse_grid_row(
    board: &mut InMemoryBoard,
    y: usize,
    row: &str,
    expected_width: usize,
    location: &ParseLocation,
) -> Result<(), BoardSnapshotReadError> {
    let mut count: usize = 0;
    for ch in row.chars() {
        if count >= expected_width {
            // Walk the rest of the string just to compute the actual width
            // for the error; we already know we've exceeded the expected.
            let extra = row.chars().count();
            return Err(BoardSnapshotReadError::Parse(ParseError::RaggedBoardRow {
                location: location.clone(),
                expected_width,
                actual_width: extra,
            }));
        }
        let state = match ch {
            '.' => CellState::Dead,
            '#' => CellState::Alive,
            other if !other.is_ascii() => {
                return Err(BoardSnapshotReadError::Parse(
                    ParseError::NonAsciiBoardCharacter {
                        location: location.clone(),
                        character: other,
                    },
                ));
            }
            other => {
                return Err(BoardSnapshotReadError::Parse(
                    ParseError::UnknownBoardCharacter {
                        location: location.clone(),
                        character: other,
                    },
                ));
            }
        };
        // The function takes &mut InMemoryBoard concretely, so we use the
        // infallible `set` directly. (The trait method `set_cell` returns
        // `Result<(), Infallible>` on `InMemoryBoard`, which works but
        // requires either `.unwrap()` or `impl From<Infallible>` to compose
        // with `?` in this error type.)
        board.set(count, y, state);
        count += 1;
    }
    if count != expected_width {
        return Err(BoardSnapshotReadError::Parse(ParseError::RaggedBoardRow {
            location: location.clone(),
            expected_width,
            actual_width: count,
        }));
    }
    Ok(())
}

fn count_alive_in_row(row: &str) -> usize {
    row.chars().filter(|c| *c == '#').count()
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

fn parse_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

fn parse_usize(value: &str) -> Option<usize> {
    value.trim().parse::<usize>().ok()
}

// -------- file I/O helpers ------------------------------------------------

/// Reads a file into memory, refusing to slurp anything larger than
/// `max_bytes`. Returns the file body as a `String`.
pub(crate) fn slurp_with_size_guard(
    path: impl AsRef<Path>,
    max_bytes: usize,
) -> Result<String, BoardSnapshotReadError> {
    let path = path.as_ref();
    let metadata = std::fs::metadata(path)
        .map_err(|e| PersistenceIoError::new(path, "stat'ing input file", e))?;
    let actual_bytes = metadata.len();
    if actual_bytes > max_bytes as u64 {
        return Err(BoardSnapshotReadError::FileTooLarge {
            path: path.to_path_buf(),
            actual_bytes,
            limit_bytes: max_bytes,
        });
    }
    let file =
        File::open(path).map_err(|e| PersistenceIoError::new(path, "opening input file", e))?;
    let mut reader = BufReader::new(file);
    let mut body = String::with_capacity(actual_bytes as usize);
    reader
        .read_to_string(&mut body)
        .map_err(|e| PersistenceIoError::new(path, "reading input file", e))?;
    Ok(body)
}

/// Convenience for callers that want the default file-size limit.
pub fn read_board_snapshot_default(
    path: impl AsRef<Path>,
    max_board_memory_bytes: usize,
) -> Result<BoardSnapshot, BoardSnapshotReadError> {
    read_board_snapshot(path, max_board_memory_bytes, DEFAULT_MAX_INPUT_FILE_BYTES)
}

// -------- line cursor ----------------------------------------------------

/// A peekable cursor over the logical lines of a file body.
///
/// Tracks 1-based line numbers for error reporting and tolerates both LF and
/// CRLF inputs. "Logical" peek/consume skips blank lines; "raw" consume reads
/// the next line as-is. Board blocks use raw reads because internal blank lines
/// inside the board block are not allowed.
pub(crate) struct LineCursor<'a> {
    path: String,
    lines: Vec<&'a str>,
    cursor: usize,
    last_consumed_line: usize,
}

impl<'a> LineCursor<'a> {
    pub(crate) fn new(path: impl Into<String>, body: &'a str) -> Self {
        Self {
            path: path.into(),
            lines: body.split_inclusive('\n').collect(),
            cursor: 0,
            last_consumed_line: 0,
        }
    }

    fn current_line_number(&self) -> usize {
        self.cursor + 1
    }

    pub(crate) fn current_location(&self) -> ParseLocation {
        ParseLocation::new(self.path.clone(), self.current_line_number())
    }

    pub(crate) fn last_consumed_location(&self) -> ParseLocation {
        ParseLocation::new(self.path.clone(), self.last_consumed_line.max(1))
    }

    pub(crate) fn eof_location(&self) -> ParseLocation {
        ParseLocation::new(self.path.clone(), self.lines.len().max(1))
    }

    /// Returns the next non-blank line content (trimmed of `\r\n`) without
    /// consuming it. Returns `None` at EOF.
    pub(crate) fn peek_logical(&mut self) -> Result<Option<&'a str>, BoardSnapshotReadError> {
        while self.cursor < self.lines.len() {
            let raw = self.lines[self.cursor];
            let trimmed = strip_trailing_cr(raw.trim_end_matches('\n'));
            if trimmed.trim().is_empty() {
                self.cursor += 1;
                continue;
            }
            return Ok(Some(trimmed));
        }
        Ok(None)
    }

    /// Consumes one line from the cursor (whatever it was). Must be paired
    /// with `peek_logical` results.
    pub(crate) fn consume(&mut self) {
        if self.cursor < self.lines.len() {
            self.last_consumed_line = self.current_line_number();
            self.cursor += 1;
        }
    }

    /// Logical equivalent of `next`: returns the next non-blank line (CR-
    /// stripped, with no trailing `\n`) and advances past it.
    pub(crate) fn next_logical(&mut self) -> Result<Option<String>, BoardSnapshotReadError> {
        if let Some(line) = self.peek_logical()? {
            let owned = line.to_string();
            self.consume();
            Ok(Some(owned))
        } else {
            Ok(None)
        }
    }

    /// Raw equivalent: returns the next line (CR-stripped, no `\n`) without
    /// skipping blanks. Used inside board blocks where blank lines are not
    /// allowed.
    pub(crate) fn next_raw(&mut self) -> Result<Option<String>, BoardSnapshotReadError> {
        if self.cursor < self.lines.len() {
            let raw = self.lines[self.cursor];
            self.last_consumed_line = self.current_line_number();
            self.cursor += 1;
            let trimmed = strip_trailing_cr(raw.trim_end_matches('\n'));
            Ok(Some(trimmed.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Validates that the magic line we already consumed during sniffing
    /// matches; this implementation just advances past line 1.
    fn expect_magic_line(&mut self, expected: &str) -> Result<(), BoardSnapshotReadError> {
        let line = self.peek_logical()?.unwrap_or("");
        if line != expected {
            return Err(BoardSnapshotReadError::Parse(
                ParseError::MalformedFieldLine {
                    location: self.current_location(),
                    line: line.to_string(),
                },
            ));
        }
        self.consume();
        Ok(())
    }
}
