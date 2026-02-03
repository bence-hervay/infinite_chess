use crate::chess::bounds::enumerate_positions_in_bound;
use crate::chess::rules::Rules;

/// Count checkmates (black king in check + no legal black moves) among positions
/// where all non-captured pieces lie within the given L∞ bound.
///
/// This uses **true infinite-board** mate logic: it does *not* treat the slice edge
/// as a wall. If black has a legal move, it's not mate.
pub fn count_checkmates_in_bound(rules: &Rules, bound: i32) -> usize {
    let positions = enumerate_positions_in_bound(&rules.layout, bound, false);
    positions.iter().filter(|p| rules.is_checkmate(p)).count()
}

/// Enumerate all checkmates within the bound.
pub fn checkmates_in_bound(rules: &Rules, bound: i32) -> Vec<crate::core::position::Position> {
    let positions = enumerate_positions_in_bound(&rules.layout, bound, false);
    positions
        .into_iter()
        .filter(|p| rules.is_checkmate(p))
        .collect()
}
