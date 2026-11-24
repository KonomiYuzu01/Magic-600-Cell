//! Stage 14's unlock levels, `t3p-levels/1` (part of LB5.15's second stage 14 family; the claim matrix's
//! `14/relations` groups of the `*-levels.json` records), rebuilt from a stored table and the model.
//!
//! Lemma L4 (PROOFS.md section 4): with I_c the pieces inside cap c, S_d the straddlers of grip d and Hold(p) = {c :
//! p ∈ I_c}, the level is the greatest solution of
//!
//! ```text
//! lev(c) = 0                                                  for c clean,
//! lev(d) = 1 + max over p ∈ S_d of min over c ∈ Hold(p) of lev(c)   for d blocked,
//! ```
//!
//! computed by monotone iteration from ∞ on the blocked grips: every sweep computes the next vector from the previous
//! one, a minimum over no finite holder is ∞, and the iteration stops at the first sweep that changes nothing; the
//! record's `iterations` counts the sweeps with that last one. λ (Corollary L4′) is the least level of a holder of a
//! piece of height at least 4, and its witness the least (level, piece, holder). Every label is recomputed at all 600
//! grips from the exact sign kernel, never read from a label cache, and the record's histogram is recounted.
//!
//! For each high unit (the pieces of one pose of height at least 4, in pose order) the record states its smallest
//! member, size, height and branch, its own holders (the grips of its branch's class that hold every member) with their
//! levels, and its members' holder rows at all 600 grips: the number of distinct rows, the grips that hold every
//! member, the grips that hold at least one, and the clean grips that hold some members but not all.
//!
//! The state is the stated table (role `table`, under the I_a menu of role `menu`), or, with the role `word`, the end
//! of a stored word from that table: a chain record (`start_digest`, `word`) or a replay record (`rows[0].digest`,
//! `word`). Every letter must be admissible, and the state reached must have the levels record's `digest`; another
//! start is the wrong start table (KN6.5).

use std::collections::{BTreeMap, BTreeSet};

use c600_jumble::model::{NP, POLES};
use c600_jumble::state::Status;
use c600_kernel::json::{self, Value};

use super::common::{self, claim_int, claim_str, engine, heights, labels, start};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, keys, reject, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::tree_tables;
use crate::report::{codes, Sources};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s14.levels",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s14/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s14/levels.rs", include_bytes!("levels.rs")),
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
    family: "S14 greatest-fixed-point unlock levels and high-unit holder rows",
};

pub const FORMAT: &str = "t3p-levels/1";
/// The optional input holding the word whose end the levels describe.
pub const WORD: &str = "word";
/// The scope sentence of every unit's holder rows.
const SCOPE: &str = "holder rows at all 600 grips, separately for every member";

/// A level: finite, or ∞ (`None`).
type Level = Option<u32>;

fn shown(level: Level) -> String {
    level.map_or_else(|| "infinity".to_owned(), |n| n.to_string())
}

