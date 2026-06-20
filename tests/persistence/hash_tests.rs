//! Unit tests for `game_of_life::persistence::hash`.
//!
//! Tests live in `tests/` (not inline `mod tests {}`) so they exercise only
//! the public API — implementation details are free to refactor without
//! disturbing the test suite. See AGENTS.md > Test Style.

use game_of_life::persistence::{fnv1a_64, format_hash, parse_hash, HashParseError};

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
fn negative_parse_hash_rejects_empty() {
    assert!(matches!(parse_hash(""), Err(HashParseError::Empty)));
    assert!(matches!(parse_hash("0x"), Err(HashParseError::Empty)));
}

#[test]
fn negative_parse_hash_rejects_too_long() {
    assert!(matches!(
        parse_hash("0x123456789012345678"),
        Err(HashParseError::TooLong { .. })
    ));
}

#[test]
fn negative_parse_hash_rejects_non_hex_chars() {
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
