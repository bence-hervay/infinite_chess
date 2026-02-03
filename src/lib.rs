//! # infinite_chess
//!
//! A small research-oriented engine for reasoning about *infinite-board* endgames of the form
//! “White has a fixed set of pieces, Black has a lone king”.
//!
//! ## Architecture
//!
//! The codebase is structured as layered components:
//!
//! - [`core`]: allocation-free primitives (`Coord`, `Square`, packed [`core::position::Position`]).
//! - [`chess`]: pure chess movement + king-safety legality in **king-relative coordinates**.
//! - [`scenario`]: scenario configuration, constraints, and budgets:
//!   - **Laws** ([`scenario::LawsLike`]) restrict which moves/states are allowed *in the scenario*.
//!   - **Domain** ([`scenario::DomainLike`]) defines the “inside” set for trap objectives
//!     (leaving is allowed; it just changes the objective outcome).
//!   - **Preferences** ([`scenario::PreferencesLike`]) are tie-breakers for demos/strategy extraction.
//!   - **Resource limits** ([`scenario::ResourceLimits`]) bound state explosion and allow graceful failure.
//! - [`search`]: objective solvers (trap / tempo trap / mate enumeration).
//! - [`scenarios`]: built-in scenarios (small demos + one data-backed reference scenario).
//!
//! ## Quick start (no heavy computation)
//!
//! ```no_run
//! use infinite_chess::scenarios;
//! use infinite_chess::search::trap::{maximal_inescapable_trap, maximal_tempo_trap};
//!
//! let scn = scenarios::three_rooks_bound2_mb1();
//! let trap = maximal_inescapable_trap(&scn).unwrap();
//! let tempo = maximal_tempo_trap(&scn, &trap).unwrap();
//! assert!(tempo.is_subset(&trap));
//! ```
//!
//! For extension and design notes, see [`scenario`] and the project `README.md`.

pub mod chess;
pub mod core;
pub mod scenario;
pub mod scenarios;
pub mod search;
