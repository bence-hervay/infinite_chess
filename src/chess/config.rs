use crate::chess::layout::PieceLayout;
use crate::chess::piece::PieceKind;
use crate::chess::rules::Rules;

/// Scenario configuration (pure Rust, no JSON).
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    pub name: &'static str,

    /// L∞ bound for enumerating *candidate* black-to-move positions.
    pub bound: i32,

    /// Maximum distance a sliding piece may move in a single white move.
    pub move_bound: i32,

    /// Whether White may pass (gain tempo) on her move.
    pub white_can_pass: bool,

    /// Remove stalemates from the candidate set (recommended for trap search).
    pub remove_stalemates: bool,

    pub layout: PieceLayout,
}

impl ScenarioConfig {
    pub fn new(
        name: &'static str,
        bound: i32,
        move_bound: i32,
        white_can_pass: bool,
        remove_stalemates: bool,
        layout: PieceLayout,
    ) -> Self {
        Self {
            name,
            bound,
            move_bound,
            white_can_pass,
            remove_stalemates,
            layout,
        }
    }

    pub fn piece_summary(&self) -> String {
        // compact, deterministic order
        let mut counts: Vec<(PieceKind, usize)> = Vec::new();
        for k in self.layout.kinds() {
            if let Some((_, c)) = counts.last_mut().filter(|(kind, _)| *kind == *k) {
                *c += 1;
            } else {
                counts.push((*k, 1));
            }
        }
        counts
            .into_iter()
            .map(|(k, c)| format!("{:?}x{}", k, c))
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn rules(&self) -> Rules {
        Rules::new(self.layout.clone(), self.move_bound)
    }
}
