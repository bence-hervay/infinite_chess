use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
use infinite_chess::core::coord::Coord;
use infinite_chess::core::position::{Position, MAX_PIECES};
use infinite_chess::core::square::Square;
use infinite_chess::scenario::{
    CacheMode, CandidateGeneration, LawsLike, NoPreferences, ResourceLimits, Scenario, Side,
    StartState, State,
};
use infinite_chess::scenarios::BuiltinDomain;
use infinite_chess::search::trap::{maximal_inescapable_trap, maximal_tempo_trap};

#[derive(Debug, Clone, Copy)]
struct KeepKingInAbsBox {
    bound: i32,
}

impl LawsLike for KeepKingInAbsBox {
    fn allow_black_move(&self, _from: &State, to: &State, _delta: Coord) -> bool {
        to.abs_king.in_linf_bound(self.bound)
    }
}

fn captured_start(layout: &PieceLayout) -> Position {
    let squares = [Square::NONE; MAX_PIECES];
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(layout);
    pos
}

#[test]
fn tempo_is_subset_of_trap() {
    // Synthetic toy: no pieces, black cannot leave the box, white can always pass.
    // Every white node is accepting, so tempo == trap, hence tempo ⊆ trap.
    let bound = 1;
    let layout = PieceLayout::from_counts(false, 0, 0, 0, 0);
    let rules = Rules::new(layout.clone(), 1);

    let scn = Scenario {
        name: "tempo_subset_abs_box_toy",
        rules,
        white_can_pass: true,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, captured_start(&layout)),
        },
        candidates: CandidateGeneration::InAbsBox {
            bound,
            allow_captures: true,
        },
        domain: BuiltinDomain::AbsBox { bound },
        laws: KeepKingInAbsBox { bound },
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    let tempo = maximal_tempo_trap(&scn, &trap).unwrap();
    assert!(tempo.is_subset(&trap));
}

#[test]
fn pass_disabled_makes_tempo_empty() {
    // Acceptance requires pass; with pass disabled, the Büchi winning region is empty.
    let bound = 2;
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0); // R
    let rules = Rules::new(layout.clone(), 2);

    let scn = Scenario {
        name: "tempo_pass_disabled_abs_box_toy",
        rules,
        white_can_pass: false,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, captured_start(&layout)),
        },
        candidates: CandidateGeneration::InAbsBox {
            bound,
            allow_captures: true,
        },
        domain: BuiltinDomain::AbsBox { bound },
        laws: KeepKingInAbsBox { bound },
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert!(!trap.is_empty(), "toy should have a non-empty trap");

    let tempo = maximal_tempo_trap(&scn, &trap).unwrap();
    assert!(tempo.is_empty());
}

#[test]
fn always_accepting_toy_has_tempo_equal_trap() {
    let bound = 1;
    let layout = PieceLayout::from_counts(false, 0, 0, 0, 0);
    let rules = Rules::new(layout.clone(), 1);

    let scn = Scenario {
        name: "tempo_equals_trap_abs_box_toy",
        rules,
        white_can_pass: true,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, captured_start(&layout)),
        },
        candidates: CandidateGeneration::InAbsBox {
            bound,
            allow_captures: true,
        },
        domain: BuiltinDomain::AbsBox { bound },
        laws: KeepKingInAbsBox { bound },
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    let tempo = maximal_tempo_trap(&scn, &trap).unwrap();
    assert_eq!(tempo.len(), trap.len());
    assert!(tempo.is_subset(&trap));
}
