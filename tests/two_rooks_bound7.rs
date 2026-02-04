use infinite_chess::scenarios;
use infinite_chess::search::trap::maximal_inescapable_trap;

/// Intentionally ignored by default: this runs a full trap fixed-point computation.
///
/// Run with:
/// `cargo test --release -- --ignored two_rooks_bound7_trap_computes_without_limits`
#[test]
#[ignore]
fn two_rooks_bound7_trap_computes_without_limits() {
    let scn = scenarios::two_rooks_bound7();
    scn.validate().unwrap();

    let trap = maximal_inescapable_trap(&scn).expect("trap computation failed");
    // We only assert the computation succeeds (no resource limit exceeded).
    // The trap may or may not be empty depending on the slice parameters.
    let _ = trap;
}
