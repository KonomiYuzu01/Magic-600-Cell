//! Configurations of the rigid jumbling puzzle (state contract, sections 2, 3 and 5).
//!
//! Every piece has an exact pose, interned in a pose table: id 0 is the identity, a pose in K⁺ is kept by its K⁺ index
//! and any other pose by its exact matrix. A piece is on the lattice exactly when its pose lies in K⁺.
//!
//! A twist (c, g) is classified exactly for the cut of c. A lattice piece is classified by its transported signature.
//! Off-lattice pieces with one pose form a pose group, which is split by anchor caps a (a lies in the signature of every
//! piece of the subgroup): the posed cap region of a contains every posed piece region of the subgroup, so one exact
//! one-sided bound over its vertices certifies the whole subgroup. Otherwise every piece is classified over the
//! complete exact vertex set of its posed region. A straddling piece gives two exact points with h < 0 and h > 0, each
//! re-checked against all 1,200 constraints of its posed region before a certificate is issued. A budget of exact
//! evaluations may leave pieces without a certificate: the twist is then uncertain. A blocked, invalid, uncertain or
//! protection-refused twist changes nothing.
//!
//! The pose table is kept canonical after every commit (identity first, then by exact key), so equal configurations
//! have equal tables, and the digest covers the exact pose of every moved piece.

use std::collections::HashMap;
use std::sync::Arc;

use c600_kernel::json::{self, Value};
use num_traits::ToPrimitive;

use crate::eval::{pullback, Class, Evaluator};
use crate::kplus::ORDER;
use crate::model::NP;
use crate::q5::{is_rotation, json_int, matmul, matrix_from_json, matrix_json, matrix_spaced, matvec, Mat4, Vec4, Q5};
use crate::regions::VForm;
use crate::twist::{Menu, Twist};
use crate::{b64, codes, Context};

/// An internal failure: a broken invariant, never a refusal of the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EngineError {}

/// An exact pose: a K⁺ index or a matrix outside K⁺.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Pose {
    K(u16),
    M(Arc<Mat4>),
}

impl Pose {
    pub fn kidx(&self) -> Option<u16> {
        match self {
            Pose::K(k) => Some(*k),
            Pose::M(_) => None,
        }
    }

    /// The canonical order of the pose table: `K%05d` for K⁺ poses, `M` + the spaced JSON matrix otherwise.
    fn sort_key(&self) -> String {
        match self {
            Pose::K(k) => format!("K{k:05}"),
            Pose::M(m) => format!("M{}", matrix_spaced(m)),
        }
    }
}

/// How the certified pieces were certified.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub signature: u64,
    pub group_superset: u64,
    pub piece_vertices: u64,
}

/// Two exact points of a posed piece region on either side of a cut, both re-checked against the whole region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    pub piece: u32,
    pub grip: usize,
    pub orbit: usize,
    pub transport: u16,
    pub vertex_below: usize,
    pub vertex_above: usize,
    pub point_below: Vec4,
    pub point_above: Vec4,
    pub h_below: Q5,
    pub h_above: Q5,
}

impl Certificate {
    pub fn to_json(&self) -> Value {
        let point = |p: &Vec4| Value::Array(p.iter().map(Q5::to_json).collect());
        json::object([
            ("piece", Value::Int(self.piece as i64)),
            ("grip", Value::Int(self.grip as i64)),
            ("orbit", Value::Int(self.orbit as i64)),
            ("transport", Value::Int(self.transport as i64)),
            ("vertex_below", Value::Int(self.vertex_below as i64)),
            ("vertex_above", Value::Int(self.vertex_above as i64)),
            ("point_below", point(&self.point_below)),
            ("point_above", point(&self.point_above)),
            ("h_below", self.h_below.to_json()),
            ("h_above", self.h_above.to_json()),
            ("points_checked_against_all_constraints", Value::Bool(true)),
        ])
    }
}

/// The status of a twist or grip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Admissible,
    Blocked,
    Invalid,
    Uncertain,
    /// Refused by pose-level protection.
    Protected,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Admissible => "admissible",
            Status::Blocked => "blocked",
            Status::Invalid => "invalid",
            Status::Uncertain => "uncertain",
            Status::Protected => "protected",
        }
    }

    /// The survey letter: a, b or u.
    pub fn letter(self) -> &'static str {
        match self {
            Status::Admissible => "a",
            Status::Blocked => "b",
            _ => "u",
        }
    }
}

