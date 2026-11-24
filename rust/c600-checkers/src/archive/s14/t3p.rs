//! Stage 14's T3P rounds (LB5.15's first stage 14 family; the claim matrix's `14/t3p` groups): Theorem T3P's
//! hypotheses P0 to P4 at the state of each round, then the round's literal replay with its effects, for
//! `t3p-certificate/1` (the W3 and W5 certificates and each scan's best round), `t3p-check/1` (the archive's check of a
//! certificate, every stated number recomputed) and `t3p-chain/1` (a chain of rounds and its word).
//!
//! # A round (PROOFS.md section 2)
//!
//! At a configuration Z a round names a target grip x of class Q = CLS(x), a piece of the high unit G (the pieces with
//! its pose), a lowering item h and helper letters (y_i, g_i). Identity helpers are dropped, as the record contract
//! does; the helper grips must be distinct. From the definitions, with every label from the exact sign kernel and
//! every height and branch from the engine's exact tree facts:
//!
//! - I_i is the set of pieces inside cap y_i at Z, and the image of a piece of I_i is its pose times g_i. Z′ is the
//!   configuration with the images on ∪I_i and the Z-poses elsewhere, built directly from those poses.
//! - The unit is lowered when every member has height at least 4 and branch Q, lies inside x, h is a jumble, and h
//!   lowers it by 2. At each helper, F and D are Lemma Φ's counts at Z (D: the pieces of the helper's class and height
//!   at least 4 that g_i lowers by 2), and Lemma Φ's ΔΦ is 0 for a retained item and 2(F − D) for a jumble.
//! - The contributions at x are read at Z′: s_i, f_i, d_i over I_i and s₀, f₀, d₀ over the other pieces (straddlers of
//!   x; pieces inside x of height at least 2 and branch other than Q; pieces inside x of branch Q and height at least 4
//!   that h lowers by 2). ΔΦ_i is the change of Φ over I_i from Z to Z′.
//! - P0: x is no helper, the I_i are pairwise disjoint and avoid G, and every straddler of each y_i at Z lies in some I_l
//!   whose image sends it out of y_i (T3P+; a clean helper has no straddler). P1: the precedence digraph (an edge
//!   y_i → y_j when the image of a piece of I_j is not out at y_i, and y_l → y_i when I_l holds a straddler of y_i) is
//!   acyclic. P2: every helper is tame (Lemma Φ's ΔΦ is at most 0 and equals ΔΦ_i). P3: S = s₀ + Σ s_i = 0.
//!   P4: F = f₀ + Σ f_i < D = d₀ + Σ d_i.
//!
//! When P0 to P4 hold, the round is replayed on the exact engine: the helpers in a topological order (the stated one,
//! which must be one, or the least by grip), each admissible, moving exactly I_i and changing Φ by ΔΦ_i; the state
//! after them must be Z′ (Lemma A), hold G inside x and have Φ(Z) + Σ ΔΦ_i; then (x, h) is admissible, moves exactly
//! the inside set of x at Z′, lowers every member by 2 and ends at Φ(Z) + Σ ΔΦ_i + 2(F − D) (Theorem T3P). Rounds of a
//! certificate or chain run in order from the stated start table, each at the state the previous one reached
//! (Corollary I).
//!
//! # Scope
//!
//! A chain record's `stop` statuses, its single-letter count and each round's `statuses_before` are the outcomes of
//! `t3p-scan/1`, a finite search (the claim matrix's `14/finite_search` group of the same record); they are not
//! checked here and the trace says so. Records of the scan, pairs and plus formats are `not_applicable`.
//!
//! A check record's `prefix_word` is replayed from the table letter by letter, each letter admissible, and the state it
//! reaches must be round 0's `digest_before`. Its `dropped_identities` (the certificate's helpers of item 0) are
//! compared with the certificate input; without it a check record can name none.

use std::collections::{BTreeMap, BTreeSet};

use c600_jumble::model::NP;
use c600_jumble::observe::Tables;
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::Twist;
use c600_kernel::json::Value;

use super::common::{
    self, claim_bool, claim_int, claim_str, engine, grip, heights, image, intersection, labels, observe, piece, start,
    Form, Heights, Labels,
};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, int, keys, reject, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{left_facts, letter, tree_tables, Jumbling};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s14.t3p",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s14/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s14/t3p.rs", include_bytes!("t3p.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUBCHECKER: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 14,
    family: "S14 W3/W5, T3P/T3P+ rounds, P0 to P4 and chain effects",
};

pub const CERTIFICATE: &str = "t3p-certificate/1";
pub const CHECK: &str = "t3p-check/1";
pub const CHAIN: &str = "t3p-chain/1";
/// The optional input of a check record: the certificate it checks.
pub const CERTIFICATE_INPUT: &str = "certificate";

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engine(engines)?;
    let doc = document(run.cert.bytes)?;
    match common::own(&doc, SOURCES.id)? {
        Form::Certificate => certificate(ctx, run, &doc),
        Form::Check => check_record(ctx, run, &doc),
        Form::Chain => chain(ctx, run, &doc),
        other => Err(reject("format", format!("{other:?} is not a T3P round form"))),
    }
}

// ------------------------------------------------------------------------------------------------------ records

/// `t3p-certificate/1`: every round satisfies P0 to P4 and replays as Theorem T3P says.
fn certificate(ctx: &c600_jumble::Context, run: &mut Run, doc: &Value) -> Step<()> {
    keys(doc, "certificate", &["format", "rounds", "table_digest"], &[])?;
    let stated = hex64(field(doc, "table_digest", "table_digest")?, "table_digest")?.to_owned();
    let mut j = start(ctx, run, "table_digest", &stated)?;
    let tables = tree_tables(ctx, "rounds")?;
    let rounds = nonempty(field(doc, "rounds", "rounds")?, "rounds")?;
    for (k, value) in rounds.iter().enumerate() {
        let path = at("rounds", k);
        keys(value, &path, &["h", "helpers", "unit_piece", "x"], &[])?;
        let spec = spec(value, &path, j.menu.items.len(), field(value, "helpers", &path)?, None)?;
        let outcome = round(run, &mut j, tables, &path, &spec, None)?;
        require_hypotheses(run, &path, &outcome)?;
        require_replay(run, &path, &outcome)?;
    }
    Ok(())
}

