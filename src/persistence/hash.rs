//! FNV-1a 64-bit hash used by the persistence layer.
//!
//! This is a non-cryptographic hash. We use it to detect accidental edits,
//! partial writes, and bit flips in run record files. It is also reused as a
//! building block for future cycle-detection work (see `docs/design.md`).
//!
//! We do **not** use this for adversarial tamper detection. If we ever need
//! that, it's a separate signing PR.

const FNV1A_64_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV1A_64_PRIME: u64 = 0x100_0000_01b3;

pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV1A_64_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV1A_64_PRIME);
    }
    hash
}

/// Formats a 64-bit hash as the canonical `0x{16-hex-chars}` form used in
/// run record files.
pub fn format_hash(hash: u64) -> String {
    format!("0x{hash:016x}")
}

/// Parses a hash string in either `0x...` or bare-hex form. The hash must
/// represent at most 64 bits.
pub fn parse_hash(value: &str) -> Result<u64, HashParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HashParseError::Empty);
    }
    let hex_part = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    if hex_part.is_empty() {
        return Err(HashParseError::Empty);
    }
    if hex_part.len() > 16 {
        return Err(HashParseError::TooLong {
            value: value.to_string(),
        });
    }
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(HashParseError::NonHex {
            value: value.to_string(),
        });
    }
    u64::from_str_radix(hex_part, 16).map_err(|_| HashParseError::NonHex {
        value: value.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashParseError {
    Empty,
    TooLong { value: String },
    NonHex { value: String },
}

impl std::fmt::Display for HashParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashParseError::Empty => write!(f, "hash value is empty"),
            HashParseError::TooLong { value } => write!(
                f,
                "hash value '{value}' is too long; expected up to 16 hex digits"
            ),
            HashParseError::NonHex { value } => write!(
                f,
                "hash value '{value}' contains non-hex characters; use 16 hex digits, optionally prefixed with 0x"
            ),
        }
    }
}

impl std::error::Error for HashParseError {}
