//! `s16.reductions`: stage 16's transport, routing, T1 and leaf or alphabet records (LB5.15's reduction row).
//!
//! # Reduced transport against the recorded unreduced case
//!
//! Family X's search (`transport.py`) applies the reductions R1 to R3 (merging letters at one grip, commuting letters
//! at grips that do not interact, minimality). Stage 16 records one comparison with the unreduced enumeration, at the
//! 7307 jam with K = 2 (`results/transport/k2raw-7307.json`, `raw: true`, against `k2-7307.json` and `k2m-7307.json`;
//! README section 2.3). This sub-checker checks that comparison as recorded: the reduced record is the certificate and
//! the unreduced one the input `unreduced`, with `jams`, `table` and `menu`. Both records must state the same seed,
//! extra prefix, start digest, Φ and K; the start is recomputed from the seed's exact table (the jam word, then the
//! extra prefix, letter by letter); and both must give the same best word per target, each replayed letter by letter
//! from the start: K retained letters, each admissible, then a jumble at the target whose F, D and terminal Φ are the
//! stated ones (F and D read before the lowering, as the macro kind reads them).
//!
//! A pass says that the two records agree at this recorded input and that each best word does what it states. It is
//! not a lemma: the reductions keep their proofs (`PROOFS.md`, Theorem X), the agreement is not evidence for any
//! other input, and a reduced record without its unreduced counterpart is `not_applicable`. The completeness of either
//! search (its counts, every leaf tried) is not checked: no KN6 checker of a search's coverage exists.
//!
//! # Every other record of the family
//!
//! T1 leaf negatives, routing release searches with a prize (beams), alphabet-restricted runs, one-helper scans,
//! routing obligation scans, two-carrier shards and the runs of the driver P** keep their own bounds: each is
//! `not_applicable`, and its reason states the bounds read from the record. None is refuted here. Each form is told by
//! one field and then checked whole (its fields, no other, and the bounds the reason states of their kind); a record
//! of none of these forms, or one that departs from its form, fails at the departing field.

use std::collections::BTreeMap;

use c600_jumble::state::Status;
use c600_kernel::json::{self, Value};

use super::release::jam_state;
use crate::archive::s15::jam::{boolean, letters, parse_word, show, Letter};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, int, keys, not_applicable, reject, string, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{left_facts, phi, tree_tables, Jumbling};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s16.reductions",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s15/grips.rs", include_bytes!("../s15/grips.rs")),
        ("c600-checkers/src/archive/s15/jam.rs", include_bytes!("../s15/jam.rs")),
        ("c600-checkers/src/archive/s16/reductions.rs", include_bytes!("reductions.rs")),
        ("c600-checkers/src/archive/s16/release.rs", include_bytes!("release.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUB: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 16,
    family: "S16 transport, routing, T1, leaf/alphabet and P** run records: reduced transport against the recorded \
             unreduced case (no lemma); every negative keeps its scope",
};

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let doc = document(run.cert.bytes)?;
    if doc.get("format").is_some() {
        return Err(reject("format", "stage 16's transport, routing, T1, leaf and run records are formatless"));
    }
    if doc.get("K").is_some() {
        keys(&doc, "record", &TRANSPORT_K, &["cpu_s", "raw"])?;
        let seed = string(field(&doc, "seed", "seed")?, "seed")?;
        let k = int(field(&doc, "K", "K")?, "K")?;
        let minimal = boolean(field(&doc, "minimal", "minimal")?, "minimal")?;
        array(field(&doc, "best_per_target", "best_per_target")?, "best_per_target")?;
        let raw = match doc.get("raw") {
            None => false,
            Some(value) => boolean(value, "raw")?,
        };
        if raw {
            return Err(not_applicable(format!(
                "an unreduced transport enumeration (seed {seed}, K {k}): its completeness is not checked here; it is \
                 the input of the comparison it was recorded for"
            )));
        }
        if run.optional_input("unreduced").is_none() {
            return Err(not_applicable(format!(
                "a reduced transport search (seed {seed}, K {k}, minimal {minimal}) with no recorded unreduced \
                 counterpart: the reductions R1 to R3 rest on their proofs (Theorem X), and the agreement recorded at \
                 7307 with K = 2 is not a lemma for this input"
            )));
        }
        return agreement(engines, run, &doc);
    }
    Err(not_applicable(scope(&doc)?))
}

