//! File-type identification via a magic prefix on the first line.
//!
//! "Magic" is the standard Unix-derived term for a short, fixed marker at the
//! start of a file that identifies its format. See `file(1)` / `libmagic`.
//! Same idea as `%PDF-` or `#!/usr/bin/env` shebangs.
//!
//! Both Game of Life persistence file types begin with a single-line magic:
//!
//! - `GOL-RUN-RECORD v1|v2`  → a full run record
//! - `GOL-BOARD-SNAPSHOT v1` → a standalone board snapshot
//!
//! Sniffing is bounded to the first 128 bytes (or the first newline, whichever
//! comes first) so the operation is cheap and safe to run on huge files.

use std::fmt;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use super::errors::PersistenceIoError;
use super::MAX_MAGIC_PEEK_BYTES;

pub const RUN_RECORD_SCHEMA_VERSION: u32 = 2;
pub const BOARD_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

pub const RUN_RECORD_MAGIC_V1: &str = "GOL-RUN-RECORD v1";
pub const RUN_RECORD_MAGIC_V2: &str = "GOL-RUN-RECORD v2";
pub const RUN_RECORD_MAGIC: &str = RUN_RECORD_MAGIC_V2;

/// Magic line for board snapshot files.
pub const BOARD_SNAPSHOT_MAGIC: &str = "GOL-BOARD-SNAPSHOT v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunRecordVersion {
    V1,
    V2,
}

impl RunRecordVersion {
    pub fn magic(self) -> &'static str {
        match self {
            RunRecordVersion::V1 => RUN_RECORD_MAGIC_V1,
            RunRecordVersion::V2 => RUN_RECORD_MAGIC_V2,
        }
    }

    pub fn schema_version(self) -> u32 {
        match self {
            RunRecordVersion::V1 => 1,
            RunRecordVersion::V2 => RUN_RECORD_SCHEMA_VERSION,
        }
    }
}

/// Identifies which kind of persistence file we're looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    RunRecord,
    BoardSnapshot,
}

impl FileKind {
    pub fn magic(self) -> &'static str {
        match self {
            FileKind::RunRecord => RUN_RECORD_MAGIC,
            FileKind::BoardSnapshot => BOARD_SNAPSHOT_MAGIC,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            FileKind::RunRecord => "run record",
            FileKind::BoardSnapshot => "board snapshot",
        }
    }
}

const ACCEPTED_MAGIC_DISPLAY: &str =
    "`GOL-RUN-RECORD v1`, `GOL-RUN-RECORD v2`, or `GOL-BOARD-SNAPSHOT v1`";

impl fmt::Display for FileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Failure modes when sniffing the magic of a file.
#[derive(Debug)]
pub enum MagicError {
    Io(PersistenceIoError),
    EmptyFile {
        path: PathBuf,
    },
    UnknownMagic {
        path: PathBuf,
        found: String,
    },
    /// The first line was too long to be a valid magic (we cap the sniff at
    /// `MAX_MAGIC_PEEK_BYTES` bytes before giving up).
    OversizedFirstLine {
        path: PathBuf,
        scanned_bytes: usize,
    },
}

impl MagicError {
    pub fn path(&self) -> &Path {
        match self {
            MagicError::Io(e) => &e.path,
            MagicError::EmptyFile { path }
            | MagicError::UnknownMagic { path, .. }
            | MagicError::OversizedFirstLine { path, .. } => path,
        }
    }
}

impl fmt::Display for MagicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MagicError::Io(e) => write!(f, "{e}"),
            MagicError::EmptyFile { path } => write!(
                f,
                "File '{}' is empty; not a Game of Life file (expected first line {ACCEPTED_MAGIC_DISPLAY}).",
                path.display()
            ),
            MagicError::UnknownMagic { path, found } => write!(
                f,
                "File '{}' is not a Game of Life file: first line was `{}`, expected {ACCEPTED_MAGIC_DISPLAY}.",
                path.display(),
                truncate_for_display(found, 64)
            ),
            MagicError::OversizedFirstLine { path, scanned_bytes } => write!(
                f,
                "File '{}' is not a Game of Life file: no newline found in the first {scanned_bytes} bytes; expected first line {ACCEPTED_MAGIC_DISPLAY}.",
                path.display()
            ),
        }
    }
}

impl std::error::Error for MagicError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MagicError::Io(e) => Some(e),
            _ => None,
        }
    }
}

fn truncate_for_display(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max_chars).collect();
    out.push('…');
    out
}

/// Reads the magic line from a file on disk and returns the identified
/// `FileKind`.
///
/// This opens the file, reads at most `MAX_MAGIC_PEEK_BYTES` bytes, and
/// returns. It does not validate any content beyond the magic line.
pub fn sniff_file_kind(path: impl AsRef<Path>) -> Result<FileKind, MagicError> {
    let path = path.as_ref();
    let file = File::open(path)
        .map_err(|e| MagicError::Io(PersistenceIoError::new(path, "opening file for sniff", e)))?;
    let mut reader = BufReader::new(file);
    sniff_from_reader(path, &mut reader)
}

