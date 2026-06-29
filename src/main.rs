//! Console application for Game of Life.
//!
//! Parses CLI args, dispatches to one of the verbs (Run / Replay /
//! ExtractBoard), wires up the persistence/stats layers, and writes a run
//! record to disk on successful runs (unless --no-save was passed).

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, process};

use game_of_life::persistence::{
    board_grid_hash, extract_board_from_run, read_board_snapshot, read_run_record_with_warnings,
    write_board_snapshot, write_run_record, write_streaming_board_snapshot, BoardSnapshot,
    BoardSnapshotReadError, BoardSnapshotWriteError, ExtractBoardError, ExtractWhich, FileKind,
    RunId, RunRecord, RunRecordConfig, RunRecordReadError, RunRecordResult, RunRecordWriteError,
    RUN_RECORD_SCHEMA_VERSION, TOOL_VERSION,
};
use game_of_life::stats::{
    run_statistics::RunStatus, terminal_status_for_outcome, AdvanceOutcome, CycleStatistics,
    IterationSeries, RunStatistics, RunStatisticsCollector,
};
use game_of_life::{
    parse_cli_args, BlinkerBoardInitializer, BoardEditor, BoardInitializer, BoardSignatureSource,
    BoardSize, BoardUpdater, BoardView, CellCoordinate, CellState, CliCommand,
    DemoBoardInitializer, ExtractBoardConfig, FullyAliveInitializer, InMemoryBoard,
    InMemoryBoardCreationError, InPlaceTransitionalUpdater, InitialBoardSource, InitialBoardSpec,
    IntegrityMode, LoadFrom, PatternAnalyzer, PatternBackend, PatternMatchDetails,
    PatternObservation, RandomBoardInitializer, ReplayConfig, SaveSettings, SimulationConfig,
    StreamingBoard, StreamingBoardCreationError, StreamingBoardParams,
};

const HELP_TEXT: &str = concat!(
    "Game of Life\n",
    "\n",
    "Usage:\n",
    "  game-of-life [OPTIONS]\n",
    "  game-of-life --replay <FILE> [--ignore-integrity] [--max-board-memory <SIZE>]\n",
    "  game-of-life --extract-board <FILE> --output <FILE> [--load-from initial|final] [--ignore-integrity]\n",
    "\n",
    "Run options:\n",
    "  -h, --help                         Print this help message.\n",
    "  -b, --board-size <WIDTHxHEIGHT>    Set the 2D board size, for example 5x5.\n",
    "  -m, --max-iterations <COUNT>       Set generations to run; 0 prints the initial board.\n",
    "      --max-board-memory <SIZE>      Set max in-memory board budget, for example 64MB.\n",
    "      --max-input-file-bytes <SIZE>  Set ceiling on input file size; default 256MB.\n",
    "      --initial-board <SOURCE>       Set initial board source: demo, alive, blinker, or random.\n",
    "      --load-board <PATH>            Load the initial board from a .gol file.\n",
    "      --load-from initial|final      With --load-board, pick which block of a run record to use.\n",
    "      --continue <PATH>              Continue a prior run: load its FINAL board as the initial board.\n",
    "      --additional-iterations <N>    With --continue: run N more generations. Mutually exclusive with --max-iterations.\n",
    "                                     (With --continue, --max-iterations M instead targets a cumulative total of M iterations across the chain.)\n",
    "\n",
    "Streaming (large boards):\n",
    "      --working-dir <PATH>           Directory for the streaming scratch file; default OS temp dir.\n",
    "                                     Only used when the run auto-promotes to streaming mode (board exceeds --max-board-memory\n",
    "                                     and uses --initial-board, not --load-board or --continue).\n",
    "      --save-board <PATH>            Save the final board as a standalone snapshot. Works in both in-memory and\n",
    "                                     streaming modes. In streaming mode this is the primary way to persist the\n",
    "                                     final state, since run-record save is not yet supported for streaming boards.\n",
    "\n",
    "Save options:\n",
    "      --runs-dir <DIR>               Save run records into this directory; default ./runs.\n",
    "      --save-run <PATH>              Save the run record to this explicit path (no auto-naming).\n",
    "      --no-save                      Suppress saving the run record.\n",
    "\n",
    "Integrity:\n",
    "      --ignore-integrity             Bypass content_hash verification when reading a run record.\n",
    "\n",
    "Verbs:\n",
    "      --replay <PATH>                Re-execute the saved run and diff its final board and stats.\n",
    "      --extract-board <PATH>         Extract a board block from a run record as a snapshot.\n",
    "      --output <PATH>                Output path for --extract-board.\n",
    "\n",
    "Defaults:\n",
    "  --board-size 10x10\n",
    "  --max-iterations 10\n",
    "  --max-board-memory 64MB\n",
    "  --max-input-file-bytes 256MB\n",
    "  --initial-board demo\n",
    "  Save: auto into ./runs/<timestamp>-<short-run-id>.gol\n",
);

