use infinite_chess::scenarios;
use infinite_chess::search::trap::{maximal_inescapable_trap, maximal_tempo_trap};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "Usage: trap_search <scenario>\n\nAvailable scenarios:\n  - {}",
            scenarios::available_names().join("\n  - ")
        );
        std::process::exit(2);
    }

    let scenario_name = &args[1];
    let cfg = scenarios::by_name(scenario_name).unwrap_or_else(|| {
        eprintln!(
            "Unknown scenario: {scenario_name}\n\nAvailable scenarios:\n  - {}",
            scenarios::available_names().join("\n  - ")
        );
        std::process::exit(2);
    });

    println!("Scenario: {}", cfg.name);
    println!("  pieces: {:?}", cfg.layout.kinds());
    println!("  bound: {}", cfg.bound);
    println!("  move_bound: {}", cfg.move_bound);
    println!("  white_can_pass: {}", cfg.white_can_pass);
    println!("  remove_stalemates: {}", cfg.remove_stalemates);

    let trap = maximal_inescapable_trap(&cfg);
    println!("inescapable trap size: {}", trap.len());

    let tempo = maximal_tempo_trap(&cfg, &trap);
    println!("tempo trap size: {}", tempo.len());
}
