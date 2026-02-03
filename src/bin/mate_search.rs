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

    let bound = match scn.candidates {
        infinite_chess::scenario::CandidateGeneration::InLinfBound { bound, .. } => bound,
        _ => {
            eprintln!("Scenario {scenario_name} does not define an L∞ bound for mate enumeration.");
            std::process::exit(2);
        }
    };

    let mates = count_checkmates_in_bound(&scn.rules, bound);

    println!("Scenario: {}", scn.name);
    println!("  pieces: {:?}", scn.rules.layout.kinds());
    println!("  bound: {}", bound);
    println!("  checkmates in slice (infinite-board legality): {}", mates);
}