fn main() {
    match parse_cli_args(env::args().skip(1)) {
        Ok(CliCommand::Help) => print_help(),
        Ok(CliCommand::Run(config)) => {
            if let Err(error) = run_simulation(config) {
                eprintln!("Error: {error}");
                eprintln!("Use --help to see usage and supported options.");
                process::exit(2);
            }
        }
        Ok(CliCommand::Replay(config)) => match replay(&config) {
            Ok(ReplayOutcome::Match) => {
                println!("Replay matched the saved run record.");
            }
            Ok(ReplayOutcome::Mismatch(diff)) => {
                eprintln!("Error: replay diverged from the saved run record:");
                for line in diff {
                    eprintln!("  {line}");
                }
                process::exit(1);
            }
            Err(error) => {
                eprintln!("Error: {error}");
                process::exit(2);
            }
        },
        Ok(CliCommand::ExtractBoard(config)) => {
            if let Err(error) = run_extract(&config) {
                eprintln!("Error: {error}");
                process::exit(2);
            }
            println!(
                "Extracted {} board to '{}'.",
                config.which,
                config.output.display()
            );
        }
        Err(error) => {
            eprintln!("Error: {error}");
            eprintln!("Use --help to see usage and supported options.");
            process::exit(2);
        }
    }
}

fn print_help() {
    print!("{HELP_TEXT}");
}

// -------- run -----------------------------------------------------------

#[derive(Debug)]
#[allow(dead_code)] // some variants are reserved for the streaming-board PR
enum RunSimulationError {
    BoardCreation(InMemoryBoardCreationError),
    StreamingBoardCreation(StreamingBoardCreationError),
    StreamingIo {
        operation: &'static str,
        source: game_of_life::persistence::scratch::ScratchFileError,
    },
    SnapshotRead(BoardSnapshotReadError),
    SnapshotWrite(BoardSnapshotWriteError),
    RunRecordRead(RunRecordReadError),
    RunRecordWrite(RunRecordWriteError),
    Io {
        path: PathBuf,
        operation: &'static str,
        source: std::io::Error,
    },
    BoardSizeMismatch {
        from_file: BoardSize,
        from_cli: BoardSize,
    },
    CumulativeMaxTooSmall {
        cumulative_max: usize,
        source_iterations_run: u64,
    },
    MaxIterationsRequiredForRandomSeed,
}

impl std::fmt::Display for RunSimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunSimulationError::BoardCreation(e) => write!(f, "{e}"),
            RunSimulationError::StreamingBoardCreation(e) => write!(f, "{e}"),
            RunSimulationError::StreamingIo { operation, source } => {
                write!(f, "Streaming I/O error while {operation}: {source}")
            }
            RunSimulationError::SnapshotRead(e) => write!(f, "{e}"),
            RunSimulationError::SnapshotWrite(e) => write!(f, "{e}"),
            RunSimulationError::RunRecordRead(e) => write!(f, "{e}"),
            RunSimulationError::RunRecordWrite(e) => write!(f, "{e}"),
            RunSimulationError::Io {
                path,
                operation,
                source,
            } => write!(
                f,
                "I/O error while {operation} '{}': {source}",
                path.display()
            ),
            RunSimulationError::BoardSizeMismatch { from_file, from_cli } => write!(
                f,
                "Board size mismatch: file declares {from_file}, --board-size declares {from_cli}. Either drop --board-size or pass the matching size.",
            ),
            RunSimulationError::CumulativeMaxTooSmall {
                cumulative_max,
                source_iterations_run,
            } => write!(
                f,
                "--max-iterations {cumulative_max} is not greater than the source run's iterations_run ({source_iterations_run}); pick a larger total, or use --additional-iterations N to add N more steps."
            ),
            RunSimulationError::MaxIterationsRequiredForRandomSeed => write!(
                f,
                "Internal: max_iterations must be resolved before random seed generation."
            ),
        }
    }
}

impl std::error::Error for RunSimulationError {}

impl From<InMemoryBoardCreationError> for RunSimulationError {
    fn from(value: InMemoryBoardCreationError) -> Self {
        Self::BoardCreation(value)
    }
}
impl From<StreamingBoardCreationError> for RunSimulationError {
    fn from(value: StreamingBoardCreationError) -> Self {
        Self::StreamingBoardCreation(value)
    }
}
impl From<BoardSnapshotReadError> for RunSimulationError {
    fn from(value: BoardSnapshotReadError) -> Self {
        Self::SnapshotRead(value)
    }
}
impl From<BoardSnapshotWriteError> for RunSimulationError {
    fn from(value: BoardSnapshotWriteError) -> Self {
        Self::SnapshotWrite(value)
    }
}
impl From<RunRecordReadError> for RunSimulationError {
    fn from(value: RunRecordReadError) -> Self {
        Self::RunRecordRead(value)
    }
}
impl From<RunRecordWriteError> for RunSimulationError {
    fn from(value: RunRecordWriteError) -> Self {
        Self::RunRecordWrite(value)
    }
}