/// Keep `FileKind` unchanged so existing callers that only branch on broad file
/// type do not need churn; run-record readers opt into this second, cheap sniff.
pub fn sniff_run_record_version(path: impl AsRef<Path>) -> Result<RunRecordVersion, MagicError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|e| {
        MagicError::Io(PersistenceIoError::new(
            path,
            "opening file for run-record version sniff",
            e,
        ))
    })?;
    let mut reader = BufReader::new(file);
    sniff_run_record_version_from_reader(path, &mut reader)
}

/// Reads the magic line from an arbitrary reader. The reader is positioned
/// after the magic line on success; callers who want to re-read the file from
/// the start (e.g. a full parser) should open a fresh reader.
pub fn sniff_from_reader<R: Read>(
    path: impl Into<PathBuf>,
    reader: &mut R,
) -> Result<FileKind, MagicError> {
    let path = path.into();
    let mut buffer = [0u8; MAX_MAGIC_PEEK_BYTES];
    let mut filled = 0;

    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                return Err(MagicError::Io(PersistenceIoError::new(
                    &path,
                    "reading file for sniff",
                    e,
                )));
            }
        }
    }

    if filled == 0 {
        return Err(MagicError::EmptyFile { path });
    }

    let peeked = &buffer[..filled];
    let line_end = peeked.iter().position(|&b| b == b'\n');
    let line_bytes = match line_end {
        Some(end) => &peeked[..end],
        None if filled < buffer.len() => peeked,
        None => {
            return Err(MagicError::OversizedFirstLine {
                path,
                scanned_bytes: buffer.len(),
            });
        }
    };

    // Strip a trailing CR for tolerance on CRLF inputs.
    let line_bytes = strip_trailing_cr(line_bytes);
    let line = std::str::from_utf8(line_bytes).map_err(|_| MagicError::UnknownMagic {
        path: path.clone(),
        found: format!("<{} non-UTF-8 bytes>", line_bytes.len()),
    })?;
    let trimmed = line.trim();

    if run_record_version_from_magic(trimmed).is_some() {
        Ok(FileKind::RunRecord)
    } else if trimmed == BOARD_SNAPSHOT_MAGIC {
        Ok(FileKind::BoardSnapshot)
    } else if trimmed.is_empty() {
        Err(MagicError::EmptyFile { path })
    } else {
        Err(MagicError::UnknownMagic {
            path,
            found: trimmed.to_string(),
        })
    }
}

pub(crate) fn sniff_run_record_version_from_reader<R: Read>(
    path: impl Into<PathBuf>,
    reader: &mut R,
) -> Result<RunRecordVersion, MagicError> {
    let path = path.into();
    let line = read_magic_line(path.clone(), reader)?;
    let trimmed = line.trim();
    if let Some(version) = run_record_version_from_magic(trimmed) {
        Ok(version)
    } else if trimmed.is_empty() {
        Err(MagicError::EmptyFile { path })
    } else {
        Err(MagicError::UnknownMagic {
            path,
            found: trimmed.to_string(),
        })
    }
}

pub(crate) fn run_record_version_from_magic(line: &str) -> Option<RunRecordVersion> {
    match line {
        RUN_RECORD_MAGIC_V1 => Some(RunRecordVersion::V1),
        RUN_RECORD_MAGIC_V2 => Some(RunRecordVersion::V2),
        _ => None,
    }
}

fn read_magic_line<R: Read>(path: PathBuf, reader: &mut R) -> Result<String, MagicError> {
    let mut buffer = [0u8; MAX_MAGIC_PEEK_BYTES];
    let mut filled = 0;

    while filled < buffer.len() {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                return Err(MagicError::Io(PersistenceIoError::new(
                    &path,
                    "reading file for sniff",
                    e,
                )));
            }
        }
    }

    if filled == 0 {
        return Err(MagicError::EmptyFile { path });
    }

    let peeked = &buffer[..filled];
    let line_end = peeked.iter().position(|&b| b == b'\n');
    let line_bytes = match line_end {
        Some(end) => &peeked[..end],
        None if filled < buffer.len() => peeked,
        None => {
            return Err(MagicError::OversizedFirstLine {
                path,
                scanned_bytes: buffer.len(),
            });
        }
    };

    let line_bytes = strip_trailing_cr(line_bytes);
    std::str::from_utf8(line_bytes)
        .map(str::to_string)
        .map_err(|_| MagicError::UnknownMagic {
            path,
            found: format!("<{} non-UTF-8 bytes>", line_bytes.len()),
        })
}

fn strip_trailing_cr(bytes: &[u8]) -> &[u8] {
    if bytes.last() == Some(&b'\r') {
        &bytes[..bytes.len() - 1]
    } else {
        bytes
    }
}
