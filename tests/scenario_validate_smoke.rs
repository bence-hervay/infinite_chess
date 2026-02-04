use infinite_chess::scenarios;

#[test]
fn built_in_scenarios_load_and_validate() {
    // `two_rooks_bound7` used to explode during candidate generation due to reachable-state blowup.
    let scn = scenarios::two_rooks_bound7();
    scn.validate().expect("two_rooks_bound7 must validate");

    // Generator-backed scenario: this ensures we can construct it and it passes basic invariants.
    let scn = scenarios::nbb7_generated().expect("failed to build nbb7_generated");
    scn.validate().expect("nbb7_generated must validate");
}
