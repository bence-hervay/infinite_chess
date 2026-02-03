//! Built-in scenarios (compile-time configs).

pub mod nbb;

use crate::chess::layout::PieceLayout;
use crate::chess::rules::Rules;
use crate::core::coord::Coord;
use crate::core::position::{Position, MAX_PIECES};
use crate::core::square::Square;
use crate::scenario::{
    AllDomain, CacheMode, CandidateGeneration, NoLaws, NoPreferences, ResourceLimits, Scenario,
    SearchError, Side, StartState, State,
};

pub type BuiltInScenario = Scenario<AllDomain, NoLaws, NoPreferences>;

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

/// 3 rooks, bound=2, move_bound=1.
///
/// This is small enough to be used in tests and fast demos.
pub fn three_rooks_bound2_mb1() -> BuiltInScenario {
    let layout = PieceLayout::from_counts(false, 0, 3, 0, 0);
    let rules = Rules::new(layout.clone(), 1);
    let start_pos = pos_from_coords(
        &layout,
        &[Coord::new(2, 2), Coord::new(-2, 2), Coord::new(2, -2)],
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
        domain: AllDomain,
        laws: NoLaws,
        preferences: NoPreferences,
        limits: demo_limits(),
        cache_mode: CacheMode::BothBounded,
        remove_stalemates: true,
    }
}

/// 2 rooks, bound=7 (used for the "no checkmates" known result).
pub fn two_rooks_bound7() -> BuiltInScenario {
    let layout = PieceLayout::from_counts(false, 0, 2, 0, 0);
    let rules = Rules::new(layout.clone(), 7);
    let start_pos = pos_from_coords(&layout, &[Coord::new(1, 3), Coord::new(-2, -5)]);

    Scenario {
        name: "two_rooks_bound7",
        rules,
        white_can_pass: true,
        track_abs_king: false,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, start_pos),
        },
        candidates: CandidateGeneration::InLinfBound {
            bound: 7,
            allow_captures: true,
        },
        domain: AllDomain,
        laws: NoLaws,
        preferences: NoPreferences,
        limits: demo_limits(),
        cache_mode: CacheMode::BothBounded,
        remove_stalemates: true,
    }
}

pub fn nbb20_from_file() -> Result<BuiltInScenario, SearchError> {
    nbb::nbb20_from_file()
}

/// Return a config by name.
pub fn by_name(name: &str) -> Result<Option<BuiltInScenario>, SearchError> {
    match name {
        "three_rooks_bound2_mb1" => Ok(Some(three_rooks_bound2_mb1())),
        "two_rooks_bound7" => Ok(Some(two_rooks_bound7())),
        "nbb20_from_file" => Ok(Some(nbb20_from_file()?)),
        _ => Ok(None),
    }
}

/// Names of all built-in scenarios.
pub fn names() -> &'static [&'static str] {
    &[
        "three_rooks_bound2_mb1",
        "two_rooks_bound7",
        "nbb20_from_file",
    ]
}

pub fn available_names() -> &'static [&'static str] {
    names()
}
