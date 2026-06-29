//! Unit tests for `game_of_life::persistence::magic` (file-type sniff).

use std::io::Cursor;

use game_of_life::persistence::{
    sniff_from_reader, FileKind, MagicError, BOARD_SNAPSHOT_MAGIC, MAX_MAGIC_PEEK_BYTES,
    RUN_RECORD_MAGIC, RUN_RECORD_MAGIC_V1,
};

#[test]
fn sniff_recognizes_run_record_magic() {
    let mut cursor = Cursor::new(format!("{RUN_RECORD_MAGIC}\nmore stuff\n"));
    assert_eq!(
        sniff_from_reader("dummy.gol", &mut cursor).unwrap(),
        FileKind::RunRecord
    );
}

#[test]
fn sniff_recognizes_legacy_run_record_magic() {
    let mut cursor = Cursor::new(format!("{RUN_RECORD_MAGIC_V1}\nmore stuff\n"));
    assert_eq!(
        sniff_from_reader("dummy.gol", &mut cursor).unwrap(),
        FileKind::RunRecord
    );
}

#[test]
fn sniff_recognizes_board_snapshot_magic() {
    let mut cursor = Cursor::new(format!("{BOARD_SNAPSHOT_MAGIC}\nmore stuff\n"));
    assert_eq!(
        sniff_from_reader("dummy.gol", &mut cursor).unwrap(),
        FileKind::BoardSnapshot
    );
}

#[test]
fn sniff_tolerates_crlf_line_endings() {
    let mut cursor = Cursor::new(format!("{RUN_RECORD_MAGIC}\r\nmore stuff\n"));
    assert_eq!(
        sniff_from_reader("dummy.gol", &mut cursor).unwrap(),
        FileKind::RunRecord
    );
}

#[test]
fn negative_sniff_empty_file() {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    assert!(matches!(
        sniff_from_reader("dummy.gol", &mut cursor),
        Err(MagicError::EmptyFile { .. })
    ));
}

#[test]
fn negative_sniff_unknown_magic() {
    let mut cursor = Cursor::new(b"NOT-A-GOL-FILE v1\n".to_vec());
    let err = sniff_from_reader("dummy.gol", &mut cursor).unwrap_err();
    match err {
        MagicError::UnknownMagic { found, .. } => assert_eq!(found, "NOT-A-GOL-FILE v1"),
        other => panic!("expected UnknownMagic, got {other:?}"),
    }
}

#[test]
fn negative_sniff_first_line_too_long_to_be_magic() {
    let huge_first_line = "X".repeat(MAX_MAGIC_PEEK_BYTES * 4);
    let mut cursor = Cursor::new(huge_first_line.into_bytes());
    assert!(matches!(
        sniff_from_reader("dummy.gol", &mut cursor),
        Err(MagicError::OversizedFirstLine { .. })
    ));
}

#[test]
fn negative_sniff_binary_garbage_first_line() {
    let mut cursor = Cursor::new(vec![0xff, 0xfe, 0xfd, 0xfc, b'\n']);
    let err = sniff_from_reader("dummy.gol", &mut cursor).unwrap_err();
    assert!(matches!(err, MagicError::UnknownMagic { .. }));
}

#[test]
fn sniff_bounded_peek_does_not_read_past_limit() {
    // First byte is a newline (empty magic -> EmptyFile), but the stream is
    // much longer than the peek limit. The sniffer must not block trying to
    // consume the whole file.
    let mut payload = vec![b'\n'];
    payload.extend(std::iter::repeat_n(b'X', MAX_MAGIC_PEEK_BYTES * 100));
    let mut cursor = Cursor::new(payload);
    let err = sniff_from_reader("dummy.gol", &mut cursor).unwrap_err();
    assert!(matches!(err, MagicError::EmptyFile { .. }));
}