fn run_simulation(config: SimulationConfig) -> Result<(), RunSimulationError> {
    // Surface any non-fatal warnings the parser collected (e.g. one flag
    // silently overridden by a higher-precedence one).
    for warning in &config.warnings {
        eprintln!("{warning}");
    }

    // Auto-promote to the streaming backend when an initializer-based run
    // would exceed --max-board-memory. --load-board / --continue stay on
    // the in-memory path for v1 (they still allocate the full board on
    // load); those paths surface the existing memory-budget error if the
    // file is too big.
    if should_auto_promote_to_streaming(&config) {
        return run_streaming_simulation(config);
    }

    let initial = resolve_initial_board(&config)?;
    if matches!(
        &config.initial_board,
        InitialBoardSpec::LoadFromFile { .. } | InitialBoardSpec::ContinueFromRun { .. }
    ) {
        if let Some(cli_size) = config.board_size {
            if (initial.board.width(), initial.board.height()) != (cli_size.width, cli_size.height)
            {
                return Err(RunSimulationError::BoardSizeMismatch {
                    from_file: BoardSize::new(initial.board.width(), initial.board.height())
                        .expect("loaded board has valid dimensions"),
                    from_cli: cli_size,
                });
            }
        }
    }

    if let Some(warning) = initial.warning.as_deref() {
        eprintln!("{warning}");
    }

    let board_size = BoardSize::new(initial.board.width(), initial.board.height())
        .expect("resolved initial board has valid dimensions");
    let max_iterations = initial.effective_max_iterations.unwrap_or_else(|| {
        // Fall back to CLI default when not derived from --additional-iterations.
        config.effective_max_iterations()
    });

    let mut board = initial.board;
    let updater = InPlaceTransitionalUpdater;
    let initial_signature = board
        .board_signature()
        .expect("in-memory board signatures are infallible");
    let initial_alive_count = initial_signature.alive_count();
    let initial_board_for_record = board.clone();
    let mut collector = RunStatisticsCollector::starting_from(initial_alive_count);
    let mut terminal_status = (initial_alive_count == 0).then_some(RunStatus::Extinct);
    let mut analyzer = PatternAnalyzer::in_memory_cycle_detection();
    let mut cycle_stats: Option<CycleStatistics> = None;
    if terminal_status.is_none() {
        let observation =
            PatternObservation::new(0, PatternBackend::InMemory, None, Some(&initial_signature));
        analyzer.observe(&observation);
    }
    let started = Instant::now();
    for _ in 0..max_iterations {
        if terminal_status.is_some() {
            break;
        }
        let summary = updater
            .advance_generation_with_signature(&mut board)
            .expect("in-memory board updates are infallible");
        let outcome: AdvanceOutcome = summary.outcome;
        let status = terminal_status_for_outcome(outcome);
        // A zero-change advance only proves the previous board was already
        // fixed-point stable; don't count the confirming no-op as work done.
        if !outcome.is_stable() {
            collector.record(outcome);
        }
        if let Some(status) = status {
            terminal_status = Some(status);
        } else if let Some(signature) = summary.signature.as_ref() {
            let observation = PatternObservation::new(
                collector.iterations_run(),
                PatternBackend::InMemory,
                Some(outcome),
                Some(signature),
            );
            if let Some(pattern_match) = analyzer.observe(&observation) {
                let PatternMatchDetails::Cycle(cycle) = pattern_match.details;
                cycle_stats = Some(CycleStatistics {
                    start_generation: cycle.start_generation,
                    detected_generation: cycle.detected_generation,
                    period: cycle.period,
                });
                terminal_status = Some(RunStatus::Cyclic);
            }
        }
    }
    let wall_time_ms = started.elapsed().as_millis() as u64;
    let status = terminal_status.unwrap_or(RunStatus::MaxIterations);
    let (stats, series) = collector.finalize_with_series(status, cycle_stats);

    println!("Game of Life");
    println!("Board size: {board_size}");
    println!("Max iterations: {max_iterations}");
    println!("Max board memory: {} bytes", config.max_board_memory_bytes);
    println!("Initial board: {}", config.initial_board.record_label());
    println!(
        "Generation 0: '{}' initial board seeded ({} alive)",
        config.initial_board.record_label(),
        initial_alive_count
    );
    print_terminal_status_message(&stats);
    println!("Final board state:");
    print!("{board}");
    println!(
        "Simulation complete: {} iterations ({})",
        stats.iterations_run,
        stats.status.as_str()
    );

    if let Some(save_path) = decide_save_path(&config.save, &initial.run_id)? {
        let record = build_run_record(
            initial.run_id,
            board_size,
            max_iterations,
            config.max_board_memory_bytes,
            &config.initial_board,
            initial.random_seed,
            initial.continued_from,
            stats,
            Some(series),
            wall_time_ms,
            initial_board_for_record,
            board.clone(),
        );
        write_run_record(&save_path, &record)?;
        println!("Saved run record: {}", save_path.display());
    }

    if let Some(save_path) = config.save_board_path.as_deref() {
        let snapshot = BoardSnapshot::for_board(board);
        write_board_snapshot(save_path, &snapshot)?;
        println!("Saved final board snapshot: {}", save_path.display());
    }

    Ok(())
}

