use crate::chess::layout::PieceLayout;
use crate::chess::piece::PieceKind;
use crate::core::coord::{Coord, KING_STEPS};
use crate::core::position::{Position, MAX_PIECES};
use crate::core::square::Square;

/// Rules for "white pieces vs a lone black king", in king-relative coordinates.
#[derive(Debug, Clone)]
pub struct Rules {
    pub layout: PieceLayout,
    pub move_bound: i32,
}

impl Rules {
    pub fn new(layout: PieceLayout, move_bound: i32) -> Self {
        assert!(move_bound >= 1);
        assert!(layout.piece_count() <= MAX_PIECES);
        Self { layout, move_bound }
    }

    /// True iff the position respects basic legality constraints:
    /// - no non-captured piece is on the origin (black king square)
    /// - no two non-captured pieces share a square
    /// - the white king (if present) is not adjacent to the black king
    pub fn is_legal_position(&self, pos: &Position) -> bool {
        // origin & duplicates
        let mut seen: [Square; MAX_PIECES] = [Square::NONE; MAX_PIECES];
        let mut seen_len = 0usize;

        for &sq in pos.squares() {
            if sq.is_none() {
                continue;
            }
            if sq.coord() == Coord::ORIGIN {
                return false;
            }
            if seen.iter().take(seen_len).any(|&s| s == sq) {
                return false;
            }
            seen[seen_len] = sq;
            seen_len += 1;
        }

        if let Some(k_idx) = self.layout.white_king_index() {
            let ks = pos.square(k_idx);
            if !ks.is_none() && ks.coord().chebyshev_norm() <= 1 {
                return false;
            }
        }

        true
    }

    /// Does *any* white piece attack `target` in this position?
    pub fn is_attacked(&self, target: Coord, pos: &Position) -> bool {
        // We do O(n^2) blocker checks by scanning other pieces; piece counts are tiny.
        for i in 0..pos.count() {
            let sq = pos.square(i);
            if sq.is_none() {
                continue;
            }
            let kind = self.layout.kind(i);
            if self.piece_attacks(kind, sq.coord(), target, pos) {
                return true;
            }
        }
        false
    }

    #[inline]
    fn piece_attacks(&self, kind: PieceKind, from: Coord, target: Coord, pos: &Position) -> bool {
        use PieceKind::*;
        match kind {
            King => {
                // White king attacks adjacent squares.
                let d = target - from;
                d.chebyshev_norm() == 1
            }
            Knight => {
                let d = target - from;
                let ax = d.x.abs();
                let ay = d.y.abs();
                (ax == 2 && ay == 1) || (ax == 1 && ay == 2)
            }
            Rook => self.rider_attacks(from, target, &ROOK_DIRS, pos),
            Bishop => self.rider_attacks(from, target, &BISHOP_DIRS, pos),
            Queen => self.rider_attacks(from, target, &QUEEN_DIRS, pos),
        }
    }

    fn rider_attacks(&self, from: Coord, target: Coord, dirs: &[Coord], pos: &Position) -> bool {
        let v = target - from;
        if v == Coord::ORIGIN {
            return false;
        }

        // Determine which direction (unit step) we would need.
        let (dir, dist) = match normalized_dir_and_distance(v) {
            None => return false,
            Some(x) => x,
        };

        if !dirs.contains(&dir) {
            return false;
        }

        // Blockers: if any piece lies strictly between `from` and `target` on the same ray.
        for &other_sq in pos.squares() {
            if other_sq.is_none() {
                continue;
            }
            let other = other_sq.coord();
            if other == from {
                continue;
            }
            let w = other - from;
            if let Some(s) = scalar_along_dir_if_aligned(w, dir) {
                if s > 0 && s < dist {
                    return false;
                }
            }
        }
        true
    }

    /// All legal black king moves (after re-centering the king at the origin).
    pub fn black_moves(&self, pos: &Position) -> Vec<Position> {
        self.black_moves_with_delta(pos)
            .into_iter()
            .map(|(_, p)| p)
            .collect()
    }

    /// All legal black king moves, paired with the king step `delta` taken in the *current*
    /// king-relative coordinate system.
    ///
    /// This is useful for scenarios that track an absolute king anchor.
    pub fn black_moves_with_delta(&self, pos: &Position) -> Vec<(Coord, Position)> {
        let mut out: Vec<(Coord, Position)> = Vec::new();

        for &delta in &KING_STEPS {
            // The black king cannot capture the white king.
            if let Some(k_idx) = self.layout.white_king_index() {
                let ks = pos.square(k_idx);
                if !ks.is_none() && ks.coord() == delta {
                    continue;
                }
            }

            let mut next = pos.clone();

            for i in 0..next.count() {
                let sq = next.square(i);
                if sq.is_none() {
                    continue;
                }
                if sq.coord() == delta {
                    // Capture (unless it's the white king, already checked above).
                    next.set_square(i, Square::NONE);
                } else {
                    next.set_square(i, sq.shifted_neg(delta));
                }
            }

            next.canonicalize(&self.layout);

            if !self.is_legal_position(&next) {
                continue;
            }
            // Illegal if the destination square is attacked.
            if self.is_attacked(Coord::ORIGIN, &next) {
                continue;
            }

            out.push((delta, next));
        }

        out
    }

