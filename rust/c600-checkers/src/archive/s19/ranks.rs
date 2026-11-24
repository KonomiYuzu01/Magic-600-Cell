//! `s19.ranks`: stage 19's finite rank arithmetic, R_L = Σ w_t q_t with w_t = Π_{s>t}(1 + L·N_s), and
//! S = (N + 1)·R + n.
//!
//! `type-height-weight-inputs.json`, which the historical arithmetic read its type counts from, is absent. This
//! checker derives the counts anew from the model (the K⁺-orbit sizes, in the model's order) and the weights from
//! them, under its own identity (CX.17); its trace says so. Two archived records are read as they are:
//!
//! - `type-height-weight-results.json` (L = 1). Inputs: the source table Y (`start`) and the four-coset record
//!   `type-lex-282-results.json` (`source`, read only for its `input_digest`, which must be Y's). The checker compares
//!   N, the type counts and the weights; each tail identity w_t = 1 + L·Σ_{s>t} N_s w_s; the product Π(1 + L·N_t), the
//!   sum Σ N_t w_t, the identity L·Σ N_t w_t = Π − 1 and the cost coefficient Π − 1; per row, the stated q difference
//!   of a one-letter step (a supplied block vector, CX.17), its first change, its total, and the ranks before and after
//!   from Y's q, recomputed from the stored table; and the synthetic counterexample to unbounded differences.
//! - `type-lex-trimmed-independent-results.json`, the anchors `one_fixed_L3`, `one_fixed_S` and
//!   `generic_mixed_rank_proof_check`. Inputs as `s19.frontier` reads them: the menu, Y (`origin`), the post-A table
//!   (`start`) and the block words (`blocks`). The checker replays A, D, C, E from Y and the compact route (the blocks
//!   that lower R: A, C, E), recomputing q and n at every boundary; R3 on the compact route, whose blocks must each
//!   have at most three letters (R_L's length premise; D has four and is accounted for by S alone); S on the cleanup
//!   route; and the counting instance of the mixed-rank argument.
//!
//! The universal bounds keep their premises: a cost formula other than the stage's conditional one, or a record that
//! declares the selector, closure and availability premises discharged, is a global claim outside this kind (LB5.5).

use c600_jumble::model::NP;
use c600_jumble::q5::Q5;
use c600_jumble::state::State;
use c600_jumble::Context;
use c600_kernel::json::Value;

use super::common::{
    big, blocks, boolean, claim_bool, claim_decimal, claim_int, claim_ints, claim_strings, decimal, factor,
    first_change, form, ia_menu, ints, json_input, metrics, minus, origin_and_start, outside, rank, read_table, s_rank,
    step, strings, twist, type_counts, weights, Blocks, TRIMMED,
};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, int, keys, not_applicable, reject, string, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::report::{codes, Sources};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s19.ranks",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s19/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s19/ranks.rs", include_bytes!("ranks.rs")),
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
    family: "S19 R3, S and R_L on stored blocks; fixed-rank arithmetic derived anew from the model's type counts",
};

/// The stage's high-phase cost formula, with its premises. Any other text may drop a premise, so it is out of scope.
const FORMULA: &str = "max(H-2,0)/2 * coefficient, CONDITIONAL on a terminating <=L lex-q-descending selector and \
                       invariant covered domain";

/// `type-height-weight-results.json`.
const WEIGHTS: [&str; 21] = [
    "status",
    "source_ref",
    "input_hashes",
    "source_hashes",
    "N",
    "L",
    "physical_type_counts",
    "counts_provenance",
    "weights_decimal",
    "all36_tail_weight_identities_exact",
    "product_all_factors_decimal",
    "sum_Nt_wt_decimal",
    "linear_high_cost_coefficient_decimal",
    "linear_high_cost_formula",
    "all_type_sum_identity_exact",
    "rows",
    "synthetic_bounded_difference_cases",
    "synthetic_unbounded_difference_counterexample",
    "geometry_imported",
    "fresh_State_apply_letters",
    "scope",
];

