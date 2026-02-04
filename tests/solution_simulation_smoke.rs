use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use infinite_chess::scenarios;
use infinite_chess::solution::{export_bundle, load_bundle, ExportOptions};

fn unique_temp_dir(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("infinite_chess_tests").join(name);
    let _ = fs::create_dir_all(&base);

    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for i in 0..1000u32 {
        let p = base.join(format!("{pid}-{nanos}-{i}"));
        if fs::create_dir(&p).is_ok() {
            return p;
        }
    }

    panic!(
        "failed to create a unique temp dir under {}",
        base.display()
    );
}

#[test]
fn solution_bundle_simulation_smoke_test() {
    let dir = unique_temp_dir("solution_simulation_smoke");

    let scn = scenarios::three_rooks_bound2_mb1();
    let mut opts = ExportOptions::default();
    opts.force = true;
    let _bundle = export_bundle(&scn, &dir, opts).unwrap();
    let loaded = load_bundle(&dir).unwrap();

    let mut b_id = loaded.manifest.start.state_id;

    for _ply in 0..20 {
        assert!(loaded.trap_ids.contains(&b_id));

        let next = loaded.transitions[b_id as usize];
        let w_id = next
            .iter()
            .copied()
            .find(|&x| x != u32::MAX)
            .expect("expected at least one legal black move");

        let next_b = loaded
            .strat_tempo
            .get(&w_id)
            .copied()
            .or_else(|| loaded.strat_trap.get(&w_id).copied())
            .expect("expected white to have a saved response");

        b_id = next_b;
    }

    let _ = fs::remove_dir_all(&dir);
}
