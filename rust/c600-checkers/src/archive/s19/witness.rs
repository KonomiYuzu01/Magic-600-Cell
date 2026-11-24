//! `s19.gamma-witness`: Γ(Y)'s synthetic validation, `profile-family-validation-results.json.gz` (its decoded JSON).
//!
//! Inputs: the reference table Y (`reference`), the synthetic input Z (`input`), Z's stored endpoint (`end`), each a
//! stored pose table validated in full, and the I_a menu (`menu`). The checker re-checks the matching (`admission`,
//! `construction`, as `s19.gamma-matching` does) and then replays the stored nine letters forward and their nine
//! inverse letters, letter by letter, on Z and on Y at once. At every letter it recomputes:
//!
//! - the moved set of Z, and that it is σ⁻¹ of the moved set of Y (Γ(Y) carries legality along the prefix);
//! - the chronological matrix contract on all 177,120 pieces of both states;
//! - the join Z' = Y'·γ on every piece, with the same γ;
//! - both state digests, and Z's metrics q, n, H, Φ, R3 = Σ w_t q_t (w_t = Π_{s>t}(1 + 3 N_s)) and S3 = (N + 1)R3 + n.
//!
//! The record's type counts are checked against the model and its weights recomputed from them. The four block
//! boundaries A, D, C, E are recomputed from the prefixes, with their q, R3 and S3 differences: S3 drops at every
//! block; R3 (the length-3 rank) covers only blocks of at most three letters, so D's four letters are accounted for by
//! S3 alone (LB5.5). The endpoint must be the stored end table and is not the solved state (no sticker goal is
//! reached or transported); the inverse letters must return both states exactly.
//!
//! Not checked: labels, the record's own scope prose, the original-source digests of its inputs (PS3's
//! correspondence), `five_letter_actual_replay` and `new_canonical_nonK_tree_factors` (statements about the historical
//! run). `reachability` must say UNKNOWN.

use std::sync::Arc;

use c600_jumble::state::State;
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::Value;

use super::common::{
    claim_bool, claim_decimal, claim_int, claim_ints, decimal, first_change, form, ia_menu, inverse, join, letters,
    metrics, minus, outside, rank, read_table, s_rank, solved, step, strings, twist, type_counts, weights, Metrics,
};
use super::matching::{admission, construction, reachability, scope, Matched};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, int, keys, reject, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::report::{codes, Sources};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s19.gamma-witness",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s19/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s19/matching.rs", include_bytes!("matching.rs")),
        ("c600-checkers/src/archive/s19/witness.rs", include_bytes!("witness.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUBCHECKER: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 19,
    family: "S19 Γ(Y) synthetic validation: the nine-letter forward and inverse witness with prefix transport",
};

/// The fixed macro length of the record's ranks R3 and S3.
const L: i64 = 3;

const KEYS: [&str; 33] = [
    "status",
    "source_ref",
    "inputs",
    "source_regions_sha256",
    "source_model_sha256",
    "reference_input_digest",
    "synthetic_input_digest",
    "synthetic_input_table",
    "synthetic_input_table_sha256",
    "reachability",
    "validity",
    "construction",
    "admission",
    "word",
    "fixed_L",
    "physical_type_counts",
    "weights_decimal",
    "forward",
    "inverse",
    "full_State_apply_letters",
    "five_letter_actual_replay",
    "boundaries",
    "scope",
    "initial_metrics",
    "all177120_initial_regional_matching_and_typed_metrics",
    "new_canonical_nonK_tree_factors",
    "matching_certificate",
    "endpoint",
    "four_boundary_rank_audit",
    "max_forward_prefix_height",
    "all177120_exact_inverse_return",
    "all18_prefix_Gamma_joins",
    "reference_virtual_return_exact",
];

const STEP_KEYS: [&str; 8] = [
    "letter",
    "actual_support_ids",
    "support_equals_decoded_pi_inverse_of_reference",
    "all177120_chronological_matrix_contract",
    "all177120_Gamma_join",
    "actual_digest",
    "reference_digest",
    "metrics",
];

/// The fixed rank the checker computes metrics under.
pub struct Ranked<'w> {
    pub weights: &'w [c600_jumble::q5::Q5],
}

