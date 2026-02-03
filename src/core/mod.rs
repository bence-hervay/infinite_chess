//! Low-level, allocation-free primitives.
//!
//! These types are intentionally compact and hash-friendly because all solvers operate on
//! large sets/maps of positions:
//!
//! - [`coord`]: integer coordinates and common step sets (king moves).
//! - [`square`]: packed coordinates in a single `i64` plus `Square::NONE` for captured pieces.
//! - [`position`]: a fixed-capacity piece placement (`MAX_PIECES`) in king-relative coordinates.

pub mod coord;
pub mod position;
pub mod square;