/// The exact classification of every piece for one cut.
#[derive(Clone, Debug)]
pub struct Classification {
    pub grip: usize,
    /// The pieces inside the cap, ascending; `None` when not requested or when the scan stopped at a straddler.
    pub inside: Option<Vec<u32>>,
    pub straddle: Option<Certificate>,
    pub uncertain: u64,
    pub counts: Counts,
    pub exact_evaluations: u64,
}

impl Classification {
    fn new(grip: usize) -> Classification {
        Classification {
            grip,
            inside: None,
            straddle: None,
            uncertain: 0,
            counts: Counts::default(),
            exact_evaluations: 0,
        }
    }

    pub fn status(&self) -> Status {
        if self.straddle.is_some() {
            Status::Blocked
        } else if self.uncertain > 0 {
            Status::Uncertain
        } else {
            Status::Admissible
        }
    }
}

/// Selection 4: the pose-level Net protection report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoseProtection {
    /// The protected pieces, ascending.
    pub protected: Vec<u32>,
    /// The protected pieces whose pose would change, ascending.
    pub pieces: Vec<u32>,
}

impl PoseProtection {
    pub fn moves(&self) -> bool {
        !self.pieces.is_empty()
    }

    pub fn to_json(&self) -> Value {
        let ids = |v: &[u32]| Value::Array(v.iter().map(|&p| Value::Int(p as i64)).collect());
        json::object([
            ("level", Value::String("poses".into())),
            ("mode", Value::String("net".into())),
            ("protected", ids(&self.protected)),
            ("pieces", ids(&self.pieces)),
            ("verdict", Value::String(if self.moves() { "moves" } else { "clear" }.into())),
            ("first_refused_prefix", Value::Null),
        ])
    }
}

/// The outcome of a twist. Only an admissible outcome changed the state.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub status: Status,
    pub grip: usize,
    /// The pieces that moved (the inside set), ascending; empty unless admissible.
    pub inside: Vec<u32>,
    /// The chapter 13 code and message of a refusal that has one.
    pub refusal: Option<(&'static str, String)>,
    pub straddle: Option<Certificate>,
    pub uncertain: u64,
    pub counts: Counts,
    pub exact_evaluations: u64,
    pub protection: Option<PoseProtection>,
}

impl Outcome {
    fn refused(status: Status, grip: usize, code: &'static str, message: String) -> Outcome {
        Outcome {
            status,
            grip,
            inside: Vec::new(),
            refusal: Some((code, message)),
            straddle: None,
            uncertain: 0,
            counts: Counts::default(),
            exact_evaluations: 0,
            protection: None,
        }
    }

    fn from_classification(cl: Classification) -> Outcome {
        Outcome {
            status: cl.status(),
            grip: cl.grip,
            inside: Vec::new(),
            refusal: None,
            straddle: cl.straddle,
            uncertain: cl.uncertain,
            counts: cl.counts,
            exact_evaluations: cl.exact_evaluations,
            protection: None,
        }
    }

    pub fn moved(&self) -> usize {
        self.inside.len()
    }
}

/// Every certified straddler of a named cap (the `straddlers` observation of the differential harness).
#[derive(Clone, Debug)]
pub struct StraddlerSet {
    pub pieces: Vec<u32>,
    pub certificates: Vec<Certificate>,
    pub uncertain: u64,
    pub counts: Counts,
    pub exact_evaluations: u64,
}

impl StraddlerSet {
    pub fn status(&self) -> &'static str {
        if self.uncertain > 0 {
            "uncertain"
        } else if self.pieces.is_empty() {
            "admissible"
        } else {
            "blocked"
        }
    }
}

type Anchors = Arc<Vec<(usize, Vec<u32>)>>;

/// A configuration: the pose of every piece, an optional twist menu, and caches that never change a result.
#[derive(Clone)]
pub struct State {
    menu: Option<Arc<Menu>>,
    pose_id: Vec<u32>,
    poses: Vec<Pose>,
    lookup: HashMap<Pose, u32>,
    /// Off-lattice pose groups (pose id, ascending members), valid until the next change.
    groups: Option<Arc<Vec<(u32, Vec<u32>)>>>,
    /// Anchor splits by pose id, valid until the next change.
    anchors: HashMap<u32, Anchors>,
}

