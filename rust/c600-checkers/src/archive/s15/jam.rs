//! Shared by the sub-checkers of stages 15 to 17: the jam catalogue, the chain binding of a `macro-certificate/1` to
//! it, and the matching of a `macro-check/1` record with a fresh check of its certificate. No sub-checker is
//! registered here; each one that runs this file names it among its sources.
//!
//! # The jam catalogue
//!
//! Stage 16's `jams.json` names, per seed 7302 to 7307, the D2 end table (`table`, `table_digest`, the state digest of
//! the table), the chain word of stage 14's T3P rounds (`chain`, which ends at stage 15's chain end), the macros of the
//! declared and two-level runs of P* (`macros`, stage 15's whole descent) and the configuration they end at (`end_digest`,
//! `end_phi`, the jam). A certificate names its table by `table_digest`, so the catalogue row is found from it, and a
//! table that is the start table of no row is the wrong start table.
//!
//! - **Stage 15** certificates start at the chain end: the prefix is the row's chain word. A certificate whose macros
//!   are the row's macros is the row's whole descent and must end at the jam: its last digest and Φ are `end_digest`
//!   and `end_phi`.
//! - **Stages 16 and 17** certificates start at the jam: the prefix is the chain word followed by the macros, and the
//!   stated start is `end_digest` and `end_phi`.
//!
//! Every certificate of the stages states at least one macro; one with none fails at `macros`. The macro kind
//! ([`crate::kinds::macros`]) then replays the prefix letter by letter from the supplied table and
//! recomputes every claim, so the catalogue's chain, macros, end digest and end Φ are recomputed from the exact table
//! by every certificate that names them.
//!
//! # Check records
//!
//! A `macro-check/1` record is the producer's account of a check of one certificate, supplied as the input
//! `certificate`. The certificate is checked afresh under the same sub-checker (its trace steps are named
//! `certificate.<claim>`), and then every claim of the record is compared with that check: `passed` and each macro's
//! `passed` are true, `failures` is empty, each macro's `claims` are the certificate's (now recomputed), `digest` and
//! `phi` are the end of the certificate's last macro, and `replay` states the same end and the number of replayed
//! letters (prefix and events). `cpu_s` and `setup` are measurements, not claims.

use c600_kernel::json::{self, Value};

use crate::claims::{
    array, at, dot, field, hex64, int, keys, not_applicable, object, reject, start_mismatch, string, Run, Step, Stop,
};
use crate::kinds::macros;
use crate::report::CertificateRef;
use crate::Engines;

/// The check-record format of stage 15's `macro.py check`.
pub const CHECK_FORMAT: &str = "macro-check/1";

/// A letter `(cap, item)`.
pub type Letter = (usize, usize);

/// Where the certificates of a stage start in their jam's catalogue row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    /// Stage 15: after the chain word.
    ChainEnd,
    /// Stages 16 and 17: after the chain word and the macros.
    JamEnd,
}

/// One row of the jam catalogue.
#[derive(Clone, Debug)]
pub struct Jam {
    pub seed: String,
    pub table_digest: String,
    pub chain: Vec<Letter>,
    pub macros: Vec<Vec<Letter>>,
    pub end_digest: String,
    pub end_phi: i64,
}

impl Jam {
    /// The jam word: the chain word followed by the macros.
    pub fn word(&self) -> Vec<Letter> {
        self.chain.iter().chain(self.macros.iter().flatten()).copied().collect()
    }
}

/// A word written `cap:item,cap:item,...` (the empty string is the empty word).
pub fn parse_word(text: &str, claim: &str) -> Step<Vec<Letter>> {
    if text.is_empty() {
        return Ok(Vec::new());
    }
    text.split(',')
        .map(|letter| {
            let parsed = letter.split_once(':').and_then(|(c, i)| Some((c.parse().ok()?, i.parse().ok()?)));
            parsed.ok_or_else(|| reject(claim, format!("{letter:?} is not a letter cap:item")))
        })
        .collect()
}