    /// All legal white moves from `pos`.
    ///
    /// `allow_pass` adds a "do nothing" move that keeps the position unchanged.
    pub fn white_moves(&self, pos: &Position, allow_pass: bool) -> Vec<Position> {
        let mut out = Vec::new();

        if allow_pass {
            out.push(pos.clone());
        }

        for i in 0..pos.count() {
            let sq = pos.square(i);
            if sq.is_none() {
                continue;
            }
            let from = sq.coord();
            let kind = self.layout.kind(i);

            match kind {
                PieceKind::King => {
                    for &d in &KING_STEPS {
                        let to = from + d;
                        if to == Coord::ORIGIN {
                            continue;
                        }
                        if to.chebyshev_norm() <= 1 {
                            // Kings can't be adjacent.
                            continue;
                        }
                        let to_sq = Square::from_coord(to);
                        if pos.is_occupied_except(to_sq, i) {
                            continue;
                        }
                        let mut next = pos.clone();
                        next.set_square(i, to_sq);
                        next.canonicalize(&self.layout);
                        // Other legality invariants should still hold.
                        if self.is_legal_position(&next) {
                            out.push(next);
                        }
                    }
                }
                PieceKind::Knight => {
                    for &d in &KNIGHT_DELTAS {
                        let to = from + d;
                        if to == Coord::ORIGIN {
                            continue;
                        }
                        let to_sq = Square::from_coord(to);
                        if pos.is_occupied_except(to_sq, i) {
                            continue;
                        }
                        let mut next = pos.clone();
                        next.set_square(i, to_sq);
                        next.canonicalize(&self.layout);
                        if self.is_legal_position(&next) {
                            out.push(next);
                        }
                    }
                }
                PieceKind::Rook | PieceKind::Bishop | PieceKind::Queen => {
                    let dirs = kind.slide_dirs();
                    for &dir in dirs {
                        for step in 1..=self.move_bound {
                            let to = from + dir * step;
                            if to == Coord::ORIGIN {
                                // The black king blocks sliding movement.
                                break;
                            }
                            let to_sq = Square::from_coord(to);
                            if pos.is_occupied_except(to_sq, i) {
                                break;
                            }
                            let mut next = pos.clone();
                            next.set_square(i, to_sq);
                            next.canonicalize(&self.layout);
                            if self.is_legal_position(&next) {
                                out.push(next);
                            }
                        }
                    }
                }
            }
        }

        out
    }

    pub fn is_checkmate(&self, pos: &Position) -> bool {
        if !self.is_attacked(Coord::ORIGIN, pos) {
            return false;
        }
        self.black_moves(pos).is_empty()
    }

    pub fn is_stalemate(&self, pos: &Position) -> bool {
        if self.is_attacked(Coord::ORIGIN, pos) {
            return false;
        }
        self.black_moves(pos).is_empty()
    }
}

const ROOK_DIRS: [Coord; 4] = [
    Coord { x: 1, y: 0 },
    Coord { x: -1, y: 0 },
    Coord { x: 0, y: 1 },
    Coord { x: 0, y: -1 },
];
const BISHOP_DIRS: [Coord; 4] = [
    Coord { x: 1, y: 1 },
    Coord { x: 1, y: -1 },
    Coord { x: -1, y: 1 },
    Coord { x: -1, y: -1 },
];
const QUEEN_DIRS: [Coord; 8] = [
    Coord { x: 1, y: 0 },
    Coord { x: -1, y: 0 },
    Coord { x: 0, y: 1 },
    Coord { x: 0, y: -1 },
    Coord { x: 1, y: 1 },
    Coord { x: 1, y: -1 },
    Coord { x: -1, y: 1 },
    Coord { x: -1, y: -1 },
];
const KNIGHT_DELTAS: [Coord; 8] = [
    Coord { x: 2, y: 1 },
    Coord { x: 2, y: -1 },
    Coord { x: -2, y: 1 },
    Coord { x: -2, y: -1 },
    Coord { x: 1, y: 2 },
    Coord { x: 1, y: -2 },
    Coord { x: -1, y: 2 },
    Coord { x: -1, y: -2 },
];

#[inline]
fn normalized_dir_and_distance(v: Coord) -> Option<(Coord, i32)> {
    let dx = v.x;
    let dy = v.y;

    // rook-like
    if dx == 0 && dy != 0 {
        return Some((Coord::new(0, dy.signum()), dy.abs()));
    }
    if dy == 0 && dx != 0 {
        return Some((Coord::new(dx.signum(), 0), dx.abs()));
    }

    // bishop-like
    if dx != 0 && dy != 0 && dx.abs() == dy.abs() {
        return Some((Coord::new(dx.signum(), dy.signum()), dx.abs()));
    }

    None
}

#[inline]
fn scalar_along_dir_if_aligned(v: Coord, dir: Coord) -> Option<i32> {
    if dir.x == 0 {
        if v.x != 0 {
            return None;
        }
        if dir.y == 0 {
            return None;
        }
        let s = v.y / dir.y;
        if s * dir.y == v.y {
            Some(s)
        } else {
            None
        }
    } else {
        let s = v.x / dir.x;
        if s * dir.x == v.x && s * dir.y == v.y {
            Some(s)
        } else {
            None
        }
    }
}
