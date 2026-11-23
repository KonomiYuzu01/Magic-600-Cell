//! `s15.pstar`: the records of P* and two-level P* (stage 15, sections 2.4 and 2.5): the clean-pair census
//! `pstar-census/1` checked in full, and every stop and run record answered `not_applicable` with its own scope.
//!
//! # The census (family L's census; LB5.15's relations row)
//!
//! A census states, at a configuration, the clean grips and every pair of clean grips whose caps share a piece of
//! height at least 4 (`pairs`), with the lens tests of each pair. Inputs: `chain` (stage 14's `t3p-chain/1` record of the
//! chain end), `table` (the chain's D2 end table), `menu` (I_a). The table must be the chain's start table (its digest
//! is the chain's `start_digest`), and the census's prefix must begin with the chain's word. The prefix is replayed
//! letter by letter; then:
//!
//! - `digest` and `phi` are the state's, and when the prefix is the chain's word they are the chain's end;
//! - `clean_grips` is the number of grips with no straddler, every grip classified exactly;
//! - the pairs are recomputed: for every piece of left height at least 4, the clean grips whose caps hold it, and every
//!   two of them. A census whose recomputed pair list is empty must state no pair, `pair_count` 0, no candidate and no
//!   unencodable cell (a candidate comes only from a pair's lens tests), and `exhausted` (the pair list is complete).
//!   A census with pairs must state as many as are recomputed (`pairs`, `pair_count`); their lens tests, which this
//!   sub-checker does not recompute, make the rest of it `not_applicable`.
//!
//! # Stops and runs (LB5.15's selector rows; no completed negative)
//!
//! A stop of P* is a stop of its declared search, and an exhaustive empty family is a negative for that family only
//! (Proposition BN, restricted to the declaration). No KN6 checker of a search's coverage exists, so every stop record
//! is `not_applicable`, and its reason states the record's own bounds, read from it, so that the scopes stay apart:
//!
//! - `pstar-step/1`: a step of P*'s declared search, family R with a beam per target (exhaustive only through the
//!   letters its record names; a beam refutes no word it dropped), or a family restricted search such as E with
//!   E₀ = 200;
//! - `pstar2-stop/1`: the unbeamed stop search of `pstar2.py`, family R to its depth without a beam, exhaustive under its
//!   exact pruning condition: a negative for family R as declared, not for all words;
//! - `pstar2/2`: a two-level run, whose stop is a negative for its declared families only;
//! - a run wrapper whose format is only inside (`run.format` is `pstar-run/1`): a declared run's trimmed record.
//!
//! A stop is never refuted here: nothing in its record is read as a claim this sub-checker could fail. A record that
//! matches none of these forms (another format, or none outside a run wrapper) fails at `format`.

use std::collections::{BTreeMap, BTreeSet};

use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_jumble::state::Status;
use c600_kernel::json::{self, Value};

