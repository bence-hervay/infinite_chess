use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
use infinite_chess::core::coord::Coord;
use infinite_chess::core::position::{Position, MAX_PIECES};
use infinite_chess::core::square::Square;
use infinite_chess::scenario::{
    CacheMode, CandidateGeneration, DomainLike, LawsLike, NoPreferences, ResourceLimits, Scenario,
    Side, StartState, State,
};
use infinite_chess::search::trap::maximal_inescapable_trap;

#[derive(Debug, Clone, Copy)]
struct OnlyBlackDelta(Coord);

impl LawsLike for OnlyBlackDelta {
    fn allow_black_move(&self, _from: &State, _to: &State, delta: Coord) -> bool {
        delta == self.0
    }
}

#[derive(Debug, Clone, Copy)]
struct RookAt(Coord);

impl DomainLike for RookAt {
    fn inside(&self, s: &State) -> bool {
        s.pos.square(0) == Square::from_coord(self.0)
    }
}

fn rook_pos(c: Coord) -> Position {
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let mut squares = [Square::NONE; MAX_PIECES];
    squares[0] = Square::from_coord(c);
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(&layout);
    pos
}

#[test]
fn leaving_domain_is_allowed_if_white_can_return_immediately() {
    // Inside domain = rook at (2,1).
    // Black is forced to play delta=(1,0), which shifts the rook out to (1,1).
    // White can then move the rook back to (2,1) in one step.
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let rules = Rules::new(layout, 1);

    let inside = State::new(Coord::ORIGIN, rook_pos(Coord::new(2, 1)));
    let scn = Scenario {
        name: "domain_return_ok",
        rules,
        white_can_pass: false,
        track_abs_king: false,
        start: StartState {
            to_move: Side::Black,
            state: inside.clone(),
        },
        candidates: CandidateGeneration::FromStates {
            states: vec![inside.clone()],
        },
        domain: RookAt(Coord::new(2, 1)),
        laws: OnlyBlackDelta(Coord::new(1, 0)),
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert!(trap.contains(&inside));
}

#[test]
fn leaving_domain_is_escape_if_white_cannot_return() {
    // Inside domain = rook at (1,0).
    // Black is forced to play delta=(1,0), capturing the rook.
    // White cannot return to the inside state.
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let rules = Rules::new(layout, 1);

    let inside = State::new(Coord::ORIGIN, rook_pos(Coord::new(1, 0)));
    let scn = Scenario {
        name: "domain_escape",
        rules,
        white_can_pass: false,
        track_abs_king: false,
        start: StartState {
            to_move: Side::Black,
            state: inside.clone(),
        },
        candidates: CandidateGeneration::FromStates {
            states: vec![inside.clone()],
        },
        domain: RookAt(Coord::new(1, 0)),
        laws: OnlyBlackDelta(Coord::new(1, 0)),
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert!(!trap.contains(&inside));
    assert!(trap.is_empty());
}
