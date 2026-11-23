//! What stage 19's sub-checkers share: the stored pose tables read and validated in full, the typed height metrics,
//! exact big-integer ranks, letters replayed with the full matrix contract, and the Γ(Y) join.
//!
//! Every value is recomputed from the stored bytes, the model in `assets/` and the exact engines. Nothing reads a
//! producer's cache, a review receipt's flag or a historical source digest: the archive's original-source digests
//! identify the captured edition and stay with PS3's import (LB5.14), never tested here against the public bytes.
//!
//! - A pose table is `{poses, pose_id, digest, journal}`: every pose a 4 × 4 matrix of lowest-terms `[a, b, d]`
//!   triples, each (a + b√5)/d, that is an exact rotation; the identity first; no pose repeated; one pose ID per
//!   piece; an empty journal; and the stated digest is the state digest the engine computes from those poses. A pose
//!   no piece uses is allowed (the joined cleanup table keeps one) and leaves the state unchanged.
//!   A stored member may be gzip; it is decoded within a stated bound.
//! - Metrics follow the stage 19 README: h is a piece's left height (every right height must be 0), the physical type
//!   of a piece is its K⁺-orbit in the model's order, q_t = Σ max(h − 2, 0)/2 over type t, Φ = Σ max(h − 2, 0), H the
//!   largest height, n the off-lattice count and the high count the pieces with h > 2.
//! - Ranks: w_t = Π_{s>t} (1 + L·N_s) over the model's type counts N_s, R_L = Σ w_t q_t and S = (N + 1)·R + n, as
//!   exact integers written in decimal.
//! - A letter `[cap, item]` of I_a is applied by the engine; the checker then compares every piece's new pose with
//!   g·(old pose) inside the moved set and with the old pose outside it (the chronological matrix contract).
//! - Γ(Y)'s join Z = Y·γ holds when every piece p has Z_p = Y_σ(p)·t_p, with t_p a K⁺ element.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use c600_jumble::model::NP;
use c600_jumble::q5::{is_rotation, matmul, matrix_from_json, Mat4, Vec4, Q5};
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{levels, Context};
use c600_kernel::json::{self, Value};

use crate::claims::{array, at, field, hex64, int, keys, not_applicable, reject, start_mismatch, Run, Step, Stop};
use crate::replay::{inverse_item, left_facts, letter, tree_tables, Jumbling};
use crate::report::codes;

/// The bound on a decoded member: 64 MiB. Stage 19's largest pose table decodes to under 1 MiB.
pub const DECODED_LIMIT: usize = 1 << 26;

/// What every stage 19 sub-checker answers for the claims outside the self-contained families of the claim matrix
/// (CX.14 to CX.17): not applicable, never a pass.
pub const OUTSIDE: &str = "Stage 19's q11 audit (its type-lex-successor-base-cache.npz is absent), its restricted \
     low tails (their array caches are absent), D[k] admission (no admission implementation exists), the remaining \
     obligations (global claims; unsupported) and the synthetic state's reachability (unknown) have no KN6 check \
     here (LB5.4, LB5.16, LB5.17)";

/// The scope note on q11's lower bound (CX.15, LB5.17).
pub const Q11: &str = "q11's lower bound is not checked: its audit needs type-lex-successor-base-cache.npz, which \
     the archive lacks, and the stored three-letter upper witness alone cannot discharge it (CX.15, LB5.17)";

/// A certificate outside the sub-checker's kind: q11's certificate by its own reason, anything else by [`OUTSIDE`].
pub fn outside(id: &str, reads: &str, doc: &Value) -> Stop {
    if doc.get("exact_base13_q11_elimination_distance").is_some() || doc.get("q11_depth_certificate").is_some() {
        return not_applicable(format!("{id} reads {reads}. {Q11}"));
    }
    not_applicable(format!("{id} reads {reads}; this certificate is outside its kind. {OUTSIDE}"))
}

