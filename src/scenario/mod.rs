//! Scenario layer: glue between pure chess rules and game-search objectives.
//!
//! This module defines:
//! - `State`: a piece placement plus an optional absolute black-king anchor
//! - `Scenario`: a fully specified search configuration (rules + laws + domain + limits)
//! - Traits for `LawsLike`, `DomainLike`, and `PreferencesLike`

use std::fmt;

use crate::chess::bounds::is_in_bound;
use crate::chess::rules::Rules;
use crate::core::coord::Coord;
use crate::core::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Black,
    White,
}

/// A game state for "white pieces vs lone black king".
///
/// - `pos` is stored in king-relative coordinates (black king at origin).
/// - `abs_king` is an optional absolute anchor used only when a scenario needs absolute constraints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    pub abs_king: Coord,
    pub pos: Position,
}

impl State {
    #[inline]
    pub fn new(abs_king: Coord, pos: Position) -> Self {
        Self { abs_king, pos }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartState {
    pub to_move: Side,
    pub state: State,
}

#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    pub max_states: usize,
    pub max_edges: usize,
    pub max_cache_entries: usize,
    pub max_cached_moves: usize,
    pub max_runtime_steps: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_states: 2_000_000,
            max_edges: 50_000_000,
            max_cache_entries: 250_000,
            max_cached_moves: 15_000_000,
            max_runtime_steps: 200_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceCounts {
    pub states: u64,
    pub edges: u64,
    pub cache_entries: u64,
    pub cached_moves: u64,
    pub runtime_steps: u64,
}

#[derive(Debug)]
pub enum SearchError {
    InvalidScenario {
        reason: String,
    },
    LimitExceeded {
        stage: &'static str,
        metric: &'static str,
        limit: u64,
        observed: u64,
        counts: ResourceCounts,
    },
    AllocationFailed {
        stage: &'static str,
        structure: &'static str,
        counts: ResourceCounts,
    },
    Io {
        stage: &'static str,
        path: String,
        error: String,
    },
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchError::InvalidScenario { reason } => write!(f, "invalid scenario: {reason}"),
            SearchError::LimitExceeded {
                stage,
                metric,
                limit,
                observed,
                counts,
            } => write!(
                f,
                "limit exceeded at {stage}: {metric} (limit={limit}, observed={observed}); \
                 counts(states={}, edges={}, cache_entries={}, cached_moves={}, runtime_steps={})",
                counts.states,
                counts.edges,
                counts.cache_entries,
                counts.cached_moves,
                counts.runtime_steps
            ),
            SearchError::AllocationFailed {
                stage,
                structure,
                counts,
            } => write!(
                f,
                "allocation failed at {stage} for {structure}; \
                 counts(states={}, edges={}, cache_entries={}, cached_moves={}, runtime_steps={})",
                counts.states,
                counts.edges,
                counts.cache_entries,
                counts.cached_moves,
                counts.runtime_steps
            ),
            SearchError::Io { stage, path, error } => {
                write!(f, "io error at {stage} for {path}: {error}")
            }
        }
    }
}

impl std::error::Error for SearchError {}

pub trait DomainLike {
    fn inside(&self, s: &State) -> bool;
}

pub trait LawsLike {
    #[inline]
    fn allow_state(&self, _s: &State) -> bool {
        true
    }

    #[inline]
    fn allow_black_move(&self, _from: &State, _to: &State, _delta: Coord) -> bool {
        true
    }

    #[inline]
    fn allow_white_move(&self, _from: &State, _to: &State) -> bool {
        true
    }

    #[inline]
    fn allow_pass(&self, _s: &State) -> bool {
        true
    }
}

pub trait PreferencesLike {
    fn rank_black_moves(&self, from: &State, moves: &[State]) -> Vec<usize>;
    fn rank_white_moves(&self, from: &State, moves: &[State]) -> Vec<usize>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoLaws;
impl LawsLike for NoLaws {}

#[derive(Debug, Clone, Copy, Default)]
pub struct AllDomain;
impl DomainLike for AllDomain {
    #[inline]
    fn inside(&self, _s: &State) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoPreferences;
impl PreferencesLike for NoPreferences {
    fn rank_black_moves(&self, _from: &State, moves: &[State]) -> Vec<usize> {
        (0..moves.len()).collect()
    }

    fn rank_white_moves(&self, _from: &State, moves: &[State]) -> Vec<usize> {
        (0..moves.len()).collect()
    }
}

#[derive(Debug, Clone)]
pub enum CandidateGeneration {
    InLinfBound { bound: i32, allow_captures: bool },
    FromStates { states: Vec<State> },
    ReachableFromStart { max_queue: usize },
}

#[derive(Debug, Clone, Copy)]
pub enum CacheMode {
    None,
    BlackOnly,
    BothBounded,
}

#[derive(Debug, Clone)]
pub struct Scenario<D, L, P> {
    pub name: &'static str,
    pub rules: Rules,
    pub white_can_pass: bool,
    pub track_abs_king: bool,
    pub start: StartState,
    pub candidates: CandidateGeneration,
    pub domain: D,
    pub laws: L,
    pub preferences: P,
    pub limits: ResourceLimits,
    pub cache_mode: CacheMode,
    pub remove_stalemates: bool,
}

impl<D: DomainLike, L: LawsLike, P> Scenario<D, L, P> {
    pub fn validate(&self) -> Result<(), SearchError> {
        let s = &self.start.state;

        if !self.track_abs_king && s.abs_king != Coord::ORIGIN {
            return Err(SearchError::InvalidScenario {
                reason: "track_abs_king=false requires start.abs_king == ORIGIN".to_string(),
            });
        }

        if !self.rules.is_legal_position(&s.pos) {
            return Err(SearchError::InvalidScenario {
                reason: "start position is not legal under pure rules".to_string(),
            });
        }

        if !self.laws.allow_state(s) {
            return Err(SearchError::InvalidScenario {
                reason: "start state rejected by laws.allow_state".to_string(),
            });
        }

        if !self.domain.inside(s) {
            return Err(SearchError::InvalidScenario {
                reason: "start state is outside the domain".to_string(),
            });
        }

        if let CandidateGeneration::InLinfBound { bound, .. } = self.candidates {
            for &sq in s.pos.squares() {
                if !is_in_bound(sq, bound) {
                    return Err(SearchError::InvalidScenario {
                        reason: format!("start has a piece outside the L∞ bound {bound}"),
                    });
                }
            }
        }

        if self.remove_stalemates
            && self.start.to_move == Side::Black
            && self.is_stalemate_under_laws(s)
        {
            return Err(SearchError::InvalidScenario {
                reason: "start is a stalemate (and remove_stalemates=true)".to_string(),
            });
        }

        Ok(())
    }

    fn is_stalemate_under_laws(&self, s: &State) -> bool {
        if self.rules.is_attacked(Coord::ORIGIN, &s.pos) {
            return false;
        }

        for (delta, pos2) in self.rules.black_moves_with_delta(&s.pos) {
            let to = State {
                abs_king: if self.track_abs_king {
                    s.abs_king + delta
                } else {
                    s.abs_king
                },
                pos: pos2,
            };
            if self.laws.allow_black_move(s, &to, delta) && self.laws.allow_state(&to) {
                return false;
            }
        }

        true
    }
}
