use infinite_chess::scenario::CandidateGeneration;
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

    println!("Scenario: {}", scn.name);
    println!("  pieces: {:?}", scn.rules.layout.kinds());
    println!("  move_bound: {}", scn.rules.move_bound);
    println!("  white_can_pass: {}", scn.white_can_pass);
    println!("  remove_stalemates: {}", scn.remove_stalemates);
    match &scn.candidates {
        CandidateGeneration::InLinfBound {
            bound,
            allow_captures,
        } => println!(
            "  candidates: InLinfBound {{ bound: {bound}, allow_captures: {allow_captures} }}"
        ),
        CandidateGeneration::InBox {
            bound,
            allow_captures,
        } => println!("  candidates: InBox {{ bound: {bound}, allow_captures: {allow_captures} }}"),
        CandidateGeneration::FromStates { states } => {
            println!("  candidates: FromStates {{ states: {} }}", states.len())
        }
        CandidateGeneration::ReachableFromStart { max_queue } => {
            println!("  candidates: ReachableFromStart {{ max_queue: {max_queue} }}")
        }
    }
    println!("  track_abs_king: {}", scn.track_abs_king);
    println!("  cache_mode: {:?}", scn.cache_mode);
    println!("  limits:");
    println!("    max_states: {}", scn.limits.max_states);
    println!("    max_edges: {}", scn.limits.max_edges);
    println!("    max_cache_entries: {}", scn.limits.max_cache_entries);
    println!("    max_cached_moves: {}", scn.limits.max_cached_moves);
    println!("    max_runtime_steps: {}", scn.limits.max_runtime_steps);

    let trap = match maximal_inescapable_trap(&scn) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Trap search failed: {e}");
            std::process::exit(1);
        }
    };
    println!("inescapable trap size: {}", trap.len());

    let tempo = match maximal_tempo_trap(&scn, &trap) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Tempo trap search failed: {e}");
            std::process::exit(1);
        }
    };
    println!("tempo trap size: {}", tempo.len());
}
