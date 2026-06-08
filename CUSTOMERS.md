# Customers

This project starts as an interview-sized Conway's Game of Life implementation, but its product direction is broader: reproducible simulation software that can help people experiment, teach, and explain design choices.

Agents and contributors should use this file as a decision lens before proposing major features. A good feature should clearly serve one or more customer jobs below, or this file should be updated to explain the newly discovered customer need.

## Primary customer: research scientist / computational experimenter

The primary customer wants controlled, repeatable simulation runs. They are interested in how simple local rules produce global behavior, and they may run many experiments across board sizes, initial conditions, rules, boundary conditions, neighborhoods, dimensions, update modes, algorithms, seeds, and iteration limits.

They value:

- Reproducible run configuration and output.
- Batch execution and parameter sweeps.
- Pattern discovery and classification.
- Aggregate statistics across many runs.
- Clear provenance for interesting findings.
- Deterministic, testable simulation behavior.
- Performance that is motivated by real experiment scale.

For this customer, telemetry is useful when it helps explain or improve large experiments, but their core need is usually analytical: which runs produced which patterns, when cycles emerged, how long transients lasted, and whether another person can reproduce the result.

## Secondary customer: educator / demonstrator

An educator wants to show how cellular automata create surprising behavior from simple rules. They need examples, visual explanation, and approachable controls more than raw throughput.

They value:

- Named patterns such as still lifes, oscillators, gliders, and spaceships.
- Step-through, replay, or timelapse views.
- Visualization that is easier to understand than console board dumps.
- Telemetry that explains operations, timing, memory-relevant dimensions, and algorithm tradeoffs.
- Examples that connect implementation choices to observable behavior.

For this customer, telemetry can be a teaching tool: changing board size, algorithm, topology, or data representation should help students see why implementation choices matter.

## Secondary customer: interviewer / evaluator

An interviewer is also a customer of this repository. They want to understand the author's thought process, design tradeoffs, and ability to evolve a simple problem into a maintainable system.

They value:

- Clear explanation of why the project starts small.
- Explicit constraints and tradeoffs.
- Tests and deterministic smoke checks.
- Documentation that explains customer-driven prioritization.
- Architecture that can grow without hiding the original simple model.

This repository should preserve artifacts that make the design process legible without turning every document into a long narrative.

## Customer jobs-to-be-done

- Define a board and initial condition.
- Select or record simulation variables: rules, boundary behavior, topology, neighborhood, dimension, update mode, RNG seed, algorithm choice, iteration limit, and software version.
- Run one simulation interactively.
- Run many simulations in batch from configurable inputs.
- Capture outcomes, metrics, and run metadata.
- Detect still lifes, oscillators, periods, spaceships, extinction, pattern occurrence, and long transients.
- Compare outcomes across parameter changes.
- Replay or visualize meaningful states without relying only on console output.
- Share enough information for another person or agent to reproduce an interesting finding.
- Explain the implementation, design choices, and tradeoffs to another person.

## Product principles

- **Reproducible by default**: interesting findings should always tie back to recorded inputs and software context.
- **Controlled variability**: the system should eventually allow deliberate variation of rules, initial states, boundary conditions, topologies, neighborhoods, dimensions, update modes, and algorithms.
- **Deterministic core**: the core simulation should remain testable and deterministic for a given configuration.
- **Console is not the whole product**: console UX is useful for automation, smoke tests, and batch runs, but rich human understanding needs better visualization and replay.
- **Machine-readable outputs matter**: summaries, metrics, and run records should be easy to analyze later.
- **Education and research overlap**: visualization, replay, and metrics can serve both teaching and experiment analysis.
- **Avoid novelty-only work**: advanced variants should be motivated by customer questions, not added only because they are interesting.

## Useful Game of Life vocabulary

- **Still life**: a stable pattern that does not change.
- **Oscillator**: a pattern that repeats after a fixed number of generations.
- **Period**: the number of generations before a repeated pattern returns.
- **Spaceship**: a repeating pattern that moves across the board.
- **Transient**: the generations before a run reaches extinction, stability, a cycle, escape, or another classified outcome.
- **Population**: the number of live cells in a generation.
- **Boundary condition**: how cells at the edge behave, such as dead outside the board, wraparound, reflective edges, or an effectively infinite universe.
- **Life-like rule / B/S notation**: a compact rule format for birth and survival. Classic Conway Life is `B3/S23`: a dead cell is born with exactly 3 live neighbors, and a live cell survives with 2 or 3 live neighbors. A variant such as HighLife (`B36/S23`) adds birth at 6 neighbors.
- **Neighborhood**: which nearby cells are counted, such as the classic 8-neighbor Moore neighborhood, a 4-neighbor von Neumann neighborhood, a hex-grid neighborhood, or higher-dimensional neighbors.
- **Update mode**: whether cells update synchronously, as in classic Life, or by a different deterministic or stochastic schedule.

## Exploratory research questions

These questions are not immediate requirements. They are prompts for future customer-driven experiments.

- How do outcomes differ between bounded, infinite, toroidal, reflective, or more exotic topologies?
- Which patterns appear only under certain boundary conditions or topologies?
- How does wraparound change extinction rates, oscillator frequency, or spaceship behavior?
- What happens on non-square lattices such as hexagonal or triangular grids?
- What changes in 3D or higher-dimensional cellular automata?
- How do different neighborhood definitions affect stability, chaos, growth, and pattern discovery?
- Which Life-like rules create replicators or other surprising structures?
- How do synchronous and asynchronous update modes change long-term behavior?
- Can the system search for initial states that produce a target pattern, long transient, oscillator, or rare event?
- What data structures and algorithms make very large or sparse universes practical?

## Roadmap lens

When planning features, prefer work that moves the project toward:

- Configurable board size.
- Pattern import/export, eventually including community formats such as RLE.
- Random initial conditions with explicit RNG seeds.
- Structured run configuration that records all inputs needed for reproduction.
- Simulation summaries: live-cell counts, extinction generation, final classification, transient length, detected period, and pattern occurrences.
- Batch execution for parameter sweeps.
- Stable-state and cycle detection.
- Parallel or multi-process execution for independent runs.
- Visualization and replay for recorded simulations.
- Telemetry and profiling that explain algorithm behavior.
- Documentation artifacts that explain product framing and design tradeoffs.

## How to update this file

Update this file when the project discovers a new persona, customer job, research question, or product principle. Keep changes concise and tied to customer value so agents can use this as a practical guide during future implementation work.
