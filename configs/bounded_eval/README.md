# Bounded-eval scenario suite

These JSON files are inputs for the `bounded_eval` binary. They define a finite **AbsBox**
universe `[-B,B]×[-B,B]` and ask Rust to compute:

- universe size + in-universe vs escaping move counts
- in-universe checkmates (infinite-board legality)
- maximal confinement trap + tempo trap
- bounded-universe forced-mate winning region

Run a case:

```bash
cargo run --release --bin bounded_eval -- configs/bounded_eval/rr_b2_mb4_inclusive.json
```

Optional cross-check against `InfiniteChessEndgameScripts` (imports upstream movegen, with a
small adapter to match Rust’s orthodox “king blocks riders” rule):

```bash
python3 tools/crosscheck/run_python_counts.py \
  --py-repo InfiniteChessEndgameScripts \
  --pretty \
  configs/bounded_eval/rr_b2_mb4_inclusive.json
```

## `move_bound_mode`

`move_bound_mode` controls rider step limits for `Q/R/B`:

- `"inclusive"`: rider moves allow `1..=move_bound` steps (Rust semantics).
- `"exclusive"`: rider moves allow `1..move_bound` steps (Python `infinite_tablebase.py` style).

The cross-check wrapper maps `"inclusive"` → `move_bound+1` when calling into the Python repo so
both sides interpret rider bounds consistently.
