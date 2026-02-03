use infinite_chess::scenarios;
use infinite_chess::search::mates::count_checkmates_in_bound;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!(
            "Usage: mate_search <scenario>\n\nAvailable scenarios:\n  - {}",
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

    let rules = cfg.rules();
    let mates = count_checkmates_in_bound(&rules, cfg.bound);

    println!("Scenario: {}", cfg.name);
    println!("  pieces: {:?}", cfg.layout.kinds());
    println!("  bound: {}", cfg.bound);
    println!("  checkmates in slice (infinite-board legality): {}", mates);
}
