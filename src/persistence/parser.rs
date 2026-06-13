//! Shared low-level parsing primitives for both run records and board snapshots.
//!
//! The format is line-oriented. This module owns the canonical normalization
//! rules, the `ParseLocation` type used by error messages, and a small set of
//! helpers for matching section headers (`[name]`), fences (`----- BEGIN X -----`
//! / `----- END X -----`), and key/value field lines (`key: value`).

use std::fmt;

/// One-based line number within the file, plus the file path for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLocation {
    pub path: String,
    pub line_number: usize,
}

impl ParseLocation {
    pub fn new(path: impl Into<String>, line_number: usize) -> Self {
        Self {
            path: path.into(),
            line_number,
        }
    }
}

impl fmt::Display for ParseLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.path, self.line_number)
    }
}

/// Errors that the shared parser primitives can return. Higher-level readers
/// wrap these into their own typed error enums so the user always knows which
/// part of which file type failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    UnexpectedEnd {
        location: ParseLocation,
        expected: String,
    },
    MalformedFieldLine {
        location: ParseLocation,
        line: String,
    },
    DuplicateField {
        location: ParseLocation,
        field: String,
    },
    MissingRequiredField {
        section: String,
        field: String,
    },
    MalformedSectionHeader {
        location: ParseLocation,
        line: String,
    },
    UnexpectedSection {
        location: ParseLocation,
        section: String,
    },
    MalformedFence {
        location: ParseLocation,
        line: String,
    },
    UnexpectedFenceLabel {
        location: ParseLocation,
        expected: String,
        actual: String,
    },
    UnclosedFence {
        location: ParseLocation,
        expected_end_label: String,
    },
    NonAsciiBoardCharacter {
        location: ParseLocation,
        character: char,
    },
    UnknownBoardCharacter {
        location: ParseLocation,
        character: char,
    },
    RaggedBoardRow {
        location: ParseLocation,
        expected_width: usize,
        actual_width: usize,
    },
    BoardSizeMismatch {
        location: ParseLocation,
        header_size: (usize, usize),
        grid_size: (usize, usize),
    },
    BoardCountMismatch {
        location: ParseLocation,
        field: &'static str,
        header_value: usize,
        grid_value: usize,
    },
    UnknownEncoding {
        location: ParseLocation,
        encoding: String,
    },
    UnsupportedSchemaVersion {
        location: ParseLocation,
        version: u32,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEnd { location, expected } => write!(
                f,
                "Unexpected end of file at {location}; expected {expected}."
            ),
            ParseError::MalformedFieldLine { location, line } => write!(
                f,
                "Malformed field line at {location}: '{line}'. Expected 'key: value'."
            ),
            ParseError::DuplicateField { location, field } => write!(
                f,
                "Duplicate field '{field}' at {location}."
            ),
            ParseError::MissingRequiredField { section, field } => write!(
                f,
                "Section [{section}] is missing required field '{field}'."
            ),
            ParseError::MalformedSectionHeader { location, line } => write!(
                f,
                "Malformed section header at {location}: '{line}'. Expected '[name]'."
            ),
            ParseError::UnexpectedSection { location, section } => write!(
                f,
                "Unexpected section [{section}] at {location}."
            ),
            ParseError::MalformedFence { location, line } => write!(
                f,
                "Malformed fence line at {location}: '{line}'. Expected '----- BEGIN <LABEL> -----' or '----- END <LABEL> -----'."
            ),
            ParseError::UnexpectedFenceLabel {
                location,
                expected,
                actual,
            } => write!(
                f,
                "Fence label mismatch at {location}: expected '{expected}', found '{actual}'."
            ),
            ParseError::UnclosedFence {
                location,
                expected_end_label,
            } => write!(
                f,
                "Unclosed fence at {location}; expected '----- END {expected_end_label} -----'."
            ),
            ParseError::NonAsciiBoardCharacter { location, character } => write!(
                f,
                "Board grid at {location} contains non-ASCII character '{}' (U+{:04X}).",
                character.escape_default(),
                *character as u32,
            ),
            ParseError::UnknownBoardCharacter { location, character } => write!(
                f,
                "Board grid at {location} contains unknown character '{}'; allowed characters are '.' (dead) and '#' (alive).",
                character.escape_default(),
            ),
            ParseError::RaggedBoardRow {
                location,
                expected_width,
                actual_width,
            } => write!(
                f,
                "Board row at {location} has width {actual_width}; expected {expected_width} based on the first row."
            ),
            ParseError::BoardSizeMismatch {
                location,
                header_size,
                grid_size,
            } => write!(
                f,
                "Board size mismatch at {location}: header declared {}x{} but grid is {}x{}.",
                header_size.0, header_size.1, grid_size.0, grid_size.1,
            ),
            ParseError::BoardCountMismatch {
                location,
                field,
                header_value,
                grid_value,
            } => write!(
                f,
                "Board {field} mismatch at {location}: header declared {header_value} but grid contains {grid_value}."
            ),
            ParseError::UnknownEncoding { location, encoding } => write!(
                f,
                "Unknown board encoding '{encoding}' at {location}; supported encodings: ascii."
            ),
            ParseError::UnsupportedSchemaVersion { location, version } => write!(
                f,
                "Unsupported schema version {version} at {location}; this tool supports schema version 1."
            ),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses a `key: value` line. Returns `(key, value)` with both trimmed.
///
/// Returns `Err(MalformedFieldLine)` if the line has no colon or the key is
/// empty.
pub fn parse_field_line(location: ParseLocation, line: &str) -> Result<(&str, &str), ParseError> {
    let colon = line
        .find(':')
        .ok_or_else(|| ParseError::MalformedFieldLine {
            location: location.clone(),
            line: line.to_string(),
        })?;
    let key = line[..colon].trim();
    if key.is_empty() {
        return Err(ParseError::MalformedFieldLine {
            location,
            line: line.to_string(),
        });
    }
    let value = line[colon + 1..].trim();
    Ok((key, value))
}

/// Recognizes a line of the form `[section]`. Returns `Some(section_name)`
/// on match, `None` otherwise. The returned name is trimmed but case is
/// preserved.
pub fn parse_section_header(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?;
    let inner_trimmed = inner.trim();
    if inner_trimmed.is_empty() {
        return None;
    }
    Some(inner_trimmed)
}

/// Recognizes a `----- BEGIN <LABEL> -----` line. Returns `Some(label)` on
/// match.
pub fn parse_begin_fence(line: &str) -> Option<&str> {
    parse_fence(line, "BEGIN")
}

/// Recognizes a `----- END <LABEL> -----` line. Returns `Some(label)` on
/// match.
pub fn parse_end_fence(line: &str) -> Option<&str> {
    parse_fence(line, "END")
}

fn parse_fence<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("-----")?.strip_suffix("-----")?;
    let inner = inner.trim();
    let after_keyword = inner.strip_prefix(keyword)?;
    let label = after_keyword.trim();
    if label.is_empty() {
        None
    } else {
        Some(label)
    }
}

