#!/usr/bin/env python3
"""
Compute bounded-AbsBox metrics using move generation from mjtb49/InfiniteChessEndgameScripts.

This script is intentionally self-contained and imports `infinite_tablebase.py` from a
user-provided clone path (no modifications to that repo are made). The wrapper applies a small
adapter so the resulting semantics match Rust's `Rules` (notably: the black king square blocks
rider paths).

Outputs JSON compatible with the Rust `bounded_eval` binary:
  { "scenario": { ... }, "counts": { ... } }
"""

from __future__ import annotations

import argparse
import importlib
import json
import os
import sys
from collections import defaultdict, deque
from dataclasses import dataclass
from itertools import combinations
from typing import Dict, Iterable, Iterator, List, Optional, Sequence, Set, Tuple


Coord = Tuple[int, int]
Board = Tuple[Optional[Coord], ...]  # king-relative piece squares; captured = None
State = Tuple[int, int, Board]  # (abs_kx, abs_ky, board)


def chebyshev_norm(c: Coord) -> int:
    return max(abs(c[0]), abs(c[1]))


def add(a: Coord, b: Coord) -> Coord:
    return (a[0] + b[0], a[1] + b[1])


def sub(a: Coord, b: Coord) -> Coord:
    return (a[0] - b[0], a[1] - b[1])


KING_STEPS: Tuple[Coord, ...] = (
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
)

def crosses_king_square(from_sq: Coord, to_sq: Coord) -> bool:
    """
    True iff a sliding move from `from_sq` to `to_sq` would pass through the black king square
    (0,0) as an intermediate square.

    Rust's `Rules` treat the black king as a hard blocker for riders (orthodox chess). The upstream
    `InfiniteChessEndgameScripts/infinite_tablebase.py` move generator does not explicitly model
    the king as an occupied square, which allows riders to "slide through" (0,0) for move_bound>1.
    For parity with Rust we exclude those cross-king rider moves here.
    """
    fx, fy = from_sq
    tx, ty = to_sq

    # Vertical line x=0: origin is on the segment if y changes sign.
    if fx == tx == 0 and fy != 0 and ty != 0:
        return (fy < 0 < ty) or (ty < 0 < fy)

    # Horizontal line y=0: origin is on the segment if x changes sign.
    if fy == ty == 0 and fx != 0 and tx != 0:
        return (fx < 0 < tx) or (tx < 0 < fx)

    # Diagonals through origin: y=x or y=-x.
    if fx == fy and tx == ty and fx != 0 and tx != 0:
        return (fx < 0 < tx) or (tx < 0 < fx)
    if fx == -fy and tx == -ty and fx != 0 and tx != 0:
        return (fx < 0 < tx) or (tx < 0 < fx)

    return False


@dataclass(frozen=True)
class ScenarioSpec:
    bound: int
    move_bound: int
    move_bound_mode: str
    white_king: bool
    queens: int
    rooks: int
    bishops: int
    knights: int
    allow_captures: bool
    white_can_pass: bool
    remove_stalemates: bool


def load_scenario(path: str) -> ScenarioSpec:
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    if "scenario" in data:
        data = data["scenario"]

    pieces = data["pieces"]
    return ScenarioSpec(
        bound=int(data["bound"]),
        move_bound=int(data["move_bound"]),
        move_bound_mode=str(data.get("move_bound_mode", "inclusive")),
        white_king=bool(pieces["white_king"]),
        queens=int(pieces["queens"]),
        rooks=int(pieces["rooks"]),
        bishops=int(pieces["bishops"]),
        knights=int(pieces["knights"]),
        allow_captures=bool(data["allow_captures"]),
        white_can_pass=bool(data["white_can_pass"]),
        remove_stalemates=bool(data.get("remove_stalemates", True)),
    )

def scenario_json(spec: ScenarioSpec) -> Dict[str, object]:
    return {
        "bound": spec.bound,
        "move_bound": spec.move_bound,
        "move_bound_mode": spec.move_bound_mode,
        "pieces": {
            "white_king": spec.white_king,
            "queens": spec.queens,
            "rooks": spec.rooks,
            "bishops": spec.bishops,
            "knights": spec.knights,
        },
        "allow_captures": spec.allow_captures,
        "white_can_pass": spec.white_can_pass,
        "remove_stalemates": spec.remove_stalemates,
    }