/// The fields of a transport record with K retained letters.
const TRANSPORT_K: [&str; 8] = ["seed", "extra_prefix", "digest", "phi", "K", "minimal", "counts", "best_per_target"];

/// A value as text.
fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.canonical(),
    }
}

/// The bounds of a record that is not a transport comparison. The record's form is told by one field and then
/// checked whole: every field of the form present, no other, and each bound it states of its kind. A record of no
/// known form fails at `record`.
fn scope(doc: &Value) -> Step<String> {
    let has = |key: &str| doc.get(key).is_some();
    let get = |key: &str| field(doc, key, key);
    let seed = || -> Step<String> {
        match get("seed")? {
            Value::String(seed) => Ok(seed.clone()),
            Value::Int(seed) => Ok(seed.to_string()),
            _ => Err(reject("seed", "a seed is a string or an integer")),
        }
    };
    let whole = |required: &[&str], optional: &[&str]| keys(doc, "record", required, optional);
    if has("leaves") {
        whole(
            &[
                "best_pair",
                "best_return",
                "complete",
                "digest",
                "emax",
                "extra",
                "grips",
                "hits",
                "leaves",
                "leaves_all",
                "n_hits",
                "per_leaf",
                "phi",
                "returns",
                "seed",
                "targets_tested",
            ],
            &["cpu_s", "leaves_done", "merged", "shard", "shard_cpu_s", "shards"],
        )?;
        let grips = get("grips")?;
        array(grips, "grips")?;
        Ok(format!(
            "a T1 leaf search at {} (leaf jumbles with a rise of at most {} at grips {}, complete {}): a bounded \
             negative for that leaf schema only, not a certificate in the sense of Proposition BN",
            seed()?,
            int(get("emax")?, "emax")?,
            text(grips),
            boolean(get("complete")?, "complete")?,
        ))
    } else if has("prizes") {
        whole(&["digest", "extra", "phi", "prizes", "results", "seed"], &["cpu_s"])?;
        array(get("results")?, "results")?;
        Ok(format!(
            "a routing release search with a prize at {} (options {}): a beam over its own alphabet, never complete; \
             its negatives decide nothing outside it",
            seed()?,
            text(get("extra")?),
        ))
    } else if has("segments") {
        whole(&["digest", "phi", "reason", "seed", "segments", "start", "status"], &["cpu_s", "start_macros"])?;
        Ok(format!(
            "a run of the driver P** at {} (status {}, reason {}): its stop is a stop of the declared families within \
             their bounds, not a negative for all words; its chosen descents are the PX certificates, which s16.macro \
             checks; no KN6 checker of the search's coverage exists (LB5.15)",
            seed()?,
            string(get("status")?, "status")?,
            string(get("reason")?, "reason")?,
        ))
    } else if has("seqs") {
        whole(
            &["B", "I_c", "S_r", "c", "digest", "mode", "newclean", "out_r", "r", "seed", "seqs"],
            &["best", "cap", "clearable", "cpu_s", "fixed", "free", "nmax", "releases_listed", "search", "target"],
        )?;
        Ok(format!(
            "a routing obligation scan at {} (carrier {}, released grip {}, mode {}): carrier sequences and the cover \
             test within its bounds, not a negative for all words",
            seed()?,
            int(get("c")?, "c")?,
            int(get("r")?, "r")?,
            text(get("mode")?),
        ))
    } else if has("shapes") {
        whole(
            &[
                "S_r", "a", "c", "complete", "counts", "digest", "early", "n_early", "r", "releases", "seed", "shapes",
                "shard",
            ],
            &["cpu_s"],
        )?;
        Ok(format!(
            "a two-carrier shard at {} (shard {}, complete {}): counts within its declared shapes only",
            seed()?,
            text(get("shard")?),
            boolean(get("complete")?, "complete")?,
        ))
    } else if has("closest") {
        whole(&["candidates", "closest", "counts", "e0", "first", "phi", "seed"], &["cpu_s", "least_excursion"])?;
        Ok(format!(
            "a one-helper scan at {} (helpers raising Φ by at most {}): a bounded negative for its two-letter words only",
            seed()?,
            int(get("e0")?, "e0")?,
        ))
    } else if has("depth") {
        whole(&["best_per_target", "clean", "counts", "depth", "digest", "phi", "seed"], &["cpu_s"])?;
        array(get("best_per_target")?, "best_per_target")?;
        let clean = get("clean")?;
        array(clean, "clean")?;
        Ok(format!(
            "a transport search at {} (depth {}, clean {}): no recorded unreduced counterpart, and its reductions are \
             not promoted to a lemma by any agreement recorded elsewhere",
            seed()?,
            int(get("depth")?, "depth")?,
            text(clean),
        ))
    } else {
        Err(reject(
            "record",
            "the record is none of stage 16's transport, routing, T1, leaf, two-carrier or run records",
        ))
    }
}

