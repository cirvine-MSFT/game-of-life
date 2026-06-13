//! Shared low-level I/O error type for persistence operations.
//!
//! Higher-level readers/writers wrap this in their own typed errors so the
//! caller always knows which operation produced the failure.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// An I/O failure that occurred while opening, reading, or writing a
/// persistence file.
#[derive(Debug)]
pub struct PersistenceIoError {
    pub path: PathBuf,
    pub operation: &'static str,
    pub source: io::Error,
}

impl PersistenceIoError {
    pub fn new(path: impl Into<PathBuf>, operation: &'static str, source: io::Error) -> Self {
        Self {
            path: path.into(),
            operation,
            source,
        }
    }
}

impl fmt::Display for PersistenceIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "I/O error while {} '{}': {}",
            self.operation,
            self.path.display(),
            self.source
        )
    }
}

impl std::error::Error for PersistenceIoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