const DERIVED: &str = "type-height-weight-inputs.json is absent: the type counts are the model's K⁺-orbit sizes and \
     the weights are derived anew from them under this checker's identity, not reproduced from the missing file \
     (CX.17)";

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = document(run.cert.bytes)?;
    match form(&doc, &[&WEIGHTS, &TRIMMED]) {
        Some(0) => {
            keys(&doc, "certificate", &WEIGHTS, &[])?;
            fixed_weights(ctx, run, &doc)
        }
        Some(_) => {
            keys(&doc, "certificate", &TRIMMED, &[])?;
            blocks_ranks(ctx, run, &doc)
        }
        None => Err(outside(SOURCES.id, "stage 19's rank records", &doc)),
    }
}

/// Compares stated decimal weights with computed ones, one claim per type.
fn claim_weights(run: &mut Run, claim: &str, value: &Value, w: &[Q5]) -> Step<()> {
    let stated = strings(value, claim)?;
    if stated.len() != w.len() {
        return Err(reject(claim, format!("{} weights for {} types", stated.len(), w.len())));
    }
    for (t, (s, x)) in stated.iter().zip(w).enumerate() {
        run.claim(&at(claim, t), s, &decimal(x))?;
    }
    Ok(())
}

/// The type counts, checked against the model.
fn counts(ctx: &Context, run: &mut Run, claim: &str, value: &Value) -> Step<Vec<i64>> {
    let counts = type_counts(ctx);
    claim_ints(run, claim, value, &counts)?;
    run.held("scope.counts", DERIVED);
    Ok(counts)
}

