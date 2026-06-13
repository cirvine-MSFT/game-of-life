//! Unit tests for `game_of_life::persistence::parser` primitives.

use game_of_life::persistence::parser::{
    format_begin_fence, format_end_fence, parse_begin_fence, parse_end_fence, parse_field_line,
    parse_section_header, strip_trailing_cr, ParseError,
};
use game_of_life::persistence::ParseLocation;

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
fn negative_parse_section_header_rejects_non_brackets() {
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
fn negative_fence_recognition_rejects_wrong_keyword() {
    assert!(parse_begin_fence("----- END INITIAL BOARD -----").is_none());
    assert!(parse_end_fence("----- BEGIN INITIAL BOARD -----").is_none());
}

#[test]
fn negative_fence_recognition_rejects_missing_label() {
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
