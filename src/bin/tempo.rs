use infinite_chess::arena::ArenaBuilder;
use infinite_chess::game::Game;
use infinite_chess::pieces::Material;
use infinite_chess::region::Region;
use infinite_chess::solve::buchi::tempo_trap;
use infinite_chess::solve::safety::safety_trap;

fn main() {
    let region = Region::linf(2);
    let material = Material::new().with_queens(2);
    let game = Game::new(region, material).with_allow_pass(true);

    let arena = ArenaBuilder::new(game).enumerate_all();

    let safety = safety_trap(&arena);
    let safety_count = safety.iter().filter(|&&b| b).count();

    let tempo = tempo_trap(&arena, &safety);
    let tempo_count = tempo.iter().filter(|&&b| b).count();

    println!("Nodes: {} (including 2 sinks)", arena.len());
    println!("Safety-trap nodes: {safety_count}");
    println!("Tempo-trap (Buchi) nodes: {tempo_count}");
}