/// `t3p-check/1`: the archive's check of a certificate, every stated number recomputed. With the input
/// [`CERTIFICATE_INPUT`], its start digest and rounds must be the ones the check names.
fn check_record(ctx: &c600_jumble::Context, run: &mut Run, doc: &Value) -> Step<()> {
    keys(
        doc,
        "certificate",
        &["cpu_s", "digest_after", "failures", "format", "passed", "prefix_word", "rounds", "setup"],
        &[],
    )?;
    common::seconds(field(doc, "cpu_s", "cpu_s")?, "cpu_s")?;
    common::setup(field(doc, "setup", "setup")?, "setup")?;
    let rounds = nonempty(field(doc, "rounds", "rounds")?, "rounds")?;
    let first = at("rounds", 0);
    let stated = hex64(field(&rounds[0], "digest_before", &first)?, &format!("{first}.digest_before"))?.to_owned();
    // The prefix word is replayed from the table; the state it reaches is round 0's.
    let prefix = field(doc, "prefix_word", "prefix_word")?;
    let mut j = common::start_after(ctx, run, &format!("{first}.digest_before"), &stated, prefix, "prefix_word")?;
    let items = j.menu.items.len();
    let certified = certified_rounds(run, items)?;
    if let Some((digest, specs)) = &certified {
        run.claim(&format!("{CERTIFICATE_INPUT}.table_digest"), digest, &stated)?;
        run.claim(&format!("{CERTIFICATE_INPUT}.rounds.length"), &specs.len(), &rounds.len())?;
    }
    let tables = tree_tables(ctx, "rounds")?;
    for (k, value) in rounds.iter().enumerate() {
        let path = at("rounds", k);
        keys(value, &path, &ROUND_KEYS, &[])?;
        common::seconds(field(value, "cpu_s", &path)?, &dot(&path, "cpu_s"))?;
        let rows = array(field(value, "helpers", &path)?, &dot(&path, "helpers"))?;
        let mut letters = Vec::with_capacity(rows.len());
        for (i, row) in rows.iter().enumerate() {
            let claim = at(&dot(&path, "helpers"), i);
            letters.push(Value::Array(vec![field(row, "grip", &claim)?.clone(), field(row, "item", &claim)?.clone()]));
        }
        let dropped = field(value, "dropped_identities", &path)?;
        let letters = Value::Array(letters);
        let spec = spec(value, &path, items, &letters, Some(dropped))?;
        // The dropped identity helpers are the certificate's helpers of item 0: they are compared with the
        // certificate input, and without it a check record can name none.
        let dp = dot(&path, "dropped_identities");
        let stated_dropped: Vec<String> = spec.dropped.iter().map(|&l| text(l)).collect();
        match &certified {
            Some((_, specs)) => {
                let named: Vec<String> = specs[k].dropped.iter().map(|&l| text(l)).collect();
                run.claim(&dp, &format!("[{}]", stated_dropped.join(", ")), &format!("[{}]", named.join(", ")))?;
            }
            None if !spec.dropped.is_empty() => {
                let stop = reject(&dp, "dropped identity helpers are checked against the certificate input");
                return Err(run.failed(stop, "no certificate input"));
            }
            None => run.held(&dp, "[] (no certificate input: no identity helper is named)"),
        }
        if let Some((_, specs)) = &certified {
            let named = &specs[k];
            let same =
                named.x == spec.x && named.h == spec.h && named.unit == spec.unit && named.helpers == spec.helpers;
            if !same {
                let stop = reject(&path, "the round is not the certificate's round");
                return Err(run.failed(stop, format!("{CERTIFICATE_INPUT}.rounds[{k}]")));
            }
            run.held(format!("{path}.certificate"), "the certificate's round");
        }
        let order = stated_order(value, &path)?;
        let outcome = round(run, &mut j, tables, &path, &spec, Some(&order))?;
        compare_round(run, value, &path, items, &outcome)?;
    }
    claim_str(run, doc, "", "digest_after", &j.digest())?;
    claim_bool(run, doc, "", "passed", true)?;
    empty_list(run, doc, "", "failures")
}