/// `type-height-weight-results.json`.
fn fixed_weights(ctx: &Context, run: &mut Run, doc: &Value) -> Step<()> {
    let get = |key: &str| field(doc, key, key);
    let source = json_input(run, "source")?;
    let source_bytes = run.input("source")?;
    let stated = hex64(field(&source, "input_digest", "source")?, "source.input_digest")?.to_owned();
    run.bind("source", source_bytes, vec![("input_digest", stated.clone())]);
    let start = read_table(ctx, None, run, "start")?;
    run.compare("start", &stated, &start.digest, codes::START_TABLE_MISMATCH)?;

    claim_int(run, "N", get("N")?, NP as i64)?;
    let l = int(get("L")?, "L")?;
    if l < 1 {
        return Err(reject("L", "a fixed macro length is positive"));
    }
    run.held("L", l);
    let n = counts(ctx, run, "physical_type_counts", get("physical_type_counts")?)?;
    let w = weights(&n, l);
    claim_weights(run, "weights_decimal", get("weights_decimal")?, &w)?;
    let tails = (0..n.len()).all(|t| {
        let tail = (t + 1..n.len()).fold(big(0), |sum, s| sum.add(&big(n[s]).mul(&w[s])));
        w[t] == big(1).add(&big(l).mul(&tail))
    });
    claim_bool(run, "all36_tail_weight_identities_exact", get("all36_tail_weight_identities_exact")?, tails)?;
    let product = n.iter().fold(big(1), |p, &x| p.mul(&factor(l, x)));
    claim_decimal(run, "product_all_factors_decimal", get("product_all_factors_decimal")?, &product)?;
    let sum = rank(&w, &n);
    claim_decimal(run, "sum_Nt_wt_decimal", get("sum_Nt_wt_decimal")?, &sum)?;
    let coefficient = product.sub(&big(1));
    let identity = big(l).mul(&sum) == coefficient;
    claim_bool(run, "all_type_sum_identity_exact", get("all_type_sum_identity_exact")?, identity)?;
    claim_decimal(
        run,
        "linear_high_cost_coefficient_decimal",
        get("linear_high_cost_coefficient_decimal")?,
        &coefficient,
    )?;
    let formula = string(get("linear_high_cost_formula")?, "linear_high_cost_formula")?;
    if formula != FORMULA {
        return Err(not_applicable(
            "a high-phase cost bound other than the stage's conditional one is a global claim outside this kind: \
             R_L's bound keeps its selector, length and invariant-domain premises (LB5.5)",
        ));
    }
    run.held(
        "linear_high_cost_formula",
        "conditional on a terminating selector of at most L letters and an invariant domain",
    );

    // Rows: one-letter steps from Y, with their stated q differences (supplied block vectors).
    let q0 = metrics(ctx, &start.state, "start")?.q;
    let rows = array(get("rows")?, "rows")?;
    if rows.is_empty() {
        return Err(reject("rows", "no rows"));
    }
    for (i, row) in rows.iter().enumerate() {
        let path = at("rows", i);
        let required = [
            "representative",
            "literal_coset",
            "first_changed_height_type",
            "first_delta_q",
            "delta_q_by_type",
            "total_delta_Phi2",
            "rank_before",
            "rank_after",
            "delta_rank",
            "strict_descent",
        ];
        keys(row, &path, &required, &[])?;
        let g = |key: &str| field(row, key, &path);
        int(g("representative")?, &dot(&path, "representative"))?;
        ints(g("literal_coset")?, &dot(&path, "literal_coset"))?;
        let delta = ints(g("delta_q_by_type")?, &dot(&path, "delta_q_by_type"))?;
        if delta.len() != q0.len() {
            return Err(reject(
                dot(&path, "delta_q_by_type"),
                format!("{} entries for {} types", delta.len(), q0.len()),
            ));
        }
        let out_of_range = || reject(dot(&path, "delta_q_by_type"), "an entry is out of range");
        let q1: Vec<i64> =
            q0.iter().zip(&delta).map(|(a, d)| a.checked_add(*d)).collect::<Option<_>>().ok_or_else(out_of_range)?;
        if q1.iter().any(|&q| q < 0) {
            return Err(reject(dot(&path, "delta_q_by_type"), "the difference leaves a negative q"));
        }
        let total = delta.iter().try_fold(0i64, |sum, d| sum.checked_add(*d)).ok_or_else(out_of_range)?;
        let (t, d) = first_change(&delta).ok_or_else(|| reject(dot(&path, "delta_q_by_type"), "no type changes"))?;
        claim_int(run, &dot(&path, "first_changed_height_type"), g("first_changed_height_type")?, t as i64)?;
        claim_int(run, &dot(&path, "first_delta_q"), g("first_delta_q")?, d)?;
        claim_int(run, &dot(&path, "total_delta_Phi2"), g("total_delta_Phi2")?, total)?;
        let (r0, r1) = (rank(&w, &q0), rank(&w, &q1));
        claim_decimal(run, &dot(&path, "rank_before"), g("rank_before")?, &r0)?;
        claim_decimal(run, &dot(&path, "rank_after"), g("rank_after")?, &r1)?;
        claim_decimal(run, &dot(&path, "delta_rank"), g("delta_rank")?, &r1.sub(&r0))?;
        claim_bool(run, &dot(&path, "strict_descent"), g("strict_descent")?, r1 < r0)?;
    }
    run.held(
        "premise.length",
        format!(
            "not checked for the rows: their q differences are the record's supplied block vectors (CX.17), not \
             replayed, so no row's length is shown within L = {l}; Y's q is recomputed from the stored table and \
             the ranks from it"
        ),
    );

    counterexample(
        run,
        get("synthetic_unbounded_difference_counterexample")?,
        "synthetic_unbounded_difference_counterexample",
    )?;
    run.held(
        "scope.not_checked",
        "synthetic_bounded_difference_cases (the 690 synthetic cases are not enumerated here), the rows' coset labels, \
         geometry_imported and fresh_State_apply_letters (statements about the historical run), status, scope, \
         counts_provenance and the original-source digests",
    );
    Ok(())
}

