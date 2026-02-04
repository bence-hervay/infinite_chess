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
fn solution_bundle_roundtrips_for_three_rooks() {
    let dir = unique_temp_dir("solution_roundtrip");

    let scn = scenarios::three_rooks_bound2_mb1();
    let mut opts = ExportOptions::default();
    opts.force = true;
    let _bundle = export_bundle(&scn, &dir, opts).unwrap();

    let loaded = load_bundle(&dir).unwrap();

    assert_eq!(loaded.manifest.counts.states as usize, loaded.states.len());
    assert_eq!(loaded.manifest.counts.trap as usize, loaded.trap_ids.len());
    assert_eq!(
        loaded.manifest.counts.tempo as usize,
        loaded.tempo_ids.len()
    );
    assert_eq!(
        loaded.manifest.counts.trap_strategy as usize,
        loaded.strat_trap.len()
    );
    assert_eq!(
        loaded.manifest.counts.tempo_strategy as usize,
        loaded.strat_tempo.len()
    );

    let start_id = loaded.manifest.start.state_id;
    assert!(loaded.trap_ids.contains(&start_id));

    let start_state = loaded.states[start_id as usize].clone();
    assert_eq!(loaded.id_of.get(&start_state).copied(), Some(start_id));

    let start_moves = loaded.transitions[start_id as usize];
    assert!(start_moves.iter().any(|&x| x != u32::MAX));

    let _ = fs::remove_dir_all(&dir);
}