/// `t3p-chain/1`: the rounds of a chain in order from its start, each round's effects and the chain's word.
fn chain(ctx: &c600_jumble::Context, run: &mut Run, doc: &Value) -> Step<()> {
    keys(
        doc,
        "certificate",
        &[
            "cpu_s",
            "end_digest",
            "end_n4",
            "end_phi",
            "format",
            "name",
            "policy",
            "rounds",
            "start_digest",
            "start_n4",
            "start_phi",
            "stop",
            "table",
            "word",
        ],
        &[],
    )?;
    // The run's name, policy and table path describe it and bind nothing (LB5.14); its time is no claim.
    common::seconds(field(doc, "cpu_s", "cpu_s")?, "cpu_s")?;
    for key in ["name", "policy", "table"] {
        crate::claims::string(field(doc, key, key)?, key)?;
    }
    let stated = hex64(field(doc, "start_digest", "start_digest")?, "start_digest")?.to_owned();
    let mut j = start(ctx, run, "start_digest", &stated)?;
    let items = j.menu.items.len();
    let tables = tree_tables(ctx, "rounds")?;
    let seen = observe(ctx, tables, &j.state, "start_phi")?;
    claim_int(run, doc, "", "start_phi", seen.phi)?;
    claim_int(run, doc, "", "start_n4", seen.n4)?;
    let rounds = array(field(doc, "rounds", "rounds")?, "rounds")?;
    let mut word = Vec::new();
    for (k, value) in rounds.iter().enumerate() {
        let path = at("rounds", k);
        keys(value, &path, &CHAIN_ROUND_KEYS, &[])?;
        claim_int(run, value, &path, "round", k as i64 + 1)?;
        let spec = spec(value, &path, items, field(value, "helpers", &path)?, None)?;
        let order = stated_order(value, &path)?;
        let outcome = round(run, &mut j, tables, &path, &spec, Some(&order))?;
        require_hypotheses(run, &path, &outcome)?;
        require_replay(run, &path, &outcome)?;
        claim_int(run, value, &path, "S", outcome.s)?;
        claim_int(run, value, &path, "F", outcome.f)?;
        claim_int(run, value, &path, "D", outcome.d)?;
        claim_int(run, value, &path, "predicted_phi", outcome.predicted_phi)?;
        claim_int(run, value, &path, "phi_after", outcome.phi_after)?;
        claim_str(run, value, &path, "digest_after", &outcome.digest_after)?;
        claim_bool(run, value, &path, "passed", true)?;
        empty_list(run, value, &path, "failures")?;
        for row in &outcome.replay {
            word.push(row.letter);
        }
    }
    let stated_word = common::word(field(doc, "word", "word")?, "word", items)?;
    run.claim("word.length", &stated_word.len(), &word.len())?;
    for (k, (s, c)) in stated_word.iter().zip(&word).enumerate() {
        run.claim(&at("word", k), &text(*s), &text(*c))?;
    }
    let end = observe(ctx, tables, &j.state, "end_phi")?;
    claim_str(run, doc, "", "end_digest", &end.digest)?;
    claim_int(run, doc, "", "end_phi", end.phi)?;
    claim_int(run, doc, "", "end_n4", end.n4)?;
    let stop = field(doc, "stop", "stop")?;
    keys(stop, "stop", &["digest", "phi", "reason", "single_letter_descents", "statuses"], &[])?;
    claim_str(run, stop, "stop", "digest", &end.digest)?;
    claim_int(run, stop, "stop", "phi", end.phi)?;
    run.held(
        "scope",
        "rounds, word and chain effects; the scan outcomes (stop.reason, stop.statuses, stop.single_letter_descents \
         and each round's statuses_before) belong to the 14/finite_search group and are not checked",
    );
    Ok(())
}

const ROUND_KEYS: [&str; 31] = [
    "D",
    "F",
    "S",
    "cpu_s",
    "d0",
    "digest_before",
    "dropped_identities",
    "edges",
    "f0",
    "failures",
    "h",
    "helper_delta_phi",
    "helper_phi_equal",
    "helpers",
    "hypotheses",
    "intersections",
    "order",
    "passed",
    "phi_before",
    "predicted_delta_phi",
    "predicted_phi",
    "replay",
    "s0",
    "target_class",
    "unit_branch",
    "unit_height",
    "unit_held_before_lowering",
    "unit_lowering",
    "unit_piece",
    "unit_size",
    "x",
];

/// The fields of a chain's round; `statuses_before` is the scan's (unchecked, `14/finite_search`) but is part of the form.
const CHAIN_ROUND_KEYS: [&str; 15] = [
    "D",
    "F",
    "S",
    "digest_after",
    "failures",
    "h",
    "helpers",
    "order",
    "passed",
    "phi_after",
    "predicted_phi",
    "round",
    "statuses_before",
    "unit_piece",
    "x",
];

const HELPER_KEYS: [&str; 17] = [
    "D_at_helper",
    "F_at_helper",
    "clean",
    "d",
    "delta_phi",
    "delta_phi_formula",
    "evacuators",
    "f",
    "grip",
    "inside_size",
    "item",
    "s",
    "straddlers",
    "straddlers_not_sent_out",
    "tame",
    "uncovered_straddlers",
    "unit_intersection",
];

fn text((cap, item): (usize, usize)) -> String {
    format!("[{cap}, {item}]")
}

fn nonempty<'v>(value: &'v Value, claim: &str) -> Step<&'v [Value]> {
    let list = array(value, claim)?;
    if list.is_empty() {
        return Err(reject(claim, "a certificate has at least one round"));
    }
    Ok(list)
}

fn empty_list(run: &mut Run, obj: &Value, path: &str, key: &str) -> Step<()> {
    let claim = dot(path, key);
    let list = array(field(obj, key, path)?, &claim)?;
    if !list.is_empty() {
        return Err(run.failed(reject(&claim, "the recomputed round records no failure"), "[]"));
    }
    run.held(&claim, "[]");
    Ok(())
}

/// The rounds of the certificate a check record names, with its table digest.
fn certified_rounds(run: &mut Run, items: usize) -> Step<Option<(String, Vec<Spec>)>> {
    let Some(bytes) = run.optional_input(CERTIFICATE_INPUT) else { return Ok(None) };
    let doc = c600_kernel::json::parse(bytes)
        .map_err(|error| reject(CERTIFICATE_INPUT, format!("the certificate is not JSON: {error}")))?;
    if doc.get("format").and_then(Value::as_str) != Some(CERTIFICATE) {
        return Err(reject(CERTIFICATE_INPUT, format!("the certificate input is not {CERTIFICATE}")));
    }
    keys(&doc, CERTIFICATE_INPUT, &["format", "rounds", "table_digest"], &[])?;
    let digest = hex64(field(&doc, "table_digest", CERTIFICATE_INPUT)?, CERTIFICATE_INPUT)?.to_owned();
    let rounds = nonempty(field(&doc, "rounds", CERTIFICATE_INPUT)?, CERTIFICATE_INPUT)?;
    let mut specs = Vec::with_capacity(rounds.len());
    for (k, value) in rounds.iter().enumerate() {
        let path = format!("{CERTIFICATE_INPUT}.{}", at("rounds", k));
        keys(value, &path, &["h", "helpers", "unit_piece", "x"], &[])?;
        specs.push(spec(value, &path, items, field(value, "helpers", &path)?, None)?);
    }
    run.bind(CERTIFICATE_INPUT, bytes, vec![("table_digest", digest.clone())]);
    Ok(Some((digest, specs)))
}

