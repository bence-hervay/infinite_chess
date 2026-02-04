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
use infinite_chess::search::trap::maximal_inescapable_trap;

fn captured_start(layout: &PieceLayout) -> Position {
    let squares = [Square::NONE; MAX_PIECES];
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(layout);
    pos
}

#[test]
fn abs_box_no_pieces_trap_is_empty() {
    let bound = 2;
    let layout = PieceLayout::from_counts(false, 0, 0, 0, 0);
    let rules = Rules::new(layout.clone(), 1);

    let scn = Scenario {
        name: "abs_box_no_pieces_enum",
        rules,
        white_can_pass: false,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, captured_start(&layout)),
        },
        candidates: CandidateGeneration::InBox {
            bound,
            allow_captures: true,
        },
        domain: BuiltinDomain::Box { bound },
        laws: NoLaws,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert!(trap.is_empty());
}

#[test]
fn abs_box_single_rook_trap_is_empty() {
    let bound = 3;
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let rules = Rules::new(layout.clone(), 7);

    let scn = Scenario {
        name: "abs_box_one_rook_enum",
        rules,
        white_can_pass: true,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, captured_start(&layout)),
        },
        candidates: CandidateGeneration::InBox {
            bound,
            allow_captures: true,
        },
        domain: BuiltinDomain::Box { bound },
        laws: NoLaws,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert!(trap.is_empty());
}

#[test]
fn abs_box_no_pieces_trap_stays_empty_when_bound_grows() {
    let layout = PieceLayout::from_counts(false, 0, 0, 0, 0);
    let rules1 = Rules::new(layout.clone(), 1);
    let rules2 = rules1.clone();

    let mk = |bound: i32, rules: Rules| -> Scenario<BuiltinDomain, NoLaws, NoPreferences> {
        Scenario {
            name: "abs_box_no_pieces_mono",
            rules,
            white_can_pass: false,
            track_abs_king: true,
            start: StartState {
                to_move: Side::Black,
                state: State::new(Coord::ORIGIN, captured_start(&layout)),
            },
            candidates: CandidateGeneration::InBox {
                bound,
                allow_captures: true,
            },
            domain: BuiltinDomain::Box { bound },
            laws: NoLaws,
            preferences: NoPreferences,
            limits: ResourceLimits::default(),
            cache_mode: CacheMode::None,
            remove_stalemates: false,
        }
    };

    assert!(maximal_inescapable_trap(&mk(1, rules1)).unwrap().is_empty());
    assert!(maximal_inescapable_trap(&mk(2, rules2)).unwrap().is_empty());
}
