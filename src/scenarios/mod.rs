//! Built-in scenarios (compile-time configs).

use crate::chess::config::ScenarioConfig;
use crate::chess::layout::PieceLayout;

/// 3 rooks, bound=2, move_bound=1.
///
/// This is small enough to be used in tests and fast demos.
pub fn three_rooks_bound2_mb1() -> ScenarioConfig {
    ScenarioConfig::new(
        "three_rooks_bound2_mb1",
        2,
        1,
        true,
        true,
        PieceLayout::from_counts(false, 0, 3, 0, 0),
    )
}

/// 2 rooks, bound=7 (used for the "no checkmates" known result).
pub fn two_rooks_bound7() -> ScenarioConfig {
    ScenarioConfig::new(
        "two_rooks_bound7",
        7,
        7,
        true,
        true,
        PieceLayout::from_counts(false, 0, 2, 0, 0),
    )
}

/// Return a config by name.
pub fn by_name(name: &str) -> Option<ScenarioConfig> {
    match name {
        "three_rooks_bound2_mb1" => Some(three_rooks_bound2_mb1()),
        "two_rooks_bound7" => Some(two_rooks_bound7()),
        _ => None,
    }
}

/// Names of all built-in scenarios.
pub fn names() -> &'static [&'static str] {
    &["three_rooks_bound2_mb1", "two_rooks_bound7"]
}

pub fn available_names() -> &'static [&'static str] {
    names()
}
