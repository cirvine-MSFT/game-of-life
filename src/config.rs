use std::fmt;

pub const DEFAULT_BOARD_WIDTH: usize = 5;
pub const DEFAULT_BOARD_HEIGHT: usize = 5;
pub const DEFAULT_MAX_ITERATIONS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardSize {
    pub width: usize,
    pub height: usize,
}

impl BoardSize {
    pub fn new(width: usize, height: usize) -> Result<Self, BoardSizeParseError> {
        if width == 0 {
            return Err(BoardSizeParseError::ZeroDimension { dimension: "width" });
        }
        if height == 0 {
            return Err(BoardSizeParseError::ZeroDimension {
                dimension: "height",
            });
        }
        if width.checked_mul(height).is_none() {
            return Err(BoardSizeParseError::BoardTooLarge { width, height });
        }
        Ok(Self { width, height })
    }

    pub fn parse(value: &str) -> Result<Self, BoardSizeParseError> {
        let trimmed = value.trim();
        let has_dimension_separator = trimmed.contains('x') || trimmed.contains('X');

        if !has_dimension_separator {
            if trimmed
                .chars()
                .any(|ch| !ch.is_ascii_digit() && !ch.is_ascii_whitespace())
            {
                return Err(BoardSizeParseError::UnsupportedSeparator {
                    value: value.to_string(),
                });
            }
            return Err(BoardSizeParseError::MissingSeparator {
                value: value.to_string(),
            });
        }

        let parts: Vec<&str> = trimmed.split(['x', 'X']).collect();
        if parts.len() > 2 {
            return Err(BoardSizeParseError::ExtraDimensions {
                value: value.to_string(),
            });
        }

        let width = parse_dimension(value, "width", parts[0])?;
        let height = parse_dimension(value, "height", parts[1])?;
        Self::new(width, height)
    }
}

impl Default for BoardSize {
    fn default() -> Self {
        Self {
            width: DEFAULT_BOARD_WIDTH,
            height: DEFAULT_BOARD_HEIGHT,
        }
    }
}

impl fmt::Display for BoardSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationConfig {
    pub board_size: BoardSize,
    pub max_iterations: usize,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            board_size: BoardSize::default(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    Run(SimulationConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    MissingOptionValue {
        option: String,
        expected: &'static str,
    },
    UnknownOption {
        option: String,
    },
    UnexpectedArgument {
        argument: String,
    },
    InvalidBoardSize(BoardSizeParseError),
    InvalidMaxIterations(IterationParseError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardSizeParseError {
    MissingSeparator {
        value: String,
    },
    MissingDimension {
        value: String,
        dimension: &'static str,
    },
    ExtraDimensions {
        value: String,
    },
    UnsupportedSeparator {
        value: String,
    },
    NegativeDimension {
        value: String,
        dimension: &'static str,
        component: String,
    },
    NonIntegerDimension {
        value: String,
        dimension: &'static str,
        component: String,
    },
    ZeroDimension {
        dimension: &'static str,
    },
    DimensionTooLarge {
        value: String,
        dimension: &'static str,
        component: String,
    },
    BoardTooLarge {
        width: usize,
        height: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IterationParseError {
    Empty,
    Negative { value: String },
    NonInteger { value: String },
    TooLarge { value: String },
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliCommand, ConfigError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut config = SimulationConfig::default();
    let mut args = args.into_iter().map(Into::into).peekable();

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return Ok(CliCommand::Help);
        }

        if arg == "--board-size" || arg == "-b" {
            let value = args.next().ok_or_else(|| ConfigError::MissingOptionValue {
                option: arg.clone(),
                expected: "a board size like 5x5",
            })?;
            config.board_size = BoardSize::parse(&value).map_err(ConfigError::InvalidBoardSize)?;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--board-size=") {
            config.board_size = BoardSize::parse(value).map_err(ConfigError::InvalidBoardSize)?;
            continue;
        }

        if arg == "--max-iterations" || arg == "-m" {
            let value = args.next().ok_or_else(|| ConfigError::MissingOptionValue {
                option: arg.clone(),
                expected: "a non-negative integer like 10",
            })?;
            config.max_iterations =
                parse_max_iterations(&value).map_err(ConfigError::InvalidMaxIterations)?;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--max-iterations=") {
            config.max_iterations =
                parse_max_iterations(value).map_err(ConfigError::InvalidMaxIterations)?;
            continue;
        }

        if arg.starts_with('-') {
            return Err(ConfigError::UnknownOption { option: arg });
        }

        return Err(ConfigError::UnexpectedArgument { argument: arg });
    }

    Ok(CliCommand::Run(config))
}

pub fn parse_max_iterations(value: &str) -> Result<usize, IterationParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(IterationParseError::Empty);
    }
    if trimmed.starts_with('-') {
        return Err(IterationParseError::Negative {
            value: value.to_string(),
        });
    }
    if !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(IterationParseError::NonInteger {
            value: value.to_string(),
        });
    }

    let parsed = trimmed
        .parse::<u128>()
        .map_err(|_| IterationParseError::TooLarge {
            value: value.to_string(),
        })?;
    if parsed > usize::MAX as u128 {
        return Err(IterationParseError::TooLarge {
            value: value.to_string(),
        });
    }

    Ok(parsed as usize)
}

fn parse_dimension(
    original_value: &str,
    dimension: &'static str,
    component: &str,
) -> Result<usize, BoardSizeParseError> {
    let trimmed = component.trim();
    if trimmed.is_empty() {
        return Err(BoardSizeParseError::MissingDimension {
            value: original_value.to_string(),
            dimension,
        });
    }
    if trimmed.starts_with('-') {
        return Err(BoardSizeParseError::NegativeDimension {
            value: original_value.to_string(),
            dimension,
            component: component.to_string(),
        });
    }
    if !trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(BoardSizeParseError::NonIntegerDimension {
            value: original_value.to_string(),
            dimension,
            component: component.to_string(),
        });
    }

