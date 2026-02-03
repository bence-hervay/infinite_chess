use crate::arena::Arena;
use crate::pieces::Turn;
use std::collections::VecDeque;

/// Compute a "tempo trap" inside an existing safety trap.
///
/// We treat as accepting (`F`) the white-to-move nodes where a pass move
/// keeps the game inside the safety trap.
///
/// Returns a boolean membership vector of length `arena.len()`.
/// Nodes outside `safety` are always `false`.
pub fn tempo_trap(arena: &Arena, safety: &[bool]) -> Vec<bool> {
    let n = arena.len();
    assert_eq!(safety.len(), n);

    let mut accept = vec![false; n];
    if arena.game.allow_pass {
        // In our enumeration builder, the "paired" node with the same state but opposite turn is id ^ 1.
        for id in 0..n {
            if !safety[id] {
                continue;
            }
            if arena.nodes[id].turn == Turn::White {
                let pass_to = id ^ 1;
                if safety.get(pass_to).copied().unwrap_or(false) {
                    accept[id] = true;
                }
            }
        }
    }

    buchi(arena, safety, &accept)
}

/// White Buchi winning set for visiting `accept` infinitely often while staying inside `base`.
pub fn buchi(arena: &Arena, base: &[bool], accept: &[bool]) -> Vec<bool> {
    let n = arena.len();
    assert_eq!(base.len(), n);
    assert_eq!(accept.len(), n);

    // Start from the base region.
    let mut w = base.to_vec();

    // If accept is empty, winning set is empty.
    if !accept.iter().any(|&b| b) {
        return vec![false; n];
    }

    loop {
        // A = Attr_white(accept & W)
        let mut target = vec![false; n];
        for i in 0..n {
            target[i] = w[i] && accept[i];
        }
        let a = attractor_white(arena, &w, &target);

        // B = W \ A
        let mut has_b = false;
        let mut bset = vec![false; n];
        for i in 0..n {
            bset[i] = w[i] && !a[i];
            if bset[i] {
                has_b = true;
            }
        }
        if !has_b {
            return w;
        }

        // C = Attr_black(B) within W
        let c = attractor_black(arena, &w, &bset);

        // W = W \ C
        let mut changed = false;
        for i in 0..n {
            if w[i] && c[i] {
                w[i] = false;
                changed = true;
            }
        }

        if !changed {
            return w;
        }
    }
}

fn attractor_white(arena: &Arena, w: &[bool], target: &[bool]) -> Vec<bool> {
    // Player0 = White (exists), Player1 = Black (forall)
    let n = arena.len();
    let mut in_attr = vec![false; n];
    let mut rem: Vec<u32> = vec![0; n];

    for id in 0..n {
        if !w[id] {
            continue;
        }
        if arena.nodes[id].turn == Turn::Black {
            let cnt = arena.nodes[id].succ.iter().filter(|&&s| w[s]).count() as u32;
            rem[id] = cnt;
        }
    }

    let mut q: VecDeque<usize> = VecDeque::new();
    for id in 0..n {
        if target[id] {
            in_attr[id] = true;
            q.push_back(id);
        }
    }

    while let Some(v) = q.pop_front() {
        for &p in &arena.nodes[v].pred {
            if !w[p] || in_attr[p] {
                continue;
            }
            match arena.nodes[p].turn {
                Turn::White => {
                    in_attr[p] = true;
                    q.push_back(p);
                }
                Turn::Black => {
                    if rem[p] == 0 {
                        continue;
                    }
                    rem[p] -= 1;
                    if rem[p] == 0 {
                        in_attr[p] = true;
                        q.push_back(p);
                    }
                }
            }
        }
    }

    in_attr
}

fn attractor_black(arena: &Arena, w: &[bool], target: &[bool]) -> Vec<bool> {
    // Player1 = Black (exists), Player0 = White (forall)
    let n = arena.len();
    let mut in_attr = vec![false; n];
    let mut rem: Vec<u32> = vec![0; n];

    for id in 0..n {
        if !w[id] {
            continue;
        }
        if arena.nodes[id].turn == Turn::White {
            let cnt = arena.nodes[id].succ.iter().filter(|&&s| w[s]).count() as u32;
            rem[id] = cnt;
        }
    }

    let mut q: VecDeque<usize> = VecDeque::new();
    for id in 0..n {
        if target[id] {
            in_attr[id] = true;
            q.push_back(id);
        }
    }

    while let Some(v) = q.pop_front() {
        for &p in &arena.nodes[v].pred {
            if !w[p] || in_attr[p] {
                continue;
            }
            match arena.nodes[p].turn {
                Turn::Black => {
                    in_attr[p] = true;
                    q.push_back(p);
                }
                Turn::White => {
                    if rem[p] == 0 {
                        continue;
                    }
                    rem[p] -= 1;
                    if rem[p] == 0 {
                        in_attr[p] = true;
                        q.push_back(p);
                    }
                }
            }
        }
    }

    in_attr
}
