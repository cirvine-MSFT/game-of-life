use std::process::{Command, Output};

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

mod normal_tests {
    use super::*;

    #[test]
    fn help_prints_usage_and_supported_options() {
        let output = run_cli(&["--help"]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Usage:"));
        assert!(stdout.contains("--board-size"));
        assert!(stdout.contains("--max-iterations"));
        assert!(stdout.contains("--max-board-memory"));
        assert!(stdout.contains("--initial-board"));
    }

    #[test]
    fn valid_run_prints_concise_final_state_only() {
        let output = run_cli(&["--board-size", "2x2", "--max-iterations", "1"]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Board size: 2x2"));
        assert!(stdout.contains("Max iterations: 1"));
        assert!(stdout.contains("Initial board: demo"));
        assert!(stdout.contains("Final board state:"));
        assert!(stdout.contains("##\n##\n"));
        assert!(stdout.contains("Simulation complete: 1 iterations"));
        assert!(!stdout.contains("Generation 1:"));
    }

    #[test]
    fn zero_iteration_run_uses_default_demo_initial_board() {
        let output = run_cli(&["--max-iterations", "0"]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Board size: 10x10"));
        assert!(stdout.contains("Initial board: demo"));
        assert!(stdout.contains(
            "Final board state:\n..........\n..........\n.....#.#..\n..#.##.#..\n......#...\n...##.....\n..##.#....\n...#......\n..........\n..........\n"
        ));
    }

    #[test]
    fn selected_blinker_initial_board_is_used() {
        let output = run_cli(&[
            "--board-size",
            "5x5",
            "--max-iterations",
            "0",
            "--initial-board",
            "blinker",
        ]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Initial board: blinker"));
        assert!(stdout.contains("Final board state:\n.....\n.....\n.###.\n.....\n.....\n"));
    }

    #[test]
    fn selected_alive_initial_board_is_used() {
        let output = run_cli(&[
            "--board-size",
            "3x2",
            "--max-iterations",
            "0",
            "--initial-board",
            "alive",
        ]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Initial board: alive"));
        assert!(stdout.contains("Final board state:\n###\n###\n"));
    }

    #[test]
    fn alive_initial_board_larger_than_two_by_two_dies_quickly() {
        let output = run_cli(&[
            "--board-size",
            "4x4",
            "--max-iterations",
            "2",
            "--initial-board",
            "alive",
        ]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Initial board: alive"));
        assert!(stdout.contains("Final board state:\n....\n....\n....\n....\n"));
    }

    #[test]
    fn selected_random_initial_board_runs_successfully() {
        let output = run_cli(&[
            "--board-size",
            "3x3",
            "--max-iterations",
            "0",
            "--initial-board",
            "random",
        ]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Initial board: random"));
        assert!(stdout.contains("Final board state:"));
    }

    #[test]
    fn stable_run_stops_before_max_iterations() {
        let output = run_cli(&[
            "--board-size",
            "2x2",
            "--max-iterations",
            "10",
            "--initial-board",
            "alive",
            "--no-save",
        ]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Stable state reached at generation 1"));
        assert!(stdout.contains("Simulation complete: 1 iterations (stable)"));
    }

    #[test]
    fn oscillator_run_does_not_stop_as_stable() {
        let output = run_cli(&[
            "--board-size",
            "5x5",
            "--max-iterations",
            "2",
            "--initial-board",
            "blinker",
            "--no-save",
        ]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(!stdout.contains("Stable state reached"));
        assert!(stdout.contains("Simulation complete: 2 iterations (max_iterations)"));
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn edge_case_zero_iterations_prints_initial_board_as_final_state() {
        let output = run_cli(&["--board-size", "1x1", "--max-iterations", "0"]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Board size: 1x1"));
        assert!(stdout.contains("Max iterations: 0"));
        assert!(stdout.contains("Final board state:\n#\n"));
        assert!(!stdout.contains("Generation 1:"));
    }
}

mod negative_tests {
    use super::*;

    #[test]
    fn negative_zero_width_exits_with_actionable_error() {
        let output = run_cli(&["--board-size", "0x2"]);

        assert!(!output.status.success());
        assert!(stdout(&output).is_empty());

        let stderr = stderr(&output);
        assert!(stderr.contains("zero width"));
        assert!(stderr.contains("greater than 0"));
        assert!(stderr.contains("--help"));
    }

    #[test]
    fn negative_negative_width_exits_with_actionable_error() {
        let output = run_cli(&["--board-size=-1x2"]);

        assert!(!output.status.success());
        assert!(stdout(&output).is_empty());

        let stderr = stderr(&output);
        assert!(stderr.contains("negative width"));
        assert!(stderr.contains("positive whole numbers"));
        assert!(stderr.contains("--help"));
    }

    #[test]
    fn negative_non_integer_width_exits_with_actionable_error() {
        let output = run_cli(&["--board-size", "2.5x3"]);

        assert!(!output.status.success());
        assert!(stdout(&output).is_empty());

        let stderr = stderr(&output);
        assert!(stderr.contains("non-integer width"));
        assert!(stderr.contains("positive whole numbers"));
        assert!(stderr.contains("--help"));
    }

    #[test]
    fn negative_invalid_iteration_count_exits_with_actionable_error() {
        let output = run_cli(&["--max-iterations", "1.5"]);

        assert!(!output.status.success());
        assert!(stdout(&output).is_empty());

        let stderr = stderr(&output);
        assert!(stderr.contains("not an integer"));
        assert!(stderr.contains("non-negative whole number"));
        assert!(stderr.contains("--help"));
    }

    #[test]
    fn negative_invalid_memory_budget_exits_with_actionable_error() {
        let output = run_cli(&["--max-board-memory", "1.5MB"]);

        assert!(!output.status.success());
        assert!(stdout(&output).is_empty());

        let stderr = stderr(&output);
        assert!(stderr.contains("whole-number size"));
        assert!(stderr.contains("--help"));
    }

    #[test]
    fn edge_case_memory_budget_too_small_auto_promotes_to_streaming() {
        // After the streaming PR, an over-budget initializer-based run no
        // longer fails; it auto-promotes to the streaming backend and
        // logs a notice on stderr.
        let output = run_cli(&[
            "--board-size",
            "10x10",
            "--max-board-memory",
            "99B",
            "--max-iterations",
            "0",
            "--no-save",
        ]);

        assert!(
            output.status.success(),
            "auto-promotion should succeed; stderr: {}",
            stderr(&output)
        );
        let stderr = stderr(&output);
        assert!(
            stderr.contains("streaming mode enabled"),
            "stderr should mention auto-promotion: {stderr}"
        );
        // Original failure message should NOT appear; the in-memory
        // BoardCreation error path is bypassed for initializer runs.
        assert!(!stderr.contains("requires 100 bytes"));
    }

    #[test]
    fn negative_invalid_initial_board_exits_with_actionable_error() {
        let output = run_cli(&["--initial-board", "file:seed.txt"]);

        assert!(!output.status.success());
        assert!(stdout(&output).is_empty());

        let stderr = stderr(&output);
        assert!(stderr.contains("demo, alive, blinker, random"));
        assert!(stderr.contains("planned"));
        assert!(stderr.contains("--help"));
    }
}

mod streaming_tests {
    use super::*;
    use std::fs;

    #[test]
    fn streaming_run_advances_and_logs_chunk_dimensions() {
        // 50x50 = 2500 cells = 2500 bytes; cap of 512 forces streaming.
        let output = run_cli(&[
            "--board-size",
            "50x50",
            "--max-board-memory",
            "512B",
            "--initial-board",
            "blinker",
            "--max-iterations",
            "3",
            "--no-save",
        ]);

        assert!(
            output.status.success(),
            "streaming run should succeed; stderr: {}",
            stderr(&output)
        );
        let stderr = stderr(&output);
        let stdout = stdout(&output);
        assert!(
            stderr.contains("streaming mode enabled"),
            "stderr should announce streaming: {stderr}"
        );
        assert!(
            stderr.contains("scratch file"),
            "stderr should mention the scratch file path: {stderr}"
        );
        assert!(
            stdout.contains("Game of Life (streaming mode)"),
            "stdout should announce streaming: {stdout}"
        );
        assert!(
            stdout.contains("Streaming chunk:"),
            "stdout should report chunk dimensions: {stdout}"
        );
        assert!(stdout.contains("Simulation complete: 3 iterations"));
    }

    #[test]
    fn streaming_run_saves_final_board_with_save_board_flag() {
        let mut snapshot_path = std::env::temp_dir();
        snapshot_path.push(format!(
            "gol-streaming-cli-test-{}.gol-snapshot",
            std::process::id()
        ));
        let _ = fs::remove_file(&snapshot_path);

        let snapshot_str = snapshot_path.to_string_lossy().into_owned();
        let output = run_cli(&[
            "--board-size",
            "50x50",
            "--max-board-memory",
            "512B",
            "--initial-board",
            "blinker",
            "--max-iterations",
            "3",
            "--no-save",
            "--save-board",
            &snapshot_str,
        ]);

        assert!(
            output.status.success(),
            "streaming run should succeed; stderr: {}",
            stderr(&output)
        );
        let stdout = stdout(&output);
        assert!(
            stdout.contains("Saved final board snapshot"),
            "stdout should confirm snapshot save: {stdout}"
        );
        assert!(
            snapshot_path.exists(),
            "snapshot file should exist at {snapshot_path:?}"
        );

        let bytes = fs::read(&snapshot_path).expect("read snapshot");
        let text = String::from_utf8(bytes).expect("snapshot should be ASCII");
        assert!(
            text.starts_with("GOL-BOARD-SNAPSHOT v1"),
            "snapshot file should start with the expected magic: {}",
            text.lines().next().unwrap_or("")
        );
        assert!(
            text.contains("size: 50x50"),
            "snapshot should declare the board size: {text}"
        );

        fs::remove_file(&snapshot_path).ok();
    }

    #[test]
    fn streaming_run_warns_when_run_record_save_requested() {
        let output = run_cli(&[
            "--board-size",
            "50x50",
            "--max-board-memory",
            "512B",
            "--initial-board",
            "blinker",
            "--max-iterations",
            "1",
            "--save-run",
            "/tmp/this-path-should-not-be-written.gol",
        ]);

        assert!(
            output.status.success(),
            "streaming run should still succeed even when run-record save is requested"
        );
        let stderr = stderr(&output);
        assert!(
            stderr.contains("run-record save is not yet supported for streaming-sized boards"),
            "stderr should warn about run-record save being unsupported: {stderr}"
        );
        // The path should NOT have been written.
        assert!(
            !std::path::Path::new("/tmp/this-path-should-not-be-written.gol").exists(),
            "the run-record path should not have been created"
        );
    }

    #[test]
    fn negative_save_board_with_replay_is_rejected() {
        // --save-board doesn't fit the --replay verb's semantics. Without
        // this conflict check the parser would silently drop the flag and
        // the user would never know — make the rejection explicit.
        let output = run_cli(&[
            "--replay",
            "some-path.gol",
            "--save-board",
            "should-error.gol-snapshot",
        ]);
        assert!(!output.status.success(), "should reject the combination");
        let stderr = stderr(&output);
        assert!(
            stderr.contains("--replay is a standalone verb"),
            "stderr should explain the conflict: {stderr}"
        );
    }

    #[test]
    fn negative_working_dir_with_extract_board_is_rejected() {
        let output = run_cli(&[
            "--extract-board",
            "some-record.gol",
            "--output",
            "out.gol-snapshot",
            "--working-dir",
            "/tmp",
        ]);
        assert!(!output.status.success(), "should reject the combination");
        let stderr = stderr(&output);
        assert!(
            stderr.contains("--extract-board is a standalone verb"),
            "stderr should explain the conflict: {stderr}"
        );
    }
}
