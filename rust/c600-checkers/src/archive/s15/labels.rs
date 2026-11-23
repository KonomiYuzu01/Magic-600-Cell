//! Shared by the sub-checkers of stages 15 to 17: the full label table of one jumbling state, every piece at every
//! grip in, out or straddling (stage 14, section 1), classified exactly. No sub-checker is registered here; each one
//! that runs this file names it among its sources.
//!
//! The table is built the way the engine's own `State::classify` classifies one cut, for all 600 cuts at once and
//! keeping every straddler: a piece of a lattice pose K(k) is inside cap e exactly when the pole that k's inverse sends
//! e to lies in the piece's signature, and never straddles; an off-lattice pose group is split by the same anchor rule
//! (repeatedly the cap held by the most remaining members, the lowest pole on a tie), each anchor's cap region is
//! classified against the pulled-back cut by the engine's exact evaluator, and the members of a straddling anchor are
//! classified one by one. Nothing is cached between states, and nothing is read from a label cache of the archive.

use c600_jumble::eval::{pullback, Class, Evaluator};
use c600_jumble::model::POLES;
use c600_jumble::state::{Pose, State};
use c600_jumble::Context;

use crate::claims::{reject, Step};

/// Every grip's inside set and straddler set, each ascending.
pub struct LabelTable {
    pub inside: Vec<Vec<u32>>,
    pub straddle: Vec<Vec<u32>>,
}

impl LabelTable {
    /// Classifies every piece of the state at every grip.
    pub fn build(ctx: &Context, state: &State, claim: &str) -> Step<LabelTable> {
        let (model, kp, reg) = (&ctx.model, &ctx.kplus, &ctx.regions);
        let mut inside: Vec<Vec<u32>> = vec![Vec::new(); POLES];
        let mut straddle: Vec<Vec<u32>> = vec![Vec::new(); POLES];
        // Lattice pieces: p is inside e exactly when perms[inv k][e] is in the signature of p.
        let mut preimage: Vec<Option<Vec<u16>>> = vec![None; kp.perms.len()];
        let mut groups: Vec<Vec<u32>> = vec![Vec::new(); state.poses().len()];
        for (p, &pid) in state.pose_ids().iter().enumerate() {
            match &state.poses()[pid as usize] {
                Pose::K(k) => {
                    let pre = preimage[*k as usize].get_or_insert_with(|| {
                        let forward = &kp.perms[kp.inv[*k as usize] as usize];
                        let mut back = vec![0u16; POLES];
                        for (e, &f) in forward.iter().enumerate() {
                            back[f as usize] = e as u16;
                        }
                        back
                    });
                    for &f in model.signature(p) {
                        inside[pre[f as usize] as usize].push(p as u32);
                    }
                }
                Pose::M(_) => groups[pid as usize].push(p as u32),
            }
        }
        // Off-lattice pose groups, split by anchors once.
        let mut split: Vec<(u32, Vec<(usize, Vec<u32>)>)> = Vec::new();
        for (pid, members) in groups.iter().enumerate() {
            if members.is_empty() {
                continue;
            }
            split.push((pid as u32, anchors(ctx, members, claim)?));
        }
        for e in 0..POLES {
            for (pid, parts) in &split {
                let ev = Evaluator::new(&pullback(state.pose_matrix(ctx, *pid), &model.poles[e]));
                for (anchor, sub) in parts {
                    match ev.classify(reg.cap_vform(*anchor, kp, model), None) {
                        Class::In(_) => inside[e].extend_from_slice(sub),
                        Class::Out(_) => {}
                        Class::Uncertain(_) => return Err(reject(claim, "an anchor's label is uncertain")),
                        Class::Straddle { .. } => {
                            for &p in sub {
                                match ev.classify(&reg.vform(p as usize, kp, model), None) {
                                    Class::In(_) => inside[e].push(p),
                                    Class::Out(_) => {}
                                    Class::Straddle { .. } => straddle[e].push(p),
                                    Class::Uncertain(_) => return Err(reject(claim, "a piece's label is uncertain")),
                                }
                            }
                        }
                    }
                }
            }
            inside[e].sort_unstable();
            straddle[e].sort_unstable();
        }
        Ok(LabelTable { inside, straddle })
    }

    /// Whether no piece straddles grip x.
    pub fn clean(&self, x: usize) -> bool {
        self.straddle[x].is_empty()
    }
}

/// The engine's anchor split of an off-lattice pose group.
fn anchors(ctx: &Context, members: &[u32], claim: &str) -> Step<Vec<(usize, Vec<u32>)>> {
    let model = &ctx.model;
    let mut left: Vec<u32> = members.to_vec();
    let mut out = Vec::new();
    let mut counts = vec![0u32; POLES];
    while !left.is_empty() {
        counts.iter_mut().for_each(|c| *c = 0);
        for &p in &left {
            for &c in model.signature(p as usize) {
                counts[c as usize] += 1;
            }
        }
        let best = *counts.iter().max().unwrap_or(&0);
        if best == 0 {
            return Err(reject(claim, "a piece has an empty signature"));
        }
        let a = counts.iter().position(|&c| c == best).unwrap_or(0);
        let (sel, rest): (Vec<u32>, Vec<u32>) = left.iter().partition(|&&p| model.in_sig(p as usize, a));
        out.push((a, sel));
        left = rest;
    }
    Ok(out)
}

/// Stage 14's greatest-fixed-point levels (`levels.py`): 0 at a clean grip; at a blocked grip, one more than the
/// largest, over its straddlers, of the least level of a grip whose cap holds the straddler, starting from infinity
/// (`INF`) and iterated until nothing changes. A level that stays `INF` is infinite.
pub const INF: i64 = 601;

pub fn levels(table: &LabelTable) -> Vec<i64> {
    // Every straddling piece, its holder grips and its straddle grips.
    let mut index: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
    for list in &table.straddle {
        for &p in list {
            let next = index.len();
            index.entry(p).or_insert(next);
        }
    }
    let mut holders: Vec<Vec<usize>> = vec![Vec::new(); index.len()];
    for (c, list) in table.inside.iter().enumerate() {
        for p in list {
            if let Some(&i) = index.get(p) {
                holders[i].push(c);
            }
        }
    }
    let clean: Vec<bool> = (0..POLES).map(|x| table.clean(x)).collect();
    let mut lev: Vec<i64> = clean.iter().map(|&c| if c { 0 } else { INF }).collect();
    loop {
        let minimum: Vec<i64> =
            holders.iter().map(|hs| hs.iter().map(|&c| lev[c]).filter(|&l| l < INF).min().unwrap_or(INF)).collect();
        let mut next = vec![0i64; POLES];
        for (d, list) in table.straddle.iter().enumerate() {
            for p in list {
                next[d] = next[d].max(minimum[index[p]]);
            }
            if !clean[d] {
                next[d] = (next[d] + 1).min(INF);
            }
        }
        if next == lev {
            return lev;
        }
        lev = next;
    }
}
