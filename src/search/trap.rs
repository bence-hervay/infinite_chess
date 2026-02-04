//! Trap and tempo-trap solvers.
//!
//! The core objective is “inescapable trap”:
//! a black-to-move state is in the trap iff for every legal black move there exists a legal white
//! reply that stays inside the current set (greatest fixed point).
//!
//! Domain membership is handled *by the candidate set*: candidates are states that satisfy
//! `domain.inside(state)`. Leaving the domain is allowed by the rules and laws, but then White may
//! fail to reply back into the candidate set, which is how “escape” is modeled.

use std::collections::VecDeque;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::chess::bounds::enumerate_positions_in_bound;
use crate::core::coord::Coord;
use crate::scenario::{
    CacheMode, CandidateGeneration, DomainLike, LawsLike, Scenario, SearchError, Side, State,
};
use crate::search::movegen::{is_stalemate_with_laws, legal_black_moves, legal_white_moves};
use crate::search::resources::ResourceTracker;
use crate::search::universe::try_for_each_state_in_abs_box;

/// Cache for move generation during trap pruning.
#[derive(Default)]
struct MoveCache {
    black: FxHashMap<State, Arc<[State]>>,
    white: FxHashMap<State, Arc<[State]>>,
}

impl MoveCache {
    fn black_moves<D, L, P>(
        &mut self,
        scn: &Scenario<D, L, P>,
        tracker: &mut ResourceTracker,
        s: &State,
    ) -> Result<Arc<[State]>, SearchError>
    where
        D: DomainLike,
        L: LawsLike,
    {
        let do_cache = matches!(
            scn.cache_mode,
            CacheMode::BlackOnly | CacheMode::BothBounded
        );
        if do_cache {
            if let Some(v) = self.black.get(s) {
                return Ok(v.clone());
            }
        }

        let moves = legal_black_moves(scn, &scn.laws, s, tracker)?;
        let arc: Arc<[State]> = moves.into();

        if do_cache {
            self.evict_to_fit(scn, tracker, 1, arc.len())?;
            tracker.try_reserve_map("cache_black", "black_move_cache", &mut self.black, 1)?;
            tracker.bump_cache_entries("cache_black", 1)?;
            tracker.bump_cached_moves("cache_black", arc.len())?;
            self.black.insert(s.clone(), arc.clone());
        }

        Ok(arc)
    }

    fn white_moves<D, L, P>(
        &mut self,
        scn: &Scenario<D, L, P>,
        tracker: &mut ResourceTracker,
        s: &State,
    ) -> Result<Arc<[State]>, SearchError>
    where
        D: DomainLike,
        L: LawsLike,
    {
        let do_cache = matches!(scn.cache_mode, CacheMode::BothBounded);
        if do_cache {
            if let Some(v) = self.white.get(s) {
                return Ok(v.clone());
            }
        }

        let moves = legal_white_moves(scn, &scn.laws, s, tracker)?;
        let arc: Arc<[State]> = moves.into();

        if do_cache {
            self.evict_to_fit(scn, tracker, 1, arc.len())?;
            tracker.try_reserve_map("cache_white", "white_move_cache", &mut self.white, 1)?;
            tracker.bump_cache_entries("cache_white", 1)?;
            tracker.bump_cached_moves("cache_white", arc.len())?;
            self.white.insert(s.clone(), arc.clone());
        }

        Ok(arc)
    }

    fn evict_to_fit<D, L, P>(
        &mut self,
        scn: &Scenario<D, L, P>,
        tracker: &mut ResourceTracker,
        add_entries: usize,
        add_moves: usize,
    ) -> Result<(), SearchError> {
        let counts = tracker.counts();

        let max_entries = scn.limits.max_cache_entries as u64;
        let max_moves = scn.limits.max_cached_moves as u64;

        let need_entries = add_entries as u64;
        let need_moves = add_moves as u64;

        if need_entries > max_entries {
            return Err(SearchError::LimitExceeded {
                stage: "cache",
                metric: "cache_entries",
                limit: max_entries,
                observed: need_entries,
                counts,
            });
        }
        if need_moves > max_moves {
            return Err(SearchError::LimitExceeded {
                stage: "cache",
                metric: "cached_moves",
                limit: max_moves,
                observed: need_moves,
                counts,
            });
        }

        while tracker.counts().cache_entries + need_entries > max_entries
            || tracker.counts().cached_moves + need_moves > max_moves
        {
            if !self.evict_one(tracker) {
                break;
            }
        }

        let counts = tracker.counts();
        if counts.cache_entries + need_entries > max_entries {
            return Err(SearchError::LimitExceeded {
                stage: "cache",
                metric: "cache_entries",
                limit: max_entries,
                observed: counts.cache_entries + need_entries,
                counts,
            });
        }
        if counts.cached_moves + need_moves > max_moves {
            return Err(SearchError::LimitExceeded {
                stage: "cache",
                metric: "cached_moves",
                limit: max_moves,
                observed: counts.cached_moves + need_moves,
                counts,
            });
        }

        Ok(())
    }

