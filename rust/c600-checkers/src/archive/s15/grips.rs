//! Shared by the sub-checkers of stages 15 to 17: the observables of one grip at a jumbling state, each recomputed on
//! the exact engine from the state as it is (stage 14, section 1; stage 15, section 1). No sub-checker is registered
//! here; each one that runs this file names it among its sources.
//!
//! - **Clean grips**: every grip classified exactly (`State::survey`, no budget); a grip is clean when no piece
//!   straddles its cut.
//! - **S_x**: the number of straddlers of grip x, every one certified (`State::straddler_set`).
//! - **I_x**: the pieces inside cap x (`State::classify` with the inside set).
//! - **F_x**: the pieces of I_x of left height at least 2 whose branch is not CLS(x).
//! - **max D_x**: the largest D_x(h) over the jumble items h of the menu at x, where D_x(h) counts the pieces of I_x of
//!   branch CLS(x) and left height at least 4 whose left height under the exact image h·g is lowered by 2 (Lemma Φ).
//!   It is read from the exact poses whether or not x is clean, as stage 14's `t3p.py pairs` and stage 16's traces read
//!   it.
//!
//! Heights and branches come from the engine's exact tree facts; a pose with a right height other than 0 is outside the
//! height calculus and fails the claim ([`crate::replay::left_facts`]).

use std::collections::BTreeMap;
use std::sync::Arc;

use c600_jumble::model::POLES;
use c600_jumble::observe::Tables;
use c600_jumble::q5::matmul;
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::Context;

use crate::claims::{reject, Step};
use crate::replay::{left_facts, Jumbling};

/// Every grip's cleanliness at the state.
pub fn clean_grips(ctx: &Context, state: &mut State, claim: &str) -> Step<Vec<bool>> {
    let grips: Vec<usize> = (0..POLES).collect();
    let statuses =
        state.survey(ctx, &grips, None).map_err(|error| reject(claim, format!("the engine stopped: {}", error.0)))?;
    statuses
        .into_iter()
        .map(|status| match status {
            Status::Admissible => Ok(true),
            Status::Blocked => Ok(false),
            other => Err(reject(claim, format!("a grip's status is {}", other.as_str()))),
        })
        .collect()
}

/// I_x: the pieces inside cap x, ascending.
pub fn inside(ctx: &Context, state: &mut State, x: usize, claim: &str) -> Step<Vec<u32>> {
    let cl = state
        .classify(ctx, x, true, false, None)
        .map_err(|error| reject(claim, format!("the engine stopped: {}", error.0)))?;
    if cl.uncertain > 0 {
        return Err(reject(claim, "a piece's label at the grip is uncertain"));
    }
    Ok(cl.inside.unwrap_or_default())
}

/// S_x: the number of certified straddlers of grip x.
pub fn straddlers(ctx: &Context, state: &mut State, x: usize, claim: &str) -> Step<usize> {
    let set =
        state.straddler_set(ctx, x, None).map_err(|error| reject(claim, format!("the engine stopped: {}", error.0)))?;
    if set.uncertain > 0 {
        return Err(reject(claim, "a piece's label at the grip is uncertain"));
    }
    Ok(set.pieces.len())
}

/// The class CLS(x) of a grip.
pub fn class(ctx: &Context, tables: &Tables, x: usize, claim: &str) -> Step<i64> {
    tables.cls(ctx, x).map_err(|error| reject(claim, format!("the grip's class: {error}")))
}

/// The left height and branch of every pose of the state that holds a piece of `pieces`, by pose ID.
pub fn pose_facts(
    ctx: &Context,
    tables: &Tables,
    state: &State,
    pieces: &[u32],
    claim: &str,
) -> Step<BTreeMap<u32, (i64, i64, usize)>> {
    let mut out: BTreeMap<u32, (i64, i64, usize)> = BTreeMap::new();
    for &p in pieces {
        let pid = state.pose_ids()[p as usize];
        if let Some(entry) = out.get_mut(&pid) {
            entry.2 += 1;
            continue;
        }
        let (h, b) = left_facts(ctx, tables, &state.poses()[pid as usize], claim)?;
        out.insert(pid, (h, b, 1));
    }
    Ok(out)
}

/// F_x from the inside set.
pub fn foreign(ctx: &Context, tables: &Tables, state: &State, x: usize, ins: &[u32], claim: &str) -> Step<u64> {
    let cls = class(ctx, tables, x, claim)?;
    let facts = pose_facts(ctx, tables, state, ins, claim)?;
    Ok(facts.values().filter(|(h, b, _)| *h >= 2 && *b != cls).map(|(_, _, n)| *n as u64).sum())
}

/// max D_x over the jumble items of the menu at x, from the inside set.
pub fn max_lowering(j: &Jumbling, tables: &Tables, x: usize, ins: &[u32], claim: &str) -> Step<u64> {
    let ctx = j.ctx;
    let cls = class(ctx, tables, x, claim)?;
    let high: Vec<(u32, i64, usize)> = pose_facts(ctx, tables, &j.state, ins, claim)?
        .into_iter()
        .filter(|(_, (h, b, _))| *h >= 4 && *b == cls)
        .map(|(pid, (h, _, n))| (pid, h, n))
        .collect();
    if high.is_empty() {
        return Ok(0);
    }
    let mut best = 0u64;
    for item in 0..j.menu.items.len() {
        let twist = j.twist(x, item, claim)?;
        if twist.kidx.is_some() {
            continue;
        }
        let mut d = 0u64;
        for &(pid, h, n) in &high {
            let image = matmul(&twist.matrix, j.state.pose_matrix(ctx, pid));
            let facts = tables
                .facts(ctx, &Pose::M(Arc::new(image)))
                .map_err(|error| reject(claim, format!("the engine's tree facts: {error}")))?;
            if facts.dl == h - 2 {
                d += n as u64;
            }
        }
        best = best.max(d);
    }
    Ok(best)
}

/// S, F and max D at grip x, as stage 16's traces and mobility records state them.
pub fn sfd(j: &mut Jumbling, tables: &Tables, x: usize, claim: &str) -> Step<(u64, u64, u64)> {
    if x >= POLES {
        return Err(reject(claim, format!("grip {x} is not 0 to {}", POLES - 1)));
    }
    let ctx = j.ctx;
    let s = straddlers(ctx, &mut j.state, x, claim)? as u64;
    let ins = inside(ctx, &mut j.state, x, claim)?;
    let f = foreign(ctx, tables, &j.state, x, &ins, claim)?;
    let d = max_lowering(j, tables, x, &ins, claim)?;
    Ok((s, f, d))
}
