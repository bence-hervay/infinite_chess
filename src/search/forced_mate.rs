//! Forced-mate solver in a bounded universe (AbsBox-style).
//!
//! This is a reachability objective on a finite state space:
//! White wins if it can force reaching a black-to-move checkmate node in finite time.
//!
//! Semantics:
//! - The universe is the scenario's candidate set (typically `CandidateGeneration::InAbsBox`).
//! - Any legal black move that leaves the universe is treated as an escape, so the origin state
//!   cannot be proven winning.
//! - White moves that leave the universe are treated as illegal (ignored).

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::core::coord::Coord;
use crate::scenario::{CandidateGeneration, DomainLike, LawsLike, Scenario, SearchError, State};
use crate::search::movegen::{legal_black_moves, legal_white_moves};
use crate::search::resources::ResourceTracker;
use crate::search::universe::try_for_each_state_in_abs_box;

#[derive(Debug, Clone)]
pub struct ForcedMateResult {
    /// Winning black-to-move placements (White can force mate from these positions).
    pub winning_btm: FxHashSet<State>,
    /// Optional distance-to-mate (plies) for winning black-to-move placements.
    pub dtm: Option<FxHashMap<State, u32>>,
}

/// Compute the winning region for a forced mate inside a bounded universe.
///
/// This routine currently requires `CandidateGeneration::InAbsBox` so that "leaving the universe"
/// (e.g. walking beyond the absolute bound) is observable.
pub fn forced_mate_bounded<D, L, P>(
    scn: &Scenario<D, L, P>,
    compute_dtm: bool,
) -> Result<ForcedMateResult, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    scn.validate()?;

    let (bound, allow_captures) = match scn.candidates {
        CandidateGeneration::InAbsBox {
            bound,
            allow_captures,
        } => (bound, allow_captures),
        _ => {
            return Err(SearchError::InvalidScenario {
                reason: "forced_mate_bounded currently requires candidates=InAbsBox".to_string(),
            })
        }
    };

    let mut tracker = ResourceTracker::new(scn.limits);

    // Build universe placements.
    let mut universe: FxHashSet<State> = FxHashSet::default();
    try_for_each_state_in_abs_box(&scn.rules.layout, bound, allow_captures, |s| {
        if !scn.rules.is_legal_position(&s.pos) {
            return Ok(());
        }
        if !scn.laws.allow_state(&s) {
            return Ok(());
        }
        if !scn.domain.inside(&s) {
            return Ok(());
        }

        if universe.insert(s) {
            tracker.bump_states("mate_universe", 1)?;
        }
        Ok(())
    })?;

    let placements: Vec<State> = universe.iter().cloned().collect();
    let n = placements.len();

    // Index placements.
    let mut idx: FxHashMap<State, usize> = FxHashMap::default();
    tracker.try_reserve_map("mate_index", "placement_index", &mut idx, n)?;
    for (i, p) in placements.iter().enumerate() {
        idx.insert(p.clone(), i);
    }

    // Build in-universe move graph, plus "escape edge" markers for black nodes.
    let mut bw_succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut wb_succ: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut black_has_escape: Vec<bool> = vec![false; n];

    for (i, p) in placements.iter().enumerate() {
        tracker.bump_steps("mate_build_edges", 1)?;

        let mut b_out: Vec<usize> = Vec::with_capacity(8);
        for wpos in legal_black_moves(scn, &scn.laws, p, &mut tracker)? {
            if let Some(&j) = idx.get(&wpos) {
                b_out.push(j);
            } else {
                black_has_escape[i] = true;
            }
        }
        b_out.sort_unstable();
        b_out.dedup();
        bw_succ[i] = b_out;

        let mut w_out: Vec<usize> = Vec::new();
        for bpos in legal_white_moves(scn, &scn.laws, p, &mut tracker)? {
            if let Some(&j) = idx.get(&bpos) {
                w_out.push(j);
            }
        }
        w_out.sort_unstable();
        w_out.dedup();
        wb_succ[i] = w_out;
    }

    // Reverse edges.
    let mut pred_b_of_w: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut pred_w_of_b: Vec<Vec<usize>> = vec![Vec::new(); n];
    for bi in 0..n {
        for &wi in bw_succ[bi].iter() {
            pred_b_of_w[wi].push(bi);
        }
    }
    for wi in 0..n {
        for &bi in wb_succ[wi].iter() {
            pred_w_of_b[bi].push(wi);
        }
    }
    for v in pred_b_of_w.iter_mut() {
        v.sort_unstable();
        v.dedup();
    }
    for v in pred_w_of_b.iter_mut() {
        v.sort_unstable();
        v.dedup();
    }

    // Mate terminals (bounded-universe): in check, no in-universe move, and no escape move.
    let mut is_mate: Vec<bool> = vec![false; n];
    let mut win_b: Vec<bool> = vec![false; n];
    let mut win_w: Vec<bool> = vec![false; n];

    let mut remaining_nonwin_w_succ: Vec<usize> = vec![0; n];
    for bi in 0..n {
        remaining_nonwin_w_succ[bi] = bw_succ[bi].len() + if black_has_escape[bi] { 1 } else { 0 };
    }

    let mut q: VecDeque<Node> = VecDeque::new();
    for bi in 0..n {
        if black_has_escape[bi] || !bw_succ[bi].is_empty() {
            continue;
        }
        if scn.rules.is_attacked(Coord::ORIGIN, &placements[bi].pos) {
            is_mate[bi] = true;
            win_b[bi] = true;
            q.push_back(Node::Black(bi));
        }
    }

    // Attractor computation for reachability to mate.
    while let Some(node) = q.pop_front() {
        tracker.bump_steps("mate_attractor", 1)?;
        match node {
            Node::Black(bi) => {
                // White nodes: exists move to winning black node.
                for &wi in pred_w_of_b[bi].iter() {
                    if win_w[wi] {
                        continue;
                    }
                    win_w[wi] = true;
                    q.push_back(Node::White(wi));
                }
            }
            Node::White(wi) => {
                // Black nodes: all moves must go to winning white nodes, and no escape move.
                for &bi in pred_b_of_w[wi].iter() {
                    if win_b[bi] {
                        continue;
                    }
                    if remaining_nonwin_w_succ[bi] > 0 {
                        remaining_nonwin_w_succ[bi] -= 1;
                    }
                    if remaining_nonwin_w_succ[bi] == 0 && !bw_succ[bi].is_empty() {
                        win_b[bi] = true;
                        q.push_back(Node::Black(bi));
                    }
                }
            }
        }
    }

    let mut winning_btm: FxHashSet<State> = FxHashSet::default();
    for bi in 0..n {
        if win_b[bi] {
            winning_btm.insert(placements[bi].clone());
        }
    }

    let dtm = if compute_dtm {
        Some(compute_dtm_layers(
            scn,
            &mut tracker,
            &placements,
            &bw_succ,
            &wb_succ,
            &win_b,
            &win_w,
            &is_mate,
        )?)
    } else {
        None
    };

    Ok(ForcedMateResult { winning_btm, dtm })
}

