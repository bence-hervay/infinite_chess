//! Scenario layer: glue between pure rules and game-search objectives.
//!
//! A [`Scenario`] bundles:
//! - pure chess [`Rules`]
//! - scenario-specific restrictions (**laws**) via [`LawsLike`]
//! - the modeled “inside” set (**domain**) via [`DomainLike`]
//! - optional tie-breakers (**preferences**) via [`PreferencesLike`]
//! - explicit budgets via [`ResourceLimits`]
//!
//! This separation keeps the core rules reusable and makes the semantics of “trap vs boundary”
//! explicit: leaving the domain is *allowed*, but it may count as escape depending on objective.

use std::fmt;

use crate::chess::bounds::is_in_bound;
use crate::chess::rules::Rules;
use crate::core::coord::Coord;
use crate::core::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Side to move in a game state.
pub enum Side {
    Black,
    White,
}

/// A game state for "white pieces vs lone black king".
///
/// - `pos` is stored in king-relative coordinates (black king at origin).
/// - `abs_king` is an optional absolute anchor used only when a scenario needs absolute constraints.
///
/// When `Scenario.track_abs_king == false`, this value must be [`Coord::ORIGIN`], and black moves
/// keep it unchanged (translation-reduced state space).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    /// Absolute black king coordinate (only meaningful if `track_abs_king=true`).
    pub abs_king: Coord,
    /// White piece placement in king-relative coordinates.
    pub pos: Position,
}

impl State {
    #[inline]
    pub fn new(abs_king: Coord, pos: Position) -> Self {
        Self { abs_king, pos }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A required scenario start state (no objective can run without it).
pub struct StartState {
    pub to_move: Side,
    pub state: State,
}

#[derive(Debug, Clone, Copy)]
/// Search budgets used to bound memory/time consumption.
///
/// These are not exact byte limits, but correlate strongly with allocation size:
/// - `max_states`: number of states admitted to candidate sets / graphs
/// - `max_edges`: number of generated moves/edges
/// - cache limits: number of cached entries and total cached moves
/// - `max_runtime_steps`: generic loop-iteration guard
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
/// Running counters tracked during a search.
pub struct ResourceCounts {
    pub states: u64,
    pub edges: u64,
    pub cache_entries: u64,
    pub cached_moves: u64,
    pub runtime_steps: u64,
}

#[derive(Debug)]
/// Structured errors returned by search routines.
pub enum SearchError {
    /// The scenario is internally inconsistent (e.g. invalid start).
    InvalidScenario { reason: String },
    /// A configured resource limit was exceeded.
    LimitExceeded {
        stage: &'static str,
        metric: &'static str,
        limit: u64,
        observed: u64,
        counts: ResourceCounts,
    },
    /// A `try_reserve` allocation failed for a large structure.
    AllocationFailed {
        stage: &'static str,
        structure: &'static str,
        counts: ResourceCounts,
    },
    /// I/O failure (used by data-backed scenarios).
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

/// Domain membership predicate (“inside vs outside”).
///
/// Domain is not legality: moves may leave the domain. Objectives interpret leaving as escape/win
/// conditions depending on the solver.
pub trait DomainLike {
    fn inside(&self, s: &State) -> bool;
}

/// Scenario-specific legality filters.
///
/// These are applied after pure move generation:
/// - `allow_state` can reject states (e.g. forbid overlaps beyond pure rules, clamp to a region)
/// - `allow_black_move` / `allow_white_move` can reject specific transitions
/// - `allow_pass` controls whether a pass is allowed at a given state (used by tempo trap)
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
    /// Return an ordering (indices into `moves`) to be used when choosing a black move for demos.
    fn rank_black_moves(&self, from: &State, moves: &[State]) -> Vec<usize>;
    /// Return an ordering (indices into `moves`) to be used when choosing a white move for demos.
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
/// How to build the candidate set for trap search.
pub enum CandidateGeneration {
    /// Enumerate all canonical placements within an L∞ bound (relative coordinates).
    InLinfBound { bound: i32, allow_captures: bool },
    /// Use an explicitly provided list of candidate states (e.g. file-backed or geometry-backed).
    FromStates { states: Vec<State> },
    /// Explore states reachable from the required `start` (often much smaller than enumeration).
    ReachableFromStart { max_queue: usize },
}

#[derive(Debug, Clone, Copy)]
/// Move-caching policy for search routines.
pub enum CacheMode {
    /// No caching (lower memory, slower).
    None,
    /// Cache only black moves.
    BlackOnly,
    /// Cache both black and white moves (bounded by [`ResourceLimits`]).
    BothBounded,
}

#[derive(Debug, Clone)]
/// A fully specified search configuration.
///
/// `Scenario::validate()` checks invariants such as start legality and candidate compatibility.
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
    /// Validate scenario invariants. Intended to be called by CLIs/tests before running solvers.
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
