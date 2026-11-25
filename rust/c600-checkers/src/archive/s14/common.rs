//! Exact computations shared by stage 14's sub-checkers: the start table and its menu, the left heights and branches of
//! a state's poses, the chronological observables of one state, the labels of every piece at named grips, and the
//! exact text of a number of Q(√5). Each sub-checker that runs on this file names it in its sources, so its identity
//! changes whenever this file does (KN6.3).
//!
//! Everything is recomputed from the exact engines and the stated inputs; nothing is read from a label cache or a
//! producer (KN6.2). A label (in, out or straddle) is the sign pattern of the cut's linear form over the vertices of a
//! piece's posed region, evaluated by the engine's exact sign kernel. A pose group's members are first tested together
//! through a cap region that contains their home regions: when its image lies wholly on one side of the cut, so does
//! each member's, by convexity; otherwise every member is evaluated on its own. A lattice piece occupies a chamber,
//! whose label is read from its signature moved by its K⁺ pose.

use std::sync::Arc;

use c600_jumble::eval::{pullback, Class, Evaluator};
use c600_jumble::model::{Model, NP, POLES};
use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_jumble::observe::Tables;
use c600_jumble::q5::{matmul, Mat4, Q5};
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::Twist;
use c600_jumble::Context;
use c600_kernel::json::Value;
use c600_kernel::parallel;

use crate::claims::{array, at, field, int, not_applicable, reject, start_mismatch, Run, Step};
use crate::replay::{left_facts, letter, read_menu, read_state, Jumbling};
use crate::report::codes;

/// The input role of the start table: a stage 13 pose table (`poses`, `pose_id`, `digest`).
pub const TABLE: &str = "table";
/// The input role of the menu record, which must be I_a by identity and order.
pub const MENU: &str = "menu";

/// The jumbling engine, which every stage 14 sub-checker needs.
pub fn engine<'e>(engines: &crate::Engines<'e>) -> Step<&'e Context> {
    engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))
}

/// The I_a menu of input [`MENU`] and the state of input [`TABLE`], whose digest must be the stated start digest
/// `stated` of claim `claim`. Another table is the wrong start table (KN6.5).
pub fn start<'e>(ctx: &'e Context, run: &mut Run, claim: &str, stated: &str) -> Step<Jumbling<'e>> {
    let menu = read_menu(ctx, run, MENU)?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    if run.optional_input(TABLE).is_none() {
        return Err(start_mismatch(TABLE, "the start table is absent"));
    }
    let state = read_state(ctx, run, &menu, TABLE)?;
    let j = Jumbling::new(ctx, menu, state);
    run.compare(claim, &stated.to_owned(), &j.digest(), codes::START_TABLE_MISMATCH)?;
    Ok(j)
}

/// As [`start`], after the prefix word `prefix` (claim `prefix_claim`) replayed letter by letter from the table, each
/// letter admissible: the state it reaches must have the stated digest, else the start table is the wrong one.
pub fn start_after<'e>(
    ctx: &'e Context,
    run: &mut Run,
    claim: &str,
    stated: &str,
    prefix: &Value,
    prefix_claim: &str,
) -> Step<Jumbling<'e>> {
    let menu = read_menu(ctx, run, MENU)?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    let letters = word(prefix, prefix_claim, menu.items.len())?;
    if run.optional_input(TABLE).is_none() {
        return Err(start_mismatch(TABLE, "the start table is absent"));
    }
    let state = read_state(ctx, run, &menu, TABLE)?;
    let mut j = Jumbling::new(ctx, menu, state);
    for (k, &(cap, item)) in letters.iter().enumerate() {
        let c = at(prefix_claim, k);
        let twist = j.twist(cap, item, &c)?;
        if j.apply(&twist, &c)?.status != Status::Admissible {
            return Err(run.failed(reject(&c, "the prefix letter is refused at the state before it"), "blocked"));
        }
    }
    run.held(prefix_claim, format!("{} letters replayed from the table", letters.len()));
    run.compare(claim, &stated.to_owned(), &j.digest(), codes::START_TABLE_MISMATCH)?;
    Ok(j)
}

