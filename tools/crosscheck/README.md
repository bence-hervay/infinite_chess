# Rust ↔ Python cross-check (bounded AbsBox)

This folder contains a tiny Python wrapper that imports move/attack generation from
`mjtb49/InfiniteChessEndgameScripts` (without modifying that repo) and computes the same bounded
metrics as the Rust `bounded_eval` binary, with a couple of small adapters (bound conventions,
king-blocking).

The intended workflow is:

1. Run Rust counts for a tiny scenario.
2. Run Python counts for the *same* scenario.
3. Compare the JSON outputs and chase down mismatches (movegen semantics, bounds, pass rules, …).

## Prereqs

- Python 3.10+.
- A local clone of `mjtb49/InfiniteChessEndgameScripts`.
  - Clone anywhere stable (outside this repo is fine).
  - Do **not** edit that repo; this wrapper only imports `infinite_tablebase.py`.

## Run Rust counts

Example:

```bash
cargo run --bin bounded_eval -- tests/golden/scenarios/empty_b1.json
```

## Run Python counts

Point the wrapper at your cloned python repo:

```bash
python3 tools/crosscheck/run_python_counts.py \
  --py-repo /path/to/InfiniteChessEndgameScripts \
  --pretty \
  tests/golden/scenarios/empty_b1.json
```

You can also set `ICE_PY_REPO=/path/to/InfiniteChessEndgameScripts` and omit `--py-repo`.

## Notes / limitations

- This wrapper is designed for **tiny** bounds and piece counts. Enumeration is exponential.
- `move_bound_mode`:
  - `"inclusive"` matches the Rust interpretation.
  - The Python repo’s rider move generator behaves like an **exclusive** bound; the wrapper maps
    `"inclusive"` → `move_bound+1` when calling into Python.
- **Black king square as a rider blocker**:
  - Rust treats `(0,0)` (the black king square in king-relative coordinates) as a hard blocker for
    `Q/R/B` sliding moves (orthodox chess).
  - Upstream `infinite_tablebase.py` does not explicitly model the king as an occupied square for
    rider movement, which can allow riders to “slide through” `(0,0)` when `move_bound>1`.
  - This wrapper filters out such cross-king rider moves so that its outputs match Rust’s model.
- Cross-check scenarios should typically set `"white_king": false` (the upstream scripts are not
  built around a full “K+pieces vs k” ruleset).
