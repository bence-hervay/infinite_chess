//! Bounded-universe enumeration helpers.
//!
//! The core representation stores white pieces in king-relative coordinates, but some experiments
//! need an *absolute* bounding box for both the king anchor and all pieces.

use crate::chess::layout::PieceLayout;
use crate::chess::piece::PieceKind;
use crate::core::coord::Coord;
use crate::core::position::{Position, MAX_PIECES};
use crate::core::square::Square;
use crate::scenario::State;

/// Enumerate all **canonical** placements inside an absolute `[-bound, bound] × [-bound, bound]`.
///
/// Returned states store:
/// - `abs_king` as the absolute black king coordinate, and
/// - `pos` as king-relative piece squares (with the king at the origin).
///
/// If `allow_captures` is true, each piece may also be absent (`Square::NONE`).
pub fn for_each_state_in_abs_box(
    layout: &PieceLayout,
    bound: i32,
    allow_captures: bool,
    mut f: impl FnMut(State),
) {
    // A wrapper that cannot fail.
    try_for_each_state_in_abs_box(layout, bound, allow_captures, |s| {
        f(s);
        Ok(())
    })
    .unwrap_or_else(|never: std::convert::Infallible| match never {});
}

/// Like [`for_each_state_in_abs_box`], but allows early exit via a fallible callback.
pub fn try_for_each_state_in_abs_box<E>(
    layout: &PieceLayout,
    bound: i32,
    allow_captures: bool,
    mut f: impl FnMut(State) -> Result<(), E>,
) -> Result<(), E> {
    assert!(bound >= 0);
    assert!(layout.piece_count() <= MAX_PIECES);

    let side = (2 * bound + 1) as usize;
    let square_count = side * side;

    // Absolute squares in the box, ordered lexicographically by (x, y).
    let mut abs_squares: Vec<Square> = Vec::with_capacity(square_count);
    for x in -bound..=bound {
        for y in -bound..=bound {
            abs_squares.push(Square::from_coord(Coord::new(x, y)));
        }
    }

    let mut used: Vec<bool> = vec![false; abs_squares.len()];
    let mut cur_abs = [Square::NONE; MAX_PIECES];

    fn choose_k<E>(
        abs_squares: &[Square],
        used: &mut [bool],
        allowed: impl Fn(usize) -> bool + Copy,
        start: usize,
        k: usize,
        chosen: &mut Vec<usize>,
        f: &mut dyn FnMut(&[usize], &mut [bool]) -> Result<(), E>,
    ) -> Result<(), E> {
        if chosen.len() == k {
            return f(chosen, used);
        }
        for i in start..abs_squares.len() {
            if used[i] || !allowed(i) {
                continue;
            }
            chosen.push(i);
            choose_k(abs_squares, used, allowed, i + 1, k, chosen, f)?;
            chosen.pop();
        }
        Ok(())
    }

    fn rec<E>(
        group_idx: usize,
        abs_squares: &[Square],
        used: &mut [bool],
        layout: &PieceLayout,
        abs_king: Coord,
        allow_captures: bool,
        cur_abs: &mut [Square; MAX_PIECES],
        f: &mut dyn FnMut(State) -> Result<(), E>,
    ) -> Result<(), E> {
        if group_idx == layout.identical_runs().len() {
            let mut cur_rel = [Square::NONE; MAX_PIECES];
            for i in 0..layout.piece_count() {
                let sq = cur_abs[i];
                cur_rel[i] = if sq.is_none() {
                    Square::NONE
                } else {
                    Square::from_coord(sq.coord() - abs_king)
                };
            }
            let mut pos = Position::new(layout.piece_count(), cur_rel);
            // Cur is constructed to already be canonical, but keep this call as an invariant check.
            pos.canonicalize(layout);
            f(State::new(abs_king, pos))?;
            return Ok(());
        }

        let run = &layout.identical_runs()[group_idx];
        let kind = layout.kind(run.start);
        let len = run.end - run.start;

        // Special legality for the white king: cannot be adjacent to the black king.
        let allowed_square = |idx: usize| -> bool {
            if kind == PieceKind::King {
                let rel = abs_squares[idx].coord() - abs_king;
                rel.chebyshev_norm() > 1
            } else {
                true
            }
        };

        let min_k = if allow_captures { 0 } else { len };
        let max_k = len;
        let mut chosen: Vec<usize> = Vec::new();

        for k in min_k..=max_k {
            let mut callback = |chosen_indices: &[usize], used: &mut [bool]| -> Result<(), E> {
                for &idx in chosen_indices {
                    used[idx] = true;
                }

                let none_count = len - k;
                for j in 0..none_count {
                    cur_abs[run.start + j] = Square::NONE;
                }
                for (offset, &idx) in chosen_indices.iter().enumerate() {
                    cur_abs[run.start + none_count + offset] = abs_squares[idx];
                }

                rec(
                    group_idx + 1,
                    abs_squares,
                    used,
                    layout,
                    abs_king,
                    allow_captures,
                    cur_abs,
                    f,
                )?;

                for &idx in chosen_indices {
                    used[idx] = false;
                }
                Ok(())
            };

            choose_k(
                abs_squares,
                used,
                allowed_square,
                0,
                k,
                &mut chosen,
                &mut callback,
            )?;
        }
        Ok(())
    }

    for kx in -bound..=bound {
        for ky in -bound..=bound {
            let abs_king = Coord::new(kx, ky);

            // Reset used squares and exclude the king square itself.
            used.fill(false);
            let king_idx = ((kx + bound) as usize) * side + ((ky + bound) as usize);
            debug_assert!(king_idx < abs_squares.len());
            used[king_idx] = true;

            rec(
                0,
                &abs_squares,
                &mut used,
                layout,
                abs_king,
                allow_captures,
                &mut cur_abs,
                &mut f,
            )?;
        }
    }

    Ok(())
}