def piece_kinds(spec: ScenarioSpec) -> List[str]:
    kinds: List[str] = []
    if spec.white_king:
        kinds.append("K")
    kinds.extend(["Q"] * spec.queens)
    kinds.extend(["R"] * spec.rooks)
    kinds.extend(["B"] * spec.bishops)
    kinds.extend(["N"] * spec.knights)
    return kinds


def identical_runs(kinds: Sequence[str]) -> List[range]:
    runs: List[range] = []
    i = 0
    while i < len(kinds):
        j = i + 1
        while j < len(kinds) and kinds[j] == kinds[i]:
            j += 1
        runs.append(range(i, j))
        i = j
    return runs


def white_king_index(kinds: Sequence[str]) -> Optional[int]:
    if kinds and kinds[0] == "K":
        return 0
    return None


def canonicalize(board: Board, runs: Sequence[range]) -> Board:
    b = list(board)
    for r in runs:
        coords = [b[i] for i in r]
        coords.sort(key=lambda c: (c is not None, (c[0] if c else 0), (c[1] if c else 0)))
        for i, v in zip(r, coords):
            b[i] = v
    return tuple(b)


def is_legal_board(board: Board, wk_idx: Optional[int]) -> bool:
    if (0, 0) in board:
        return False
    seen: Set[Coord] = set()
    for c in board:
        if c is None:
            continue
        if c in seen:
            return False
        seen.add(c)
    if wk_idx is not None:
        k = board[wk_idx]
        if k is not None and chebyshev_norm(k) <= 1:
            return False
    return True


def in_universe(abs_king: Coord, board: Board, bound: int) -> bool:
    if chebyshev_norm(abs_king) > bound:
        return False
    for rel in board:
        if rel is None:
            continue
        abs_piece = add(abs_king, rel)
        if chebyshev_norm(abs_piece) > bound:
            return False
    return True


def import_tablebase(py_repo: str):
    sys.path.insert(0, py_repo)
    return importlib.import_module("infinite_tablebase")


def configure_pieces(ice, kinds: Sequence[str]) -> None:
    mapping = {
        "K": ice.KING,
        "Q": ice.QUEEN,
        "R": ice.ROOK,
        "B": ice.BISHOP,
        "N": ice.KNIGHT,
    }
    ice.PIECES = [mapping[k] for k in kinds]


def enumerate_states_abs_box(
    bound: int,
    kinds: Sequence[str],
    allow_captures: bool,
) -> Iterator[State]:
    runs = identical_runs(kinds)
    wk_idx = white_king_index(kinds)

    abs_squares: List[Coord] = [(x, y) for x in range(-bound, bound + 1) for y in range(-bound, bound + 1)]

    def rec(
        run_idx: int,
        abs_king: Coord,
        used: Set[Coord],
        cur_abs: List[Optional[Coord]],
    ) -> Iterator[State]:
        if run_idx == len(runs):
            rel_board: List[Optional[Coord]] = []
            for c in cur_abs:
                if c is None:
                    rel_board.append(None)
                else:
                    rel_board.append(sub(c, abs_king))
            board = canonicalize(tuple(rel_board), runs)
            if not is_legal_board(board, wk_idx):
                return
            yield (abs_king[0], abs_king[1], board)
            return

        r = runs[run_idx]
        kind = kinds[r.start]
        run_len = len(r)

        min_k = 0 if allow_captures else run_len
        for k in range(min_k, run_len + 1):
            none_count = run_len - k

            def allowed_square(c: Coord) -> bool:
                if c in used:
                    return False
                if kind == "K":
                    # White king cannot be adjacent to the black king.
                    if chebyshev_norm(sub(c, abs_king)) <= 1:
                        return False
                return True

            available = [c for c in abs_squares if allowed_square(c)]
            for chosen in combinations(available, k):
                for c in chosen:
                    used.add(c)

                # Canonical within identical run: Nones first, then chosen in sorted order.
                chosen_sorted = sorted(chosen)
                for offset in range(run_len):
                    idx = r.start + offset
                    if offset < none_count:
                        cur_abs[idx] = None
                    else:
                        cur_abs[idx] = chosen_sorted[offset - none_count]

                yield from rec(run_idx + 1, abs_king, used, cur_abs)

                for c in chosen:
                    used.remove(c)

    for abs_king in abs_squares:
        used: Set[Coord] = {abs_king}
        cur_abs: List[Optional[Coord]] = [None] * len(kinds)
        yield from rec(0, abs_king, used, cur_abs)


