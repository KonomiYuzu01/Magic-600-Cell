//! Shared setup of R4's tests: archive paths, the inputs a stage's sub-checkers read, and checks of a record against
//! its own stated values. Every input is an archive file read in place; the jam catalogue names each seed's table.

#![allow(dead_code)]

use c600_checkers::gate::Fixture;
use c600_checkers::{CheckReport, CheckerKind, Verdict};
use c600_kernel::json::Value;

use crate::common::*;

pub const S13: &str = "research/jumbling/restoration/13-tutorial-runner/runs/dev";
pub const S14: &str = "research/jumbling/restoration/14-blocking-structure/results";
pub const S15: &str = "research/jumbling/restoration/15-lowering-machine/results";
pub const S16: &str = "research/jumbling/restoration/16-six-jams";
pub const S17: &str = "research/jumbling/restoration/17-routing-in-driver/results";
pub const SEEDS: [&str; 6] = ["7302", "7303", "7304", "7305", "7306", "7307"];

pub fn jams() -> Vec<u8> {
    read(&format!("{S16}/jams.json"))
}

/// The D2 end table of a seed, read from the path the jam catalogue names.
pub fn table(seed: &str) -> Vec<u8> {
    let catalogue = parse(&jams());
    let path = catalogue.get(seed).and_then(|row| row.get("table")).and_then(Value::as_str).expect("a jam row");
    read(path)
}

/// The inputs of the macro, release and transport families at a seed.
pub fn inputs(seed: &str) -> Vec<(&'static str, Vec<u8>)> {
    vec![("table", table(seed)), ("menu", menu_record("I_a")), ("jams", jams())]
}

/// Checks a record with owned inputs under a sub-checker.
pub fn check(id: &'static str, record: &[u8], inputs: &[(&str, Vec<u8>)]) -> CheckReport {
    let borrowed: Vec<(&str, &[u8])> = inputs.iter().map(|(r, b)| (*r, b.as_slice())).collect();
    run(CheckerKind::Archive(id), record, &borrowed)
}

/// The seed a certificate's table names, by the catalogue's table digests.
pub fn seed_of(cert: &[u8]) -> String {
    let doc = parse(cert);
    let digest = doc.get("table_digest").and_then(Value::as_str).expect("a table digest");
    let catalogue = parse(&jams());
    SEEDS
        .iter()
        .find(|s| catalogue.get(s).and_then(|r| r.get("table_digest")).and_then(Value::as_str) == Some(digest))
        .expect("a jam table")
        .to_string()
}

/// A certificate passes under the stage's macro sub-checker, and its check record passes with it as `certificate`;
/// the check's final digest is the one the record states, and the record's trace covers the certificate's.
pub fn certificate_and_record(id: &'static str, cert_path: &str, check_path: &str) {
    let cert = read(cert_path);
    let seed = seed_of(&cert);
    let report = check(id, &cert, &inputs(&seed));
    assert_eq!(report.verdict, Verdict::Pass, "{cert_path}");
    assert!(report.complete());
    let record = read(check_path);
    let mut with = inputs(&seed);
    with.push(("certificate", cert));
    let checked = check(id, &record, &with);
    assert_eq!(checked.verdict, Verdict::Pass, "{check_path}");
    assert!(checked.complete());
    let stated = parse(&record);
    let end = checked.trace.iter().find(|s| s.claim == "digest").expect("the record's digest step");
    assert_eq!(Some(end.computed.as_str()), stated.get("digest").and_then(Value::as_str), "{check_path}");
    assert!(checked.trace.iter().any(|s| s.claim.starts_with("certificate.macros[0].claims.digests[")));
    assert!(checked.inputs.iter().any(|i| i.role == "certificate"));
}

/// A fixture of an archive record with the stage inputs, tampering only the named claims.
pub fn archive_fixture(
    name: &str,
    id: &'static str,
    record: Vec<u8>,
    inputs: Vec<(&str, Vec<u8>)>,
    claims: &[&str],
    wrong_table: Vec<u8>,
) -> Fixture {
    let mut f = fixture(name, CheckerKind::Archive(id), record, inputs, claims, Some(("table", wrong_table)));
    f.skip.clear();
    f
}

/// One step of a claim path: an object key or an array index.
#[derive(Clone, Debug)]
enum Seg {
    Key(String),
    Index(usize),
}

fn segments(path: &str) -> Vec<Seg> {
    let mut out = Vec::new();
    for part in path.split('.') {
        let (key, mut rest) = part.split_once('[').map_or((part, ""), |(k, r)| (k, r));
        if !key.is_empty() {
            out.push(Seg::Key(key.to_owned()));
        }
        while let Some((index, after)) = rest.split_once(']') {
            out.push(Seg::Index(index.parse().expect("an index")));
            rest = after.strip_prefix('[').unwrap_or(after);
        }
    }
    out
}

