//! UTC ISO-8601 timestamp formatting and parsing.
//!
//! Format: `YYYY-MM-DDTHH:MM:SSZ`. UTC only — no timezone support.
//!
//! Hand-rolled to avoid adding a dependency on `chrono`/`time`. Conversion
//! from `std::time::SystemTime` uses the standard Gregorian day-number
//! algorithm.

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Formats a `SystemTime` as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_utc(time: SystemTime) -> String {
    let seconds_since_epoch = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_seconds_since_epoch(seconds_since_epoch)
}

/// Parses a `YYYY-MM-DDTHH:MM:SSZ` timestamp into a `SystemTime`.
pub fn parse_utc(value: &str) -> Result<SystemTime, TimestampParseError> {
    let trimmed = value.trim();
    if trimmed.len() != 20 {
        return Err(TimestampParseError::WrongLength {
            value: value.to_string(),
            actual: trimmed.len(),
        });
    }
    let bytes = trimmed.as_bytes();
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(TimestampParseError::MissingDateSeparator {
            value: value.to_string(),
        });
    }
    if bytes[10] != b'T' {
        return Err(TimestampParseError::MissingDateTimeSeparator {
            value: value.to_string(),
        });
    }
    if bytes[13] != b':' || bytes[16] != b':' {
        return Err(TimestampParseError::MissingTimeSeparator {
            value: value.to_string(),
        });
    }
    if bytes[19] != b'Z' {
        return Err(TimestampParseError::MissingZuluSuffix {
            value: value.to_string(),
        });
    }

    let year = parse_int(&trimmed[0..4], "year", value)?;
    let month = parse_int(&trimmed[5..7], "month", value)?;
    let day = parse_int(&trimmed[8..10], "day", value)?;
    let hour = parse_int(&trimmed[11..13], "hour", value)?;
    let minute = parse_int(&trimmed[14..16], "minute", value)?;
    let second = parse_int(&trimmed[17..19], "second", value)?;

    if !(1..=12).contains(&month) {
        return Err(TimestampParseError::FieldOutOfRange {
            value: value.to_string(),
            field: "month",
        });
    }
    let days_in_month = days_in_month(year, month as u32);
    if day < 1 || day > days_in_month as i64 {
        return Err(TimestampParseError::FieldOutOfRange {
            value: value.to_string(),
            field: "day",
        });
    }
    if hour > 23 {
        return Err(TimestampParseError::FieldOutOfRange {
            value: value.to_string(),
            field: "hour",
        });
    }
    if minute > 59 {
        return Err(TimestampParseError::FieldOutOfRange {
            value: value.to_string(),
            field: "minute",
        });
    }
    if second > 59 {
        return Err(TimestampParseError::FieldOutOfRange {
            value: value.to_string(),
            field: "second",
        });
    }

    let seconds_since_epoch =
        days_from_civil(year, month as u32, day as u32) * 86_400 + hour * 3600 + minute * 60 + second;
    if seconds_since_epoch < 0 {
        return Err(TimestampParseError::BeforeUnixEpoch {
            value: value.to_string(),
        });
    }
    Ok(UNIX_EPOCH + Duration::from_secs(seconds_since_epoch as u64))
}

fn parse_int(s: &str, field: &'static str, original: &str) -> Result<i64, TimestampParseError> {
    if !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(TimestampParseError::NonNumericField {
            value: original.to_string(),
            field,
        });
    }
    s.parse::<i64>()
        .map_err(|_| TimestampParseError::FieldOutOfRange {
            value: original.to_string(),
            field,
        })
}

/// Errors when parsing a UTC timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimestampParseError {
    WrongLength {
        value: String,
        actual: usize,
    },
    MissingDateSeparator {
        value: String,
    },
    MissingDateTimeSeparator {
        value: String,
    },
    MissingTimeSeparator {
        value: String,
    },
    MissingZuluSuffix {
        value: String,
    },
    NonNumericField {
        value: String,
        field: &'static str,
    },
    FieldOutOfRange {
        value: String,
        field: &'static str,
    },
    BeforeUnixEpoch {
        value: String,
    },
}

impl fmt::Display for TimestampParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimestampParseError::WrongLength { value, actual } => write!(
                f,
                "timestamp '{value}' has {actual} characters; expected 20 in the form YYYY-MM-DDTHH:MM:SSZ"
            ),
            TimestampParseError::MissingDateSeparator { value } => write!(
                f,
                "timestamp '{value}' is missing a '-' between date parts (expected YYYY-MM-DDTHH:MM:SSZ)"
            ),
            TimestampParseError::MissingDateTimeSeparator { value } => write!(
                f,
                "timestamp '{value}' is missing the 'T' between date and time (expected YYYY-MM-DDTHH:MM:SSZ)"
            ),
            TimestampParseError::MissingTimeSeparator { value } => write!(
                f,
                "timestamp '{value}' is missing a ':' between time parts (expected YYYY-MM-DDTHH:MM:SSZ)"
            ),
            TimestampParseError::MissingZuluSuffix { value } => write!(
                f,
                "timestamp '{value}' is missing the trailing 'Z'; UTC-only timestamps are required (YYYY-MM-DDTHH:MM:SSZ)"
            ),
            TimestampParseError::NonNumericField { value, field } => write!(
                f,
                "timestamp '{value}' has non-numeric {field}"
            ),
            TimestampParseError::FieldOutOfRange { value, field } => write!(
                f,
                "timestamp '{value}' has out-of-range {field}"
            ),
            TimestampParseError::BeforeUnixEpoch { value } => write!(
                f,
                "timestamp '{value}' is before the Unix epoch (1970-01-01T00:00:00Z); not supported"
            ),
        }
    }
}

impl std::error::Error for TimestampParseError {}

fn format_seconds_since_epoch(seconds: i64) -> String {
    let total_days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(total_days);
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

// Reference: Howard Hinnant's date algorithms.
// http://howardhinnant.github.io/date_algorithms.html
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u64; // [0, 399]
    let m = month as i64;
    let d = day as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe as i64 * 365 + (yoe as i64 / 4) - (yoe as i64 / 100) + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
