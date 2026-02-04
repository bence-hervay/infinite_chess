//! Data-backed NBB scenario.
//!
//! This scenario loads a precomputed candidate set from `tests/data/kNBB_20_3_2.5_23.txt`,
//! originally produced by an external script. It is useful as a regression / existence proof
//! that “bishops + knight can trap the king” under a bounded slider model.
//!
//! The file encodes **absolute** coordinates, so this scenario sets `track_abs_king=true` and
//! stores the black king anchor in [`State::abs_king`](crate::scenario::State::abs_king).

use std::path::{Path, PathBuf};

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
    let rules = Rules::new(layout.clone(), 23);

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
        max_states: 500_000,
        max_edges: 200_000_000,
        max_cache_entries: 250_000,
        max_cached_moves: 3_000_000,
        max_runtime_steps: 500_000_000,
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
