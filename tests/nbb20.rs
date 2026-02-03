use infinite_chess::scenarios;
use infinite_chess::search::trap::{maximal_inescapable_trap, maximal_tempo_trap};

/// This test is intentionally ignored by default:
/// it loads a ~8.5MB reference trap set and then runs the fixed-point pruning algorithm,
/// which can be slow and memory-hungry.
///
/// Run with:
/// `cargo test --release -- --ignored nbb20_from_file_has_nonempty_trap_sets`
#[test]
#[ignore]
fn nbb20_from_file_has_nonempty_trap_sets() {
    let scn = scenarios::nbb20_from_file().expect("failed to load NBB scenario");

    let trap = maximal_inescapable_trap(&scn).expect("trap computation failed");
    assert!(!trap.is_empty());

    let tempo = maximal_tempo_trap(&scn, &trap).expect("tempo trap computation failed");
    assert!(!tempo.is_empty());
}
