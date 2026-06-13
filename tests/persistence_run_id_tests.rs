//! Unit tests for `game_of_life::persistence::run_id`.

use game_of_life::persistence::{format_run_id, parse_run_id, RunId, RunIdParseError};

#[test]
fn generate_sets_v4_version_and_variant_bits() {
    let id = RunId::generate();
    let bytes = id.as_bytes();
    // version nibble is at byte 6 high-nibble; must be 4.
    assert_eq!(bytes[6] >> 4, 4);
    // variant bits at byte 8 high-nibble must start with 10xx.
    assert_eq!(bytes[8] & 0xc0, 0x80);
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
        0x7b, 0x3a, 0x1f, 0x0c, 0x4d, 0x2e, 0x4a, 0x51, 0x9c, 0x5e, 0x2f, 0x8c, 0x3a, 0x1b, 0x9d,
        0x77,
    ]);
    assert_eq!(format_run_id(&id), "7b3a1f0c-4d2e-4a51-9c5e-2f8c3a1b9d77");
}

#[test]
fn short_returns_eight_hex_chars() {
    let id = RunId::from_bytes([
        0x7b, 0x3a, 0x1f, 0x0c, 0x4d, 0x2e, 0x4a, 0x51, 0x9c, 0x5e, 0x2f, 0x8c, 0x3a, 0x1b, 0x9d,
        0x77,
    ]);
    assert_eq!(id.short(), "7b3a1f0c");
}

#[test]
fn parse_accepts_uppercase() {
    let id = parse_run_id("7B3A1F0C-4D2E-4A51-9C5E-2F8C3A1B9D77").unwrap();
    assert_eq!(format_run_id(&id), "7b3a1f0c-4d2e-4a51-9c5e-2f8c3a1b9d77");
}

#[test]
fn negative_parse_rejects_wrong_length() {
    assert!(matches!(
        parse_run_id("too-short"),
        Err(RunIdParseError::WrongLength { .. })
    ));
}

#[test]
fn negative_parse_rejects_missing_hyphen() {
    // 36 chars but hyphen replaced with x at position 8.
    assert!(matches!(
        parse_run_id("7b3a1f0cx4d2e-4a51-9c5e-2f8c3a1b9d77"),
        Err(RunIdParseError::MissingHyphen { .. })
    ));
}

#[test]
fn negative_parse_rejects_non_hex() {
    assert!(matches!(
        parse_run_id("zzzzzzzz-4d2e-4a51-9c5e-2f8c3a1b9d77"),
        Err(RunIdParseError::NonHex { .. })
    ));
}

#[test]
fn negative_parse_rejects_empty() {
    assert!(matches!(parse_run_id(""), Err(RunIdParseError::Empty)));
}
