//! UUID v4 generation and parsing for run identifiers.
//!
//! Zero external dependencies: we generate a fresh 128-bit value from the
//! existing RNG-seed helper (which mixes hashed system time + process ID) and
//! then set the RFC 4122 version and variant bits.
//!
//! The wire format is the canonical 8-4-4-4-12 lowercase hex with hyphens,
//! e.g. `7b3a1f0c-4d2e-4a51-9c5e-2f8c3a1b9d77`.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hasher};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// A 128-bit run identifier, formatted as a UUID v4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId([u8; 16]);

impl RunId {
    /// Generates a fresh UUID v4 from system entropy.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        fill_with_pseudo_random_bytes(&mut bytes);

        // Set RFC 4122 version (4) and variant (10xx) bits.
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Returns the first 8 hex characters, used for human-readable filename
    /// prefixes.
    pub fn short(&self) -> String {
        short_run_id(self)
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_run_id(self))
    }
}

/// Formats a `RunId` as canonical lowercase 8-4-4-4-12 hex.
pub fn format_run_id(id: &RunId) -> String {
    let b = &id.0;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3],
        b[4], b[5],
        b[6], b[7],
        b[8], b[9],
        b[10], b[11], b[12], b[13], b[14], b[15],
    )
}

/// Returns the first 8 hex characters of the canonical form.
pub fn short_run_id(id: &RunId) -> String {
    let s = format_run_id(id);
    s[..8].to_string()
}

/// Parses a canonical 8-4-4-4-12 lowercase or uppercase hex UUID into a `RunId`.
///
/// We accept any UUID-shaped value; we do not enforce v4 version/variant bits
/// on parse so that round-trips with arbitrary correctly-formatted IDs work.
pub fn parse_run_id(value: &str) -> Result<RunId, RunIdParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RunIdParseError::Empty);
    }
    if trimmed.len() != 36 {
        return Err(RunIdParseError::WrongLength {
            value: value.to_string(),
            actual: trimmed.len(),
        });
    }
    let bytes = trimmed.as_bytes();
    for &index in &[8usize, 13, 18, 23] {
        if bytes[index] != b'-' {
            return Err(RunIdParseError::MissingHyphen {
                value: value.to_string(),
                position: index,
            });
        }
    }
    let mut out = [0u8; 16];
    let positions = [
        (0, 0),
        (1, 2),
        (2, 4),
        (3, 6),
        (4, 9),
        (5, 11),
        (6, 14),
        (7, 16),
        (8, 19),
        (9, 21),
        (10, 24),
        (11, 26),
        (12, 28),
        (13, 30),
        (14, 32),
        (15, 34),
    ];
    for &(out_index, str_index) in &positions {
        let hi = decode_hex_nibble(bytes[str_index]).ok_or_else(|| RunIdParseError::NonHex {
            value: value.to_string(),
            position: str_index,
        })?;
        let lo =
            decode_hex_nibble(bytes[str_index + 1]).ok_or_else(|| RunIdParseError::NonHex {
                value: value.to_string(),
                position: str_index + 1,
            })?;
        out[out_index] = (hi << 4) | lo;
    }
    Ok(RunId(out))
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunIdParseError {
    Empty,
    WrongLength { value: String, actual: usize },
    MissingHyphen { value: String, position: usize },
    NonHex { value: String, position: usize },
}

impl fmt::Display for RunIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunIdParseError::Empty => write!(f, "run id is empty"),
            RunIdParseError::WrongLength { value, actual } => write!(
                f,
                "run id '{value}' has {actual} characters; expected 36 in the form xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
            ),
            RunIdParseError::MissingHyphen { value, position } => write!(
                f,
                "run id '{value}' is missing a hyphen at position {position}"
            ),
            RunIdParseError::NonHex { value, position } => write!(
                f,
                "run id '{value}' has a non-hex character at position {position}"
            ),
        }
    }
}

impl std::error::Error for RunIdParseError {}

fn fill_with_pseudo_random_bytes(buffer: &mut [u8]) {
    // Mix three independent entropy sources: system time, process id, and
    // RandomState (which is OS-seeded). We hash each into a 64-bit word with a
    // fresh RandomState to spread the bits evenly across the output.
    let mut chunks = buffer.chunks_mut(8);
    let mut counter: u64 = 0;
    for chunk in chunks.by_ref() {
        let mut hasher = RandomState::new().build_hasher();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        hasher.write_u128(nanos);
        hasher.write_u32(process::id());
        hasher.write_u64(counter);
        counter = counter.wrapping_add(1);
        let word = hasher.finish().to_le_bytes();
        for (dst, src) in chunk.iter_mut().zip(word.iter()) {
            *dst = *src;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sets_v4_version_and_variant_bits() {
        let id = RunId::generate();
        // version nibble is at byte 6 high-nibble; must be 4.
        assert_eq!(id.0[6] >> 4, 4);
        // variant bits at byte 8 high-nibble must start with 10xx.
        assert_eq!(id.0[8] & 0xc0, 0x80);
    }

    #[test]
    fn generate_produces_distinct_ids() {
        let a = RunId::generate();
        let b = RunId::generate();
        assert_ne!(a, b, "two independently generated ids should differ");
    }

    #[test]
    fn format_then_parse_roundtrips() {
        let id = RunId::generate();
        let s = format_run_id(&id);
        let parsed = parse_run_id(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn format_is_canonical_8_4_4_4_12() {
        let id = RunId::from_bytes([
            0x7b, 0x3a, 0x1f, 0x0c, 0x4d, 0x2e, 0x4a, 0x51, 0x9c, 0x5e, 0x2f, 0x8c, 0x3a, 0x1b,
            0x9d, 0x77,
        ]);
        assert_eq!(format_run_id(&id), "7b3a1f0c-4d2e-4a51-9c5e-2f8c3a1b9d77");
    }

    #[test]
    fn short_returns_eight_hex_chars() {
        let id = RunId::from_bytes([
            0x7b, 0x3a, 0x1f, 0x0c, 0x4d, 0x2e, 0x4a, 0x51, 0x9c, 0x5e, 0x2f, 0x8c, 0x3a, 0x1b,
            0x9d, 0x77,
        ]);
        assert_eq!(id.short(), "7b3a1f0c");
    }

    #[test]
    fn parse_accepts_uppercase() {
        let id = parse_run_id("7B3A1F0C-4D2E-4A51-9C5E-2F8C3A1B9D77").unwrap();
        assert_eq!(format_run_id(&id), "7b3a1f0c-4d2e-4a51-9c5e-2f8c3a1b9d77");
    }

    #[test]
    fn parse_rejects_wrong_length() {
        assert!(matches!(
            parse_run_id("too-short"),
            Err(RunIdParseError::WrongLength { .. })
        ));
    }

    #[test]
    fn parse_rejects_missing_hyphen() {
        // 36 chars but hyphen replaced with x at position 8.
        assert!(matches!(
            parse_run_id("7b3a1f0cx4d2e-4a51-9c5e-2f8c3a1b9d77"),
            Err(RunIdParseError::MissingHyphen { .. })
        ));
    }

    #[test]
    fn parse_rejects_non_hex() {
        assert!(matches!(
            parse_run_id("zzzzzzzz-4d2e-4a51-9c5e-2f8c3a1b9d77"),
            Err(RunIdParseError::NonHex { .. })
        ));
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(parse_run_id(""), Err(RunIdParseError::Empty)));
    }
}