impl Ranked<'_> {
    /// Compares a stated `{q_by_type, n, H, Phi, R3, S3}` with computed metrics.
    pub fn claim(&self, run: &mut Run, claim: &str, value: &Value, m: &Metrics) -> Step<()> {
        keys(value, claim, &["q_by_type", "n", "H", "Phi", "R3", "S3"], &[])?;
        let get = |key: &str| field(value, key, claim);
        claim_ints(run, &dot(claim, "q_by_type"), get("q_by_type")?, &m.q)?;
        claim_int(run, &dot(claim, "n"), get("n")?, m.n)?;
        claim_int(run, &dot(claim, "H"), get("H")?, m.h)?;
        claim_int(run, &dot(claim, "Phi"), get("Phi")?, m.phi)?;
        let r = rank(self.weights, &m.q);
        claim_decimal(run, &dot(claim, "R3"), get("R3")?, &r)?;
        claim_decimal(run, &dot(claim, "S3"), get("S3")?, &s_rank(&r, m.n))
    }
}

/// One state of the replay: the digests of Z and Y and Z's metrics.
#[derive(Clone)]
struct Point {
    actual: String,
    reference: String,
    metrics: Metrics,
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = document(run.cert.bytes)?;
    if form(&doc, &[&KEYS]).is_none() {
        return Err(outside(SOURCES.id, "Γ(Y)'s synthetic validation record", &doc));
    }
    keys(&doc, "certificate", &KEYS, &[])?;
    let get = |key: &str| field(&doc, key, key);
    let menu = ia_menu(ctx, run, "menu")?;
    let items = menu.items.len();
    let mut y = read_table(ctx, Some(menu.clone()), run, "reference")?;
    let stated = hex64(get("reference_input_digest")?, "reference_input_digest")?.to_owned();
    run.compare("reference_input_digest", &stated, &y.digest, codes::START_TABLE_MISMATCH)?;
    let mut z = read_table(ctx, Some(menu.clone()), run, "input")?;
    let stated = hex64(get("synthetic_input_digest")?, "synthetic_input_digest")?.to_owned();
    run.compare("synthetic_input_digest", &stated, &z.digest, codes::START_TABLE_MISMATCH)?;
    reachability(run, get("reachability")?, "reachability")?;

    // Type counts from the model, and the fixed weights.
    let counts = type_counts(ctx);
    claim_ints(run, "physical_type_counts", get("physical_type_counts")?, &counts)?;
    // The record's ranks are R3 and S3: its fixed length is 3.
    claim_int(run, "fixed_L", get("fixed_L")?, L)?;
    let w = weights(&counts, L);
    let stated = strings(get("weights_decimal")?, "weights_decimal")?;
    if stated.len() != w.len() {
        return Err(reject("weights_decimal", format!("{} weights for {} types", stated.len(), w.len())));
    }
    for (t, (s, x)) in stated.iter().zip(&w).enumerate() {
        run.claim(&at("weights_decimal", t), s, &decimal(x))?;
    }
    let ranked = Ranked { weights: &w };

    // The matching, as `s19.gamma-matching` checks it.
    let matched = admission(ctx, run, get("admission")?, "admission", &y.state, &z.state)?;
    construction(ctx, run, get("construction")?, "construction", &y.state, &z.state, &matched)?;
    let initial = metrics(ctx, &z.state, "initial_metrics")?;
    ranked.claim(run, "initial_metrics", get("initial_metrics")?, &initial)?;
    let transferred = join(ctx, &y.state, &z.state, &matched.gamma) && metrics(ctx, &y.state, "reference")? == initial;
    let key = "all177120_initial_regional_matching_and_typed_metrics";
    claim_bool(run, key, get(key)?, transferred)?;