fn eval(ev: &Evaluator, v: &VForm, cl: &mut Classification, budget: Option<u64>) -> Class {
    let left = budget.map(|b| b.saturating_sub(cl.exact_evaluations));
    let r = ev.classify(v, left);
    cl.exact_evaluations += r.evaluations();
    r
}

impl State {
    /// The solved configuration; `menu` is `None` in free mode.
    pub fn new(menu: Option<Arc<Menu>>) -> State {
        let identity = Pose::K(0);
        State {
            menu,
            pose_id: vec![0; NP],
            poses: vec![identity.clone()],
            lookup: HashMap::from([(identity, 0)]),
            groups: None,
            anchors: HashMap::new(),
        }
    }

    pub fn menu(&self) -> Option<&Menu> {
        self.menu.as_deref()
    }

    pub fn menu_arc(&self) -> Option<Arc<Menu>> {
        self.menu.clone()
    }

    pub fn pose_ids(&self) -> &[u32] {
        &self.pose_id
    }

    pub fn poses(&self) -> &[Pose] {
        &self.poses
    }

    pub fn pose_of(&self, piece: usize) -> &Pose {
        &self.poses[self.pose_id[piece] as usize]
    }

    fn intern(&mut self, pose: Pose) -> u32 {
        if let Some(&i) = self.lookup.get(&pose) {
            return i;
        }
        let i = self.poses.len() as u32;
        self.poses.push(pose.clone());
        self.lookup.insert(pose, i);
        i
    }

    fn intern_matrix(&mut self, ctx: &Context, m: Mat4) -> u32 {
        match ctx.kplus.index_of_matrix(&m, &ctx.model) {
            Some(k) => self.intern(Pose::K(k)),
            None => self.intern(Pose::M(Arc::new(m))),
        }
    }

    /// The exact matrix of a pose.
    pub fn pose_matrix<'a>(&'a self, ctx: &'a Context, pid: u32) -> &'a Mat4 {
        match &self.poses[pid as usize] {
            Pose::K(k) => ctx.kplus.matrix(*k, &ctx.model),
            Pose::M(m) => m,
        }
    }

    pub fn is_lattice_piece(&self, piece: usize) -> bool {
        self.pose_of(piece).kidx().is_some()
    }

    pub fn off_lattice_count(&self) -> usize {
        self.pose_id.iter().filter(|&&id| self.poses[id as usize].kidx().is_none()).count()
    }

    pub fn moved_count(&self) -> usize {
        self.pose_id.iter().filter(|&&id| id != 0).count()
    }

    fn invalidate(&mut self) {
        self.groups = None;
        self.anchors.clear();
    }

    /// The off-lattice pose groups: ascending pose id, members ascending.
    fn groups(&mut self) -> Arc<Vec<(u32, Vec<u32>)>> {
        if let Some(g) = &self.groups {
            return g.clone();
        }
        let mut members: Vec<Vec<u32>> = vec![Vec::new(); self.poses.len()];
        for (p, &id) in self.pose_id.iter().enumerate() {
            if self.poses[id as usize].kidx().is_none() {
                members[id as usize].push(p as u32);
            }
        }
        let groups: Vec<(u32, Vec<u32>)> =
            members.into_iter().enumerate().filter(|(_, m)| !m.is_empty()).map(|(id, m)| (id as u32, m)).collect();
        let groups = Arc::new(groups);
        self.groups = Some(groups.clone());
        groups
    }

    /// The anchor split of a pose group: repeatedly the cap held by the most remaining members (lowest pole on a tie),
    /// with the remaining members that it holds, in member order.
    fn anchors(&mut self, ctx: &Context, pid: u32, members: &[u32]) -> Result<Anchors, EngineError> {
        if let Some(a) = self.anchors.get(&pid) {
            return Ok(a.clone());
        }
        let model = &ctx.model;
        let mut left: Vec<u32> = members.to_vec();
        let mut out = Vec::new();
        let mut counts = vec![0u32; crate::model::POLES];
        while !left.is_empty() {
            counts.iter_mut().for_each(|c| *c = 0);
            for &p in &left {
                for &c in model.signature(p as usize) {
                    counts[c as usize] += 1;
                }
            }
            let best = *counts.iter().max().unwrap_or(&0);
            if best == 0 {
                return Err(EngineError("a piece has an empty signature".into()));
            }
            let a = counts.iter().position(|&c| c == best).unwrap_or(0);
            let (sel, rest): (Vec<u32>, Vec<u32>) = left.iter().partition(|&&p| model.in_sig(p as usize, a));
            out.push((a, sel));
            left = rest;
        }
        let out = Arc::new(out);
        self.anchors.insert(pid, out.clone());
        Ok(out)
    }

