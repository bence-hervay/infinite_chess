use crate::scenario::{DomainLike, LawsLike, Scenario, SearchError, State};
use crate::search::resources::ResourceTracker;

pub fn legal_black_moves<D, L, P>(
    scn: &Scenario<D, L, P>,
    laws: &L,
    s: &State,
    tracker: &mut ResourceTracker,
) -> Result<Vec<State>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    let mut out: Vec<State> = Vec::with_capacity(8);

    for (delta, pos2) in scn.rules.black_moves_with_delta(&s.pos) {
        let to = State {
            abs_king: if scn.track_abs_king {
                s.abs_king + delta
            } else {
                s.abs_king
            },
            pos: pos2,
        };

        if !laws.allow_black_move(s, &to, delta) {
            continue;
        }
        if !laws.allow_state(&to) {
            continue;
        }

        out.push(to);
    }

    tracker.bump_edges("movegen_black", out.len())?;
    Ok(out)
}

pub fn legal_white_moves<D, L, P>(
    scn: &Scenario<D, L, P>,
    laws: &L,
    s: &State,
    tracker: &mut ResourceTracker,
) -> Result<Vec<State>, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    let mut out: Vec<State> = Vec::new();

    if scn.white_can_pass && laws.allow_pass(s) {
        out.push(s.clone());
    }

    for pos2 in scn.rules.white_moves(&s.pos, false) {
        let to = State {
            abs_king: s.abs_king,
            pos: pos2,
        };

        if !laws.allow_white_move(s, &to) {
            continue;
        }
        if !laws.allow_state(&to) {
            continue;
        }

        out.push(to);
    }

    tracker.bump_edges("movegen_white", out.len())?;
    Ok(out)
}

pub fn is_checkmate_with_laws<D, L, P>(
    scn: &Scenario<D, L, P>,
    laws: &L,
    s: &State,
    tracker: &mut ResourceTracker,
) -> Result<bool, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    if !scn
        .rules
        .is_attacked(crate::core::coord::Coord::ORIGIN, &s.pos)
    {
        return Ok(false);
    }
    Ok(legal_black_moves(scn, laws, s, tracker)?.is_empty())
}

pub fn is_stalemate_with_laws<D, L, P>(
    scn: &Scenario<D, L, P>,
    laws: &L,
    s: &State,
    tracker: &mut ResourceTracker,
) -> Result<bool, SearchError>
where
    D: DomainLike,
    L: LawsLike,
{
    if scn
        .rules
        .is_attacked(crate::core::coord::Coord::ORIGIN, &s.pos)
    {
        return Ok(false);
    }
    Ok(legal_black_moves(scn, laws, s, tracker)?.is_empty())
}