    // The forward word.
    let word = letters(get("word")?, "word", items)?;
    let start = Point { actual: z.digest.clone(), reference: y.digest.clone(), metrics: initial.clone() };
    let mut points = vec![start.clone()];
    let mut joins = true;
    let forward = array(get("forward")?, "forward")?;
    if forward.len() != word.len() {
        return Err(reject("forward", format!("{} steps for {} letters", forward.len(), word.len())));
    }
    for (i, &letter) in word.iter().enumerate() {
        let point = replay(
            ctx,
            run,
            &menu,
            &at("forward", i),
            &forward[i],
            letter,
            (&mut z.state, &mut y.state),
            &matched,
            &ranked,
            &mut joins,
        )?;
        points.push(point);
    }
    let end = points.last().expect("a forward point").clone();
    let max_h = points[1..].iter().map(|p| p.metrics.h).max().unwrap_or(0);
    claim_int(run, "max_forward_prefix_height", get("max_forward_prefix_height")?, max_h)?;
    endpoint(ctx, run, &menu, get("endpoint")?, &end, &y.state, &z.state, &matched, &ranked)?;

    // The inverse word, back to both inputs.
    let back =
        word.iter().rev().enumerate().map(|(i, &l)| inverse(&menu, l, &at("inverse", i))).collect::<Step<Vec<_>>>()?;
    let inverse_steps = array(get("inverse")?, "inverse")?;
    if inverse_steps.len() != back.len() {
        return Err(reject("inverse", format!("{} steps for {} letters", inverse_steps.len(), back.len())));
    }
    for (i, &letter) in back.iter().enumerate() {
        replay(
            ctx,
            run,
            &menu,
            &at("inverse", i),
            &inverse_steps[i],
            letter,
            (&mut z.state, &mut y.state),
            &matched,
            &ranked,
            &mut joins,
        )?;
    }
    let applied = (word.len() + back.len()) as i64;
    claim_int(run, "full_State_apply_letters", get("full_State_apply_letters")?, applied)?;
    claim_bool(
        run,
        "all177120_exact_inverse_return",
        get("all177120_exact_inverse_return")?,
        z.state.digest(ctx) == start.actual,
    )?;
    claim_bool(
        run,
        "reference_virtual_return_exact",
        get("reference_virtual_return_exact")?,
        y.state.digest(ctx) == start.reference,
    )?;
    claim_bool(run, "all18_prefix_Gamma_joins", get("all18_prefix_Gamma_joins")?, joins)?;

    boundaries(run, get("boundaries")?, "boundaries", &points, &ranked, L)?;
    let strict = strict_blocks(get("boundaries")?, &points, &ranked)?;
    claim_bool(run, "four_boundary_rank_audit", get("four_boundary_rank_audit")?, strict)?;

    scope(run, &matched);
    run.held(
        "scope.not_checked",
        "status, scope and validity prose, source_ref, the original-source digests of inputs, the matching \
         certificate and the end table (PS3's correspondence), five_letter_actual_replay and \
         new_canonical_nonK_tree_factors (statements about the historical run)",
    );
    Ok(())
}

/// One letter on Z and Y together, with every per-step claim.
#[allow(clippy::too_many_arguments)]
fn replay(
    ctx: &Context,
    run: &mut Run,
    menu: &Arc<Menu>,
    claim: &str,
    value: &Value,
    letter: (usize, usize),
    (z, y): (&mut State, &mut State),
    matched: &Matched,
    ranked: &Ranked,
    joins: &mut bool,
) -> Step<Point> {
    keys(value, claim, &STEP_KEYS, &[])?;
    let get = |key: &str| field(value, key, claim);
    let stated = crate::replay::letter(get("letter")?, &dot(claim, "letter"), menu.items.len())?;
    if stated != letter {
        return Err(reject(dot(claim, "letter"), format!("stated {stated:?}, the word gives {letter:?}")));
    }
    let g = twist(ctx, menu, letter, claim)?;
    let actual = step(ctx, z, &g, claim)?;
    let reference = step(ctx, y, &g, &dot(claim, "reference"))?;
    let support: Vec<i64> = actual.inside.iter().map(|&p| p as i64).collect();
    claim_ints(run, &dot(claim, "actual_support_ids"), get("actual_support_ids")?, &support)?;
    let transported = actual.inside == matched.gamma.preimage(&reference.inside);
    claim_bool(
        run,
        &dot(claim, "support_equals_decoded_pi_inverse_of_reference"),
        get("support_equals_decoded_pi_inverse_of_reference")?,
        transported,
    )?;
    let key = "all177120_chronological_matrix_contract";
    claim_bool(run, &dot(claim, key), get(key)?, actual.contract && reference.contract)?;
    let joined = join(ctx, y, z, &matched.gamma);
    *joins &= joined;
    claim_bool(run, &dot(claim, "all177120_Gamma_join"), get("all177120_Gamma_join")?, joined)?;
    let point = Point { actual: z.digest(ctx), reference: y.digest(ctx), metrics: metrics(ctx, z, claim)? };
    let stated = hex64(get("actual_digest")?, &dot(claim, "actual_digest"))?.to_owned();
    run.claim(&dot(claim, "actual_digest"), &stated, &point.actual)?;
    let stated = hex64(get("reference_digest")?, &dot(claim, "reference_digest"))?.to_owned();
    run.claim(&dot(claim, "reference_digest"), &stated, &point.reference)?;
    ranked.claim(run, &dot(claim, "metrics"), get("metrics")?, &point.metrics)?;
    Ok(point)
}

