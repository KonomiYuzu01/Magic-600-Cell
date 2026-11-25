//! `s19.frontier`: the compact and cleanup frontier exits of stage 19, replayed letter by letter from stored tables.
//!
//! Two archived records are read as they are:
//!
//! - `type-lex-trimmed-macro.json`, the compact route's C,E suffix from the post-A table. Inputs: the I_a menu
//!   (`menu`), the post-A table (`start`) and the compact exit's table (`end`). The checker replays the four letters,
//!   recomputing per prefix the moved count, the matrix contract on every piece and the metrics q, Φ, H, n; the
//!   endpoint digest against the stored end table; the q difference, its first changed type and the strict descent;
//!   the fixed-L4 rank before and after, which needs the word to have at most four letters (R_L's length premise);
//!   and the inverse letters, which must undo every prefix exactly.
//! - `type-lex-trimmed-independent-results.json`, the frontier part. Inputs: the menu, the source table Y (`origin`),
//!   the post-A table (`start`), both exits' tables (`end`, compact; `long`, cleanup) and the block words (`blocks`:
//!   Γ(Y)'s validation record, read only for its `word`, its block boundaries and its reference digest, which must be
//!   Y's). The checker replays A from Y to the stored post-A table, then C and E (the record's prefixes, with digests,
//!   moved counts, matrix contracts, metrics and the height of piece 114169), the inverse back to the post-A table, and
//!   D from the compact exit to the cleanup exit; it recomputes both exits' metrics, D's q-neutral drop of n and the
//!   height of all five compact prefixes.
//!
//! At both exits q0 to q15 are 0 and q16 is not: they are frontier exits, not restored states. The trace says that the
//! sticker goal fails there (n pieces off the lattice). Not checked here: the rank anchors of the second record
//! (`s19.ranks`), its q11 lower bound (its cache is absent), the global noninteraction of caps, and the records'
//! prose, labels, original-source digests and counts about the historical runs.

use std::sync::Arc;

use c600_jumble::state::State;
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::Value;

use super::common::{
    blocks, claim_bool, claim_decimal, claim_int, claim_ints, first_change, form, height, ia_menu, inverse, letters,
    metrics, minus, origin_and_start, outside, rank, read_table, solved, step, twist, type_counts, weights, Metrics,
    Q11, TRIMMED,
};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, keys, reject, string, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::report::{codes, Sources};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s19.frontier",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s19/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s19/frontier.rs", include_bytes!("frontier.rs")),
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
    family: "S19 frontier exits: compact A,C,E and cleanup A,D,C,E, full backgrounds, prefix heights and q16 frontier",
};

/// The compact suffix record.
const MACRO: [&str; 21] = [
    "status",
    "source_ref",
    "input_hashes",
    "input_digest",
    "word",
    "tested_word_count",
    "full_letters",
    "prefixes",
    "inverse",
    "scope",
    "global_deleted_critical_pairs",
    "all_deleted_critical_disjoint",
    "before",
    "after",
    "completed_word",
    "delta_q",
    "first_changed_type",
    "strict_q_descent",
    "fixed_L4_rank",
    "endpoint",
    "all_applied_prefix_inverse_exact",
];

/// The piece whose height the trimmed record follows (its sole type-11 high piece after A).
const TARGET: usize = 114_169;

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = document(run.cert.bytes)?;
    match form(&doc, &[&MACRO, &TRIMMED]) {
        Some(0) => {
            keys(&doc, "certificate", &MACRO, &[])?;
            compact_suffix(ctx, run, &doc)
        }
        Some(_) => {
            keys(&doc, "certificate", &TRIMMED, &[])?;
            exits(ctx, run, &doc)
        }
        None => Err(outside(SOURCES.id, "stage 19's frontier exit records", &doc)),
    }
}

/// Compares a stated `{q, Phi, H, n}` with computed metrics.
pub fn claim_metrics(run: &mut Run, claim: &str, value: &Value, m: &Metrics) -> Step<()> {
    keys(value, claim, &["q", "Phi", "H", "n"], &[])?;
    let get = |key: &str| field(value, key, claim);
    claim_ints(run, &dot(claim, "q"), get("q")?, &m.q)?;
    claim_int(run, &dot(claim, "Phi"), get("Phi")?, m.phi)?;
    claim_int(run, &dot(claim, "H"), get("H")?, m.h)?;
    claim_int(run, &dot(claim, "n"), get("n")?, m.n)
}

