//! Unit tests for `game_of_life::persistence::timestamps`.

use std::time::{Duration, UNIX_EPOCH};

use game_of_life::persistence::{format_utc, parse_utc, TimestampParseError};

#[test]
fn format_unix_epoch_is_known_value() {
    assert_eq!(format_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
}

#[test]
fn format_known_timestamp() {
    let when = UNIX_EPOCH + Duration::from_secs(1_780_000_000);
    // Computed via independent reference: 1_780_000_000 seconds after epoch
    // = 2026-05-28T20:26:40Z.
    assert_eq!(format_utc(when), "2026-05-28T20:26:40Z");
}

#[test]
fn parse_then_format_roundtrips() {
    let inputs = [
        "1970-01-01T00:00:00Z",
        "2000-02-29T12:34:56Z", // leap day
        "2024-02-29T00:00:00Z", // leap year
        "2026-06-12T22:55:20Z",
        "2099-12-31T23:59:59Z",
    ];
    for input in inputs {
        let parsed = parse_utc(input).expect(input);
        let reformatted = format_utc(parsed);
        assert_eq!(reformatted, input, "roundtrip failed for {input}");
    }
}

#[test]
fn negative_parse_missing_z() {
    assert!(matches!(
        parse_utc("2026-06-12T22:55:20"),
        Err(TimestampParseError::WrongLength { .. })
    ));
}

#[test]
fn negative_parse_wrong_separator() {
    assert!(matches!(
        parse_utc("2026/06/12T22:55:20Z"),
        Err(TimestampParseError::MissingDateSeparator { .. })
    ));
}

#[test]
fn negative_parse_missing_t_separator() {
    assert!(matches!(
        parse_utc("2026-06-12 22:55:20Z"),
        Err(TimestampParseError::MissingDateTimeSeparator { .. })
    ));
}

#[test]
fn negative_parse_out_of_range_month() {
    assert!(matches!(
        parse_utc("2026-13-01T00:00:00Z"),
        Err(TimestampParseError::FieldOutOfRange { field: "month", .. })
    ));
}

#[test]
fn negative_parse_out_of_range_day() {
    assert!(matches!(
        parse_utc("2025-02-29T00:00:00Z"),
        Err(TimestampParseError::FieldOutOfRange { field: "day", .. })
    ));
}

#[test]
fn negative_parse_non_numeric_field() {
    assert!(matches!(
        parse_utc("2026-AB-12T00:00:00Z"),
        Err(TimestampParseError::NonNumericField { field: "month", .. })
    ));
}