/// The endpoint after the forward word: the stored end table, Y's endpoint, the join, the metrics, and that it is not
/// the solved state.
#[allow(clippy::too_many_arguments)]
fn endpoint(
    ctx: &Context,
    run: &mut Run,
    menu: &Arc<Menu>,
    value: &Value,
    end: &Point,
    y: &State,
    z: &State,
    matched: &Matched,
    ranked: &Ranked,
) -> Step<()> {
    let claim = "endpoint";
    let required =
        ["table", "sha256", "digest", "reference_digest", "all177120_exact_gauge_join", "metrics", "not_restored"];
    keys(value, claim, &required, &[])?;
    let get = |key: &str| field(value, key, claim);
    let stored = read_table(ctx, Some(menu.clone()), run, "end").map_err(|stop| match stop {
        crate::claims::Stop::Fail { reason, .. } => reject("end", reason),
        other => other,
    })?;
    if stored.digest != end.actual {
        return Err(reject(
            "end",
            format!("the stored end table has digest {}, the replay ends at {}", stored.digest, end.actual),
        ));
    }
    let stated = hex64(get("digest")?, "endpoint.digest")?.to_owned();
    run.claim("endpoint.digest", &stated, &end.actual)?;
    let stated = hex64(get("reference_digest")?, "endpoint.reference_digest")?.to_owned();
    run.claim("endpoint.reference_digest", &stated, &end.reference)?;
    claim_bool(
        run,
        "endpoint.all177120_exact_gauge_join",
        get("all177120_exact_gauge_join")?,
        join(ctx, y, &stored.state, &matched.gamma),
    )?;
    ranked.claim(run, "endpoint.metrics", get("metrics")?, &end.metrics)?;
    let restored = solved(z);
    claim_bool(run, "endpoint.not_restored", get("not_restored")?, !restored)?;
    run.held(
        "goal.sticker",
        format!(
            "{} at the endpoint: {} pieces are off the lattice and q16 = {}; Γ(Y) transports no sticker goal",
            if restored { "holds" } else { "fails" },
            end.metrics.n,
            end.metrics.q.get(16).copied().unwrap_or(0)
        ),
    );
    Ok(())
}

/// The prefixes of the stated block boundaries, checked to be increasing and to end with the word.
fn boundary_prefixes(value: &Value, claim: &str, word: usize) -> Step<Vec<usize>> {
    let entries = array(value, claim)?;
    let mut prefixes = Vec::with_capacity(entries.len());
    for (j, entry) in entries.iter().enumerate() {
        let path = at(claim, j);
        let p = int(field(entry, "prefix", &path)?, &dot(&path, "prefix"))?;
        let previous = prefixes.last().copied().unwrap_or(0);
        let p = usize::try_from(p).ok().filter(|&p| p > previous && p <= word);
        prefixes.push(p.ok_or_else(|| reject(dot(&path, "prefix"), "the block prefixes increase within the word"))?);
    }
    if prefixes.last() != Some(&word) {
        return Err(reject(claim, "the last block ends with the word"));
    }
    Ok(prefixes)
}

