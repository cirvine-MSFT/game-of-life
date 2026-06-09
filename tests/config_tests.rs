use game_of_life::{
    parse_cli_args, BoardSize, BoardSizeParseError, CliCommand, ConfigError, IterationParseError,
    SimulationConfig,
};

mod normal_tests {
    use super::*;

    #[test]
    fn default_config_uses_five_by_five_and_ten_iterations() {
        assert_eq!(
            SimulationConfig::default(),
            SimulationConfig {
                board_size: BoardSize {
                    width: 5,
                    height: 5
                },
                max_iterations: 10,
            }
        );
    }

    #[test]
    fn parses_long_cli_options_into_run_config() {
        let command = parse_cli_args(["--board-size", "2x3", "--max-iterations", "4"]);

        assert_eq!(
            command,
            Ok(CliCommand::Run(SimulationConfig {
                board_size: BoardSize {
                    width: 2,
                    height: 3
                },
                max_iterations: 4,
            }))
        );
    }

    #[test]
    fn parses_short_cli_options_into_run_config() {
        let command = parse_cli_args(["-b", "3x4", "-m", "5"]);

        assert_eq!(
            command,
            Ok(CliCommand::Run(SimulationConfig {
                board_size: BoardSize {
                    width: 3,
                    height: 4
                },
                max_iterations: 5,
            }))
        );
    }

    #[test]
    fn parses_equals_form_options_into_run_config() {
        let command = parse_cli_args(["--board-size=4x5", "--max-iterations=6"]);

        assert_eq!(
            command,
            Ok(CliCommand::Run(SimulationConfig {
                board_size: BoardSize {
                    width: 4,
                    height: 5
                },
                max_iterations: 6,
            }))
        );
    }

    #[test]
    fn parses_help_command() {
        assert_eq!(parse_cli_args(["--help"]), Ok(CliCommand::Help));
        assert_eq!(parse_cli_args(["-h"]), Ok(CliCommand::Help));
    }
}

mod edge_case_tests {
    use super::*;

    #[test]
    fn edge_case_one_by_one_board_size_is_valid() {
        assert_eq!(
            BoardSize::parse("1x1"),
            Ok(BoardSize {
                width: 1,
                height: 1
            })
        );
    }

    #[test]
    fn edge_case_board_size_accepts_uppercase_separator_and_whitespace() {
        assert_eq!(
            BoardSize::parse(" 2X3 "),
            Ok(BoardSize {
                width: 2,
                height: 3
            })
        );
    }

    #[test]
    fn edge_case_zero_max_iterations_is_valid() {
        let command = parse_cli_args(["--max-iterations", "0"]);

        assert_eq!(
            command,
            Ok(CliCommand::Run(SimulationConfig {
                board_size: BoardSize::default(),
                max_iterations: 0,
            }))
        );
    }
}

mod negative_tests {
    use super::*;

    #[test]
    fn negative_board_size_zero_width_has_actionable_error() {
        let error = BoardSize::parse("0x2").expect_err("zero width should fail");

        assert_eq!(
            error,
            BoardSizeParseError::ZeroDimension { dimension: "width" }
        );
        assert!(error.to_string().contains("zero width"));
        assert!(error.to_string().contains("greater than 0"));
    }

    #[test]
    fn negative_board_size_zero_height_has_actionable_error() {
        let error = BoardSize::parse("2x0").expect_err("zero height should fail");

        assert_eq!(
            error,
            BoardSizeParseError::ZeroDimension {
                dimension: "height"
            }
        );
        assert!(error.to_string().contains("zero height"));
        assert!(error.to_string().contains("greater than 0"));
    }

    #[test]
    fn negative_board_size_negative_width_has_actionable_error() {
        let error = BoardSize::parse("-1x2").expect_err("negative width should fail");

        assert_eq!(
            error,
            BoardSizeParseError::NegativeDimension {
                value: "-1x2".to_string(),
                dimension: "width",
                component: "-1".to_string(),
            }
        );
        assert!(error.to_string().contains("negative width"));
        assert!(error.to_string().contains("positive whole numbers"));
    }

