use crate::arena::Arena;
use crate::pieces::Turn;
use std::collections::VecDeque;

/// Compute the maximal safety trap inside the region.
///
/// Returns a boolean membership vector of length `arena.len()`.
/// Sinks are always `false`.
pub fn safety_trap(arena: &Arena) -> Vec<bool> {
    let n = arena.len();
    let mut in_set = vec![true; n];
    in_set[arena.sink_black] = false;
    in_set[arena.sink_white] = false;

    let mut out_count: Vec<usize> = vec![0; n];
    for id in 0..n {
        if !in_set[id] {
            continue;
        }
        out_count[id] = arena.nodes[id].succ.iter().filter(|&&s| in_set[s]).count();
    }

    let mut q: VecDeque<usize> = VecDeque::new();

    for id in 0..n {
        if !in_set[id] {
            continue;
        }
        let node = &arena.nodes[id];
        match node.turn {
            Turn::White => {
                if out_count[id] == 0 {
                    q.push_back(id);
                }
            }
            Turn::Black => {
                if out_count[id] < node.succ.len() {
                    q.push_back(id);
                }
            }
        }
    }

    while let Some(v) = q.pop_front() {
        if !in_set[v] {
            continue;
        }
        in_set[v] = false;

        for &p in &arena.nodes[v].pred {
            if !in_set[p] {
                continue;
            }
            match arena.nodes[p].turn {
                Turn::White => {
                    // White needs at least one successor remaining.
                    if out_count[p] > 0 {
                        out_count[p] -= 1;
                    }
                    if out_count[p] == 0 {
                        q.push_back(p);
                    }
                }
                Turn::Black => {
                    // Black is bad for White if ANY successor is outside the set.
                    q.push_back(p);
                }
            }
        }
    }

    in_set
}