/// A running time in seconds (`cpu_s`): no claim, read for its form only (a nonnegative number).
pub fn seconds(value: &Value, claim: &str) -> Step<()> {
    let ok = match value {
        Value::Int(n) => *n >= 0,
        Value::Number(lexeme) => !lexeme.starts_with('-'),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(reject(claim, "expected a running time in seconds"))
    }
}

/// A record's `setup` timings (`labels`, `load`, `trees` and `word`, each `{cpu_s}`): no claim, read for its form.
pub fn setup(value: &Value, claim: &str) -> Step<()> {
    const PARTS: [&str; 4] = ["labels", "load", "trees", "word"];
    crate::claims::keys(value, claim, &PARTS, &[])?;
    for part in PARTS {
        let path = crate::claims::dot(claim, part);
        let timing = field(value, part, claim)?;
        crate::claims::keys(timing, &path, &["cpu_s"], &[])?;
        seconds(field(timing, "cpu_s", &path)?, &crate::claims::dot(&path, "cpu_s"))?;
    }
    Ok(())
}

/// A stage 14 form that a sub-checker checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// `t3p-certificate/1` (`s14.t3p`).
    Certificate,
    /// `t3p-check/1` (`s14.t3p`).
    Check,
    /// `t3p-chain/1` (`s14.t3p`).
    Chain,
    /// `t3p-locality/1` (`s14.locality`).
    Locality,
    /// `t3p-levels/1` (`s14.levels`).
    Levels,
    /// A formatless replay record: `table`, `word` and `rows` (`s14.literal`).
    Replay,
    /// A formatless co-held record: `digest` and `groups` of high units (`s14.blocks`).
    Coheld,
    /// A formatless per-grip record: `digest` and `caps` (`s14.blocks`).
    Caps,
    /// A formatless anatomy record: `digest`, `phi`, `targets` and `anatomy` (`s14.blocks`).
    Anatomy,
}

impl Form {
    /// The ID of the sub-checker that checks the form.
    pub fn checker(self) -> &'static str {
        match self {
            Form::Certificate | Form::Check | Form::Chain => "s14.t3p",
            Form::Locality => "s14.locality",
            Form::Levels => "s14.levels",
            Form::Replay => "s14.literal",
            Form::Coheld | Form::Caps | Form::Anatomy => "s14.blocks",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Form::Certificate => "a t3p-certificate/1 record",
            Form::Check => "a t3p-check/1 record",
            Form::Chain => "a t3p-chain/1 record",
            Form::Locality => "a t3p-locality/1 record",
            Form::Levels => "a t3p-levels/1 record",
            Form::Replay => "a replay record",
            Form::Coheld => "a co-held record",
            Form::Caps => "a per-grip record",
            Form::Anatomy => "an anatomy record",
        }
    }

    /// The exact top-level fields of a formatless form (none for a form its format names).
    pub fn fields(self) -> Option<&'static [&'static str]> {
        match self {
            Form::Replay => Some(&["rows", "table", "word"]),
            Form::Coheld => Some(&["digest", "groups"]),
            Form::Caps => Some(&["caps", "digest"]),
            Form::Anatomy => Some(&["anatomy", "digest", "phi", "targets"]),
            _ => None,
        }
    }
}

/// The stage 14 form of a record, by its signature, when the sub-checker `id` checks it.
///
/// A record is `not_applicable` only when it is a finite search (`t3p-scan/1`, `t3p-pairs/1`, `t3p-plus/1`, a
/// retained-letter beam record, a peel record, each by its exact signature), one of the two input catalogues
/// (`sources.json`, `states.json`), or a record of a stage 14 form another sub-checker checks (which it names): its
/// format names that form, or, formatless, it has exactly that form's fields. Every other record fails at the field
/// that departs from the stage 14 forms, a format no stage 14 form has included: a record of a stage 14 form is checked
/// to its last field by its sub-checker, so a malformed one is a failed claim, never an unchecked one.
pub fn own(doc: &Value, id: &str) -> Step<Form> {
    let form = form(doc)?;
    if form.checker() == id {
        return Ok(form);
    }
    // A formatless record is read as a form by one of its fields; it is left to another sub-checker only when it has
    // exactly that form's fields, so a damaged record (a chain without its format) never goes unchecked.
    if let Some(fields) = form.fields() {
        crate::claims::keys(doc, "certificate", fields, &[])?;
    }
    Err(not_applicable(format!("{} is checked by {}, not {id}", form.name(), form.checker())))
}

