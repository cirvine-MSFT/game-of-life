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
    }

    #[test]
    fn valid_run_prints_concise_final_state_only() {
        let output = run_cli(&["--board-size", "2x2", "--max-iterations", "1"]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Board size: 2x2"));
        assert!(stdout.contains("Max iterations: 1"));
        assert!(stdout.contains("Final board state:"));
        assert!(stdout.contains("..\n..\n"));
        assert!(stdout.contains("Simulation complete: 1 iterations"));
        assert!(!stdout.contains("Generation 1:"));
    }

    #[test]
    fn zero_iteration_run_uses_default_centered_blinker_initializer() {
        let output = run_cli(&["--max-iterations", "0"]);

        assert!(output.status.success());
        assert!(stderr(&output).is_empty());

        let stdout = stdout(&output);
        assert!(stdout.contains("Board size: 5x5"));
        assert!(stdout.contains("Final board state:\n.....\n.....\n.###.\n.....\n.....\n"));
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
}
