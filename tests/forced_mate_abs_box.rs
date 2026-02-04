use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
use infinite_chess::core::coord::Coord;
use infinite_chess::core::position::{Position, MAX_PIECES};
use infinite_chess::core::square::Square;
use infinite_chess::scenario::{
    CacheMode, CandidateGeneration, DomainLike, NoLaws, NoPreferences, ResourceLimits, Scenario,
    Side, StartState, State,
};
use infinite_chess::scenarios::BuiltinDomain;
use infinite_chess::search::forced_mate::forced_mate_bounded;
use infinite_chess::search::movegen::{legal_black_moves, legal_white_moves};
use infinite_chess::search::resources::ResourceTracker;
use infinite_chess::search::universe::try_for_each_state_in_abs_box;
use rustc_hash::FxHashSet;

fn captured_start(layout: &PieceLayout) -> Position {
    let squares = [Square::NONE; MAX_PIECES];
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(layout);
    pos
}

fn abs_box_universe(
    scn: &Scenario<BuiltinDomain, NoLaws, NoPreferences>,
    bound: i32,
    allow_captures: bool,
) -> FxHashSet<State> {
    let mut out: FxHashSet<State> = FxHashSet::default();
    try_for_each_state_in_abs_box::<std::convert::Infallible>(
        &scn.rules.layout,
        bound,
        allow_captures,
        |s| {
            if !scn.rules.is_legal_position(&s.pos) {
                return Ok(());
            }
            if !scn.domain.inside(&s) {
                return Ok(());
            }
            out.insert(s);
            Ok(())
        },
    )
    .unwrap();
    out
}

#[test]
fn three_rooks_in_small_abs_box_has_some_forced_mates() {
    let bound = 2;
    let layout = PieceLayout::from_counts(false, 0, 3, 0, 0);
    let rules = Rules::new(layout.clone(), 1);

    let scn = Scenario {
        name: "mate_rrr_abs_box",
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

    let result = forced_mate_bounded(&scn, true).unwrap();
    assert!(!result.winning_btm.is_empty());

    let dtm = result.dtm.as_ref().expect("DTM requested");
    let mate_terminals: Vec<State> = dtm
        .iter()
        .filter_map(|(s, &d)| (d == 0).then(|| s.clone()))
        .collect();
    assert!(!mate_terminals.is_empty());

    // Soundness: dtm=0 implies checkmate (in check + no legal black move).
    let mut tracker = ResourceTracker::new(ResourceLimits::default());
    for s in mate_terminals.iter() {
        assert!(scn.rules.is_attacked(Coord::ORIGIN, &s.pos));
        assert!(legal_black_moves(&scn, &scn.laws, s, &mut tracker)
            .unwrap()
            .is_empty());
    }
}

#[test]
fn mate_winning_region_is_closed_under_optimal_replies() {
    let bound = 2;
    let layout = PieceLayout::from_counts(false, 0, 3, 0, 0);
    let rules = Rules::new(layout.clone(), 1);

    let scn = Scenario {
        name: "mate_rrr_abs_box_closure",
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

    let result = forced_mate_bounded(&scn, false).unwrap();
    let universe = abs_box_universe(&scn, bound, true);

    let mut tracker = ResourceTracker::new(ResourceLimits::default());

    for b in result.winning_btm.iter() {
        // No escape moves: every legal black move must stay inside the universe.
        for w in legal_black_moves(&scn, &scn.laws, b, &mut tracker).unwrap() {
            assert!(
                universe.contains(&w),
                "winning black node has an escape move"
            );

            // Winning white node: must have some in-universe reply back into the winning region.
            let replies = legal_white_moves(&scn, &scn.laws, &w, &mut tracker).unwrap();
            let has_reply = replies
                .into_iter()
                .filter(|b2| universe.contains(b2))
                .any(|b2| result.winning_btm.contains(&b2));
            assert!(has_reply, "missing winning reply from a white node");
        }
    }
}

#[test]
fn two_rooks_has_no_forced_mate_region_in_small_abs_box() {
    let bound = 2;
    let layout = PieceLayout::from_counts(false, 0, 2, 0, 0);
    let rules = Rules::new(layout.clone(), 1);

    let scn = Scenario {
        name: "mate_rr_abs_box",
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

    let result = forced_mate_bounded(&scn, false).unwrap();
    assert!(result.winning_btm.is_empty());
}
