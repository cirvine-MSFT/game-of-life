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

/// Computes the FNV-1a 64-bit hash of the given bytes.
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

#[cfg(test)]
mod tests {
    use super::*;

    // Published FNV-1a 64-bit test vectors.
    // Source: http://www.isthe.com/chongo/tech/comp/fnv/index.html
    #[test]
    fn fnv1a_64_empty_input() {
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn fnv1a_64_a() {
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn fnv1a_64_foobar() {
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn fnv1a_64_is_idempotent_on_same_input() {
        let payload = b"some board grid bytes here";
        assert_eq!(fnv1a_64(payload), fnv1a_64(payload));
    }

    #[test]
    fn fnv1a_64_is_sensitive_to_single_bit_change() {
        let a = fnv1a_64(b"hello world");
        let b = fnv1a_64(b"Hello world");
        assert_ne!(a, b);
    }

    #[test]
    fn format_hash_pads_to_sixteen_hex_chars() {
        assert_eq!(format_hash(0), "0x0000000000000000");
        assert_eq!(format_hash(0x123), "0x0000000000000123");
        assert_eq!(format_hash(u64::MAX), "0xffffffffffffffff");
    }

    #[test]
    fn parse_hash_accepts_prefixed_and_bare_hex() {
        assert_eq!(
            parse_hash("0x9f2b1c4e7a5d3088").unwrap(),
            0x9f2b_1c4e_7a5d_3088
        );
        assert_eq!(
            parse_hash("0X9f2b1c4e7a5d3088").unwrap(),
            0x9f2b_1c4e_7a5d_3088
        );
        assert_eq!(
            parse_hash("9f2b1c4e7a5d3088").unwrap(),
            0x9f2b_1c4e_7a5d_3088
        );
    }

    #[test]
    fn parse_hash_accepts_short_values() {
        assert_eq!(parse_hash("0x1").unwrap(), 1);
        assert_eq!(parse_hash("ff").unwrap(), 0xff);
    }

    #[test]
    fn parse_hash_rejects_empty() {
        assert!(matches!(parse_hash(""), Err(HashParseError::Empty)));
        assert!(matches!(parse_hash("0x"), Err(HashParseError::Empty)));
    }

    #[test]
    fn parse_hash_rejects_too_long() {
        assert!(matches!(
            parse_hash("0x123456789012345678"),
            Err(HashParseError::TooLong { .. })
        ));
    }

    #[test]
    fn parse_hash_rejects_non_hex_chars() {
        assert!(matches!(
            parse_hash("0xZZZZ"),
            Err(HashParseError::NonHex { .. })
        ));
        assert!(matches!(
            parse_hash("0x9f2b 1c4e"),
            Err(HashParseError::NonHex { .. })
        ));
    }

    #[test]
    fn format_then_parse_roundtrips() {
        for value in [0u64, 1, 0xabc, 0xdead_beef_cafe_babe, u64::MAX] {
            assert_eq!(parse_hash(&format_hash(value)).unwrap(), value);
        }
    }
}
