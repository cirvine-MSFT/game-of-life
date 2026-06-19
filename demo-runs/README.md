# demo-runs

Hand-named run records used to illustrate the new `status: stable` terminal
outcome alongside the existing `max_iterations` outcome. Filenames describe
the scenario; the standard tool-generated convention is
`<timestamp>-<short-run-id>.gol` (e.g. `20260618T195957Z-3ba4d93b.gol`), but
these were saved with `--save-run <PATH>` to keep the demo readable.

| File | Scenario | Recorded status |
|---|---|---|
| `block-2x2-stable.gol` | 2x2 fully alive (still life) | `stable` |
| `blinker-5x5-max-iterations.gol` | 5x5 blinker, 6 iterations (oscillator, not classified as stable) | `max_iterations` |

Replay any of them with:

```pwsh
.\target\release\game-of-life.exe --replay .\demo-runs\block-2x2-stable.gol
```

This folder is a local demo only — it is not consumed by tests or CI.
