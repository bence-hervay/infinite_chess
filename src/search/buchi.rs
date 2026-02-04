//! Büchi-game solver for the "tempo trap" refinement.
//!
//! The game graph is bipartite:
//! - Black nodes: black-to-move states inside an inescapable trap.
//! - White nodes: states that arise immediately after a legal black king move.
//!
//! White chooses a reply (including optional pass), and we only keep replies that
//! stay inside the inescapable trap.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::scenario::{DomainLike, LawsLike, Scenario, SearchError, State};
use crate::search::movegen::{legal_black_moves, legal_white_moves};
use crate::search::resources::ResourceTracker;

#[derive(Debug)]
struct BuchiGraph {
    b_list: Vec<State>,
    b_index: FxHashMap<State, usize>,
    w_list: Vec<State>,
    bw_succ: Vec<Vec<usize>>,
    wb_succ: Vec<Vec<usize>>,
    is_accept_w: Vec<bool>,
    in_z_b: Vec<bool>,
    in_z_w: Vec<bool>,
}

/// Compute the maximal tempo trap via a Büchi winning-region algorithm.
///
/// Returned set is a subset of `btm_trap` (black-to-move states).
pub fn tempo_trap_buchi<D, L, P>(
    scn: &Scenario<D, L, P>,
    btm_trap: &FxHashSet<State>,
) -> Result<FxHashSet<State>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    let g = compute_winning_region(scn, btm_trap)?;
    Ok(extract_b_set(&g))
}

/// Compute the maximal tempo trap plus a memoryless White strategy within it.
///
/// The returned map is keyed by *white-to-move* nodes (states after a black move) and chooses a
/// black-to-move successor that keeps play inside the tempo trap.
pub fn tempo_trap_buchi_with_strategy<D, L, P>(
    scn: &Scenario<D, L, P>,
    btm_trap: &FxHashSet<State>,
) -> Result<(FxHashSet<State>, FxHashMap<State, State>), SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    let g = compute_winning_region(scn, btm_trap)?;
    Ok((extract_b_set(&g), extract_tempo_strategy(&g)?))
}

/// Attractor to the accepting set for White.
///
/// Player 1 = White.
/// - White nodes join the attractor if they have *some* edge into it.
/// - Black nodes join if *all* their edges (within Z) go into it.
///
/// We intentionally require black nodes to have at least one successor inside Z;
/// otherwise the play ends and cannot satisfy an "infinitely often" objective.
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

