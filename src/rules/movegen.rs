use crate::coord::Coord;
use crate::game::Game;
use crate::pieces::{PieceKind, Turn};
use crate::state::{canonicalize, PackedState};

use super::attacks::{build_white_occupancy, is_attacked_by_white, Occ};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Succ {
    State(PackedState),
    Sink,
}

#[derive(Clone, Debug)]
pub struct Scratch {
    whites: Vec<u16>,
}

impl Scratch {
    pub fn new(n_white: usize) -> Self {
        Self {
            whites: vec![0u16; n_white],
        }
    }

    pub fn whites(&self) -> &[u16] {
        &self.whites
    }

    pub fn whites_mut(&mut self) -> &mut [u16] {
        &mut self.whites
    }
}

pub fn successors(game: &Game, turn: Turn, state: PackedState, scratch: &mut Scratch) -> Vec<Succ> {
    let cap = game.captured_code();
    let bk_sq = game.packer.unpack(state, &mut scratch.whites);

    match turn {
        Turn::Black => black_succ(game, bk_sq, &scratch.whites, cap),
        Turn::White => white_succ(game, bk_sq, state, &scratch.whites, cap),
    }
}

fn black_succ(game: &Game, bk_sq: u16, whites: &[u16], captured_code: u16) -> Vec<Succ> {
    let region = &game.region;
    let layout = &game.layout;

    let bk_c = region.coord_of(bk_sq);
    let occ_white = build_white_occupancy(region, whites, captured_code);

    let steps: [Coord; 8] = [
        Coord::new(-1, -1),
        Coord::new(-1, 0),
        Coord::new(-1, 1),
        Coord::new(0, -1),
        Coord::new(0, 1),
        Coord::new(1, -1),
        Coord::new(1, 0),
        Coord::new(1, 1),
    ];

    let mut out: Vec<Succ> = Vec::new();
    let mut has_sink = false;

    for step in steps {
        let dst = Coord::new(bk_c.x + step.x, bk_c.y + step.y);
        if let Some(dst_sq) = region.sq_of(dst) {
            if occ_white.get(dst_sq) {
                // capture that piece (unless it's the white king)
                let Some(slot_idx) = find_slot_at(whites, dst_sq, captured_code) else {
                    continue;
                };
                if layout.slots[slot_idx] == PieceKind::King {
                    continue;
                }

                let mut whites2 = whites.to_vec();
                whites2[slot_idx] = captured_code;
                canonicalize(&mut whites2, layout);

                let mut occ_after = occ_white.clone();
                occ_after.clear(dst_sq);

                let attacked = is_attacked_by_white(dst, region, layout, &whites2, captured_code, &occ_after);
                if attacked {
                    continue;
                }
                let st = game.packer.pack(dst_sq, &whites2);
                out.push(Succ::State(st));
            } else {
                // normal move
                let attacked = is_attacked_by_white(dst, region, layout, whites, captured_code, &occ_white);
                if attacked {
                    continue;
                }
                let st = game.packer.pack(dst_sq, whites);
                out.push(Succ::State(st));
            }
        } else {
            // outside region => escape sink (if not attacked)
            let attacked = is_attacked_by_white(dst, region, layout, whites, captured_code, &occ_white);
            if attacked {
                continue;
            }
            has_sink = true;
        }
    }

    if has_sink {
        out.push(Succ::Sink);
    }

    out
}