    let parsed = trimmed
        .parse::<u128>()
        .map_err(|_| BoardSizeParseError::DimensionTooLarge {
            value: original_value.to_string(),
            dimension,
            component: component.to_string(),
        })?;
    if parsed > usize::MAX as u128 {
        return Err(BoardSizeParseError::DimensionTooLarge {
            value: original_value.to_string(),
            dimension,
            component: component.to_string(),
        });
    }

    Ok(parsed as usize)
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingOptionValue { option, expected } => {
                write!(f, "Option '{option}' requires {expected}.")
            }
            ConfigError::UnknownOption { option } => {
                write!(
                    f,
                    "Unknown option '{option}'. Use --help to see supported options."
                )
            }
            ConfigError::UnexpectedArgument { argument } => {
                write!(
                    f,
                    "Unexpected argument '{argument}'. Use named options like --board-size 5x5."
                )
            }
            ConfigError::InvalidBoardSize(error) => write!(f, "{error}"),
            ConfigError::InvalidMaxIterations(error) => write!(f, "{error}"),
        }
    }
}

impl fmt::Display for BoardSizeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoardSizeParseError::MissingSeparator { value } => write!(
                f,
                "Board size '{value}' is missing the 'x' separator; use WIDTHxHEIGHT, for example 5x5."
            ),
            BoardSizeParseError::MissingDimension { value, dimension } => write!(
                f,
                "Board size '{value}' is missing a {dimension}; use WIDTHxHEIGHT, for example 5x5."
            ),
            BoardSizeParseError::ExtraDimensions { value } => write!(
                f,
                "Board size '{value}' has too many dimensions; only 2D boards are supported right now. Use WIDTHxHEIGHT, for example 5x5."
            ),
            BoardSizeParseError::UnsupportedSeparator { value } => write!(
                f,
                "Board size '{value}' uses an unsupported separator; use 'x' as in 5x5."
            ),
            BoardSizeParseError::NegativeDimension {
                value,
                dimension,
                component,
            } => write!(
                f,
                "Board size '{value}' has a negative {dimension} ('{}'); dimensions must be positive whole numbers.",
                component.trim()
            ),
            BoardSizeParseError::NonIntegerDimension {
                value,
                dimension,
                component,
            } => write!(
                f,
                "Board size '{value}' has a non-integer {dimension} ('{}'); use positive whole numbers like 5x5.",
                component.trim()
            ),
            BoardSizeParseError::ZeroDimension { dimension } => write!(
                f,
                "Board size has zero {dimension}; {dimension} must be greater than 0."
            ),
            BoardSizeParseError::DimensionTooLarge {
                value,
                dimension,
                component,
            } => write!(
                f,
                "Board size '{value}' has a {dimension} ('{}') that is too large for this platform.",
                component.trim()
            ),
            BoardSizeParseError::BoardTooLarge { width, height } => write!(
                f,
                "Board size '{width}x{height}' is too large; width times height exceeds the supported board capacity."
            ),
        }
    }
}

impl fmt::Display for IterationParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IterationParseError::Empty => write!(
                f,
                "Max iterations is empty; use a non-negative integer like 10."
            ),
            IterationParseError::Negative { value } => write!(
                f,
                "Max iterations '{value}' is negative; use 0 or a positive whole number."
            ),
            IterationParseError::NonInteger { value } => write!(
                f,
                "Max iterations '{value}' is not an integer; use a non-negative whole number like 10."
            ),
            IterationParseError::TooLarge { value } => write!(
                f,
                "Max iterations '{value}' is too large for this platform."
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl std::error::Error for BoardSizeParseError {}

impl std::error::Error for IterationParseError {}