/// A word written as a list `[[cap, item], ...]`; the menu check comes when a letter is replayed.
pub fn letters(value: &Value, claim: &str) -> Step<Vec<Letter>> {
    array(value, claim)?
        .iter()
        .enumerate()
        .map(|(k, pair)| {
            let path = at(claim, k);
            let pair = array(pair, &path)?;
            match pair {
                [c, i] => {
                    let c = usize::try_from(int(c, &path)?).map_err(|_| reject(&path, "a cap is nonnegative"))?;
                    let i = usize::try_from(int(i, &path)?).map_err(|_| reject(&path, "an item is nonnegative"))?;
                    Ok((c, i))
                }
                _ => Err(reject(&path, "a letter is [cap, item]")),
            }
        })
        .collect()
}

/// Shows a word as `cap:item,...`.
pub fn show(word: &[Letter]) -> String {
    word.iter().map(|(c, i)| format!("{c}:{i}")).collect::<Vec<_>>().join(",")
}

/// A boolean.
pub fn boolean(value: &Value, claim: &str) -> Step<bool> {
    match value {
        Value::Bool(b) => Ok(*b),
        _ => Err(reject(claim, "expected true or false")),
    }
}

/// The jam catalogue of input `jams`, bound with its seeds.
pub fn read_jams(run: &mut Run) -> Step<Vec<Jam>> {
    let bytes = run.input("jams")?;
    let doc = json::parse(bytes).map_err(|error| reject("jams", format!("the catalogue is not JSON: {error}")))?;
    let mut jams = Vec::new();
    for (seed, row) in object(&doc, "jams")? {
        let path = dot("jams", seed);
        keys(row, &path, &["table", "table_digest", "chain", "macros", "end_digest", "end_phi"], &[])?;
        string(field(row, "table", &path)?, &dot(&path, "table"))?;
        let macros = array(field(row, "macros", &path)?, &dot(&path, "macros"))?
            .iter()
            .enumerate()
            .map(|(m, w)| {
                let claim = at(&dot(&path, "macros"), m);
                parse_word(string(w, &claim)?, &claim)
            })
            .collect::<Step<Vec<_>>>()?;
        jams.push(Jam {
            seed: seed.clone(),
            table_digest: hex64(field(row, "table_digest", &path)?, &dot(&path, "table_digest"))?.to_owned(),
            chain: parse_word(string(field(row, "chain", &path)?, &dot(&path, "chain"))?, &dot(&path, "chain"))?,
            macros,
            end_digest: hex64(field(row, "end_digest", &path)?, &dot(&path, "end_digest"))?.to_owned(),
            end_phi: int(field(row, "end_phi", &path)?, &dot(&path, "end_phi"))?,
        });
    }
    let seeds = jams.iter().map(|j| j.seed.as_str()).collect::<Vec<_>>().join(",");
    run.bind("jams", bytes, vec![("seeds", seeds)]);
    Ok(jams)
}

/// The catalogue row of `seed`.
pub fn jam_of_seed<'j>(jams: &'j [Jam], seed: &str, claim: &str) -> Step<&'j Jam> {
    jams.iter().find(|j| j.seed == seed).ok_or_else(|| reject(claim, format!("the catalogue has no jam {seed}")))
}

/// The family of `macro-certificate/1` certificates and `macro-check/1` records of one stage.
pub fn run_family(engines: &Engines, run: &mut Run, start: Start) -> Step<()> {
    let doc = crate::claims::document(run.cert.bytes)?;
    match doc.get("format").and_then(Value::as_str) {
        Some(macros::FORMAT) => certificate(engines, run, &doc, start),
        Some(CHECK_FORMAT) => check_record(engines, run, &doc, start),
        Some(other) => Err(reject(
            "format",
            format!("the record's format is {other}; this family reads {} and {CHECK_FORMAT}", macros::FORMAT),
        )),
        None => Err(reject(
            "format",
            format!("the record states no format; this family reads {} and {CHECK_FORMAT}", macros::FORMAT),
        )),
    }
}