/// The trace's account of an exit: the frontier coordinates and the sticker goal it does not reach.
fn exit_note(run: &mut Run, claim: &str, state: &State, m: &Metrics) {
    let cleared = m.q.iter().take(16).all(|&q| q == 0);
    let q16 = m.q.get(16).copied().unwrap_or(0);
    let goal = if solved(state) { "holds" } else { "fails" };
    run.held(
        claim,
        format!(
            "q0 to q15 {}, q16 = {q16}; the sticker goal {goal}: {} pieces are off the lattice. A frontier exit is \
             not a restored state (LB5.5)",
            if cleared { "are 0" } else { "are not all 0" },
            m.n
        ),
    );
}

/// `type-lex-trimmed-macro.json`.
fn compact_suffix(ctx: &Context, run: &mut Run, doc: &Value) -> Step<()> {
    let get = |key: &str| field(doc, key, key);
    let menu = ia_menu(ctx, run, "menu")?;
    let start = read_table(ctx, Some(menu.clone()), run, "start")?;
    let stated = hex64(get("input_digest")?, "input_digest")?.to_owned();
    run.compare("input_digest", &stated, &start.digest, codes::START_TABLE_MISMATCH)?;
    let word = letters(get("word")?, "word", menu.items.len())?;
    run.held("word", format!("{} letters, the subject", word.len()));
    claim_int(run, "tested_word_count", get("tested_word_count")?, 1)?;

    let mut state = start.state.clone();
    let before = metrics(ctx, &state, "before")?;
    claim_metrics(run, "before", get("before")?, &before)?;
    let prefixes = array(get("prefixes")?, "prefixes")?;
    if prefixes.len() != word.len() {
        return Err(reject("prefixes", format!("{} prefixes for {} letters", prefixes.len(), word.len())));
    }
    let mut digests = vec![start.digest.clone()];
    for (i, (&letter, entry)) in word.iter().zip(prefixes).enumerate() {
        let path = at("prefixes", i);
        keys(entry, &path, &["status", "cap", "item", "inside_count", "allN_matrix_contract", "metrics"], &[])?;
        let g = |key: &str| field(entry, key, &path);
        claim_int(run, &dot(&path, "cap"), g("cap")?, letter.0 as i64)?;
        claim_int(run, &dot(&path, "item"), g("item")?, letter.1 as i64)?;
        let applied = step(ctx, &mut state, &twist(ctx, &menu, letter, &path)?, &path)?;
        run.claim(
            &dot(&path, "status"),
            &string(g("status")?, &dot(&path, "status"))?.to_owned(),
            &"APPLIED".to_owned(),
        )?;
        claim_int(run, &dot(&path, "inside_count"), g("inside_count")?, applied.inside.len() as i64)?;
        claim_bool(run, &dot(&path, "allN_matrix_contract"), g("allN_matrix_contract")?, applied.contract)?;
        claim_metrics(run, &dot(&path, "metrics"), g("metrics")?, &metrics(ctx, &state, &path)?)?;
        digests.push(state.digest(ctx));
    }
    claim_bool(run, "completed_word", get("completed_word")?, true)?;
    let after = metrics(ctx, &state, "after")?;
    claim_metrics(run, "after", get("after")?, &after)?;
    let delta = minus(&after.q, &before.q);
    claim_ints(run, "delta_q", get("delta_q")?, &delta)?;
    let first = first_change(&delta);
    claim_int(run, "first_changed_type", get("first_changed_type")?, first.map_or(-1, |(t, _)| t as i64))?;
    claim_bool(run, "strict_q_descent", get("strict_q_descent")?, first.is_some_and(|(_, d)| d < 0))?;

    // R_L with L = 4: its length premise covers this word only if it has at most four letters.
    let rank4 = get("fixed_L4_rank")?;
    keys(rank4, "fixed_L4_rank", &["before", "after", "drop"], &[])?;
    if word.len() > 4 {
        return Err(reject(
            "fixed_L4_rank",
            format!("R_L's length premise: a {}-letter word exceeds L = 4", word.len()),
        ));
    }
    let w = weights(&type_counts(ctx), 4);
    let (r0, r1) = (rank(&w, &before.q), rank(&w, &after.q));
    claim_decimal(run, "fixed_L4_rank.before", field(rank4, "before", "fixed_L4_rank")?, &r0)?;
    claim_decimal(run, "fixed_L4_rank.after", field(rank4, "after", "fixed_L4_rank")?, &r1)?;
    claim_decimal(run, "fixed_L4_rank.drop", field(rank4, "drop", "fixed_L4_rank")?, &r0.sub(&r1))?;

    let end = get("endpoint")?;
    keys(end, "endpoint", &["digest", "table", "sha256"], &[])?;
    let final_digest = state.digest(ctx);
    let stated = hex64(field(end, "digest", "endpoint")?, "endpoint.digest")?.to_owned();
    run.claim("endpoint.digest", &stated, &final_digest)?;
    stored_end(ctx, run, &menu, "end", &final_digest)?;
    exit_note(run, "goal.sticker", &state, &after);

    // The inverse letters undo every prefix.
    let back = letters(get("inverse")?, "inverse", menu.items.len())?;
    let expected =
        word.iter().rev().enumerate().map(|(i, &l)| inverse(&menu, l, &at("inverse", i))).collect::<Step<Vec<_>>>()?;
    if back != expected {
        return Err(reject("inverse", format!("stated {back:?}, the inverse of the word is {expected:?}")));
    }
    run.held("inverse", "the word's inverse, letter by letter");
    let mut exact = true;
    for (i, &letter) in back.iter().enumerate() {
        let path = at("inverse", i);
        step(ctx, &mut state, &twist(ctx, &menu, letter, &path)?, &path)?;
        exact &= state.digest(ctx) == digests[digests.len() - 2 - i];
    }
    claim_bool(run, "all_applied_prefix_inverse_exact", get("all_applied_prefix_inverse_exact")?, exact)?;
    claim_int(run, "full_letters", get("full_letters")?, (word.len() + back.len()) as i64)?;
    run.held(
        "scope.not_checked",
        "global_deleted_critical_pairs and all_deleted_critical_disjoint (global cap noninteraction), status, scope, \
         source_ref, endpoint.table (the end table's original name) and the original-source digests of inputs and \
         the end table",
    );
    Ok(())
}

