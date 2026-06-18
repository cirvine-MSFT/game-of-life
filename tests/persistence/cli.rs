//! End-to-end integration tests for persistence-related CLI verbs:
//! save, load, continue, replay, and extract-board.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(label: &str) -> PathBuf {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "gol_cli_persistence_{label}_{}_{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_game-of-life"))
        .args(args)
        .output()
        .expect("CLI should run")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn one_run_record_in(dir: &Path) -> PathBuf {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("gol"))
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one .gol file in {dir:?}"
    );
    entries.remove(0)
}

mod save_load_tests {
    use super::*;

    #[test]
    fn save_writes_run_record_into_runs_dir() {
        let dir = unique_temp_dir("save_basic");
        let runs_dir = dir.join("runs");
        let output = run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "2",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let stdout_text = stdout(&output);
        assert!(stdout_text.contains("Saved run record:"));
        let record_path = one_run_record_in(&runs_dir);
        let body = std::fs::read_to_string(&record_path).expect("read record");
        assert!(body.starts_with("GOL-RUN-RECORD v1"));
        assert!(body.contains("content_hash:"));
    }

    #[test]
    fn explicit_save_run_writes_to_named_path() {
        let dir = unique_temp_dir("save_explicit");
        let path = dir.join("my-named-run.gol");
        let output = run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--save-run",
            path.to_str().unwrap(),
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert!(path.exists());
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .starts_with("GOL-RUN-RECORD v1"));
    }

    #[test]
    fn no_save_suppresses_run_record() {
        let dir = unique_temp_dir("no_save");
        let runs_dir = dir.join("runs");
        let output = run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
            "--no-save",
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert!(
            !runs_dir.exists() || std::fs::read_dir(&runs_dir).map(|i| i.count()).unwrap_or(0) == 0
        );
        assert!(!stdout(&output).contains("Saved run record:"));
    }

    #[test]
    fn negative_save_run_refuses_to_overwrite_existing_path() {
        let dir = unique_temp_dir("collision");
        let path = dir.join("existing.gol");
        std::fs::write(&path, b"existing").unwrap();
        let output = run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--save-run",
            path.to_str().unwrap(),
        ]);
        assert!(!output.status.success(), "should have failed");
        assert!(
            stderr(&output).contains("Refusing to overwrite"),
            "got: {}",
            stderr(&output)
        );
    }

    #[test]
    fn load_board_from_snapshot_starts_run_from_extracted_file() {
        // First produce a snapshot via extract-board, then load it.
        let dir = unique_temp_dir("load_snapshot");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "2",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let snap_path = dir.join("snap.gol");
        let extract = run_cli(&[
            "--extract-board",
            source.to_str().unwrap(),
            "--load-from",
            "final",
            "--output",
            snap_path.to_str().unwrap(),
        ]);
        assert!(extract.status.success(), "stderr: {}", stderr(&extract));
        let run_again = run_cli(&[
            "--load-board",
            snap_path.to_str().unwrap(),
            "--max-iterations",
            "1",
            "--no-save",
        ]);
        assert!(run_again.status.success(), "stderr: {}", stderr(&run_again));
    }

    #[test]
    fn load_board_from_run_record_default_picks_initial() {
        let dir = unique_temp_dir("load_run_default");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "3",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output = run_cli(&[
            "--load-board",
            source.to_str().unwrap(),
            "--max-iterations",
            "0",
            "--no-save",
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        // Initial board for "alive" 4x4 has 16 alive cells.
        let out = stdout(&output);
        assert!(
            out.contains("16 alive"),
            "expected 16 alive on initial; got:\n{out}"
        );
    }

    #[test]
    fn load_board_with_initial_board_emits_takes_precedence_warning() {
        // --load-board takes precedence over --initial-board. The behavior
        // should be: load wins AND user gets a warning naming the conflict.
        let dir = unique_temp_dir("load_initial_conflict");
        let snap_path = dir.join("snap.gol");
        std::fs::write(
            &snap_path,
            "GOL-BOARD-SNAPSHOT v1\nschema_version: 1\ncreated_at: 2026-06-12T22:55:20Z\n\n----- BEGIN BOARD -----\nsize: 3x3\nencoding: ascii\nalive_count: 1\ndead_count: 8\n.#.\n...\n...\n----- END BOARD -----\n",
        )
        .unwrap();
        let output = run_cli(&[
            "--load-board",
            snap_path.to_str().unwrap(),
            "--initial-board",
            "alive",
            "--max-iterations",
            "0",
            "--no-save",
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("--load-board takes precedence")
                && stderr_text.contains("--initial-board"),
            "expected stderr warning naming both flags; got:\n{stderr_text}"
        );
    }
}

mod continuation_tests {
    use super::*;

    #[test]
    fn continue_records_provenance_to_source_run() {
        let dir = unique_temp_dir("continue_provenance");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "5x5",
            "--max-iterations",
            "2",
            "--initial-board",
            "blinker",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let source_id = std::fs::read_to_string(&source)
            .unwrap()
            .lines()
            .find_map(|l| l.strip_prefix("run_id: ").map(|s| s.to_string()))
            .expect("source run id");

        let continued_runs_dir = dir.join("continued");
        let output = run_cli(&[
            "--continue",
            source.to_str().unwrap(),
            "--additional-iterations",
            "3",
            "--runs-dir",
            continued_runs_dir.to_str().unwrap(),
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let continued = one_run_record_in(&continued_runs_dir);
        let body = std::fs::read_to_string(&continued).unwrap();
        assert!(
            body.contains(&format!("continued_from: {source_id}")),
            "continued_from missing or wrong; body:\n{body}"
        );
    }

    #[test]
    fn negative_continue_without_additional_iterations_errors() {
        let dir = unique_temp_dir("continue_missing_iter");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output = run_cli(&["--continue", source.to_str().unwrap()]);
        assert!(!output.status.success());
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("--additional-iterations or --max-iterations")
                && stderr_text.contains("required"),
            "stderr should require either --additional-iterations or --max-iterations; got:\n{stderr_text}"
        );
    }

    #[test]
    fn continue_with_cumulative_max_iterations_runs_the_remainder() {
        // Source ran 4 iterations. Continuation with --max-iterations 10 should
        // run for 6 more, and the new record's iterations_run reflects that.
        let dir = unique_temp_dir("continue_cumulative");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "5x5",
            "--max-iterations",
            "4",
            "--initial-board",
            "blinker",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);

        let continued_runs_dir = dir.join("continued");
        let output = run_cli(&[
            "--continue",
            source.to_str().unwrap(),
            "--max-iterations",
            "10",
            "--runs-dir",
            continued_runs_dir.to_str().unwrap(),
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let continued = one_run_record_in(&continued_runs_dir);
        let body = std::fs::read_to_string(&continued).unwrap();
        assert!(
            body.contains("iterations_run: 6"),
            "expected the continuation to run 6 iterations (10 cumulative - 4 source); body:\n{body}"
        );
    }

    #[test]
    fn negative_continue_cumulative_max_not_greater_than_source_iterations() {
        // Source ran 4 iterations; --max-iterations 4 has nothing left to do.
        let dir = unique_temp_dir("continue_cum_equal");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "5x5",
            "--max-iterations",
            "4",
            "--initial-board",
            "blinker",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);

        let output = run_cli(&[
            "--continue",
            source.to_str().unwrap(),
            "--max-iterations",
            "4",
        ]);
        assert!(
            !output.status.success(),
            "should reject cumulative max == source iterations_run"
        );
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("not greater than")
                && stderr_text.contains("iterations_run")
                && stderr_text.contains("--additional-iterations"),
            "stderr should explain cumulative-max constraint and offer --additional-iterations alternative; got:\n{stderr_text}"
        );

        // Also assert the "strictly less than" case (3 < 4) is rejected with the
        // same error.
        let output_lt = run_cli(&[
            "--continue",
            source.to_str().unwrap(),
            "--max-iterations",
            "3",
        ]);
        assert!(
            !output_lt.status.success(),
            "should reject cumulative max < source iterations_run"
        );
        assert!(
            stderr(&output_lt).contains("not greater than"),
            "stderr should reject cumulative < source; got:\n{}",
            stderr(&output_lt)
        );
    }

    #[test]
    fn negative_continue_with_both_additional_and_max_iterations_is_mutually_exclusive() {
        let dir = unique_temp_dir("continue_both_budgets");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "2",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output = run_cli(&[
            "--continue",
            source.to_str().unwrap(),
            "--additional-iterations",
            "3",
            "--max-iterations",
            "10",
        ]);
        assert!(!output.status.success());
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("mutually exclusive")
                && stderr_text.contains("--additional-iterations")
                && stderr_text.contains("--max-iterations"),
            "stderr should name both flags as mutually exclusive; got:\n{stderr_text}"
        );
    }

    #[test]
    fn negative_continue_with_load_board_is_mutually_exclusive() {
        let dir = unique_temp_dir("continue_conflict");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output = run_cli(&[
            "--continue",
            source.to_str().unwrap(),
            "--additional-iterations",
            "1",
            "--load-board",
            source.to_str().unwrap(),
        ]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("mutually exclusive"));
    }

    #[test]
    fn negative_continue_with_initial_board_is_mutually_exclusive() {
        let dir = unique_temp_dir("continue_conflict2");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output = run_cli(&[
            "--continue",
            source.to_str().unwrap(),
            "--additional-iterations",
            "1",
            "--initial-board",
            "demo",
        ]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("mutually exclusive"));
    }
}

mod replay_tests {
    use super::*;

    #[test]
    fn replay_round_trip_succeeds_with_zero_exit() {
        let dir = unique_temp_dir("replay_ok");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "5x5",
            "--max-iterations",
            "3",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output = run_cli(&["--replay", source.to_str().unwrap()]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert!(stdout(&output).contains("Replay matched"));
    }

    #[test]
    fn stable_run_record_replays_successfully() {
        let dir = unique_temp_dir("replay_stable");
        let runs_dir = dir.join("runs");
        let run = run_cli(&[
            "--board-size",
            "2x2",
            "--max-iterations",
            "10",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        assert!(run.status.success(), "stderr: {}", stderr(&run));
        let source = one_run_record_in(&runs_dir);
        let body = std::fs::read_to_string(&source).unwrap();
        assert!(body.contains("status: stable"), "body:\n{body}");
        assert!(body.contains("iterations_run: 1"), "body:\n{body}");

        let output = run_cli(&["--replay", source.to_str().unwrap()]);

        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert!(stdout(&output).contains("Replay matched"));
    }

    #[test]
    fn legacy_max_iterations_record_for_stable_board_still_replays() {
        let dir = unique_temp_dir("replay_legacy_stable");
        let runs_dir = dir.join("runs");
        let run = run_cli(&[
            "--board-size",
            "2x2",
            "--max-iterations",
            "10",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        assert!(run.status.success(), "stderr: {}", stderr(&run));
        let source = one_run_record_in(&runs_dir);
        let body = std::fs::read_to_string(&source).unwrap();
        let legacy_body = body
            .replacen("status: stable", "status: max_iterations", 1)
            .replacen("iterations_run: 1", "iterations_run: 10", 1);
        std::fs::write(&source, legacy_body).unwrap();

        let output = run_cli(&["--replay", source.to_str().unwrap(), "--ignore-integrity"]);

        assert!(output.status.success(), "stderr: {}", stderr(&output));
        assert!(stdout(&output).contains("Replay matched"));
    }

    #[test]
    fn negative_replay_corrupted_file_under_enforce() {
        let dir = unique_temp_dir("replay_corrupted");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "2",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        // Flip a cell from '.' to '#' (or similar) somewhere in the body.
        let body = std::fs::read_to_string(&source).unwrap();
        // The file should contain a final board grid; corrupt the first '.' we find.
        let corrupted = body.replacen('.', "X", 1);
        std::fs::write(&source, corrupted).unwrap();
        let output = run_cli(&["--replay", source.to_str().unwrap()]);
        assert!(!output.status.success(), "should fail integrity");
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("failed integrity check")
                || stderr_text.contains("Unknown board character"),
            "expected integrity-failure or grid-validity error; got:\n{stderr_text}"
        );
    }

    #[test]
    fn negative_replay_with_snapshot_file_errors_with_helpful_message() {
        let dir = unique_temp_dir("replay_wrong_kind");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let snap = dir.join("snap.gol");
        let extract = run_cli(&[
            "--extract-board",
            source.to_str().unwrap(),
            "--output",
            snap.to_str().unwrap(),
        ]);
        assert!(extract.status.success());
        let output = run_cli(&["--replay", snap.to_str().unwrap()]);
        assert!(!output.status.success());
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("is a board snapshot"),
            "expected wrong-file-kind message; got:\n{stderr_text}"
        );
    }
}

mod extract_tests {
    use super::*;

    #[test]
    fn extract_final_produces_hash_free_snapshot() {
        let dir = unique_temp_dir("extract_final");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output_path = dir.join("snap.gol");
        let output = run_cli(&[
            "--extract-board",
            source.to_str().unwrap(),
            "--load-from",
            "final",
            "--output",
            output_path.to_str().unwrap(),
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let body = std::fs::read_to_string(&output_path).unwrap();
        assert!(body.starts_with("GOL-BOARD-SNAPSHOT v1"));
        assert!(
            !body.contains("content_hash:"),
            "snapshot must not contain content_hash; got:\n{body}"
        );
    }

    #[test]
    fn negative_extract_requires_output() {
        let dir = unique_temp_dir("extract_missing_output");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let output = run_cli(&["--extract-board", source.to_str().unwrap()]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("--output"));
    }

    #[test]
    fn negative_extract_refuses_to_overwrite_existing_output() {
        let dir = unique_temp_dir("extract_collision");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "1",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let existing = dir.join("existing.gol");
        std::fs::write(&existing, b"hi").unwrap();
        let output = run_cli(&[
            "--extract-board",
            source.to_str().unwrap(),
            "--output",
            existing.to_str().unwrap(),
        ]);
        assert!(!output.status.success());
        assert!(stderr(&output).contains("Refusing to overwrite"));
    }
}

mod magic_and_negative_tests {
    use super::*;

    #[test]
    fn negative_load_from_random_file_reports_wrong_magic() {
        let dir = unique_temp_dir("wrong_magic");
        let path = dir.join("random.gol");
        std::fs::write(&path, b"hello there\nnot a gol file\n").unwrap();
        let output = run_cli(&[
            "--load-board",
            path.to_str().unwrap(),
            "--no-save",
            "--max-iterations",
            "0",
        ]);
        assert!(!output.status.success());
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("not a Game of Life file"),
            "expected magic-mismatch error; got:\n{stderr_text}"
        );
    }

    #[test]
    fn negative_load_from_empty_file_reports_empty() {
        let dir = unique_temp_dir("empty");
        let path = dir.join("empty.gol");
        std::fs::write(&path, b"").unwrap();
        let output = run_cli(&[
            "--load-board",
            path.to_str().unwrap(),
            "--no-save",
            "--max-iterations",
            "0",
        ]);
        assert!(!output.status.success());
        assert!(
            stderr(&output).contains("is empty")
                || stderr(&output).contains("not a Game of Life file")
        );
    }
}

mod stats_tests {
    use super::*;

    #[test]
    fn extinct_run_reports_status_extinct_in_record() {
        // A fully-alive 4x4 collapses fast: every interior and edge cell has
        // 5+ live neighbors and dies in gen 1, leaving only the four corners
        // (each with 0 live neighbors). Gen 2 wipes them out. So with
        // max_iterations=10 the run early-stops at gen 2 with status=extinct.
        let dir = unique_temp_dir("extinct");
        let runs_dir = dir.join("runs");
        let output = run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "10",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let record = one_run_record_in(&runs_dir);
        let body = std::fs::read_to_string(&record).expect("read record");
        assert!(
            body.contains("status: extinct"),
            "expected status: extinct in record; body:\n{body}"
        );
        assert!(
            body.contains("final_alive_count: 0"),
            "expected final_alive_count: 0; body:\n{body}"
        );
        // Should have early-stopped well before max_iterations=10.
        let iter_line = body
            .lines()
            .find(|l| l.starts_with("iterations_run: "))
            .expect("iterations_run line");
        let iter: u64 = iter_line
            .trim_start_matches("iterations_run: ")
            .parse()
            .unwrap();
        assert!(
            iter < 10,
            "expected early-stop before max_iterations=10; got iterations_run={iter}"
        );
    }

    /// Helper: parse `field_name: value` lines from a run record body.
    fn extract_field<'a>(body: &'a str, field: &str) -> Option<&'a str> {
        body.lines()
            .find_map(|l| l.strip_prefix(&format!("{field}: ")))
    }

    fn extract_u64(body: &str, field: &str) -> u64 {
        extract_field(body, field)
            .unwrap_or_else(|| panic!("field '{field}' not present in record:\n{body}"))
            .parse()
            .unwrap_or_else(|e| panic!("field '{field}' not parseable as u64: {e}"))
    }

    #[test]
    fn stats_recorded_in_run_record_match_runtime() {
        // 4x4 alive collapses to 4 corners at gen 1 (born=0, died=12), then
        // each corner has zero live neighbors at gen 2 so all die (born=0,
        // died=4). Total births=0, total_deaths=16, final_alive_count=0,
        // peak=16@gen 0, min=0@gen 2, iterations_run=2, status=extinct
        // (early-stop on extinction).
        let dir = unique_temp_dir("stats");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "10",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let body = std::fs::read_to_string(&source).unwrap();

        assert_eq!(extract_field(body.as_ref(), "status"), Some("extinct"));
        assert_eq!(extract_u64(&body, "initial_alive_count"), 16);
        assert_eq!(extract_u64(&body, "final_alive_count"), 0);
        assert_eq!(extract_u64(&body, "peak_alive_count"), 16);
        assert_eq!(extract_u64(&body, "peak_alive_generation"), 0);
        assert_eq!(extract_u64(&body, "min_alive_count"), 0);
        assert_eq!(extract_u64(&body, "iterations_run"), 2);
        assert_eq!(extract_u64(&body, "total_births"), 0);
        assert_eq!(extract_u64(&body, "total_deaths"), 16);
        // Invariant: initial_alive_count + total_births - total_deaths
        //          = final_alive_count.
        let initial = extract_u64(&body, "initial_alive_count") as i64;
        let births = extract_u64(&body, "total_births") as i64;
        let deaths = extract_u64(&body, "total_deaths") as i64;
        let final_alive = extract_u64(&body, "final_alive_count") as i64;
        assert_eq!(
            initial + births - deaths,
            final_alive,
            "alive-cell conservation: initial + births - deaths must equal final"
        );
    }
}

mod integrity_tests {
    use super::*;

    #[test]
    fn negative_corrupted_run_record_under_enforce_fails_with_actionable_message() {
        let dir = unique_temp_dir("corrupt_actionable");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "2",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let body = std::fs::read_to_string(&source).unwrap();
        // Edit one of the [result] numeric fields to break integrity but
        // keep the file structurally valid.
        let corrupted = body.replacen("wall_time_ms: ", "wall_time_ms: 9999999", 1);
        std::fs::write(&source, corrupted).unwrap();
        let output = run_cli(&["--replay", source.to_str().unwrap()]);
        assert!(!output.status.success());
        let stderr_text = stderr(&output);
        assert!(stderr_text.contains("failed integrity check"));
        assert!(stderr_text.contains("--ignore-integrity"));
        assert!(stderr_text.contains("--extract-board"));
    }

    #[test]
    fn ignore_integrity_bypasses_with_warning() {
        let dir = unique_temp_dir("ignore_integrity");
        let runs_dir = dir.join("runs");
        run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "2",
            "--initial-board",
            "alive",
            "--runs-dir",
            runs_dir.to_str().unwrap(),
        ]);
        let source = one_run_record_in(&runs_dir);
        let body = std::fs::read_to_string(&source).unwrap();
        let corrupted = body.replacen("wall_time_ms: ", "wall_time_ms: 9999999", 1);
        std::fs::write(&source, corrupted).unwrap();
        let output = run_cli(&["--replay", source.to_str().unwrap(), "--ignore-integrity"]);
        // --ignore-integrity must bypass the content_hash check entirely:
        // the user sees a Warning: that names the bypass, and the
        // Corrupted error path must not fire.
        let stderr_text = stderr(&output);
        assert!(
            stderr_text.contains("integrity check bypassed"),
            "expected stderr to contain the integrity-bypassed warning; got:\n{stderr_text}"
        );
        assert!(
            !stderr_text.contains("failed integrity check"),
            "did not expect the failed-integrity error when --ignore-integrity is set; got:\n{stderr_text}"
        );
    }

    #[test]
    fn snapshot_files_never_integrity_checked() {
        // Hand-craft a snapshot, edit it, load it -- should succeed.
        let dir = unique_temp_dir("snapshot_no_integrity");
        let path = dir.join("snap.gol");
        std::fs::write(
            &path,
            "GOL-BOARD-SNAPSHOT v1\nschema_version: 1\ncreated_at: 2026-06-12T22:55:20Z\n\n----- BEGIN BOARD -----\nsize: 3x3\nencoding: ascii\nalive_count: 1\ndead_count: 8\n.#.\n...\n...\n----- END BOARD -----\n",
        )
        .unwrap();
        let output = run_cli(&[
            "--load-board",
            path.to_str().unwrap(),
            "--max-iterations",
            "0",
            "--no-save",
        ]);
        assert!(output.status.success(), "stderr: {}", stderr(&output));
    }
}
