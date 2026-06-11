use game_of_life::{
    parse_cli_args, parse_memory_size, BoardSize, BoardSizeParseError, CliCommand, ConfigError,
    InitialBoardSource, InitialBoardSourceParseError, IterationParseError, MemorySizeParseError,
    SimulationConfig, DEFAULT_MAX_BOARD_MEMORY_BYTES,
};

mod normal_tests {
    use super::*;

    #[test]
    fn default_config_uses_five_by_five_and_ten_iterations() {
        assert_eq!(
            SimulationConfig::default(),
            SimulationConfig {
                board_size: BoardSize {
                    width: 10,
                    height: 10
                },
                max_iterations: 10,
                max_board_memory_bytes: DEFAULT_MAX_BOARD_MEMORY_BYTES,
                initial_board: InitialBoardSource::Demo,
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
                ..SimulationConfig::default()
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
                ..SimulationConfig::default()
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
                ..SimulationConfig::default()
            }))
        );
    }

    #[test]
    fn parses_memory_budget_and_initial_board_options_into_run_config() {
        let command = parse_cli_args(["--max-board-memory", "64 MB", "--initial-board", "blinker"]);

        assert_eq!(
            command,
            Ok(CliCommand::Run(SimulationConfig {
                max_board_memory_bytes: 64 * 1024 * 1024,
                initial_board: InitialBoardSource::Blinker,
                ..SimulationConfig::default()
            }))
        );
    }

    #[test]
    fn parses_equals_form_memory_budget_and_initial_board_options() {
        let command = parse_cli_args(["--max-board-memory=1GB", "--initial-board=random"]);

        assert_eq!(
            command,
            Ok(CliCommand::Run(SimulationConfig {
                max_board_memory_bytes: 1024 * 1024 * 1024,
                initial_board: InitialBoardSource::Random,
                ..SimulationConfig::default()
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
                ..SimulationConfig::default()
            }))
        );
    }

    #[test]
    fn edge_case_memory_size_accepts_bytes_and_units() {
        assert_eq!(parse_memory_size("1024"), Ok(1024));
        assert_eq!(parse_memory_size("1B"), Ok(1));
        assert_eq!(parse_memory_size("64kb"), Ok(64 * 1024));
        assert_eq!(parse_memory_size("64 MB"), Ok(64 * 1024 * 1024));
    }

    #[test]
    fn edge_case_initial_board_source_parses_supported_values() {
        assert_eq!(
            InitialBoardSource::parse("demo"),
            Ok(InitialBoardSource::Demo)
        );
        assert_eq!(
            InitialBoardSource::parse("blinker"),
            Ok(InitialBoardSource::Blinker)
        );
        assert_eq!(
            InitialBoardSource::parse("random"),
            Ok(InitialBoardSource::Random)
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
    fn negative_board_size_larger_than_u128_has_actionable_error() {
        let value = "340282366920938463463374607431768211456x1";
        let error = BoardSize::parse(value).expect_err("u128 overflow should fail");

        assert_eq!(
            error,
            BoardSizeParseError::DimensionTooLarge {
                value: value.to_string(),
                dimension: "width",
                component: "340282366920938463463374607431768211456".to_string(),
            }
        );
        assert!(error.to_string().contains("too large for this platform"));
    }

    #[test]
    fn negative_board_size_larger_than_usize_has_actionable_error() {
        let component = (usize::MAX as u128 + 1).to_string();
        let value = format!("{component}x1");
        let error = BoardSize::parse(&value).expect_err("usize overflow should fail");

        assert_eq!(
            error,
            BoardSizeParseError::DimensionTooLarge {
                value,
                dimension: "width",
                component,
            }
        );
        assert!(error.to_string().contains("too large for this platform"));
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
    fn negative_missing_memory_budget_value_has_actionable_error() {
        let error =
            parse_cli_args(["--max-board-memory"]).expect_err("missing memory budget should fail");

        assert_eq!(
            error,
            ConfigError::MissingOptionValue {
                option: "--max-board-memory".to_string(),
                expected: "a memory size like 64MB",
            }
        );
        assert!(error.to_string().contains("requires"));
    }

    #[test]
    fn negative_memory_budget_zero_has_actionable_error() {
        let error = parse_cli_args(["--max-board-memory", "0"])
            .expect_err("zero memory budget should fail");

        assert_eq!(
            error,
            ConfigError::InvalidMaxBoardMemory(MemorySizeParseError::Zero {
                value: "0".to_string()
            })
        );
        assert!(error.to_string().contains("greater than 0 bytes"));
    }

    #[test]
    fn negative_memory_budget_decimal_has_actionable_error() {
        let error = parse_cli_args(["--max-board-memory", "1.5MB"])
            .expect_err("decimal memory budget should fail");

        assert_eq!(
            error,
            ConfigError::InvalidMaxBoardMemory(MemorySizeParseError::NonInteger {
                value: "1.5MB".to_string()
            })
        );
        assert!(error.to_string().contains("whole-number size"));
    }

    #[test]
    fn negative_memory_budget_unknown_unit_has_actionable_error() {
        let error =
            parse_cli_args(["--max-board-memory", "64TB"]).expect_err("unknown unit should fail");

        assert_eq!(
            error,
            ConfigError::InvalidMaxBoardMemory(MemorySizeParseError::UnknownUnit {
                value: "64TB".to_string(),
                unit: "TB".to_string(),
            })
        );
        assert!(error.to_string().contains("supported units"));
    }

    #[test]
    fn negative_memory_budget_too_large_has_actionable_error() {
        let value = format!("{}GB", usize::MAX as u128);
        let error = parse_cli_args(["--max-board-memory", &value])
            .expect_err("too large memory budget should fail");

        assert_eq!(
            error,
            ConfigError::InvalidMaxBoardMemory(MemorySizeParseError::TooLarge { value })
        );
        assert!(error.to_string().contains("too large for this platform"));
    }

    #[test]
    fn negative_initial_board_source_unknown_value_has_actionable_error() {
        let error = parse_cli_args(["--initial-board", "file:seed.txt"])
            .expect_err("unsupported initial board source should fail");

        assert_eq!(
            error,
            ConfigError::InvalidInitialBoard(InitialBoardSourceParseError::Unsupported {
                value: "file:seed.txt".to_string()
            })
        );
        assert!(error.to_string().contains("demo, blinker, random"));
        assert!(error.to_string().contains("planned"));
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
