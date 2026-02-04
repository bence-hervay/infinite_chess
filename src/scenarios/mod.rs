//! Built-in scenarios.
//!
//! Scenarios live in Rust code so they can bundle:
//! - piece layout + `move_bound` (pure rules)
//! - a required start state
//! - candidate generation strategy (enumeration / reachable exploration / file-backed sets)
//! - optional laws/domain/preferences and resource limits
//!
//! To add a new scenario:
//! 1) create a constructor function (e.g. `pub fn my_scenario() -> BuiltInScenario`)
//! 2) add it to [`by_name`] and [`names`]

pub mod nbb;

use crate::chess::layout::PieceLayout;
use crate::chess::rules::Rules;
use crate::core::coord::Coord;
use crate::core::position::{Position, MAX_PIECES};
use crate::core::square::Square;
use crate::scenario::{
    CacheMode, CandidateGeneration, DomainLike, NoLaws, NoPreferences, ResourceLimits, Scenario,
    SearchError, Side, StartState, State,
};

/// Built-in domains used by the built-in scenarios.
///
/// This is intentionally small and concrete (no "framework"):
/// - [`BuiltinDomain::All`] keeps the legacy behavior (purely translation-reduced search).
/// - [`BuiltinDomain::AbsBox`] anchors the state space by tracking an absolute king coordinate and
///   bounding both king and pieces to a finite box. This makes "walking off to infinity" observable
///   as leaving the domain.
#[derive(Debug, Clone, Copy)]
pub enum BuiltinDomain {
    All,
    AbsBox { bound: i32 },
}

impl DomainLike for BuiltinDomain {
    fn inside(&self, s: &State) -> bool {
        match *self {
            BuiltinDomain::All => true,
            BuiltinDomain::AbsBox { bound } => {
                if !s.abs_king.in_linf_bound(bound) {
                    return false;
                }
                for (_, sq) in s.pos.iter_present() {
                    let abs = s.abs_king + sq.coord();
                    if !abs.in_linf_bound(bound) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

pub type BuiltInScenario = Scenario<BuiltinDomain, NoLaws, NoPreferences>;

fn pos_from_coords(layout: &PieceLayout, coords: &[Coord]) -> Position {
    assert_eq!(layout.piece_count(), coords.len());
    let mut squares = [Square::NONE; MAX_PIECES];
    for (i, c) in coords.iter().copied().enumerate() {
        squares[i] = Square::from_coord(c);
    }
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(layout);
    pos
}

fn demo_limits() -> ResourceLimits {
    ResourceLimits {
        max_states: 1_000_000,
        max_edges: 25_000_000,
        max_cache_entries: 100_000,
        max_cached_moves: 5_000_000,
        max_runtime_steps: 50_000_000,
    }
}

fn two_rooks_limits() -> ResourceLimits {
    ResourceLimits {
        max_states: 2_000_000,
        max_edges: 2_000_000_000,
        max_cache_entries: 250_000,
        max_cached_moves: 15_000_000,
        max_runtime_steps: 2_000_000_000,
    }
}

/// 3 rooks, bound=2, move_bound=1.
///
/// This is small enough to be used in tests and fast demos.
pub fn three_rooks_bound2_mb1() -> BuiltInScenario {
    let layout = PieceLayout::from_counts(false, 0, 3, 0, 0);
    let rules = Rules::new(layout.clone(), 1);
    let start_pos = pos_from_coords(
        &layout,
        &[Coord::new(2, 2), Coord::new(2, 1), Coord::new(1, 2)],
    );

    Scenario {
        name: "three_rooks_bound2_mb1",
        rules,
        white_can_pass: true,
        track_abs_king: false,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, start_pos),
        },
        candidates: CandidateGeneration::InLinfBound {
            bound: 2,
            allow_captures: true,
        },
        domain: BuiltinDomain::All,
        laws: NoLaws,
        preferences: NoPreferences,
        limits: demo_limits(),
        cache_mode: CacheMode::BothBounded,
        remove_stalemates: true,
    }
}

/// 2 rooks, `move_bound=7`, with an absolute "board window" domain.
///
/// This scenario is anchored (`track_abs_king=true`) so "walk away forever" becomes observable as
/// leaving the domain.
pub fn two_rooks_bound7() -> BuiltInScenario {
    let layout = PieceLayout::from_counts(false, 0, 2, 0, 0);
    let rules = Rules::new(layout.clone(), 7);
    let start_pos = pos_from_coords(&layout, &[Coord::new(1, 3), Coord::new(-2, -5)]);

    Scenario {
        name: "two_rooks_bound7",
        rules,
        white_can_pass: true,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, start_pos),
        },
        candidates: CandidateGeneration::ReachableFromStart {
            max_queue: 2_000_000,
        },
        domain: BuiltinDomain::AbsBox { bound: 7 },
        laws: NoLaws,
        preferences: NoPreferences,
        limits: two_rooks_limits(),
        cache_mode: CacheMode::BothBounded,
        remove_stalemates: true,
    }
}

pub fn nbb20_from_file() -> Result<BuiltInScenario, SearchError> {
    nbb::nbb20_from_file()
}

pub fn nbb7_generated() -> Result<BuiltInScenario, SearchError> {
    nbb::nbb7_generated()
}

/// Return a config by name.
pub fn by_name(name: &str) -> Result<Option<BuiltInScenario>, SearchError> {
    match name {
        "three_rooks_bound2_mb1" => Ok(Some(three_rooks_bound2_mb1())),
        "two_rooks_bound7" => Ok(Some(two_rooks_bound7())),
        "nbb20_from_file" => Ok(Some(nbb20_from_file()?)),
        "nbb7_generated" => Ok(Some(nbb7_generated()?)),
        _ => Ok(None),
    }
}

/// Names of all built-in scenarios.
pub fn names() -> &'static [&'static str] {
    &[
        "three_rooks_bound2_mb1",
        "two_rooks_bound7",
        "nbb20_from_file",
        "nbb7_generated",
    ]
}

pub fn available_names() -> &'static [&'static str] {
    names()
}
