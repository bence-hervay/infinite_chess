use crate::core::coord::Coord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PieceKind {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
}

impl PieceKind {
    /// Unit directions for sliding pieces.
    pub fn slide_dirs(self) -> &'static [Coord] {
        use PieceKind::*;
        match self {
            Queen => &QUEEN_DIRS,
            Rook => &ROOK_DIRS,
            Bishop => &BISHOP_DIRS,
            _ => &[],
        }
    }
}

pub const KING_STEPS: [Coord; 8] = [
    Coord { x: -1, y: -1 },
    Coord { x: -1, y: 0 },
    Coord { x: -1, y: 1 },
    Coord { x: 0, y: -1 },
    Coord { x: 0, y: 1 },
    Coord { x: 1, y: -1 },
    Coord { x: 1, y: 0 },
    Coord { x: 1, y: 1 },
];

pub const KNIGHT_STEPS: [Coord; 8] = [
    Coord { x: -2, y: -1 },
    Coord { x: -2, y: 1 },
    Coord { x: -1, y: -2 },
    Coord { x: -1, y: 2 },
    Coord { x: 1, y: -2 },
    Coord { x: 1, y: 2 },
    Coord { x: 2, y: -1 },
    Coord { x: 2, y: 1 },
];

pub const BISHOP_DIRS: [Coord; 4] = [
    Coord { x: 1, y: 1 },
    Coord { x: 1, y: -1 },
    Coord { x: -1, y: 1 },
    Coord { x: -1, y: -1 },
];

pub const ROOK_DIRS: [Coord; 4] = [
    Coord { x: 1, y: 0 },
    Coord { x: -1, y: 0 },
    Coord { x: 0, y: 1 },
    Coord { x: 0, y: -1 },
];

pub const QUEEN_DIRS: [Coord; 8] = [
    Coord { x: 1, y: 0 },
    Coord { x: -1, y: 0 },
    Coord { x: 0, y: 1 },
    Coord { x: 0, y: -1 },
    Coord { x: 1, y: 1 },
    Coord { x: 1, y: -1 },
    Coord { x: -1, y: 1 },
    Coord { x: -1, y: -1 },
];
