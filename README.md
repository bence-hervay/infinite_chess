# Infinite Chess

The project focuses on two kinds of computations:

1. **Exact checkmate detection on the infinite board** (`Rules::is_checkmate`).
   - This does **not** treat the slice boundary as an edge/wall.
   - If black has any legal king move (on the infinite board), it is **not** mate.

2. **Finite-slice trap search** inside a chosen L∞ bound.
   - Candidate positions are those where all non-captured white pieces lie within `[-bound, bound]^2`.
   - A position is considered **escaped** if black can move to a state where **every** white reply leaves the candidate set.
   - The solver computes:
     - the **maximal inescapable trap** (a greatest fixed point), and
     - the **tempo trap** refinement (Büchi-game winning set), where white can force visiting "passable" states infinitely often.

## Repository layout

```
src/
  core/      # packed squares, coordinates, fixed-size positions
  chess/     # traditional-piece rules, attack + move generation, L∞ enumeration
  search/    # trap solver + Büchi solver + mate enumeration
  scenarios/ # compile-time configs
  bin/       # small CLIs

tests/       # higher-order property tests
```

## Running

> Note: you need a Rust toolchain (`cargo`, `rustc`).

Run tests:

```bash
cargo test
```

Run the small built-in demos:

```bash
cargo run --release --bin mate_search -- three_rooks_bound2_mb1
cargo run --release --bin trap_search -- three_rooks_bound2_mb1
```

## What the tests prove

The tests include several **known-results** checks that exercise large parts of the code:

- **3 rooks vs K** has **48** checkmate placements within L∞ bound 2.
- **2 rooks vs K** has **0** checkmate placements even within L∞ bound 7.
- For the slice configuration *(3 rooks, bound=2, move_bound=1, white_can_pass=true, remove_stalemates=true)*:
  - maximal inescapable trap size is **169**,
  - tempo trap size is **113**, and
  - the tempo trap contains **no immediate checkmates** (it is a Büchi infinite-play objective).