def python_move_bound(spec: ScenarioSpec) -> int:
    if spec.move_bound < 1:
        raise ValueError("move_bound must be >= 1")
    mode = spec.move_bound_mode
    if mode == "inclusive":
        # InfiniteChessEndgameScripts' rider move generator uses an exclusive upper bound:
        # to match a Rust inclusive bound M, pass M+1.
        return spec.move_bound + 1
    if mode == "exclusive":
        return spec.move_bound
    raise ValueError(f"unknown move_bound_mode: {mode!r}")


def gen_black_moves_inf(
    ice,
    kinds: Sequence[str],
    runs: Sequence[range],
    wk_idx: Optional[int],
    state: State,
) -> List[State]:
    abs_kx, abs_ky, board = state
    out: List[State] = []

    for dx, dy in KING_STEPS:
        # Match Rust: the black king cannot capture the white king (if present).
        if wk_idx is not None:
            wk = board[wk_idx]
            if wk is not None and wk == (dx, dy):
                continue

        new_abs = (abs_kx + dx, abs_ky + dy)
        shifted: List[Optional[Coord]] = []
        for c in board:
            if c is None:
                shifted.append(None)
                continue
            nc = (c[0] - dx, c[1] - dy)
            shifted.append(None if nc == (0, 0) else nc)

        new_board = canonicalize(tuple(shifted), runs)
        if not is_legal_board(new_board, wk_idx):
            continue
        if ice.is_threatened(ice.KING_SQUARE, new_board):
            continue

        out.append((new_abs[0], new_abs[1], new_board))

    return out


def gen_white_moves_inf(
    ice,
    kinds: Sequence[str],
    runs: Sequence[range],
    wk_idx: Optional[int],
    py_move_bound: int,
    can_pass: bool,
    state: State,
) -> List[State]:
    abs_kx, abs_ky, board = state
    out: List[State] = []

    if can_pass:
        out.append(state)

    for i, c in enumerate(board):
        if c is None:
            continue
        for nb in ice.PIECES[i].get_resulting_board_states(board, i, py_move_bound):
            if crosses_king_square(c, nb[i]):
                continue
            nb2 = canonicalize(nb, runs)
            if not is_legal_board(nb2, wk_idx):
                continue
            out.append((abs_kx, abs_ky, nb2))

    return out


