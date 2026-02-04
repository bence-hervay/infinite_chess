# Bounded AbsBox model spec (parity reference)

This project models endgames of the form **White pieces vs a lone Black king** on an *infinite*
board, but certain solvers operate on a **finite bounded universe**. This document defines the
exact semantics used by the bounded solvers (trap / tempo / forced mate) so that multiple
implementations can be compared consistently.

## 1) State representation

A game *placement* is represented as:

- **Absolute black king anchor**: `abs_king = (kx, ky)` in absolute coordinates.
- **White piece squares**: stored **relative to the black king** (king is always at the origin in
  the relative frame). Captured pieces are represented as **absent** (`NONE`).

Formally, for a present piece with relative coordinate `rel = (x, y)`, the piece’s absolute square
is:

`abs_piece = abs_king + rel`.

## 2) Move generation

### Black move (king step + re-centering)

Black chooses one of the 8 king steps `delta ∈ {(-1,-1), …, (1,1)} \ {(0,0)}`.

The resulting placement is computed as:

1. Update the absolute anchor: `abs_king := abs_king + delta`.
2. Re-center all remaining white pieces by shifting their relative squares by `-delta`:
   `rel := rel - delta`.
3. **Capture on the destination square**: if a re-centered piece lands on the origin `(0,0)` it is
   captured (becomes absent).

### White move (piece move in relative coordinates)

White chooses exactly one white piece and moves it according to orthodox rules (K/Q/R/B/N),
interpreted in the **relative** coordinate system.

- The absolute anchor `abs_king` is unchanged.
- White pieces may not overlap.
- White pieces may not move onto the origin `(0,0)` (the black king square).

Captured pieces are absent and cannot be moved.

## 3) Check legality

A black move is legal iff, after re-centering and any capture, the black king square (the origin in
relative coordinates) is **not attacked** by any remaining white piece.

## 4) Universe / bounds (AbsBox)

Fix an absolute bounding box:

`[-B, B] × [-B, B]`.

A placement is **in-universe** iff:

1. `abs_king` lies inside the box, and
2. every present white piece’s absolute square `abs_king + rel` lies inside the box.

Any move that produces an out-of-universe placement is treated as **leaving the universe**.

## 5) White “pass”

Some solvers optionally allow White to play a **pass** (no change in placement), modeling spending
tempo to “move unmodeled slow pieces”.

- Whether passing is available is controlled explicitly by scenario configuration.
- Solvers must **not** implicitly enable passing.

## 6) Objective-level interpretations

### Confinement trap (safety)

Work in the finite graph induced by in-universe placements.

A black-to-move placement is in the confinement trap iff:

- Black has **no legal escape move** (no legal black move that leaves the universe), and
- for every legal black move that stays in-universe, White has some legal reply (including pass if
  enabled) that returns to a placement still inside the current trap set.

### Tempo-gaining trap (Büchi)

Compute `Trap` first. Restrict play to placements that stay within `Trap`.

A white-to-move node is **accepting** if:

- pass is enabled and allowed, and
- passing keeps the game in `Trap` (i.e. the placement itself is in `Trap`).

The tempo set is the set of black-to-move trap nodes from which White can force visiting accepting
nodes infinitely often.

### Forced mate (reachability)

Terminal wins are **black-to-move checkmates** under bounded-universe interpretation:

- black is in check, and
- black has **no legal black move that stays in-universe**, and
- black has **no escape move**.

White-to-move passing is only available if enabled explicitly in the scenario (typically disabled
for mate searches).

## 7) Reference harness

For tiny scenarios, `src/bin/bounded_eval.rs` evaluates an AbsBox universe and prints a JSON bundle
of counts (universe size, in-universe vs escaping moves, checkmates, trap/tempo/mate sizes).

Example:

```bash
cargo run --release --bin bounded_eval -- tests/golden/scenarios/rrr_b2_mb1_pass.json
```