    /// Exact classification of every piece for the cut of pole e, with certificates.
    pub fn classify(
        &mut self,
        ctx: &Context,
        e: usize,
        need_inside: bool,
        stop_at_straddle: bool,
        budget: Option<u64>,
    ) -> Result<Classification, EngineError> {
        let (model, kp, reg) = (&ctx.model, &ctx.kplus, &ctx.regions);
        let mut cl = Classification::new(e);
        let mut inside: Vec<u32> = Vec::new();
        for (p, &id) in self.pose_id.iter().enumerate() {
            if let Pose::K(k) = self.poses[id as usize] {
                cl.counts.signature += 1;
                if need_inside {
                    let ep = kp.perms[kp.inv[k as usize] as usize][e];
                    if model.in_sig(p, ep as usize) {
                        inside.push(p as u32);
                    }
                }
            }
        }
        let groups = self.groups();
        for (pid, members) in groups.iter() {
            let anchors = self.anchors(ctx, *pid, members)?;
            let ev = Evaluator::new(&pullback(self.pose_matrix(ctx, *pid), &model.poles[e]));
            for (anchor, sub) in anchors.iter() {
                if budget.is_some_and(|b| cl.exact_evaluations >= b) {
                    cl.uncertain += sub.len() as u64;
                    continue;
                }
                match eval(&ev, reg.cap_vform(*anchor, kp, model), &mut cl, budget) {
                    Class::Uncertain(_) => {
                        cl.uncertain += sub.len() as u64;
                        continue;
                    }
                    Class::In(_) => {
                        cl.counts.group_superset += sub.len() as u64;
                        if need_inside {
                            inside.extend_from_slice(sub);
                        }
                        continue;
                    }
                    Class::Out(_) => {
                        cl.counts.group_superset += sub.len() as u64;
                        continue;
                    }
                    Class::Straddle { .. } => {}
                }
                for &p in sub {
                    match eval(&ev, &reg.vform(p as usize, kp, model), &mut cl, budget) {
                        Class::Uncertain(_) => cl.uncertain += 1,
                        Class::Straddle { below, above, .. } => {
                            if cl.straddle.is_none() {
                                cl.straddle = Some(self.certificate(ctx, p, *pid, e, below, above)?);
                            }
                            if stop_at_straddle {
                                return Ok(cl);
                            }
                        }
                        Class::In(_) => {
                            cl.counts.piece_vertices += 1;
                            if need_inside {
                                inside.push(p);
                            }
                        }
                        Class::Out(_) => cl.counts.piece_vertices += 1,
                    }
                }
            }
        }
        if need_inside {
            inside.sort_unstable();
            cl.inside = Some(inside);
        }
        Ok(cl)
    }

    fn certificate(
        &self,
        ctx: &Context,
        p: u32,
        pid: u32,
        e: usize,
        jneg: usize,
        jpos: usize,
    ) -> Result<Certificate, EngineError> {
        let model = &ctx.model;
        let g = self.pose_matrix(ctx, pid);
        let home = ctx.regions.vertices(p as usize, &ctx.kplus, model);
        let (xb, xa) = (matvec(g, &home[jneg]), matvec(g, &home[jpos]));
        let (hb, ha) = (model.h(e, &xb), model.h(e, &xa));
        let sig = model.signature(p as usize);
        if !(hb.sign() < 0
            && ha.sign() > 0
            && model.posed_point_check(sig, g, &xb)
            && model.posed_point_check(sig, g, &xa))
        {
            return Err(EngineError("a straddle certificate failed its exact re-check".into()));
        }
        let (orbit, transport) = ctx.regions.rep_of(p as usize);
        Ok(Certificate {
            piece: p,
            grip: e,
            orbit,
            transport,
            vertex_below: jneg,
            vertex_above: jpos,
            point_below: xb,
            point_above: xa,
            h_below: hb,
            h_above: ha,
        })
    }