def compute_counts(ice, spec: ScenarioSpec) -> Dict[str, int]:
    kinds = piece_kinds(spec)
    configure_pieces(ice, kinds)

    runs = identical_runs(kinds)
    wk_idx = white_king_index(kinds)
    py_mb = python_move_bound(spec)

    universe: List[State] = list(enumerate_states_abs_box(spec.bound, kinds, spec.allow_captures))
    universe_set: Set[State] = set(universe)

    # Precompute move lists (with duplicates) for counts, plus deduped sets for solvers.
    attacked: Dict[State, bool] = {}
    black_all: Dict[State, List[State]] = {}
    black_in: Dict[State, List[State]] = {}
    black_escape: Dict[State, bool] = {}
    white_all: Dict[State, List[State]] = {}
    white_in: Dict[State, List[State]] = {}

    black_moves_in = 0
    black_moves_escape = 0
    white_moves_in = 0
    white_moves_escape = 0
    checkmates = 0

    for s in universe:
        _, _, board = s
        a = ice.is_threatened(ice.KING_SQUARE, board)
        attacked[s] = bool(a)

        b_moves = gen_black_moves_inf(ice, kinds, runs, wk_idx, s)
        black_all[s] = b_moves
        in_list: List[State] = []
        esc = False
        for t in b_moves:
            in_u = t in universe_set
            if in_u:
                black_moves_in += 1
                in_list.append(t)
            else:
                black_moves_escape += 1
                esc = True
        black_in[s] = sorted(set(in_list))
        black_escape[s] = esc

        if a and len(b_moves) == 0:
            checkmates += 1

        w_moves = gen_white_moves_inf(
            ice,
            kinds,
            runs,
            wk_idx,
            py_mb,
            spec.white_can_pass,
            s,
        )
        white_all[s] = w_moves
        w_in_list: List[State] = []
        for t in w_moves:
            in_u = t in universe_set
            if in_u:
                white_moves_in += 1
                w_in_list.append(t)
            else:
                white_moves_escape += 1
        white_in[s] = sorted(set(w_in_list))

    universe_states = len(universe)

    # Trap (safety): greatest fixed point on candidate black-to-move states.
    in_s: Dict[State, bool] = {s: True for s in universe}
    if spec.remove_stalemates:
        for s in universe:
            if not attacked[s] and len(black_all[s]) == 0:
                in_s[s] = False

    # Count of replies into current set for each white node.
    reply_count: Dict[State, int] = {}
    for w in universe:
        reply_count[w] = sum(1 for b in white_in[w] if in_s.get(b, False))

    # Reverse edges for incremental removals.
    white_pred: Dict[State, List[State]] = defaultdict(list)  # black b <- white w edges
    for w in universe:
        for b in white_in[w]:
            white_pred[b].append(w)

    black_pred: Dict[State, List[State]] = defaultdict(list)  # white w <- black b edges
    for b in universe:
        for w in black_in[b]:
            black_pred[w].append(b)

    q: deque[State] = deque()
    for b in universe:
        if not in_s[b]:
            continue
        if black_escape[b]:
            q.append(b)
            continue
        for w in black_in[b]:
            if reply_count[w] == 0:
                q.append(b)
                break

    while q:
        b = q.popleft()
        if not in_s.get(b, False):
            continue
        in_s[b] = False

        # Removing a black node reduces the available replies for predecessor white nodes.
        for w in white_pred.get(b, []):
            if reply_count[w] <= 0:
                continue
            reply_count[w] -= 1
            if reply_count[w] == 0:
                for pb in black_pred.get(w, []):
                    if in_s.get(pb, False):
                        q.append(pb)

    trap_set: Set[State] = {s for s, ok in in_s.items() if ok}

    # Tempo (Büchi) inside Trap.
    tempo_set: Set[State] = set()
    if spec.white_can_pass and trap_set:
        tempo_set = tempo_trap_buchi(ice, trap_set, black_in, white_in)

    # Forced mate region (reachability).
    mate_set = forced_mate_region(universe, universe_set, attacked, black_in, black_escape, white_in)

    return {
        "universe_states": universe_states,
        "black_moves_in": int(black_moves_in),
        "black_moves_escape": int(black_moves_escape),
        "white_moves_in": int(white_moves_in),
        "white_moves_escape": int(white_moves_escape),
        "checkmates": int(checkmates),
        "trap": int(len(trap_set)),
        "tempo": int(len(tempo_set)),
        "mate": int(len(mate_set)),
    }


