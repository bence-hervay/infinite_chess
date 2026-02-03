//! Game-search utilities (trap, tempo trap, mate enumeration).
//!
//! The search layer is intentionally parameterized by [`crate::scenario::Scenario`]:
//! - Pure move generation lives in [`crate::chess::rules::Rules`].
//! - Scenario-specific legality is enforced by [`crate::scenario::LawsLike`] via [`movegen`].
//! - Domain membership (`inside`) is interpreted by objectives like [`trap`].
//! - All heavy routines use [`resources::ResourceTracker`] and return `Result<_, crate::scenario::SearchError>`.

pub mod buchi;
pub mod mates;
pub mod movegen;
pub mod resources;
pub mod strategy;
pub mod trap;