/// The block boundaries: metrics, digests, q differences and the R3 and S3 differences.
fn boundaries(run: &mut Run, value: &Value, claim: &str, points: &[Point], ranked: &Ranked, l: i64) -> Step<()> {
    let prefixes = boundary_prefixes(value, claim, points.len() - 1)?;
    let entries = array(value, claim)?;
    let strict = strict_blocks(value, points, ranked)?;
    let mut previous = 0usize;
    let mut lengths = Vec::new();
    for (j, (&p, entry)) in prefixes.iter().zip(entries).enumerate() {
        let path = at(claim, j);
        let required = [
            "prefix",
            "block",
            "metrics",
            "actual_digest",
            "reference_digest",
            "delta_q",
            "first_changed_type",
            "delta_R3",
            "delta_S3",
            "all_four_blocks_same_S3_strict",
        ];
        keys(entry, &path, &required, &[])?;
        let get = |key: &str| field(entry, key, &path);
        let name = crate::claims::string(get("block")?, &dot(&path, "block"))?.to_owned();
        let (before, after) = (&points[previous], &points[p]);
        run.held(dot(&path, "prefix"), format!("block {name}: letters {} to {}", previous + 1, p));
        ranked.claim(run, &dot(&path, "metrics"), get("metrics")?, &after.metrics)?;
        let stated = hex64(get("actual_digest")?, &dot(&path, "actual_digest"))?.to_owned();
        run.claim(&dot(&path, "actual_digest"), &stated, &after.actual)?;
        let stated = hex64(get("reference_digest")?, &dot(&path, "reference_digest"))?.to_owned();
        run.claim(&dot(&path, "reference_digest"), &stated, &after.reference)?;
        let delta = minus(&after.metrics.q, &before.metrics.q);
        claim_ints(run, &dot(&path, "delta_q"), get("delta_q")?, &delta)?;
        let first = first_change(&delta).map_or(Value::Null, |(t, _)| Value::Int(t as i64));
        run.claim(
            &dot(&path, "first_changed_type"),
            &format!("{:?}", get("first_changed_type")?),
            &format!("{first:?}"),
        )?;
        let (r0, r1) = (rank(ranked.weights, &before.metrics.q), rank(ranked.weights, &after.metrics.q));
        claim_decimal(run, &dot(&path, "delta_R3"), get("delta_R3")?, &r1.sub(&r0))?;
        let ds = s_rank(&r1, after.metrics.n).sub(&s_rank(&r0, before.metrics.n));
        claim_decimal(run, &dot(&path, "delta_S3"), get("delta_S3")?, &ds)?;
        let key = "all_four_blocks_same_S3_strict";
        claim_bool(run, &dot(&path, key), get(key)?, strict)?;
        let length = (p - previous) as i64;
        lengths.push((name, length, r1 < r0));
        previous = p;
    }
    let notes: Vec<String> = lengths
        .iter()
        .map(|(name, length, high)| {
            let rank = if *high { format!("R{l} drops") } else { "R unchanged; S drops through n".to_owned() };
            format!("{name}: {length} letters, {rank}")
        })
        .collect();
    run.held(
        "premise.length",
        format!(
            "{}; the length-{l} rank lemma applies only to blocks of at most {l} letters, and its universal bound \
             keeps the selector, availability and invariant-domain premises (LB5.5)",
            notes.join("; ")
        ),
    );
    Ok(())
}

/// Whether S3 drops strictly at every stated block boundary.
fn strict_blocks(value: &Value, points: &[Point], ranked: &Ranked) -> Step<bool> {
    let prefixes = boundary_prefixes(value, "boundaries", points.len() - 1)?;
    let mut previous = 0usize;
    let mut strict = true;
    for p in prefixes {
        let s = |point: &Point| s_rank(&rank(ranked.weights, &point.metrics.q), point.metrics.n);
        strict &= s(&points[p]) < s(&points[previous]);
        previous = p;
    }
    Ok(strict)
}
