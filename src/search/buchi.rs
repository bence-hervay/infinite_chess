//! Büchi-game solver for the "tempo trap" refinement.
//!
//! The game graph is bipartite:
//! - Black nodes: black-to-move positions inside an inescapable trap.
//! - White nodes: positions that arise immediately after a legal black king move.
//!
//! White chooses a reply (including optional pass), and we only keep replies that
//! stay inside the inescapable trap.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::chess::rules::Rules;
use crate::core::position::Position;

/// Compute the maximal tempo trap via a Büchi winning-region algorithm.
///
/// Returned set is a subset of `btm_trap` (black-to-move positions).
pub fn tempo_trap_buchi(
    rules: &Rules,
    btm_trap: &FxHashSet<Position>,
    white_can_pass: bool,
) -> FxHashSet<Position> {
    // Index black nodes.
    let b_list: Vec<Position> = btm_trap.iter().cloned().collect();
    let b_len = b_list.len();
    let mut b_index: FxHashMap<Position, usize> = FxHashMap::default();
    for (i, p) in b_list.iter().enumerate() {
        b_index.insert(p.clone(), i);
    }

    // Discover white nodes and black->white edges.
    let mut w_list: Vec<Position> = Vec::new();
    let mut w_index: FxHashMap<Position, usize> = FxHashMap::default();
    let mut bw_succ: Vec<Vec<usize>> = vec![Vec::new(); b_len];

    for (bi, bpos) in b_list.iter().enumerate() {
        let mut succ_w: Vec<usize> = Vec::new();
        for wpos in rules.black_moves(bpos).into_iter() {
            let wi = if let Some(&existing) = w_index.get(&wpos) {
                existing
            } else {
                let idx = w_list.len();
                w_list.push(wpos.clone());
                w_index.insert(wpos.clone(), idx);
                idx
            };
            succ_w.push(wi);
        }
        succ_w.sort_unstable();
        succ_w.dedup();
        bw_succ[bi] = succ_w;
    }

    let w_len = w_list.len();

    // White->black edges (only replies that stay inside btm_trap).
    let mut wb_succ: Vec<Vec<usize>> = vec![Vec::new(); w_len];
    for (wi, wpos) in w_list.iter().enumerate() {
        let mut succ_b: Vec<usize> = Vec::new();
        for bnext in rules.white_moves(wpos, white_can_pass).into_iter() {
            if let Some(&bi) = b_index.get(&bnext) {
                succ_b.push(bi);
            }
        }
        succ_b.sort_unstable();
        succ_b.dedup();
        wb_succ[wi] = succ_b;
    }

    // Acceptance set F: white nodes where passing is possible, i.e. the placement itself is in btm_trap.
    let mut is_accept_w: Vec<bool> = vec![false; w_len];
    for (wi, wpos) in w_list.iter().enumerate() {
        if b_index.contains_key(wpos) {
            is_accept_w[wi] = true;
        }
    }

    // Z is the current candidate winning region (subgame). Initially all nodes.
    let mut in_z_b: Vec<bool> = vec![true; b_len];
    let mut in_z_w: Vec<bool> = vec![true; w_len];

    loop {
        // Y = Attr_white(F) within Z.
        let (in_y_b, in_y_w) = attractor_white(&in_z_b, &in_z_w, &bw_succ, &wb_succ, &is_accept_w);

        // Target for black attractor is Z \ Y.
        let mut target_b: Vec<bool> = vec![false; b_len];
        let mut target_w: Vec<bool> = vec![false; w_len];
        for i in 0..b_len {
            if in_z_b[i] && !in_y_b[i] {
                target_b[i] = true;
            }
        }
        for i in 0..w_len {
            if in_z_w[i] && !in_y_w[i] {
                target_w[i] = true;
            }
        }

        let (in_x_b, in_x_w) =
            attractor_black(&in_z_b, &in_z_w, &bw_succ, &wb_succ, &target_b, &target_w);

        let mut any_removed = false;
        for i in 0..b_len {
            if in_z_b[i] && in_x_b[i] {
                in_z_b[i] = false;
                any_removed = true;
            }
        }
        for i in 0..w_len {
            if in_z_w[i] && in_x_w[i] {
                in_z_w[i] = false;
                any_removed = true;
            }
        }

        if !any_removed {
            break;
        }
    }

    // Return black positions that remain in Z.
    let mut out: FxHashSet<Position> = FxHashSet::default();
    for (i, p) in b_list.into_iter().enumerate() {
        if in_z_b[i] {
            out.insert(p);
        }
    }
    out
}