fn stated_level(run: &mut Run, value: &Value, claim: &str, computed: Level) -> Step<()> {
    let stated = match value {
        Value::String(s) if s == "infinity" => "infinity".to_owned(),
        Value::Int(n) if *n >= 0 => n.to_string(),
        _ => return Err(reject(claim, "a level is a nonnegative integer or \"infinity\"")),
    };
    run.claim(claim, &stated, &shown(computed))
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engine(engines)?;
    let doc = document(run.cert.bytes)?;
    common::own(&doc, SOURCES.id)?;
    keys(
        &doc,
        "certificate",
        &[
            "cpu_s",
            "digest",
            "format",
            "high_units",
            "histogram",
            "iterations",
            "lambda",
            "lambda_witness",
            "levels",
            "setup",
        ],
        &[],
    )?;
    common::seconds(field(&doc, "cpu_s", "cpu_s")?, "cpu_s")?;
    common::setup(field(&doc, "setup", "setup")?, "setup")?;
    let stated = hex64(field(&doc, "digest", "digest")?, "digest")?.to_owned();

    // The state: the table, or the end of a stored word from it.
    let mut j = match run.optional_input(WORD) {
        None => start(ctx, run, "digest", &stated)?,
        Some(bytes) => {
            let record = json::parse(bytes).map_err(|e| reject(WORD, format!("the word record is not JSON: {e}")))?;
            let from = match record.get("start_digest") {
                Some(digest) => digest,
                None => field(
                    record.get("rows").and_then(|r| r.as_array()?.first()).unwrap_or(&Value::Null),
                    "digest",
                    WORD,
                )?,
            };
            let from = hex64(from, &format!("{WORD}.start"))?.to_owned();
            run.bind(WORD, bytes, vec![("start", from.clone())]);
            let mut j = start(ctx, run, &format!("{WORD}.start"), &from)?;
            let letters = common::word(field(&record, "word", WORD)?, &format!("{WORD}.word"), j.menu.items.len())?;
            for (k, &(cap, item)) in letters.iter().enumerate() {
                let claim = at(&format!("{WORD}.word"), k);
                let twist = j.twist(cap, item, &claim)?;
                if j.apply(&twist, &claim)?.status != Status::Admissible {
                    return Err(run.failed(reject(&claim, "the word's letter is refused"), "refused"));
                }
            }
            run.held(format!("{WORD}.word"), format!("{} letters admissible", letters.len()));
            run.compare("digest", &stated, &j.digest(), codes::START_TABLE_MISMATCH)?;
            j
        }
    };
    let state = std::mem::replace(&mut j.state, c600_jumble::state::State::new(None));
    let tables = tree_tables(ctx, "levels")?;
    let all: Vec<usize> = (0..POLES).collect();
    let table = labels(ctx, &state, &all, false, "levels")?;
    let hz = heights(ctx, tables, &state, "high_units")?;

    // Holders of the pieces off the lattice.
    let mut holders: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for c in 0..POLES {
        for &p in table.inside(c) {
            holders.entry(p).or_default().push(c);
        }
    }
    // The greatest fixed point, by sweeps from ∞ on the blocked grips.
    let clean: Vec<bool> = (0..POLES).map(|c| table.straddle(c).is_empty()).collect();
    let mut lev: Vec<Level> = clean.iter().map(|&c| if c { Some(0) } else { None }).collect();
    let mut sweeps = 0i64;
    loop {
        let next: Vec<Level> = (0..POLES)
            .map(|d| {
                if clean[d] {
                    return Some(0);
                }
                let mut worst: Level = Some(0);
                for p in table.straddle(d) {
                    let least = holders.get(p).into_iter().flatten().filter_map(|&c| lev[c]).min();
                    worst = match (worst, least) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        _ => None,
                    };
                }
                worst.map(|n| n + 1)
            })
            .collect();
        sweeps += 1;
        if next
            .iter()
            .zip(&lev)
            .any(|(n, l)| matches!((n, l), (None, Some(_))) || matches!((n, l), (Some(a), Some(b)) if a > b))
        {
            return Err(reject("iterations", "the iteration from infinity did not decrease"));
        }
        if next == lev {
            break;
        }
        lev = next;
    }
    let levels = array(field(&doc, "levels", "levels")?, "levels")?;
    run.claim("levels.length", &levels.len(), &POLES)?;
    for (c, value) in levels.iter().enumerate() {
        stated_level(run, value, &at("levels", c), lev[c])?;
    }
    claim_int(run, &doc, "", "iterations", sweeps)?;
    let mut histogram: BTreeMap<String, i64> = BTreeMap::new();
    for &l in &lev {
        *histogram.entry(shown(l)).or_default() += 1;
    }
    let stated_histogram = crate::claims::object(field(&doc, "histogram", "histogram")?, "histogram")?;
    let named: BTreeSet<&String> = stated_histogram.keys().collect();
    let computed: BTreeSet<&String> = histogram.keys().collect();
    if named != computed {
        let stop = reject("histogram", "the histogram names other levels than the recomputed ones");
        return Err(run.failed(stop, format!("{computed:?}")));
    }
    for (key, &count) in &histogram {
        claim_int(run, field(&doc, "histogram", "histogram")?, "histogram", key, count)?;
    }

    // λ and its witness.
    let high: Vec<u32> = (0..NP as u32).filter(|&p| hz.of(&state, p as usize).0 >= 4).collect();
    let mut best: Option<(Level, u32, usize)> = None;
    let key = |l: Level| l.map_or(u64::MAX, u64::from);
    for &p in &high {
        for &c in holders.get(&p).into_iter().flatten() {
            let candidate = (lev[c], p, c);
            if best.is_none_or(|b| (key(candidate.0), candidate.1, candidate.2) < (key(b.0), b.1, b.2)) {
                best = Some(candidate);
            }
        }
    }
    stated_level(run, field(&doc, "lambda", "lambda")?, "lambda", best.and_then(|b| b.0))?;
    let witness = field(&doc, "lambda_witness", "lambda_witness")?;
    match best {
        None => {
            if !matches!(witness, Value::Null) {
                return Err(
                    run.failed(reject("lambda_witness", "no high piece is held, so there is no witness"), "null")
                );
            }
            run.held("lambda_witness", "null");
        }
        Some((_, p, c)) => {
            keys(witness, "lambda_witness", &["holder", "piece"], &[])?;
            claim_int(run, witness, "lambda_witness", "piece", p as i64)?;
            claim_int(run, witness, "lambda_witness", "holder", c as i64)?;
        }
    }

    // The high units, in pose order.
    let class: Vec<i64> = (0..POLES)
        .map(|c| tables.cls(ctx, c).map_err(|e| reject("high_units", format!("the class of grip {c}: {e}"))))
        .collect::<Step<_>>()?;
    let mut units: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &p in &high {
        units.entry(state.pose_ids()[p as usize]).or_default();
    }
    for p in 0..NP as u32 {
        if let Some(members) = units.get_mut(&state.pose_ids()[p as usize]) {
            members.push(p);
        }
    }
    let stated_units = array(field(&doc, "high_units", "high_units")?, "high_units")?;
    run.claim("high_units.length", &stated_units.len(), &units.len())?;
    for (k, (value, (pose, members))) in stated_units.iter().zip(&units).enumerate() {
        let path = at("high_units", k);
        keys(value, &path, &["branch", "cell_split", "height", "own_holders", "pose", "size", "unit_piece"], &[])?;
        let (h, b) = hz.of(&state, members[0] as usize);
        claim_int(run, value, &path, "pose", *pose as i64)?;
        claim_int(run, value, &path, "unit_piece", members[0] as i64)?;
        claim_int(run, value, &path, "size", members.len() as i64)?;
        claim_int(run, value, &path, "height", h)?;
        claim_int(run, value, &path, "branch", b)?;
        let mut counts = vec![0i64; POLES];
        let mut rows: BTreeSet<&[usize]> = BTreeSet::new();
        for p in members {
            let row = holders.get(p).map_or(&[][..], |r| &r[..]);
            rows.insert(row);
            for &c in row {
                counts[c] += 1;
            }
        }
        let size = members.len() as i64;
        let own: Vec<usize> = (0..POLES).filter(|&c| counts[c] == size && class[c] == b).collect();
        let op = dot(&path, "own_holders");
        let stated_own = array(field(value, "own_holders", &path)?, &op)?;
        run.claim(&format!("{op}.length"), &stated_own.len(), &own.len())?;
        for (i, (entry, &c)) in stated_own.iter().zip(&own).enumerate() {
            let ep = at(&op, i);
            keys(entry, &ep, &["grip", "level"], &[])?;
            claim_int(run, entry, &ep, "grip", c as i64)?;
            stated_level(run, field(entry, "level", &ep)?, &dot(&ep, "level"), lev[c])?;
        }
        let cp = dot(&path, "cell_split");
        let cell = field(value, "cell_split", &path)?;
        keys(
            cell,
            &cp,
            &["clean_partial_holders", "distinct_holder_rows", "intersection_size", "scope", "union_size"],
            &[],
        )?;
        claim_str(run, cell, &cp, "scope", SCOPE)?;
        claim_int(run, cell, &cp, "distinct_holder_rows", rows.len() as i64)?;
        claim_int(run, cell, &cp, "intersection_size", counts.iter().filter(|&&n| n == size).count() as i64)?;
        claim_int(run, cell, &cp, "union_size", counts.iter().filter(|&&n| n > 0).count() as i64)?;
        let partial: Vec<(usize, i64)> = (0..POLES)
            .filter(|&c| lev[c] == Some(0) && counts[c] > 0 && counts[c] < size)
            .map(|c| (c, counts[c]))
            .collect();
        let pp = dot(&cp, "clean_partial_holders");
        let stated_partial = array(field(cell, "clean_partial_holders", &cp)?, &pp)?;
        run.claim(&format!("{pp}.length"), &stated_partial.len(), &partial.len())?;
        for (i, (entry, &(c, n))) in stated_partial.iter().zip(&partial).enumerate() {
            let ep = at(&pp, i);
            keys(entry, &ep, &["grip", "members"], &[])?;
            claim_int(run, entry, &ep, "grip", c as i64)?;
            claim_int(run, entry, &ep, "members", n)?;
        }
    }
    Ok(())
}