/// A certificate: the chain binding, then the macro kind, then the end of a whole descent.
fn certificate(engines: &Engines, run: &mut Run, doc: &Value, start: Start) -> Step<()> {
    let jams = read_jams(run)?;
    let stated = hex64(field(doc, "table_digest", "table_digest")?, "table_digest")?;
    let jam = jams.iter().find(|j| j.table_digest == stated).ok_or_else(|| {
        start_mismatch("table_digest", "the certificate's table is the start table of no jam of the catalogue")
    })?;
    run.held("jam", &jam.seed);
    let prefix = letters(field(doc, "prefix", "prefix")?, "prefix")?;
    let (expected, what) = match start {
        Start::ChainEnd => (jam.chain.clone(), "chain word"),
        Start::JamEnd => (jam.word(), "jam word"),
    };
    if prefix != expected {
        let stop = reject("prefix", format!("the prefix is not the {what} of jam {}", jam.seed));
        return Err(run.failed(stop, show(&prefix)));
    }
    run.held("prefix", format!("the {what} of jam {}", jam.seed));
    if array(field(doc, "macros", "macros")?, "macros")?.is_empty() {
        return Err(reject("macros", "the certificate states no macro; every certificate of the stages states one"));
    }
    if start == Start::JamEnd {
        let begin = field(doc, "start", "start")?;
        let digest = hex64(field(begin, "digest", "start")?, "start.digest")?.to_owned();
        run.claim("start.digest", &digest, &jam.end_digest)?;
        run.claim("start.phi", &int(field(begin, "phi", "start")?, "start.phi")?, &jam.end_phi)?;
    }
    (macros::CHECKER.run)(engines, run)?;
    if start == Start::ChainEnd {
        let events = array(field(doc, "macros", "macros")?, "macros")?
            .iter()
            .enumerate()
            .map(|(m, spec)| letters(field(spec, "events", &at("macros", m))?, &dot(&at("macros", m), "events")))
            .collect::<Step<Vec<_>>>()?;
        if events == jam.macros {
            // The jam's whole descent: its end, recomputed by the macro kind, is the catalogue's.
            let (digest, phi) = end_of(doc, "certificate")?;
            run.claim("jam.end_digest", &jam.end_digest, &digest)?;
            run.claim("jam.end_phi", &jam.end_phi, &phi)?;
        }
    }
    Ok(())
}

/// The end of a certificate whose claims have been checked: the last digest and Φ of its last macro, or its start.
fn end_of(doc: &Value, claim: &str) -> Step<(String, i64)> {
    let macros = array(field(doc, "macros", claim)?, claim)?;
    match macros.last() {
        None => {
            let begin = field(doc, "start", claim)?;
            Ok((hex64(field(begin, "digest", claim)?, claim)?.to_owned(), int(field(begin, "phi", claim)?, claim)?))
        }
        Some(last) => {
            let claims = field(last, "claims", claim)?;
            let digests = array(field(claims, "digests", claim)?, claim)?;
            let phis = array(field(claims, "phi", claim)?, claim)?;
            match (digests.last(), phis.last()) {
                (Some(d), Some(p)) => Ok((hex64(d, claim)?.to_owned(), int(p, claim)?)),
                _ => Err(reject(claim, "a macro has no events")),
            }
        }
    }
}