/// Decide whether to auto-promote a Run into streaming mode. v1 only
/// auto-promotes initializer-based runs; --load-board and --continue
/// keep the existing in-memory failure mode if the board is too big.
fn should_auto_promote_to_streaming(config: &SimulationConfig) -> bool {
    if !matches!(&config.initial_board, InitialBoardSpec::Initializer(_)) {
        return false;
    }
    let size = config.effective_board_size();
    let required = match InMemoryBoard::allocation_bytes(size.width, size.height) {
        Ok(bytes) => bytes,
        // Overflow-style failures mean the board is far too big for the
        // cap; promote.
        Err(_) => return true,
    };
    required > config.max_board_memory_bytes
}

/// Streaming-backed simulation path. Only supports initializer-based
/// initial boards in v1 (no --load-board / --continue / run-record save).
fn run_streaming_simulation(config: SimulationConfig) -> Result<(), RunSimulationError> {
    let board_size = config.effective_board_size();
    let required =
        InMemoryBoard::allocation_bytes(board_size.width, board_size.height).unwrap_or(usize::MAX);
    eprintln!(
        "Note: streaming mode enabled because the board needs {required} bytes but \
         --max-board-memory is {} bytes.",
        config.max_board_memory_bytes
    );

    let run_id = RunId::generate();
    let random_seed = generate_random_seed();
    let run_id_hint = format!("{run_id}");
    let chunk_rows_override = None;
    let chunk_cols_override = None;
    let mut board = StreamingBoard::new(StreamingBoardParams {
        width: board_size.width,
        height: board_size.height,
        max_board_memory_bytes: config.max_board_memory_bytes,
        working_dir: config.working_dir.as_deref(),
        scratch_name_hint: &run_id_hint,
        chunk_rows_override,
        chunk_cols_override,
    })?;
    let scratch_path = board.scratch_path().to_path_buf();
    let (target_rows, target_cols) = board.target_owned_chunk();
    eprintln!(
        "Note: streaming chunk dimensions: {target_rows}x{target_cols} owned cells \
         ({}); scratch file: {}",
        if board.is_row_band_fast_path() {
            "row-band fast path"
        } else {
            "general 2D path"
        },
        scratch_path.display()
    );

    // Seed via initializer (the only InitialBoardSpec variant supported in
    // streaming mode for v1).
    match &config.initial_board {
        InitialBoardSpec::Initializer(source) => {
            seed_streaming_with_initializer(*source, &mut board, random_seed)?;
            board.flush().map_err(|e| RunSimulationError::StreamingIo {
                operation: "flushing initial board to scratch",
                source: e,
            })?;
        }
        _ => unreachable!("should_auto_promote_to_streaming gates this branch"),
    }

    let initial_alive_count = count_alive_streaming(&mut board)?;
    let max_iterations = config.effective_max_iterations();
    let updater = InPlaceTransitionalUpdater;
    let mut collector = RunStatisticsCollector::starting_from(initial_alive_count);
    let mut terminal_status = (initial_alive_count == 0).then_some(RunStatus::Extinct);
    let started = Instant::now();
    for _ in 0..max_iterations {
        if terminal_status.is_some() {
            break;
        }
        let outcome: AdvanceOutcome =
            board
                .advance_with_rule(&updater)
                .map_err(|e| RunSimulationError::StreamingIo {
                    operation: "advancing streaming board generation",
                    source: e,
                })?;
        let status = terminal_status_for_outcome(outcome);
        // A zero-change advance only proves the previous board was already
        // fixed-point stable; don't count the confirming no-op as work done.
        if !outcome.is_stable() {
            collector.record(outcome);
        }
        if let Some(status) = status {
            terminal_status = Some(status);
        }
    }
    let wall_time_ms = started.elapsed().as_millis() as u64;
    let status = terminal_status.unwrap_or(RunStatus::MaxIterations);
    let stats = collector.finalize(status);

    println!("Game of Life (streaming mode)");
    println!("Board size: {board_size}");
    println!("Max iterations: {max_iterations}");
    println!("Max board memory: {} bytes", config.max_board_memory_bytes);
    println!(
        "Streaming chunk: {target_rows}x{target_cols} owned cells ({})",
        if board.is_row_band_fast_path() {
            "row-band fast path"
        } else {
            "general 2D path"
        }
    );
    println!("Initial board: {}", config.initial_board.record_label());
    println!(
        "Generation 0: '{}' initial board seeded ({} alive)",
        config.initial_board.record_label(),
        initial_alive_count
    );
    print_terminal_status_message(&stats);
    println!(
        "Simulation complete: {} iterations ({})",
        stats.iterations_run,
        stats.status.as_str()
    );

    // Run-record save is not yet supported for streaming-sized boards;
    // warn instead of failing so the run still produces visible output.
    if matches!(
        &config.save,
        SaveSettings::ExplicitFile(_) | SaveSettings::AutoIntoDir(_)
    ) {
        eprintln!(
            "Note: run-record save is not yet supported for streaming-sized boards. \
             Use --save-board <PATH> to save the final board as a standalone snapshot, \
             or raise --max-board-memory enough to keep the board in memory. \
             Wall time: {wall_time_ms} ms."
        );
    }

    if let Some(save_path) = config.save_board_path.as_deref() {
        write_streaming_board_snapshot(save_path, &mut board)?;
        println!("Saved final board snapshot: {}", save_path.display());
    }

    // Lifecycle: explicit flush + delete on success. On error, the
    // scratch file stays on disk (see plan: failures preserve scratch
    // files for debugging).
    board.flush().map_err(|e| RunSimulationError::StreamingIo {
        operation: "flushing scratch file before cleanup",
        source: e,
    })?;
    drop(board);
    if let Err(e) = fs::remove_file(&scratch_path) {
        eprintln!(
            "Warning: failed to delete scratch file '{}': {e}",
            scratch_path.display()
        );
    }

    Ok(())
}