def tempo_trap_buchi(
    ice,
    trap_set: Set[State],
    black_in: Dict[State, List[State]],
    white_in: Dict[State, List[State]],
) -> Set[State]:
    # Black nodes = trap_set
    b_list: List[State] = list(trap_set)
    b_index: Dict[State, int] = {s: i for i, s in enumerate(b_list)}

    # White nodes discovered from black moves.
    w_list: List[State] = []
    w_index: Dict[State, int] = {}
    bw_succ: List[List[int]] = [[] for _ in range(len(b_list))]

    for bi, b in enumerate(b_list):
        succ_w: List[int] = []
        for w in black_in[b]:
            wi = w_index.get(w)
            if wi is None:
                wi = len(w_list)
                w_list.append(w)
                w_index[w] = wi
            succ_w.append(wi)
        bw_succ[bi] = sorted(set(succ_w))

    # White->black edges (only replies that stay in trap_set).
    wb_succ: List[List[int]] = [[] for _ in range(len(w_list))]
    for wi, w in enumerate(w_list):
        succ_b = [b_index[b] for b in white_in[w] if b in b_index]
        wb_succ[wi] = sorted(set(succ_b))

    # Accepting white nodes: pass enabled and passing stays in trap_set.
    is_accept_w: List[bool] = [w in b_index for w in w_list]

    in_z_b: List[bool] = [True] * len(b_list)
    in_z_w: List[bool] = [True] * len(w_list)

    while True:
        in_y_b, in_y_w = attractor_white(in_z_b, in_z_w, bw_succ, wb_succ, is_accept_w)

        target_b = [in_z_b[i] and (not in_y_b[i]) for i in range(len(b_list))]
        target_w = [in_z_w[i] and (not in_y_w[i]) for i in range(len(w_list))]

        in_x_b, in_x_w = attractor_black(in_z_b, in_z_w, bw_succ, wb_succ, target_b, target_w)

        any_removed = False
        for i in range(len(b_list)):
            if in_z_b[i] and in_x_b[i]:
                in_z_b[i] = False
                any_removed = True
        for i in range(len(w_list)):
            if in_z_w[i] and in_x_w[i]:
                in_z_w[i] = False
                any_removed = True

        if not any_removed:
            break

    return {b_list[i] for i in range(len(b_list)) if in_z_b[i]}


def attractor_white(
    in_z_b: Sequence[bool],
    in_z_w: Sequence[bool],
    bw_succ: Sequence[Sequence[int]],
    wb_succ: Sequence[Sequence[int]],
    is_accept_w: Sequence[bool],
) -> Tuple[List[bool], List[bool]]:
    b_len = len(in_z_b)
    w_len = len(in_z_w)
    in_a_b = [False] * b_len
    in_a_w = [False] * w_len

    for wi in range(w_len):
        if in_z_w[wi] and is_accept_w[wi]:
            in_a_w[wi] = True

    changed = True
    while changed:
        changed = False

        # White nodes: exists succ in A.
        for wi in range(w_len):
            if not in_z_w[wi] or in_a_w[wi]:
                continue
            if any(in_z_b[bi] and in_a_b[bi] for bi in wb_succ[wi]):
                in_a_w[wi] = True
                changed = True

        # Black nodes: all succ in A (and succ non-empty inside Z).
        for bi in range(b_len):
            if not in_z_b[bi] or in_a_b[bi]:
                continue
            saw = False
            all_in = True
            for wi in bw_succ[bi]:
                if not in_z_w[wi]:
                    continue
                saw = True
                if not in_a_w[wi]:
                    all_in = False
                    break
            if saw and all_in:
                in_a_b[bi] = True
                changed = True

    return in_a_b, in_a_w


def attractor_black(
    in_z_b: Sequence[bool],
    in_z_w: Sequence[bool],
    bw_succ: Sequence[Sequence[int]],
    wb_succ: Sequence[Sequence[int]],
    target_b: Sequence[bool],
    target_w: Sequence[bool],
) -> Tuple[List[bool], List[bool]]:
    b_len = len(in_z_b)
    w_len = len(in_z_w)
    in_a_b = [False] * b_len
    in_a_w = [False] * w_len

    for bi in range(b_len):
        if in_z_b[bi] and target_b[bi]:
            in_a_b[bi] = True
    for wi in range(w_len):
        if in_z_w[wi] and target_w[wi]:
            in_a_w[wi] = True

    changed = True
    while changed:
        changed = False

        # Black nodes: exists succ in A.
        for bi in range(b_len):
            if not in_z_b[bi] or in_a_b[bi]:
                continue
            if any(in_z_w[wi] and in_a_w[wi] for wi in bw_succ[bi]):
                in_a_b[bi] = True
                changed = True

        # White nodes: all succ in A (and succ non-empty inside Z).
        for wi in range(w_len):
            if not in_z_w[wi] or in_a_w[wi]:
                continue
            saw = False
            all_in = True
            for bi in wb_succ[wi]:
                if not in_z_b[bi]:
                    continue
                saw = True
                if not in_a_b[bi]:
                    all_in = False
                    break
            if saw and all_in:
                in_a_w[wi] = True
                changed = True

    return in_a_b, in_a_w


