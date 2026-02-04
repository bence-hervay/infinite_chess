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

#[test]
fn king_cannot_be_trapped_with_no_white_pieces() {
    let layout = PieceLayout::from_counts(false, 0, 0, 0, 0);
    let rules = Rules::new(layout, 1);

    let squares = [Square::NONE; MAX_PIECES];
    let pos = Position::new(0, squares);
    let start = State::new(Coord::ORIGIN, pos);

    let scn = Scenario {
        name: "no_pieces_abs_box",
        rules,
        white_can_pass: false,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: start.clone(),
        },
        candidates: CandidateGeneration::ReachableFromStart { max_queue: 10_000 },
        domain: BuiltinDomain::AbsBox { bound: 2 },
        laws: NoLaws,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert!(trap.is_empty());
    assert!(!trap.contains(&start));
}

#[test]
fn king_cannot_be_trapped_with_a_single_rook() {
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let rules = Rules::new(layout.clone(), 7);

    let mut squares = [Square::NONE; MAX_PIECES];
    squares[0] = Square::from_coord(Coord::new(2, 0));
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(&layout);

    let start = State::new(Coord::ORIGIN, pos);

    let scn = Scenario {
        name: "one_rook_abs_box",
        rules,
        white_can_pass: true,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: start.clone(),
        },
        candidates: CandidateGeneration::ReachableFromStart { max_queue: 200_000 },
        domain: BuiltinDomain::AbsBox { bound: 4 },
        laws: NoLaws,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert!(trap.is_empty());
    assert!(!trap.contains(&start));
}
