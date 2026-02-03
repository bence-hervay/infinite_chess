use infinite_chess::arena::ArenaBuilder;
use infinite_chess::coord::Coord;
use infinite_chess::game::Game;
use infinite_chess::pieces::{Material, Turn};
use infinite_chess::region::Region;
use infinite_chess::solve::buchi::tempo_trap;
use infinite_chess::solve::reach::{checkmate_targets, reachability_white};
use infinite_chess::solve::safety::safety_trap;

fn find_node_id(arena: &infinite_chess::arena::Arena, turn: Turn, state: u128) -> usize {
    arena
        .nodes
        .iter()
        .enumerate()
        .find(|(_, n)| n.turn == turn && n.state == Some(state))
        .map(|(id, _)| id)
        .expect("node should exist")
}

#[test]
fn two_queens_r2_safety_and_tempo_sizes() {
    let region = Region::linf(2);
    let material = Material::new().with_queens(2);
    let game = Game::new(region, material).with_allow_pass(true);

    let arena = ArenaBuilder::new(game).enumerate_all();

    let safety = safety_trap(&arena);
    let safety_count = safety.iter().filter(|&&b| b).count();
    assert_eq!(safety_count, 4600);

    let tempo = tempo_trap(&arena, &safety);
    let tempo_count = tempo.iter().filter(|&&b| b).count();
    assert_eq!(tempo_count, 1824);
}

#[test]
fn two_queens_r2_checkmate_target_count_and_example() {
    let region = Region::linf(2);
    let material = Material::new().with_queens(2);
    let game = Game::new(region, material).with_allow_pass(true);
    let arena = ArenaBuilder::new(game.clone()).enumerate_all();

    let targets = checkmate_targets(&arena);
    let target_count = targets.iter().filter(|&&b| b).count();
    assert_eq!(target_count, 352);

    // A concrete checkmate:
    // Black king at (0,0), queens at (-2,-1) and (0,1), black to move.
    let st = game.pack_from_coords(
        Coord::new(0, 0),
        &[Some(Coord::new(-2, -1)), Some(Coord::new(0, 1))],
    );
    let id = find_node_id(&arena, Turn::Black, st);
    assert!(targets[id]);
}

#[test]
fn two_queens_r2_reachability_to_mate_properties() {
    let region = Region::linf(2);
    let material = Material::new().with_queens(2);
    let game = Game::new(region, material).with_allow_pass(true);
    let arena = ArenaBuilder::new(game.clone()).enumerate_all();

    let safety = safety_trap(&arena);
    let safety_count = safety.iter().filter(|&&b| b).count();
    assert_eq!(safety_count, 4600);

    let targets = checkmate_targets(&arena);
    let win = reachability_white(&arena, &targets);
    let win_count = win.iter().filter(|&&b| b).count();
    assert_eq!(win_count, 4572);

    // Reachability-to-mate should be a subset of the safety trap in this configuration.
    for i in 0..arena.len() {
        if win[i] {
            assert!(safety[i]);
        }
    }

    let diff = safety
        .iter()
        .zip(win.iter())
        .filter(|(&s, &w)| s && !w)
        .count();
    assert_eq!(diff, 28);

    // "Far" initial: black at center, queens at opposite corners.
    let st_far = game.pack_from_coords(
        Coord::new(0, 0),
        &[Some(Coord::new(-2, -2)), Some(Coord::new(2, 2))],
    );
    let id_far = find_node_id(&arena, Turn::Black, st_far);
    assert!(win[id_far]);

    // A known safety-but-not-mate node.
    let st_safe_not_mate = game.pack_from_coords(
        Coord::new(0, -1),
        &[Some(Coord::new(-2, -2)), Some(Coord::new(1, 2))],
    );
    let id2 = find_node_id(&arena, Turn::Black, st_safe_not_mate);
    assert!(safety[id2]);
    assert!(!win[id2]);
}
