use std::io::{self, Write};
use std::path::Path;

use infinite_chess::chess::piece::PieceKind;
use infinite_chess::core::coord::Coord;
use infinite_chess::core::square::Square;
use infinite_chess::solution::{delta_from_dir_index, dir_index_from_key, load_bundle, ViewMode};

const MAX_ABS_DIM: i32 = 81;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: play_solution <bundle_dir> [--view relative|absolute] [--bound <B>]");
        std::process::exit(2);
    }

    let bundle_dir = Path::new(&args[1]);
    let mut view_override: Option<ViewMode> = None;
    let mut bound_override: Option<i32> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--view" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("--view requires 'relative' or 'absolute'");
                    std::process::exit(2);
                };
                view_override = match v.as_str() {
                    "relative" => Some(ViewMode::Relative),
                    "absolute" => Some(ViewMode::Absolute),
                    _ => {
                        eprintln!("--view requires 'relative' or 'absolute'");
                        std::process::exit(2);
                    }
                };
                i += 2;
            }
            "--bound" => {
                let Some(v) = args.get(i + 1) else {
                    eprintln!("--bound requires an integer argument");
                    std::process::exit(2);
                };
                bound_override = match v.parse::<i32>() {
                    Ok(b) => Some(b),
                    Err(e) => {
                        eprintln!("invalid --bound {v}: {e}");
                        std::process::exit(2);
                    }
                };
                i += 2;
            }
            x => {
                eprintln!("Unknown option: {x}");
                std::process::exit(2);
            }
        }
    }

    let sol = match load_bundle(bundle_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load bundle: {e}");
            std::process::exit(1);
        }
    };

    if sol.manifest.start.to_move != infinite_chess::solution::ManifestSide::Black {
        eprintln!("This interactive tool currently requires start.to_move == black.");
        std::process::exit(2);
    }

    let mut view = view_override.unwrap_or(sol.manifest.view.default_mode);
    let mut rel_bound = bound_override
        .unwrap_or(sol.manifest.view.recommended_bound)
        .max(1);

    let mut current_b_id = sol.manifest.start.state_id as usize;
    if current_b_id >= sol.states.len() {
        eprintln!("Invalid bundle: start.state_id is out of range.");
        std::process::exit(2);
    }

    let mut display_king = sol.states[current_b_id].abs_king;

    print_help();

    loop {
        let b_state = &sol.states[current_b_id];

        let legal_dirs = legal_dirs(&sol.transitions[current_b_id]);
        let in_tempo = sol.tempo_ids.contains(&(current_b_id as u32));
        let in_trap = sol.trap_ids.contains(&(current_b_id as u32));

        let in_check = sol
            .rules
            .is_attacked(infinite_chess::core::coord::Coord::ORIGIN, &b_state.pos);

        if in_check && legal_dirs.is_empty() {
            println!("Checkmate. White wins.");
            break;
        }

        render(
            &sol.rules.layout,
            b_state,
            display_king,
            view,
            rel_bound,
            &legal_dirs,
        );

        print!(
            "Black to move | trap:{} tempo:{} check:{} | moves:{} > ",
            yesno(in_trap),
            yesno(in_tempo),
            yesno(in_check),
            legal_dirs
                .iter()
                .map(|&d| dir_key(d).to_string())
                .collect::<Vec<_>>()
                .join("")
        );
        io::stdout().flush().ok();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }

        // Command mode.
        match cmd {
            "help" => {
                print_help();
                continue;
            }
            "exit" | "quit" | "q!" => break,
            "view relative" => {
                view = ViewMode::Relative;
                continue;
            }
            "view absolute" => {
                view = ViewMode::Absolute;
                continue;
            }
            _ => {}
        }

        let ch = cmd.chars().find(|c| !c.is_whitespace()).unwrap_or(' ');
        if ch == 's' {
            println!("'s' is the center key; black cannot pass. Use qwe/adzxc to move, or 'help'.");
            continue;
        }

        let Some(dir_idx) = dir_index_from_key(ch) else {
            println!("Unknown input '{cmd}'. Type 'help' for commands.");
            continue;
        };

        let w_id = sol.transitions[current_b_id][dir_idx];
        if w_id == u32::MAX {
            println!("Illegal move '{ch}' from this position.");
            continue;
        }

        // Apply black move.
        let delta = delta_from_dir_index(dir_idx);
        println!("Black: {ch} (delta {}, {})", delta.x, delta.y);

        display_king = display_king + delta;
        let w_id_usize = w_id as usize;
        if w_id_usize >= sol.states.len() {
            println!("Bundle error: transition to out-of-range state id {w_id}.");
            break;
        }
        if sol.manifest.params.track_abs_king {
            display_king = sol.states[w_id_usize].abs_king;
        }

        // White reply using saved strategy.
        let next_b = sol
            .strat_tempo
            .get(&w_id)
            .copied()
            .or_else(|| sol.strat_trap.get(&w_id).copied());

        let Some(next_b_id) = next_b else {
            println!("White has no saved response here. Black escaped the solved region.");
            break;
        };

        let next_b_id_usize = next_b_id as usize;
        if next_b_id_usize >= sol.states.len() {
            println!("Bundle error: white strategy returns out-of-range state id {next_b_id}.");
            break;
        }

        let w_state = &sol.states[w_id_usize];
        let b_next = &sol.states[next_b_id_usize];
        println!(
            "{}",
            describe_white_action(
                &sol.rules.layout,
                &w_state.pos,
                &b_next.pos,
                display_king,
                view
            )
        );

        current_b_id = next_b_id_usize;

        if view == ViewMode::Relative {
            // Keep relative view bound stable but let users enlarge it live if needed.
            rel_bound = rel_bound.max(sol.manifest.view.recommended_bound).max(1);
        }
    }
}

