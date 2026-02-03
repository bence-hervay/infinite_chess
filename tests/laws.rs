use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
use infinite_chess::core::coord::Coord;
use infinite_chess::core::position::{Position, MAX_PIECES};
use infinite_chess::core::square::Square;
use infinite_chess::scenario::{
    AllDomain, CacheMode, CandidateGeneration, LawsLike, NoPreferences, ResourceLimits, Scenario,
    Side, StartState, State,
};
use infinite_chess::search::movegen::{legal_black_moves, legal_white_moves};
use infinite_chess::search::resources::ResourceTracker;

#[derive(Debug, Clone, Copy)]
struct NoCapturesLaws;

impl LawsLike for NoCapturesLaws {
    fn allow_black_move(&self, from: &State, _to: &State, delta: Coord) -> bool {
        let dst = Square::from_coord(delta);
        !from
            .pos
            .squares()
            .iter()
            .any(|&sq| !sq.is_none() && sq == dst)
    }
}

#[derive(Debug, Clone, Copy)]
struct PassIfAnyPieceOnPositiveX;

impl LawsLike for PassIfAnyPieceOnPositiveX {
    fn allow_pass(&self, s: &State) -> bool {
        s.pos.iter_present().any(|(_, sq)| sq.coord().x > 0)
    }
}

fn one_rook_pos(c: Coord) -> Position {
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let mut squares = [Square::NONE; MAX_PIECES];
    squares[0] = Square::from_coord(c);
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(&layout);
    pos
}

#[test]
fn law_can_forbid_black_capture() {
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let rules = Rules::new(layout, 1);
    let start = State::new(Coord::ORIGIN, one_rook_pos(Coord::new(1, 0)));

    let scn_allow = Scenario {
        name: "allow_capture",
        rules: rules.clone(),
        white_can_pass: true,
        track_abs_king: false,
        start: StartState {
            to_move: Side::Black,
            state: start.clone(),
        },
        candidates: CandidateGeneration::FromStates {
            states: vec![start.clone()],
        },
        domain: AllDomain,
        laws: infinite_chess::scenario::NoLaws,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let mut tracker = ResourceTracker::new(scn_allow.limits);
    let moves = legal_black_moves(&scn_allow, &scn_allow.laws, &start, &mut tracker).unwrap();
    assert!(moves.iter().any(|m| m.pos.square(0).is_none()));

    let scn_forbid = Scenario {
        name: "forbid_capture",
        rules: scn_allow.rules.clone(),
        white_can_pass: scn_allow.white_can_pass,
        track_abs_king: scn_allow.track_abs_king,
        start: scn_allow.start.clone(),
        candidates: scn_allow.candidates.clone(),
        domain: scn_allow.domain,
        laws: NoCapturesLaws,
        preferences: NoPreferences,
        limits: scn_allow.limits,
        cache_mode: scn_allow.cache_mode,
        remove_stalemates: scn_allow.remove_stalemates,
    };

    let mut tracker = ResourceTracker::new(scn_forbid.limits);
    let moves = legal_black_moves(&scn_forbid, &scn_forbid.laws, &start, &mut tracker).unwrap();
    assert!(moves.iter().all(|m| !m.pos.square(0).is_none()));
}

#[test]
fn pass_can_be_controlled_by_law_predicate() {
    let layout = PieceLayout::from_counts(false, 0, 1, 0, 0);
    let rules = Rules::new(layout, 1);

    let base = |state: State| Scenario {
        name: "pass_predicate",
        rules: rules.clone(),
        white_can_pass: true,
        track_abs_king: false,
        start: StartState {
            to_move: Side::White,
            state: state.clone(),
        },
        candidates: CandidateGeneration::FromStates {
            states: vec![state],
        },
        domain: AllDomain,
        laws: PassIfAnyPieceOnPositiveX,
        preferences: NoPreferences,
        limits: ResourceLimits::default(),
        cache_mode: CacheMode::None,
        remove_stalemates: false,
    };

    let s_pos = State::new(Coord::ORIGIN, one_rook_pos(Coord::new(1, 0)));
    let scn_pos = base(s_pos.clone());
    let mut tracker = ResourceTracker::new(scn_pos.limits);
    let moves = legal_white_moves(&scn_pos, &scn_pos.laws, &s_pos, &mut tracker).unwrap();
    assert!(moves.iter().any(|m| m == &s_pos));

    let s_neg = State::new(Coord::ORIGIN, one_rook_pos(Coord::new(-1, 0)));
    let scn_neg = base(s_neg.clone());
    let mut tracker = ResourceTracker::new(scn_neg.limits);
    let moves = legal_white_moves(&scn_neg, &scn_neg.laws, &s_neg, &mut tracker).unwrap();
    assert!(moves.iter().all(|m| m != &s_neg));
}