    /// An independent check of a certificate against this state: both points lie in the piece's posed region, and
    /// h < 0 at the first and h > 0 at the second.
    pub fn check_certificate(&self, ctx: &Context, cap: usize, piece: usize, below: &Vec4, above: &Vec4) -> bool {
        if cap >= crate::model::POLES || piece >= NP {
            return false;
        }
        let model = &ctx.model;
        let g = self.pose_matrix(ctx, self.pose_id[piece]);
        let sig = model.signature(piece);
        model.posed_point_check(sig, g, below)
            && model.posed_point_check(sig, g, above)
            && model.h(cap, below).sign() < 0
            && model.h(cap, above).sign() > 0
    }

    /// Applies a twist if it is certified admissible and clears the protection; otherwise nothing changes.
    /// `protect` lists protected pieces (selection 4).
    pub fn apply(
        &mut self,
        ctx: &Context,
        twist: &Twist,
        budget: Option<u64>,
        protect: &[u32],
    ) -> Result<Outcome, EngineError> {
        if let Some(menu) = &self.menu {
            if !menu.contains(ctx, twist) {
                return Ok(Outcome::refused(
                    Status::Invalid,
                    twist.grip,
                    codes::TWIST_OFF_MENU,
                    format!("The twist is not in menu {}", menu.name),
                ));
            }
        }
        let cl = self.classify(ctx, twist.grip, true, true, budget)?;
        if cl.status() != Status::Admissible {
            return Ok(Outcome::from_classification(cl));
        }
        let mut cl = cl;
        let inside = cl.inside.take().unwrap_or_default();
        let mut outcome = Outcome::from_classification(cl);
        if !protect.is_empty() {
            let mut protected = protect.to_vec();
            protected.sort_unstable();
            protected.dedup();
            // The pose of an inside piece changes from g_A to g g_A, so it changes exactly when g is not the identity.
            let pieces = if twist.kidx == Some(0) {
                Vec::new()
            } else {
                protected.iter().copied().filter(|p| inside.binary_search(p).is_ok()).collect()
            };
            let report = PoseProtection { protected, pieces };
            if report.moves() {
                outcome.status = Status::Protected;
                outcome.refusal = Some((
                    codes::PIECE_WOULD_MOVE,
                    format!("The twist would change the pose of protected piece {}", report.pieces[0]),
                ));
                outcome.protection = Some(report);
                return Ok(outcome);
            }
            outcome.protection = Some(report);
        }
        self.commit(ctx, twist, &inside);
        outcome.inside = inside;
        Ok(outcome)
    }

    /// Replaces g_A by g g_A for the inside pieces and makes the table canonical.
    fn commit(&mut self, ctx: &Context, twist: &Twist, inside: &[u32]) {
        let mut uniq: Vec<u32> = inside.iter().map(|&p| self.pose_id[p as usize]).collect();
        uniq.sort_unstable();
        uniq.dedup();
        let mut new_of = HashMap::with_capacity(uniq.len());
        for &u in &uniq {
            let n = match (self.poses[u as usize].kidx(), twist.kidx) {
                (Some(k), Some(t)) => self.intern(Pose::K(ctx.kplus.compose(t, k))),
                _ => {
                    let m = matmul(&twist.matrix, self.pose_matrix(ctx, u));
                    self.intern_matrix(ctx, m)
                }
            };
            new_of.insert(u, n);
        }
        for &p in inside {
            let id = &mut self.pose_id[p as usize];
            *id = new_of[&*id];
        }
        self.compact();
        self.invalidate();
    }