/// A check record: its certificate checked afresh under the same family, then every claim of the record.
fn check_record(engines: &Engines, run: &mut Run, doc: &Value, start: Start) -> Step<()> {
    keys(doc, "record", &["format", "digest", "phi", "passed", "failures", "macros", "replay"], &["cpu_s", "setup"])?;
    let bytes = run.input("certificate")?;
    let cert = json::parse(bytes).map_err(|error| reject("certificate", format!("not a JSON document: {error}")))?;
    if cert.get("format").and_then(Value::as_str) != Some(macros::FORMAT) {
        return Err(reject("certificate", format!("the record's certificate is not a {}", macros::FORMAT)));
    }
    let digest = c600_digest::sha256_hex(bytes);
    run.bind("certificate", bytes, vec![]);
    let inner = CertificateRef { digest: &digest, bytes, inputs: run.cert.inputs };
    let mut sub = Run::new(&inner);
    let result = certificate(engines, &mut sub, &cert, start);
    let Run { trace, inputs, .. } = sub;
    for mut step in trace {
        step.claim = dot("certificate", &step.claim);
        run.trace.push(step);
    }
    run.inputs.extend(inputs);
    match result {
        Ok(()) => {}
        Err(Stop::Fail { claim, code, reason }) => {
            return Err(Stop::Fail { claim: dot("certificate", &claim), code, reason })
        }
        Err(Stop::NotApplicable { reason }) => return Err(not_applicable(reason)),
    }

    run.claim("passed", &boolean(field(doc, "passed", "record")?, "passed")?, &true)?;
    let failures = array(field(doc, "failures", "record")?, "failures")?.len();
    run.claim("failures", &failures, &0)?;
    let stated = array(field(doc, "macros", "record")?, "macros")?;
    let checked = array(field(&cert, "macros", "certificate")?, "certificate.macros")?;
    run.claim("macros", &stated.len(), &checked.len())?;
    for (m, (row, spec)) in stated.iter().zip(checked).enumerate() {
        let path = at("macros", m);
        keys(row, &path, &["claims", "passed"], &[])?;
        run.claim(&dot(&path, "passed"), &boolean(field(row, "passed", &path)?, &dot(&path, "passed"))?, &true)?;
        compare_claims(run, &dot(&path, "claims"), field(row, "claims", &path)?, field(spec, "claims", &path)?)?;
    }
    let (end, phi) = end_of(&cert, "certificate")?;
    run.claim("digest", &hex64(field(doc, "digest", "record")?, "digest")?.to_owned(), &end)?;
    run.claim("phi", &int(field(doc, "phi", "record")?, "phi")?, &phi)?;
    let replay = field(doc, "replay", "record")?;
    keys(replay, "replay", &["digest", "events", "passed", "phi"], &[])?;
    let letters_replayed = array(field(&cert, "prefix", "certificate")?, "certificate.prefix")?.len()
        + checked
            .iter()
            .map(|spec| field(spec, "events", "certificate").and_then(|e| array(e, "certificate").map(<[_]>::len)))
            .sum::<Step<usize>>()?;
    run.claim("replay.digest", &hex64(field(replay, "digest", "replay")?, "replay.digest")?.to_owned(), &end)?;
    run.claim("replay.phi", &int(field(replay, "phi", "replay")?, "replay.phi")?, &phi)?;
    run.claim("replay.events", &int(field(replay, "events", "replay")?, "replay.events")?, &(letters_replayed as i64))?;
    run.claim("replay.passed", &boolean(field(replay, "passed", "replay")?, "replay.passed")?, &true)?;
    Ok(())
}

/// The record's claims of one macro, leaf by leaf against the certificate's checked claims.
fn compare_claims(run: &mut Run, path: &str, stated: &Value, checked: &Value) -> Step<()> {
    match (stated, checked) {
        (Value::Object(a), Value::Object(b)) => {
            if a.len() != b.len() || a.keys().any(|k| !b.contains_key(k)) {
                return Err(reject(path, "the record's claims are not the certificate's"));
            }
            for (key, value) in a {
                compare_claims(run, &dot(path, key), value, &b[key])?;
            }
            Ok(())
        }
        (Value::Array(a), Value::Array(b)) => {
            run.claim(path, &a.len(), &b.len())?;
            for (k, (x, y)) in a.iter().zip(b).enumerate() {
                compare_claims(run, &at(path, k), x, y)?;
            }
            Ok(())
        }
        (x, y) => run.claim(path, &x.canonical(), &y.canonical()),
    }
}