/// The fields every group of a retained-letter beam record has.
const BEAM_GROUP: [&str; 9] = ["best", "branch", "height", "pose", "seconds", "size", "start", "status", "word"];

/// The fields of a two-letter peel record.
const PEEL: [&str; 9] = ["A", "B", "digest", "first", "keep", "n4", "phi", "rows", "x"];

fn form(doc: &Value) -> Step<Form> {
    if let Some(list) = doc.as_array() {
        if !list.is_empty() && list.iter().all(|entry| entry.get("table_sha256").is_some()) {
            return Err(not_applicable(CATALOGUE));
        }
        return Err(reject("certificate", "a list that is not the stage's table catalogue is no stage 14 record"));
    }
    let Some(map) = doc.as_object() else {
        return Err(reject("certificate", "a stage 14 record is a JSON object"));
    };
    if let Some(format) = map.get("format") {
        let Some(format) = format.as_str() else {
            return Err(reject("format", "expected a string"));
        };
        let text = |key: &str| doc.get(key).and_then(Value::as_str).unwrap_or("not stated").to_owned();
        return match format {
            "t3p-certificate/1" => Ok(Form::Certificate),
            "t3p-check/1" => Ok(Form::Check),
            "t3p-chain/1" => Ok(Form::Chain),
            "t3p-locality/1" => Ok(Form::Locality),
            "t3p-levels/1" => Ok(Form::Levels),
            "t3p-scan/1" => Err(not_applicable(format!(
                "t3p-scan/1 is a finite search: its statuses and its empty lowering layer are negatives of its own \
                 family only ({}), and no complete coverage certificate is stored",
                text("family_exhausted")
            ))),
            "t3p-pairs/1" => Err(not_applicable(format!(
                "t3p-pairs/1 is a finite search over two-letter words at two grips ({}); it keeps {}, so its counts \
                 and its negative are not checked",
                text("metric_scope"),
                text("rows_kept")
            ))),
            "t3p-plus/1" => Err(not_applicable(
                "t3p-plus/1 is a finite search for T3P+ instances with one unlocking helper; its empty result is a \
                 negative of that family only, and no complete coverage certificate is stored",
            )),
            // Any other format, a damaged stage 14 one or another stage's, is no stage 14 record: it fails here, so a
            // record whose format was changed never goes unchecked.
            other => Err(reject(
                "format",
                format!(
                    "{other} is no stage 14 format (t3p-certificate/1, t3p-check/1, t3p-chain/1, t3p-locality/1, \
                     t3p-levels/1 or a finite search's)"
                ),
            )),
        };
    }
    let has = |key: &str| map.contains_key(key);
    let exactly = |fields: &[&str]| map.len() == fields.len() && fields.iter().all(|key| has(key));
    let groups = map.get("groups").and_then(Value::as_array);
    let beam_group = |g: &Value| g.as_object().is_some_and(|g| BEAM_GROUP.iter().all(|key| g.contains_key(*key)));
    if exactly(&["digest", "groups"]) && groups.is_some_and(|g| !g.is_empty() && g.iter().all(beam_group)) {
        return Err(not_applicable(
            "a retained-letter beam search (bounded depth, beam and time): its best words and its no-improvement \
             results are outcomes of that beam only, not a coverage claim",
        ));
    }
    if exactly(&PEEL) {
        return Err(not_applicable(
            "a two-letter peel search that follows only the kept first letters: its counts cover the pairs it \
             explored, not every word",
        ));
    }
    if !map.is_empty() && map.values().all(|v| v.get("summary").is_some() && v.get("high").is_some()) {
        return Err(not_applicable(CATALOGUE));
    }
    if has("rows") || has("word") || has("table") {
        Ok(Form::Replay)
    } else if has("groups") {
        Ok(Form::Coheld)
    } else if has("caps") {
        Ok(Form::Caps)
    } else if has("anatomy") || has("targets") {
        Ok(Form::Anatomy)
    } else {
        Err(reject(
            "certificate",
            "a formatless record of no stage 14 form (replay rows, co-held units, per-grip rows or anatomy)",
        ))
    }
}