    fn evict_one(&mut self, tracker: &mut ResourceTracker) -> bool {
        if let Some((k, v_len)) = self.black.iter().next().map(|(k, v)| (k.clone(), v.len())) {
            self.black.remove(&k);
            tracker.dec_cache_entries(1);
            tracker.dec_cached_moves(v_len);
            return true;
        }
        if let Some((k, v_len)) = self.white.iter().next().map(|(k, v)| (k.clone(), v.len())) {
            self.white.remove(&k);
            tracker.dec_cache_entries(1);
            tracker.dec_cached_moves(v_len);
            return true;
        }
        false
    }
}

/// Compute the maximal inescapable trap inside the scenario's domain.
///
/// The returned set is a set of **black-to-move** states inside the domain.
pub fn maximal_inescapable_trap<D, L, P>(
    scn: &Scenario<D, L, P>,
) -> Result<FxHashSet<State>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    scn.validate()?;
    let mut tracker = ResourceTracker::new(scn.limits);

    let mut trap = initial_candidate_set(scn, &mut tracker)?;

    let mut cache = MoveCache::default();

    loop {
        tracker.bump_steps("trap_prune_iter", 1)?;

        let mut to_remove: Vec<State> = Vec::new();

        for p in trap.iter() {
            tracker.bump_steps("trap_prune_scan", 1)?;

            // If black has a move to a position from which every white reply exits the current set,
            // then `p` cannot be in an inescapable trap.
            let black_moves = cache.black_moves(scn, &mut tracker, p)?;

            let mut fails = false;
            for after_black in black_moves.iter() {
                let white_moves = cache.white_moves(scn, &mut tracker, after_black)?;
                let has_reply_in_trap = white_moves.iter().any(|q| trap.contains(q));
                if !has_reply_in_trap {
                    fails = true;
                    break;
                }
            }

            if fails {
                to_remove.push(p.clone());
            }
        }

        if to_remove.is_empty() {
            break;
        }

        for p in to_remove {
            trap.remove(&p);
        }
    }

    Ok(trap)
}

/// Compute the maximal *tempo* trap inside an already-computed inescapable trap.
///
/// A tempo trap is a Büchi objective: White must be able to stay inside the inescapable trap
/// forever *and* force infinitely many visits to "passable" white-to-move states.
pub fn maximal_tempo_trap<D, L, P>(
    scn: &Scenario<D, L, P>,
    inescapable: &FxHashSet<State>,
) -> Result<FxHashSet<State>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    scn.validate()?;
    crate::search::buchi::tempo_trap_buchi(scn, inescapable)
}

