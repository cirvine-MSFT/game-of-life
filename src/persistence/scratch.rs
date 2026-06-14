//! Internal binary scratch file used by the streaming board.
//!
//! A scratch file is the disk backing store for a single in-progress
//! Game of Life run on a board that's too large to keep entirely in
//! RAM. It is *not* a user-facing file format: it's created on demand
//! by [`StreamingBoard`], owned by exactly one run, and (under normal
//! lifecycle) deleted when that run finishes. Snapshots remain the
//! durable, human-editable storage for board states; the scratch file
//! is a transient working area.
//!
//! # Format
//!
//! The file has a fixed-size 64-byte binary header followed by
//! `height × row_bytes` bytes of row payload. The header is binary
//! (not the existing text format) because random-access reads need
//! fixed strides and the scratch format is internal-only.
//!
//! ```text
//! Header (64 bytes):
//!   bytes  0..16: magic                = "GOL-SCRATCH v1\n\0"
//!   bytes 16..20: schema_version       = u32 LE (currently 1)
//!   bytes 20..28: created_at_unix_secs = u64 LE
//!   bytes 28..36: width                = u64 LE  (cells, > 0)
//!   bytes 36..44: height               = u64 LE  (cells, > 0)
//!   bytes 44..52: row_bytes            = u64 LE  (= ceil(width * 2 / 8))
//!   bytes 52..64: reserved             = 12 × 0
//!
//! Row payload (height rows, each row_bytes long, fixed stride):
//!   2 bits per cell, packed little-endian within each byte.
//!   4 cells per byte: lo-bit pair is the leftmost cell of the quad.
//!     byte layout (bit 7 .. bit 0): | c3 c3 | c2 c2 | c1 c1 | c0 c0 |
//!     where c0 is the leftmost cell of that 4-cell quad.
//!   Cell codes:
//!     00 = Dead
//!     01 = Alive
//!     10 = Dying
//!     11 = Resurrecting
//! ```
//!
//! Random-access to row `y`, columns `cstart..cend`:
//!
//! - Row file offset: `HEADER_SIZE + y * row_bytes`
//! - Byte range within row: `[cstart / 4, (cend + 3) / 4)` — i.e. the
//!   set of bytes that contain *any* bit belonging to the requested
//!   column range. This matters when `cstart` or `cend` is not a
//!   multiple of 4.
//!
//! Writes use **read-modify-write** for the two boundary bytes (the
//! byte containing `cstart` and the byte containing `cend - 1`), so
//! bits belonging to cells outside the write range are preserved.
//! Interior bytes are written wholesale.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::board::CellState;

use super::errors::PersistenceIoError;

/// Fixed magic prefix occupying the first 16 bytes of every scratch file.
pub const SCRATCH_MAGIC: &[u8; 16] = b"GOL-SCRATCH v1\n\0";

/// Schema version of the current scratch layout. Bumped if the on-disk
/// format ever changes.
pub const SCRATCH_SCHEMA_VERSION: u32 = 1;

/// Total fixed header size, in bytes.
pub const HEADER_SIZE: u64 = 64;

const CODE_DEAD: u8 = 0b00;
const CODE_ALIVE: u8 = 0b01;
const CODE_DYING: u8 = 0b10;
const CODE_RESURRECTING: u8 = 0b11;

/// Open scratch file, holding the metadata read from / written to the
/// header. Provides row-range read/write primitives.
#[derive(Debug)]
pub struct ScratchFile {
    file: File,
    path: PathBuf,
    width: u64,
    height: u64,
    row_bytes: u64,
}

impl ScratchFile {
    /// Compute the row stride for a board of the given width: the number
    /// of bytes needed to hold `width` cells at 2 bits each, rounded up
    /// to the next whole byte.
    pub fn row_bytes_for_width(width: u64) -> u64 {
        // ceil(width * 2 / 8) = ceil(width / 4)
        width.div_ceil(4)
    }

    /// Total file size for a board with the given dimensions: header
    /// plus `height × row_bytes`.
    pub fn file_size_for(width: u64, height: u64) -> u64 {
        HEADER_SIZE + height * Self::row_bytes_for_width(width)
    }