/// The reason an input catalogue is unchecked.
const CATALOGUE: &str = "an input catalogue (the stage's tables or its named states) asserts no mathematical result";

/// A word of `[cap, item]` letters of the I_a menu.
pub fn word(value: &Value, claim: &str, items: usize) -> Step<Vec<(usize, usize)>> {
    array(value, claim)?.iter().enumerate().map(|(k, v)| letter(v, &at(claim, k), items)).collect()
}

/// A grip ID, 0 to 599.
pub fn grip(value: &Value, claim: &str) -> Step<usize> {
    usize::try_from(int(value, claim)?)
        .ok()
        .filter(|&c| c < POLES)
        .ok_or_else(|| reject(claim, format!("a grip is 0 to {}", POLES - 1)))
}

/// A piece ID.
pub fn piece(value: &Value, claim: &str) -> Step<usize> {
    usize::try_from(int(value, claim)?)
        .ok()
        .filter(|&p| p < NP)
        .ok_or_else(|| reject(claim, format!("a piece is 0 to {}", NP - 1)))
}

/// A JSON boolean.
pub fn boolean(value: &Value, claim: &str) -> Step<bool> {
    match value {
        Value::Bool(b) => Ok(*b),
        _ => Err(reject(claim, "expected true or false")),
    }
}

/// The stated integer at `key` of `obj` compared with the computed one.
pub fn claim_int(run: &mut Run, obj: &Value, path: &str, key: &str, computed: i64) -> Step<()> {
    let claim = crate::claims::dot(path, key);
    let stated = int(field(obj, key, path)?, &claim)?;
    run.claim(&claim, &stated, &computed)
}

/// The stated boolean at `key` of `obj` compared with the computed one.
pub fn claim_bool(run: &mut Run, obj: &Value, path: &str, key: &str, computed: bool) -> Step<()> {
    let claim = crate::claims::dot(path, key);
    let stated = boolean(field(obj, key, path)?, &claim)?;
    run.claim(&claim, &stated, &computed)
}

/// The stated string at `key` of `obj` compared with the computed one.
pub fn claim_str(run: &mut Run, obj: &Value, path: &str, key: &str, computed: &str) -> Step<()> {
    let claim = crate::claims::dot(path, key);
    let stated = crate::claims::string(field(obj, key, path)?, &claim)?.to_owned();
    run.claim(&claim, &stated, &computed.to_owned())
}