/// One best word of a transport record.
struct Best {
    word: Vec<Letter>,
    terminal_phi: i64,
    f: i64,
    d: i64,
}

fn best_per_target(doc: &Value, claim: &str) -> Step<BTreeMap<i64, (usize, Best)>> {
    let mut out = BTreeMap::new();
    for (k, row) in array(field(doc, "best_per_target", claim)?, &dot(claim, "best_per_target"))?.iter().enumerate() {
        let path = at(&dot(claim, "best_per_target"), k);
        keys(row, &path, &["word", "terminal_phi", "F", "D", "target"], &[])?;
        let target = int(field(row, "target", &path)?, &dot(&path, "target"))?;
        let best = Best {
            word: letters(field(row, "word", &path)?, &dot(&path, "word"))?,
            terminal_phi: int(field(row, "terminal_phi", &path)?, &dot(&path, "terminal_phi"))?,
            f: int(field(row, "F", &path)?, &dot(&path, "F"))?,
            d: int(field(row, "D", &path)?, &dot(&path, "D"))?,
        };
        if out.insert(target, (k, best)).is_some() {
            return Err(reject(&path, format!("target {target} is named twice")));
        }
    }
    Ok(out)
}

fn agreement(engines: &Engines, run: &mut Run, doc: &Value) -> Step<()> {
    let bytes = run.input("unreduced")?;
    let raw = json::parse(bytes).map_err(|error| reject("unreduced", format!("not JSON: {error}")))?;
    run.bind("unreduced", bytes, vec![]);
    keys(doc, "record", &TRANSPORT_K, &["cpu_s"])?;
    let mut raw_fields = TRANSPORT_K.to_vec();
    raw_fields.push("raw");
    keys(&raw, "unreduced", &raw_fields, &["cpu_s"])?;
    if !boolean(field(&raw, "raw", "unreduced")?, "unreduced.raw")? {
        return Err(reject("unreduced", "the input is not an unreduced enumeration"));
    }
    if boolean(field(&raw, "minimal", "unreduced")?, "unreduced.minimal")? {
        return Err(reject("unreduced", "the unreduced enumeration applies the minimality reduction"));
    }
    for key in ["seed", "extra_prefix", "digest", "phi", "K"] {
        let stated = field(doc, key, key)?.canonical();
        run.claim(key, &stated, &field(&raw, key, "unreduced")?.canonical())?;
    }
    let seed = string(field(doc, "seed", "seed")?, "seed")?.to_owned();
    let k = usize::try_from(int(field(doc, "K", "K")?, "K")?).map_err(|_| reject("K", "K is nonnegative"))?;
    let mut j = jam_state(engines, run, &seed, "seed")?;
    let extra = parse_word(string(field(doc, "extra_prefix", "extra_prefix")?, "extra_prefix")?, "extra_prefix")?;
    for (n, &(cap, item)) in extra.iter().enumerate() {
        let claim = at("extra_prefix", n);
        let twist = j.twist(cap, item, &claim)?;
        if j.apply(&twist, &claim)?.status != Status::Admissible {
            return Err(run.failed(reject(&claim, "the letter is not admissible"), "refused"));
        }
    }
    let ctx = j.ctx;
    let tables = tree_tables(ctx, "phi")?;
    let start_phi = phi(ctx, tables, &j.state, "phi")?;
    run.claim("digest", &hex64(field(doc, "digest", "digest")?, "digest")?.to_owned(), &j.digest())?;
    run.claim("phi", &int(field(doc, "phi", "phi")?, "phi")?, &start_phi)?;

    let reduced = best_per_target(doc, "")?;
    let unreduced = best_per_target(&raw, "unreduced")?;
    let named = |m: &BTreeMap<i64, (usize, Best)>| m.keys().map(|t| t.to_string()).collect::<Vec<_>>().join(",");
    run.claim("best_per_target", &named(&reduced), &named(&unreduced))?;
    for (target, (index, best)) in &reduced {
        let path = at("best_per_target", *index);
        let other = &unreduced[target].1;
        run.claim(&dot(&path, "word"), &show(&best.word), &show(&other.word))?;
        run.claim(&dot(&path, "terminal_phi"), &best.terminal_phi, &other.terminal_phi)?;
        run.claim(&dot(&path, "F"), &best.f, &other.f)?;
        run.claim(&dot(&path, "D"), &best.d, &other.d)?;
        let (f, d, terminal) = replay_best(&j, &best.word, *target, k, &path)?;
        run.claim(&dot(&path, "F"), &best.f, &f)?;
        run.claim(&dot(&path, "D"), &best.d, &d)?;
        run.claim(&dot(&path, "terminal_phi"), &best.terminal_phi, &terminal)?;
    }
    run.held(
        "agreement",
        "the reduced and the unreduced records give the same best word per target at this input; no lemma",
    );
    Ok(())
}