/// The keys of `type-lex-trimmed-independent-results.json`, whose frontier claims `s19.frontier` checks and whose rank
/// anchors `s19.ranks` checks.
pub const TRIMMED: [&str; 27] = [
    "status",
    "source_ref",
    "new_State_apply_letters",
    "fresh_classified_forward_cuts",
    "fresh_physical_region_tests",
    "prefixes",
    "findings",
    "phase",
    "inputs_sha256",
    "source_hashes",
    "all12_disjoint_pairs",
    "cap282_noninteraction",
    "all177120_short_endpoint_match",
    "independent_inverse_all177120_exact",
    "all177120_long_equals_short_plus_recorded_D",
    "canonical_short_metrics",
    "canonical_long_metrics",
    "D_exact_q_neutral_off_drop",
    "one_fixed_L3",
    "one_fixed_S",
    "all_five_short_forward_prefixes_H4",
    "q11_depth_certificate",
    "generic_mixed_rank_proof_check",
    "canonical_nonK_tree_factors",
    "fresh_unique_pose_height_cache",
    "reused_cache_premise",
    "scope",
];

/// The form of a record among a sub-checker's forms: the one sharing more than half of its top-level keys with the
/// document. A record of that form with a key missing, added or renamed is then a malformed record of the form, and
/// the caller's `keys` check fails it by name; only a document of none of the forms is outside the kind.
pub fn form(doc: &Value, forms: &[&[&str]]) -> Option<usize> {
    let map = doc.as_object()?;
    forms
        .iter()
        .enumerate()
        .map(|(i, keys)| (i, keys.iter().filter(|k| map.contains_key(**k)).count(), keys.len()))
        .filter(|&(_, shared, len)| 2 * shared > len)
        .max_by(|a, b| (a.1 * b.2).cmp(&(b.1 * a.2)))
        .map(|(i, _, _)| i)
}

/// A failure of a stated input recast as the wrong start table it shows.
pub fn as_start(stop: Stop) -> Stop {
    match stop {
        Stop::Fail { claim, reason, .. } => Stop::Fail { claim, code: codes::START_TABLE_MISMATCH, reason },
        other => other,
    }
}

/// Whether bytes are a gzip member.
pub fn is_gzip(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

/// A stored member's decoded bytes: a gzip member inflated within `limit`, anything else as it is.
pub fn decode<'b>(bytes: &'b [u8], limit: usize) -> Result<Cow<'b, [u8]>, String> {
    if is_gzip(bytes) {
        c600_kernel::archive::gunzip(bytes, limit)
            .map(Cow::Owned)
            .map_err(|error| format!("the gzip member: {}", error.0))
    } else {
        Ok(Cow::Borrowed(bytes))
    }
}

/// A JSON input, decoded if stored as gzip.
pub fn json_input(run: &Run, role: &str) -> Step<Value> {
    let bytes = run.input(role)?;
    let decoded = decode(bytes, DECODED_LIMIT).map_err(|error| reject(role, error))?;
    json::parse(&decoded).map_err(|error| reject(role, format!("not JSON: {error}")))
}

// ------------------------------------------------------------------------------------------------------ tables

/// A stored pose table, validated in full.
pub struct Table {
    pub state: State,
    /// The state digest, computed by the engine and equal to the table's own stated digest.
    pub digest: String,
    pub poses: usize,
}

/// Reads input `role` as a pose table and binds it with its state digest. Any defect is the wrong start table.
pub fn read_table(ctx: &Context, menu: Option<Arc<Menu>>, run: &mut Run, role: &str) -> Step<Table> {
    let bytes = run.input(role).map_err(as_start)?;
    let table = parse_table(ctx, menu, bytes, role)?;
    run.bind(role, bytes, vec![("digest", table.digest.clone()), ("poses", table.poses.to_string())]);
    Ok(table)
}

