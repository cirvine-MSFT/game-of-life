//! Integration tests for utilities exposed by the run-commands module.
//!
//! The Tauri command wrappers themselves require a real Tauri runtime
//! (with a `State` and an `AppHandle`) so they're exercised end-to-end
//! via `RunSession` integration tests in `session.rs` instead. This
//! file covers the small set of pure helpers we deliberately export so
//! they can be unit-tested without bringing up the runtime.

use game_of_life_desktop_lib::commands::run_commands::{clamp_gps, MAX_GPS, MIN_GPS};

#[test]
fn clamp_gps_passes_through_values_in_range() {
    assert_eq!(clamp_gps(MIN_GPS), MIN_GPS);
    assert_eq!(clamp_gps(5), 5);
    assert_eq!(clamp_gps(60), 60);
    assert_eq!(clamp_gps(MAX_GPS), MAX_GPS);
}

#[test]
fn clamp_gps_lifts_zero_to_min() {
    assert_eq!(clamp_gps(0), MIN_GPS);
}

#[test]
fn clamp_gps_caps_above_max_to_max() {
    assert_eq!(clamp_gps(u16::MAX), MAX_GPS);
    assert_eq!(clamp_gps(MAX_GPS + 1), MAX_GPS);
}

#[test]
fn min_gps_and_max_gps_form_a_valid_range() {
    // Sanity check that a future tuning of the constants doesn't
    // accidentally invert the range.
    const _: () = assert!(MIN_GPS <= MAX_GPS);
    const _: () = assert!(
        MIN_GPS > 0,
        "MIN_GPS must be > 0 so 1_000_000 / gps doesn't divide by zero",
    );
}
