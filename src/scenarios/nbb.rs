//! Data-backed NBB scenario.
//!
//! This scenario loads a precomputed candidate set from `tests/data/kNBB_20_3_2.5_23.txt`,
//! originally produced by an external script. It is useful as a regression / existence proof
//! that “bishops + knight can trap the king” under a bounded slider model.
//!
//! ⚠ Move-bound conventions:
//! - `InfiniteChessEndgameScripts/infinite_tablebase.py` uses an **exclusive** rider bound
//!   (`step < move_bound`).
//! - `InfiniteChessEndgameScripts/trap_tester.py` uses an **inclusive** rider bound
//!   (`step <= move_bound`).
//!
//! The StackExchange write-up for this trap suggests testing it via
//! `play_vs_trap(load_trap("kNBB_20_3_2.5_23.txt"), 22, ...)`, so this scenario uses
//! `move_bound=22` under Rust's inclusive semantics.
//!
//! The file encodes **absolute** coordinates, so this scenario sets `track_abs_king=true` and
//! stores the black king anchor in [`State::abs_king`](crate::scenario::State::abs_king).

use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::chess::layout::PieceLayout;
use crate::chess::rules::Rules;
use crate::core::coord::Coord;
use crate::core::position::{Position, MAX_PIECES};
use crate::core::square::Square;
use crate::scenario::{
    CacheMode, CandidateGeneration, NoLaws, NoPreferences, ResourceLimits, Scenario, SearchError,
    Side, StartState, State,
};

use super::BuiltinDomain;

pub fn nbb20_from_file() -> Result<Scenario<BuiltinDomain, NoLaws, NoPreferences>, SearchError> {
    let path = default_trap_file_path();

    let layout = PieceLayout::from_counts(false, 0, 0, 2, 1); // B B N
    let rules = Rules::new(layout.clone(), 22);

    let states = parse_k_nbb_trap_file(&path, &layout, &rules).or_else(|e| {
        let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("BBN_script")
            .join("InfiniteChessEndgameScripts")
            .join("kNBB_20_3_2.5_23.txt");
        if fallback != path {
            parse_k_nbb_trap_file(&fallback, &layout, &rules)
        } else {
            Err(e)
        }
    })?;

    let start = states
        .first()
        .cloned()
        .ok_or_else(|| SearchError::InvalidScenario {
            reason: "NBB trap file parsed to an empty set".to_string(),
        })?;

    Ok(Scenario {
        name: "nbb20_from_file",
        rules,
        white_can_pass: true,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: start,
        },
        candidates: CandidateGeneration::FromStates { states },
        domain: BuiltinDomain::All,
        laws: NoLaws,
        preferences: NoPreferences,
        limits: nbb_limits(),
        cache_mode: CacheMode::BlackOnly,
        remove_stalemates: true,
    })
}

fn default_trap_file_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("kNBB_20_3_2.5_23.txt")
}

fn nbb_limits() -> ResourceLimits {
    ResourceLimits {
        // Tempo-trap (Büchi) construction introduces additional "white nodes" (states after a black
        // king move). With up to 8 king moves, the graph can be several times larger than the
        // underlying black candidate/trap set; keep this high enough for `nbb20_from_file`.
        max_states: 2_000_000,
        max_edges: 400_000_000,
        max_cache_entries: 250_000,
        max_cached_moves: 3_000_000,
        max_runtime_steps: 500_000_000,
    }
}

pub fn nbb7_generated() -> Result<Scenario<BuiltinDomain, NoLaws, NoPreferences>, SearchError> {
    let n = 7;
    let edge_size = 3;
    let knight_bound = 2.5;
    let move_bound = n + 2; // mirrors the typical `move_bound = n+2` convention in the scripts.

    let layout = PieceLayout::from_counts(false, 0, 0, 2, 1); // B B N
    let rules = Rules::new(layout.clone(), move_bound);

    let states = generate_potential_nbb_traps(n, edge_size, knight_bound, &layout, &rules);

    let start_pos = pos_from_rel_coords(
        &layout,
        &[Coord::new(1, 1), Coord::new(2, 2), Coord::new(1, 2)],
    );

    // Sanity: ensure the chosen start is legal under the pure rules, otherwise validate() fails.
    if !rules.is_legal_position(&start_pos) {
        return Err(SearchError::InvalidScenario {
            reason: "nbb7_generated: internal start position is not legal".to_string(),
        });
    }

    Ok(Scenario {
        name: "nbb7_generated",
        rules,
        white_can_pass: true,
        track_abs_king: true,
        start: StartState {
            to_move: Side::Black,
            state: State::new(Coord::ORIGIN, start_pos),
        },
        candidates: CandidateGeneration::FromStates { states },
        domain: BuiltinDomain::All,
        laws: NoLaws,
        preferences: NoPreferences,
        limits: nbb7_limits(),
        cache_mode: CacheMode::BlackOnly,
        remove_stalemates: true,
    })
}

