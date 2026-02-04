//! Exportable “solved scenario” bundles and loaders.
//!
//! A solution bundle is intended to be:
//! - **stable**: it stores enough information to be replayed even if the built-in scenario code
//!   changes,
//! - **compact**: JSON for human-readable metadata + a dense binary blob for large tables, and
//! - **fast to play**: the interactive CLI consumes *precomputed* legality and strategies.
//!
//! See `src/bin/export_solution.rs` and `src/bin/play_solution.rs` for the user-facing tools.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::chess::layout::PieceLayout;
use crate::chess::piece::PieceKind;
use crate::chess::rules::Rules;
use crate::core::coord::Coord;
use crate::core::position::{Position, MAX_PIECES};
use crate::core::square::Square;
use crate::scenario::{
    CandidateGeneration, DomainLike, LawsLike, PreferencesLike, Scenario, SearchError, Side, State,
};
use crate::search::buchi::tempo_trap_buchi_with_strategy;
use crate::search::strategy::extract_white_stay_strategy;
use crate::search::trap::maximal_inescapable_trap;

const FORMAT_VERSION: u32 = 1;
const MANIFEST_FILENAME: &str = "manifest.json";
const DATA_FILENAME: &str = "data.bin";
const DATA_MAGIC: &[u8; 8] = b"ICHSOL01";

