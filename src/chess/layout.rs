use std::ops::Range;

use crate::chess::piece::PieceKind;

/// A fixed list of piece kinds ("slots") plus contiguous ranges of identical pieces.
///
/// We keep identical pieces contiguous so we can canonicalize positions by sorting
/// the squares only within each identical run.
#[derive(Debug, Clone)]
pub struct PieceLayout {
    kinds: Vec<PieceKind>,
    identical_runs: Vec<Range<usize>>,
    white_king_index: Option<usize>,
}

impl PieceLayout {
    /// Build a layout in a fixed, predictable order:
    ///
    /// `K, Q..., R..., B..., N...`
    pub fn from_counts(
        white_king: bool,
        queens: usize,
        rooks: usize,
        bishops: usize,
        knights: usize,
    ) -> Self {
        let mut kinds = Vec::new();
        let mut white_king_index = None;
        if white_king {
            white_king_index = Some(0);
            kinds.push(PieceKind::King);
        }
        kinds.extend(std::iter::repeat(PieceKind::Queen).take(queens));
        kinds.extend(std::iter::repeat(PieceKind::Rook).take(rooks));
        kinds.extend(std::iter::repeat(PieceKind::Bishop).take(bishops));
        kinds.extend(std::iter::repeat(PieceKind::Knight).take(knights));

        let identical_runs = compute_runs(&kinds);

        Self {
            kinds,
            identical_runs,
            white_king_index,
        }
    }

    #[inline]
    pub fn piece_count(&self) -> usize {
        self.kinds.len()
    }

    #[inline]
    pub fn kind(&self, index: usize) -> PieceKind {
        self.kinds[index]
    }

    #[inline]
    pub fn kinds(&self) -> &[PieceKind] {
        &self.kinds
    }

    #[inline]
    pub fn identical_runs(&self) -> &[Range<usize>] {
        &self.identical_runs
    }

    #[inline]
    pub fn white_king_index(&self) -> Option<usize> {
        self.white_king_index
    }
}

fn compute_runs(kinds: &[PieceKind]) -> Vec<Range<usize>> {
    if kinds.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut start = 0;
    for i in 1..=kinds.len() {
        if i == kinds.len() || kinds[i] != kinds[start] {
            runs.push(start..i);
            start = i;
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_group_identical_pieces() {
        let layout = PieceLayout::from_counts(true, 0, 3, 2, 0);
        // K R R R B B
        assert_eq!(layout.piece_count(), 6);
        assert_eq!(layout.identical_runs().len(), 3);
        assert_eq!(layout.identical_runs()[0], 0..1);
        assert_eq!(layout.identical_runs()[1], 1..4);
        assert_eq!(layout.identical_runs()[2], 4..6);
    }
}
