//! Utilities for evaluating bounded-universe scenarios (AbsBox).
//!
//! This is primarily intended for parity / cross-check harnesses:
//! - enumerate the finite universe,
//! - count in-universe vs escaping moves,
//! - run trap / tempo / forced-mate solvers.

use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::core::coord::Coord;
use crate::scenario::{CandidateGeneration, DomainLike, LawsLike, Scenario, SearchError, State};
use crate::search::forced_mate::forced_mate_bounded;
use crate::search::movegen::{legal_black_moves, legal_white_moves};
use crate::search::resources::ResourceTracker;
use crate::search::trap::{maximal_inescapable_trap, maximal_tempo_trap};
use crate::search::universe::try_for_each_state_in_abs_box;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedCounts {
    pub universe_states: usize,
    pub black_moves_in: u64,
    pub black_moves_escape: u64,
    pub white_moves_in: u64,
    pub white_moves_escape: u64,
    #[serde(rename = "checkmates")]
    pub checkmates_in_universe: usize,
    pub trap: usize,
    pub tempo: usize,
    pub mate: usize,
}

/// Compute a bundle of bounded-universe metrics.
///
/// Requires `CandidateGeneration::InBox`.
pub fn compute_bounded_counts<D, L, P>(
    scn: &Scenario<D, L, P>,
) -> Result<BoundedCounts, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    scn.validate()?;

    let (bound, allow_captures) = match scn.candidates {
        CandidateGeneration::InBox {
            bound,
            allow_captures,
        } => (bound, allow_captures),
        _ => {
            return Err(SearchError::InvalidScenario {
                reason: "compute_bounded_counts requires candidates=InBox".to_string(),
            })
        }
    };

    let mut tracker = ResourceTracker::new(scn.limits);

    // Universe placements.
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
            tracker.bump_states("bounded_universe", 1)?;
        }
        Ok(())
    })?;

    // Move counts + checkmates (infinite-board legality).
    let mut black_in: u64 = 0;
    let mut black_escape: u64 = 0;
    let mut white_in: u64 = 0;
    let mut white_escape: u64 = 0;
    let mut mates: usize = 0;

    for s in universe.iter() {
        tracker.bump_steps("bounded_scan", 1)?;

        if scn.rules.is_checkmate(&s.pos) {
            mates += 1;
        }

        for to in legal_black_moves(scn, &scn.laws, s, &mut tracker)? {
            if universe.contains(&to) {
                black_in += 1;
            } else {
                black_escape += 1;
            }
        }

        for to in legal_white_moves(scn, &scn.laws, s, &mut tracker)? {
            if universe.contains(&to) {
                white_in += 1;
            } else {
                white_escape += 1;
            }
        }
    }

    // Trap / tempo.
    let trap_set = maximal_inescapable_trap(scn)?;
    let tempo_set = maximal_tempo_trap(scn, &trap_set)?;

    // Forced mate region (bounded-universe interpretation).
    // Passing is controlled by the scenario (typically disabled for mate).
    let mate_region = forced_mate_bounded(scn, false)?;

    Ok(BoundedCounts {
        universe_states: universe.len(),
        black_moves_in: black_in,
        black_moves_escape: black_escape,
        white_moves_in: white_in,
        white_moves_escape: white_escape,
        checkmates_in_universe: mates,
        trap: trap_set.len(),
        tempo: tempo_set.len(),
        mate: mate_region.winning_btm.len(),
    })
}

/// Helper: compute the absolute square of a relative square in a state.
#[inline]
pub fn abs_square(s: &State, rel: Coord) -> Coord {
    s.abs_king + rel
}
