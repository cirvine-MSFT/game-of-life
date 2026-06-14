use std::fmt;
use std::path::PathBuf;

use crate::board::{BoardSize, BoardSizeParseError};
use crate::persistence;

pub const DEFAULT_MAX_ITERATIONS: usize = 10;
pub const DEFAULT_MAX_BOARD_MEMORY_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulationConfig {
    pub board_size: Option<BoardSize>,
    pub max_iterations: Option<usize>,
    pub max_board_memory_bytes: usize,
    pub initial_board: InitialBoardSpec,
    pub save: SaveSettings,
    pub integrity: IntegrityMode,
    pub max_input_file_bytes: usize,
    /// Working directory for the streaming board's scratch file. `None`
    /// means use the OS temp directory. Only consulted when the run
    /// auto-promotes to streaming mode.
    pub working_dir: Option<PathBuf>,
    /// Explicit path to save the final board as a standalone snapshot.
    /// Independent of `--save-run`. Useful when the board is too large
    /// to embed in a run record (streaming mode).
    pub save_board_path: Option<PathBuf>,
    /// Non-fatal warnings raised during CLI parsing (e.g. an option that was
    /// silently overridden by a higher-precedence option). `main.rs` prints
    /// these to stderr before starting the run so the user sees them in
    /// context.
    pub warnings: Vec<String>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            board_size: None,
            max_iterations: None,
            max_board_memory_bytes: DEFAULT_MAX_BOARD_MEMORY_BYTES,
            initial_board: InitialBoardSpec::Initializer(InitialBoardSource::default()),
            save: SaveSettings::default(),
            integrity: IntegrityMode::Enforce,
            max_input_file_bytes: persistence::DEFAULT_MAX_INPUT_FILE_BYTES,
            working_dir: None,
            save_board_path: None,
            warnings: Vec::new(),
        }
    }
}

impl SimulationConfig {
    /// Resolves the effective board size, applying the default when the user
    /// didn't explicitly pass `--board-size`.
    pub fn effective_board_size(&self) -> BoardSize {
        self.board_size.unwrap_or_default()
    }

    /// Resolves the effective max-iterations, applying the default when the
    /// user didn't explicitly pass `--max-iterations`.
    pub fn effective_max_iterations(&self) -> usize {
        self.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS)
    }
}

/// Specifies where the initial board for a run comes from. Either a
/// stateless built-in initializer, a snapshot file, or a continuation of a
/// prior run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitialBoardSpec {
    Initializer(InitialBoardSource),
    LoadFromFile {
        path: std::path::PathBuf,
        from: Option<LoadFrom>,
    },
    ContinueFromRun {
        path: std::path::PathBuf,
        budget: ContinuationBudget,
    },
}

/// How a `--continue` invocation expressed its iteration budget.
///
/// Both forms are accepted; only one may be provided per `--continue`. The
/// difference shows up at resolution time:
///
/// - `Additional(N)` means "run for N more generations" regardless of how many
///   the source already ran.
/// - `CumulativeMax(M)` means "run until the chain's total reaches M", i.e. the
///   continuation runs for `M - source.iterations_run` more. The resolver
///   rejects values where `M <= source.iterations_run` so the user gets a
///   clear error instead of a silent zero-iteration run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuationBudget {
    Additional(usize),
    CumulativeMax(usize),
}