/// Attractor to the accepting set for White, plus a witness move for white nodes.
///
/// The witness for a white node is a black successor index that moves "towards" the attractor.
fn attractor_white_with_witness(
    in_z_b: &[bool],
    in_z_w: &[bool],
    bw_succ: &[Vec<usize>],
    wb_succ: &[Vec<usize>],
    is_accept_w: &[bool],
) -> (Vec<bool>, Vec<bool>, Vec<Option<usize>>) {
    let b_len = in_z_b.len();
    let w_len = in_z_w.len();

    let mut in_a_b: Vec<bool> = vec![false; b_len];
    let mut in_a_w: Vec<bool> = vec![false; w_len];
    let mut witness_w: Vec<Option<usize>> = vec![None; w_len];

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
            if let Some(&bi) = wb_succ[wi].iter().find(|&&bi| in_z_b[bi] && in_a_b[bi]) {
                in_a_w[wi] = true;
                witness_w[wi] = Some(bi);
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

    (in_a_b, in_a_w, witness_w)
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

fn compute_winning_region<D, L, P>(
    scn: &Scenario<D, L, P>,
    btm_trap: &FxHashSet<State>,
) -> Result<BuchiGraph, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    let mut tracker = ResourceTracker::new(scn.limits);

    // Index black nodes.
    let b_list: Vec<State> = btm_trap.iter().cloned().collect();
    tracker.bump_states("buchi_black_nodes", b_list.len())?;

    let b_len = b_list.len();
    let mut b_index: FxHashMap<State, usize> = FxHashMap::default();
    tracker.try_reserve_map("buchi_black_index", "b_index", &mut b_index, b_len)?;
    for (i, p) in b_list.iter().enumerate() {
        b_index.insert(p.clone(), i);
    }

    // Discover white nodes and black->white edges.
    let mut w_list: Vec<State> = Vec::new();
    let mut w_index: FxHashMap<State, usize> = FxHashMap::default();
    let mut bw_succ: Vec<Vec<usize>> = vec![Vec::new(); b_len];

    for (bi, bpos) in b_list.iter().enumerate() {
        tracker.bump_steps("buchi_build_bw", 1)?;

        let mut succ_w: Vec<usize> = Vec::new();
        for wpos in legal_black_moves(scn, &scn.laws, bpos, &mut tracker)? {
            let wi = if let Some(&existing) = w_index.get(&wpos) {
                existing
            } else {
                let idx = w_list.len();
                w_list.push(wpos.clone());
                w_index.insert(wpos.clone(), idx);
                tracker.bump_states("buchi_white_nodes", 1)?;
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
        tracker.bump_steps("buchi_build_wb", 1)?;

        let mut succ_b: Vec<usize> = Vec::new();
        for bnext in legal_white_moves(scn, &scn.laws, wpos, &mut tracker)? {
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
        if scn.white_can_pass && scn.laws.allow_pass(wpos) && b_index.contains_key(wpos) {
            is_accept_w[wi] = true;
        }
    }

    // Z is the current candidate winning region (subgame). Initially all nodes.
    let mut in_z_b: Vec<bool> = vec![true; b_len];
    let mut in_z_w: Vec<bool> = vec![true; w_len];

    loop {
        tracker.bump_steps("buchi_iter", 1)?;

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

    Ok(BuchiGraph {
        b_list,
        b_index,
        w_list,
        bw_succ,
        wb_succ,
        is_accept_w,
        in_z_b,
        in_z_w,
    })
}

fn extract_b_set(g: &BuchiGraph) -> FxHashSet<State> {
    let mut out: FxHashSet<State> = FxHashSet::default();
    for (i, p) in g.b_list.iter().cloned().enumerate() {
        if g.in_z_b[i] {
            out.insert(p);
        }
    }
    out
}

fn extract_tempo_strategy(g: &BuchiGraph) -> Result<FxHashMap<State, State>, SearchError> {
    let b_len = g.b_list.len();
    let w_len = g.w_list.len();

    // Compute an attractor to the accepting set within the final winning subgame.
    let (_in_a_b, in_a_w, witness_w) =
        attractor_white_with_witness(&g.in_z_b, &g.in_z_w, &g.bw_succ, &g.wb_succ, &g.is_accept_w);

    // Extract a memoryless strategy for all winning white nodes.
    let mut out: FxHashMap<State, State> = FxHashMap::default();

    for wi in 0..w_len {
        if !g.in_z_w[wi] {
            continue;
        }

        let succ_in_z: Vec<usize> = g.wb_succ[wi]
            .iter()
            .copied()
            .filter(|&bi| bi < b_len && g.in_z_b[bi])
            .collect();
        if succ_in_z.is_empty() {
            return Err(SearchError::InvalidScenario {
                reason:
                    "tempo strategy extraction found a terminal white node inside winning subgame"
                        .to_string(),
            });
        }

        let chosen_bi = if g.is_accept_w[wi] {
            // Prefer pass if it stays in the winning region.
            if let Some(&pass_bi) = g.b_index.get(&g.w_list[wi]) {
                if pass_bi < b_len && g.in_z_b[pass_bi] {
                    pass_bi
                } else {
                    succ_in_z[0]
                }
            } else {
                succ_in_z[0]
            }
        } else if in_a_w[wi] {
            witness_w[wi]
                .filter(|&bi| bi < b_len && g.in_z_b[bi])
                .unwrap_or(succ_in_z[0])
        } else {
            succ_in_z[0]
        };

        out.insert(g.w_list[wi].clone(), g.b_list[chosen_bi].clone());
    }

    Ok(out)
}