    #[test]
    fn negative_board_size_non_integer_dimension_has_actionable_error() {
        let error = BoardSize::parse("2.5x3").expect_err("decimal width should fail");

        assert_eq!(
            error,
            BoardSizeParseError::NonIntegerDimension {
                value: "2.5x3".to_string(),
                dimension: "width",
                component: "2.5".to_string(),
            }
        );
        assert!(error.to_string().contains("non-integer width"));
        assert!(error.to_string().contains("positive whole numbers"));
    }

    #[test]
    fn negative_board_size_word_dimension_has_actionable_error() {
        let error = BoardSize::parse("twox2").expect_err("word width should fail");

        assert_eq!(
            error,
            BoardSizeParseError::NonIntegerDimension {
                value: "twox2".to_string(),
                dimension: "width",
                component: "two".to_string(),
            }
        );
        assert!(error.to_string().contains("non-integer width"));
    }

    #[test]
    fn negative_board_size_missing_width_has_actionable_error() {
        let error = BoardSize::parse("x2").expect_err("missing width should fail");

        assert_eq!(
            error,
            BoardSizeParseError::MissingDimension {
                value: "x2".to_string(),
                dimension: "width",
            }
        );
        assert!(error.to_string().contains("missing a width"));
    }

    #[test]
    fn negative_board_size_missing_height_has_actionable_error() {
        let error = BoardSize::parse("2x").expect_err("missing height should fail");

        assert_eq!(
            error,
            BoardSizeParseError::MissingDimension {
                value: "2x".to_string(),
                dimension: "height",
            }
        );
        assert!(error.to_string().contains("missing a height"));
    }

    #[test]
    fn negative_board_size_extra_dimension_has_actionable_error() {
        let error = BoardSize::parse("2x2x2").expect_err("extra dimension should fail");

        assert_eq!(
            error,
            BoardSizeParseError::ExtraDimensions {
                value: "2x2x2".to_string()
            }
        );
        assert!(error.to_string().contains("too many dimensions"));
        assert!(error.to_string().contains("only 2D boards"));
    }

    #[test]
    fn negative_board_size_unsupported_separator_has_actionable_error() {
        let error = BoardSize::parse("2,2").expect_err("unsupported separator should fail");

        assert_eq!(
            error,
            BoardSizeParseError::UnsupportedSeparator {
                value: "2,2".to_string()
            }
        );
        assert!(error.to_string().contains("unsupported separator"));
        assert!(error.to_string().contains("use 'x'"));
    }

    #[test]
    fn negative_board_size_missing_separator_has_actionable_error() {
        let error = BoardSize::parse("22").expect_err("missing separator should fail");

        assert_eq!(
            error,
            BoardSizeParseError::MissingSeparator {
                value: "22".to_string()
            }
        );
        assert!(error.to_string().contains("missing the 'x' separator"));
    }

    #[test]
    fn negative_max_iterations_negative_value_has_actionable_error() {
        let error = parse_cli_args(["--max-iterations", "-1"])
            .expect_err("negative iteration count should fail");

        assert_eq!(
            error,
            ConfigError::InvalidMaxIterations(IterationParseError::Negative {
                value: "-1".to_string()
            })
        );
        assert!(error.to_string().contains("negative"));
        assert!(error.to_string().contains("0 or a positive whole number"));
    }

    #[test]
    fn negative_max_iterations_non_integer_value_has_actionable_error() {
        let error = parse_cli_args(["--max-iterations", "1.5"])
            .expect_err("decimal iteration count should fail");

        assert_eq!(
            error,
            ConfigError::InvalidMaxIterations(IterationParseError::NonInteger {
                value: "1.5".to_string()
            })
        );
        assert!(error.to_string().contains("not an integer"));
        assert!(error.to_string().contains("non-negative whole number"));
    }

    #[test]
    fn negative_missing_option_value_has_actionable_error() {
        let error = parse_cli_args(["--board-size"]).expect_err("missing option value should fail");

        assert_eq!(
            error,
            ConfigError::MissingOptionValue {
                option: "--board-size".to_string(),
                expected: "a board size like 5x5",
            }
        );
        assert!(error.to_string().contains("requires"));
    }

    #[test]
    fn negative_unknown_option_has_actionable_error() {
        let error = parse_cli_args(["--unknown"]).expect_err("unknown option should fail");

        assert_eq!(
            error,
            ConfigError::UnknownOption {
                option: "--unknown".to_string()
            }
        );
        assert!(error.to_string().contains("Unknown option"));
        assert!(error.to_string().contains("--help"));
    }
}