fn seed_streaming_with_initializer(
    source: InitialBoardSource,
    board: &mut StreamingBoard,
    seed: u64,
) -> Result<(), RunSimulationError> {
    let result = match source {
        InitialBoardSource::Demo => DemoBoardInitializer.initialize(board),
        InitialBoardSource::Alive => FullyAliveInitializer.initialize(board),
        InitialBoardSource::Blinker => BlinkerBoardInitializer.initialize(board),
        InitialBoardSource::Random => RandomBoardInitializer::new(seed).initialize(board),
    };
    result.map_err(|e| RunSimulationError::StreamingIo {
        operation: "seeding initial board into scratch",
        source: e,
    })
}

fn count_alive_streaming(board: &mut StreamingBoard) -> Result<u64, RunSimulationError> {
    let mut alive = 0u64;
    let width = board.width();
    let height = board.height();
    for y in 0..height {
        for x in 0..width {
            let state = board.peek_cell(CellCoordinate::new(x, y)).map_err(|e| {
                RunSimulationError::StreamingIo {
                    operation: "counting alive cells",
                    source: e,
                }
            })?;
            if matches!(state, CellState::Alive) {
                alive += 1;
            }
        }
    }
    Ok(alive)
}

struct ResolvedInitial {
    board: InMemoryBoard,
    run_id: RunId,
    random_seed: u64,
    continued_from: Option<RunId>,
    /// Set when the initial source determines its own iteration count (e.g.
    /// `--continue --additional-iterations N`).
    effective_max_iterations: Option<usize>,
    /// Warning to emit on stderr before the run starts (e.g. `--load-board`
    /// won over `--initial-board`).
    warning: Option<String>,
}

fn resolve_initial_board(config: &SimulationConfig) -> Result<ResolvedInitial, RunSimulationError> {
    let run_id = RunId::generate();
    let random_seed = generate_random_seed();
    match &config.initial_board {
        InitialBoardSpec::Initializer(source) => {
            let size = config.effective_board_size();
            let mut board =
                InMemoryBoard::try_new(size.width, size.height, config.max_board_memory_bytes)?;
            seed_with_initializer(*source, &mut board, random_seed);
            Ok(ResolvedInitial {
                board,
                run_id,
                random_seed,
                continued_from: None,
                effective_max_iterations: None,
                warning: None,
            })
        }
        InitialBoardSpec::LoadFromFile { path, from } => {
            let snapshot_or_run = load_initial_from_path(
                path,
                from.unwrap_or(LoadFrom::Initial),
                config.max_board_memory_bytes,
                config.max_input_file_bytes,
                config.integrity,
            )?;
            let _ = snapshot_or_run.source_kind; // reserved for future per-kind logic
            Ok(ResolvedInitial {
                board: snapshot_or_run.board,
                run_id,
                random_seed,
                continued_from: None,
                effective_max_iterations: None,
                warning: None,
            })
        }
        InitialBoardSpec::ContinueFromRun { path, budget } => {
            let loaded = read_run_record_with_warnings(
                path,
                config.max_board_memory_bytes,
                config.max_input_file_bytes,
                config.integrity.to_content_hash_mode(),
            )?;
            let warning = if loaded.warnings.is_empty() {
                None
            } else {
                Some(loaded.warnings.join("\n"))
            };
            let source_iterations_run = loaded.record.result.iterations_run;
            let additional = match *budget {
                game_of_life::ContinuationBudget::Additional(n) => n,
                game_of_life::ContinuationBudget::CumulativeMax(m) => {
                    let m_u64 = m as u64;
                    if m_u64 <= source_iterations_run {
                        return Err(RunSimulationError::CumulativeMaxTooSmall {
                            cumulative_max: m,
                            source_iterations_run,
                        });
                    }
                    (m_u64 - source_iterations_run) as usize
                }
            };
            Ok(ResolvedInitial {
                board: loaded.record.final_board,
                run_id,
                random_seed,
                continued_from: Some(loaded.record.run_id),
                effective_max_iterations: Some(additional),
                warning,
            })
        }
    }
}

