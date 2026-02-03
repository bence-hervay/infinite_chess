# Infinite Chess

The project models “white pieces vs a lone black king” on an infinite board and supports multiple objectives under a configurable scenario layer.

## Concepts: Rules vs Scenario

This codebase intentionally separates concerns:

- **Rules** (`src/chess/`): pure chess movement + legality on the infinite board (attacks, captures, king safety, slider `move_bound`).
- **Scenario** (`src/scenario/`): adds *scenario-specific* constraints and search configuration via:
  - **Laws**: filters on what moves/states are allowed (scenario legality restrictions).
  - **Domain**: membership predicate for what counts as “inside the modeled set” for trap objectives (leaving is allowed; it just counts as escape if the opponent can force it).
  - **Preferences**: non-adversarial move ordering used only for demos/strategy extraction (never affects correctness of trap sets).
  - **Resource limits**: explicit budgets so large searches fail with a structured error instead of OOM.

## What it computes

1. **Exact checkmate detection on the infinite board** (`Rules::is_checkmate`).
   - This does **not** treat the slice boundary as an edge/wall.
   - If black has any legal king move (on the infinite board), it is **not** mate.

2. **Trap / tempo-trap search** under `Scenario` constraints.
   - `maximal_inescapable_trap`: greatest fixed point of black-to-move states inside the scenario’s **domain** where White can always reply to stay in the current set.
   - `maximal_tempo_trap`: Büchi refinement where White can force infinitely many visits to “passable” white-to-move states (controlled by `white_can_pass` + `laws.allow_pass`).

## Repository layout

```
src/
  core/      # packed squares, coordinates, fixed-size positions
  chess/     # traditional-piece rules, attack + move generation, L∞ enumeration
  scenario/  # State/Scenario + Laws/Domain/Preferences + limits/errors
  search/    # trap solver + Büchi solver + movegen + resource tracking + mate enumeration
  scenarios/ # built-in scenarios (Rust code + optional data-backed scenarios)
  bin/       # small CLIs

tests/       # higher-order property tests
```

## State model

- `core::Position` stores only **king-relative** squares (black king is always at origin).
- `scenario::State` adds `abs_king: Coord` as an optional absolute anchor.
  - If `Scenario.track_abs_king == false`, the absolute coordinate is ignored and must stay at `ORIGIN` (translation-reduced state space).
  - If `Scenario.track_abs_king == true`, black king moves update `abs_king` by the moved `delta`, enabling “absolute” laws/domains (e.g. clamp to a half-plane).

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

Built-in scenarios are listed by:

```bash
cargo run --release --bin trap_search
```

## Extending scenarios (how-to)

You usually extend the project by adding a new scenario function in `src/scenarios/` and wiring it into `src/scenarios/mod.rs:by_name`.

### 1) Pick your pieces and movement cap (Rules)

Rules are created from a `PieceLayout` + `move_bound`:

```rust
use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;

let layout = PieceLayout::from_counts(false, 0, 0, 2, 1); // B,B,N
let rules = Rules::new(layout, 23);
```

### 2) Choose candidate generation

`scenario::CandidateGeneration` controls which black-to-move states are considered “candidates” for trap search:

- `InLinfBound { bound, allow_captures }`: enumerate all canonical placements within an L∞ bound (cheap and great for small piece counts).
- `FromStates { states }`: provide a precomputed list (e.g. from a file or a geometric generator).
- `ReachableFromStart { max_queue }`: BFS explore reachable states from the required `start` (often far smaller than full enumeration).

### 3) Define Domain (membership, not legality)

Implement `scenario::DomainLike`:

```rust
use infinite_chess::scenario::{DomainLike, State};

#[derive(Clone, Copy)]
struct MyDomain;
impl DomainLike for MyDomain {
    fn inside(&self, s: &State) -> bool {
        // “inside” predicate (leaving is allowed, it just affects objectives)
        !s.pos.squares().is_empty()
    }
}
```

### 4) Define Laws (scenario-specific move legality)

Implement `scenario::LawsLike` and override only what you need:

```rust
use infinite_chess::core::coord::Coord;
use infinite_chess::scenario::{LawsLike, State};

#[derive(Clone, Copy)]
struct NoCaptures;
impl LawsLike for NoCaptures {
    fn allow_black_move(&self, from: &State, _to: &State, delta: Coord) -> bool {
        // Forbid king-step captures: disallow if a white piece sits on the destination delta.
        let dst = infinite_chess::core::square::Square::from_coord(delta);
        !from.pos.squares().iter().any(|&sq| !sq.is_none() && sq == dst)
    }
}
```

Pass control for tempo objectives lives in laws too:
- `Scenario.white_can_pass` enables passing globally
- `laws.allow_pass(state)` can further restrict when passing is allowed

### 5) Define Preferences (tie-breakers only)

Implement `scenario::PreferencesLike` to rank moves for demos/strategy extraction. This never changes the computed trap sets.
The helper `search::strategy::extract_white_stay_strategy` can pick a single “stay in trap” reply for each white node using preferences.

### 6) Set limits + caching

Scenario searches use explicit budgets:

- `ResourceLimits` bounds state/edge growth and runtime steps.
- `CacheMode::{None, BlackOnly, BothBounded}` selects move-caching policy.

All solvers return `Result<_, SearchError>`. On failure you get a structured error with:
- stage + metric (which budget was hit)
- observed/limit
- running counters (states/edges/cache entries/cached moves/steps)

## Data-backed scenario: NBB reference set

The built-in `nbb20_from_file` scenario loads a large reference set from:
- `tests/data/kNBB_20_3_2.5_23.txt`

It is intentionally heavy; the test is ignored by default:

```bash
cargo test --release -- --ignored nbb20_from_file_has_nonempty_trap_sets
```

## What the tests prove

The tests include several **known-results** checks that exercise large parts of the code:

- **3 rooks vs K** has **48** checkmate placements within L∞ bound 2.
- **2 rooks vs K** has **0** checkmate placements even within L∞ bound 7.
- For the slice configuration *(3 rooks, bound=2, move_bound=1, white_can_pass=true, remove_stalemates=true)*:
  - maximal inescapable trap size is **169**,
  - tempo trap size is **113**, and
  - the tempo trap contains **no immediate checkmates** (it is a Büchi infinite-play objective).