use super::grips::{clean_grips, inside};
use super::jam::{boolean, letters, show, Letter};
use crate::archive::SubChecker;
use crate::claims::{array, document, field, hex64, int, keys, not_applicable, reject, start_mismatch, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{left_facts, phi, read_menu, read_state, tree_tables, Jumbling};
use crate::report::{codes, Sources};
use crate::Engines;

pub const CENSUS: &str = "pstar-census/1";
pub const STEP: &str = "pstar-step/1";
pub const STOP: &str = "pstar2-stop/1";
pub const TWO_LEVEL: &str = "pstar2/2";
pub const RUN: &str = "pstar-run/1";
/// Stage 14's chain record, the census's binding input.
pub const CHAIN: &str = "t3p-chain/1";

pub static SOURCES: Sources = Sources::new(
    "s15.pstar",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s15/grips.rs", include_bytes!("grips.rs")),
        ("c600-checkers/src/archive/s15/jam.rs", include_bytes!("jam.rs")),
        ("c600-checkers/src/archive/s15/pstar.rs", include_bytes!("pstar.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUB: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 15,
    family: "S15 P*, two-level P*, pstar2/2, beamed and unbeamed stop records (scope only); the P* clean-pair census",
};

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let doc = document(run.cert.bytes)?;
    let format = doc.get("format").and_then(Value::as_str);
    match format {
        Some(CENSUS) => census(engines, run, &doc),
        Some(STEP) => Err(not_applicable(step_scope(&doc))),
        Some(STOP) => Err(not_applicable(stop_scope(&doc))),
        Some(TWO_LEVEL) => Err(not_applicable(two_level_scope(&doc))),
        None if doc.get("run").and_then(|r| r.get("format")).and_then(Value::as_str) == Some(RUN) => {
            Err(not_applicable(run_scope(&doc)))
        }
        Some(other) => Err(reject("format", format!("the record's format is {other}, not a record of P*"))),
        None => Err(reject("format", "the record states no format and is not a declared run of P*")),
    }
}

// ------------------------------------------------------------------------------------------------------- census

/// Stage 14's chain record: start table digest, word, end digest and Φ.
struct Chain {
    start: String,
    word: Vec<Letter>,
    end: String,
    end_phi: i64,
}

fn read_chain(run: &mut Run) -> Step<Chain> {
    let bytes = run.input("chain")?;
    let doc = json::parse(bytes).map_err(|error| reject("chain", format!("the chain record is not JSON: {error}")))?;
    if doc.get("format").and_then(Value::as_str) != Some(CHAIN) {
        return Err(reject("chain", format!("the chain record is not a {CHAIN}")));
    }
    let chain = Chain {
        start: hex64(field(&doc, "start_digest", "chain")?, "chain.start_digest")?.to_owned(),
        word: letters(field(&doc, "word", "chain")?, "chain.word")?,
        end: hex64(field(&doc, "end_digest", "chain")?, "chain.end_digest")?.to_owned(),
        end_phi: int(field(&doc, "end_phi", "chain")?, "chain.end_phi")?,
    };
    run.bind("chain", bytes, vec![("start_digest", chain.start.clone()), ("end_digest", chain.end.clone())]);
    Ok(chain)
}

fn census(engines: &Engines, run: &mut Run, doc: &Value) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    keys(
        doc,
        "certificate",
        &[
            "format",
            "digest",
            "phi",
            "prefix",
            "clean_grips",
            "pairs",
            "pair_count",
            "candidates",
            "unencodable",
            "exhausted",
        ],
        &["cpu_s", "setup"],
    )?;
    let menu = read_menu(ctx, run, "menu")?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    let chain = read_chain(run)?;
    if run.optional_input("table").is_none() {
        return Err(start_mismatch("table", "the chain's start table is absent"));
    }
    let table = read_state(ctx, run, &menu, "table")?;
    let mut j = Jumbling::new(ctx, menu, table);
    run.compare("table", &chain.start, &j.digest(), codes::START_TABLE_MISMATCH)?;

    let prefix = letters(field(doc, "prefix", "prefix")?, "prefix")?;
    if !prefix.starts_with(&chain.word) {
        let stop = reject("prefix", "the census's prefix does not begin with the chain's word");
        return Err(run.failed(stop, show(&prefix)));
    }
    for (k, &(cap, item)) in prefix.iter().enumerate() {
        let claim = crate::claims::at("prefix", k);
        let twist = j.twist(cap, item, &claim)?;
        let outcome = j.apply(&twist, &claim)?;
        if outcome.status != Status::Admissible {
            return Err(run.failed(reject(&claim, format!("the letter is {}", outcome.status.as_str())), "refused"));
        }
        run.held(&claim, "admissible");
    }
    let tables = tree_tables(ctx, "phi")?;
    let digest = j.digest();
    let phi_now = phi(ctx, tables, &j.state, "phi")?;
    if prefix == chain.word {
        run.claim("chain.end_digest", &chain.end, &digest)?;
        run.claim("chain.end_phi", &chain.end_phi, &phi_now)?;
    }
    run.claim("digest", &hex64(field(doc, "digest", "digest")?, "digest")?.to_owned(), &digest)?;
    run.claim("phi", &int(field(doc, "phi", "phi")?, "phi")?, &phi_now)?;

    let clean = clean_grips(ctx, &mut j.state, "clean_grips")?;
    let count = clean.iter().filter(|&&c| c).count() as i64;
    run.claim("clean_grips", &int(field(doc, "clean_grips", "clean_grips")?, "clean_grips")?, &count)?;

    // Every piece of left height at least 4, with the clean grips whose caps hold it.
    let mut holders: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    let mut heights: BTreeMap<u32, i64> = BTreeMap::new();
    for x in (0..clean.len()).filter(|&x| clean[x]) {
        for p in inside(ctx, &mut j.state, x, "pairs")? {
            let pid = j.state.pose_ids()[p as usize];
            let h = match heights.get(&pid) {
                Some(&h) => h,
                None => {
                    let (h, _) = left_facts(ctx, tables, &j.state.poses()[pid as usize], "pairs")?;
                    heights.insert(pid, h);
                    h
                }
            };
            if h >= 4 {
                holders.entry(p).or_default().push(x);
            }
        }
    }
    let mut pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
    for grips in holders.values() {
        for (a, &x) in grips.iter().enumerate() {
            for &z in &grips[a + 1..] {
                pairs.insert((x.min(z), x.max(z)));
            }
        }
    }
    let stated = array(field(doc, "pairs", "pairs")?, "pairs")?;
    run.claim("pairs", &stated.len(), &pairs.len())?;
    run.claim("pair_count", &int(field(doc, "pair_count", "pair_count")?, "pair_count")?, &(pairs.len() as i64))?;
    if !pairs.is_empty() {
        return Err(not_applicable(format!(
            "the census states {} clean pairs, as recomputed; their lens tests, and the candidates, unencodable cells              and exhaustion that rest on them, are outside this sub-checker",
            stated.len()
        )));
    }
    let candidates = array(field(doc, "candidates", "candidates")?, "candidates")?.len();
    run.claim("candidates", &candidates, &0)?;
    let unencodable = array(field(doc, "unencodable", "unencodable")?, "unencodable")?.len();
    run.claim("unencodable", &unencodable, &0)?;
    run.claim("exhausted", &boolean(field(doc, "exhausted", "exhausted")?, "exhausted")?, &true)?;
    Ok(())
}

// --------------------------------------------------------------------------------------------------- the scopes

/// A record value as text, or `unstated`.
fn text(value: Option<&Value>) -> String {
    match value {
        None => "unstated".to_owned(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "none".to_owned(),
        Some(other) => other.canonical(),
    }
}

fn families(record: &Value) -> String {
    match record.get("families").and_then(Value::as_object) {
        None => "unstated".to_owned(),
        Some(map) => map
            .iter()
            .map(|(name, family)| {
                let exhausted = text(family.get("exhausted"));
                match family.get("exhaustive_through") {
                    Some(n) => format!("{name} (exhausted {exhausted}, exhaustive through {})", n.canonical()),
                    None => format!("{name} (exhausted {exhausted})"),
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn step_scope(doc: &Value) -> String {
    let options = doc.get("options");
    let beam = options.and_then(|o| o.get("beam"));
    format!(
        "a beamed stop of P*'s declared step search at digest {} (families {}; E0 {}; R to depth {} with a beam of {} \
         per target; per family: {}): an exhaustive empty family is a negative for that family only, and the beam \
         refutes no word it dropped; no KN6 checker of the search's coverage exists (LB5.15)",
        text(doc.get("digest")),
        text(options.and_then(|o| o.get("families"))),
        text(options.and_then(|o| o.get("e0"))),
        text(options.and_then(|o| o.get("depth"))),
        text(beam),
        families(doc),
    )
}

fn stop_scope(doc: &Value) -> String {
    let options = doc.get("options");
    format!(
        "an unbeamed stop search of two-level P* at digest {} (families {}; E0 {}; R to depth {} without a beam, \
         search {}, beam {}; per family: {}): a negative for the declared families as searched, under the search's \
         exact pruning condition, not for all words; no KN6 checker of the search's coverage exists (LB5.15)",
        text(doc.get("digest")),
        text(options.and_then(|o| o.get("families"))),
        text(options.and_then(|o| o.get("e0"))),
        text(options.and_then(|o| o.get("depth"))),
        text(options.and_then(|o| o.get("search"))),
        text(doc.get("stop_options").and_then(|o| o.get("beam"))),
        families(doc),
    )
}

fn two_level_scope(doc: &Value) -> String {
    format!(
        "a two-level P* run ({}, status {}, reason {}; options {}; stop options {}): its stop is a negative for the \
         declared families with R searched again without a beam (Proposition BN restricted to the families), not for \
         all words; no KN6 checker of the search's coverage exists (LB5.15)",
        text(doc.get("label")),
        text(doc.get("status")),
        text(doc.get("reason")),
        text(doc.get("options")),
        text(doc.get("stop_options")),
    )
}

fn run_scope(doc: &Value) -> String {
    format!(
        "a declared P* run record ({}, {}): its stop is a stop of the declared search with family R's beam, not a \
         negative certificate for family R; no KN6 checker of the search's coverage exists (LB5.15)",
        text(doc.get("kind")),
        text(doc.get("label")),
    )
}