/// Renders the canonical BEGIN fence line for a given label.
pub fn format_begin_fence(label: &str) -> String {
    format!("----- BEGIN {label} -----")
}

/// Renders the canonical END fence line for a given label.
pub fn format_end_fence(label: &str) -> String {
    format!("----- END {label} -----")
}

/// Strips a single trailing `\r` (for tolerance on CRLF inputs).
pub fn strip_trailing_cr(s: &str) -> &str {
    s.strip_suffix('\r').unwrap_or(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> ParseLocation {
        ParseLocation::new("dummy.gol", 1)
    }

    #[test]
    fn parse_field_line_extracts_key_and_value() {
        let (key, value) = parse_field_line(loc(), "size: 10x10").unwrap();
        assert_eq!(key, "size");
        assert_eq!(value, "10x10");
    }

    #[test]
    fn parse_field_line_trims_around_colon() {
        let (key, value) = parse_field_line(loc(), "  size  :   10x10   ").unwrap();
        assert_eq!(key, "size");
        assert_eq!(value, "10x10");
    }

    #[test]
    fn parse_field_line_allows_empty_value() {
        let (key, value) = parse_field_line(loc(), "continued_from: ").unwrap();
        assert_eq!(key, "continued_from");
        assert_eq!(value, "");
    }

    #[test]
    fn negative_parse_field_line_no_colon() {
        assert!(matches!(
            parse_field_line(loc(), "no-colon-here"),
            Err(ParseError::MalformedFieldLine { .. })
        ));
    }

    #[test]
    fn negative_parse_field_line_empty_key() {
        assert!(matches!(
            parse_field_line(loc(), ": just-a-value"),
            Err(ParseError::MalformedFieldLine { .. })
        ));
    }

    #[test]
    fn parse_section_header_matches_bracketed_name() {
        assert_eq!(parse_section_header("[config]"), Some("config"));
        assert_eq!(parse_section_header("  [ result ]  "), Some("result"));
    }

    #[test]
    fn parse_section_header_rejects_non_brackets() {
        assert!(parse_section_header("config").is_none());
        assert!(parse_section_header("[config").is_none());
        assert!(parse_section_header("config]").is_none());
        assert!(parse_section_header("[]").is_none());
    }

    #[test]
    fn parse_begin_fence_matches_canonical() {
        assert_eq!(
            parse_begin_fence("----- BEGIN INITIAL BOARD -----"),
            Some("INITIAL BOARD")
        );
        assert_eq!(parse_begin_fence("----- BEGIN BOARD -----"), Some("BOARD"));
    }

    #[test]
    fn parse_end_fence_matches_canonical() {
        assert_eq!(
            parse_end_fence("----- END FINAL BOARD -----"),
            Some("FINAL BOARD")
        );
    }

    #[test]
    fn fence_recognition_rejects_wrong_keyword() {
        assert!(parse_begin_fence("----- END INITIAL BOARD -----").is_none());
        assert!(parse_end_fence("----- BEGIN INITIAL BOARD -----").is_none());
    }

    #[test]
    fn fence_recognition_rejects_missing_label() {
        assert!(parse_begin_fence("----- BEGIN -----").is_none());
    }

    #[test]
    fn format_fence_lines_are_canonical() {
        assert_eq!(
            format_begin_fence("INITIAL BOARD"),
            "----- BEGIN INITIAL BOARD -----"
        );
        assert_eq!(
            format_end_fence("FINAL BOARD"),
            "----- END FINAL BOARD -----"
        );
    }

    #[test]
    fn strip_trailing_cr_works() {
        assert_eq!(strip_trailing_cr("line\r"), "line");
        assert_eq!(strip_trailing_cr("line"), "line");
    }
}
