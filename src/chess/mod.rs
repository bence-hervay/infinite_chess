//! Pure chess rules (traditional pieces) in king-relative coordinates.
//!
//! This module contains no scenario-specific restrictions. It answers questions like:
//! - “Is this [`Position`](crate::core::position::Position) structurally legal?”
//! - “Which squares are attacked by White?”
//! - “What are the legal black king moves (including captures)?”
//!
//! ## Coordinate system
//!
//! The black king is always at the origin `(0,0)`. A [`Position`](crate::core::position::Position) stores only
//! the white piece squares relative to that king.
//!
//! When the black king “moves” by `delta`, the position is re-centered by shifting every piece
//! by `-delta`, possibly capturing a piece that sits on `delta`.
//!
//! ## Movement cap
//!
//! Sliding pieces are limited by `move_bound` (a scenario parameter). This keeps move generation
//! finite and makes enumeration/search feasible in practice.

pub mod bounds;
pub mod layout;
pub mod piece;
pub mod rules;