/// Validates a stored pose table (see the module notes); every failure names `role` under
/// `KN6:evidence.start_table_mismatch`.
pub fn parse_table(ctx: &Context, menu: Option<Arc<Menu>>, bytes: &[u8], role: &str) -> Step<Table> {
    let wrong = |reason: String| start_mismatch(role, reason);
    let decoded = decode(bytes, DECODED_LIMIT).map_err(wrong)?;
    let doc = json::parse(&decoded).map_err(|error| wrong(format!("the table is not JSON: {error}")))?;
    keys(&doc, role, &["poses", "pose_id", "digest", "journal"], &[]).map_err(as_start)?;
    let journal = array(field(&doc, "journal", role)?, role).map_err(as_start)?;
    if !journal.is_empty() {
        return Err(wrong(format!("a pose table carries no journal; this one has {} records", journal.len())));
    }
    let stated = hex64(field(&doc, "digest", role)?, role).map_err(as_start)?.to_owned();
    let entries = array(field(&doc, "poses", role)?, role).map_err(as_start)?;
    let mut table = Vec::with_capacity(entries.len());
    let mut seen = HashSet::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let m = matrix_from_json(entry).map_err(|error| wrong(format!("pose {index}: {error}")))?;
        if !is_rotation(&m) {
            return Err(wrong(format!("pose {index} is not an exact rotation")));
        }
        let pose = match ctx.kplus.index_of_matrix(&m, &ctx.model) {
            Some(k) => Pose::K(k),
            None => Pose::M(Arc::new(m)),
        };
        if index == 0 && pose != Pose::K(0) {
            return Err(wrong("the first pose of a table is the identity".into()));
        }
        if !seen.insert(pose.clone()) {
            return Err(wrong(format!("pose {index} repeats an earlier pose")));
        }
        table.push(pose);
    }
    let ids = array(field(&doc, "pose_id", role)?, role).map_err(as_start)?;
    if ids.len() != NP {
        return Err(wrong(format!("the table names {} pieces, not {NP}", ids.len())));
    }
    let mut per_piece = Vec::with_capacity(NP);
    for (piece, id) in ids.iter().enumerate() {
        let id = id.as_i64().and_then(|id| usize::try_from(id).ok()).filter(|&id| id < table.len());
        let id = id.ok_or_else(|| wrong(format!("piece {piece} names no pose of the table")))?;
        per_piece.push(table[id].clone());
    }
    let state = levels::state_of(ctx, menu, &per_piece).map_err(wrong)?;
    let digest = state.digest(ctx);
    if digest != stated {
        return Err(wrong(format!("the table states digest {stated} but its poses give {digest}")));
    }
    Ok(Table { state, digest, poses: table.len() })
}

// ----------------------------------------------------------------------------------------------------- metrics

/// The typed height metrics of a state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metrics {
    pub q: Vec<i64>,
    pub n: i64,
    pub h: i64,
    pub phi: i64,
    pub high: i64,
}

/// The physical type of every piece: its K⁺-orbit, in the model's order.
pub fn types(ctx: &Context) -> &[u16] {
    &ctx.regions.orbit_of
}

/// The number of pieces of each physical type, counted from the model.
pub fn type_counts(ctx: &Context) -> Vec<i64> {
    let mut counts = vec![0i64; ctx.regions.reps.len()];
    for &t in types(ctx) {
        counts[t as usize] += 1;
    }
    counts
}

/// The left height of every pose of the state's table, recomputed from the engine's exact tree facts.
pub fn pose_heights(ctx: &Context, state: &State, claim: &str) -> Step<Vec<i64>> {
    let tables = tree_tables(ctx, claim)?;
    state.poses().iter().map(|pose| left_facts(ctx, tables, pose, claim).map(|(h, _)| h)).collect()
}

/// The metrics of a state, recomputed in full.
pub fn metrics(ctx: &Context, state: &State, claim: &str) -> Step<Metrics> {
    let heights = pose_heights(ctx, state, claim)?;
    let types = types(ctx);
    let mut twice = vec![0i64; ctx.regions.reps.len()];
    let (mut h, mut phi, mut high) = (0i64, 0i64, 0i64);
    for (piece, &id) in state.pose_ids().iter().enumerate() {
        let height = heights[id as usize];
        h = h.max(height);
        if height > 2 {
            phi += height - 2;
            high += 1;
            twice[types[piece] as usize] += height - 2;
        }
    }
    let mut q = Vec::with_capacity(twice.len());
    for (t, excess) in twice.into_iter().enumerate() {
        if excess % 2 != 0 {
            return Err(reject(claim, format!("type {t} has an odd height excess {excess}")));
        }
        q.push(excess / 2);
    }
    Ok(Metrics { q, n: state.off_lattice_count() as i64, h, phi, high })
}

