use crate::coord::{signum_i16, Coord};
use crate::pieces::{Layout, PieceKind};
use crate::region::Region;

#[derive(Clone, Debug)]
pub struct Occ {
    data: Vec<u64>,
}

impl Occ {
    pub fn new(num_squares: usize) -> Self {
        let words = (num_squares + 63) / 64;
        Self { data: vec![0; words] }
    }

    #[inline]
    pub fn set(&mut self, sq: u16) {
        let i = sq as usize;
        self.data[i >> 6] |= 1u64 << (i & 63);
    }

    #[inline]
    pub fn clear(&mut self, sq: u16) {
        let i = sq as usize;
        self.data[i >> 6] &= !(1u64 << (i & 63));
    }

    #[inline]
    pub fn get(&self, sq: u16) -> bool {
        let i = sq as usize;
        (self.data[i >> 6] >> (i & 63)) & 1u64 == 1u64
    }
}

pub fn build_white_occupancy(region: &Region, whites: &[u16], captured_code: u16) -> Occ {
    let mut occ = Occ::new(region.size());
    for &c in whites {
        if c != captured_code {
            occ.set(c);
        }
    }
    occ
}

/// True iff `target` is attacked by any white piece.
///
/// Uses `occ_white` only as blockers for sliding pieces.
pub fn is_attacked_by_white(
    target: Coord,
    region: &Region,
    layout: &Layout,
    whites: &[u16],
    captured_code: u16,
    occ_white: &Occ,
) -> bool {
    for (i, kind) in layout.slots.iter().enumerate() {
        let code = whites[i];
        if code == captured_code {
            continue;
        }
        let from = region.coord_of(code);
        if from == target {
            continue;
        }
        if piece_attacks(*kind, from, target, region, occ_white) {
            return true;
        }
    }
    false
}

fn piece_attacks(kind: PieceKind, from: Coord, target: Coord, region: &Region, occ_white: &Occ) -> bool {
    let dx = target.x - from.x;
    let dy = target.y - from.y;

    match kind {
        PieceKind::King => {
            dx.abs() <= 1 && dy.abs() <= 1 && !(dx == 0 && dy == 0)
        }
        PieceKind::Knight => {
            let ax = dx.abs();
            let ay = dy.abs();
            (ax == 1 && ay == 2) || (ax == 2 && ay == 1)
        }
        PieceKind::Rook => {
            if dx != 0 && dy != 0 {
                return false;
            }
            let step = Coord::new(signum_i16(dx), signum_i16(dy));
            ray_clear(from, target, step, region, occ_white)
        }
        PieceKind::Bishop => {
            if dx.abs() != dy.abs() {
                return false;
            }
            let step = Coord::new(signum_i16(dx), signum_i16(dy));
            ray_clear(from, target, step, region, occ_white)
        }
        PieceKind::Queen => {
            if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
                let step = Coord::new(signum_i16(dx), signum_i16(dy));
                ray_clear(from, target, step, region, occ_white)
            } else {
                false
            }
        }
    }
}

fn ray_clear(from: Coord, target: Coord, step: Coord, region: &Region, occ_white: &Occ) -> bool {
    if step.x == 0 && step.y == 0 {
        return false;
    }

    let mut cur = from + step;
    while cur != target {
        if let Some(sq) = region.sq_of(cur) {
            if occ_white.get(sq) {
                return false;
            }
        }
        cur += step;
    }
    true
}