impl InitialBoardSpec {
    /// Human-readable label used in run-record `initial_board_source` field.
    pub fn record_label(&self) -> String {
        match self {
            InitialBoardSpec::Initializer(source) => source.to_string(),
            InitialBoardSpec::LoadFromFile { path, from } => match from {
                Some(LoadFrom::Initial) => format!("load:{} (initial)", path.display()),
                Some(LoadFrom::Final) => format!("load:{} (final)", path.display()),
                None => format!("load:{}", path.display()),
            },
            InitialBoardSpec::ContinueFromRun { path, .. } => {
                format!("continue:{}", path.display())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadFrom {
    #[default]
    Initial,
    Final,
}

impl LoadFrom {
    pub fn parse(value: &str) -> Result<Self, LoadFromParseError> {
        match value.trim() {
            "initial" => Ok(LoadFrom::Initial),
            "final" => Ok(LoadFrom::Final),
            other => Err(LoadFromParseError::Unsupported {
                value: other.to_string(),
            }),
        }
    }
}

impl fmt::Display for LoadFrom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadFrom::Initial => f.write_str("initial"),
            LoadFrom::Final => f.write_str("final"),
        }
    }
}

impl std::str::FromStr for LoadFrom {
    type Err = LoadFromParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadFromParseError {
    Unsupported { value: String },
}

impl fmt::Display for LoadFromParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadFromParseError::Unsupported { value } => write!(
                f,
                "--load-from value '{value}' is not supported; use 'initial' or 'final'."
            ),
        }
    }
}

impl std::error::Error for LoadFromParseError {}

/// Where (and whether) to write the run record at the end of a simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveSettings {
    /// Auto-save into a directory; filename derived from timestamp + run id.
    AutoIntoDir(std::path::PathBuf),
    /// Save to an explicit path; refuses to overwrite.
    ExplicitFile(std::path::PathBuf),
    /// Suppressed via `--no-save`.
    Suppressed,
}

impl Default for SaveSettings {
    fn default() -> Self {
        SaveSettings::AutoIntoDir(std::path::PathBuf::from(DEFAULT_RUNS_DIR))
    }
}

pub const DEFAULT_RUNS_DIR: &str = "runs";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IntegrityMode {
    #[default]
    Enforce,
    Ignore,
}