fn initial_candidate_set<D, L, P>(
    scn: &Scenario<D, L, P>,
    tracker: &mut ResourceTracker,
) -> Result<FxHashSet<State>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    let mut trap: FxHashSet<State> = FxHashSet::default();

    match &scn.candidates {
        CandidateGeneration::InLinfBound {
            bound,
            allow_captures,
        } => {
            let positions =
                enumerate_positions_in_bound(&scn.rules.layout, *bound, *allow_captures);

            for pos in positions {
                if !scn.rules.is_legal_position(&pos) {
                    continue;
                }
                let s = State {
                    abs_king: Coord::ORIGIN,
                    pos,
                };
                if !scn.laws.allow_state(&s) {
                    continue;
                }
                if !scn.domain.inside(&s) {
                    continue;
                }
                if scn.remove_stalemates && is_stalemate_with_laws(scn, &scn.laws, &s, tracker)? {
                    continue;
                }

                if trap.insert(s) {
                    tracker.bump_states("candidates_in_bound", 1)?;
                }
            }
        }

        CandidateGeneration::InBox {
            bound,
            allow_captures,
        } => {
            if !scn.track_abs_king {
                return Err(SearchError::InvalidScenario {
                    reason: "InBox candidate generation requires track_abs_king=true".to_string(),
                });
            }

            try_for_each_state_in_abs_box(&scn.rules.layout, *bound, *allow_captures, |s| {
                if !scn.rules.is_legal_position(&s.pos) {
                    return Ok(());
                }
                if !scn.laws.allow_state(&s) {
                    return Ok(());
                }
                if !scn.domain.inside(&s) {
                    return Ok(());
                }
                if scn.remove_stalemates && is_stalemate_with_laws(scn, &scn.laws, &s, tracker)? {
                    return Ok(());
                }

                if trap.insert(s) {
                    tracker.bump_states("candidates_in_abs_box", 1)?;
                }
                Ok(())
            })?;
        }

        CandidateGeneration::FromStates { states } => {
            for s0 in states.iter() {
                if !scn.track_abs_king && s0.abs_king != Coord::ORIGIN {
                    return Err(SearchError::InvalidScenario {
                        reason: "track_abs_king=false requires abs_king==ORIGIN for FromStates"
                            .to_string(),
                    });
                }

                let mut pos = s0.pos.clone();
                pos.canonicalize(&scn.rules.layout);
                if !scn.rules.is_legal_position(&pos) {
                    continue;
                }

                let s = State {
                    abs_king: s0.abs_king,
                    pos,
                };

                if !scn.laws.allow_state(&s) {
                    continue;
                }
                if !scn.domain.inside(&s) {
                    continue;
                }
                if scn.remove_stalemates && is_stalemate_with_laws(scn, &scn.laws, &s, tracker)? {
                    continue;
                }

                if trap.insert(s) {
                    tracker.bump_states("candidates_from_states", 1)?;
                }
            }
        }

        CandidateGeneration::ReachableFromStart { max_queue } => {
            if *max_queue == 0 {
                return Err(SearchError::InvalidScenario {
                    reason: "ReachableFromStart requires max_queue > 0".to_string(),
                });
            }

            let mut q: VecDeque<State> = VecDeque::new();

            match scn.start.to_move {
                Side::Black => {
                    try_add_reachable_b(
                        scn,
                        tracker,
                        *max_queue,
                        &mut trap,
                        &mut q,
                        scn.start.state.clone(),
                    )?;
                }
                Side::White => {
                    let moves = legal_white_moves(scn, &scn.laws, &scn.start.state, tracker)?;
                    for b in moves {
                        try_add_reachable_b(scn, tracker, *max_queue, &mut trap, &mut q, b)?;
                    }
                }
            }

            while let Some(b) = q.pop_front() {
                tracker.bump_steps("candidates_reachable_scan", 1)?;

                let after_black = legal_black_moves(scn, &scn.laws, &b, tracker)?;
                for w in after_black {
                    let replies = legal_white_moves(scn, &scn.laws, &w, tracker)?;
                    for b2 in replies {
                        try_add_reachable_b(scn, tracker, *max_queue, &mut trap, &mut q, b2)?;
                    }
                }
            }
        }
    }

    Ok(trap)
}

fn try_add_reachable_b<D, L, P>(
    scn: &Scenario<D, L, P>,
    tracker: &mut ResourceTracker,
    max_queue: usize,
    trap: &mut FxHashSet<State>,
    q: &mut VecDeque<State>,
    b: State,
) -> Result<(), SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    if !scn.laws.allow_state(&b) {
        return Ok(());
    }
    if !scn.domain.inside(&b) {
        return Ok(());
    }
    if scn.remove_stalemates && is_stalemate_with_laws(scn, &scn.laws, &b, tracker)? {
        return Ok(());
    }

    if trap.insert(b.clone()) {
        tracker.bump_states("candidates_reachable", 1)?;
        if q.len() >= max_queue {
            return Err(SearchError::LimitExceeded {
                stage: "candidates_reachable",
                metric: "queue",
                limit: max_queue as u64,
                observed: (q.len() + 1) as u64,
                counts: tracker.counts(),
            });
        }
        q.push_back(b);
    }

    Ok(())
}
