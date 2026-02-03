use crate::game::Game;
use crate::pieces::PieceKind;
use crate::state::{canonicalize, PackedState};

/// Enumerate all legal packed states inside the region for the game's material.
///
/// Legality here includes only:
/// - all pieces on distinct squares (captured pieces omitted)
/// - no white piece shares the black king square
/// - if a white king exists, it is not adjacent to the black king
pub fn all_states(game: &Game) -> Vec<PackedState> {
    let n = game.region_size() as usize;
    let cap = game.captured_code();
    let layout = &game.layout;

    let mut whites: Vec<u16> = vec![cap; layout.total_white()];
    let mut used: Vec<bool> = vec![false; n];

    let mut out: Vec<PackedState> = Vec::new();

    for bk_sq in 0..(n as u16) {
        // reset occupancy
        for u in &mut used {
            *u = false;
        }
        used[bk_sq as usize] = true;

        rec_group(0, bk_sq, game, &mut whites, &mut used, &mut out);

        used[bk_sq as usize] = false;
    }

    out
}

fn rec_group(
    g_idx: usize,
    bk_sq: u16,
    game: &Game,
    whites: &mut [u16],
    used: &mut [bool],
    out: &mut Vec<PackedState>,
) {
    let layout = &game.layout;
    let cap = game.captured_code();

    if g_idx == layout.groups.len() {
        let mut w = whites.to_vec();
        canonicalize(&mut w, layout);
        let st = game.packer.pack(bk_sq, &w);
        out.push(st);
        return;
    }

    let g = &layout.groups[g_idx];
    let start = g.start;
    let len = g.len;

    // Compute free squares.
    let mut free: Vec<u16> = Vec::new();
    for (sq, &is_used) in used.iter().enumerate() {
        if is_used {
            continue;
        }
        let sq_u16 = sq as u16;
        if g.kind == PieceKind::King {
            let bk_c = game.region.coord_of(bk_sq);
            let c = game.region.coord_of(sq_u16);
            if (c.x - bk_c.x).abs() <= 1 && (c.y - bk_c.y).abs() <= 1 {
                continue;
            }
        }
        free.push(sq_u16);
    }

    let min_alive = if g.kind == PieceKind::King { len } else { 0 };

    for alive in min_alive..=len {
        let mut chosen: Vec<u16> = Vec::with_capacity(alive);
        choose_k(&free, alive, 0, &mut chosen, &mut |chosen| {
            // write this group
            for i in 0..len {
                whites[start + i] = cap;
            }
            for (i, &sq) in chosen.iter().enumerate() {
                whites[start + i] = sq;
                used[sq as usize] = true;
            }

            rec_group(g_idx + 1, bk_sq, game, whites, used, out);

            for &sq in chosen {
                used[sq as usize] = false;
            }
        });
    }
}

fn choose_k(
    free: &[u16],
    k: usize,
    start: usize,
    chosen: &mut Vec<u16>,
    cb: &mut impl FnMut(&[u16]),
) {
    if chosen.len() == k {
        cb(chosen);
        return;
    }
    if start >= free.len() {
        return;
    }

    // Remaining needed
    let need = k - chosen.len();
    if free.len() - start < need {
        return;
    }

    for i in start..free.len() {
        chosen.push(free[i]);
        choose_k(free, k, i + 1, chosen, cb);
        chosen.pop();
    }
}