/// Replays a best word from the start: `k` retained letters, then a jumble at the target. Returns F and D at the
/// lowering (read before it) and the terminal Φ.
fn replay_best(start: &Jumbling, word: &[Letter], target: i64, k: usize, path: &str) -> Step<(i64, i64, i64)> {
    let claim = dot(path, "word");
    if word.len() != k + 1 || word.last().map(|&(c, _)| c as i64) != Some(target) {
        return Err(reject(&claim, format!("the word is not {k} letters followed by a lowering at the target")));
    }
    let mut j = Jumbling::new(start.ctx, start.menu.clone(), start.state.clone());
    let ctx = j.ctx;
    let tables = tree_tables(ctx, &claim)?;
    for (n, &(cap, item)) in word.iter().enumerate() {
        let letter = at(&claim, n);
        let twist = j.twist(cap, item, &letter)?;
        let last = n == word.len() - 1;
        if twist.kidx.is_some() == last {
            return Err(reject(&letter, if last { "the last letter is not a jumble" } else { "a letter is a jumble" }));
        }
        let before = last.then(|| (j.state.pose_ids().to_vec(), j.state.poses().to_vec()));
        let outcome = j.apply(&twist, &letter)?;
        if outcome.status != Status::Admissible {
            return Err(reject(&letter, format!("the letter is {}", outcome.status.as_str())));
        }
        if let Some((ids, poses)) = before {
            let cls = tables.cls(ctx, cap).map_err(|error| reject(&letter, format!("the cap's class: {error}")))?;
            let (mut f, mut d) = (0i64, 0i64);
            for &p in &outcome.inside {
                let (h, b) = left_facts(ctx, tables, &poses[ids[p as usize] as usize], &letter)?;
                if h >= 2 && b != cls {
                    f += 1;
                }
                if h >= 4 && b == cls && left_facts(ctx, tables, j.state.pose_of(p as usize), &letter)?.0 == h - 2 {
                    d += 1;
                }
            }
            let terminal = phi(ctx, tables, &j.state, &letter)?;
            return Ok((f, d, terminal));
        }
    }
    Err(reject(&claim, "the word is empty"))
}
