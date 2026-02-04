use std::cmp::Ordering;

use crate::core::coord::Coord;

/// A board square packed into a single `i64`.
///
/// We use it to keep positions hashable and cheap to compare.
///
/// `Square::NONE` represents a captured piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(i64);

impl Square {
    pub const NONE: Square = Square(i64::MIN);

    pub fn is_none(self) -> bool {
        self.0 == Self::NONE.0
    }

    /// Raw packed representation of this square.
    ///
    /// This is intended for compact serialization formats. `Square::NONE` is represented as
    /// `i64::MIN`.
    pub fn raw(self) -> i64 {
        self.0
    }

    /// Construct from a raw packed square representation.
    ///
    /// This is intended for compact serialization formats. `i64::MIN` represents `Square::NONE`.
    pub fn from_raw(raw: i64) -> Square {
        Square(raw)
    }

    pub fn from_coord(c: Coord) -> Square {
        // High 32 bits = x, low 32 bits = y.
        Square(((c.x as i64) << 32) | (c.y as u32 as i64))
    }

    pub fn coord(self) -> Coord {
        debug_assert!(!self.is_none());
        let x = (self.0 >> 32) as i32;
        let y = self.0 as i32;
        Coord { x, y }
    }

    pub fn shifted(self, delta: Coord) -> Square {
        if self.is_none() {
            self
        } else {
            let c = self.coord();
            Square::from_coord(Coord {
                x: c.x + delta.x,
                y: c.y + delta.y,
            })
        }
    }

    pub fn shifted_neg(self, delta: Coord) -> Square {
        self.shifted(Coord {
            x: -delta.x,
            y: -delta.y,
        })
    }
}

impl Ord for Square {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_none(), other.is_none()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => {
                let a = self.coord();
                let b = other.coord();
                (a.x, a.y).cmp(&(b.x, b.y))
            }
        }
    }
}

impl PartialOrd for Square {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