impl IntegrityMode {
    pub fn to_content_hash_mode(self) -> persistence::ContentHashMode {
        match self {
            IntegrityMode::Enforce => persistence::ContentHashMode::Enforce,
            IntegrityMode::Ignore => persistence::ContentHashMode::Ignore,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InitialBoardSource {
    #[default]
    Demo,
    Alive,
    Blinker,
    Random,
}

impl InitialBoardSource {
    pub const SUPPORTED_VALUES: &'static str = "demo, alive, blinker, random";

    pub fn parse(value: &str) -> Result<Self, InitialBoardSourceParseError> {
        match value.trim() {
            "demo" => Ok(Self::Demo),
            "alive" => Ok(Self::Alive),
            "blinker" => Ok(Self::Blinker),
            "random" => Ok(Self::Random),
            _ => Err(InitialBoardSourceParseError::Unsupported {
                value: value.to_string(),
            }),
        }
    }
}

impl fmt::Display for InitialBoardSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            InitialBoardSource::Demo => "demo",
            InitialBoardSource::Alive => "alive",
            InitialBoardSource::Blinker => "blinker",
            InitialBoardSource::Random => "random",
        };
        write!(f, "{value}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    Run(SimulationConfig),
    Replay(ReplayConfig),
    ExtractBoard(ExtractBoardConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayConfig {
    pub source: PathBuf,
    pub max_board_memory_bytes: usize,
    pub max_input_file_bytes: usize,
    pub integrity: IntegrityMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractBoardConfig {
    pub source: PathBuf,
    pub which: LoadFrom,
    pub output: PathBuf,
    pub max_board_memory_bytes: usize,
    pub max_input_file_bytes: usize,
    pub integrity: IntegrityMode,
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
    InvalidMaxBoardMemory(MemorySizeParseError),
    InvalidMaxInputFileBytes(MemorySizeParseError),
    InvalidInitialBoard(InitialBoardSourceParseError),
    InvalidLoadFrom(LoadFromParseError),
    ConflictingInitialBoardOptions {
        details: &'static str,
    },
    ConflictingSaveOptions {
        details: &'static str,
    },
    ConflictingCommands {
        details: &'static str,
    },
    MissingRequiredOption {
        option: &'static str,
        context: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IterationParseError {
    Empty,
    Negative { value: String },
    NonInteger { value: String },
    TooLarge { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemorySizeParseError {
    Empty,
    Negative { value: String },
    NonInteger { value: String },
    Zero { value: String },
    UnknownUnit { value: String, unit: String },
    TooLarge { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitialBoardSourceParseError {
    Unsupported { value: String },
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliCommand, ConfigError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let raw: Vec<String> = args.into_iter().map(Into::into).collect();
    let mut args = raw.into_iter().peekable();

    // Pending option tracking across the loop.
    let mut board_size: Option<BoardSize> = None;
    let mut max_iterations: Option<usize> = None;
    let mut max_board_memory_bytes: usize = DEFAULT_MAX_BOARD_MEMORY_BYTES;
    let mut max_input_file_bytes: usize = persistence::DEFAULT_MAX_INPUT_FILE_BYTES;
    let mut initial_board_named: Option<InitialBoardSource> = None;
    let mut load_board_path: Option<PathBuf> = None;
    let mut load_from: Option<LoadFrom> = None;
    let mut continue_path: Option<PathBuf> = None;
    let mut additional_iterations: Option<usize> = None;
    let mut save_run_path: Option<PathBuf> = None;
    let mut runs_dir: Option<PathBuf> = None;
    let mut no_save = false;
    let mut working_dir: Option<PathBuf> = None;
    let mut save_board_path: Option<PathBuf> = None;
    let mut integrity = IntegrityMode::Enforce;
    let mut replay_path: Option<PathBuf> = None;
    let mut extract_board_path: Option<PathBuf> = None;
    let mut extract_output_path: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return Ok(CliCommand::Help);
        }

        // Helper closures.
        let take_value = |peek: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
                          option: &str,
                          expected: &'static str|
         -> Result<String, ConfigError> {
            peek.next().ok_or_else(|| ConfigError::MissingOptionValue {
                option: option.to_string(),
                expected,
            })
        };

        if arg == "--board-size" || arg == "-b" {
            let value = take_value(&mut args, &arg, "a board size like 5x5")?;
            board_size = Some(BoardSize::parse(&value).map_err(ConfigError::InvalidBoardSize)?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--board-size=") {
            board_size = Some(BoardSize::parse(value).map_err(ConfigError::InvalidBoardSize)?);
            continue;
        }

        if arg == "--max-iterations" || arg == "-m" {
            let value = take_value(&mut args, &arg, "a non-negative integer like 10")?;
            max_iterations =
                Some(parse_max_iterations(&value).map_err(ConfigError::InvalidMaxIterations)?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--max-iterations=") {
            max_iterations =
                Some(parse_max_iterations(value).map_err(ConfigError::InvalidMaxIterations)?);
            continue;
        }

        if arg == "--max-board-memory" {
            let value = take_value(&mut args, &arg, "a memory size like 64MB")?;
            max_board_memory_bytes =
                parse_memory_size(&value).map_err(ConfigError::InvalidMaxBoardMemory)?;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--max-board-memory=") {
            max_board_memory_bytes =
                parse_memory_size(value).map_err(ConfigError::InvalidMaxBoardMemory)?;
            continue;
        }

        if arg == "--max-input-file-bytes" {
            let value = take_value(&mut args, &arg, "a memory size like 256MB")?;
            max_input_file_bytes =
                parse_memory_size(&value).map_err(ConfigError::InvalidMaxInputFileBytes)?;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--max-input-file-bytes=") {
            max_input_file_bytes =
                parse_memory_size(value).map_err(ConfigError::InvalidMaxInputFileBytes)?;
            continue;
        }

        if arg == "--initial-board" {
            let value = take_value(
                &mut args,
                &arg,
                "an initial board source like demo, alive, blinker, or random",
            )?;
            initial_board_named =
                Some(InitialBoardSource::parse(&value).map_err(ConfigError::InvalidInitialBoard)?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--initial-board=") {
            initial_board_named =
                Some(InitialBoardSource::parse(value).map_err(ConfigError::InvalidInitialBoard)?);
            continue;
        }

        if arg == "--load-board" {
            let value = take_value(&mut args, &arg, "a path to a .gol file")?;
            load_board_path = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--load-board=") {
            load_board_path = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--load-from" {
            let value = take_value(&mut args, &arg, "'initial' or 'final'")?;
            load_from = Some(LoadFrom::parse(&value).map_err(ConfigError::InvalidLoadFrom)?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--load-from=") {
            load_from = Some(LoadFrom::parse(value).map_err(ConfigError::InvalidLoadFrom)?);
            continue;
        }

        if arg == "--continue" {
            let value = take_value(&mut args, &arg, "a path to a run record file")?;
            continue_path = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--continue=") {
            continue_path = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--additional-iterations" {
            let value = take_value(&mut args, &arg, "a non-negative integer like 100")?;
            additional_iterations =
                Some(parse_max_iterations(&value).map_err(ConfigError::InvalidMaxIterations)?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--additional-iterations=") {
            additional_iterations =
                Some(parse_max_iterations(value).map_err(ConfigError::InvalidMaxIterations)?);
            continue;
        }

        if arg == "--save-run" {
            let value = take_value(&mut args, &arg, "an output file path")?;
            save_run_path = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--save-run=") {
            save_run_path = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--runs-dir" {
            let value = take_value(&mut args, &arg, "a directory path")?;
            runs_dir = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--runs-dir=") {
            runs_dir = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--no-save" {
            no_save = true;
            continue;
        }

        if arg == "--ignore-integrity" {
            integrity = IntegrityMode::Ignore;
            continue;
        }

        if arg == "--working-dir" {
            let value = take_value(&mut args, &arg, "a path to a directory")?;
            working_dir = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--working-dir=") {
            working_dir = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--save-board" {
            let value = take_value(&mut args, &arg, "a path to save the final board snapshot")?;
            save_board_path = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--save-board=") {
            save_board_path = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--replay" {
            let value = take_value(&mut args, &arg, "a path to a run record file")?;
            replay_path = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--replay=") {
            replay_path = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--extract-board" {
            let value = take_value(&mut args, &arg, "a path to a run record file")?;
            extract_board_path = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--extract-board=") {
            extract_board_path = Some(PathBuf::from(value));
            continue;
        }

        if arg == "--output" {
            let value = take_value(&mut args, &arg, "an output file path")?;
            extract_output_path = Some(PathBuf::from(value));
            continue;
        }
        if let Some(value) = arg.strip_prefix("--output=") {
            extract_output_path = Some(PathBuf::from(value));
            continue;
        }

        if arg.starts_with('-') {
            return Err(ConfigError::UnknownOption { option: arg });
        }
        return Err(ConfigError::UnexpectedArgument { argument: arg });
    }

    // Verb dispatch: --replay and --extract-board are mutually exclusive
    // verbs that supersede a normal run.
    if replay_path.is_some() && extract_board_path.is_some() {
        return Err(ConfigError::ConflictingCommands {
            details: "--replay and --extract-board cannot be combined; pick one",
        });
    }

    if let Some(source) = replay_path {
        let conflicting = load_board_path.is_some()
            || continue_path.is_some()
            || initial_board_named.is_some()
            || save_run_path.is_some()
            || runs_dir.is_some()
            || no_save
            || additional_iterations.is_some();
        if conflicting {
            return Err(ConfigError::ConflictingCommands {
                details: "--replay is a standalone verb; do not combine with run/load/continue/save options",
            });
        }
        return Ok(CliCommand::Replay(ReplayConfig {
            source,
            max_board_memory_bytes,
            max_input_file_bytes,
            integrity,
        }));
    }

    if let Some(source) = extract_board_path {
        let output = extract_output_path.ok_or(ConfigError::MissingRequiredOption {
            option: "--output",
            context: "required when using --extract-board",
        })?;
        let which = load_from.unwrap_or(LoadFrom::Initial);
        let conflicting = load_board_path.is_some()
            || continue_path.is_some()
            || initial_board_named.is_some()
            || save_run_path.is_some()
            || runs_dir.is_some()
            || no_save
            || additional_iterations.is_some();
        if conflicting {
            return Err(ConfigError::ConflictingCommands {
                details:
                    "--extract-board is a standalone verb; do not combine with run/load/continue/save options",
            });
        }
        return Ok(CliCommand::ExtractBoard(ExtractBoardConfig {
            source,
            which,
            output,
            max_board_memory_bytes,
            max_input_file_bytes,
            integrity,
        }));
    }

    // From here on, it's a Run command. Decide initial-board spec.
    let mut continue_used = false;
    let mut warnings: Vec<String> = Vec::new();
    let initial_board = match (continue_path, load_board_path, initial_board_named) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
            return Err(ConfigError::ConflictingCommands {
                details: "--continue is mutually exclusive with --load-board and --initial-board",
            });
        }
        (None, Some(path), Some(_)) => {
            warnings.push(
                "Warning: --load-board takes precedence; --initial-board ignored.".to_string(),
            );
            InitialBoardSpec::LoadFromFile {
                path,
                from: load_from,
            }
        }
        (Some(path), None, None) => {
            continue_used = true;
            let budget = match (additional_iterations, max_iterations) {
                (Some(_), Some(_)) => {
                    return Err(ConfigError::ConflictingCommands {
                        details:
                            "--additional-iterations and --max-iterations are mutually exclusive with --continue; pick one (--additional-iterations N means N more steps; --max-iterations M means total chain target M)",
                    });
                }
                (Some(a), None) => ContinuationBudget::Additional(a),
                (None, Some(m)) => ContinuationBudget::CumulativeMax(m),
                (None, None) => {
                    return Err(ConfigError::MissingRequiredOption {
                        option: "--additional-iterations or --max-iterations",
                        context:
                            "required when using --continue (either N more steps, or a cumulative chain target)",
                    });
                }
            };
            // The cumulative branch consumes --max-iterations into the
            // continuation budget; clear it so the Run config's
            // max_iterations slot stays None for replay/honesty.
            max_iterations = None;
            InitialBoardSpec::ContinueFromRun { path, budget }
        }
        (None, Some(path), None) => InitialBoardSpec::LoadFromFile {
            path,
            from: load_from,
        },
        (None, None, Some(named)) => InitialBoardSpec::Initializer(named),
        (None, None, None) => InitialBoardSpec::Initializer(InitialBoardSource::default()),
    };

    if !continue_used && additional_iterations.is_some() {
        return Err(ConfigError::ConflictingCommands {
            details: "--additional-iterations is only valid together with --continue",
        });
    }
    if load_from.is_some() && !matches!(initial_board, InitialBoardSpec::LoadFromFile { .. }) {
        return Err(ConfigError::ConflictingCommands {
            details: "--load-from is only valid together with --load-board",
        });
    }

    // Save settings.
    let save = match (no_save, save_run_path, runs_dir) {
        (true, _, _) => SaveSettings::Suppressed,
        (false, Some(path), _) => SaveSettings::ExplicitFile(path),
        (false, None, Some(dir)) => SaveSettings::AutoIntoDir(dir),
        (false, None, None) => SaveSettings::AutoIntoDir(PathBuf::from(DEFAULT_RUNS_DIR)),
    };

    Ok(CliCommand::Run(SimulationConfig {
        board_size,
        max_iterations,
        max_board_memory_bytes,
        initial_board,
        save,
        integrity,
        max_input_file_bytes,
        working_dir,
        save_board_path,
        warnings,
    }))
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

pub fn parse_memory_size(value: &str) -> Result<usize, MemorySizeParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MemorySizeParseError::Empty);
    }
    if trimmed.starts_with('-') {
        return Err(MemorySizeParseError::Negative {
            value: value.to_string(),
        });
    }

    let number_end = trimmed
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
        .unwrap_or(0);

    if number_end == 0 {
        return Err(MemorySizeParseError::NonInteger {
            value: value.to_string(),
        });
    }

    let number_component = &trimmed[..number_end];
    let unit_component = trimmed[number_end..].trim();
    if unit_component.starts_with('.') {
        return Err(MemorySizeParseError::NonInteger {
            value: value.to_string(),
        });
    }

    let parsed = number_component
        .parse::<u128>()
        .map_err(|_| MemorySizeParseError::TooLarge {
            value: value.to_string(),
        })?;
    if parsed == 0 {
        return Err(MemorySizeParseError::Zero {
            value: value.to_string(),
        });
    }

    let multiplier = match unit_component.to_ascii_uppercase().as_str() {
        "" | "B" => 1_u128,
        "KB" => 1024_u128,
        "MB" => 1024_u128 * 1024,
        "GB" => 1024_u128 * 1024 * 1024,
        _ => {
            return Err(MemorySizeParseError::UnknownUnit {
                value: value.to_string(),
                unit: unit_component.to_string(),
            });
        }
    };

    let bytes = parsed
        .checked_mul(multiplier)
        .ok_or_else(|| MemorySizeParseError::TooLarge {
            value: value.to_string(),
        })?;
    if bytes > usize::MAX as u128 {
        return Err(MemorySizeParseError::TooLarge {
            value: value.to_string(),
        });
    }

    Ok(bytes as usize)
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
            ConfigError::InvalidMaxBoardMemory(error) => write!(f, "{error}"),
            ConfigError::InvalidMaxInputFileBytes(error) => {
                write!(
                    f,
                    "Option '--max-input-file-bytes' rejected the supplied value: "
                )?;
                match error {
                    MemorySizeParseError::Empty => write!(
                        f,
                        "value is empty; use a positive memory size like 256MB."
                    ),
                    MemorySizeParseError::Negative { value } => write!(
                        f,
                        "value '{value}' is negative; use a positive memory size like 256MB."
                    ),
                    MemorySizeParseError::NonInteger { value } => write!(
                        f,
                        "value '{value}' is not a whole-number size; use values like 64KB, 256MB, 1GB, or raw bytes."
                    ),
                    MemorySizeParseError::Zero { value } => write!(
                        f,
                        "value '{value}' is zero; use a size greater than 0 bytes."
                    ),
                    MemorySizeParseError::UnknownUnit { value, unit } => write!(
                        f,
                        "value '{value}' uses unsupported unit '{unit}'; supported units are B, KB, MB, and GB."
                    ),
                    MemorySizeParseError::TooLarge { value } => write!(
                        f,
                        "value '{value}' is too large for this platform."
                    ),
                }
            }
            ConfigError::InvalidInitialBoard(error) => write!(f, "{error}"),
            ConfigError::InvalidLoadFrom(error) => write!(f, "{error}"),
            ConfigError::ConflictingInitialBoardOptions { details }
            | ConfigError::ConflictingSaveOptions { details }
            | ConfigError::ConflictingCommands { details } => {
                write!(f, "Conflicting options: {details}.")
            }
            ConfigError::MissingRequiredOption { option, context } => {
                write!(f, "Option '{option}' is {context}.")
            }
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

impl fmt::Display for MemorySizeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemorySizeParseError::Empty => write!(
                f,
                "Max board memory is empty; use a positive memory size like 64MB."
            ),
            MemorySizeParseError::Negative { value } => write!(
                f,
                "Max board memory '{value}' is negative; use a positive memory size like 64MB."
            ),
            MemorySizeParseError::NonInteger { value } => write!(
                f,
                "Max board memory '{value}' is not a whole-number size; use values like 64KB, 64MB, 1GB, or raw bytes."
            ),
            MemorySizeParseError::Zero { value } => write!(
                f,
                "Max board memory '{value}' is zero; use a size greater than 0 bytes."
            ),
            MemorySizeParseError::UnknownUnit { value, unit } => write!(
                f,
                "Max board memory '{value}' uses unsupported unit '{unit}'; supported units are B, KB, MB, and GB."
            ),
            MemorySizeParseError::TooLarge { value } => write!(
                f,
                "Max board memory '{value}' is too large for this platform."
            ),
        }
    }
}

impl fmt::Display for InitialBoardSourceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitialBoardSourceParseError::Unsupported { value } => write!(
                f,
                "Initial board source '{value}' is not supported; use one of: {}. File-based initial boards are planned but not supported yet.",
                InitialBoardSource::SUPPORTED_VALUES
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl std::error::Error for IterationParseError {}

impl std::error::Error for MemorySizeParseError {}

impl std::error::Error for InitialBoardSourceParseError {}