fn white_succ(game: &Game, bk_sq: u16, state: PackedState, whites: &[u16], captured_code: u16) -> Vec<Succ> {
    let region = &game.region;
    let layout = &game.layout;

    let bk_c = region.coord_of(bk_sq);

    let occ_white = build_white_occupancy(region, whites, captured_code);
    let mut occ_all = occ_white.clone();
    occ_all.set(bk_sq);

    let mut out: Vec<Succ> = Vec::new();
    let mut has_sink = false;

    if game.allow_pass {
        out.push(Succ::State(state));
    }

    for (slot_idx, kind) in layout.slots.iter().enumerate() {
        let code = whites[slot_idx];
        if code == captured_code {
            continue;
        }
        let from = region.coord_of(code);

        match kind {
            PieceKind::King => {
                let steps: [Coord; 8] = [
                    Coord::new(-1, -1),
                    Coord::new(-1, 0),
                    Coord::new(-1, 1),
                    Coord::new(0, -1),
                    Coord::new(0, 1),
                    Coord::new(1, -1),
                    Coord::new(1, 0),
                    Coord::new(1, 1),
                ];
                for step in steps {
                    let dst = Coord::new(from.x + step.x, from.y + step.y);
                    // cannot move adjacent to black king
                    if (dst.x - bk_c.x).abs() <= 1 && (dst.y - bk_c.y).abs() <= 1 {
                        continue;
                    }
                    if let Some(dst_sq) = region.sq_of(dst) {
                        if occ_all.get(dst_sq) {
                            continue;
                        }
                        let mut whites2 = whites.to_vec();
                        whites2[slot_idx] = dst_sq;
                        canonicalize(&mut whites2, layout);
                        let st = game.packer.pack(bk_sq, &whites2);
                        out.push(Succ::State(st));
                    } else {
                        has_sink = true;
                    }
                }
            }
            PieceKind::Knight => {
                let moves: [Coord; 8] = [
                    Coord::new(1, 2),
                    Coord::new(2, 1),
                    Coord::new(-1, 2),
                    Coord::new(-2, 1),
                    Coord::new(1, -2),
                    Coord::new(2, -1),
                    Coord::new(-1, -2),
                    Coord::new(-2, -1),
                ];
                for mv in moves {
                    let dst = Coord::new(from.x + mv.x, from.y + mv.y);
                    if let Some(dst_sq) = region.sq_of(dst) {
                        if occ_all.get(dst_sq) {
                            continue;
                        }
                        let mut whites2 = whites.to_vec();
                        whites2[slot_idx] = dst_sq;
                        canonicalize(&mut whites2, layout);
                        let st = game.packer.pack(bk_sq, &whites2);
                        out.push(Succ::State(st));
                    } else {
                        has_sink = true;
                    }
                }
            }
            PieceKind::Rook => {
                gen_sliding(
                    game,
                    bk_sq,
                    whites,
                    captured_code,
                    slot_idx,
                    &occ_all,
                    &mut out,
                    &mut has_sink,
                    &[
                        Coord::new(1, 0),
                        Coord::new(-1, 0),
                        Coord::new(0, 1),
                        Coord::new(0, -1),
                    ],
                );
            }
            PieceKind::Bishop => {
                gen_sliding(
                    game,
                    bk_sq,
                    whites,
                    captured_code,
                    slot_idx,
                    &occ_all,
                    &mut out,
                    &mut has_sink,
                    &[
                        Coord::new(1, 1),
                        Coord::new(1, -1),
                        Coord::new(-1, 1),
                        Coord::new(-1, -1),
                    ],
                );
            }
            PieceKind::Queen => {
                gen_sliding(
                    game,
                    bk_sq,
                    whites,
                    captured_code,
                    slot_idx,
                    &occ_all,
                    &mut out,
                    &mut has_sink,
                    &[
                        Coord::new(1, 0),
                        Coord::new(-1, 0),
                        Coord::new(0, 1),
                        Coord::new(0, -1),
                        Coord::new(1, 1),
                        Coord::new(1, -1),
                        Coord::new(-1, 1),
                        Coord::new(-1, -1),
                    ],
                );
            }
        }
    }

    if has_sink {
        out.push(Succ::Sink);
    }

    out
}

fn gen_sliding(
    game: &Game,
    bk_sq: u16,
    whites: &[u16],
    _captured_code: u16,
    slot_idx: usize,
    occ_all: &Occ,
    out: &mut Vec<Succ>,
    has_sink: &mut bool,
    dirs: &[Coord],
) {
    let region = &game.region;
    let layout = &game.layout;

    let from_sq = whites[slot_idx];
    let from = region.coord_of(from_sq);
    let bound = game.move_bound;

    for dir in dirs {
        let mut step_count: u16 = 0;
        let mut cur = Coord::new(from.x + dir.x, from.y + dir.y);

        loop {
            if let Some(b) = bound {
                if step_count >= b {
                    break;
                }
            }

            if let Some(cur_sq) = region.sq_of(cur) {
                if occ_all.get(cur_sq) {
                    break;
                }
                let mut whites2 = whites.to_vec();
                whites2[slot_idx] = cur_sq;
                canonicalize(&mut whites2, layout);
                let st = game.packer.pack(bk_sq, &whites2);
                out.push(Succ::State(st));

                step_count += 1;
                cur = Coord::new(cur.x + dir.x, cur.y + dir.y);
            } else {
                *has_sink = true;
                break;
            }
        }
    }
}

fn find_slot_at(whites: &[u16], sq: u16, captured_code: u16) -> Option<usize> {
    whites
        .iter()
        .enumerate()
        .find(|(_, &c)| c != captured_code && c == sq)
        .map(|(i, _)| i)
}