// ------------------------------------------------------------------------------------------------------ a round

/// A round as named: target, lowering item, unit piece, the helpers (identity helpers dropped, sorted by grip and
/// item) and the dropped identities.
struct Spec {
    x: usize,
    h: usize,
    unit: usize,
    helpers: Vec<(usize, usize)>,
    dropped: Vec<(usize, usize)>,
}

/// Reads a round. `dropped` is a check record's own list of dropped identity helpers, which joins its helper rows.
fn spec(value: &Value, path: &str, items: usize, helpers: &Value, dropped: Option<&Value>) -> Step<Spec> {
    let x = grip(field(value, "x", path)?, &dot(path, "x"))?;
    let h = usize::try_from(int(field(value, "h", path)?, &dot(path, "h"))?)
        .ok()
        .filter(|&h| h < items)
        .ok_or_else(|| reject(dot(path, "h"), format!("an item is below {items}")))?;
    let unit = piece(field(value, "unit_piece", path)?, &dot(path, "unit_piece"))?;
    let mut raw = common::word(helpers, &dot(path, "helpers"), items)?;
    if let Some(dropped) = dropped {
        raw.extend(common::word(dropped, &dot(path, "dropped_identities"), items)?);
    }
    let mut helpers: Vec<(usize, usize)> = raw.iter().copied().filter(|&(_, i)| i != 0).collect();
    helpers.sort_unstable();
    let mut dropped: Vec<(usize, usize)> = raw.iter().copied().filter(|&(_, i)| i == 0).collect();
    dropped.sort_unstable();
    let grips: BTreeSet<usize> = helpers.iter().map(|&(c, _)| c).collect();
    if grips.len() != helpers.len() {
        return Err(reject(dot(path, "helpers"), "P0: the helper grips are not distinct"));
    }
    Ok(Spec { x, h, unit, helpers, dropped })
}

/// A record's stated topological order of the helper grips.
fn stated_order(value: &Value, path: &str) -> Step<Vec<usize>> {
    let claim = dot(path, "order");
    array(field(value, "order", path)?, &claim)?.iter().enumerate().map(|(k, v)| grip(v, &at(&claim, k))).collect()
}

/// One helper's facts at Z and its contributions at x.
#[derive(Default)]
struct Helper {
    grip: usize,
    item: usize,
    inside: Vec<u32>,
    unit_intersection: i64,
    clean: bool,
    f_at: i64,
    d_at: i64,
    formula: i64,
    delta_phi: i64,
    tame: bool,
    s: i64,
    f: i64,
    d: i64,
    straddlers: i64,
    uncovered: i64,
    not_sent_out: i64,
    evacuators: Vec<(usize, i64, i64)>,
}

/// One replayed letter.
struct Row {
    letter: (usize, usize),
    admissible: bool,
    moved: i64,
    moved_set_equal: bool,
    delta_phi_equal: Option<bool>,
    every_member_lowered: Option<bool>,
    digest: String,
    n4: i64,
    phi: i64,
}

/// Everything a round's check computes.
#[derive(Default)]
struct Outcome {
    target_class: i64,
    unit_size: i64,
    unit_height: i64,
    unit_branch: i64,
    digest_before: String,
    phi_before: i64,
    hypotheses: [bool; 5],
    failed: Vec<(usize, String)>,
    unit_lowering: bool,
    helpers: Vec<Helper>,
    intersections: Vec<(usize, usize, i64)>,
    edges: BTreeSet<(usize, usize)>,
    order: Vec<usize>,
    s0: i64,
    f0: i64,
    d0: i64,
    s: i64,
    f: i64,
    d: i64,
    helper_delta_phi: i64,
    predicted_phi: i64,
    replay: Vec<Row>,
    replayed: bool,
    helper_phi_equal: bool,
    held: bool,
    digest_after: String,
    phi_after: i64,
}

fn phi_of(h: i64) -> i64 {
    (h - 2).max(0)
}

/// The left height of the image of pose `pid` of `state` under `twist`, once per pose.
fn image_height(
    j: &Jumbling,
    tables: &Tables,
    state: &State,
    twist: &Twist,
    pid: u32,
    memo: &mut BTreeMap<u32, i64>,
    claim: &str,
) -> Step<i64> {
    if let Some(&h) = memo.get(&pid) {
        return Ok(h);
    }
    let (h, _) = left_facts(j.ctx, tables, &image(j.ctx, state, twist, pid), claim)?;
    memo.insert(pid, h);
    Ok(h)
}

/// The pieces of `pieces` inside a cap of class `class` that `twist` lowers by 2 from a height of at least 4 (Lemma
/// Φ's D), and the pieces of height at least 2 and another branch (its F).
#[allow(clippy::too_many_arguments)]
fn lemma_phi_counts(
    j: &Jumbling,
    tables: &Tables,
    state: &State,
    hz: &Heights,
    pieces: &[u32],
    class: i64,
    twist: &Twist,
    claim: &str,
) -> Step<(i64, i64)> {
    let (mut f, mut d) = (0, 0);
    let mut memo = BTreeMap::new();
    for &p in pieces {
        let (h, b) = hz.of(state, p as usize);
        if h >= 2 && b != class {
            f += 1;
        }
        if h >= 4 && b == class {
            let pid = state.pose_ids()[p as usize];
            if image_height(j, tables, state, twist, pid, &mut memo, claim)? == h - 2 {
                d += 1;
            }
        }
    }
    Ok((f, d))
}