#[derive(Debug, Clone, Copy)]
pub struct ExportOptions {
    /// If the output directory already exists, remove it first.
    pub force: bool,
    /// Compute tempo trap + tempo strategy (otherwise export only the inescapable trap).
    pub compute_tempo: bool,
    /// Override the recommended relative view bound stored in the manifest.
    pub view_bound: Option<i32>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            force: false,
            compute_tempo: true,
            view_bound: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionManifest {
    pub format_version: u32,
    pub created_unix_secs: u64,
    pub scenario_name: String,
    pub rules: RulesManifest,
    pub params: ParamsManifest,
    pub start: StartManifest,
    pub view: ViewManifest,
    pub counts: CountsManifest,
    pub files: FilesManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesManifest {
    pub white_king: bool,
    pub queens: u16,
    pub rooks: u16,
    pub bishops: u16,
    pub knights: u16,
    pub move_bound: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamsManifest {
    pub white_can_pass: bool,
    pub track_abs_king: bool,
    pub remove_stalemates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewManifest {
    pub default_mode: ViewMode,
    pub recommended_bound: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountsManifest {
    pub states: u32,
    pub trap: u32,
    pub tempo: u32,
    pub trap_strategy: u32,
    pub tempo_strategy: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesManifest {
    pub data_bin: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManifestSide {
    Black,
    White,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartManifest {
    pub to_move: ManifestSide,
    pub state_id: u32,
}

#[derive(Debug, Clone)]
pub struct SolutionData {
    pub states: Vec<State>,
    pub trap_set_ids: Vec<u32>,
    pub tempo_set_ids: Vec<u32>,
    pub transitions: Vec<(u32, [u32; 8])>,
    pub strategy_trap: Vec<(u32, u32)>,
    pub strategy_tempo: Vec<(u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct SolutionBundle {
    pub manifest: SolutionManifest,
    pub data: SolutionData,
}

/// Loaded bundle plus convenience indices for fast interactive play.
#[derive(Debug, Clone)]
pub struct LoadedSolution {
    pub manifest: SolutionManifest,
    pub rules: Rules,
    pub states: Vec<State>,
    pub id_of: FxHashMap<State, u32>,
    pub trap_ids: FxHashSet<u32>,
    pub tempo_ids: FxHashSet<u32>,
    pub transitions: Vec<[u32; 8]>,
    pub strat_trap: FxHashMap<u32, u32>,
    pub strat_tempo: FxHashMap<u32, u32>,
}

pub fn export_bundle<D, L, P>(
    scn: &Scenario<D, L, P>,
    out_dir: &Path,
    options: ExportOptions,
) -> Result<SolutionBundle, SearchError>
where
    D: DomainLike,
    L: LawsLike,
    P: PreferencesLike,
{
    scn.validate()?;
    if scn.start.to_move != Side::Black {
        return Err(SearchError::InvalidScenario {
            reason: "export_bundle currently requires start.to_move == Black".to_string(),
        });
    }

    prepare_output_dir(out_dir, options.force)?;

    let trap = maximal_inescapable_trap(scn)?;

    let (tempo, tempo_strategy) = if options.compute_tempo {
        tempo_trap_buchi_with_strategy(scn, &trap)?
    } else {
        (FxHashSet::default(), FxHashMap::default())
    };

    let play_start =
        if trap.contains(&scn.start.state) && has_any_legal_black_move(scn, &scn.start.state) {
            scn.start.state.clone()
        } else {
            choose_play_start(scn, &trap, &tempo)?
        };

    let trap_strategy = extract_white_stay_strategy(scn, &trap)?;

    let created_unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let rules_manifest = rules_manifest_from_rules(&scn.rules);
    let params_manifest = ParamsManifest {
        white_can_pass: scn.white_can_pass,
        track_abs_king: scn.track_abs_king,
        remove_stalemates: scn.remove_stalemates,
    };

    // Intern states (assign stable ids within this bundle).
    let mut states: Vec<State> = Vec::new();
    let mut id_of: FxHashMap<State, u32> = FxHashMap::default();

    let start_id = intern_state(&mut states, &mut id_of, play_start)?;

    let mut trap_set_ids: Vec<u32> = Vec::with_capacity(trap.len());
    for s in trap.iter() {
        let id = intern_state(&mut states, &mut id_of, s.clone())?;
        trap_set_ids.push(id);
    }

    // Build deterministic transitions for every trap black node.
    let mut transitions: Vec<(u32, [u32; 8])> = Vec::with_capacity(trap_set_ids.len());
    for b in trap.iter() {
        let b_id = *id_of.get(b).expect("trap states were interned");
        let mut next = [u32::MAX; 8];

        for (delta, pos2) in scn.rules.black_moves_with_delta(&b.pos) {
            let to = State {
                abs_king: if scn.track_abs_king {
                    b.abs_king + delta
                } else {
                    b.abs_king
                },
                pos: pos2,
            };

            if !scn.laws.allow_black_move(b, &to, delta) {
                continue;
            }
            if !scn.laws.allow_state(&to) {
                continue;
            }

            let Some(dir) = dir_index(delta) else {
                continue;
            };

            let w_id = intern_state(&mut states, &mut id_of, to)?;
            next[dir] = w_id;
        }

        transitions.push((b_id, next));
    }

    // Convert strategies to (id,id) pairs and ensure all referenced states are interned.
    let mut strategy_trap: Vec<(u32, u32)> = Vec::with_capacity(trap_strategy.len());
    for (w, b) in trap_strategy.into_iter() {
        let w_id = intern_state(&mut states, &mut id_of, w)?;
        let b_id = intern_state(&mut states, &mut id_of, b)?;
        strategy_trap.push((w_id, b_id));
    }

    let mut strategy_tempo: Vec<(u32, u32)> = Vec::with_capacity(tempo_strategy.len());
    for (w, b) in tempo_strategy.into_iter() {
        let w_id = intern_state(&mut states, &mut id_of, w)?;
        let b_id = intern_state(&mut states, &mut id_of, b)?;
        strategy_tempo.push((w_id, b_id));
    }

    let mut tempo_set_ids: Vec<u32> = Vec::with_capacity(tempo.len());
    for s in tempo.iter() {
        let id = intern_state(&mut states, &mut id_of, s.clone())?;
        tempo_set_ids.push(id);
    }

    let mut recommended_bound = match scn.candidates {
        CandidateGeneration::InLinfBound { bound, .. } => bound,
        _ => compute_recommended_bound(&trap).max(2),
    };
    if let Some(b) = options.view_bound {
        recommended_bound = b;
    }

    let counts = CountsManifest {
        states: to_u32_len(states.len(), "manifest_counts_states")?,
        trap: to_u32_len(trap_set_ids.len(), "manifest_counts_trap")?,
        tempo: to_u32_len(tempo_set_ids.len(), "manifest_counts_tempo")?,
        trap_strategy: to_u32_len(strategy_trap.len(), "manifest_counts_trap_strategy")?,
        tempo_strategy: to_u32_len(strategy_tempo.len(), "manifest_counts_tempo_strategy")?,
    };

    let manifest = SolutionManifest {
        format_version: FORMAT_VERSION,
        created_unix_secs,
        scenario_name: scn.name.to_string(),
        rules: rules_manifest,
        params: params_manifest,
        start: StartManifest {
            to_move: ManifestSide::Black,
            state_id: start_id,
        },
        view: ViewManifest {
            default_mode: ViewMode::Relative,
            recommended_bound,
        },
        counts,
        files: FilesManifest {
            data_bin: DATA_FILENAME.to_string(),
        },
    };

    let data = SolutionData {
        states,
        trap_set_ids,
        tempo_set_ids,
        transitions,
        strategy_trap,
        strategy_tempo,
    };

    write_manifest(out_dir, &manifest)?;
    write_data(out_dir, &manifest, &data)?;

    Ok(SolutionBundle { manifest, data })
}

pub fn load_bundle(bundle_dir: &Path) -> Result<LoadedSolution, SearchError> {
    let manifest = read_manifest(bundle_dir)?;

    if manifest.format_version != FORMAT_VERSION {
        return Err(SearchError::InvalidScenario {
            reason: format!(
                "unsupported solution format_version {} (expected {FORMAT_VERSION})",
                manifest.format_version
            ),
        });
    }

    let rules = rules_from_manifest(&manifest.rules)?;
    let piece_count = rules.layout.piece_count();

    let data = read_data(bundle_dir, piece_count)?;

    // Build indices for fast access.
    let mut id_of: FxHashMap<State, u32> = FxHashMap::default();
    id_of.reserve(data.states.len());
    for (i, s) in data.states.iter().cloned().enumerate() {
        let id = u32::try_from(i).map_err(|_| SearchError::InvalidScenario {
            reason: "solution has too many states for u32 indexing".to_string(),
        })?;
        id_of.insert(s, id);
    }

    let trap_ids: FxHashSet<u32> = data.trap_set_ids.iter().copied().collect();
    let tempo_ids: FxHashSet<u32> = data.tempo_set_ids.iter().copied().collect();

    let mut transitions: Vec<[u32; 8]> = vec![[u32::MAX; 8]; data.states.len()];
    for (state_id, next) in data.transitions.into_iter() {
        let idx = usize::try_from(state_id).map_err(|_| SearchError::InvalidScenario {
            reason: "transition state_id does not fit usize".to_string(),
        })?;
        if idx >= transitions.len() {
            return Err(SearchError::InvalidScenario {
                reason: format!("transition references out-of-range state_id {state_id}"),
            });
        }
        transitions[idx] = next;
    }

    let strat_trap: FxHashMap<u32, u32> = data.strategy_trap.into_iter().collect();
    let strat_tempo: FxHashMap<u32, u32> = data.strategy_tempo.into_iter().collect();

    Ok(LoadedSolution {
        manifest,
        rules,
        states: data.states,
        id_of,
        trap_ids,
        tempo_ids,
        transitions,
        strat_trap,
        strat_tempo,
    })
}

fn prepare_output_dir(out_dir: &Path, force: bool) -> Result<(), SearchError> {
    if out_dir.exists() {
        if !force {
            return Err(SearchError::Io {
                stage: "solution_export_create_dir",
                path: out_dir.display().to_string(),
                error: "output directory already exists (use --force to overwrite)".to_string(),
            });
        }
        fs::remove_dir_all(out_dir).map_err(|e| SearchError::Io {
            stage: "solution_export_remove_dir",
            path: out_dir.display().to_string(),
            error: e.to_string(),
        })?;
    }

    fs::create_dir_all(out_dir).map_err(|e| SearchError::Io {
        stage: "solution_export_create_dir",
        path: out_dir.display().to_string(),
        error: e.to_string(),
    })
}

fn write_manifest(out_dir: &Path, manifest: &SolutionManifest) -> Result<(), SearchError> {
    let path = out_dir.join(MANIFEST_FILENAME);
    let f = fs::File::create(&path).map_err(|e| SearchError::Io {
        stage: "solution_export_manifest_create",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    let mut w = BufWriter::new(f);
    serde_json::to_writer_pretty(&mut w, manifest).map_err(|e| SearchError::Io {
        stage: "solution_export_manifest_serialize",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    w.flush().map_err(|e| SearchError::Io {
        stage: "solution_export_manifest_flush",
        path: path.display().to_string(),
        error: e.to_string(),
    })
}

fn read_manifest(bundle_dir: &Path) -> Result<SolutionManifest, SearchError> {
    let path = bundle_dir.join(MANIFEST_FILENAME);
    let f = fs::File::open(&path).map_err(|e| SearchError::Io {
        stage: "solution_load_manifest_open",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    let r = BufReader::new(f);
    serde_json::from_reader(r).map_err(|e| SearchError::Io {
        stage: "solution_load_manifest_parse",
        path: path.display().to_string(),
        error: e.to_string(),
    })
}

fn write_data(
    out_dir: &Path,
    manifest: &SolutionManifest,
    data: &SolutionData,
) -> Result<(), SearchError> {
    let path = out_dir.join(&manifest.files.data_bin);
    let f = fs::File::create(&path).map_err(|e| SearchError::Io {
        stage: "solution_export_data_create",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    let mut w = BufWriter::new(f);

    let piece_count = piece_count_from_manifest(&manifest.rules)? as u32;

    w.write_all(DATA_MAGIC).map_err(|e| SearchError::Io {
        stage: "solution_export_data_write",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    write_u32(
        &mut w,
        manifest.format_version,
        "solution_export_data_write",
        &path,
    )?;
    write_u32(&mut w, piece_count, "solution_export_data_write", &path)?;

    write_u32(
        &mut w,
        to_u32_len(data.states.len(), "solution_export_states_len")?,
        "solution_export_data_write",
        &path,
    )?;
    for s in data.states.iter() {
        write_i32(&mut w, s.abs_king.x, "solution_export_data_write", &path)?;
        write_i32(&mut w, s.abs_king.y, "solution_export_data_write", &path)?;
        for &sq in s.pos.squares().iter() {
            write_i64(&mut w, sq.raw(), "solution_export_data_write", &path)?;
        }
    }

    write_u32(
        &mut w,
        to_u32_len(data.trap_set_ids.len(), "solution_export_trap_len")?,
        "solution_export_data_write",
        &path,
    )?;
    for &id in data.trap_set_ids.iter() {
        write_u32(&mut w, id, "solution_export_data_write", &path)?;
    }

    write_u32(
        &mut w,
        to_u32_len(data.tempo_set_ids.len(), "solution_export_tempo_len")?,
        "solution_export_data_write",
        &path,
    )?;
    for &id in data.tempo_set_ids.iter() {
        write_u32(&mut w, id, "solution_export_data_write", &path)?;
    }

    write_u32(
        &mut w,
        to_u32_len(data.transitions.len(), "solution_export_transitions_len")?,
        "solution_export_data_write",
        &path,
    )?;
    for (state_id, next) in data.transitions.iter() {
        write_u32(&mut w, *state_id, "solution_export_data_write", &path)?;
        for &dst in next.iter() {
            write_u32(&mut w, dst, "solution_export_data_write", &path)?;
        }
    }

    write_u32(
        &mut w,
        to_u32_len(
            data.strategy_trap.len(),
            "solution_export_strategy_trap_len",
        )?,
        "solution_export_data_write",
        &path,
    )?;
    for (w_id, b_id) in data.strategy_trap.iter() {
        write_u32(&mut w, *w_id, "solution_export_data_write", &path)?;
        write_u32(&mut w, *b_id, "solution_export_data_write", &path)?;
    }

    write_u32(
        &mut w,
        to_u32_len(
            data.strategy_tempo.len(),
            "solution_export_strategy_tempo_len",
        )?,
        "solution_export_data_write",
        &path,
    )?;
    for (w_id, b_id) in data.strategy_tempo.iter() {
        write_u32(&mut w, *w_id, "solution_export_data_write", &path)?;
        write_u32(&mut w, *b_id, "solution_export_data_write", &path)?;
    }

    w.flush().map_err(|e| SearchError::Io {
        stage: "solution_export_data_flush",
        path: path.display().to_string(),
        error: e.to_string(),
    })
}

fn read_data(bundle_dir: &Path, piece_count: usize) -> Result<SolutionData, SearchError> {
    let path = bundle_dir.join(DATA_FILENAME);
    let f = fs::File::open(&path).map_err(|e| SearchError::Io {
        stage: "solution_load_data_open",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    let mut r = BufReader::new(f);

    let mut magic = [0u8; 8];
    r.read_exact(&mut magic).map_err(|e| SearchError::Io {
        stage: "solution_load_data_read",
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    if &magic != DATA_MAGIC {
        return Err(SearchError::InvalidScenario {
            reason: "solution data.bin has wrong magic bytes".to_string(),
        });
    }

    let version = read_u32(&mut r, "solution_load_data_read", &path)?;
    if version != FORMAT_VERSION {
        return Err(SearchError::InvalidScenario {
            reason: format!("solution data.bin version {version} is not supported"),
        });
    }

    let file_piece_count = read_u32(&mut r, "solution_load_data_read", &path)? as usize;
    if file_piece_count != piece_count {
        return Err(SearchError::InvalidScenario {
            reason: format!(
                "solution data.bin piece_count {file_piece_count} mismatches manifest {piece_count}"
            ),
        });
    }

    let states_len = read_u32(&mut r, "solution_load_data_read", &path)? as usize;
    let mut states: Vec<State> = Vec::with_capacity(states_len);
    for _ in 0..states_len {
        let x = read_i32(&mut r, "solution_load_data_read", &path)?;
        let y = read_i32(&mut r, "solution_load_data_read", &path)?;

        let mut squares = [Square::NONE; MAX_PIECES];
        for square in squares.iter_mut().take(piece_count) {
            let raw = read_i64(&mut r, "solution_load_data_read", &path)?;
            *square = Square::from_raw(raw);
        }

        let pos = Position::new(piece_count, squares);
        states.push(State {
            abs_king: Coord::new(x, y),
            pos,
        });
    }

    let trap_len = read_u32(&mut r, "solution_load_data_read", &path)? as usize;
    let mut trap_set_ids = Vec::with_capacity(trap_len);
    for _ in 0..trap_len {
        trap_set_ids.push(read_u32(&mut r, "solution_load_data_read", &path)?);
    }

    let tempo_len = read_u32(&mut r, "solution_load_data_read", &path)? as usize;
    let mut tempo_set_ids = Vec::with_capacity(tempo_len);
    for _ in 0..tempo_len {
        tempo_set_ids.push(read_u32(&mut r, "solution_load_data_read", &path)?);
    }

    let transitions_len = read_u32(&mut r, "solution_load_data_read", &path)? as usize;
    let mut transitions: Vec<(u32, [u32; 8])> = Vec::with_capacity(transitions_len);
    for _ in 0..transitions_len {
        let state_id = read_u32(&mut r, "solution_load_data_read", &path)?;
        let mut next = [u32::MAX; 8];
        for d in next.iter_mut() {
            *d = read_u32(&mut r, "solution_load_data_read", &path)?;
        }
        transitions.push((state_id, next));
    }

    let strategy_trap_len = read_u32(&mut r, "solution_load_data_read", &path)? as usize;
    let mut strategy_trap: Vec<(u32, u32)> = Vec::with_capacity(strategy_trap_len);
    for _ in 0..strategy_trap_len {
        let w_id = read_u32(&mut r, "solution_load_data_read", &path)?;
        let b_id = read_u32(&mut r, "solution_load_data_read", &path)?;
        strategy_trap.push((w_id, b_id));
    }

    let strategy_tempo_len = read_u32(&mut r, "solution_load_data_read", &path)? as usize;
    let mut strategy_tempo: Vec<(u32, u32)> = Vec::with_capacity(strategy_tempo_len);
    for _ in 0..strategy_tempo_len {
        let w_id = read_u32(&mut r, "solution_load_data_read", &path)?;
        let b_id = read_u32(&mut r, "solution_load_data_read", &path)?;
        strategy_tempo.push((w_id, b_id));
    }

    Ok(SolutionData {
        states,
        trap_set_ids,
        tempo_set_ids,
        transitions,
        strategy_trap,
        strategy_tempo,
    })
}

fn rules_manifest_from_rules(rules: &Rules) -> RulesManifest {
    let mut queens = 0u16;
    let mut rooks = 0u16;
    let mut bishops = 0u16;
    let mut knights = 0u16;
    for &k in rules.layout.kinds() {
        match k {
            PieceKind::King => {}
            PieceKind::Queen => queens += 1,
            PieceKind::Rook => rooks += 1,
            PieceKind::Bishop => bishops += 1,
            PieceKind::Knight => knights += 1,
        }
    }

    RulesManifest {
        white_king: rules.layout.white_king_index().is_some(),
        queens,
        rooks,
        bishops,
        knights,
        move_bound: rules.move_bound,
    }
}

fn piece_count_from_manifest(rules: &RulesManifest) -> Result<usize, SearchError> {
    let mut n = rules.queens as usize
        + rules.rooks as usize
        + rules.bishops as usize
        + rules.knights as usize;
    if rules.white_king {
        n += 1;
    }
    if n > MAX_PIECES {
        return Err(SearchError::InvalidScenario {
            reason: format!("manifest piece_count {n} exceeds MAX_PIECES={MAX_PIECES}"),
        });
    }
    Ok(n)
}

fn rules_from_manifest(rules: &RulesManifest) -> Result<Rules, SearchError> {
    let layout = PieceLayout::from_counts(
        rules.white_king,
        rules.queens as usize,
        rules.rooks as usize,
        rules.bishops as usize,
        rules.knights as usize,
    );

    let count = layout.piece_count();
    let want = piece_count_from_manifest(rules)?;
    if count != want {
        return Err(SearchError::InvalidScenario {
            reason: format!("layout piece_count {count} mismatches manifest {want}"),
        });
    }

    Ok(Rules::new(layout, rules.move_bound))
}

fn intern_state(
    states: &mut Vec<State>,
    id_of: &mut FxHashMap<State, u32>,
    s: State,
) -> Result<u32, SearchError> {
    if let Some(&id) = id_of.get(&s) {
        return Ok(id);
    }
    let id = u32::try_from(states.len()).map_err(|_| SearchError::InvalidScenario {
        reason: "too many states to index with u32".to_string(),
    })?;
    states.push(s.clone());
    id_of.insert(s, id);
    Ok(id)
}

fn compute_recommended_bound(trap: &FxHashSet<State>) -> i32 {
    let mut max_norm = 0i32;
    for s in trap.iter() {
        for &sq in s.pos.squares() {
            if sq.is_none() {
                continue;
            }
            max_norm = max_norm.max(sq.coord().chebyshev_norm());
        }
    }
    max_norm
}

fn choose_play_start<D, L, P>(
    scn: &Scenario<D, L, P>,
    trap: &FxHashSet<State>,
    tempo: &FxHashSet<State>,
) -> Result<State, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    if trap.is_empty() {
        return Err(SearchError::InvalidScenario {
            reason: "cannot export: inescapable trap is empty".to_string(),
        });
    }

    // Prefer a state inside the tempo trap if available (tends to give longer interactive play).
    let primary = if !tempo.is_empty() { tempo } else { trap };

    for s in primary.iter() {
        if has_any_legal_black_move(scn, s) {
            return Ok(s.clone());
        }
    }

    // Fall back to any trap state.
    Ok(trap.iter().next().expect("trap non-empty").clone())
}

fn has_any_legal_black_move<D, L, P>(scn: &Scenario<D, L, P>, b: &State) -> bool
where
    D: DomainLike,
    L: LawsLike,
{
    for (delta, pos2) in scn.rules.black_moves_with_delta(&b.pos) {
        let to = State {
            abs_king: if scn.track_abs_king {
                b.abs_king + delta
            } else {
                b.abs_king
            },
            pos: pos2,
        };
        if scn.laws.allow_black_move(b, &to, delta) && scn.laws.allow_state(&to) {
            return true;
        }
    }
    false
}

fn to_u32_len(len: usize, stage: &'static str) -> Result<u32, SearchError> {
    u32::try_from(len).map_err(|_| SearchError::InvalidScenario {
        reason: format!("{stage}: length {len} does not fit u32"),
    })
}

fn dir_index(delta: Coord) -> Option<usize> {
    match (delta.x, delta.y) {
        (-1, 1) => Some(0),  // q
        (0, 1) => Some(1),   // w
        (1, 1) => Some(2),   // e
        (-1, 0) => Some(3),  // a
        (1, 0) => Some(4),   // d
        (-1, -1) => Some(5), // z
        (0, -1) => Some(6),  // x
        (1, -1) => Some(7),  // c
        _ => None,
    }
}

fn write_u32(
    w: &mut dyn Write,
    v: u32,
    stage: &'static str,
    path: &Path,
) -> Result<(), SearchError> {
    w.write_all(&v.to_le_bytes()).map_err(|e| SearchError::Io {
        stage,
        path: path.display().to_string(),
        error: e.to_string(),
    })
}

fn write_i32(
    w: &mut dyn Write,
    v: i32,
    stage: &'static str,
    path: &Path,
) -> Result<(), SearchError> {
    w.write_all(&v.to_le_bytes()).map_err(|e| SearchError::Io {
        stage,
        path: path.display().to_string(),
        error: e.to_string(),
    })
}

fn write_i64(
    w: &mut dyn Write,
    v: i64,
    stage: &'static str,
    path: &Path,
) -> Result<(), SearchError> {
    w.write_all(&v.to_le_bytes()).map_err(|e| SearchError::Io {
        stage,
        path: path.display().to_string(),
        error: e.to_string(),
    })
}

fn read_u32(r: &mut dyn Read, stage: &'static str, path: &Path) -> Result<u32, SearchError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf).map_err(|e| SearchError::Io {
        stage,
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i32(r: &mut dyn Read, stage: &'static str, path: &Path) -> Result<i32, SearchError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf).map_err(|e| SearchError::Io {
        stage,
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    Ok(i32::from_le_bytes(buf))
}

fn read_i64(r: &mut dyn Read, stage: &'static str, path: &Path) -> Result<i64, SearchError> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf).map_err(|e| SearchError::Io {
        stage,
        path: path.display().to_string(),
        error: e.to_string(),
    })?;
    Ok(i64::from_le_bytes(buf))
}

/// Return a human-readable mapping from direction indices to key labels (q,w,e,a,d,z,x,c).
pub fn direction_labels() -> &'static [char; 8] {
    &['q', 'w', 'e', 'a', 'd', 'z', 'x', 'c']
}

/// Return a mapping from direction keys to king-step deltas.
///
/// This is primarily used by the interactive CLI and mirrors the encoding in the solution bundle.
pub fn direction_deltas() -> BTreeMap<char, Coord> {
    // Keep this deterministic for help output.
    let mut m = BTreeMap::new();
    m.insert('q', Coord::new(-1, 1));
    m.insert('w', Coord::new(0, 1));
    m.insert('e', Coord::new(1, 1));
    m.insert('a', Coord::new(-1, 0));
    m.insert('d', Coord::new(1, 0));
    m.insert('z', Coord::new(-1, -1));
    m.insert('x', Coord::new(0, -1));
    m.insert('c', Coord::new(1, -1));
    m
}

pub fn dir_index_from_key(ch: char) -> Option<usize> {
    match ch {
        'q' => Some(0),
        'w' => Some(1),
        'e' => Some(2),
        'a' => Some(3),
        'd' => Some(4),
        'z' => Some(5),
        'x' => Some(6),
        'c' => Some(7),
        _ => None,
    }
}

pub fn delta_from_dir_index(idx: usize) -> Coord {
    match idx {
        0 => Coord::new(-1, 1),
        1 => Coord::new(0, 1),
        2 => Coord::new(1, 1),
        3 => Coord::new(-1, 0),
        4 => Coord::new(1, 0),
        5 => Coord::new(-1, -1),
        6 => Coord::new(0, -1),
        7 => Coord::new(1, -1),
        _ => Coord::ORIGIN,
    }
}

pub fn bundle_paths(bundle_dir: &Path) -> (PathBuf, PathBuf) {
    (
        bundle_dir.join(MANIFEST_FILENAME),
        bundle_dir.join(DATA_FILENAME),
    )
}
