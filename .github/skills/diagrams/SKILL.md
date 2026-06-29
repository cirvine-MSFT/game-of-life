---
name: diagrams
description: Repo-scoped skill for creating and updating architecture diagrams (.excalidraw + .svg) in docs/. Use whenever a PR touches code paths listed in catalog.yml, when the user asks to refresh or add an architecture diagram, or when the per-PR diagram workflow assigns you to update stale diagrams. Authors diagrams using the excalidraw-workbench canvas locally and renders them to SVG via .github/scripts/render-excalidraw.mjs for CI.
---

# Game of Life — Diagrams skill

This skill owns the diagrams under `docs/`. It tells you **which diagrams exist, what each one documents, how to update them, and how to render them**. It does NOT restate the Excalidraw JSON format — for that, defer to the user-level `excalidraw` skill (`~/.copilot/skills/excalidraw/SKILL.md`).

## When to use this skill

- A PR touches one or more source paths that a diagram tracks (see `catalog.yml`)
- The user asks to update, refresh, create, or audit a diagram in `docs/`
- The `.github/workflows/update-diagrams.yml` workflow has flagged diagrams as stale
- You're adding a new architectural concept that deserves a diagram (then add a `catalog.yml` entry too)

## Catalog

The authoritative list of diagrams is `catalog.yml` next to this file. Each entry has:

| Field | Meaning |
|---|---|
| `slug` | Stem used for `docs/<slug>.excalidraw` and `docs/<slug>.svg` |
| `title` | Human-readable name shown in markdown captions |
| `description` | One-paragraph summary of what the diagram conveys |
| `tracks` | Glob list of source paths whose changes are likely to invalidate the diagram |
| `audience` | Who the diagram is for (e.g. "new contributors", "library consumers") |
| `must_show` | Required elements/concepts that the diagram must depict |

Add a new entry whenever you create a new diagram; do not let `docs/*.excalidraw` and `catalog.yml` drift.

## Stylistic guardrails for this repo

These are repo-wide conventions on top of the user-level `excalidraw` skill:

