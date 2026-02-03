use crate::arena::Arena;
use crate::pieces::Turn;
use crate::rules::attacks::{build_white_occupancy, is_attacked_by_white};
use crate::rules::movegen::Scratch;

use std::collections::VecDeque;

/// Generic reachability winning set for White: nodes from which White can force
/// reaching `target`.
///
/// `target` is a boolean membership vector over nodes.
pub fn reachability_white(arena: &Arena, target: &[bool]) -> Vec<bool> {
    let n = arena.len();
    assert_eq!(target.len(), n);

    let mut win = target.to_vec();

    // For black nodes: number of successors that are *not* in win.
    let mut rem_black: Vec<u32> = vec![0; n];
    for id in 0..n {
        if arena.nodes[id].turn == Turn::Black {
            let cnt = arena.nodes[id].succ.iter().filter(|&&s| !win[s]).count() as u32;
            rem_black[id] = cnt;
        }
    }

    let mut q: VecDeque<usize> = VecDeque::new();
    for id in 0..n {
        if win[id] {
            q.push_back(id);
        }
    }

    while let Some(v) = q.pop_front() {
        for &p in &arena.nodes[v].pred {
            if win[p] {
                continue;
            }
            match arena.nodes[p].turn {
                Turn::White => {
                    // White can choose a winning successor.
                    win[p] = true;
                    q.push_back(p);
                }
                Turn::Black => {
                    // Black is only winning for White if *all* successors are winning.
                    if rem_black[p] == 0 {
                        continue;
                    }
                    rem_black[p] -= 1;
                    if rem_black[p] == 0 && !arena.nodes[p].succ.is_empty() {
                        win[p] = true;
                        q.push_back(p);
                    }
                }
            }
        }
    }

    win
}

/// Target set: checkmates (black-to-move, in check, with no legal moves).
pub fn checkmate_targets(arena: &Arena) -> Vec<bool> {
    let n = arena.len();
    let mut target = vec![false; n];

    let game = &arena.game;
    let cap = game.captured_code();
    let region = &game.region;
    let layout = &game.layout;

    let mut scratch = Scratch::new(layout.total_white());

    for id in 0..n {
        if arena.nodes[id].turn != Turn::Black {
            continue;
        }
        if arena.is_sink(id) {
            continue;
        }
        if !arena.nodes[id].succ.is_empty() {
            continue;
        }
        let st = arena.nodes[id].state.expect("non-sink state");
        let bk_sq = game.packer.unpack(st, scratch.whites_mut());
        let whites = scratch.whites();
        let occ = build_white_occupancy(region, whites, cap);
        let bk_c = region.coord_of(bk_sq);
        let in_check = is_attacked_by_white(bk_c, region, layout, whites, cap, &occ);
        if in_check {
            target[id] = true;
        }
    }

    target
}
