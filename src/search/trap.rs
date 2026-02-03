use rustc_hash::{FxHashMap, FxHashSet};

use crate::chess::bounds::enumerate_positions_in_bound;
use crate::chess::config::ScenarioConfig;
use crate::chess::rules::Rules;
use crate::core::position::Position;

/// Cache for move generation during trap pruning.
#[derive(Default)]
struct MoveCache {
    black: FxHashMap<Position, Vec<Position>>,
    white: FxHashMap<Position, Vec<Position>>,
}

impl MoveCache {
    fn black_moves(&mut self, rules: &Rules, p: &Position) -> Vec<Position> {
        if let Some(v) = self.black.get(p) {
            return v.clone();
        }
        let moves = rules.black_moves(p);
        self.black.insert(p.clone(), moves.clone());
        moves
    }

    fn white_moves(&mut self, rules: &Rules, p: &Position, white_can_pass: bool) -> Vec<Position> {
        // Keying only by `Position` is OK because `white_can_pass` is config-global.
        if let Some(v) = self.white.get(p) {
            return v.clone();
        }
        let moves = rules.white_moves(p, white_can_pass);
        self.white.insert(p.clone(), moves.clone());
        moves
    }
}

/// Compute the maximal inescapable trap inside the scenario's L∞ slice.
///
/// The returned set is a set of **black-to-move** positions within the bound.
pub fn maximal_inescapable_trap(cfg: &ScenarioConfig) -> FxHashSet<Position> {
    let rules = Rules::new(cfg.layout.clone(), cfg.move_bound);

    let mut trap: FxHashSet<Position> =
        enumerate_positions_in_bound(&rules.layout, cfg.bound, true)
            .into_iter()
            .filter(|p| rules.is_legal_position(p))
            .collect();

    if cfg.remove_stalemates {
        trap.retain(|p| !rules.is_stalemate(p));
    }

    let mut cache = MoveCache::default();

    loop {
        let mut to_remove: Vec<Position> = Vec::new();

        for p in trap.iter() {
            // If black has a move to a position from which every white reply exits the current set,
            // then `p` cannot be in an inescapable trap.
            let black_moves = cache.black_moves(&rules, p);

            let mut fails = false;
            for after_black in black_moves.iter() {
                let white_moves = cache.white_moves(&rules, after_black, cfg.white_can_pass);
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

    trap
}

/// Compute the maximal *tempo* trap inside an already-computed inescapable trap.
///
/// A tempo trap is the Büchi-winning region where White can:
/// 1) stay inside the inescapable trap forever, and
/// 2) force *infinitely many* visits to "passable" white-to-move positions.
///
/// "Passable" means: after Black's move, the resulting piece placement is itself
/// a black-to-move trap position, so White may pass and still remain inside the trap.
pub fn maximal_tempo_trap(
    cfg: &ScenarioConfig,
    inescapable: &FxHashSet<Position>,
) -> FxHashSet<Position> {
    let rules = Rules::new(cfg.layout.clone(), cfg.move_bound);

    crate::search::buchi::tempo_trap_buchi(&rules, inescapable, cfg.white_can_pass)
}