fn yesno(v: bool) -> &'static str {
    if v {
        "yes"
    } else {
        "no"
    }
}

fn dir_key(idx: usize) -> char {
    match idx {
        0 => 'q',
        1 => 'w',
        2 => 'e',
        3 => 'a',
        4 => 'd',
        5 => 'z',
        6 => 'x',
        7 => 'c',
        _ => '?',
    }
}

fn legal_dirs(next: &[u32; 8]) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, &v) in next.iter().enumerate() {
        if v != u32::MAX {
            out.push(i);
        }
    }
    out
}

fn piece_char(kind: PieceKind) -> char {
    match kind {
        PieceKind::King => 'K',
        PieceKind::Queen => 'Q',
        PieceKind::Rook => 'R',
        PieceKind::Bishop => 'B',
        PieceKind::Knight => 'N',
    }
}

fn render(
    layout: &infinite_chess::chess::layout::PieceLayout,
    state: &infinite_chess::scenario::State,
    display_king: Coord,
    mode: ViewMode,
    rel_bound: i32,
    legal_dirs: &[usize],
) {
    let (min_x, max_x, min_y, max_y, cropped) = match mode {
        ViewMode::Relative => (-rel_bound, rel_bound, -rel_bound, rel_bound, false),
        ViewMode::Absolute => compute_abs_window(layout, state, display_king, legal_dirs),
    };

    let w = (max_x - min_x + 1).max(1) as usize;
    let h = (max_y - min_y + 1).max(1) as usize;

    let mut grid: Vec<Vec<char>> = vec![vec!['-'; w]; h];

    // Pieces.
    for i in 0..state.pos.count() {
        let sq = state.pos.square(i);
        if sq.is_none() {
            continue;
        }
        let rel = sq.coord();
        let abs = match mode {
            ViewMode::Relative => rel,
            ViewMode::Absolute => display_king + rel,
        };

        if abs.x < min_x || abs.x > max_x || abs.y < min_y || abs.y > max_y {
            continue;
        }
        let gx = (abs.x - min_x) as usize;
        let gy = (max_y - abs.y) as usize;
        grid[gy][gx] = piece_char(layout.kind(i));
    }

    // Black king.
    let k_abs = match mode {
        ViewMode::Relative => Coord::ORIGIN,
        ViewMode::Absolute => display_king,
    };
    if k_abs.x >= min_x && k_abs.x <= max_x && k_abs.y >= min_y && k_abs.y <= max_y {
        let gx = (k_abs.x - min_x) as usize;
        let gy = (max_y - k_abs.y) as usize;
        grid[gy][gx] = 'k';
    }

    // Legal move overlay: '+' only on empty destination squares.
    for &dir in legal_dirs.iter() {
        let delta = delta_from_dir_index(dir);
        if !is_empty_destination(&state.pos, delta) {
            continue;
        }
        let dst = match mode {
            ViewMode::Relative => delta,
            ViewMode::Absolute => display_king + delta,
        };
        if dst.x < min_x || dst.x > max_x || dst.y < min_y || dst.y > max_y {
            continue;
        }
        let gx = (dst.x - min_x) as usize;
        let gy = (max_y - dst.y) as usize;
        if grid[gy][gx] == '-' {
            grid[gy][gx] = '+';
        }
    }

    // Capture list.
    let captures = capture_list(layout, &state.pos, display_king, mode, legal_dirs);

    println!();
    for row in grid {
        let line: String = row.into_iter().collect();
        println!("{line}");
    }

    if cropped {
        println!(
            "(absolute view cropped to {}x{} around king at ({}, {}))",
            w, h, display_king.x, display_king.y
        );
    }

    if !captures.is_empty() {
        println!("Captures: {}", captures.join(", "));
    }
    println!();
}

