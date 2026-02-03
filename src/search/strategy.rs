//! Strategy extraction helpers.
//!
//! Search routines compute *sets* (winning regions / trap sets). For demos or interactive play you
//! often want a concrete “what should White do here?” choice.
//!
//! The helpers here extract a memoryless strategy *after* correctness-critical computation.
//! Preferences are used only as tie-breakers and do not affect trap set membership.

use rustc_hash::FxHashMap;

use crate::scenario::{DomainLike, LawsLike, PreferencesLike, Scenario, SearchError, State};
use crate::search::movegen::{legal_black_moves, legal_white_moves};
use crate::search::resources::ResourceTracker;

/// Extract a memoryless "stay in trap" strategy for White.
///
/// Returns a map from white-to-move nodes (states that arise after a black move) to a
/// chosen black-to-move successor that stays inside `btm_trap`.
///
/// Preferences are only used to break ties among multiple staying replies.
pub fn extract_white_stay_strategy<D, L, P>(
    scn: &Scenario<D, L, P>,
    btm_trap: &rustc_hash::FxHashSet<State>,
) -> Result<FxHashMap<State, State>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
    P: PreferencesLike,
{
    let mut tracker = ResourceTracker::new(scn.limits);
    let mut out: FxHashMap<State, State> = FxHashMap::default();

    for b in btm_trap.iter() {
        tracker.bump_steps("strategy_extract", 1)?;

        for w in legal_black_moves(scn, &scn.laws, b, &mut tracker)? {
            if out.contains_key(&w) {
                continue;
            }

            let replies = legal_white_moves(scn, &scn.laws, &w, &mut tracker)?;
            let mut stay: Vec<State> = replies
                .into_iter()
                .filter(|r| btm_trap.contains(r))
                .collect();
            if stay.is_empty() {
                continue;
            }

            let ranking = scn.preferences.rank_white_moves(&w, &stay);
            let choice = ranking
                .into_iter()
                .find_map(|idx| stay.get(idx).cloned())
                .unwrap_or_else(|| stay.swap_remove(0));

            out.insert(w, choice);
        }
    }

    Ok(out)
}
