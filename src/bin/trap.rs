use infinite_chess::arena::ArenaBuilder;
use infinite_chess::game::Game;
use infinite_chess::pieces::Material;
use infinite_chess::region::Region;
use infinite_chess::solve::safety::safety_trap;

fn main() {
    // Demo: two white queens vs black king, bounded to an L_inf square of radius 2.
    let region = Region::linf(2);
    let material = Material::new().with_queens(2);
    let game = Game::new(region, material).with_allow_pass(true);

    let arena = ArenaBuilder::new(game).enumerate_all();

    let safety = safety_trap(&arena);
    let safe_count = safety.iter().filter(|&&b| b).count();

    println!("Nodes: {} (including 2 sinks)", arena.len());
    println!("Safety-trap nodes: {safe_count}");
}
