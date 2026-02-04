use std::path::Path;

use infinite_chess::scenarios;
use infinite_chess::solution::{export_bundle, ExportOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: export_solution <scenario> <out_dir> [--force] [--no-tempo] [--view-bound <B>]\n\nAvailable scenarios:\n  - {}",
            scenarios::available_names().join("\n  - ")
        );
        std::process::exit(2);
    }

    let scenario_name = &args[1];
    let out_dir = Path::new(&args[2]);

    let mut opts = ExportOptions::default();

    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--force" => {
                opts.force = true;
                i += 1;
            }
            "--no-tempo" => {
                opts.compute_tempo = false;
                i += 1;
            }
            "--view-bound" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("--view-bound requires an integer argument");
                    std::process::exit(2);
                };
                let b: i32 = match v.parse() {
                    Ok(x) => x,
                    Err(e) => {
                        eprintln!("invalid --view-bound {v}: {e}");
                        std::process::exit(2);
                    }
                };
                opts.view_bound = Some(b);
                i += 2;
            }
            x => {
                eprintln!("Unknown option: {x}");
                std::process::exit(2);
            }
        }
    }

    let scn = match scenarios::by_name(scenario_name) {
        Ok(Some(s)) => s,
        Ok(None) => {
            eprintln!(
                "Unknown scenario: {scenario_name}\n\nAvailable scenarios:\n  - {}",
                scenarios::available_names().join("\n  - ")
            );
            std::process::exit(2);
        }
        Err(e) => {
            eprintln!("Failed to load scenario {scenario_name}: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = scn.validate() {
        eprintln!("Invalid scenario {scenario_name}: {e}");
        std::process::exit(2);
    }

    match export_bundle(&scn, out_dir, opts) {
        Ok(bundle) => {
            println!("Exported solution bundle to {}", out_dir.display());
            println!(
                "  states: {}, trap: {}, tempo: {}",
                bundle.manifest.counts.states,
                bundle.manifest.counts.trap,
                bundle.manifest.counts.tempo
            );
        }
        Err(e) => {
            eprintln!("Export failed: {e}");
            std::process::exit(1);
        }
    }
}