#[derive(Debug, Clone, Copy)]
enum Node {
    Black(usize),
    White(usize),
}

fn compute_dtm_layers<D, L, P>(
    scn: &Scenario<D, L, P>,
    tracker: &mut ResourceTracker,
    placements: &[State],
    bw_succ: &[Vec<usize>],
    wb_succ: &[Vec<usize>],
    win_b: &[bool],
    win_w: &[bool],
    is_mate: &[bool],
) -> Result<FxHashMap<State, u32>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    let n = placements.len();
    let inf = u32::MAX;

    let mut dtm_b: Vec<u32> = vec![inf; n];
    let mut dtm_w: Vec<u32> = vec![inf; n];

    for bi in 0..n {
        if win_b[bi] && is_mate[bi] {
            dtm_b[bi] = 0;
        }
    }

    loop {
        tracker.bump_steps("mate_dtm_iter", 1)?;

        let mut changed = false;

        // White nodes: 1 + min successor dtm_b.
        for wi in 0..n {
            if !win_w[wi] {
                continue;
            }
            let mut best = inf;
            for &bi in wb_succ[wi].iter() {
                if !win_b[bi] {
                    continue;
                }
                best = best.min(dtm_b[bi]);
            }
            let cand = if best == inf {
                inf
            } else {
                best.saturating_add(1)
            };
            if cand < dtm_w[wi] {
                dtm_w[wi] = cand;
                changed = true;
            }
        }

        // Black nodes: 1 + max successor dtm_w.
        for bi in 0..n {
            if !win_b[bi] || is_mate[bi] {
                continue;
            }

            // Winning non-mate black nodes must have at least one in-universe move.
            if bw_succ[bi].is_empty() {
                return Err(SearchError::InvalidScenario {
                    reason: "DTM requested but found a winning non-mate black node with no moves"
                        .to_string(),
                });
            }

            let mut max_v = 0u32;
            for &wi in bw_succ[bi].iter() {
                if !win_w[wi] {
                    // Should not happen inside winning region.
                    return Err(SearchError::InvalidScenario {
                        reason: "DTM requested but winning black node has non-winning successor"
                            .to_string(),
                    });
                }
                let v = dtm_w[wi];
                if v == inf {
                    max_v = inf;
                    break;
                }
                max_v = max_v.max(v);
            }
            let cand = if max_v == inf {
                inf
            } else {
                max_v.saturating_add(1)
            };
            if cand < dtm_b[bi] {
                dtm_b[bi] = cand;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    // Extract winning black nodes.
    let mut out: FxHashMap<State, u32> = FxHashMap::default();
    tracker.try_reserve_map("mate_dtm", "dtm_map", &mut out, n)?;

    for bi in 0..n {
        if !win_b[bi] {
            continue;
        }
        let v = dtm_b[bi];
        if v == inf {
            // This indicates non-convergence or a bug.
            return Err(SearchError::InvalidScenario {
                reason: "DTM did not converge for all winning nodes".to_string(),
            });
        }
        out.insert(placements[bi].clone(), v);
    }

    // Sanity: all mates are dtm=0 and in check.
    for (s, &d) in out.iter() {
        if d == 0 && !scn.rules.is_attacked(Coord::ORIGIN, &s.pos) {
            return Err(SearchError::InvalidScenario {
                reason: "DTM map contains a depth-0 node that is not in check".to_string(),
            });
        }
    }

    Ok(out)
}