/// A stored end table that must hold the replayed endpoint.
fn stored_end(ctx: &Context, run: &mut Run, menu: &Arc<Menu>, role: &str, digest: &str) -> Step<State> {
    let table = read_table(ctx, Some(menu.clone()), run, role).map_err(|stop| match stop {
        crate::claims::Stop::Fail { reason, .. } => reject(role, reason),
        other => other,
    })?;
    if table.digest != digest {
        return Err(reject(role, format!("the stored table has digest {}, the replay ends at {digest}", table.digest)));
    }
    Ok(table.state)
}

/// `type-lex-trimmed-independent-results.json`, the frontier part.
fn exits(ctx: &Context, run: &mut Run, doc: &Value) -> Step<()> {
    let get = |key: &str| field(doc, key, key);
    let menu = ia_menu(ctx, run, "menu")?;
    let blocks = blocks(run, "blocks", menu.items.len())?;
    let (_, start) = origin_and_start(ctx, run, &menu, &blocks)?;
    let mut heights = vec![metrics(ctx, &start, "start")?.h];

    // C and E from the post-A table: the record's prefixes.
    let suffix: Vec<(usize, usize)> = [blocks.get("C")?, blocks.get("E")?].concat();
    let prefixes = array(get("prefixes")?, "prefixes")?;
    if prefixes.len() != suffix.len() {
        return Err(reject("prefixes", format!("{} prefixes for C and E's {} letters", prefixes.len(), suffix.len())));
    }
    let mut state = start.clone();
    for (i, (&letter, entry)) in suffix.iter().zip(prefixes).enumerate() {
        let path = at("prefixes", i);
        let required = [
            "index",
            "letter",
            "digest",
            "inside_count",
            "all177120_matrix_contract",
            "metrics",
            "target114169_height",
        ];
        keys(entry, &path, &required, &[])?;
        let g = |key: &str| field(entry, key, &path);
        claim_int(run, &dot(&path, "index"), g("index")?, i as i64 + 1)?;
        let stated = crate::replay::letter(g("letter")?, &dot(&path, "letter"), menu.items.len())?;
        if stated != letter {
            return Err(reject(dot(&path, "letter"), format!("stated {stated:?}, blocks C and E give {letter:?}")));
        }
        let applied = step(ctx, &mut state, &twist(ctx, &menu, letter, &path)?, &path)?;
        let stated = hex64(g("digest")?, &dot(&path, "digest"))?.to_owned();
        run.claim(&dot(&path, "digest"), &stated, &state.digest(ctx))?;
        claim_int(run, &dot(&path, "inside_count"), g("inside_count")?, applied.inside.len() as i64)?;
        claim_bool(run, &dot(&path, "all177120_matrix_contract"), g("all177120_matrix_contract")?, applied.contract)?;
        let m = metrics(ctx, &state, &path)?;
        claim_metrics(run, &dot(&path, "metrics"), g("metrics")?, &m)?;
        heights.push(m.h);
        claim_int(
            run,
            &dot(&path, "target114169_height"),
            g("target114169_height")?,
            height(ctx, &state, TARGET, &path)?,
        )?;
    }

    // The compact exit.
    let short = read_table(ctx, Some(menu.clone()), run, "end").map_err(|stop| match stop {
        crate::claims::Stop::Fail { reason, .. } => reject("end", reason),
        other => other,
    })?;
    let key = "all177120_short_endpoint_match";
    claim_bool(run, key, get(key)?, state.digest(ctx) == short.digest)?;
    let short_m = metrics(ctx, &short.state, "canonical_short_metrics")?;
    claim_metrics(run, "canonical_short_metrics", get("canonical_short_metrics")?, &short_m)?;
    exit_note(run, "goal.sticker.compact", &short.state, &short_m);
    let key = "all_five_short_forward_prefixes_H4";
    claim_bool(run, key, get(key)?, heights.iter().all(|&h| h == 4))?;

    // The inverse back to the post-A table.
    let mut back = state.clone();
    for (i, &letter) in suffix.iter().rev().enumerate() {
        let path = at("inverse", i);
        let undo = inverse(&menu, letter, &path)?;
        step(ctx, &mut back, &twist(ctx, &menu, undo, &path)?, &path)?;
    }
    let key = "independent_inverse_all177120_exact";
    claim_bool(run, key, get(key)?, back.digest(ctx) == start.digest(ctx))?;

    // D from the compact exit gives the cleanup exit.
    let long = read_table(ctx, Some(menu.clone()), run, "long").map_err(|stop| match stop {
        crate::claims::Stop::Fail { reason, .. } => reject("long", reason),
        other => other,
    })?;
    let mut joined = short.state.clone();
    for (i, &letter) in blocks.get("D")?.iter().enumerate() {
        let path = at("blocks.D", i);
        step(ctx, &mut joined, &twist(ctx, &menu, letter, &path)?, &path)?;
    }
    let key = "all177120_long_equals_short_plus_recorded_D";
    claim_bool(run, key, get(key)?, joined.digest(ctx) == long.digest)?;
    let long_m = metrics(ctx, &long.state, "canonical_long_metrics")?;
    claim_metrics(run, "canonical_long_metrics", get("canonical_long_metrics")?, &long_m)?;
    exit_note(run, "goal.sticker.cleanup", &long.state, &long_m);
    if short_m.q != long_m.q {
        return Err(reject(
            "D_exact_q_neutral_off_drop",
            "the exits' q vectors differ: D is not q-neutral between them",
        ));
    }
    claim_int(run, "D_exact_q_neutral_off_drop", get("D_exact_q_neutral_off_drop")?, short_m.n - long_m.n)?;

    run.held("scope.q11_depth_certificate", Q11);
    run.held("scope.ranks", "one_fixed_L3, one_fixed_S and generic_mixed_rank_proof_check are s19.ranks' claims");
    run.held(
        "scope.not_checked",
        "all12_disjoint_pairs and cap282_noninteraction (global cap noninteraction), the counts and caches of the \
         historical run, findings, phase, status, scope, the reused-cache premise and the original-source digests",
    );
    Ok(())
}
