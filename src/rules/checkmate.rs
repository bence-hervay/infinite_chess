use crate::game::Game;
use crate::pieces::Turn;
use crate::state::PackedState;

use super::attacks::{build_white_occupancy, is_attacked_by_white};
use super::movegen::{successors, Scratch};

pub fn is_in_check(game: &Game, bk_sq: u16, whites: &[u16]) -> bool {
    let region = &game.region;
    let layout = &game.layout;
    let cap = game.captured_code();
    let bk_c = region.coord_of(bk_sq);
    let occ_white = build_white_occupancy(region, whites, cap);
    is_attacked_by_white(bk_c, region, layout, whites, cap, &occ_white)
}

/// True if this black-to-move position is checkmate in the finite-slice game.
///
/// That means:
/// - black king is currently in check, and
/// - black has no legal move (including moves that would leave the region).
pub fn is_checkmate_black_to_move(game: &Game, state: PackedState, scratch: &mut Scratch) -> bool {
    let cap = game.captured_code();
    let bk_sq = game.packer.unpack(state, scratch.whites_mut());
    let bk_c = game.region.coord_of(bk_sq);
    let whites = scratch.whites();
    let occ_white = build_white_occupancy(&game.region, whites, cap);
    let in_check = is_attacked_by_white(bk_c, &game.region, &game.layout, whites, cap, &occ_white);
    if !in_check {
        return false;
    }
    let mvs = successors(game, Turn::Black, state, scratch);
    mvs.is_empty()
}

/// Utility: does Black have *any* legal move from this state?
pub fn black_has_any_legal_move(game: &Game, state: PackedState, scratch: &mut Scratch) -> bool {
    let mvs = successors(game, Turn::Black, state, scratch);
    !mvs.is_empty()
}