1. **Palette**: FluentUI-aligned (per the user skill's "FluentUI-Aligned Colors" table). Use the **Primary/Brand blue** for product code, **Success green** for data flow / outputs, **Accent purple** for the desktop UI surface, **Warning orange** for transitional state, **Neutral** for annotations and infrastructure.
2. **Container boxes are transparent.** Always. Inner leaf boxes carry the fill.
3. **Text is always black** (`#000000`). Use box color for grouping.
4. **Fonts**: `fontFamily: 3` (Cascadia, monospace) for symbol names like `InMemoryBoard::advance_generation`, type names, or shell commands; `fontFamily: 1` (Virgil, hand-drawn) for everything else.
5. **Title block** at the top of every diagram: large Virgil text with the diagram title and a one-line subtitle. Use the `title` and `description` from `catalog.yml`.
6. **Element IDs are stable.** When updating an existing diagram, preserve `id` values on elements that still exist. Only mint new IDs for new elements. This keeps Excalidraw diffs reviewable and lets the workbench's comments stay anchored.
7. **Prefer surgical patches over full regenerations.** When a small code change invalidates a couple of labels, edit those labels — don't reflow the whole canvas.
8. **No screenshots of code in diagrams.** Use Cascadia text labels instead so the source stays diff-friendly.

## Workflow — local authoring (preferred)

Use the `excalidraw-workbench` canvas extension. It runs offline (no SSO, no internet), edits the file in place, and produces SVG/PNG snapshots locally.

1. **Open the diagram in the workbench canvas:**

   ```text
   open_canvas with:
     canvasId: "excalidraw-workbench"
     instanceId: "diagram-<slug>"
     input: { "path": "docs/<slug>.excalidraw" }
   ```

   Reuse the same `instanceId` to focus an open panel; pick a new one to open another diagram side-by-side.

2. **Patch the scene.** Use `apply_element_patch` for small tweaks (move a box, rename a label, recolor a section) or `save_source` for larger restructures. Always preserve existing `id` values for elements that survive the edit.

3. **Capture the rendered SVG:**

   ```text
   invoke_canvas_action with:
     instanceId: "diagram-<slug>"
     actionName: "capture_snapshot"
     input: { "format": "svg" }
   ```

   The snapshot writes into the session artifact directory. Move it to `docs/<slug>.svg` with a regular file copy (the workbench intentionally doesn't write to the repo on snapshot).

4. **Commit both files together** — `docs/<slug>.excalidraw` and `docs/<slug>.svg`. The source is the truth; the SVG is the rendered companion.

If the workbench canvas is not available (e.g., you're a Copilot Coding Agent in a sandboxed CI runner), fall back to the CI renderer below.

## Workflow — CI / scripted rendering (fallback)

`.github/scripts/render-excalidraw.mjs` is a small Node script using `@excalidraw/excalidraw` headlessly. It produces deterministic SVGs and is what the per-PR workflow runs.

```bash
cd .github/scripts
npm install   # one-time
node render-excalidraw.mjs ../../docs/architecture.excalidraw ../../docs/architecture.svg
```

Or render every diagram in the catalog:

```bash
node render-all.mjs    # reads catalog.yml, renders docs/<slug>.svg for each entry
```

Use this path when:
- You don't have the workbench canvas available
- You need byte-deterministic output for CI
- You're regenerating SVGs for many diagrams at once

## Workflow — handling "diagrams stale" notifications

When the `update-diagrams` workflow assigns you (Copilot Coding Agent) to a PR with stale diagrams:

1. Read the PR description + diff (`gh pr diff <number>`) to understand what changed.
2. Read `catalog.yml` and `SKILL.md` (this file).
3. For each stale diagram slug in the workflow comment:
   - Open `docs/<slug>.excalidraw` and read the current scene.
   - Decide: does this code change actually require a diagram update, or is the catalog over-tracking?
   - If yes, patch the scene surgically. Preserve IDs. Honor the style guardrails above.
   - Re-render: `node .github/scripts/render-excalidraw.mjs docs/<slug>.excalidraw docs/<slug>.svg`.
   - If no, leave the diagram alone and note in your PR comment why ("the change in `src/board/coordinate.rs` was a doc-comment cleanup; no diagram update needed").
4. Commit all changes in a single commit titled `docs: refresh diagrams for <short summary of code change>` with the standard `Co-authored-by: Copilot` trailer.
5. Reply on the PR with a short summary table: `| slug | action | rationale |`.

## Adding a new diagram

1. Add a new entry to `catalog.yml` with `slug`, `title`, `description`, `tracks`, `audience`, `must_show`.
2. Author the diagram via the workbench canvas (see local authoring workflow). Bootstrap with a title block, the required elements from `must_show`, and the standard color palette.
3. Render the SVG.
4. Reference the SVG in the appropriate markdown file (usually `docs/design.md` or the relevant module-level doc) with: `![<title>](./<slug>.svg)`.
5. Commit all four artifacts together: `catalog.yml`, `docs/<slug>.excalidraw`, `docs/<slug>.svg`, and the markdown reference.

## Failure modes — don't do these

- ❌ Do **not** invent a `catalog.yml` entry's `tracks` glob to include unrelated paths "just in case". Tracks should be tight enough that PRs touching them really do invalidate the diagram.
- ❌ Do **not** commit a `.svg` without re-rendering it from the current `.excalidraw` source. Drifted SVGs are worse than no SVG.
- ❌ Do **not** regenerate a diagram from scratch when a small patch would do — element IDs and layout stability matter for review.
- ❌ Do **not** use `aka.ms/excalidraw` for this repo's diagrams. Use the workbench canvas locally or the OSS renderer in CI. (The user-level `excalidraw` skill's browser flow is fine for ad-hoc one-off diagrams elsewhere, but this repo's diagrams must be reproducible from source without external services.)