fn compute_abs_window(
    layout: &infinite_chess::chess::layout::PieceLayout,
    state: &infinite_chess::scenario::State,
    display_king: Coord,
    legal_dirs: &[usize],
) -> (i32, i32, i32, i32, bool) {
    let mut min_x = display_king.x;
    let mut max_x = display_king.x;
    let mut min_y = display_king.y;
    let mut max_y = display_king.y;

    for i in 0..state.pos.count() {
        let sq = state.pos.square(i);
        if sq.is_none() {
            continue;
        }
        let abs = display_king + sq.coord();
        min_x = min_x.min(abs.x);
        max_x = max_x.max(abs.x);
        min_y = min_y.min(abs.y);
        max_y = max_y.max(abs.y);
    }

    // Include legal destinations (for '+' overlay).
    for &dir in legal_dirs.iter() {
        let abs = display_king + delta_from_dir_index(dir);
        min_x = min_x.min(abs.x);
        max_x = max_x.max(abs.x);
        min_y = min_y.min(abs.y);
        max_y = max_y.max(abs.y);
    }

    // Small padding.
    min_x -= 1;
    max_x += 1;
    min_y -= 1;
    max_y += 1;

    let mut cropped = false;
    let w = max_x - min_x + 1;
    let h = max_y - min_y + 1;
    if w > MAX_ABS_DIM {
        cropped = true;
        let half = MAX_ABS_DIM / 2;
        min_x = display_king.x - half;
        max_x = min_x + MAX_ABS_DIM - 1;
    }
    if h > MAX_ABS_DIM {
        cropped = true;
        let half = MAX_ABS_DIM / 2;
        min_y = display_king.y - half;
        max_y = min_y + MAX_ABS_DIM - 1;
    }

    // Keep the king visible.
    if display_king.x < min_x {
        min_x = display_king.x;
    }
    if display_king.x > max_x {
        max_x = display_king.x;
    }
    if display_king.y < min_y {
        min_y = display_king.y;
    }
    if display_king.y > max_y {
        max_y = display_king.y;
    }

    // Ensure non-empty.
    if min_x == max_x {
        min_x -= 1;
        max_x += 1;
    }
    if min_y == max_y {
        min_y -= 1;
        max_y += 1;
    }

    // Silence an unused-parameter warning if this grows: `layout` is currently only used for type parity.
    let _ = layout;

    (min_x, max_x, min_y, max_y, cropped)
}

