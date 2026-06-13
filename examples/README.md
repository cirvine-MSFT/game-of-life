# Example Game of Life Boards

Hand-curated `.gol` files that ship with the repo. There are two flavors:

| Folder | Purpose |
|--------|---------|
| `patterns/` | Happy-path demos. Load any of these as the initial board and watch interesting Game of Life behavior. |
| `negative/` | **Intentionally malformed files.** They demonstrate the reader's error UX. Loading any of them produces an actionable error and a non-zero exit. Use them to see what the messages look like, or as templates when reasoning about your own malformed inputs. |

Both folders contain `GOL-BOARD-SNAPSHOT v1` files (the standalone board flavor — intentionally hash-free, freely editable). For full information about the file format see [`docs/design.md`](../docs/design.md#persistence-design).

## `patterns/` — load and run

```bash
./target/release/game-of-life --load-board examples/patterns/glider.gol --max-iterations 50
```

| File | Pattern | Behavior |
|------|---------|----------|
| `block-still-life.gol`       | 2×2 block on a 4×4 board       | Period-1 still life — never changes. |
| `blinker-oscillator.gol`     | Three cells in a row, 5×5      | Period-2 oscillator — alternates horizontal/vertical forever. |
| `glider.gol`                 | Glider in the corner of 10×10  | Period-4 spaceship — drifts diagonally until it hits the boundary. |
| `r-pentomino-methuselah.gol` | R-pentomino centered on 20×20  | Famous methuselah. On an unbounded board it stabilizes after 1103 generations; on this bounded 20×20 it bumps into the edges and produces a different (but still interesting) trajectory. |

### Crafting your own

1. Copy any pattern: `cp examples/patterns/glider.gol my-board.gol`.
2. Edit the grid (`.` = dead, `#` = alive). Keep the row width consistent and update the `size:`, `alive_count:`, and `dead_count:` headers to match.
3. Run it: `./target/release/game-of-life --load-board my-board.gol --max-iterations 100`.

The reader verifies `alive_count` and `dead_count` against the grid on load, so you'll get an actionable error if they drift out of sync (see the `negative/` files for examples).

To capture an interesting state from a live run and turn it into a new pattern, see the `--extract-board` verb in the main [README](../README.md).

## `negative/` — these are supposed to fail

Every file in `negative/` triggers a different error class. Each one is documented below with the command to reproduce, the abridged error you should see on stderr, and what the file deliberately gets wrong. The behavior of each is also enforced by integration tests in [`tests/persistence_cli_tests.rs`](../tests/persistence_cli_tests.rs) — these files aren't the test inputs themselves, but they exercise the same code paths.

> Each command below exits with a non-zero status. That's the point — they are demonstrations of how the tool surfaces malformed input.

| File | What's wrong | Error class |
|------|--------------|-------------|
| `not-a-gol-file.gol`         | First line isn't a recognized magic prefix. | Magic mismatch → `is not a Game of Life file: first line was ...` |
| `truncated-grid.gol`         | Header declares `size: 5x5` but the grid only contains 3 rows; the `END BOARD` fence gets parsed as a malformed row. | Ragged-row detection → `Board row at ...:13 has width 21; expected 5 based on the first row.` |
| `wrong-alive-count.gol`      | Header says `alive_count: 99` but the grid has 3 live cells. | Count header / grid mismatch → `Board alive_count mismatch at ...: header declared 99 but grid contains 3.` |
| `unknown-cell-character.gol` | Grid contains the character `X` which isn't in the alphabet. | Grid character validation → `Board grid at ...:12 contains unknown character 'X'; allowed characters are '.' (dead) and '#' (alive).` |
| `unknown-encoding.gol`       | Header sets `encoding: rle`. Only `ascii` is supported in v1. | Encoding allow-list → `Unknown board encoding 'rle' at ...; supported encodings: ascii.` |

To reproduce any of them, replace `<file>` in:

```bash
./target/release/game-of-life --load-board examples/negative/<file> --max-iterations 0 --no-save
```
