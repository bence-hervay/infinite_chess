use crate::chess::layout::PieceLayout;
use crate::core::square::Square;

/// Maximum number of white pieces we support (not counting the black king).
///
/// This is intentionally small: the search space explodes combinatorially.
pub const MAX_PIECES: usize = 16;

/// A piece-placement position in **king-relative coordinates**.
///
/// The black king is always at the origin (0,0). White pieces are stored as
/// squares relative to that king. Captured pieces are stored as `Square::NONE`.
///
/// The piece *types* are not stored here; that's provided by a `PieceLayout`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Position {
    squares: [Square; MAX_PIECES],
    count: u8,
}

impl Position {
    pub fn new(count: usize, squares: [Square; MAX_PIECES]) -> Self {
        debug_assert!(count <= MAX_PIECES);
        Self {
            squares,
            count: count as u8,
        }
    }

    pub fn count(&self) -> usize {
        self.count as usize
    }

    pub fn squares(&self) -> &[Square] {
        &self.squares[..self.count()]
    }

    pub fn squares_mut(&mut self) -> &mut [Square] {
        let n = self.count as usize;
        &mut self.squares[..n]
    }

    pub fn get(&self, idx: usize) -> Square {
        self.squares()[idx]
    }

    pub fn square(&self, idx: usize) -> Square {
        self.get(idx)
    }

    pub fn set(&mut self, idx: usize, sq: Square) {
        self.squares_mut()[idx] = sq;
    }

    pub fn set_square(&mut self, idx: usize, sq: Square) {
        self.set(idx, sq);
    }

    pub fn canonicalize(&mut self, layout: &PieceLayout) {
        for run in layout.identical_runs() {
            self.squares[run.start..run.end].sort();
        }
    }

    pub fn is_occupied(&self, sq: Square) -> bool {
        self.squares().iter().any(|&s| !s.is_none() && s == sq)
    }

    pub fn is_occupied_except(&self, sq: Square, except_idx: usize) -> bool {
        self.squares()
            .iter()
            .enumerate()
            .any(|(i, &s)| i != except_idx && !s.is_none() && s == sq)
    }

    pub fn iter_present(&self) -> impl Iterator<Item = (usize, Square)> + '_ {
        self.squares()
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, s)| !s.is_none())
    }

    pub fn clone_squares_array(&self) -> [Square; MAX_PIECES] {
        self.squares
    }
}