fn nbb7_limits() -> ResourceLimits {
    ResourceLimits {
        // `get_potential_nbb_traps(7, ...)` is large (≈1.6M states after basic legality +
        // canonicalization + dedup). Keep `max_states` comfortably above that so we can run the
        // full pipeline without immediately hitting a budget.
        max_states: 2_000_000,
        // The fixed-point pruning scan is expensive; keep this high enough for exploratory runs.
        max_edges: 4_000_000_000,
        max_cache_entries: 250_000,
        max_cached_moves: 3_000_000,
        max_runtime_steps: 4_000_000_000,
    }
}

fn pos_from_rel_coords(layout: &PieceLayout, coords: &[Coord]) -> Position {
    assert_eq!(layout.piece_count(), coords.len());
    let mut squares = [Square::NONE; MAX_PIECES];
    for (i, c) in coords.iter().copied().enumerate() {
        squares[i] = Square::from_coord(c);
    }
    let mut pos = Position::new(layout.piece_count(), squares);
    pos.canonicalize(layout);
    pos
}

fn generate_potential_nbb_traps(
    n: i32,
    edge_size: i32,
    knight_bound: f64,
    layout: &PieceLayout,
    rules: &Rules,
) -> Vec<State> {
    assert!(n >= 1);
    assert!(edge_size >= 0);
    assert!(edge_size <= n);

    let mut bishop_1: Vec<Coord> = Vec::new();
    let mut bishop_2: Vec<Coord> = Vec::new();

    let special_bishops = [
        Coord::new(1 - n, 0),
        Coord::new(n - 1, 0),
        Coord::new(0, 1 - n),
        Coord::new(0, n - 1),
    ];

    for x in -n..=n {
        for y in -n..=n {
            if x + y == n + 1 || x + y == -n - 1 {
                bishop_1.push(Coord::new(x, y));
            }
            if x - y == n + 1 || x - y == -n - 1 {
                bishop_2.push(Coord::new(x, y));
            }
        }
    }
    bishop_1.extend_from_slice(&special_bishops);
    bishop_2.extend_from_slice(&special_bishops);

    let mut all_bishops: Vec<(Coord, Coord)> = Vec::new();
    let mut bishops_corner: Vec<(Coord, Coord)> = Vec::new();

    for &b1 in bishop_1.iter() {
        let b1_corner = b1.x == n || b1.x == -n || b1.y == n || b1.y == -n;
        for &b2 in bishop_2.iter() {
            let has_boundary_coord = [b1.x, b1.y, b2.x, b2.y]
                .iter()
                .any(|&v| v == n || v == -n || v == n - 1 || v == 1 - n);
            if has_boundary_coord {
                all_bishops.push((b1, b2));
            }

            let b2_corner = b2.x == n || b2.x == -n || b2.y == n || b2.y == -n;
            if b1_corner && b2_corner {
                bishops_corner.push((b1, b2));
            }
        }
    }

    let mut knights: Vec<Coord> = Vec::new();
    let k_bound = n + 3;
    for x in -k_bound..=k_bound {
        for y in -k_bound..=k_bound {
            let c = Coord::new(x, y);
            if l1_norm(c) <= k_bound {
                knights.push(c);
            }
        }
    }

    let mut edge_kings: Vec<Coord> = Vec::new();
    let mut center_kings: Vec<Coord> = Vec::new();
    for x in -n..=n {
        for y in -n..=n {
            let c = Coord::new(x, y);
            let l1 = l1_norm(c);
            if n - edge_size <= l1 && l1 <= n {
                edge_kings.push(c);
            } else if l1 < n - edge_size {
                center_kings.push(c);
            }
        }
    }

    let mut out: Vec<State> = Vec::new();
    let mut seen: FxHashSet<State> = FxHashSet::default();

    let mut squares = [Square::NONE; MAX_PIECES];

    let mut push_state = |abs_king: Coord, knight: Coord, b1: Coord, b2: Coord| {
        // Convert absolute coords to king-relative coords.
        let n_rel = knight - abs_king;
        let b1_rel = b1 - abs_king;
        let b2_rel = b2 - abs_king;

        // Layout is B,B,N.
        squares[0] = Square::from_coord(b1_rel);
        squares[1] = Square::from_coord(b2_rel);
        squares[2] = Square::from_coord(n_rel);
        for sq in squares[layout.piece_count()..].iter_mut() {
            *sq = Square::NONE;
        }

        let mut pos = Position::new(layout.piece_count(), squares);
        pos.canonicalize(layout);
        if !rules.is_legal_position(&pos) {
            return;
        }

        let s = State::new(abs_king, pos);
        if seen.insert(s.clone()) {
            out.push(s);
        }
    };

    // Center kings use only `bishops_corner`.
    for &k in center_kings.iter() {
        for &kn in knights.iter() {
            if dist_knight_norm(k, kn) < knight_bound {
                for &(b1, b2) in bishops_corner.iter() {
                    push_state(k, kn, b1, b2);
                }
            }
        }
    }

    // Edge kings use `all_bishops`.
    for &k in edge_kings.iter() {
        for &kn in knights.iter() {
            if dist_knight_norm(k, kn) < knight_bound {
                for &(b1, b2) in all_bishops.iter() {
                    push_state(k, kn, b1, b2);
                }
            }
        }
    }

    out
}

