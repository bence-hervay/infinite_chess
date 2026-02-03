use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
use infinite_chess::core::coord::{Coord, KING_STEPS};
use infinite_chess::core::position::{Position, MAX_PIECES};
use infinite_chess::core::square::Square;
use infinite_chess::scenario::{
    AllDomain, CacheMode, CandidateGeneration, NoLaws, NoPreferences, ResourceLimits, Scenario,
    Side, StartState, State,
};
use infinite_chess::search::movegen::{is_checkmate_with_laws, legal_black_moves};
use infinite_chess::search::resources::ResourceTracker;

#[test]
fn mate_logic_does_not_treat_enumeration_bound_as_a_wall() {
    // Construct a position inside bound=2 where Black is in check and the only legal king escape
    // shifts some pieces outside the bound. Mate logic must still see that escape as legal.
    //
    // Pieces:
    // - Rook at (0,2) gives check along the file.
    // - Bishops arranged to attack every king move except (1,0).
    // - Additional bishops sit at x=-2 so that after delta=(1,0) they shift to x=-3 (outside bound=2).
    let bound = 2;
    let layout = PieceLayout::from_counts(false, 0, 1, 3, 0); // R B B B
    let rules = Rules::new(layout.clone(), 1);

    let mut squares = [Square::NONE; MAX_PIECES];
    squares[0] = Square::from_coord(Coord::new(0, 2)); // rook
    squares[1] = Square::from_coord(Coord::new(-2, 2));
    squares[2] = Square::from_coord(Coord::new(-2, -2));
    squares[3] = Square::from_coord(Coord::new(-2, 1));

    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(&layout);
    let state = State::new(Coord::ORIGIN, pos);

    assert!(rules.is_attacked(Coord::ORIGIN, &state.pos));

    let scn = Scenario {
        name: "mate_nonwall",
        rules,
        white_can_pass: false,
        track_abs_king: false,
        start: StartState {
            to_move: Side::Black,
            state: state.clone(),
        },
        candidates: CandidateGeneration::InLinfBound {
            bound,
            allow_captures: false,
        },
        domain: AllDomain,
        laws: NoLaws,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let mut tracker = ResourceTracker::new(scn.limits);
    assert!(!is_checkmate_with_laws(&scn, &scn.laws, &state, &mut tracker).unwrap());

    let moves = legal_black_moves(&scn, &scn.laws, &state, &mut tracker).unwrap();
    assert!(!moves.is_empty());

    for m in moves {
        // The escape move must be allowed even if it shifts pieces outside the enumeration bound.
        let any_outside = m
            .pos
            .squares()
            .iter()
            .any(|&sq| !sq.is_none() && !sq.coord().in_linf_bound(bound));
        assert!(any_outside);
    }

    // Sanity: the intended escape is delta=(1,0).
    assert!(KING_STEPS.contains(&Coord::new(1, 0)));
}
