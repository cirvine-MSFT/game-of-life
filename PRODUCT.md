# Product

## Users

Researchers, educators, students, and developers use this project locally to explore Conway's Game of Life, inspect saved runs, and discuss implementation tradeoffs in interview or learning contexts. The desktop UI serves users who want to edit boards, run simulations, and compare saved run records without switching back to the CLI.

## Product Purpose

Game of Life is reproducible simulation software that starts as a clear Rust implementation and grows into a small desktop workbench. Success means users can understand the rules, trust saved results, load or replay runs, and compare run behavior through compact statistics and charts.

## Brand Personality

Clear, rigorous, and calm. The interface should feel like a trustworthy local engineering tool: direct controls, legible data, restrained color, and no ornamental effects that distract from simulation state.

## Anti-references

Avoid decorative dashboards, novelty game skins, opaque black-box analytics, and marketing-style hero treatments. The UI should not look like a SaaS landing page, a noisy game HUD, or a generic chart demo disconnected from the run files.

## Design Principles

- Preserve trust by making file state, errors, and limitations visible at the point of action.
- Keep simulation data exact in state and apply visual simplification only at rendering boundaries.
- Prefer familiar desktop-tool patterns over custom controls.
- Let charts and tables serve comparison tasks instead of decoration.
- Keep future AI and telemetry surfaces clearly marked when they are not yet wired.

## Accessibility & Inclusion

Target WCAG AA contrast, keyboard-accessible controls, screen-reader labels for charts and pane regions, and reduced-motion-safe product interactions. Color must never be the only indicator for run status or chart identity.
