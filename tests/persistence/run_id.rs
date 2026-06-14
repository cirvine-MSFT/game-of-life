//! Unit tests for `game_of_life::persistence::run_id`.

use std::collections::HashSet;

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
fn generate_sets_v4_bits_on_a_thousand_ids() {
    // Bit-fixup must run on *every* generated id, not just the first.
    for _ in 0..1_000 {
        let id = RunId::generate();
        let b = id.as_bytes();
        assert_eq!(b[6] >> 4, 4, "version nibble must be 4 on every id");
        assert_eq!(b[8] & 0xc0, 0x80, "variant bits must be 10xx on every id");
    }
}

#[test]
fn generate_produces_distinct_ids_at_scale() {
    // 10,000 ids; every one of them must be unique in this process. The
    // generator mixes per-call entropy (RandomState + hashed time + pid +
    // monotonic counter), so collisions within a single process would
    // indicate a real defect in the entropy mixing, not normal birthday-
    // paradox behavior on a 122-bit space.
    const N: usize = 10_000;
    let mut seen: HashSet<RunId> = HashSet::with_capacity(N);
    for _ in 0..N {
        let id = RunId::generate();
        assert!(
            seen.insert(id),
            "duplicate id generated within a single process: {id}"
        );
    }
}

#[test]
fn generate_produces_distinct_ids_across_threads() {
    // 8 threads each generating 1,000 ids in parallel. Collision here would
    // indicate the entropy source is being clobbered by concurrent generation
    // (e.g. a shared but unsynchronized counter).
    const THREADS: usize = 8;
    const PER_THREAD: usize = 1_000;

    let mut handles = Vec::with_capacity(THREADS);
    for _ in 0..THREADS {
        handles.push(std::thread::spawn(|| {
            let mut ids = Vec::with_capacity(PER_THREAD);
            for _ in 0..PER_THREAD {
                ids.push(RunId::generate());
            }
            ids
        }));
    }

    let mut seen: HashSet<RunId> = HashSet::with_capacity(THREADS * PER_THREAD);
    for handle in handles {
        for id in handle.join().expect("worker thread panicked") {
            assert!(seen.insert(id), "cross-thread duplicate: {id}");
        }
    }
    assert_eq!(seen.len(), THREADS * PER_THREAD);
}

#[test]
fn format_then_parse_roundtrips() {
    let id = RunId::generate();
    let s = format_run_id(&id);
    let parsed = parse_run_id(&s).unwrap();
    assert_eq!(id, parsed);
}

#[test]
fn format_then_parse_roundtrips_for_a_thousand_ids() {
    // Roundtripping a single id only proves the easy case. Run the
    // format -> parse -> format cycle across many ids so a regression in
    // either function shows up as a mismatch on at least one of them.
    for _ in 0..1_000 {
        let id = RunId::generate();
        let s = format_run_id(&id);
        let parsed = parse_run_id(&s).unwrap_or_else(|e| {
            panic!("format produced an un-parseable string '{s}': {e}");
        });
        assert_eq!(id, parsed, "format/parse roundtrip diverged for {s}");
        assert_eq!(
            format_run_id(&parsed),
            s,
            "second-pass format must equal first-pass format for {s}",
        );
    }
}

#[test]
fn format_output_is_structurally_canonical_for_generated_ids() {
    // Validate the structural shape: length, hyphen positions, all-lowercase
    // hex. Done across many random ids so we catch any positional bug.
    for _ in 0..1_000 {
        let s = format_run_id(&RunId::generate());
        assert_eq!(s.len(), 36, "wrong length: '{s}'");
        let bytes = s.as_bytes();
        assert_eq!(bytes[8], b'-', "missing hyphen at pos 8 in '{s}'");
        assert_eq!(bytes[13], b'-', "missing hyphen at pos 13 in '{s}'");
        assert_eq!(bytes[18], b'-', "missing hyphen at pos 18 in '{s}'");
        assert_eq!(bytes[23], b'-', "missing hyphen at pos 23 in '{s}'");
        for (i, c) in s.char_indices() {
            if [8, 13, 18, 23].contains(&i) {
                continue;
            }
            assert!(
                c.is_ascii_hexdigit() && (!c.is_ascii_alphabetic() || c.is_ascii_lowercase()),
                "non-canonical char '{c}' at position {i} in '{s}'"
            );
        }
    }
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