/// The left height of one piece.
pub fn height(ctx: &Context, state: &State, piece: usize, claim: &str) -> Step<i64> {
    let tables = tree_tables(ctx, claim)?;
    left_facts(ctx, tables, state.pose_of(piece), claim).map(|(h, _)| h)
}

/// Whether the state is the solved state: every piece at the identity pose. A state with an off-lattice piece is not.
pub fn solved(state: &State) -> bool {
    state.moved_count() == 0
}

/// An integer vector claim.
pub fn ints(value: &Value, claim: &str) -> Step<Vec<i64>> {
    array(value, claim)?.iter().enumerate().map(|(k, v)| int(v, &at(claim, k))).collect()
}

/// The first type whose entry is nonzero, with that entry.
pub fn first_change(delta: &[i64]) -> Option<(usize, i64)> {
    delta.iter().position(|&d| d != 0).map(|t| (t, delta[t]))
}

/// The difference of two vectors.
pub fn minus(a: &[i64], b: &[i64]) -> Vec<i64> {
    a.iter().zip(b).map(|(x, y)| x - y).collect()
}

// ------------------------------------------------------------------------------------------------------- ranks

/// An exact integer.
pub fn big(n: i64) -> Q5 {
    Q5::int(n)
}

/// An exact integer in decimal, without leading zeros.
pub fn decimal(x: &Q5) -> String {
    match x.to_json() {
        Value::Array(parts) => match parts.first() {
            Some(Value::Int(n)) => n.to_string(),
            Some(Value::Number(text)) => text.clone(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// 1 + L·N, as an exact integer (stated values never overflow a machine word here).
pub fn factor(l: i64, n: i64) -> Q5 {
    big(1).add(&big(l).mul(&big(n)))
}

/// w_t = Π_{s>t} (1 + L·N_s), as exact integers.
pub fn weights(counts: &[i64], l: i64) -> Vec<Q5> {
    let mut w = vec![big(1); counts.len()];
    for t in (0..counts.len().saturating_sub(1)).rev() {
        w[t] = w[t + 1].mul(&factor(l, counts[t + 1]));
    }
    w
}

/// Σ w_t v_t.
pub fn rank(weights: &[Q5], v: &[i64]) -> Q5 {
    weights.iter().zip(v).fold(big(0), |sum, (w, &x)| sum.add(&w.mul(&big(x))))
}

/// S = (N + 1)·R + n.
pub fn s_rank(r: &Q5, n: i64) -> Q5 {
    r.mul(&big(NP as i64 + 1)).add(&big(n))
}

/// The decimal claims of a list of exact integers.
pub fn decimals(xs: &[Q5]) -> Vec<String> {
    xs.iter().map(decimal).collect()
}

/// A list of decimal strings.
pub fn strings(value: &Value, claim: &str) -> Step<Vec<String>> {
    array(value, claim)?
        .iter()
        .enumerate()
        .map(|(k, v)| v.as_str().map(str::to_owned).ok_or_else(|| reject(at(claim, k), "expected a decimal string")))
        .collect()
}

/// Compares a decimal claim with an exact integer.
pub fn claim_decimal(run: &mut Run, claim: &str, value: &Value, computed: &Q5) -> Step<()> {
    let stated = value.as_str().ok_or_else(|| reject(claim, "expected a decimal string"))?.to_owned();
    run.claim(claim, &stated, &decimal(computed))
}

/// Compares an integer-vector claim.
pub fn claim_ints(run: &mut Run, claim: &str, value: &Value, computed: &[i64]) -> Step<()> {
    let stated = ints(value, claim)?;
    run.claim(claim, &format!("{stated:?}"), &format!("{computed:?}"))
}

/// Compares a list claim element by element, after its length.
pub fn claim_strings(run: &mut Run, claim: &str, stated: &[String], computed: &[String]) -> Step<()> {
    if stated.len() != computed.len() {
        return Err(reject(claim, format!("{} entries, not {}", stated.len(), computed.len())));
    }
    for (k, (s, c)) in stated.iter().zip(computed).enumerate() {
        run.claim(&at(claim, k), s, c)?;
    }
    Ok(())
}

/// A JSON boolean.
pub fn boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

/// Compares a boolean claim.
pub fn claim_bool(run: &mut Run, claim: &str, value: &Value, computed: bool) -> Step<()> {
    let stated = boolean(value).ok_or_else(|| reject(claim, "expected a boolean"))?;
    run.claim(claim, &stated, &computed)
}

/// Compares an integer claim.
pub fn claim_int(run: &mut Run, claim: &str, value: &Value, computed: i64) -> Step<()> {
    let stated = int(value, claim)?;
    run.claim(claim, &stated, &computed)
}

// ----------------------------------------------------------------------------------------------------- letters

/// The I_a menu of input `role`, checked by identity and order.
pub fn ia_menu(ctx: &Context, run: &mut Run, role: &str) -> Step<Arc<Menu>> {
    let menu = crate::replay::read_menu(ctx, run, role)?;
    run.claim(&format!("{role}.identity"), &c600_jumble::nu::I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim(&format!("{role}.order"), &c600_jumble::nu::I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    Ok(menu)
}

/// A list of `[cap, item]` letters.
pub fn letters(value: &Value, claim: &str, items: usize) -> Step<Vec<(usize, usize)>> {
    array(value, claim)?.iter().enumerate().map(|(k, v)| letter(v, &at(claim, k), items)).collect()
}

/// The letter that undoes `(cap, item)`: the same cap and the inverse item.
pub fn inverse(menu: &Menu, (cap, item): (usize, usize), claim: &str) -> Step<(usize, usize)> {
    inverse_item(menu, item).map(|inv| (cap, inv)).ok_or_else(|| reject(claim, format!("item {item} has no inverse")))
}

/// The twist of a letter: the menu item transported to its cap by the retained frame.
pub fn twist(ctx: &Context, menu: &Arc<Menu>, (cap, item): (usize, usize), claim: &str) -> Step<Twist> {
    Jumbling::new(ctx, menu.clone(), State::new(None)).twist(cap, item, claim)
}

/// One applied letter: the moved pieces, ascending, and whether every piece kept the matrix contract.
pub struct Applied {
    pub inside: Vec<u32>,
    pub contract: bool,
}

/// Applies a letter that must be admissible and checks the chronological matrix contract on every piece.
pub fn step(ctx: &Context, state: &mut State, twist: &Twist, claim: &str) -> Step<Applied> {
    let before = state.clone();
    let outcome = state
        .apply(ctx, twist, None, &[])
        .map_err(|error| reject(claim, format!("the engine stopped: {}", error.0)))?;
    if outcome.status != Status::Admissible {
        return Err(reject(claim, format!("the letter is {} at this state", outcome.status.as_str())));
    }
    let contract = contract(ctx, &before, state, &outcome.inside, &twist.matrix);
    Ok(Applied { inside: outcome.inside, contract })
}

/// Every piece of `after` has pose g·(its pose in `before`) when it moved and its old pose otherwise.
pub fn contract(ctx: &Context, before: &State, after: &State, inside: &[u32], g: &Mat4) -> bool {
    let mut moved = vec![false; NP];
    for &p in inside {
        moved[p as usize] = true;
    }
    let mut seen: HashMap<(u32, bool, u32), bool> = HashMap::new();
    for (piece, &inside) in moved.iter().enumerate() {
        let (old, new) = (before.pose_ids()[piece], after.pose_ids()[piece]);
        let held = *seen.entry((old, inside, new)).or_insert_with(|| {
            let old = before.pose_matrix(ctx, old);
            let new = after.pose_matrix(ctx, new);
            if inside {
                matmul(g, old) == *new
            } else {
                old == new
            }
        });
        if !held {
            return false;
        }
    }
    true
}

/// The pieces whose exact pose matrices differ between two states, ascending.
pub fn changed(ctx: &Context, a: &State, b: &State) -> Vec<u32> {
    let mut seen: HashMap<(u32, u32), bool> = HashMap::new();
    (0..NP as u32)
        .filter(|&p| {
            let (x, y) = (a.pose_ids()[p as usize], b.pose_ids()[p as usize]);
            *seen.entry((x, y)).or_insert_with(|| a.pose_matrix(ctx, x) != b.pose_matrix(ctx, y))
        })
        .collect()
}

// ----------------------------------------------------------------------------------------------------- Γ(Y)

/// A gauge γ = (σ, t) of Γ(Y): piece p of Z occupies the region σ(p) occupies in Y, turned by the K⁺ element t_p.
pub struct Gamma {
    pub sigma: Vec<u32>,
    pub t: Vec<u16>,
}

impl Gamma {
    /// The identity except on the listed rows.
    pub fn with(rows: &[(u32, u32, u16)]) -> Gamma {
        let mut sigma: Vec<u32> = (0..NP as u32).collect();
        let mut t = vec![0u16; NP];
        for &(p, r, k) in rows {
            sigma[p as usize] = r;
            t[p as usize] = k;
        }
        Gamma { sigma, t }
    }

    /// Whether σ is a permutation of the pieces that keeps every piece's physical type.
    pub fn bijective_and_typed(&self, ctx: &Context) -> bool {
        let types = types(ctx);
        let mut taken = vec![false; NP];
        self.sigma.iter().enumerate().all(|(p, &r)| {
            (r as usize) < NP && !std::mem::replace(&mut taken[r as usize], true) && types[p] == types[r as usize]
        })
    }

    /// The pieces σ moves.
    pub fn moved(&self) -> usize {
        self.sigma.iter().enumerate().filter(|(p, &r)| *p != r as usize).count()
    }

    /// σ⁻¹ of a set: the pieces p with σ(p) in it, ascending.
    pub fn preimage(&self, set: &[u32]) -> Vec<u32> {
        let mut member = vec![false; NP];
        for &r in set {
            member[r as usize] = true;
        }
        (0..NP as u32).filter(|&p| member[self.sigma[p as usize] as usize]).collect()
    }
}

/// Z = Y·γ on all pieces: Z_p = Y_σ(p)·t_p, each product exact.
pub fn join(ctx: &Context, y: &State, z: &State, gamma: &Gamma) -> bool {
    let mut seen: HashMap<(u32, u16, u32), bool> = HashMap::new();
    (0..NP).all(|p| {
        let key = (y.pose_ids()[gamma.sigma[p] as usize], gamma.t[p], z.pose_ids()[p]);
        *seen.entry(key).or_insert_with(|| {
            let ym = y.pose_matrix(ctx, key.0);
            let zm = z.pose_matrix(ctx, key.2);
            if key.1 == 0 {
                ym == zm
            } else {
                matmul(ym, ctx.kplus.matrix(key.1, &ctx.model)) == *zm
            }
        })
    })
}

// ------------------------------------------------------------------------------------------------------ blocks

/// The blocks of Γ(Y)'s validation record: its word cut at its stated block boundaries, and the digest of the reference
/// table Y they start from.
pub struct Blocks {
    pub reference: String,
    pub names: Vec<String>,
    pub words: Vec<Vec<(usize, usize)>>,
}

impl Blocks {
    /// The letters of a named block.
    pub fn get(&self, name: &str) -> Step<&[(usize, usize)]> {
        self.names
            .iter()
            .position(|n| n == name)
            .map(|i| self.words[i].as_slice())
            .ok_or_else(|| reject("blocks", format!("the blocks name no block {name}")))
    }
}

/// Reads input `role` as Γ(Y)'s validation record for its `word`, its `boundaries` (prefix and block name) and its
/// `reference_input_digest`, and nothing else.
pub fn blocks(run: &mut Run, role: &str, items: usize) -> Step<Blocks> {
    let bytes = run.input(role)?;
    let doc = json_input(run, role)?;
    let reference = hex64(field(&doc, "reference_input_digest", role)?, role)?.to_owned();
    let word = letters(field(&doc, "word", role)?, &format!("{role}.word"), items)?;
    let bounds = array(field(&doc, "boundaries", role)?, role)?;
    let (mut previous, mut names, mut words) = (0usize, Vec::new(), Vec::new());
    for (j, bound) in bounds.iter().enumerate() {
        let path = at(&format!("{role}.boundaries"), j);
        let prefix = int(field(bound, "prefix", &path)?, &path)?;
        let prefix = usize::try_from(prefix).ok().filter(|&p| p > previous && p <= word.len());
        let prefix = prefix.ok_or_else(|| reject(&path, "the block prefixes increase within the word"))?;
        let name = field(bound, "block", &path)?.as_str().ok_or_else(|| reject(&path, "a block is named"))?;
        names.push(name.to_owned());
        words.push(word[previous..prefix].to_vec());
        previous = prefix;
    }
    if previous != word.len() || names.is_empty() {
        return Err(reject(role, "the blocks cover the word"));
    }
    run.bind(role, bytes, vec![("reference", reference.clone()), ("blocks", names.join(","))]);
    Ok(Blocks { reference, names, words })
}

/// Reads the source table Y (`origin`), which must be the blocks' reference, and the stored post-A table (`start`),
/// which block A from Y must reach exactly.
pub fn origin_and_start(ctx: &Context, run: &mut Run, menu: &Arc<Menu>, blocks: &Blocks) -> Step<(State, State)> {
    let origin = read_table(ctx, Some(menu.clone()), run, "origin")?;
    run.compare("origin", &blocks.reference.clone(), &origin.digest, codes::START_TABLE_MISMATCH)?;
    let start = read_table(ctx, Some(menu.clone()), run, "start")?;
    let mut after_a = origin.state.clone();
    for (i, &letter) in blocks.get("A")?.iter().enumerate() {
        let path = at("blocks.A", i);
        step(ctx, &mut after_a, &twist(ctx, menu, letter, &path)?, &path)?;
    }
    if after_a.digest(ctx) != start.digest {
        return Err(start_mismatch("start", "block A from the origin does not reach the start table"));
    }
    run.held("start", "block A from the origin reaches the stored post-A table");
    Ok((origin.state, start.state))
}

/// The home vertices of a piece, sorted.
pub fn home(ctx: &Context, piece: usize) -> Vec<Vec4> {
    let mut v = ctx.regions.vertices(piece, &ctx.kplus, &ctx.model);
    v.sort();
    v
}

/// The images of points under a matrix, sorted.
pub fn image(m: &Mat4, points: &[Vec4]) -> Vec<Vec4> {
    let mut out: Vec<Vec4> = points.iter().map(|v| c600_jumble::q5::matvec(m, v)).collect();
    out.sort();
    out
}

/// The K⁺ element t with Z_p = Y_r·t whose image carries piece p's home region onto piece r's, if there is one: the
/// edge (p, r) of the matching problem.
pub fn edge(ctx: &Context, y: &State, z: &State, p: usize, r: usize) -> Option<u16> {
    let types = types(ctx);
    if types[p] != types[r] {
        return None;
    }
    let t =
        matmul(&c600_jumble::q5::transpose(y.pose_matrix(ctx, y.pose_ids()[r])), z.pose_matrix(ctx, z.pose_ids()[p]));
    let k = ctx.kplus.index_of_matrix(&t, &ctx.model)?;
    (image(ctx.kplus.matrix(k, &ctx.model), &home(ctx, p)) == home(ctx, r)).then_some(k)
}
