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
use infinite_chess::search::bounded::compute_bounded_counts;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PieceCounts {
    white_king: bool,
    queens: usize,
    rooks: usize,
    bishops: usize,
    knights: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct InputFile {
    scenario: ScenarioSpec,
}

fn captured_start(layout: &PieceLayout) -> Position {
    let squares = [Square::NONE; MAX_PIECES];
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(layout);
    pos
}

fn build_scenario(
    spec: &ScenarioSpec,
) -> Result<Scenario<BuiltinDomain, NoLaws, NoPreferences>, String> {
    if spec.bound < 0 {
        return Err("bound must be >= 0".to_string());
    }
    if spec.move_bound < 1 {
        return Err("move_bound must be >= 1".to_string());
    }

    let layout = PieceLayout::from_counts(
        spec.pieces.white_king,
        spec.pieces.queens,
        spec.pieces.rooks,
        spec.pieces.bishops,
        spec.pieces.knights,
    );

    let effective_move_bound = match spec.move_bound_mode {
        MoveBoundMode::Inclusive => spec.move_bound,
        MoveBoundMode::Exclusive => {
            if spec.move_bound < 2 {
                return Err("move_bound_mode=exclusive requires move_bound >= 2".to_string());
            }
            spec.move_bound - 1
        }
    };

    let rules = Rules::new(layout.clone(), effective_move_bound);
    let bound = spec.bound;

    Ok(Scenario {
        name: "bounded_eval",
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
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: bounded_eval <scenario.json>");
        std::process::exit(2);
    }

    let path = PathBuf::from(&args[1]);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read {}: {e}", path.display());
            std::process::exit(1);
        }
    };

    let input: InputFile = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Invalid JSON in {}: {e}", path.display());
            std::process::exit(2);
        }
    };

    let scn = match build_scenario(&input.scenario) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Invalid scenario spec: {e}");
            std::process::exit(2);
        }
    };

    let counts = match compute_bounded_counts(&scn) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Evaluation failed: {e}");
            std::process::exit(1);
        }
    };

    let out = serde_json::json!({
        "scenario": input.scenario,
        "counts": counts,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
