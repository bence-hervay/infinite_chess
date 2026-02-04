use std::path::PathBuf;

use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
use infinite_chess::core::coord::Coord;
use infinite_chess::core::position::{Position, MAX_PIECES};
use infinite_chess::core::square::Square;
use infinite_chess::scenario::{
    CacheMode, CandidateGeneration, NoLaws, NoPreferences, ResourceLimits, Scenario, Side,
    StartState, State,
};
use infinite_chess::scenarios::BuiltinDomain;
use infinite_chess::search::bounded::{compute_bounded_counts, BoundedCounts};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct PieceCounts {
    white_king: bool,
    queens: usize,
    rooks: usize,
    bishops: usize,
    knights: usize,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MoveBoundMode {
    Inclusive,
    Exclusive,
}

fn default_move_bound_mode() -> MoveBoundMode {
    MoveBoundMode::Inclusive
}

fn default_remove_stalemates() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
struct ScenarioSpec {
    bound: i32,
    move_bound: i32,
    #[serde(default = "default_move_bound_mode")]
    move_bound_mode: MoveBoundMode,
    pieces: PieceCounts,
    allow_captures: bool,
    white_can_pass: bool,
    #[serde(default = "default_remove_stalemates")]
    remove_stalemates: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct GoldenCase {
    scenario: ScenarioSpec,
    expected: BoundedCounts,
}

fn captured_start(layout: &PieceLayout) -> Position {
    let squares = [Square::NONE; MAX_PIECES];
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(layout);
    pos
}

fn build_scenario(spec: &ScenarioSpec) -> Scenario<BuiltinDomain, NoLaws, NoPreferences> {
    let layout = PieceLayout::from_counts(
        spec.pieces.white_king,
        spec.pieces.queens,
        spec.pieces.rooks,
        spec.pieces.bishops,
        spec.pieces.knights,
    );

    let effective_move_bound = match spec.move_bound_mode {
        MoveBoundMode::Inclusive => spec.move_bound,
        MoveBoundMode::Exclusive => spec.move_bound - 1,
    };

    let rules = Rules::new(layout.clone(), effective_move_bound);
    let bound = spec.bound;

    Scenario {
        name: "golden_abs_box",
        rules,
        white_can_pass: spec.white_can_pass,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, captured_start(&layout)),
        },
        candidates: CandidateGeneration::InBox {
            bound,
            allow_captures: spec.allow_captures,
        },
        domain: BuiltinDomain::Box { bound },
        laws: NoLaws,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::BothBounded,
        remove_stalemates: spec.remove_stalemates,
    }
}

#[test]
fn golden_abs_box_counts_match() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join("scenarios");

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("failed to read golden scenarios directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "json").unwrap_or(false))
        .collect();
    files.sort();

    assert!(!files.is_empty(), "no golden scenario JSONs found");

    for path in files {
        let bytes = std::fs::read(&path).expect("failed to read golden scenario file");
        let case: GoldenCase =
            serde_json::from_slice(&bytes).expect("failed to parse golden scenario JSON");

        let scn = build_scenario(&case.scenario);
        let observed = compute_bounded_counts(&scn).unwrap();

        assert_eq!(observed, case.expected, "mismatch for {}", path.display());
    }
}
