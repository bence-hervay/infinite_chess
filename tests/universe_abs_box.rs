use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::core::coord::Coord;
use infinite_chess::core::position::{Position, MAX_PIECES};
use infinite_chess::core::square::Square;
use infinite_chess::scenario::State;
use infinite_chess::search::universe::for_each_state_in_abs_box;
use rustc_hash::FxHashSet;

#[test]
fn universe_size_sanity_no_pieces() {
    let layout = PieceLayout::from_counts(false, 0, 0, 0, 0);
    let bound = 2;

    let mut count = 0usize;
    for_each_state_in_abs_box(&layout, bound, true, |_| count += 1);

    let side = (2 * bound + 1) as usize;
    assert_eq!(count, side * side);
}

#[test]
fn enumerated_states_respect_abs_box_membership() {
    let layout = PieceLayout::from_counts(true, 0, 1, 1, 1); // K R B N
    let bound = 1;

    for_each_state_in_abs_box(&layout, bound, true, |s| {
        assert!(s.abs_king.in_linf_bound(bound));
        for (_, sq) in s.pos.iter_present() {
            assert_ne!(sq.coord(), Coord::ORIGIN);
            let abs = s.abs_king + sq.coord();
            assert!(abs.in_linf_bound(bound));
        }
    });
}

#[test]
fn king_on_boundary_has_out_of_universe_moves() {
    let layout = PieceLayout::from_counts(false, 0, 0, 0, 0);
    let bound = 1;

    let mut universe: FxHashSet<State> = FxHashSet::default();
    for_each_state_in_abs_box(&layout, bound, true, |s| {
        universe.insert(s);
    });

    let pos = Position::new(0, [Square::NONE; MAX_PIECES]);
    let corner = State::new(Coord::new(bound, bound), pos);

    let mut saw_in = false;
    let mut saw_out = false;
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let to = State::new(corner.abs_king + Coord::new(dx, dy), corner.pos.clone());
            if universe.contains(&to) {
                saw_in = true;
            } else {
                saw_out = true;
            }
        }
    }

    assert!(saw_in);
    assert!(saw_out);
}
