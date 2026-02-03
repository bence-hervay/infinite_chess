# infinite_chess

Rust framework for finding **traps** (safety) and **mates** (reachability) on a **finite slice** of an otherwise infinite chessboard.

## What this project does

- Models an **infinite board** via a finite **region** (your "candidate slice").
- Any move that exits the region transitions to a **sink** (escape).
- States are tuples of piece locations; capturable white pieces are stored as `captured`.
- Builds a turn-based game graph:
  - **Black**: lone king
  - **White**: configurable multiset of pieces (optionally including a king)
  - Optional **white pass** (tempo gain)

Then it provides solvers:
- **Safety / trap**: largest subset closed under
  - for White: there exists a reply that stays inside
  - for Black: all moves stay inside
- **Buchi** (tempo traps): within the safety trap, compute positions where White can force visiting "passable" positions infinitely often.
- **Reachability** (mate): positions where White can force reaching a checkmate node.

## How to run

From the project root:

```bash
cargo run --bin trap
cargo run --bin tempo
cargo run --bin mate
```

Run the test suite:

```bash
cargo test
```

## Notes

- The included demos use an \(L_\infty\) square of radius 2 and two white queens, because it has a small finite state space and gives deterministic expected counts for regression tests.
- The library is written to be modular: region generation, packing, movegen, graph building, and solvers are separated.