/// The pieces of sorted `a` not in sorted `b`.
fn minus(a: &[u32], b: &[u32]) -> Vec<u32> {
    a.iter().copied().filter(|p| b.binary_search(p).is_err()).collect()
}

/// Whether the edges over `nodes` admit `order` as a topological order (a permutation of the nodes).
fn topological(nodes: &[usize], edges: &BTreeSet<(usize, usize)>, order: &[usize]) -> bool {
    let mut sorted = order.to_vec();
    sorted.sort_unstable();
    let mut expected = nodes.to_vec();
    expected.sort_unstable();
    if sorted != expected {
        return false;
    }
    let position: BTreeMap<usize, usize> = order.iter().enumerate().map(|(k, &c)| (c, k)).collect();
    edges.iter().all(|(a, b)| position[a] < position[b])
}

/// The least topological order by grip, if the digraph is acyclic.
fn least_order(nodes: &[usize], edges: &BTreeSet<(usize, usize)>) -> Option<Vec<usize>> {
    let mut left: BTreeSet<usize> = nodes.iter().copied().collect();
    let mut order = Vec::with_capacity(nodes.len());
    while !left.is_empty() {
        let next = *left.iter().find(|&&c| !edges.iter().any(|&(a, b)| b == c && left.contains(&a)))?;
        left.remove(&next);
        order.push(next);
    }
    Some(order)
}

