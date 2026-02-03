use infinite_chess::arena::ArenaBuilder;
use infinite_chess::coord::Coord;
use infinite_chess::game::Game;
use infinite_chess::pieces::Material;
use infinite_chess::region::Region;
use infinite_chess::solve::reach::{checkmate_targets, reachability_white};

fn main() {
    let region = Region::linf(2);
    let material = Material::new().with_queens(2);
    let game = Game::new(region, material).with_allow_pass(true);

    let arena = ArenaBuilder::new(game.clone()).enumerate_all();

    let target = checkmate_targets(&arena);
    let n_target = target.iter().filter(|&&b| b).count();

    let win = reachability_white(&arena, &target);
    let n_win = win.iter().filter(|&&b| b).count();

    println!("Nodes: {} (including 2 sinks)", arena.len());
    println!("Checkmate target nodes: {n_target}");
    println!("Reachability-to-mate winning nodes: {n_win}");

    // Example query: black king at (0,0), queens at (-2,-2) and (2,2), black to move.
    let init = game.pack_from_coords(
        Coord::new(0, 0),
        &[Some(Coord::new(-2, -2)), Some(Coord::new(2, 2))],
    );
    // Node id in the arena: black nodes start at 2 and alternate (black, white).
    // We can just look it up by scanning once here (small demo).
    let init_id = arena
        .nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.state == Some(init) && n.turn == infinite_chess::pieces::Turn::Black)
        .map(|(id, _)| id)
        .unwrap();

    println!(
        "Example init node id = {init_id}, mate-winning? {}",
        win[init_id]
    );
}