    /// Create a fresh scratch file at `path`. Refuses to overwrite an
    /// existing file. Writes the header, then zero-initializes the row
    /// payload sequentially so the file has a known final size.
    ///
    /// `created_at` is taken at call time from `SystemTime::now()` so
    /// the timestamp matches the run's start.
    pub fn create(
        path: impl AsRef<Path>,
        width: u64,
        height: u64,
    ) -> Result<Self, ScratchFileError> {
        let path = path.as_ref();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| match e.kind() {
                io::ErrorKind::AlreadyExists => ScratchFileError::AlreadyExists {
                    path: path.to_path_buf(),
                },
                _ => {
                    ScratchFileError::Io(PersistenceIoError::new(path, "creating scratch file", e))
                }
            })?;

        let row_bytes = Self::row_bytes_for_width(width);
        let created_at_unix_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut scratch = ScratchFile {
            file,
            path: path.to_path_buf(),
            width,
            height,
            row_bytes,
        };
        scratch.write_header(created_at_unix_secs).map_err(|e| {
            ScratchFileError::Io(PersistenceIoError::new(path, "writing scratch header", e))
        })?;
        scratch.zero_initialize_payload().map_err(|e| {
            ScratchFileError::Io(PersistenceIoError::new(
                path,
                "zero-initializing scratch payload",
                e,
            ))
        })?;
        Ok(scratch)
    }

    /// Open an existing scratch file for read+write. Validates magic,
    /// schema version, and reads dimensions from the header.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ScratchFileError> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| {
                ScratchFileError::Io(PersistenceIoError::new(path, "opening scratch file", e))
            })?;

        // Read the magic first so a short or non-scratch file yields a
        // BadMagic error instead of an EOF error.
        let mut magic = [0u8; 16];
        file.read_exact(&mut magic)
            .map_err(|_| ScratchFileError::BadMagic {
                path: path.to_path_buf(),
            })?;
        if &magic != SCRATCH_MAGIC {
            return Err(ScratchFileError::BadMagic {
                path: path.to_path_buf(),
            });
        }

        let mut rest = [0u8; (HEADER_SIZE - 16) as usize];
        file.read_exact(&mut rest).map_err(|e| {
            ScratchFileError::Io(PersistenceIoError::new(path, "reading scratch header", e))
        })?;
        let schema_version = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
        if schema_version != SCRATCH_SCHEMA_VERSION {
            return Err(ScratchFileError::UnsupportedSchemaVersion {
                path: path.to_path_buf(),
                version: schema_version,
            });
        }
        let width = u64::from_le_bytes(rest[12..20].try_into().unwrap());
        let height = u64::from_le_bytes(rest[20..28].try_into().unwrap());
        let row_bytes = u64::from_le_bytes(rest[28..36].try_into().unwrap());

        if width == 0 || height == 0 {
            return Err(ScratchFileError::InvalidHeader {
                path: path.to_path_buf(),
                detail: format!("width {width} and height {height} must both be positive"),
            });
        }
        let expected_row_bytes = Self::row_bytes_for_width(width);
        if row_bytes != expected_row_bytes {
            return Err(ScratchFileError::InvalidHeader {
                path: path.to_path_buf(),
                detail: format!(
                    "row_bytes {row_bytes} does not match expected ceil(width*2/8) = {expected_row_bytes} for width {width}"
                ),
            });
        }

        Ok(ScratchFile {
            file,
            path: path.to_path_buf(),
            width,
            height,
            row_bytes,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn width(&self) -> u64 {
        self.width
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn row_bytes(&self) -> u64 {
        self.row_bytes
    }

    /// Read cells in row `y`, columns `cstart..cend`, appending them to
    /// `out` (which is cleared first).
    pub fn read_row_range(
        &mut self,
        y: u64,
        cstart: u64,
        cend: u64,
        out: &mut Vec<CellState>,
    ) -> Result<(), ScratchFileError> {
        out.clear();
        if cend <= cstart {
            return Ok(());
        }
        self.validate_row_range(y, cstart, cend)?;

        let (byte_start, byte_end) = byte_range_for_cells(cstart, cend);
        let row_offset = HEADER_SIZE + y * self.row_bytes;
        let mut buf = vec![0u8; (byte_end - byte_start) as usize];
        self.file
            .seek(SeekFrom::Start(row_offset + byte_start))
            .map_err(|e| self.io_err("seeking for row read", e))?;
        self.file
            .read_exact(&mut buf)
            .map_err(|e| self.io_err("reading row range", e))?;

        // Unpack cells, skipping leading bits inside the first byte and
        // truncating trailing bits inside the last byte.
        let first_cell_offset = (cstart - byte_start * 4) as usize;
        let cell_count = (cend - cstart) as usize;
        out.reserve(cell_count);
        for i in 0..cell_count {
            let absolute_cell_in_buf = first_cell_offset + i;
            let byte_idx = absolute_cell_in_buf / 4;
            let bit_pair_idx = absolute_cell_in_buf % 4;
            let bits = (buf[byte_idx] >> (bit_pair_idx * 2)) & 0b11;
            out.push(decode_cell(bits));
        }
        Ok(())
    }

    /// Write cells into row `y`, columns `cstart..cstart + cells.len()`.
    /// Uses read-modify-write for the two boundary bytes to preserve
    /// bits belonging to cells outside the write range.
    pub fn write_row_range(
        &mut self,
        y: u64,
        cstart: u64,
        cells: &[CellState],
    ) -> Result<(), ScratchFileError> {
        if cells.is_empty() {
            return Ok(());
        }
        let cend = cstart + cells.len() as u64;
        self.validate_row_range(y, cstart, cend)?;

        let (byte_start, byte_end) = byte_range_for_cells(cstart, cend);
        let buf_len = (byte_end - byte_start) as usize;
        let row_offset = HEADER_SIZE + y * self.row_bytes;
        let file_byte_start = row_offset + byte_start;

        // Read the existing bytes so we can preserve bits in boundary
        // bytes that belong to neighboring cells outside the write range.
        let mut buf = vec![0u8; buf_len];
        self.file
            .seek(SeekFrom::Start(file_byte_start))
            .map_err(|e| self.io_err("seeking for row write read-modify-write", e))?;
        self.file
            .read_exact(&mut buf)
            .map_err(|e| self.io_err("read-modify-write read", e))?;

        let first_cell_offset = (cstart - byte_start * 4) as usize;
        for (i, cell) in cells.iter().enumerate() {
            let absolute_cell_in_buf = first_cell_offset + i;
            let byte_idx = absolute_cell_in_buf / 4;
            let bit_pair_idx = absolute_cell_in_buf % 4;
            let shift = bit_pair_idx * 2;
            let mask = !(0b11u8 << shift);
            buf[byte_idx] = (buf[byte_idx] & mask) | (encode_cell(*cell) << shift);
        }

        self.file
            .seek(SeekFrom::Start(file_byte_start))
            .map_err(|e| self.io_err("seeking for row write back", e))?;
        self.file
            .write_all(&buf)
            .map_err(|e| self.io_err("writing row range", e))?;
        Ok(())
    }

    /// Flush buffered writes to disk.
    pub fn flush(&mut self) -> Result<(), ScratchFileError> {
        self.file
            .flush()
            .map_err(|e| self.io_err("flushing scratch file", e))
    }

    fn write_header(&mut self, created_at_unix_secs: u64) -> io::Result<()> {
        let mut header = [0u8; HEADER_SIZE as usize];
        header[0..16].copy_from_slice(SCRATCH_MAGIC);
        header[16..20].copy_from_slice(&SCRATCH_SCHEMA_VERSION.to_le_bytes());
        header[20..28].copy_from_slice(&created_at_unix_secs.to_le_bytes());
        header[28..36].copy_from_slice(&self.width.to_le_bytes());
        header[36..44].copy_from_slice(&self.height.to_le_bytes());
        header[44..52].copy_from_slice(&self.row_bytes.to_le_bytes());
        // bytes 52..64 reserved, already zero.
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&header)?;
        Ok(())
    }

    fn zero_initialize_payload(&mut self) -> io::Result<()> {
        // Total payload bytes = height * row_bytes. Write in chunks of
        // 64 KiB so we don't allocate a multi-GB zero buffer for huge
        // boards.
        let total = self.height * self.row_bytes;
        if total == 0 {
            return Ok(());
        }
        const CHUNK_BYTES: u64 = 64 * 1024;
        let zeros = vec![0u8; CHUNK_BYTES as usize];
        self.file.seek(SeekFrom::Start(HEADER_SIZE))?;
        let mut written = 0u64;
        while written < total {
            let remaining = total - written;
            let to_write = remaining.min(CHUNK_BYTES) as usize;
            self.file.write_all(&zeros[..to_write])?;
            written += to_write as u64;
        }
        self.file.flush()?;
        Ok(())
    }

    fn validate_row_range(&self, y: u64, cstart: u64, cend: u64) -> Result<(), ScratchFileError> {
        if y >= self.height {
            return Err(ScratchFileError::OutOfBounds {
                detail: format!("row {y} >= height {}", self.height),
            });
        }
        if cend > self.width {
            return Err(ScratchFileError::OutOfBounds {
                detail: format!("cend {cend} > width {}", self.width),
            });
        }
        if cstart > cend {
            return Err(ScratchFileError::OutOfBounds {
                detail: format!("cstart {cstart} > cend {cend}"),
            });
        }
        Ok(())
    }

    fn io_err(&self, doing: &'static str, e: io::Error) -> ScratchFileError {
        ScratchFileError::Io(PersistenceIoError::new(&self.path, doing, e))
    }
}

/// Compute the byte range [start, end) within a row that holds any bit
/// for the cell column range [cstart, cend).
fn byte_range_for_cells(cstart: u64, cend: u64) -> (u64, u64) {
    let start = cstart / 4;
    let end = cend.div_ceil(4);
    (start, end)
}

fn encode_cell(state: CellState) -> u8 {
    match state {
        CellState::Dead => CODE_DEAD,
        CellState::Alive => CODE_ALIVE,
        CellState::Dying => CODE_DYING,
        CellState::Resurrecting => CODE_RESURRECTING,
    }
}

fn decode_cell(bits: u8) -> CellState {
    match bits & 0b11 {
        CODE_DEAD => CellState::Dead,
        CODE_ALIVE => CellState::Alive,
        CODE_DYING => CellState::Dying,
        CODE_RESURRECTING => CellState::Resurrecting,
        // The mask guarantees only the low 2 bits, so all 4 values are covered.
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub enum ScratchFileError {
    Io(PersistenceIoError),
    AlreadyExists { path: PathBuf },
    BadMagic { path: PathBuf },
    UnsupportedSchemaVersion { path: PathBuf, version: u32 },
    InvalidHeader { path: PathBuf, detail: String },
    OutOfBounds { detail: String },
}

impl std::fmt::Display for ScratchFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScratchFileError::Io(e) => write!(f, "{e}"),
            ScratchFileError::AlreadyExists { path } => write!(
                f,
                "Refusing to overwrite existing scratch file '{}'.",
                path.display()
            ),
            ScratchFileError::BadMagic { path } => write!(
                f,
                "Scratch file '{}' does not begin with the expected magic header.",
                path.display()
            ),
            ScratchFileError::UnsupportedSchemaVersion { path, version } => write!(
                f,
                "Scratch file '{}' has unsupported schema version {version}; this tool supports version {SCRATCH_SCHEMA_VERSION}.",
                path.display()
            ),
            ScratchFileError::InvalidHeader { path, detail } => write!(
                f,
                "Scratch file '{}' has an invalid header: {detail}.",
                path.display()
            ),
            ScratchFileError::OutOfBounds { detail } => write!(
                f,
                "Scratch file row-range access out of bounds: {detail}."
            ),
        }
    }
}

impl std::error::Error for ScratchFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScratchFileError::Io(e) => Some(e),
            _ => None,
        }
    }
}