fn is_empty_destination(pos: &infinite_chess::core::position::Position, delta: Coord) -> bool {
    let dst = Square::from_coord(delta);
    !pos.squares().iter().any(|&s| !s.is_none() && s == dst)
}

fn capture_list(
    layout: &infinite_chess::chess::layout::PieceLayout,
    pos: &infinite_chess::core::position::Position,
    display_king: Coord,
    mode: ViewMode,
    legal_dirs: &[usize],
) -> Vec<String> {
    let mut out = Vec::new();
    for &dir in legal_dirs.iter() {
        let delta = delta_from_dir_index(dir);
        let dst_sq = Square::from_coord(delta);
        for i in 0..pos.count() {
            let sq = pos.square(i);
            if sq.is_none() || sq != dst_sq {
                continue;
            }
            let kind = layout.kind(i);
            let here = match mode {
                ViewMode::Relative => delta,
                ViewMode::Absolute => display_king + delta,
            };
            out.push(format!("{}@({}, {})", piece_char(kind), here.x, here.y));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn describe_white_action(
    layout: &infinite_chess::chess::layout::PieceLayout,
    from: &infinite_chess::core::position::Position,
    to: &infinite_chess::core::position::Position,
    display_king: Coord,
    view: ViewMode,
) -> String {
    if from == to {
        return "White: pass".to_string();
    }

    let Some((kind, rel_from, rel_to)) = diff_single_piece_move(layout, from, to) else {
        return "White: (move)".to_string();
    };

    if view == ViewMode::Absolute {
        let abs_from = display_king + rel_from;
        let abs_to = display_king + rel_to;
        format!(
            "White: {} ({}, {}) -> ({}, {}) [abs: ({}, {}) -> ({}, {})]",
            piece_char(kind),
            rel_from.x,
            rel_from.y,
            rel_to.x,
            rel_to.y,
            abs_from.x,
            abs_from.y,
            abs_to.x,
            abs_to.y
        )
    } else {
        format!(
            "White: {} ({}, {}) -> ({}, {})",
            piece_char(kind),
            rel_from.x,
            rel_from.y,
            rel_to.x,
            rel_to.y
        )
    }
}

fn diff_single_piece_move(
    layout: &infinite_chess::chess::layout::PieceLayout,
    from: &infinite_chess::core::position::Position,
    to: &infinite_chess::core::position::Position,
) -> Option<(PieceKind, Coord, Coord)> {
    for run in layout.identical_runs().iter() {
        let kind = layout.kind(run.start);

        let mut a: Vec<Coord> = Vec::new();
        let mut b: Vec<Coord> = Vec::new();

        for i in run.start..run.end {
            let sa = from.square(i);
            if !sa.is_none() {
                a.push(sa.coord());
            }
            let sb = to.square(i);
            if !sb.is_none() {
                b.push(sb.coord());
            }
        }

        let removed = coords_in_a_not_in_b(&a, &b);
        let added = coords_in_a_not_in_b(&b, &a);

        if removed.is_empty() && added.is_empty() {
            continue;
        }
        if removed.len() == 1 && added.len() == 1 {
            return Some((kind, removed[0], added[0]));
        }
    }
    None
}

fn coords_in_a_not_in_b(a: &[Coord], b: &[Coord]) -> Vec<Coord> {
    let mut out = Vec::new();
    'outer: for &x in a.iter() {
        for &y in b.iter() {
            if x == y {
                continue 'outer;
            }
        }
        out.push(x);
    }
    out
}

fn print_help() {
    println!("Commands:");
    println!("  q w e");
    println!("  a s d    (s is center; black cannot pass)");
    println!("  z x c");
    println!("  help | view relative | view absolute | exit");
    println!();
}
