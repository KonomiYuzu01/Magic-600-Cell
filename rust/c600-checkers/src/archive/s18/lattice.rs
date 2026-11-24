//! Stage 18's lattice run record, `runner-run/3` (stage 18 `results/lattice-d1/run/run.json`), shared by
//! `s18.lattice_run` and `s18.lattice_replay`.
//!
//! The whole record is read in its exact form before any input is read ([`read_run`]); a field missing, an unknown
//! field, a list of the wrong length or a value of the wrong kind fails, naming the field. Its claims:
//!
//! - the format is `runner-run/3`, the menu I_a, the result `solved`, with no error, no stop and no lattice failure;
//!   the acceptance verdict is `solved` with no certificate, fallback or timed-out step and no probe;
//! - the start is a stored table (its file and digest, nothing off);
//! - every step is a lattice step played literally (`phase` lattice, no subphase, contract `lattice_literal`, gate
//!   `player`, encoding `ia16`, standing rule `rule`, no attempt) with its index, its own word file and declared length
//!   admitted at most 10⁷ before anything is decoded, its hash and its own tree file; the steps chain from the start,
//!   each before point the previous after point, with no piece off before or after any step; a step is an orbit step
//!   (`lattice/orbit-o`, its factors counted from the first, no folded tree) or a correction step
//!   (`lattice/correction-c-m`, `times` copies of a macro of `macro_letters` letters);
//! - the bulk phase ended on the lattice at the start table; the height, high and tail phases (and the height phase's
//!   sub-phases, exactly those the configuration orders) did not apply and played nothing; the lattice phase's step
//!   and twist counts and the totals' lattice twists are the steps' sums, and the totals' other twists are 0.
//!
//! Not claims, so not read: the producer's configuration (apart from the sub-phase order), the timings (`seconds`)
//! and the labels flag (`labels_identity`, a boolean).
//!
//! A point `{digest, off, heights}` gives a table digest, the off count and the off pieces' heights; with nothing off
//! the heights are empty.

use c600_kernel::json::Value;

use super::record::{parse_record, RUN_FORMAT};
use super::words::admit;
use crate::claims::{array, at, count, dot, field, hex64, keys, object, reject, string, Run, Step};

/// A point of the run: a table digest and its off count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub digest: String,
    pub off: u64,
}

/// The kind of a lattice step.
#[derive(Clone, Debug)]
pub enum Kind {
    Orbit { orbit: u64, factors: u64 },
    Correction { correction: u64, name: String, times: u64, macro_letters: u64 },
}

/// A step's tree reference: its file, the SHA-256 of its decoded JSON and its node count.
#[derive(Clone, Debug)]
pub struct TreeRef {
    pub file: String,
    pub sha: String,
    pub size: u64,
}

/// One lattice step as the record states it.
#[derive(Clone, Debug)]
pub struct LatticeStep {
    /// The claim path, `steps[k]`.
    pub path: String,
    pub word: String,
    pub letters: u64,
    pub sha: String,
    pub before: Point,
    pub after: Point,
    /// A table stored at the step's end, by file.
    pub table: Option<String>,
    pub kind: Kind,
    pub tree: TreeRef,
}

/// The run as the record states it.
#[derive(Clone, Debug)]
pub struct RunRecord {
    pub start_file: String,
    pub start: Point,
    pub steps: Vec<LatticeStep>,
    pub twists: u64,
    /// The bulk phase's end table, by file.
    pub bulk_table: String,
    /// The lattice phase's report, by file, and its SHA-256.
    pub report_file: String,
    pub report_sha: String,
}

const ROOT: [&str; 12] = [
    "format",
    "menu",
    "start",
    "config",
    "phases",
    "steps",
    "stuck",
    "lattice",
    "result",
    "error",
    "acceptance",
    "totals",
];
const BASE: [&str; 16] = [
    "index",
    "phase",
    "subphase",
    "rule",
    "word",
    "letters",
    "encoding",
    "word_sha256",
    "tree",
    "contract",
    "gate",
    "standing",
    "before",
    "after",
    "attempts",
    "seconds",
];
const ORBIT: [&str; 5] = ["orbit", "factor_first", "factor_count", "tree_folded", "tree_folded_depth"];
const CORRECTION: [&str; 6] = ["correction", "macro", "family", "times", "inverse", "macro_letters"];

/// The certificate as a lattice run record. A record the plan leaves unchecked is `not_applicable` with its scope;
/// anything else that is not `runner-run/3` fails, naming the field.
pub fn open_run(run: &Run, scope: &str) -> Step<Value> {
    let doc = parse_record(run.cert.bytes, scope)?;
    match doc.get("format") {
        Some(Value::String(format)) if format == RUN_FORMAT => Ok(doc),
        Some(_) => Err(reject("format", format!("this checker reads {RUN_FORMAT} lattice run records"))),
        None => Err(reject("format", format!("the field format is missing; this checker reads {RUN_FORMAT} records"))),
    }
}