#[inline]
fn l1_norm(c: Coord) -> i32 {
    c.x.abs() + c.y.abs()
}

#[inline]
fn dist_knight_norm(a: Coord, b: Coord) -> f64 {
    knight_norm(a.x - b.x, a.y - b.y)
}

// Ported from `trap_tester.py`:
// - use a custom "knight metric" to bound candidate placements.
#[inline]
fn knight_norm(dx: i32, dy: i32) -> f64 {
    let r = (dx.abs()) as f64;
    let s = (dy.abs()) as f64;
    let mx = r.max(s);
    let mn = r.min(s);
    if 2.0 * mn < mx {
        mx / 2.0
    } else {
        (r + s) / 3.0
    }
}

fn parse_k_nbb_trap_file(
    path: &Path,
    layout: &PieceLayout,
    rules: &Rules,
) -> Result<Vec<State>, SearchError> {
    let bytes = std::fs::read(path).map_err(|e| SearchError::Io {
        stage: "nbb_read",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;

    // Streaming-ish parser: scan all integers and process in 8-tuples:
    // (king_x, king_y, knight_x, knight_y, bishop1_x, bishop1_y, bishop2_x, bishop2_y)
    let mut out: Vec<State> = Vec::new();
    let mut buf = [0i32; 8];
    let mut buf_len = 0usize;

    let mut in_num = false;
    let mut sign: i32 = 1;
    let mut acc: i32 = 0;

    let push_int = |x: i32, out: &mut Vec<State>, buf: &mut [i32; 8], buf_len: &mut usize| {
        buf[*buf_len] = x;
        *buf_len += 1;
        if *buf_len == 8 {
            let kx = buf[0];
            let ky = buf[1];
            let nx = buf[2];
            let ny = buf[3];
            let b1x = buf[4];
            let b1y = buf[5];
            let b2x = buf[6];
            let b2y = buf[7];

            let abs_king = Coord::new(kx, ky);
            let n_rel = Coord::new(nx - kx, ny - ky);
            let b1_rel = Coord::new(b1x - kx, b1y - ky);
            let b2_rel = Coord::new(b2x - kx, b2y - ky);

            let mut squares = [Square::NONE; MAX_PIECES];
            // Layout is B,B,N.
            squares[0] = Square::from_coord(b1_rel);
            squares[1] = Square::from_coord(b2_rel);
            squares[2] = Square::from_coord(n_rel);

            let mut pos = Position::new(layout.piece_count(), squares);
            pos.canonicalize(layout);

            if rules.is_legal_position(&pos) {
                out.push(State::new(abs_king, pos));
            }

            *buf_len = 0;
        }
    };

    for &b in bytes.iter() {
        match b {
            b'-' => {
                in_num = true;
                sign = -1;
                acc = 0;
            }
            b'0'..=b'9' => {
                if !in_num {
                    in_num = true;
                    sign = 1;
                    acc = 0;
                }
                acc = acc.saturating_mul(10).saturating_add((b - b'0') as i32);
            }
            _ => {
                if in_num {
                    push_int(sign * acc, &mut out, &mut buf, &mut buf_len);
                    in_num = false;
                    sign = 1;
                    acc = 0;
                }
            }
        }
    }

    if in_num {
        push_int(sign * acc, &mut out, &mut buf, &mut buf_len);
    }

    Ok(out)
}