/// Computes a round at the current state of `j` and, when P0 to P4 hold, replays it, leaving `j` at the round's end.
fn round(
    run: &mut Run,
    j: &mut Jumbling,
    tables: &Tables,
    path: &str,
    spec: &Spec,
    order: Option<&[usize]>,
) -> Step<Outcome> {
    let ctx = j.ctx;
    let z = j.state.clone();
    let hz = heights(ctx, tables, &z, path)?;
    let mut out = Outcome { digest_before: j.digest(), ..Outcome::default() };
    out.phi_before = (0..NP).map(|p| phi_of(hz.of(&z, p).0)).sum();
    let class = tables.cls(ctx, spec.x).map_err(|error| reject(path, format!("the class of the target: {error}")))?;
    out.target_class = class;
    let pid = z.pose_ids()[spec.unit];
    let members: Vec<u32> = (0..NP as u32).filter(|&p| z.pose_ids()[p as usize] == pid).collect();
    let (uh, ub) = hz.of(&z, spec.unit);
    (out.unit_size, out.unit_height, out.unit_branch) = (members.len() as i64, uh, ub);

    let grips: Vec<usize> = std::iter::once(spec.x).chain(spec.helpers.iter().map(|&(c, _)| c)).collect();
    let at_z = labels(ctx, &z, &grips, true, path)?;
    let lowering = j.twist(spec.x, spec.h, &dot(path, "h"))?;
    let high_own = uh >= 4 && ub == class && members.iter().all(|&p| common::holds(at_z.inside(spec.x), p));
    let lowered = lowering.kidx.is_none() && {
        let mut memo = BTreeMap::new();
        image_height(j, tables, &z, &lowering, pid, &mut memo, path)? == uh - 2
    };
    out.unit_lowering = high_own && lowered;
    if !out.unit_lowering {
        out.failed.push((0, "the target does not own a high unit that h lowers".into()));
    }

    // P0's disjointness, at Z.
    for &(c, i) in &spec.helpers {
        let inside = at_z.inside(c).to_vec();
        out.helpers.push(Helper {
            grip: c,
            item: i,
            unit_intersection: intersection(&inside, &members) as i64,
            clean: at_z.straddle(c).is_empty(),
            straddlers: at_z.straddle(c).len() as i64,
            inside,
            ..Helper::default()
        });
    }
    let mut p0 = !spec.helpers.iter().any(|&(c, _)| c == spec.x);
    for a in 0..out.helpers.len() {
        for b in a + 1..out.helpers.len() {
            let size = intersection(&out.helpers[a].inside, &out.helpers[b].inside) as i64;
            out.intersections.push((out.helpers[a].grip, out.helpers[b].grip, size));
            p0 &= size == 0;
        }
    }
    p0 &= out.helpers.iter().all(|h| h.unit_intersection == 0);
    if !p0 {
        out.hypotheses = [false, false, false, false, false];
        out.failed.push((0, "the helpers' inside sets meet each other, the unit or the target".into()));
        return Ok(out);
    }

    // Lemma Φ at each helper, and Z′.
    let mut union: Vec<u32> = Vec::new();
    let mut per_piece: Vec<Pose> = (0..NP).map(|p| z.pose_of(p).clone()).collect();
    let mut twists = Vec::with_capacity(spec.helpers.len());
    for (k, &(c, i)) in spec.helpers.iter().enumerate() {
        let claim = at(&dot(path, "helpers"), k);
        let twist = j.twist(c, i, &claim)?;
        let helper_class =
            tables.cls(ctx, c).map_err(|error| reject(&claim, format!("the class of the helper: {error}")))?;
        let (f, d) = lemma_phi_counts(j, tables, &z, &hz, &out.helpers[k].inside, helper_class, &twist, &claim)?;
        let helper = &mut out.helpers[k];
        (helper.f_at, helper.d_at) = (f, d);
        helper.formula = if twist.kidx.is_some() { 0 } else { 2 * (f - d) };
        let mut images: BTreeMap<u32, Pose> = BTreeMap::new();
        for &p in &helper.inside {
            let id = z.pose_ids()[p as usize];
            let pose = images.entry(id).or_insert_with(|| image(ctx, &z, &twist, id)).clone();
            per_piece[p as usize] = pose;
        }
        union.extend_from_slice(&helper.inside);
        twists.push(twist);
    }
    union.sort_unstable();
    let zp = c600_jumble::levels::state_of(ctx, Some(j.menu.clone()), &per_piece)
        .map_err(|error| reject(path, format!("the configuration after the helpers' images: {error}")))?;
    let hzp = heights(ctx, tables, &zp, path)?;
    let at_zp = labels(ctx, &zp, &grips, true, path)?;

    // Contributions at x, read at Z′.
    let x_inside = at_zp.inside(spec.x);
    let x_straddle = at_zp.straddle(spec.x);
    let mut memo = BTreeMap::new();
    let mut contributions = |pieces: &[u32]| -> Step<(i64, i64, i64)> {
        let s = intersection(x_straddle, pieces) as i64;
        let (mut f, mut d) = (0, 0);
        for &p in pieces {
            if !common::holds(x_inside, p) {
                continue;
            }
            let (h, b) = hzp.of(&zp, p as usize);
            if h >= 2 && b != class {
                f += 1;
            }
            if h >= 4 && b == class {
                let id = zp.pose_ids()[p as usize];
                if image_height(j, tables, &zp, &lowering, id, &mut memo, path)? == h - 2 {
                    d += 1;
                }
            }
        }
        Ok((s, f, d))
    };
    for helper in out.helpers.iter_mut() {
        (helper.s, helper.f, helper.d) = contributions(&helper.inside)?;
        helper.delta_phi =
            helper.inside.iter().map(|&p| phi_of(hzp.of(&zp, p as usize).0) - phi_of(hz.of(&z, p as usize).0)).sum();
        helper.tame = helper.formula <= 0 && helper.formula == helper.delta_phi;
    }
    let mut rest: Vec<u32> = minus(x_inside, &union);
    rest.extend(minus(x_straddle, &union));
    rest.sort_unstable();
    (out.s0, out.f0, out.d0) = contributions(&rest)?;

    // P1's precedence digraph and T3P+.
    let helper_grips: Vec<usize> = out.helpers.iter().map(|h| h.grip).collect();
    let not_out =
        |labels: &Labels, c: usize, p: u32| common::holds(labels.inside(c), p) || common::holds(labels.straddle(c), p);
    for a in 0..out.helpers.len() {
        for b in 0..out.helpers.len() {
            if a != b && out.helpers[b].inside.iter().any(|&p| not_out(&at_zp, out.helpers[a].grip, p)) {
                out.edges.insert((out.helpers[a].grip, out.helpers[b].grip));
            }
        }
    }
    for a in 0..out.helpers.len() {
        let c = out.helpers[a].grip;
        let straddlers = at_z.straddle(c);
        out.helpers[a].uncovered = minus(straddlers, &union).len() as i64;
        for b in 0..out.helpers.len() {
            let shared: Vec<u32> =
                out.helpers[b].inside.iter().copied().filter(|&p| common::holds(straddlers, p)).collect();
            if !shared.is_empty() {
                let bad = shared.iter().filter(|&&p| not_out(&at_zp, c, p)).count() as i64;
                let evacuator = out.helpers[b].grip;
                out.edges.insert((evacuator, c));
                out.helpers[a].evacuators.push((evacuator, shared.len() as i64, bad));
                out.helpers[a].not_sent_out += bad;
            }
        }
        p0 &= out.helpers[a].uncovered == 0 && out.helpers[a].not_sent_out == 0;
    }
    let acyclic = least_order(&helper_grips, &out.edges);
    out.order = match order {
        Some(order) => order.to_vec(),
        None => acyclic.clone().unwrap_or_default(),
    };

    out.s = out.s0 + out.helpers.iter().map(|h| h.s).sum::<i64>();
    out.f = out.f0 + out.helpers.iter().map(|h| h.f).sum::<i64>();
    out.d = out.d0 + out.helpers.iter().map(|h| h.d).sum::<i64>();
    out.helper_delta_phi = out.helpers.iter().map(|h| h.delta_phi).sum();
    out.predicted_phi = out.phi_before + out.helper_delta_phi + 2 * (out.f - out.d);
    out.hypotheses = [p0, acyclic.is_some(), out.helpers.iter().all(|h| h.tame), out.s == 0, out.f < out.d];
    for (k, &held) in out.hypotheses.iter().enumerate() {
        if !held {
            out.failed.push((k, format!("P{k} does not hold")));
        }
    }
    if !out.failed.is_empty() {
        return Ok(out);
    }
    if !topological(&helper_grips, &out.edges, &out.order) {
        let stop = reject(dot(path, "order"), "the order is not a topological order of the precedence digraph");
        return Err(run.failed(stop, format!("{:?}", acyclic.unwrap_or_default())));
    }

    // The replay (Lemma A, Theorem T3P).
    out.replayed = true;
    let mut phi = out.phi_before;
    for &c in &out.order {
        let k = out.helpers.iter().position(|h| h.grip == c).expect("the order names the helpers");
        let claim = at(&dot(path, "replay"), out.replay.len());
        let outcome = j.apply(&twists[k], &claim)?;
        let seen = observe(ctx, tables, &j.state, &claim)?;
        let admissible = outcome.status == Status::Admissible;
        let row = Row {
            letter: (c, out.helpers[k].item),
            admissible,
            moved: outcome.inside.len() as i64,
            moved_set_equal: admissible && outcome.inside == out.helpers[k].inside,
            delta_phi_equal: Some(admissible && seen.phi - phi == out.helpers[k].delta_phi),
            every_member_lowered: None,
            digest: seen.digest,
            n4: seen.n4,
            phi: seen.phi,
        };
        phi = seen.phi;
        let ok = row.admissible && row.moved_set_equal && row.delta_phi_equal == Some(true);
        out.replay.push(row);
        if !ok {
            return Ok(out);
        }
    }
    out.helper_phi_equal = phi == out.phi_before + out.helper_delta_phi;
    let lemma_a = j.digest() == zp.digest(ctx);
    if !lemma_a {
        let stop = reject(dot(path, "replay"), "the helpers' replay does not reach the configuration of their images");
        return Err(run.failed(stop, j.digest()));
    }
    run.held(format!("{path}.lemma_a"), "the helpers' replay reaches the configuration of their images");
    out.held = members.iter().all(|&p| common::holds(x_inside, p));
    if !out.held || !out.helper_phi_equal {
        return Ok(out);
    }
    let claim = at(&dot(path, "replay"), out.replay.len());
    let outcome = j.apply(&lowering, &claim)?;
    let seen = observe(ctx, tables, &j.state, &claim)?;
    let admissible = outcome.status == Status::Admissible;
    let after = heights(ctx, tables, &j.state, &claim)?;
    let lowered = admissible && members.iter().all(|&p| after.of(&j.state, p as usize).0 == uh - 2);
    out.replay.push(Row {
        letter: (spec.x, spec.h),
        admissible,
        moved: outcome.inside.len() as i64,
        moved_set_equal: admissible && outcome.inside == x_inside,
        delta_phi_equal: None,
        every_member_lowered: Some(lowered),
        digest: seen.digest.clone(),
        n4: seen.n4,
        phi: seen.phi,
    });
    out.digest_after = seen.digest;
    out.phi_after = seen.phi;
    Ok(out)
}