/// A point `{digest, off, heights}` (and `table` where `with` names it).
pub fn read_point(value: &Value, claim: &str, with: &[&str]) -> Step<Point> {
    keys(value, claim, &["digest", "off", "heights"], with)?;
    let digest = hex64(field(value, "digest", claim)?, &dot(claim, "digest"))?.to_owned();
    let off = count(field(value, "off", claim)?, &dot(claim, "off"))?;
    let heights = object(field(value, "heights", claim)?, &dot(claim, "heights"))?;
    if off == 0 && !heights.is_empty() {
        return Err(reject(dot(claim, "heights"), "a point with nothing off has no heights"));
    }
    Ok(Point { digest, off })
}

/// A string field equal to `wanted`.
pub fn word_is(run: &mut Run, value: &Value, key: &str, claim: &str, wanted: &str) -> Step<()> {
    let path = dot(claim, key);
    let found = string(field(value, key, claim)?, &path)?.to_owned();
    run.claim(&path, &wanted.to_owned(), &found)
}

/// A field that must be null.
pub fn null(value: &Value, key: &str, claim: &str) -> Step<()> {
    match field(value, key, claim)? {
        Value::Null => Ok(()),
        _ => Err(reject(dot(claim, key), "expected null")),
    }
}

/// A count field that must be 0.
fn zero(run: &mut Run, value: &Value, key: &str, claim: &str) -> Step<()> {
    let path = dot(claim, key);
    run.claim(&path, &count(field(value, key, claim)?, &path)?, &0u64)
}

/// A field that must be a boolean.
fn flag(value: &Value, key: &str, claim: &str) -> Step<bool> {
    match field(value, key, claim)? {
        Value::Bool(b) => Ok(*b),
        _ => Err(reject(dot(claim, key), "expected true or false")),
    }
}

/// Reads a lattice run record in its exact form and checks its own claims. No input is read.
pub fn read_run(run: &mut Run, doc: &Value) -> Step<RunRecord> {
    keys(doc, "certificate", &ROOT, &[])?;
    word_is(run, doc, "format", "", RUN_FORMAT)?;
    word_is(run, doc, "menu", "", "I_a")?;
    word_is(run, doc, "result", "", "solved")?;
    for key in ["error", "stuck", "lattice"] {
        null(doc, key, "")?;
    }
    object(field(doc, "config", "certificate")?, "config")?;
    let acceptance = field(doc, "acceptance", "certificate")?;
    keys(
        acceptance,
        "acceptance",
        &["verdict", "certificate_steps", "fallback_steps", "probe", "timed_out_attempts"],
        &[],
    )?;
    word_is(run, acceptance, "verdict", "acceptance", "solved")?;
    for key in ["certificate_steps", "fallback_steps", "timed_out_attempts"] {
        zero(run, acceptance, key, "acceptance")?;
    }
    null(acceptance, "probe", "acceptance")?;

    let start = field(doc, "start", "certificate")?;
    keys(start, "start", &["kind", "file", "digest", "off", "heights"], &[])?;
    word_is(run, start, "kind", "start", "table")?;
    let start_file = string(field(start, "file", "start")?, "start.file")?.to_owned();
    let start_point = read_point(start, "start", &["kind", "file"])?;
    if start_point.off != 0 {
        return Err(reject("start.off", "the lattice phase starts with every piece on the lattice"));
    }

    let list = array(field(doc, "steps", "certificate")?, "steps")?;
    if list.is_empty() {
        return Err(reject("steps", "the lattice phase has no step"));
    }
    let mut steps: Vec<LatticeStep> = Vec::new();
    let mut previous = start_point.clone();
    let (mut twists, mut factor_first) = (0u64, 0u64);
    for (k, value) in list.iter().enumerate() {
        let step = read_step(run, value, &at("steps", k), k, &previous, &mut factor_first)?;
        if steps.iter().any(|s| s.word == step.word) {
            return Err(reject(dot(&step.path, "word"), "each step names its own ia16 word file"));
        }
        if steps.iter().any(|s| s.tree.file == step.tree.file) {
            return Err(reject(dot(&step.path, "tree.file"), "each step names its own tree file"));
        }
        twists = twists.checked_add(step.letters).ok_or_else(|| reject("totals", "too many twists"))?;
        previous = step.after.clone();
        steps.push(step);
    }
    run.held("steps.admission", format!("{} words, each at most 10^7 letters, before decoding", steps.len()));

    let (bulk_table, report_file, report_sha) = phases(run, doc, &start_point, steps.len() as u64, twists)?;
    let totals = field(doc, "totals", "certificate")?;
    keys(
        totals,
        "totals",
        &["bulk_twists", "height_twists", "high_twists", "tail_twists", "fallback_twists", "lattice_twists", "seconds"],
        &[],
    )?;
    for key in ["bulk_twists", "height_twists", "high_twists", "tail_twists", "fallback_twists"] {
        zero(run, totals, key, "totals")?;
    }
    let stated = count(field(totals, "lattice_twists", "totals")?, "totals.lattice_twists")?;
    run.claim("totals.lattice_twists", &stated, &twists)?;
    Ok(RunRecord { start_file, start: start_point, steps, twists, bulk_table, report_file, report_sha })
}