/// The synthetic counterexample: a lexicographic descent whose weighted rank rises because a later coordinate grows
/// past the bounded-difference premise L·N_t.
fn counterexample(run: &mut Run, value: &Value, claim: &str) -> Step<()> {
    let required = ["counts", "L", "weights", "delta", "lex_sign", "weighted_delta", "violated_later_growth_bound"];
    keys(value, claim, &required, &[])?;
    let get = |key: &str| field(value, key, claim);
    let counts = ints(get("counts")?, &dot(claim, "counts"))?;
    let l = int(get("L")?, &dot(claim, "L"))?;
    let delta = ints(get("delta")?, &dot(claim, "delta"))?;
    if counts.is_empty() || delta.len() != counts.len() || l < 1 {
        return Err(reject(claim, "counts and delta have one entry per type, and L is positive"));
    }
    let w = weights(&counts, l);
    let stated = ints(get("weights")?, &dot(claim, "weights"))?;
    let computed: Vec<String> = w.iter().map(decimal).collect();
    let stated: Vec<String> = stated.iter().map(i64::to_string).collect();
    claim_strings(run, &dot(claim, "weights"), &stated, &computed)?;
    let (t0, d0) = first_change(&delta).ok_or_else(|| reject(dot(claim, "delta"), "no coordinate changes"))?;
    claim_int(run, &dot(claim, "lex_sign"), get("lex_sign")?, d0.signum())?;
    let weighted = rank(&w, &delta);
    claim_decimal_int(run, &dot(claim, "weighted_delta"), get("weighted_delta")?, &weighted)?;
    let limit = |t: usize| i128::from(l) * i128::from(counts[t]);
    let bound = (t0 + 1..counts.len()).find(|&t| i128::from(delta[t]).abs() > limit(t)).map(limit);
    let bound =
        bound.ok_or_else(|| reject(dot(claim, "violated_later_growth_bound"), "no later coordinate exceeds L·N_t"))?;
    let stated = int(get("violated_later_growth_bound")?, &dot(claim, "violated_later_growth_bound"))?;
    run.claim(&dot(claim, "violated_later_growth_bound"), &stated.to_string(), &bound.to_string())?;
    if !(d0 < 0 && weighted > big(0)) {
        return Err(reject(claim, "a counterexample descends lexicographically while its weighted rank rises"));
    }
    run.held(claim, "lexicographic descent with a rising weighted rank, only past the bounded-difference premise");
    Ok(())
}

/// An integer claim compared with an exact integer.
fn claim_decimal_int(run: &mut Run, claim: &str, value: &Value, computed: &Q5) -> Step<()> {
    let stated = int(value, claim)?;
    run.claim(claim, &stated.to_string(), &decimal(computed))
}

/// One route's boundaries: q and n before the first block and after each.
struct Route {
    names: Vec<String>,
    lengths: Vec<usize>,
    q: Vec<Vec<i64>>,
    n: Vec<i64>,
}

/// Replays named blocks from a state, recording the boundaries.
fn route(
    ctx: &Context,
    menu: &std::sync::Arc<c600_jumble::twist::Menu>,
    from: &State,
    blocks: &Blocks,
    names: &[String],
) -> Step<Route> {
    let mut state = from.clone();
    let m = metrics(ctx, &state, "origin")?;
    let mut out = Route { names: names.to_vec(), lengths: Vec::new(), q: vec![m.q], n: vec![m.n] };
    for name in names {
        let word = blocks.get(name)?;
        for (i, &letter) in word.iter().enumerate() {
            let path = at(&format!("blocks.{name}"), i);
            step(ctx, &mut state, &twist(ctx, menu, letter, &path)?, &path)?;
        }
        let m = metrics(ctx, &state, &format!("blocks.{name}"))?;
        out.lengths.push(word.len());
        out.q.push(m.q);
        out.n.push(m.n);
    }
    Ok(out)
}