/// Attractor to the accepting set for White.
///
/// Player 1 = White.
/// - White nodes join the attractor if they have *some* edge into it.
/// - Black nodes join if *all* their edges (within Z) go into it.
///
/// We intentionally require black nodes to have at least one successor inside Z;
/// otherwise the play ends (mate/stalemate) and cannot satisfy an "infinitely often"
/// objective.
fn attractor_white(
    in_z_b: &[bool],
    in_z_w: &[bool],
    bw_succ: &[Vec<usize>],
    wb_succ: &[Vec<usize>],
    is_accept_w: &[bool],
) -> (Vec<bool>, Vec<bool>) {
    let b_len = in_z_b.len();
    let w_len = in_z_w.len();

    let mut in_a_b: Vec<bool> = vec![false; b_len];
    let mut in_a_w: Vec<bool> = vec![false; w_len];

    for wi in 0..w_len {
        if in_z_w[wi] && is_accept_w[wi] {
            in_a_w[wi] = true;
        }
    }

    let mut changed = true;
    while changed {
        changed = false;

        // White nodes: exists succ in A.
        for wi in 0..w_len {
            if !in_z_w[wi] || in_a_w[wi] {
                continue;
            }
            let has_edge = wb_succ[wi].iter().any(|&bi| in_z_b[bi] && in_a_b[bi]);
            if has_edge {
                in_a_w[wi] = true;
                changed = true;
            }
        }

        // Black nodes: all succ in A (and succ non-empty inside Z).
        for bi in 0..b_len {
            if !in_z_b[bi] || in_a_b[bi] {
                continue;
            }
            let mut saw_succ_in_z = false;
            let mut all_in_a = true;
            for &wi in bw_succ[bi].iter() {
                if !in_z_w[wi] {
                    continue;
                }
                saw_succ_in_z = true;
                if !in_a_w[wi] {
                    all_in_a = false;
                    break;
                }
            }
            if saw_succ_in_z && all_in_a {
                in_a_b[bi] = true;
                changed = true;
            }
        }
    }

    (in_a_b, in_a_w)
}

/// Attractor to a target set for Black.
///
/// Player 2 = Black.
/// - Black nodes join the attractor if they have *some* edge into it.
/// - White nodes join if *all* their edges (within Z) go into it.
fn attractor_black(
    in_z_b: &[bool],
    in_z_w: &[bool],
    bw_succ: &[Vec<usize>],
    wb_succ: &[Vec<usize>],
    target_b: &[bool],
    target_w: &[bool],
) -> (Vec<bool>, Vec<bool>) {
    let b_len = in_z_b.len();
    let w_len = in_z_w.len();

    let mut in_a_b: Vec<bool> = vec![false; b_len];
    let mut in_a_w: Vec<bool> = vec![false; w_len];

    // Seed with target.
    for bi in 0..b_len {
        if in_z_b[bi] && target_b[bi] {
            in_a_b[bi] = true;
        }
    }
    for wi in 0..w_len {
        if in_z_w[wi] && target_w[wi] {
            in_a_w[wi] = true;
        }
    }

    let mut changed = true;
    while changed {
        changed = false;

        // Black nodes: exists succ in A.
        for bi in 0..b_len {
            if !in_z_b[bi] || in_a_b[bi] {
                continue;
            }
            let has_edge = bw_succ[bi].iter().any(|&wi| in_z_w[wi] && in_a_w[wi]);
            if has_edge {
                in_a_b[bi] = true;
                changed = true;
            }
        }

        // White nodes: all succ in A (and succ non-empty inside Z).
        for wi in 0..w_len {
            if !in_z_w[wi] || in_a_w[wi] {
                continue;
            }
            let mut saw_succ_in_z = false;
            let mut all_in_a = true;
            for &bi in wb_succ[wi].iter() {
                if !in_z_b[bi] {
                    continue;
                }
                saw_succ_in_z = true;
                if !in_a_b[bi] {
                    all_in_a = false;
                    break;
                }
            }
            if saw_succ_in_z && all_in_a {
                in_a_w[wi] = true;
                changed = true;
            }
        }
    }

    (in_a_b, in_a_w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny hand-made Büchi game sanity check.
    ///
    /// Graph:
    ///   B0 -> W0
    ///   B1 -> W1
    ///   W0 -> B0 or B1
    ///   W1 -> B1
    /// Acceptance set: {W0}
    ///
    /// From B0, White can keep going through W0 forever -> winning.
    /// From B1, the play is stuck visiting only W1 -> losing.
    #[test]
    fn buchi_sanity_game() {
        // We'll "fake" the chess layer by directly calling the internal attractor functions.
        // Two black nodes, two white nodes.
        let b_len = 2usize;
        let w_len = 2usize;

        let bw_succ = vec![vec![0usize], vec![1usize]];
        let wb_succ = vec![vec![0usize, 1usize], vec![1usize]];
        let is_accept_w = vec![true, false];

        let mut in_z_b = vec![true; b_len];
        let mut in_z_w = vec![true; w_len];

        loop {
            let (in_y_b, in_y_w) =
                attractor_white(&in_z_b, &in_z_w, &bw_succ, &wb_succ, &is_accept_w);

            let mut target_b = vec![false; b_len];
            let mut target_w = vec![false; w_len];
            for i in 0..b_len {
                if in_z_b[i] && !in_y_b[i] {
                    target_b[i] = true;
                }
            }
            for i in 0..w_len {
                if in_z_w[i] && !in_y_w[i] {
                    target_w[i] = true;
                }
            }

            let (in_x_b, in_x_w) =
                attractor_black(&in_z_b, &in_z_w, &bw_succ, &wb_succ, &target_b, &target_w);

            let mut any_removed = false;
            for i in 0..b_len {
                if in_z_b[i] && in_x_b[i] {
                    in_z_b[i] = false;
                    any_removed = true;
                }
            }
            for i in 0..w_len {
                if in_z_w[i] && in_x_w[i] {
                    in_z_w[i] = false;
                    any_removed = true;
                }
            }

            if !any_removed {
                break;
            }
        }

        assert_eq!(in_z_b, vec![true, false]);
    }
}