/// One step in its exact form, chained to the point before it.
fn read_step(
    run: &mut Run,
    value: &Value,
    path: &str,
    k: usize,
    previous: &Point,
    factor_first: &mut u64,
) -> Step<LatticeStep> {
    let orbit_step = object(value, path)?.contains_key("orbit");
    let own: &[&str] = if orbit_step { &ORBIT } else { &CORRECTION };
    keys(value, path, &[&BASE[..], own].concat(), &["labels_identity", "table"])?;
    let name = |key: &str| dot(path, key);
    let number = |key: &str| -> Step<u64> { count(field(value, key, path)?, &name(key)) };
    run.claim(&name("index"), &(k as u64), &number("index")?)?;
    word_is(run, value, "phase", path, "lattice")?;
    null(value, "subphase", path)?;
    word_is(run, value, "contract", path, "lattice_literal")?;
    word_is(run, value, "gate", path, "player")?;
    word_is(run, value, "encoding", path, "ia16")?;
    word_is(run, value, "standing", path, "rule")?;
    if !array(field(value, "attempts", path)?, &name("attempts"))?.is_empty() {
        return Err(reject(name("attempts"), "a lattice step of this run made no attempt"));
    }
    let word = string(field(value, "word", path)?, &name("word"))?.to_owned();
    if !(word.ends_with(".ia16") || word.ends_with(".ia16.gz")) {
        return Err(reject(name("word"), "each step names its own ia16 word file"));
    }
    let letters = number("letters")?;
    admit(letters, &name("letters"))?;
    let sha = hex64(field(value, "word_sha256", path)?, &name("word_sha256"))?.to_owned();
    let tree = field(value, "tree", path)?;
    let tree_path = name("tree");
    keys(tree, &tree_path, &["file", "sha256", "size"], &[])?;
    let tree = TreeRef {
        file: string(field(tree, "file", &tree_path)?, &dot(&tree_path, "file"))?.to_owned(),
        sha: hex64(field(tree, "sha256", &tree_path)?, &dot(&tree_path, "sha256"))?.to_owned(),
        size: count(field(tree, "size", &tree_path)?, &dot(&tree_path, "size"))?,
    };
    let before = read_point(field(value, "before", path)?, &name("before"), &[])?;
    let after = read_point(field(value, "after", path)?, &name("after"), &[])?;
    run.claim(&name("before.digest"), &before.digest, &previous.digest)?;
    for (point, key) in [(&before, "before.off"), (&after, "after.off")] {
        run.claim(&name(key), &point.off, &0u64)?;
    }
    let table = match value.get("table") {
        Some(t) => Some(string(t, &name("table"))?.to_owned()),
        None => None,
    };
    if value.get("labels_identity").is_some() {
        flag(value, "labels_identity", path)?;
    }
    let rule = string(field(value, "rule", path)?, &name("rule"))?.to_owned();
    let kind = if orbit_step {
        let orbit = number("orbit")?;
        run.claim(&name("rule"), &format!("lattice/orbit-{orbit}"), &rule)?;
        run.claim(&name("factor_first"), &number("factor_first")?, factor_first)?;
        let factors = number("factor_count")?;
        *factor_first = factor_first.checked_add(factors).ok_or_else(|| reject(name("factor_count"), "too many"))?;
        for key in ["tree_folded", "tree_folded_depth"] {
            zero(run, value, key, path)?;
        }
        Kind::Orbit { orbit, factors }
    } else {
        let (correction, macro_) = (number("correction")?, number("macro")?);
        run.claim(&name("rule"), &format!("lattice/correction-{correction}-{macro_}"), &rule)?;
        let (times, macro_letters) = (number("times")?, number("macro_letters")?);
        run.claim(&name("letters"), &letters, &times.saturating_mul(macro_letters))?;
        let family = string(field(value, "family", path)?, &name("family"))?;
        let suffix = if flag(value, "inverse", path)? { "⁻¹" } else { "" };
        Kind::Correction { correction, name: format!("macro {macro_} {family}{suffix}"), times, macro_letters }
    };
    Ok(LatticeStep { path: path.to_owned(), word, letters, sha, before, after, table, kind, tree })
}

