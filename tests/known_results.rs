use infinite_chess::chess::config::ScenarioConfig;
use infinite_chess::chess::layout::PieceLayout;
use infinite_chess::chess::rules::Rules;
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
    let cfg = ScenarioConfig::new(
        "test_3rooks",
        2,
        1,
        true,
        true,
        PieceLayout::from_counts(false, 0, 3, 0, 0),
    );

    let trap = maximal_inescapable_trap(&cfg);
    assert_eq!(trap.len(), 169);

    // Verify the defining property: from every position in the trap,
    // every legal black move has some white reply that stays in the trap.
    let rules = cfg.rules();
    for p in &trap {
        for after_black in rules.black_moves(p) {
            let replies = rules.white_moves(&after_black, cfg.white_can_pass);
            assert!(replies.iter().any(|r| trap.contains(r)));
        }
    }
}

#[test]
fn tempo_trap_size_for_three_rooks_bound2_mb1_is_113_and_excludes_mates() {
    let cfg = ScenarioConfig::new(
        "test_3rooks",
        2,
        1,
        true,
        true,
        PieceLayout::from_counts(false, 0, 3, 0, 0),
    );

    let trap = maximal_inescapable_trap(&cfg);
    let tempo = maximal_tempo_trap(&cfg, &trap);

    assert_eq!(tempo.len(), 113);
    assert!(tempo.is_subset(&trap));

    // Tempo traps are about infinite play with "free passes" occurring infinitely often.
    // Immediate checkmates are terminal (no black move), so they are not part of the
    // Büchi winning set.
    let rules = cfg.rules();
    for p in &tempo {
        assert!(!rules.is_checkmate(p));
    }
}
