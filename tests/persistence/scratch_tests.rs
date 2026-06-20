//! Unit tests for `game_of_life::persistence::scratch`.
//!
//! These exercise the scratch file's binary header, fixed-stride row
//! layout, and the critical sub-byte read-modify-write semantics. See
//! AGENTS.md > Test Style for the integration-test layout.

use std::fs;

use game_of_life::persistence::scratch::{
    ScratchFile, ScratchFileError, HEADER_SIZE, SCRATCH_MAGIC, SCRATCH_SCHEMA_VERSION,
};
use game_of_life::CellState;

/// Build a unique scratch file path under the system temp dir for one
/// test, removing any leftover from a prior run with the same name.
fn temp_scratch_path(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "gol-test-scratch-{}-{}.bin",
        name,
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    path
}

#[test]
fn header_layout_is_correct() {
    let path = temp_scratch_path("header_layout");
    let scratch = ScratchFile::create(&path, 10, 5).expect("create");
    drop(scratch);

    let bytes = fs::read(&path).expect("read");
    assert!(bytes.len() as u64 >= HEADER_SIZE);
    assert_eq!(&bytes[0..16], SCRATCH_MAGIC);
    let version = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    assert_eq!(version, SCRATCH_SCHEMA_VERSION);
    let width = u64::from_le_bytes(bytes[28..36].try_into().unwrap());
    let height = u64::from_le_bytes(bytes[36..44].try_into().unwrap());
    let row_bytes = u64::from_le_bytes(bytes[44..52].try_into().unwrap());
    assert_eq!(width, 10);
    assert_eq!(height, 5);
    // ceil(10 * 2 / 8) = ceil(2.5) = 3
    assert_eq!(row_bytes, 3);

    fs::remove_file(&path).ok();
}

#[test]
fn create_refuses_to_overwrite_existing_file() {
    let path = temp_scratch_path("refuse_overwrite");
    let _scratch = ScratchFile::create(&path, 4, 2).expect("create first");
    let err = ScratchFile::create(&path, 4, 2).expect_err("create should refuse");
    assert!(matches!(err, ScratchFileError::AlreadyExists { .. }));
    fs::remove_file(&path).ok();
}