/// The round satisfies P0 to P4 (a certificate's or chain's claim).
fn require_hypotheses(run: &mut Run, path: &str, out: &Outcome) -> Step<()> {
    if let Some((k, reason)) = out.failed.first() {
        let claim = if out.unit_lowering { format!("{path}.P{k}") } else { format!("{path}.unit_piece") };
        return Err(run.failed(reject(&claim, reason.clone()), "false"));
    }
    run.held(
        format!("{path}.hypotheses"),
        format!("P0 to P4: S = {}, F = {}, D = {}, helpers ΔΦ = {}", out.s, out.f, out.d, out.helper_delta_phi),
    );
    Ok(())
}

/// The replay ran to its end and reached Theorem T3P's Φ.
fn require_replay(run: &mut Run, path: &str, out: &Outcome) -> Step<()> {
    let claim = dot(path, "replay");
    let Some(last) = out.replay.last() else {
        return Err(run.failed(reject(&claim, "the round was not replayed"), "no replay"));
    };
    for (k, row) in out.replay.iter().enumerate() {
        let ok = row.admissible
            && row.moved_set_equal
            && row.delta_phi_equal != Some(false)
            && row.every_member_lowered != Some(false);
        if !ok {
            let stop = reject(at(&claim, k), "the letter is refused, moves another set or changes Φ otherwise");
            return Err(run.failed(stop, text(row.letter)));
        }
    }
    if !out.helper_phi_equal || !out.held || last.every_member_lowered != Some(true) {
        let stop = reject(&claim, "the replay does not reach the lowering of the whole unit");
        return Err(run.failed(stop, text(last.letter)));
    }
    if out.phi_after != out.predicted_phi {
        let stop = reject(&claim, format!("Φ after the round is {}, not {}", out.phi_after, out.predicted_phi));
        return Err(run.failed(stop, out.phi_after));
    }
    run.held(&claim, format!("Φ {} → {} at {}", out.phi_before, out.phi_after, out.digest_after));
    Ok(())
}