struct LoadedBoardFromPath {
    board: InMemoryBoard,
    source_kind: FileKind,
}

fn load_initial_from_path(
    path: &Path,
    which: LoadFrom,
    max_board_memory_bytes: usize,
    max_input_file_bytes: usize,
    integrity: IntegrityMode,
) -> Result<LoadedBoardFromPath, RunSimulationError> {
    // Sniff first to decide whether it's a snapshot or a run record.
    let kind = game_of_life::persistence::sniff_file_kind(path)
        .map_err(|e| RunSimulationError::SnapshotRead(BoardSnapshotReadError::Magic(e)))?;
    match kind {
        FileKind::BoardSnapshot => {
            let snap: BoardSnapshot =
                read_board_snapshot(path, max_board_memory_bytes, max_input_file_bytes)?;
            Ok(LoadedBoardFromPath {
                board: snap.board,
                source_kind: FileKind::BoardSnapshot,
            })
        }
        FileKind::RunRecord => {
            let loaded = read_run_record_with_warnings(
                path,
                max_board_memory_bytes,
                max_input_file_bytes,
                integrity.to_content_hash_mode(),
            )?;
            for warning in &loaded.warnings {
                eprintln!("{warning}");
            }
            let board = match which {
                LoadFrom::Initial => loaded.record.initial_board,
                LoadFrom::Final => loaded.record.final_board,
            };
            Ok(LoadedBoardFromPath {
                board,
                source_kind: FileKind::RunRecord,
            })
        }
    }
}

fn seed_with_initializer(source: InitialBoardSource, board: &mut InMemoryBoard, seed: u64) {
    match source {
        InitialBoardSource::Demo => DemoBoardInitializer
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
        InitialBoardSource::Alive => FullyAliveInitializer
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
        InitialBoardSource::Blinker => BlinkerBoardInitializer
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
        InitialBoardSource::Random => RandomBoardInitializer::new(seed)
            .initialize(board)
            .expect("in-memory board initialization is infallible"),
    }
}

fn generate_random_seed() -> u64 {
    let mut hasher = RandomState::new().build_hasher();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    hasher.write_u128(now);
    hasher.write_u32(process::id());
    hasher.finish()
}