def forced_mate_region(
    universe: Sequence[State],
    universe_set: Set[State],
    attacked: Dict[State, bool],
    black_in: Dict[State, List[State]],
    black_escape: Dict[State, bool],
    white_in: Dict[State, List[State]],
) -> Set[State]:
    placements: List[State] = list(universe)
    idx: Dict[State, int] = {s: i for i, s in enumerate(placements)}
    n = len(placements)

    bw_succ: List[List[int]] = [[] for _ in range(n)]
    wb_succ: List[List[int]] = [[] for _ in range(n)]
    has_escape: List[bool] = [False] * n

    for i, p in enumerate(placements):
        has_escape[i] = bool(black_escape[p])
        bw_succ[i] = [idx[q] for q in black_in[p] if q in idx]
        wb_succ[i] = [idx[q] for q in white_in[p] if q in idx]

    # Reverse edges.
    pred_b_of_w: List[List[int]] = [[] for _ in range(n)]
    pred_w_of_b: List[List[int]] = [[] for _ in range(n)]
    for bi in range(n):
        for wi in bw_succ[bi]:
            pred_b_of_w[wi].append(bi)
    for wi in range(n):
        for bi in wb_succ[wi]:
            pred_w_of_b[bi].append(wi)

    for v in pred_b_of_w:
        v.sort()
    for v in pred_w_of_b:
        v.sort()

    is_mate = [False] * n
    win_b = [False] * n
    win_w = [False] * n

    remaining = [0] * n
    for bi in range(n):
        remaining[bi] = len(bw_succ[bi]) + (1 if has_escape[bi] else 0)

    q: deque[Tuple[str, int]] = deque()

    # Terminal mates.
    for bi in range(n):
        if has_escape[bi] or bw_succ[bi]:
            continue
        if attacked[placements[bi]]:
            is_mate[bi] = True
            win_b[bi] = True
            q.append(("b", bi))

    while q:
        kind, i = q.popleft()
        if kind == "b":
            # White nodes: exists move to winning black node.
            for wi in pred_w_of_b[i]:
                if win_w[wi]:
                    continue
                win_w[wi] = True
                q.append(("w", wi))
        else:
            # Black nodes: all moves must go to winning white nodes, and no escape.
            for bi in pred_b_of_w[i]:
                if win_b[bi]:
                    continue
                if remaining[bi] > 0:
                    remaining[bi] -= 1
                if remaining[bi] == 0 and bw_succ[bi]:
                    win_b[bi] = True
                    q.append(("b", bi))

    return {placements[i] for i in range(n) if win_b[i]}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario_json", nargs="+", help="scenario JSON file(s)")
    ap.add_argument(
        "--py-repo",
        default=os.environ.get("ICE_PY_REPO"),
        help="Path to a clone of mjtb49/InfiniteChessEndgameScripts (or set ICE_PY_REPO)",
    )
    ap.add_argument("--pretty", action="store_true", help="pretty-print JSON")
    args = ap.parse_args()

    if not args.py_repo:
        print("error: --py-repo is required (or set ICE_PY_REPO)", file=sys.stderr)
        return 2

    ice = import_tablebase(args.py_repo)

    outputs = []
    for path in args.scenario_json:
        spec = load_scenario(path)
        counts = compute_counts(ice, spec)
        out = {"scenario": scenario_json(spec), "counts": counts}
        outputs.append(out)

    if len(outputs) == 1:
        text = json.dumps(outputs[0], indent=2, sort_keys=True) if args.pretty else json.dumps(outputs[0], sort_keys=True)
        print(text)
    else:
        text = json.dumps(outputs, indent=2, sort_keys=True) if args.pretty else json.dumps(outputs, sort_keys=True)
        print(text)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