fn node<'v>(value: &'v mut Value, path: &[Seg]) -> Option<&'v mut Value> {
    let mut value = value;
    for seg in path {
        value = match (seg, value) {
            (Seg::Key(k), Value::Object(map)) => map.get_mut(k)?,
            (Seg::Index(i), Value::Array(items)) => items.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(value)
}

fn remove_key(v: &mut Value, k: &str) -> bool {
    matches!(v, Value::Object(m) if m.contains_key(k)) && {
        let Value::Object(m) = v else { unreachable!() };
        m.remove(k);
        true
    }
}

fn rename_key(v: &mut Value, k: &str) -> bool {
    let Value::Object(m) = v else { return false };
    let Some(x) = m.remove(k) else { return false };
    m.insert(format!("{k}_renamed"), x);
    true
}

fn add_unknown(v: &mut Value) -> bool {
    let Value::Object(m) = v else { return false };
    m.insert("kn6_unknown".to_owned(), Value::Null).is_none()
}

fn first_key(v: &Value) -> Option<String> {
    v.as_object().and_then(|m| m.keys().next().cloned())
}

fn drop_at(v: &mut Value, i: usize) -> bool {
    let Value::Array(a) = v else { return false };
    if i >= a.len() {
        return false;
    }
    a.remove(i);
    true
}

fn empty(v: &mut Value) -> bool {
    let Value::Array(a) = v else { return false };
    if a.is_empty() {
        return false;
    }
    a.clear();
    true
}

fn drop_last(v: &mut Value) -> bool {
    let Value::Array(a) = v else { return false };
    a.pop().is_some()
}

/// The structural mutations of the extended mutation gate, at and around each claim path: the claim's key deleted or
/// renamed and an unknown key added beside it; inside an object or array at the path, a key deleted, renamed or added,
/// the array emptied or one element dropped; and the nearest enclosing array emptied or the claim's element dropped.
/// Each mutated document differs from the record.
pub fn structural_mutations(record: &[u8], claims: &[&str]) -> Vec<(String, Vec<u8>)> {
    let doc = parse(record);
    let original = doc.canonical().into_bytes();
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut push = |name: String, value: Option<Value>| {
        if let Some(value) = value {
            let bytes = value.canonical().into_bytes();
            if bytes != original && !out.iter().any(|(_, b)| *b == bytes) {
                out.push((name, bytes));
            }
        }
    };
    let edit = |path: &[Seg], f: &dyn Fn(&mut Value) -> bool| -> Option<Value> {
        let mut copy = doc.clone();
        let target = node(&mut copy, path)?;
        f(target).then_some(copy)
    };
    for claim in claims {
        let path = segments(claim);
        let (parent, last) = path.split_at(path.len() - 1);
        match &last[0] {
            Seg::Key(k) => {
                push(format!("{claim}: deleted"), edit(parent, &|v| remove_key(v, k)));
                push(format!("{claim}: renamed"), edit(parent, &|v| rename_key(v, k)));
                push(format!("{claim}: unknown key beside"), edit(parent, &add_unknown));
            }
            Seg::Index(i) => push(format!("{claim}: element dropped"), edit(parent, &|v| drop_at(v, *i))),
        }
        // Inside an object or an array at the path.
        let mut copy = doc.clone();
        if let Some(k) = node(&mut copy, &path).and_then(|v| first_key(v)) {
            push(format!("{claim}: first key deleted"), edit(&path, &|v| remove_key(v, &k)));
            push(format!("{claim}: first key renamed"), edit(&path, &|v| rename_key(v, &k)));
        }
        push(format!("{claim}: unknown key inside"), edit(&path, &add_unknown));
        push(format!("{claim}: emptied"), edit(&path, &empty));
        push(format!("{claim}: first element dropped"), edit(&path, &|v| drop_at(v, 0)));
        push(format!("{claim}: last element dropped"), edit(&path, &drop_last));
        // The nearest enclosing array.
        if let Some(at) = path.iter().rposition(|s| matches!(s, Seg::Index(_))) {
            let Seg::Index(i) = path[at] else { unreachable!() };
            push(format!("{claim}: enclosing array emptied"), edit(&path[..at], &empty));
            push(format!("{claim}: enclosing element dropped"), edit(&path[..at], &|v| drop_at(v, i)));
        }
    }
    out
}

/// Every structural mutation of the record at the claim paths fails under the sub-checker (never a pass, never
/// `not_applicable`).
pub fn assert_structural_mutations_fail(id: &'static str, record: &[u8], inputs: &[(&str, Vec<u8>)], claims: &[&str]) {
    let mutations = structural_mutations(record, claims);
    assert!(!mutations.is_empty());
    let mut problems = Vec::new();
    for (name, bytes) in &mutations {
        let report = check(id, bytes, inputs);
        if !matches!(report.verdict, Verdict::Fail { .. }) {
            problems.push(format!("{name}: {:?}", report.verdict));
        }
    }
    assert!(
        problems.is_empty(),
        "{id}: {} of {} mutations did not fail: {problems:#?}",
        problems.len(),
        mutations.len()
    );
}

/// The structural mutations of a gate fixture at its claim paths all fail.
pub fn assert_fixture_structure(f: &Fixture) {
    let CheckerKind::Archive(id) = f.kind else { panic!("{}: not an archive fixture", f.name) };
    let inputs: Vec<(&str, Vec<u8>)> = f.inputs.iter().map(|(r, b)| (r.as_str(), b.clone())).collect();
    let claims: Vec<&str> = f.claims.iter().map(String::as_str).collect();
    assert_structural_mutations_fail(id, &f.certificate, &inputs, &claims);
}

/// The reason of a `not_applicable` verdict.
pub fn not_applicable(report: &CheckReport) -> &str {
    match &report.verdict {
        Verdict::NotApplicable { reason } => reason,
        other => panic!("expected not_applicable, got {other:?}"),
    }
}