/// The phases: the bulk phase ended at the start, the height, high and tail phases did not apply, the lattice phase's
/// counts are the steps'. Returns the bulk phase's end table and the lattice phase's report file and hash.
fn phases(run: &mut Run, doc: &Value, start: &Point, steps: u64, twists: u64) -> Step<(String, String, String)> {
    let phases = field(doc, "phases", "certificate")?;
    keys(phases, "phases", &["bulk", "height", "high", "tail", "lattice"], &[])?;
    let bulk = field(phases, "bulk", "phases")?;
    keys(bulk, "phases.bulk", &["result", "completed", "steps", "twists", "end"], &[])?;
    word_is(run, bulk, "result", "phases.bulk", "lattice")?;
    if !flag(bulk, "completed", "phases.bulk")? {
        return Err(reject("phases.bulk.completed", "the bulk phase completed"));
    }
    for key in ["steps", "twists"] {
        zero(run, bulk, key, "phases.bulk")?;
    }
    let end = field(bulk, "end", "phases.bulk")?;
    let point = read_point(end, "phases.bulk.end", &["table"])?;
    run.claim("phases.bulk.end.digest", &start.digest, &point.digest)?;
    run.claim("phases.bulk.end.off", &start.off, &point.off)?;
    let bulk_table = string(field(end, "table", "phases.bulk.end")?, "phases.bulk.end.table")?.to_owned();

    // The height phase's sub-phases are exactly those the configuration orders.
    let height = field(phases, "height", "phases")?;
    let sub = object(field(height, "sub", "phases.height")?, "phases.height.sub")?;
    let config = field(doc, "config", "certificate")?;
    let order = field(field(config, "subphase_order", "config")?, "height", "config.subphase_order")?;
    let mut ordered: Vec<String> = array(order, "config.subphase_order.height")?
        .iter()
        .map(|v| string(v, "config.subphase_order.height").map(str::to_owned))
        .collect::<Step<_>>()?;
    ordered.sort();
    let present: Vec<String> = sub.keys().cloned().collect();
    run.claim("phases.height.sub", &ordered.join(", "), &present.join(", "))?;
    let mut skipped: Vec<(String, &Value)> =
        vec![("phases.height".to_owned(), height), ("phases.high".to_owned(), field(phases, "high", "phases")?)];
    for (name, value) in sub {
        skipped.push((format!("phases.height.sub.{name}"), value));
    }
    for (path, value) in skipped {
        let optional: &[&str] = if path == "phases.height" { &["sub"] } else { &[] };
        keys(value, &path, &["result", "steps", "twists", "start", "end"], optional)?;
        word_is(run, value, "result", &path, "not_applicable")?;
        for key in ["steps", "twists"] {
            zero(run, value, key, &path)?;
        }
        for key in ["start", "end"] {
            let point = read_point(field(value, key, &path)?, &dot(&path, key), &[])?;
            run.claim(&dot(&dot(&path, key), "digest"), &start.digest, &point.digest)?;
            run.claim(&dot(&dot(&path, key), "off"), &start.off, &point.off)?;
        }
    }
    let tail = field(phases, "tail", "phases")?;
    keys(tail, "phases.tail", &["result", "steps", "twists"], &[])?;
    word_is(run, tail, "result", "phases.tail", "not_applicable")?;
    for key in ["steps", "twists"] {
        zero(run, tail, key, "phases.tail")?;
    }

    let lattice = field(phases, "lattice", "phases")?;
    keys(lattice, "phases.lattice", &["result", "steps", "twists", "report"], &[])?;
    word_is(run, lattice, "result", "phases.lattice", "solved")?;
    let stated = count(field(lattice, "steps", "phases.lattice")?, "phases.lattice.steps")?;
    run.claim("phases.lattice.steps", &stated, &steps)?;
    let stated = count(field(lattice, "twists", "phases.lattice")?, "phases.lattice.twists")?;
    run.claim("phases.lattice.twists", &stated, &twists)?;
    let report = field(lattice, "report", "phases.lattice")?;
    keys(report, "phases.lattice.report", &["file", "sha256"], &[])?;
    let report_file = string(field(report, "file", "phases.lattice.report")?, "phases.lattice.report.file")?.to_owned();
    let report_sha =
        hex64(field(report, "sha256", "phases.lattice.report")?, "phases.lattice.report.sha256")?.to_owned();
    Ok((bulk_table, report_file, report_sha))
}

/// The roles of a run's word files.
pub fn word_roles(record: &RunRecord) -> impl Iterator<Item = &str> {
    record.steps.iter().map(|s| s.word.as_str())
}

/// Checks that every supplied input is one of `named`.
pub fn only(run: &Run, named: &[&str]) -> Step<()> {
    for supplied in run.cert.inputs {
        if !named.contains(&supplied.role) {
            return Err(reject("inputs", format!("the input {} is not named by the record", supplied.role)));
        }
    }
    Ok(())
}
