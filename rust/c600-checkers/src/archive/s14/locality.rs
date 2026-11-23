//! Stage 14's locality summary, `t3p-locality/1` (part of LB5.15's second stage 14 family; the claim matrix's
//! `14/relations` group of `results/locality-summary.json`), recomputed from the model.
//!
//! Lemma L1 (PROOFS.md section 1): with |n|² the squared norm of a pole vector (the model's NN), κ the cut offset and R²
//! the largest squared norm of a vertex of cap 0's region,
//!
//! ```text
//! Δ(c, d) = R²(|n|² + n_c·n_d) − 2κ²  ∈ Q(√5),
//! ```
//!
//! and distinct grips interact when Δ(c, d) > 0. The check computes R² from cap 0's exact region vertices, every
//! n_c·n_d from the exact pole vectors, and Δ once for each distinct product. It then compares, as exact text, R², NN
//! and κ; every grip's partner count, whether the counts are equal and their common value; and the shells around grip
//! 0: the grips d grouped by n_0·n_d in decreasing order, each with its product, size, grips, Δ(0, d) and whether it
//! interacts (grip 0 itself never does). The summary omits the archive output's per-pair `delta_index`,
//! `delta_values` and `partners` (README section 7); the counts above are recomputed over all pairs, not read from them.
//!
//! The record depends on the model alone. Its stated input is the model manifest (`assets/manifest.json`) under the
//! role `model`, whose `model_id` must be the loaded model's: another model is the wrong start input (KN6.5).

use std::collections::BTreeMap;

use c600_jumble::model::POLES;
use c600_jumble::q5::{dot, Q5};
use c600_kernel::json::{self, Value};
use c600_kernel::parallel;

use super::common::{self, claim_bool, claim_int, claim_ints, claim_str, engine, exact_text};
use crate::archive::SubChecker;
use crate::claims::{
    array, at, document, field, keys, not_applicable, reject, require_format, start_mismatch, Run, Step,
};
use crate::kinds::{Checker, Needs};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s14.locality",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s14/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s14/locality.rs", include_bytes!("locality.rs")),
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
    family: "S14 locality (Lemma L1), recomputed from the model",
};

pub const FORMAT: &str = "t3p-locality/1";
/// The input role of the model manifest.
pub const MODEL: &str = "model";

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engine(engines)?;
    let doc = document(run.cert.bytes)?;
    if let Some(reason) = common::unchecked_scope(&doc) {
        return Err(not_applicable(reason));
    }
    require_format(&doc, FORMAT)?;
    keys(
        &doc,
        "certificate",
        &["NN", "R2", "equal_partner_counts", "format", "kappa", "partner_count", "partner_counts", "shells"],
        &["cpu_s", "delta_encoding"],
    )?;
    let model = &ctx.model;
    let bytes = run.optional_input(MODEL).ok_or_else(|| start_mismatch(MODEL, "the model manifest is absent"))?;
    let manifest = json::parse(bytes).map_err(|error| start_mismatch(MODEL, format!("not JSON: {error}")))?;
    let named = manifest.get("model_id").and_then(Value::as_str).unwrap_or("").to_owned();
    run.bind(MODEL, bytes, vec![("model_id", named.clone())]);
    run.compare(&format!("{MODEL}.model_id"), &named, &model.model_id, crate::report::codes::START_TABLE_MISMATCH)?;

    let r2 = ctx.regions.cap_vertices.iter().map(|v| dot(v, v)).max().ok_or_else(|| reject("R2", "no vertices"))?;
    claim_str(run, &doc, "", "R2", &exact_text(&r2))?;
    claim_str(run, &doc, "", "NN", &exact_text(&model.nn))?;
    claim_str(run, &doc, "", "kappa", &exact_text(&model.kappa))?;
    let two_kappa_sq = model.kappa.mul(&model.kappa).mul(&Q5::int(2));
    let delta = |product: &Q5| r2.mul(&model.nn.add(product)).sub(&two_kappa_sq);

    // Every product n_c·n_d with c < d, by grip c, on the machine's threads.
    let rows =
        parallel::map(POLES, |c| (c + 1..POLES).map(|d| dot(&model.poles[c], &model.poles[d])).collect::<Vec<Q5>>());
    let mut deltas: BTreeMap<Q5, Q5> = BTreeMap::new();
    let mut counts = vec![0i64; POLES];
    for (c, row) in rows.into_iter().enumerate() {
        let row = row.map_err(|_| reject("partner_counts", "a product evaluation did not finish"))?;
        for (k, product) in row.into_iter().enumerate() {
            let value = deltas.entry(product).or_insert_with_key(|p| delta(p));
            if value.sign() > 0 {
                counts[c] += 1;
                counts[c + 1 + k] += 1;
            }
        }
    }
    claim_ints(run, field(&doc, "partner_counts", "partner_counts")?, "partner_counts", &counts)?;
    let equal = counts.iter().all(|&n| n == counts[0]);
    claim_bool(run, &doc, "", "equal_partner_counts", equal)?;
    let count = field(&doc, "partner_count", "partner_count")?;
    if equal {
        claim_int(run, &doc, "", "partner_count", counts[0])?;
    } else if !matches!(count, Value::Null) {
        return Err(run.failed(reject("partner_count", "the counts differ, so there is no common count"), "null"));
    }

    // The shells around grip 0, by decreasing n_0·n_d.
    let mut shells: BTreeMap<Q5, Vec<usize>> = BTreeMap::new();
    for d in 0..POLES {
        shells.entry(dot(&model.poles[0], &model.poles[d])).or_default().push(d);
    }
    let stated = array(field(&doc, "shells", "shells")?, "shells")?;
    run.claim("shells.length", &stated.len(), &shells.len())?;
    for (k, (value, (product, grips))) in stated.iter().zip(shells.iter().rev()).enumerate() {
        let path = at("shells", k);
        keys(value, &path, &["count", "delta", "dot", "grips", "interacts"], &[])?;
        let shell_delta = delta(product);
        claim_str(run, value, &path, "dot", &exact_text(product))?;
        claim_int(run, value, &path, "count", grips.len() as i64)?;
        let ids: Vec<i64> = grips.iter().map(|&d| d as i64).collect();
        claim_ints(run, field(value, "grips", &path)?, &format!("{path}.grips"), &ids)?;
        claim_str(run, value, &path, "delta", &exact_text(&shell_delta))?;
        claim_bool(run, value, &path, "interacts", grips[0] != 0 && shell_delta.sign() > 0)?;
    }
    Ok(())
}
