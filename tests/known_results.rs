use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
use infinite_chess::core::coord::Coord;
use infinite_chess::scenario::State;
use infinite_chess::scenarios;
use infinite_chess::search::mates::count_checkmates_in_bound;
use infinite_chess::search::trap::{maximal_inescapable_trap, maximal_tempo_trap};

#[test]
fn three_rooks_has_48_checkmates_in_linf_bound2() {
    let layout = PieceLayout::from_counts(false, 0, 3, 0, 0);
    let rules = Rules::new(layout, 1);
    let count = count_checkmates_in_bound(&rules, 2);
    assert_eq!(count, 48);
}

#[test]
fn two_rooks_has_no_checkmate_even_in_linf_bound7() {
    let layout = PieceLayout::from_counts(false, 0, 2, 0, 0);
    let rules = Rules::new(layout, 1);
    let count = count_checkmates_in_bound(&rules, 7);
    assert_eq!(count, 0);
}

#[test]
fn inescapable_trap_size_for_three_rooks_bound2_mb1_is_169() {
    let scn = scenarios::three_rooks_bound2_mb1();

    let trap = maximal_inescapable_trap(&scn).unwrap();
    assert_eq!(trap.len(), 169);

    // Verify the defining property: from every position in the trap,
    // every legal black move has some white reply that stays in the trap.
    let rules = &scn.rules;
    for p in &trap {
        for after_black_pos in rules.black_moves(&p.pos) {
            let after_black = State::new(Coord::ORIGIN, after_black_pos);
            let replies = rules.white_moves(&after_black.pos, scn.white_can_pass);
            assert!(replies
                .into_iter()
                .map(|pos| State::new(Coord::ORIGIN, pos))
                .any(|r| trap.contains(&r)));
        }
    }
}

#[test]
fn tempo_trap_size_for_three_rooks_bound2_mb1_is_113_and_excludes_mates() {
    let scn = scenarios::three_rooks_bound2_mb1();

    let trap = maximal_inescapable_trap(&scn).unwrap();
    let tempo = maximal_tempo_trap(&scn, &trap).unwrap();

    assert_eq!(tempo.len(), 113);
    assert!(tempo.is_subset(&trap));

    // Tempo traps are about infinite play with "free passes" occurring infinitely often.
    // Immediate checkmates are terminal (no black move), so they are not part of the
    // Büchi winning set.
    for p in &tempo {
        assert!(!scn.rules.is_checkmate(&p.pos));
    }
}