/// The anchors `one_fixed_L3`, `one_fixed_S` and `generic_mixed_rank_proof_check` of the trimmed review record.
fn blocks_ranks(ctx: &Context, run: &mut Run, doc: &Value) -> Step<()> {
    let get = |key: &str| field(doc, key, key);
    let menu = ia_menu(ctx, run, "menu")?;
    let blocks = blocks(run, "blocks", menu.items.len())?;
    let (origin, _) = origin_and_start(ctx, run, &menu, &blocks)?;
    let counts = type_counts(ctx);
    run.held("scope.counts", DERIVED);
    const L: i64 = 3;
    let w = weights(&counts, L);
    let r = |q: &[i64]| rank(&w, q);

    // The cleanup route, all blocks in order; the compact route, the blocks that lower R.
    let cleanup = route(ctx, &menu, &origin, &blocks, &blocks.names)?;
    let high: Vec<bool> = (0..cleanup.names.len()).map(|b| r(&cleanup.q[b + 1]) < r(&cleanup.q[b])).collect();
    let compact_names: Vec<String> =
        cleanup.names.iter().zip(&high).filter(|(_, h)| **h).map(|(name, _)| name.clone()).collect();
    let compact = route(ctx, &menu, &origin, &blocks, &compact_names)?;

    // R3 on the compact route.
    let anchor = "one_fixed_L3";
    let v = get(anchor)?;
    keys(v, anchor, &["macro_lengths", "weights", "boundary_R", "first_changes", "all_strict"], &[])?;
    let g = |key: &str| field(v, key, anchor);
    let lengths = ints(g("macro_lengths")?, &dot(anchor, "macro_lengths"))?;
    if let Some(long) = lengths.iter().find(|&&len| len > L) {
        return Err(reject(
            dot(anchor, "macro_lengths"),
            format!("R_L's length premise: a {long}-letter block is not covered by the length-{L} rank lemma"),
        ));
    }
    let computed: Vec<i64> = compact.lengths.iter().map(|&len| len as i64).collect();
    if let Some(long) = computed.iter().find(|&&len| len > L) {
        return Err(reject(dot(anchor, "macro_lengths"), format!("a {long}-letter block lowers R beyond L = {L}")));
    }
    claim_ints(run, &dot(anchor, "macro_lengths"), g("macro_lengths")?, &computed)?;
    claim_weights(run, &dot(anchor, "weights"), g("weights")?, &w)?;
    let boundary: Vec<String> = compact.q.iter().map(|q| decimal(&r(q))).collect();
    let stated = strings(g("boundary_R")?, &dot(anchor, "boundary_R"))?;
    claim_strings(run, &dot(anchor, "boundary_R"), &stated, &boundary)?;
    let changes = array(g("first_changes")?, &dot(anchor, "first_changes"))?;
    if changes.len() != compact.lengths.len() {
        return Err(reject(
            dot(anchor, "first_changes"),
            format!("{} changes for {} blocks", changes.len(), compact.lengths.len()),
        ));
    }
    for (b, change) in changes.iter().enumerate() {
        let path = at(&dot(anchor, "first_changes"), b);
        keys(change, &path, &["type", "delta"], &[])?;
        let (t, d) = first_change(&minus(&compact.q[b + 1], &compact.q[b]))
            .ok_or_else(|| reject(&path, "the block leaves q unchanged"))?;
        claim_int(run, &dot(&path, "type"), field(change, "type", &path)?, t as i64)?;
        claim_int(run, &dot(&path, "delta"), field(change, "delta", &path)?, d)?;
    }
    let strict = (0..compact.lengths.len()).all(|b| r(&compact.q[b + 1]) < r(&compact.q[b]));
    claim_bool(run, &dot(anchor, "all_strict"), g("all_strict")?, strict)?;

    // S on the cleanup route.
    let anchor = "one_fixed_S";
    let v = get(anchor)?;
    keys(v, anchor, &["N", "boundary_S", "order", "all_strict"], &[])?;
    let g = |key: &str| field(v, key, anchor);
    claim_int(run, &dot(anchor, "N"), g("N")?, NP as i64)?;
    let order = array(g("order")?, &dot(anchor, "order"))?
        .iter()
        .enumerate()
        .map(|(k, x)| string(x, &at(&dot(anchor, "order"), k)).map(str::to_owned))
        .collect::<Step<Vec<_>>>()?;
    claim_strings(run, &dot(anchor, "order"), &order, &cleanup.names)?;
    let s: Vec<Q5> = cleanup.q.iter().zip(&cleanup.n).map(|(q, &n)| s_rank(&r(q), n)).collect();
    let boundary: Vec<String> = s.iter().map(decimal).collect();
    let stated = strings(g("boundary_S")?, &dot(anchor, "boundary_S"))?;
    claim_strings(run, &dot(anchor, "boundary_S"), &stated, &boundary)?;
    let strict = s.windows(2).all(|pair| pair[1] < pair[0]);
    claim_bool(run, &dot(anchor, "all_strict"), g("all_strict")?, strict)?;

    // The counting instance of the mixed-rank argument.
    let anchor = "generic_mixed_rank_proof_check";
    let v = get(anchor)?;
    let required = [
        "S_strict_high_drop_dominates_n",
        "high_blocks_at_most_R0",
        "low_blocks_at_most_n0_plus_N_R0",
        "literal_cost",
        "peak",
        "tail_sticker_addend",
        "selector_closure_and_complete_low_tail_availability_remain_hypotheses",
        "reused_sticker_theorem_not_recertified",
    ];
    keys(v, anchor, &required, &[])?;
    let g = |key: &str| field(v, key, anchor);
    for key in [
        "selector_closure_and_complete_low_tail_availability_remain_hypotheses",
        "reused_sticker_theorem_not_recertified",
    ] {
        let flag = boolean(g(key)?).ok_or_else(|| reject(dot(anchor, key), "expected a boolean"))?;
        if !flag {
            return Err(not_applicable(format!(
                "{key} is not true: discharging the selector, closure and availability premises, or recertifying the \
                 reused sticker theorem, is a global claim outside this kind (LB5.5)"
            )));
        }
        run.held(dot(anchor, key), "the premise stays a hypothesis; this instance does not discharge it");
    }
    let dominated = cleanup.n.iter().all(|&n| (0..=NP as i64).contains(&n));
    claim_bool(run, &dot(anchor, "S_strict_high_drop_dominates_n"), g("S_strict_high_drop_dominates_n")?, dominated)?;
    let (mut highs, mut lows, mut high_len, mut low_len) = (0i64, 0i64, 0usize, 0usize);
    for b in 0..cleanup.names.len() {
        let (r0, r1) = (r(&cleanup.q[b]), r(&cleanup.q[b + 1]));
        if r1 < r0 {
            highs += 1;
            high_len = high_len.max(cleanup.lengths[b]);
        } else if r1 == r0 && cleanup.n[b + 1] < cleanup.n[b] {
            lows += 1;
            low_len = low_len.max(cleanup.lengths[b]);
        } else {
            return Err(reject(
                anchor,
                format!("block {} neither lowers R nor keeps R and lowers n", cleanup.names[b]),
            ));
        }
    }
    let r0 = r(&cleanup.q[0]);
    let key = "high_blocks_at_most_R0";
    claim_bool(run, &dot(anchor, key), g(key)?, big(highs) <= r0)?;
    let key = "low_blocks_at_most_n0_plus_N_R0";
    claim_bool(run, &dot(anchor, key), g(key)?, big(lows) <= big(cleanup.n[0]).add(&big(NP as i64).mul(&r0)))?;
    let cost = format!("({high_len}+{low_len}N)R0+{low_len}n0");
    run.claim(
        &dot(anchor, "literal_cost"),
        &string(g("literal_cost")?, &dot(anchor, "literal_cost"))?.to_owned(),
        &cost,
    )?;
    let longest = cleanup.lengths.iter().copied().max().unwrap_or(0);
    let peak = format!("2R0+{}", 2 + 2 * longest);
    run.claim(&dot(anchor, "peak"), &string(g("peak")?, &dot(anchor, "peak"))?.to_owned(), &peak)?;
    run.held(
        "premise.length",
        format!(
            "R{L} covers the compact blocks {} (each at most {L} letters); the cleanup's low block of {low_len} \
             letters keeps R and lowers n, so S, not the length-{L} lemma, accounts for it. The universal bound keeps \
             the selector, availability and invariant-domain premises (LB5.5)",
            compact_names.join(", ")
        ),
    );
    run.held(
        "scope.not_checked",
        "tail_sticker_addend (the reused lattice sticker theorem's constant, not recertified here); the record's \
         frontier claims are s19.frontier's, and its q11 lower bound needs an absent cache",
    );
    Ok(())
}