fn print_terminal_status_message(stats: &RunStatistics) {
    match stats.status {
        RunStatus::Stable => {
            println!(
                "Stable state reached at generation {}",
                stats.iterations_run
            );
        }
        RunStatus::Cyclic => {
            if let Some(cycle) = stats.cycle {
                println!(
                    "Cycle detected at generation {}: period {} (first seen at generation {})",
                    cycle.detected_generation, cycle.period, cycle.start_generation
                );
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn build_run_record(
    run_id: RunId,
    board_size: BoardSize,
    max_iterations: usize,
    max_board_memory_bytes: usize,
    initial_board_spec: &InitialBoardSpec,
    random_seed: u64,
    continued_from: Option<RunId>,
    stats: game_of_life::stats::RunStatistics,
    series: Option<IterationSeries>,
    wall_time_ms: u64,
    initial_board: InMemoryBoard,
    final_board: InMemoryBoard,
) -> RunRecord {
    let initial_board_hash = board_grid_hash(&initial_board);
    let final_board_hash = board_grid_hash(&final_board);
    RunRecord {
        run_id,
        schema_version: RUN_RECORD_SCHEMA_VERSION,
        created_at: SystemTime::now(),
        tool_version: TOOL_VERSION.to_string(),
        config: RunRecordConfig {
            board_size,
            max_iterations,
            max_board_memory_bytes,
            initial_board_source: initial_board_spec.record_label(),
            random_seed,
            updater: "in_place_transitional".to_string(),
            continued_from,
        },
        result: RunRecordResult {
            status: stats.status.as_str().to_string(),
            iterations_run: stats.iterations_run,
            wall_time_ms,
            initial_alive_count: stats.initial_alive_count,
            final_alive_count: stats.final_alive_count,
            peak_alive_count: stats.peak_alive_count,
            peak_alive_generation: stats.peak_alive_generation,
            min_alive_count: stats.min_alive_count,
            min_alive_generation: stats.min_alive_generation,
            total_births: stats.total_births,
            total_deaths: stats.total_deaths,
            cycle_start_generation: stats.cycle.map(|cycle| cycle.start_generation),
            cycle_detected_generation: stats.cycle.map(|cycle| cycle.detected_generation),
            cycle_period: stats.cycle.map(|cycle| cycle.period),
            initial_board_hash,
            final_board_hash,
        },
        series,
        initial_board,
        final_board,
    }
}

fn decide_save_path(
    save: &SaveSettings,
    run_id: &RunId,
) -> Result<Option<PathBuf>, RunSimulationError> {
    match save {
        SaveSettings::Suppressed => Ok(None),
        SaveSettings::ExplicitFile(path) => Ok(Some(path.clone())),
        SaveSettings::AutoIntoDir(dir) => {
            if !dir.exists() {
                fs::create_dir_all(dir).map_err(|e| RunSimulationError::Io {
                    path: dir.clone(),
                    operation: "creating runs directory",
                    source: e,
                })?;
            }
            let stamp = format_filename_timestamp(SystemTime::now());
            let filename = format!("{stamp}-{}.gol", run_id.short());
            Ok(Some(dir.join(filename)))
        }
    }
}

fn format_filename_timestamp(when: SystemTime) -> String {
    let seconds_since_epoch = when
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // Reuse the timestamp module's civil-from-days math via format_utc + reshape.
    let iso = game_of_life::persistence::format_utc(when);
    // iso: 2026-06-12T22:55:20Z -> 20260612T225520Z
    let _ = seconds_since_epoch;
    iso.replace(['-', ':'], "")
}

// -------- replay --------------------------------------------------------

#[derive(Debug)]
enum ReplayOutcome {
    Match,
    Mismatch(Vec<String>),
}

#[derive(Debug)]
enum ReplayError {
    Read(RunRecordReadError),
    BoardCreation(InMemoryBoardCreationError),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::Read(e) => write!(f, "{e}"),
            ReplayError::BoardCreation(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ReplayError {}

impl From<RunRecordReadError> for ReplayError {
    fn from(value: RunRecordReadError) -> Self {
        ReplayError::Read(value)
    }
}

impl From<InMemoryBoardCreationError> for ReplayError {
    fn from(value: InMemoryBoardCreationError) -> Self {
        ReplayError::BoardCreation(value)
    }
}

fn replay(config: &ReplayConfig) -> Result<ReplayOutcome, ReplayError> {
    let loaded = read_run_record_with_warnings(
        &config.source,
        config.max_board_memory_bytes,
        config.max_input_file_bytes,
        config.integrity.to_content_hash_mode(),
    )?;
    for warning in &loaded.warnings {
        eprintln!("{warning}");
    }
    let record = loaded.record;

    // Recreate the initial board using the same source label + seed.
    let initial_label = record.config.initial_board_source.as_str();
    let mut board = if initial_label.starts_with("load:") || initial_label.starts_with("continue:")
    {
        // The initial board came from a file; we already have it captured in
        // the run record. Use it as-is.
        record.initial_board.clone()
    } else {
        // Stateless initializer: regenerate from the seed so we verify that
        // the initializer is still deterministic.
        let source =
            InitialBoardSource::parse(initial_label.trim()).unwrap_or(InitialBoardSource::Demo);
        let (w, h) = (
            record.config.board_size.width,
            record.config.board_size.height,
        );
        let mut b = InMemoryBoard::try_new(w, h, record.config.max_board_memory_bytes)?;
        seed_with_initializer(source, &mut b, record.config.random_seed);
        b
    };

    // Verify the initial board matches what was recorded; if it doesn't,
    // that's a divergence (initializer drift between versions).
    let mut diffs = Vec::new();
    if board != record.initial_board {
        diffs.push(
            "initial board differs: recorded initial board does not match the regenerated one (initializer drift or seed mismatch)"
                .to_string(),
        );
        // Use the recorded board so the rest of the comparison is meaningful.
        board = record.initial_board.clone();
    }

    // Re-execute.
    let updater = InPlaceTransitionalUpdater;
    let initial_signature = board
        .board_signature()
        .expect("in-memory board signatures are infallible");
    let mut collector = RunStatisticsCollector::starting_from(initial_signature.alive_count());
    let stop_on_stability = record.result.status == RunStatus::Stable.as_str();
    let stop_on_cycle = record.result.status == RunStatus::Cyclic.as_str();
    let mut terminal_status = (collector.final_alive_count() == 0).then_some(RunStatus::Extinct);
    let mut analyzer = PatternAnalyzer::in_memory_cycle_detection();
    let mut cycle_stats: Option<CycleStatistics> = None;
    if terminal_status.is_none() {
        let observation =
            PatternObservation::new(0, PatternBackend::InMemory, None, Some(&initial_signature));
        analyzer.observe(&observation);
    }
    for _ in 0..record.config.max_iterations {
        if terminal_status.is_some() {
            break;
        }
        let summary = updater
            .advance_generation_with_signature(&mut board)
            .expect("in-memory board updates are infallible");
        let outcome = summary.outcome;
        match terminal_status_for_outcome(outcome) {
            Some(RunStatus::Stable) if stop_on_stability => {
                // New stable records exclude the no-op confirmation from
                // iterations_run; legacy max_iterations records keep replaying.
                terminal_status = Some(RunStatus::Stable);
            }
            Some(RunStatus::Stable) => collector.record(outcome),
            Some(status) => {
                if !outcome.is_stable() {
                    collector.record(outcome);
                }
                terminal_status = Some(status);
            }
            None => {
                collector.record(outcome);
                if let Some(signature) = summary.signature.as_ref() {
                    let observation = PatternObservation::new(
                        collector.iterations_run(),
                        PatternBackend::InMemory,
                        Some(outcome),
                        Some(signature),
                    );
                    if let Some(pattern_match) = analyzer.observe(&observation) {
                        let PatternMatchDetails::Cycle(cycle) = pattern_match.details;
                        if stop_on_cycle {
                            cycle_stats = Some(CycleStatistics {
                                start_generation: cycle.start_generation,
                                detected_generation: cycle.detected_generation,
                                period: cycle.period,
                            });
                            terminal_status = Some(RunStatus::Cyclic);
                        }
                    }
                }
            }
        }
    }
    let recomputed = collector.finalize_with_cycle(
        terminal_status.unwrap_or(RunStatus::MaxIterations),
        cycle_stats,
    );

    if board != record.final_board {
        diffs.push("final board differs".to_string());
    }
    if recomputed.iterations_run != record.result.iterations_run {
        diffs.push(format!(
            "iterations_run differs: recorded={}, recomputed={}",
            record.result.iterations_run, recomputed.iterations_run
        ));
    }
    if recomputed.status.as_str() != record.result.status {
        diffs.push(format!(
            "status differs: recorded='{}', recomputed='{}'",
            record.result.status,
            recomputed.status.as_str()
        ));
    }
    if recomputed.total_births != record.result.total_births {
        diffs.push(format!(
            "total_births differs: recorded={}, recomputed={}",
            record.result.total_births, recomputed.total_births
        ));
    }
    if recomputed.total_deaths != record.result.total_deaths {
        diffs.push(format!(
            "total_deaths differs: recorded={}, recomputed={}",
            record.result.total_deaths, recomputed.total_deaths
        ));
    }
    if recomputed.final_alive_count != record.result.final_alive_count {
        diffs.push(format!(
            "final_alive_count differs: recorded={}, recomputed={}",
            record.result.final_alive_count, recomputed.final_alive_count
        ));
    }
    if recomputed.cycle.map(|cycle| cycle.start_generation) != record.result.cycle_start_generation
    {
        diffs.push(format!(
            "cycle_start_generation differs: recorded={:?}, recomputed={:?}",
            record.result.cycle_start_generation,
            recomputed.cycle.map(|cycle| cycle.start_generation)
        ));
    }
    if recomputed.cycle.map(|cycle| cycle.detected_generation)
        != record.result.cycle_detected_generation
    {
        diffs.push(format!(
            "cycle_detected_generation differs: recorded={:?}, recomputed={:?}",
            record.result.cycle_detected_generation,
            recomputed.cycle.map(|cycle| cycle.detected_generation)
        ));
    }
    if recomputed.cycle.map(|cycle| cycle.period) != record.result.cycle_period {
        diffs.push(format!(
            "cycle_period differs: recorded={:?}, recomputed={:?}",
            record.result.cycle_period,
            recomputed.cycle.map(|cycle| cycle.period)
        ));
    }
    if diffs.is_empty() {
        Ok(ReplayOutcome::Match)
    } else {
        Ok(ReplayOutcome::Mismatch(diffs))
    }
}

// -------- extract-board -------------------------------------------------

fn run_extract(config: &ExtractBoardConfig) -> Result<(), ExtractBoardCliError> {
    let which = match config.which {
        LoadFrom::Initial => ExtractWhich::Initial,
        LoadFrom::Final => ExtractWhich::Final,
    };
    extract_board_from_run(
        &config.source,
        which,
        &config.output,
        config.max_board_memory_bytes,
        config.max_input_file_bytes,
        config.integrity.to_content_hash_mode(),
    )
    .map_err(ExtractBoardCliError::Extract)
}

#[derive(Debug)]
enum ExtractBoardCliError {
    Extract(ExtractBoardError),
}

impl std::fmt::Display for ExtractBoardCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractBoardCliError::Extract(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ExtractBoardCliError {}