    /// Drops unused poses and orders the rest canonically: identity first, then by exact key.
    fn compact(&mut self) {
        let mut used = vec![false; self.poses.len()];
        for &id in &self.pose_id {
            used[id as usize] = true;
        }
        let mut rest: Vec<u32> = (1..self.poses.len() as u32).filter(|&i| used[i as usize]).collect();
        rest.sort_by_cached_key(|&i| self.poses[i as usize].sort_key());
        let keep: Vec<u32> = std::iter::once(0).chain(rest).collect();
        if keep.iter().enumerate().all(|(n, &i)| n as u32 == i) && keep.len() == self.poses.len() {
            return;
        }
        let mut remap = vec![0u32; self.poses.len()];
        for (n, &i) in keep.iter().enumerate() {
            remap[i as usize] = n as u32;
        }
        for id in self.pose_id.iter_mut() {
            *id = remap[*id as usize];
        }
        self.poses = keep.iter().map(|&i| self.poses[i as usize].clone()).collect();
        self.lookup = self.poses.iter().enumerate().map(|(n, p)| (p.clone(), n as u32)).collect();
        self.invalidate();
    }

    /// The canonical digest: SHA-256 over the spaced JSON matrix and the sorted little-endian member IDs of every moved
    /// pose group, in key order. Equal configurations have equal digests.
    pub fn digest(&self, ctx: &Context) -> String {
        let mut members: Vec<Vec<u32>> = vec![Vec::new(); self.poses.len()];
        for (p, &id) in self.pose_id.iter().enumerate() {
            if id != 0 {
                members[id as usize].push(p as u32);
            }
        }
        let mut items: Vec<(String, &[u32])> = members
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.is_empty())
            .map(|(id, m)| (matrix_spaced(self.pose_matrix(ctx, id as u32)), m.as_slice()))
            .collect();
        items.sort();
        let mut bytes = Vec::new();
        for (key, m) in items {
            bytes.extend_from_slice(key.as_bytes());
            for &p in m {
                bytes.extend_from_slice(&(p as i32).to_le_bytes());
            }
        }
        c600_digest::sha256_hex(&bytes)
    }

    /// SHA-256 over one byte per piece: 1 on the lattice, 0 off it.
    pub fn lattice_digest(&self) -> String {
        let flags: Vec<u8> = self.pose_id.iter().map(|&id| self.poses[id as usize].kidx().is_some() as u8).collect();
        c600_digest::sha256_hex(&flags)
    }

    /// The status of each grip, each classified exactly (the scan stops at the first straddler).
    pub fn survey(&mut self, ctx: &Context, grips: &[usize], budget: Option<u64>) -> Result<Vec<Status>, EngineError> {
        grips.iter().map(|&e| Ok(self.classify(ctx, e, false, true, budget)?.status())).collect()
    }

    /// Every certified straddler of cap `cap`; pieces a budget leaves unclassified count as uncertain only.
    pub fn straddler_set(
        &mut self,
        ctx: &Context,
        cap: usize,
        budget: Option<u64>,
    ) -> Result<StraddlerSet, EngineError> {
        let (model, kp, reg) = (&ctx.model, &ctx.kplus, &ctx.regions);
        let mut cl = Classification::new(cap);
        cl.counts.signature = (NP - self.off_lattice_count()) as u64;
        let mut certificates = Vec::new();
        let groups = self.groups();
        for (pid, members) in groups.iter() {
            let anchors = self.anchors(ctx, *pid, members)?;
            let ev = Evaluator::new(&pullback(self.pose_matrix(ctx, *pid), &model.poles[cap]));
            for (anchor, sub) in anchors.iter() {
                if budget.is_some_and(|b| cl.exact_evaluations >= b) {
                    cl.uncertain += sub.len() as u64;
                    continue;
                }
                match eval(&ev, reg.cap_vform(*anchor, kp, model), &mut cl, budget) {
                    Class::Uncertain(_) => {
                        cl.uncertain += sub.len() as u64;
                        continue;
                    }
                    Class::Straddle { .. } => {}
                    _ => {
                        cl.counts.group_superset += sub.len() as u64;
                        continue;
                    }
                }
                for &p in sub {
                    match eval(&ev, &reg.vform(p as usize, kp, model), &mut cl, budget) {
                        Class::Uncertain(_) => cl.uncertain += 1,
                        Class::Straddle { below, above, .. } => {
                            certificates.push(self.certificate(ctx, p, *pid, cap, below, above)?)
                        }
                        _ => cl.counts.piece_vertices += 1,
                    }
                }
            }
        }
        certificates.sort_by_key(|c| c.piece);
        Ok(StraddlerSet {
            pieces: certificates.iter().map(|c| c.piece).collect(),
            certificates,
            uncertain: cl.uncertain,
            counts: cl.counts,
            exact_evaluations: cl.exact_evaluations,
        })
    }

    /// The state record: the canonical pose table, the little-endian pose IDs and the digest.
    pub fn record(&self, ctx: &Context) -> Value {
        let table = self
            .poses
            .iter()
            .map(|p| match p {
                Pose::K(k) => json::object([("k", Value::Int(*k as i64))]),
                Pose::M(m) => json::object([("m", matrix_json(m))]),
            })
            .collect();
        let raw: Vec<u8> = self.pose_id.iter().flat_map(|id| id.to_le_bytes()).collect();
        json::object([
            ("pose_table", Value::Array(table)),
            (
                "pose_ids",
                json::object([
                    ("encoding", Value::String("u32le-base64".into())),
                    ("count", Value::Int(NP as i64)),
                    ("sha256", Value::String(c600_digest::sha256_hex(&raw))),
                    ("data", Value::String(b64::encode(&raw))),
                ]),
            ),
            ("digest", Value::String(self.digest(ctx))),
        ])
    }

    /// Reads a state record exactly. The table must start with the identity, hold no repeated pose, and keep every
    /// pose in K⁺ as its index; every matrix entry must be an exact rotation. The stated digest must match.
    pub fn from_record(ctx: &Context, menu: Option<Arc<Menu>>, record: &Value) -> Result<State, String> {
        let table = record.get("pose_table").and_then(Value::as_array).ok_or("a state record needs a pose table")?;
        let mut state = State::new(menu);
        state.poses.clear();
        state.lookup.clear();
        for (index, entry) in table.iter().enumerate() {
            let fields =
                entry.as_object().filter(|f| f.len() == 1).ok_or(format!("pose {index} needs one field, k or m"))?;
            let pose = if let Some(k) = fields.get("k") {
                let k = json_int(k).map_err(|e| format!("pose {index}: {e}"))?;
                let k =
                    k.to_u16().filter(|&k| (k as usize) < ORDER).ok_or(format!("pose {index}: k is not a K+ index"))?;
                Pose::K(k)
            } else if let Some(m) = fields.get("m") {
                let m = matrix_from_json(m).map_err(|e| format!("pose {index}: {e}"))?;
                if !is_rotation(&m) {
                    return Err(format!("pose {index} is not an exact rotation"));
                }
                if ctx.kplus.index_of_matrix(&m, &ctx.model).is_some() {
                    return Err(format!("pose {index} lies in K+ and must be stored by its index"));
                }
                Pose::M(Arc::new(m))
            } else {
                return Err(format!("pose {index} needs one field, k or m"));
            };
            if index == 0 && pose != Pose::K(0) {
                return Err("the pose table must start with the identity".into());
            }
            if state.lookup.insert(pose.clone(), index as u32).is_some() {
                return Err(format!("pose {index} repeats an earlier pose"));
            }
            state.poses.push(pose);
        }
        if state.poses.is_empty() {
            return Err("the pose table must start with the identity".into());
        }
        let ids = record.get("pose_ids").ok_or("a state record needs pose IDs")?;
        if ids.get("encoding").and_then(Value::as_str) != Some("u32le-base64")
            || ids.get("count").and_then(Value::as_i64) != Some(NP as i64)
        {
            return Err("pose IDs must be u32le-base64 with one ID per piece".into());
        }
        let raw = ids.get("data").and_then(Value::as_str).and_then(b64::decode).ok_or("pose IDs are not base64")?;
        if raw.len() != 4 * NP
            || ids.get("sha256").and_then(Value::as_str) != Some(c600_digest::sha256_hex(&raw).as_str())
        {
            return Err("pose IDs do not match their count and SHA-256".into());
        }
        for (p, chunk) in raw.chunks_exact(4).enumerate() {
            let id = u32::from_le_bytes(chunk.try_into().expect("four bytes"));
            if id as usize >= state.poses.len() {
                return Err(format!("piece {p} names no pose"));
            }
            state.pose_id[p] = id;
        }
        state.compact();
        if let Some(stated) = record.get("digest") {
            if stated.as_str() != Some(state.digest(ctx).as_str()) {
                return Err("the state record's digest differs from its poses".into());
            }
        }
        Ok(state)
    }
}
