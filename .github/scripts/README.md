# `.github/scripts/`

Repo-scoped helper scripts for the diagram skill and the `update-diagrams`
workflow. Not part of the Rust product — `cargo` ignores this directory.

## What's here

| Script | Purpose |
|---|---|
| `render-excalidraw.mjs` | Render a single `.excalidraw` source file to SVG. |
| `render-all.mjs` | Render every diagram listed in `.github/skills/diagrams/catalog.yml`. |
| `detect-stale-diagrams.mjs` | Compare a git diff against the catalog and emit a list of "stale" diagram slugs. |

## Requirements

- Node.js 20+
- `npm install` from this directory (one-time per checkout)

The first install downloads a headless Chromium for Puppeteer; subsequent runs
use the cached browser.

## Running locally

```bash
cd .github/scripts
npm install

# Render one diagram
node render-excalidraw.mjs ../../docs/architecture.excalidraw ../../docs/architecture.svg

# Render every diagram in the catalog
node render-all.mjs

# Detect stale diagrams between two refs (defaults: origin/main..HEAD)
node detect-stale-diagrams.mjs
node detect-stale-diagrams.mjs main HEAD
```

`render-*.mjs` exit non-zero on render failure so CI can surface the error.
`detect-stale-diagrams.mjs` always exits 0 — it just prints the list (one slug
per line on stdout) and optionally writes to `$GITHUB_OUTPUT`.

## Why a separate `package.json`?

This keeps Node tooling totally isolated from the Rust product. `cargo build`,
`cargo test`, and the existing CI never touch this directory. The desktop UI
(`desktop/ui/`) has its own `package.json` for the same reason.