/// A check record's round compared field by field with the recomputed round.
fn compare_round(run: &mut Run, value: &Value, path: &str, items: usize, out: &Outcome) -> Step<()> {
    claim_int(run, value, path, "target_class", out.target_class)?;
    claim_int(run, value, path, "unit_size", out.unit_size)?;
    claim_int(run, value, path, "unit_height", out.unit_height)?;
    claim_int(run, value, path, "unit_branch", out.unit_branch)?;
    claim_str(run, value, path, "digest_before", &out.digest_before)?;
    claim_int(run, value, path, "phi_before", out.phi_before)?;
    claim_bool(run, value, path, "unit_lowering", out.unit_lowering)?;
    let hypotheses = field(value, "hypotheses", path)?;
    let hp = dot(path, "hypotheses");
    keys(hypotheses, &hp, &["P0", "P1", "P2", "P3", "P4"], &[])?;
    for (k, &held) in out.hypotheses.iter().enumerate() {
        claim_bool(run, hypotheses, &hp, &format!("P{k}"), held)?;
    }
    // The helper rows, in the order of their grips and items.
    let rows = array(field(value, "helpers", path)?, &dot(path, "helpers"))?;
    run.claim(&format!("{path}.helpers.length"), &rows.len(), &out.helpers.len())?;
    for (k, (row, helper)) in rows.iter().zip(&out.helpers).enumerate() {
        let rp = at(&dot(path, "helpers"), k);
        keys(row, &rp, &HELPER_KEYS, &[])?;
        claim_int(run, row, &rp, "grip", helper.grip as i64)?;
        claim_int(run, row, &rp, "item", helper.item as i64)?;
        claim_int(run, row, &rp, "inside_size", helper.inside.len() as i64)?;
        claim_int(run, row, &rp, "unit_intersection", helper.unit_intersection)?;
        claim_bool(run, row, &rp, "clean", helper.clean)?;
        claim_int(run, row, &rp, "F_at_helper", helper.f_at)?;
        claim_int(run, row, &rp, "D_at_helper", helper.d_at)?;
        claim_int(run, row, &rp, "delta_phi", helper.delta_phi)?;
        claim_int(run, row, &rp, "delta_phi_formula", helper.formula)?;
        claim_bool(run, row, &rp, "tame", helper.tame)?;
        claim_int(run, row, &rp, "s", helper.s)?;
        claim_int(run, row, &rp, "f", helper.f)?;
        claim_int(run, row, &rp, "d", helper.d)?;
        claim_int(run, row, &rp, "straddlers", helper.straddlers)?;
        claim_int(run, row, &rp, "uncovered_straddlers", helper.uncovered)?;
        claim_int(run, row, &rp, "straddlers_not_sent_out", helper.not_sent_out)?;
        let ep = dot(&rp, "evacuators");
        let evacuators = array(field(row, "evacuators", &rp)?, &ep)?;
        run.claim(&format!("{ep}.length"), &evacuators.len(), &helper.evacuators.len())?;
        for (e, (stated, &(g, pieces, bad))) in evacuators.iter().zip(&helper.evacuators).enumerate() {
            let p = at(&ep, e);
            keys(stated, &p, &["grip", "not_out", "pieces"], &[])?;
            claim_int(run, stated, &p, "grip", g as i64)?;
            claim_int(run, stated, &p, "pieces", pieces)?;
            claim_int(run, stated, &p, "not_out", bad)?;
        }
    }
    let ip = dot(path, "intersections");
    let intersections = array(field(value, "intersections", path)?, &ip)?;
    run.claim(&format!("{ip}.length"), &intersections.len(), &out.intersections.len())?;
    for (k, (stated, &(a, b, size))) in intersections.iter().zip(&out.intersections).enumerate() {
        let p = at(&ip, k);
        keys(stated, &p, &["grips", "size"], &[])?;
        common::claim_ints(run, field(stated, "grips", &p)?, &dot(&p, "grips"), &[a as i64, b as i64])?;
        claim_int(run, stated, &p, "size", size)?;
    }
    let edp = dot(path, "edges");
    let edges = array(field(value, "edges", path)?, &edp)?;
    run.claim(&format!("{edp}.length"), &edges.len(), &out.edges.len())?;
    for (k, (stated, &(a, b))) in edges.iter().zip(&out.edges).enumerate() {
        common::claim_ints(run, stated, &at(&edp, k), &[a as i64, b as i64])?;
    }
    if !out.failed.is_empty() {
        let (k, reason) = &out.failed[0];
        return Err(run.failed(reject(format!("{path}.passed"), format!("P{k}: {reason}")), "false"));
    }
    let helper_grips: Vec<usize> = out.helpers.iter().map(|h| h.grip).collect();
    run.claim(&format!("{path}.order.topological"), &true, &topological(&helper_grips, &out.edges, &out.order))?;
    claim_int(run, value, path, "s0", out.s0)?;
    claim_int(run, value, path, "f0", out.f0)?;
    claim_int(run, value, path, "d0", out.d0)?;
    claim_int(run, value, path, "S", out.s)?;
    claim_int(run, value, path, "F", out.f)?;
    claim_int(run, value, path, "D", out.d)?;
    claim_int(run, value, path, "helper_delta_phi", out.helper_delta_phi)?;
    claim_int(run, value, path, "predicted_phi", out.predicted_phi)?;
    claim_int(run, value, path, "predicted_delta_phi", out.predicted_phi - out.phi_before)?;
    // The replay rows, in the stated order.
    let rp = dot(path, "replay");
    let rows = array(field(value, "replay", path)?, &rp)?;
    run.claim(&format!("{rp}.length"), &rows.len(), &out.replay.len())?;
    for (k, (stated, row)) in rows.iter().zip(&out.replay).enumerate() {
        let p = at(&rp, k);
        let last = row.every_member_lowered.is_some();
        let flag = if last { "every_member_lowered" } else { "delta_phi_equal" };
        keys(stated, &p, &["admissible", "digest", "letter", "moved", "moved_set_equal", "n4", "phi", flag], &[])?;
        let lp = dot(&p, "letter");
        let named = letter(field(stated, "letter", &p)?, &lp, items)?;
        run.claim(&lp, &text(named), &text(row.letter))?;
        claim_bool(run, stated, &p, "admissible", row.admissible)?;
        claim_int(run, stated, &p, "moved", row.moved)?;
        claim_bool(run, stated, &p, "moved_set_equal", row.moved_set_equal)?;
        claim_bool(run, stated, &p, flag, row.delta_phi_equal.or(row.every_member_lowered).unwrap_or(false))?;
        claim_str(run, stated, &p, "digest", &row.digest)?;
        claim_int(run, stated, &p, "n4", row.n4)?;
        claim_int(run, stated, &p, "phi", row.phi)?;
    }
    claim_bool(run, value, path, "helper_phi_equal", out.helper_phi_equal)?;
    claim_bool(run, value, path, "unit_held_before_lowering", out.held)?;
    require_replay(run, path, out)?;
    claim_bool(run, value, path, "passed", true)?;
    empty_list(run, value, path, "failures")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CT-LB5-01, precedence part: a reversed real edge is no topological order. No stored round of stage 14 has a
    /// precedence edge, so the edge is set here on the functions every round's order goes through.
    #[test]
    fn ct_lb5_01_reversed_precedence_edge_is_no_topological_order() {
        let nodes = [344, 451, 522];
        let edges: BTreeSet<(usize, usize)> = [(451, 344)].into_iter().collect();
        assert_eq!(least_order(&nodes, &edges), Some(vec![451, 344, 522]));
        assert!(topological(&nodes, &edges, &[451, 344, 522]));
        assert!(topological(&nodes, &edges, &[451, 522, 344]));
        // The edge reversed, a helper left out, a helper repeated.
        assert!(!topological(&nodes, &edges, &[344, 451, 522]));
        assert!(!topological(&nodes, &edges, &[451, 344]));
        assert!(!topological(&nodes, &edges, &[451, 451, 522]));
        // A cycle has no order at all (P1 fails).
        let cycle: BTreeSet<(usize, usize)> = [(451, 344), (344, 451)].into_iter().collect();
        assert_eq!(least_order(&nodes, &cycle), None);
    }
}