/// A stated list of integers compared with the computed one, element by element.
pub fn claim_ints(run: &mut Run, value: &Value, claim: &str, computed: &[i64]) -> Step<()> {
    let stated = array(value, claim)?;
    run.claim(&format!("{claim}.length"), &stated.len(), &computed.len())?;
    for (k, (s, c)) in stated.iter().zip(computed).enumerate() {
        let path = at(claim, k);
        run.claim(&path, &int(s, &path)?, c)?;
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------ heights and observables

/// The left height and left branch of every pose of a state, by pose ID (a lattice pose has height 0).
pub struct Heights {
    pub height: Vec<i64>,
    pub branch: Vec<i64>,
}

impl Heights {
    /// The left height and branch of piece `p` of `state`.
    pub fn of(&self, state: &State, p: usize) -> (i64, i64) {
        let id = state.pose_ids()[p] as usize;
        (self.height[id], self.branch[id])
    }
}

/// The heights of every pose of `state`, each from the engine's exact tree facts.
pub fn heights(ctx: &Context, tables: &Tables, state: &State, claim: &str) -> Step<Heights> {
    let mut height = Vec::with_capacity(state.poses().len());
    let mut branch = Vec::with_capacity(state.poses().len());
    for pose in state.poses() {
        let (h, b) = left_facts(ctx, tables, pose, claim)?;
        height.push(h);
        branch.push(b);
    }
    Ok(Heights { height, branch })
}

/// The number of pieces in each pose, by pose ID.
pub fn pose_counts(state: &State) -> Vec<i64> {
    let mut counts = vec![0i64; state.poses().len()];
    for &id in state.pose_ids() {
        counts[id as usize] += 1;
    }
    counts
}

/// The pose of an exact rotation: its K⁺ index when it lies in K⁺.
pub fn pose_of(ctx: &Context, m: Mat4) -> Pose {
    match ctx.kplus.index_of_matrix(&m, &ctx.model) {
        Some(k) => Pose::K(k),
        None => Pose::M(Arc::new(m)),
    }
}

/// The pose that a letter's twist gives a piece of pose `pid` inside its cap: the twist's matrix times the pose's.
pub fn image(ctx: &Context, state: &State, twist: &Twist, pid: u32) -> Pose {
    match (state.poses()[pid as usize].kidx(), twist.kidx) {
        (Some(k), Some(t)) => Pose::K(ctx.kplus.compose(t, k)),
        _ => pose_of(ctx, matmul(&twist.matrix, state.pose_matrix(ctx, pid))),
    }
}

/// The chronological observables of one state: the counts of pieces of left height 2, 4 and 6, the largest height,
/// Φ = Σ max(0, h − 2), the number of pieces off the lattice and the state digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observables {
    pub n2: i64,
    pub n4: i64,
    pub n6: i64,
    pub max_h: i64,
    pub phi: i64,
    pub off: i64,
    pub digest: String,
}

/// The observables of `state`, recomputed in full from the heights of its poses.
pub fn observe(ctx: &Context, tables: &Tables, state: &State, claim: &str) -> Step<Observables> {
    Ok(observe_from(ctx, state, &heights(ctx, tables, state, claim)?))
}

/// The observables of `state` from the heights of its poses, already computed.
pub fn observe_from(ctx: &Context, state: &State, heights: &Heights) -> Observables {
    let counts = pose_counts(state);
    let mut out = Observables { n2: 0, n4: 0, n6: 0, max_h: 0, phi: 0, off: 0, digest: state.digest(ctx) };
    for (id, &n) in counts.iter().enumerate() {
        if n == 0 {
            continue;
        }
        let h = heights.height[id];
        match h {
            2 => out.n2 += n,
            4 => out.n4 += n,
            6 => out.n6 += n,
            _ => {}
        }
        out.max_h = out.max_h.max(h);
        out.phi += n * (h - 2).max(0);
        if state.poses()[id].kidx().is_none() {
            out.off += n;
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------------ labels

/// The labels of every piece at named grips: the pieces inside each cap and the pieces that straddle its cut, both
/// ascending. A piece in neither is out. With `lattice` false, only the pieces off the lattice are listed inside (a
/// lattice piece never straddles: it occupies a chamber).
pub struct Labels {
    index: Vec<Option<usize>>,
    inside: Vec<Vec<u32>>,
    straddle: Vec<Vec<u32>>,
}

impl Labels {
    /// The pieces inside cap `grip`, ascending; empty for a grip that was not named.
    pub fn inside(&self, grip: usize) -> &[u32] {
        self.index[grip].map_or(&[], |i| &self.inside[i])
    }

    /// The pieces that straddle the cut of `grip`, ascending; empty for a grip that was not named.
    pub fn straddle(&self, grip: usize) -> &[u32] {
        self.index[grip].map_or(&[], |i| &self.straddle[i])
    }
}

/// The off-lattice pose groups of a state: pose ID and ascending members.
pub fn off_groups(state: &State) -> Vec<(u32, Vec<u32>)> {
    let mut members: Vec<Vec<u32>> = vec![Vec::new(); state.poses().len()];
    for (p, &id) in state.pose_ids().iter().enumerate() {
        if state.poses()[id as usize].kidx().is_none() {
            members[id as usize].push(p as u32);
        }
    }
    members.into_iter().enumerate().filter(|(_, m)| !m.is_empty()).map(|(id, m)| (id as u32, m)).collect()
}

/// Splits a group's members by caps that hold them at home: repeatedly the cap held by the most remaining members
/// (the lowest pole on a tie) with the remaining members it holds.
fn anchors(model: &Model, members: &[u32]) -> Option<Vec<(usize, Vec<u32>)>> {
    let mut left = members.to_vec();
    let mut out = Vec::new();
    let mut counts = vec![0u32; POLES];
    while !left.is_empty() {
        counts.iter_mut().for_each(|c| *c = 0);
        for &p in &left {
            for &c in model.signature(p as usize) {
                counts[c as usize] += 1;
            }
        }
        let best = counts.iter().copied().max().unwrap_or(0);
        if best == 0 {
            return None;
        }
        let a = counts.iter().position(|&c| c == best)?;
        let (held, rest): (Vec<u32>, Vec<u32>) = left.iter().partition(|&&p| model.in_sig(p as usize, a));
        out.push((a, held));
        left = rest;
    }
    Some(out)
}

/// The inside and straddling members of one off-lattice pose group at each named grip.
fn group_labels(ctx: &Context, g: &Mat4, members: &[u32], grips: &[usize]) -> Option<Vec<(Vec<u32>, Vec<u32>)>> {
    let (model, kp, regions) = (&ctx.model, &ctx.kplus, &ctx.regions);
    let anchors = anchors(model, members)?;
    let mut out = Vec::with_capacity(grips.len());
    for &e in grips {
        let ev = Evaluator::new(&pullback(g, &model.poles[e]));
        let (mut inside, mut straddle) = (Vec::new(), Vec::new());
        for (a, held) in &anchors {
            match ev.classify(regions.cap_vform(*a, kp, model), None) {
                Class::In(_) => inside.extend_from_slice(held),
                Class::Out(_) => {}
                _ => {
                    for &p in held {
                        match ev.classify(&regions.vform(p as usize, kp, model), None) {
                            Class::In(_) => inside.push(p),
                            Class::Straddle { .. } => straddle.push(p),
                            Class::Out(_) => {}
                            Class::Uncertain(_) => return None,
                        }
                    }
                }
            }
        }
        out.push((inside, straddle));
    }
    Some(out)
}

/// The labels of every piece of `state` at `grips`, the pose groups evaluated on the machine's threads. The result does
/// not depend on the threads: each group's labels are its own, and the lists are sorted.
pub fn labels(ctx: &Context, state: &State, grips: &[usize], lattice: bool, claim: &str) -> Step<Labels> {
    let mut index = vec![None; POLES];
    let mut named = Vec::new();
    for &g in grips {
        if g >= POLES {
            return Err(reject(claim, format!("grip {g} is not a pole")));
        }
        if index[g].is_none() {
            index[g] = Some(named.len());
            named.push(g);
        }
    }
    let groups = off_groups(state);
    let evaluated = parallel::map(groups.len(), |k| {
        let (pid, members) = &groups[k];
        group_labels(ctx, state.pose_matrix(ctx, *pid), members, &named)
    });
    let mut inside: Vec<Vec<u32>> = vec![Vec::new(); named.len()];
    let mut straddle: Vec<Vec<u32>> = vec![Vec::new(); named.len()];
    for result in evaluated {
        let rows = result.ok().flatten().ok_or_else(|| reject(claim, "an exact label evaluation did not finish"))?;
        for (i, (ins, str)) in rows.into_iter().enumerate() {
            inside[i].extend(ins);
            straddle[i].extend(str);
        }
    }
    if lattice {
        for (i, held) in lattice_inside(ctx, state, &named, claim)?.into_iter().enumerate() {
            inside[i].extend(held);
        }
    }
    for list in inside.iter_mut().chain(straddle.iter_mut()) {
        list.sort_unstable();
    }
    Ok(Labels { index, inside, straddle })
}

/// The pieces of `pieces` that straddle each of `grips` in `state`, ascending, each evaluated exactly at its pose by
/// pose group (a lattice piece occupies a chamber and straddles nothing).
pub fn straddlers_among(
    ctx: &Context,
    state: &State,
    pieces: &[u32],
    grips: &[usize],
    claim: &str,
) -> Step<Vec<Vec<u32>>> {
    let mut by_pose: std::collections::BTreeMap<u32, Vec<u32>> = std::collections::BTreeMap::new();
    for &p in pieces {
        let id = state.pose_ids()[p as usize];
        if state.poses()[id as usize].kidx().is_none() {
            by_pose.entry(id).or_default().push(p);
        }
    }
    let groups: Vec<(u32, Vec<u32>)> = by_pose.into_iter().collect();
    let evaluated = parallel::map(groups.len(), |k| {
        let (pid, members) = &groups[k];
        group_labels(ctx, state.pose_matrix(ctx, *pid), members, grips)
    });
    let mut out: Vec<Vec<u32>> = vec![Vec::new(); grips.len()];
    for result in evaluated {
        let rows = result.ok().flatten().ok_or_else(|| reject(claim, "an exact label evaluation did not finish"))?;
        for (i, (_, straddle)) in rows.into_iter().enumerate() {
            out[i].extend(straddle);
        }
    }
    for list in &mut out {
        list.sort_unstable();
    }
    Ok(out)
}

/// The lattice pieces inside each of `grips`, ascending: a piece at K⁺ pose k is inside cap e exactly when its home
/// signature holds k⁻¹(e).
pub fn lattice_inside(ctx: &Context, state: &State, grips: &[usize], claim: &str) -> Step<Vec<Vec<u32>>> {
    let (model, kp) = (&ctx.model, &ctx.kplus);
    let found = parallel::map(grips.len(), |i| {
        let e = grips[i];
        let mut held = Vec::new();
        for (p, &id) in state.pose_ids().iter().enumerate() {
            if let Pose::K(k) = state.poses()[id as usize] {
                if model.in_sig(p, kp.perms[kp.inv[k as usize] as usize][e] as usize) {
                    held.push(p as u32);
                }
            }
        }
        held
    });
    found
        .into_iter()
        .map(|result| result.map_err(|_| reject(claim, "a lattice label evaluation did not finish")))
        .collect()
}

/// Whether sorted `a` and `b` share an element.
pub fn meets(a: &[u32], b: &[u32]) -> bool {
    intersection(a, b) > 0
}

/// The number of elements that sorted `a` and `b` share.
pub fn intersection(a: &[u32], b: &[u32]) -> usize {
    let (mut i, mut j, mut n) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                n += 1;
                i += 1;
                j += 1;
            }
        }
    }
    n
}

/// Whether sorted `list` holds `p`.
pub fn holds(list: &[u32], p: u32) -> bool {
    list.binary_search(&p).is_ok()
}

// ------------------------------------------------------------------------------------------------------ numbers

/// The exact text of a number of Q(√5) as the archive writes it: `a`, `b*sqrt(5)` or `a ± |b|*sqrt(5)`, in parentheses
/// over the denominator when it is not 1.
pub fn exact_text(q: &Q5) -> String {
    let (a, b, d) = (q.a().to_string(), q.b().to_string(), q.d().to_string());
    let body = if b == "0" {
        a
    } else if a == "0" {
        format!("{b}*sqrt(5)")
    } else if let Some(magnitude) = b.strip_prefix('-') {
        format!("{a} - {magnitude}*sqrt(5)")
    } else {
        format!("{a} + {b}*sqrt(5)")
    };
    if d == "1" {
        body
    } else {
        format!("({body})/{d}")
    }
}
