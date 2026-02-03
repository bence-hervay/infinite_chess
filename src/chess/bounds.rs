use crate::chess::layout::PieceLayout;
use crate::chess::piece::PieceKind;
use crate::core::coord::Coord;
use crate::core::position::{Position, MAX_PIECES};
use crate::core::square::Square;

/// All squares within the L∞ bound, excluding the origin.
pub fn squares_in_linf_ball(bound: i32) -> Vec<Square> {
    let mut out = Vec::new();
    for x in -bound..=bound {
        for y in -bound..=bound {
            if x == 0 && y == 0 {
                continue;
            }
            out.push(Square::from_coord(Coord::new(x, y)));
        }
    }
    out.sort_unstable();
    out
}

/// Enumerate all **canonical** positions where every non-captured piece lies within `|x|,|y| <= bound`.
///
/// If `allow_captures` is true, each piece may also be `Square::NONE`.
///
/// Enumeration respects:
/// - distinct squares for all non-captured pieces
/// - the white king (if present) cannot be adjacent to the black king at the origin
pub fn enumerate_positions_in_bound(
    layout: &PieceLayout,
    bound: i32,
    allow_captures: bool,
) -> Vec<Position> {
    assert!(layout.piece_count() <= MAX_PIECES);

    let squares = squares_in_linf_ball(bound);
    // We'll track squares by index for a cheap "used" bitmap.
    let mut used = vec![false; squares.len()];

    let mut cur = [Square::NONE; MAX_PIECES];
    let mut out = Vec::new();

    fn choose_k(
        squares: &[Square],
        used: &mut [bool],
        allowed: impl Fn(usize) -> bool + Copy,
        start: usize,
        k: usize,
        chosen: &mut Vec<usize>,
        f: &mut dyn FnMut(&[usize], &mut [bool]),
    ) {
        if chosen.len() == k {
            f(chosen, used);
            return;
        }
        for i in start..squares.len() {
            if used[i] || !allowed(i) {
                continue;
            }
            chosen.push(i);
            choose_k(squares, used, allowed, i + 1, k, chosen, f);
            chosen.pop();
        }
    }

    fn rec(
        group_idx: usize,
        squares: &[Square],
        used: &mut [bool],
        layout: &PieceLayout,
        allow_captures: bool,
        cur: &mut [Square; MAX_PIECES],
        out: &mut Vec<Position>,
    ) {
        if group_idx == layout.identical_runs().len() {
            let mut pos = Position::new(layout.piece_count(), *cur);
            // Cur is constructed to already be canonical, but keep this call as an invariant check.
            pos.canonicalize(layout);
            out.push(pos);
            return;
        }

        let run = &layout.identical_runs()[group_idx];
        let kind = layout.kind(run.start);
        let len = run.end - run.start;

        // Special legality for the white king: cannot be adjacent to origin.
        let allowed_square = |idx: usize| -> bool {
            let c = squares[idx].coord();
            if kind == PieceKind::King {
                c.chebyshev_norm() > 1
            } else {
                true
            }
        };

        let min_k = if allow_captures { 0 } else { len };
        let max_k = len;
        let mut chosen = Vec::new();

        for k in min_k..=max_k {
            let mut callback = |chosen_indices: &[usize], used: &mut [bool]| {
                // Mark chosen squares as used.
                for &idx in chosen_indices {
                    used[idx] = true;
                }

                // Fill this run: first Nones, then chosen squares in ascending order.
                let none_count = len - k;
                for j in 0..none_count {
                    cur[run.start + j] = Square::NONE;
                }
                for (offset, &idx) in chosen_indices.iter().enumerate() {
                    cur[run.start + none_count + offset] = squares[idx];
                }

                rec(
                    group_idx + 1,
                    squares,
                    used,
                    layout,
                    allow_captures,
                    cur,
                    out,
                );

                // Unmark.
                for &idx in chosen_indices {
                    used[idx] = false;
                }
            };
            choose_k(
                squares,
                used,
                allowed_square,
                0,
                k,
                &mut chosen,
                &mut callback,
            );
        }
    }

    rec(
        0,
        &squares,
        &mut used,
        layout,
        allow_captures,
        &mut cur,
        &mut out,
    );

    out
}

/// Convenience: filter an iterator of squares to those within a bound.
#[inline]
pub fn is_in_bound(sq: Square, bound: i32) -> bool {
    if sq.is_none() {
        return true;
    }
    sq.coord().in_linf_bound(bound)
}