#[test]
fn create_zero_initializes_payload_to_dead() {
    let path = temp_scratch_path("zero_init");
    let mut scratch = ScratchFile::create(&path, 8, 3).expect("create");

    let mut out = Vec::new();
    for y in 0..3 {
        scratch.read_row_range(y, 0, 8, &mut out).expect("read");
        for cell in &out {
            assert_eq!(*cell, CellState::Dead);
        }
    }
    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn open_reads_dimensions_back() {
    let path = temp_scratch_path("open_roundtrip");
    {
        let _scratch = ScratchFile::create(&path, 7, 4).expect("create");
    }
    let scratch = ScratchFile::open(&path).expect("open");
    assert_eq!(scratch.width(), 7);
    assert_eq!(scratch.height(), 4);
    // ceil(7 * 2 / 8) = ceil(1.75) = 2
    assert_eq!(scratch.row_bytes(), 2);
    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn open_rejects_bad_magic() {
    let path = temp_scratch_path("bad_magic");
    fs::write(&path, b"NOT-A-SCRATCH-FILE-AT-ALL-SAME-LEN").expect("write");
    let err = ScratchFile::open(&path).expect_err("open should reject");
    assert!(matches!(err, ScratchFileError::BadMagic { .. }));
    fs::remove_file(&path).ok();
}

#[test]
fn write_and_read_single_row_full_width() {
    let path = temp_scratch_path("full_row_roundtrip");
    let mut scratch = ScratchFile::create(&path, 8, 2).expect("create");

    let row = vec![
        CellState::Alive,
        CellState::Dead,
        CellState::Dying,
        CellState::Resurrecting,
        CellState::Alive,
        CellState::Alive,
        CellState::Dead,
        CellState::Dying,
    ];
    scratch.write_row_range(1, 0, &row).expect("write");

    let mut out = Vec::new();
    scratch.read_row_range(1, 0, 8, &mut out).expect("read");
    assert_eq!(out, row);

    // The other row should still be all-Dead.
    scratch.read_row_range(0, 0, 8, &mut out).expect("read");
    for cell in &out {
        assert_eq!(*cell, CellState::Dead);
    }

    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn sub_byte_write_preserves_other_cells_in_same_byte() {
    // This is THE critical correctness test for 2-bit packing. We have
    // a row of 4 cells (one byte). We pre-populate all 4 cells with
    // distinct values, then overwrite just cell 1 with a different
    // value, and assert that cells 0, 2, 3 are unchanged.
    let path = temp_scratch_path("rmw_single_cell");
    let mut scratch = ScratchFile::create(&path, 4, 1).expect("create");

    let initial = vec![
        CellState::Alive,
        CellState::Dying,
        CellState::Resurrecting,
        CellState::Dead,
    ];
    scratch
        .write_row_range(0, 0, &initial)
        .expect("write initial");

    // Now overwrite only cell index 1, changing it to Alive.
    scratch
        .write_row_range(0, 1, &[CellState::Alive])
        .expect("rmw write");

    let mut out = Vec::new();
    scratch.read_row_range(0, 0, 4, &mut out).expect("read");
    assert_eq!(
        out,
        vec![
            CellState::Alive,        // unchanged
            CellState::Alive,        // overwritten
            CellState::Resurrecting, // unchanged
            CellState::Dead,         // unchanged
        ],
        "single-cell write must not corrupt the other 3 cells in the same byte"
    );

    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn sub_byte_write_across_byte_boundary_preserves_both_partial_bytes() {
    // Row of 8 cells (two bytes, 4 cells each). We pre-populate, then
    // write cells [3, 5) which straddles the byte boundary AND has a
    // partial first byte (only cell 3) AND a partial second byte (only
    // cell 4). Cells 0, 1, 2 in byte 0 and cells 5, 6, 7 in byte 1
    // must all be untouched.
    let path = temp_scratch_path("rmw_crossing");
    let mut scratch = ScratchFile::create(&path, 8, 1).expect("create");

    let initial = vec![
        CellState::Alive,
        CellState::Dying,
        CellState::Resurrecting,
        CellState::Dead,
        CellState::Alive,
        CellState::Dying,
        CellState::Resurrecting,
        CellState::Dead,
    ];
    scratch
        .write_row_range(0, 0, &initial)
        .expect("write initial");

    scratch
        .write_row_range(0, 3, &[CellState::Alive, CellState::Alive])
        .expect("rmw across boundary");

    let mut out = Vec::new();
    scratch.read_row_range(0, 0, 8, &mut out).expect("read");
    assert_eq!(
        out,
        vec![
            CellState::Alive,
            CellState::Dying,
            CellState::Resurrecting,
            CellState::Alive,        // overwritten
            CellState::Alive,        // overwritten
            CellState::Dying,        // unchanged
            CellState::Resurrecting, // unchanged
            CellState::Dead,         // unchanged
        ]
    );

    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn partial_range_read_returns_only_requested_cells() {
    let path = temp_scratch_path("partial_read");
    let mut scratch = ScratchFile::create(&path, 8, 1).expect("create");

    let row = vec![
        CellState::Dead,
        CellState::Alive,
        CellState::Dying,
        CellState::Resurrecting,
        CellState::Dead,
        CellState::Alive,
        CellState::Dying,
        CellState::Resurrecting,
    ];
    scratch.write_row_range(0, 0, &row).expect("write");

    let mut out = Vec::new();
    scratch.read_row_range(0, 2, 6, &mut out).expect("read mid");
    assert_eq!(
        out,
        vec![
            CellState::Dying,
            CellState::Resurrecting,
            CellState::Dead,
            CellState::Alive,
        ]
    );

    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn out_of_bounds_access_is_rejected() {
    let path = temp_scratch_path("oob");
    let mut scratch = ScratchFile::create(&path, 4, 2).expect("create");

    let mut out = Vec::new();
    let err = scratch
        .read_row_range(2, 0, 4, &mut out)
        .expect_err("row 2 out of bounds on height-2 board");
    assert!(matches!(err, ScratchFileError::OutOfBounds { .. }));

    let err = scratch
        .read_row_range(0, 0, 5, &mut out)
        .expect_err("cend 5 out of bounds on width-4 board");
    assert!(matches!(err, ScratchFileError::OutOfBounds { .. }));

    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn empty_range_is_a_noop() {
    let path = temp_scratch_path("empty_range");
    let mut scratch = ScratchFile::create(&path, 4, 1).expect("create");

    let mut out = vec![CellState::Alive]; // should be cleared
    scratch
        .read_row_range(0, 2, 2, &mut out)
        .expect("noop read");
    assert!(out.is_empty(), "empty range should leave out empty");

    scratch.write_row_range(0, 1, &[]).expect("noop write");

    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn round_trip_after_reopen_preserves_data() {
    let path = temp_scratch_path("reopen_roundtrip");
    {
        let mut scratch = ScratchFile::create(&path, 6, 2).expect("create");
        scratch
            .write_row_range(
                0,
                0,
                &[
                    CellState::Alive,
                    CellState::Dying,
                    CellState::Dead,
                    CellState::Resurrecting,
                    CellState::Alive,
                    CellState::Dead,
                ],
            )
            .expect("write");
        scratch
            .write_row_range(1, 2, &[CellState::Dying, CellState::Resurrecting])
            .expect("write");
        scratch.flush().expect("flush");
    }

    let mut scratch = ScratchFile::open(&path).expect("reopen");
    let mut out = Vec::new();
    scratch
        .read_row_range(0, 0, 6, &mut out)
        .expect("read row 0");
    assert_eq!(
        out,
        vec![
            CellState::Alive,
            CellState::Dying,
            CellState::Dead,
            CellState::Resurrecting,
            CellState::Alive,
            CellState::Dead,
        ]
    );
    scratch
        .read_row_range(1, 0, 6, &mut out)
        .expect("read row 1");
    assert_eq!(
        out,
        vec![
            CellState::Dead,
            CellState::Dead,
            CellState::Dying,
            CellState::Resurrecting,
            CellState::Dead,
            CellState::Dead,
        ]
    );

    drop(scratch);
    fs::remove_file(&path).ok();
}

#[test]
fn row_bytes_for_width_rounds_up() {
    assert_eq!(ScratchFile::row_bytes_for_width(0), 0);
    assert_eq!(ScratchFile::row_bytes_for_width(1), 1);
    assert_eq!(ScratchFile::row_bytes_for_width(4), 1);
    assert_eq!(ScratchFile::row_bytes_for_width(5), 2);
    assert_eq!(ScratchFile::row_bytes_for_width(8), 2);
    assert_eq!(ScratchFile::row_bytes_for_width(9), 3);
    assert_eq!(ScratchFile::row_bytes_for_width(1_000_000), 250_000);
}

#[test]
fn file_size_for_includes_header_plus_payload() {
    assert_eq!(
        ScratchFile::file_size_for(8, 4),
        HEADER_SIZE + 4 * 2 // row_bytes(8) = 2
    );
    assert_eq!(
        ScratchFile::file_size_for(7, 4),
        HEADER_SIZE + 4 * 2 // row_bytes(7) = 2 (ceil(14/8))
    );
}
