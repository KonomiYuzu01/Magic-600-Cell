//! KN6.5's mutation gate, reusable for every checker and fixture (CT-KN6-01's core, CT-KN6-03's harness).
//!
//! A [`Fixture`] is a certificate that passes, its inputs, the claim leaves to tamper and a wrong start table. [`run`]
//! checks the fixture as it is (it must pass), then each mutation (each must fail):
//!
//! - a wrong input digest: the certificate stated to have another SHA-256 than its bytes;
//! - a wrong start table: the start input replaced by another table, which must fail with
//!   `KN6:evidence.start_table_mismatch`;
//! - a tampered claim, once per claim leaf: one scalar under the fixture's claim paths changed (an integer by 2, a
//!   digest in its first character, a status word to another, a flag negated), the certificate re-serialized and its
//!   stated digest recomputed, so only the claim differs.
//!
//! The gate reads nothing but the fixture and runs nothing but [`crate::check`] or [`crate::run_checker`]; it holds no
//! producer code.

use c600_kernel::json::{self, Value};

use crate::kinds::Checker;
use crate::report::{codes, CertificateRef, CheckReport, CheckerKind, Input, Refusal, Verdict};
use crate::Engines;

/// A passing certificate with what the gate needs to mutate it.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub name: String,
    pub kind: CheckerKind,
    pub certificate: Vec<u8>,
    pub inputs: Vec<(String, Vec<u8>)>,
    /// Claim paths (as claim IDs name them: `a.b[2].c`); every scalar leaf at or under one of them is tampered. Empty:
    /// every scalar leaf of the certificate.
    pub claims: Vec<String>,
    /// Paths at or under which no leaf is tampered: a certificate's subject (its letters) and values that are not
    /// claims (an allowance, which a larger value only weakens).
    pub skip: Vec<String>,
    /// The start input's role and the bytes of a wrong start table for it. KN6.5 requires one: a fixture without it
    /// is a problem of the gate.
    pub wrong_start: Option<(String, Vec<u8>)>,
}

/// What one mutation did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub mutation: String,
    pub verdict: Verdict,
}

/// The gate's result: the baseline report, every mutation's verdict and every broken expectation.
#[derive(Clone, Debug)]
pub struct GateReport {
    pub baseline: CheckReport,
    pub outcomes: Vec<Outcome>,
    pub problems: Vec<String>,
}

impl GateReport {
    pub fn passed(&self) -> bool {
        self.problems.is_empty()
    }

    /// The number of tampered claim leaves.
    pub fn tampered(&self) -> usize {
        self.outcomes.iter().filter(|o| o.mutation.starts_with("tampered ")).count()
    }
}

/// Runs the gate through [`crate::check`].
pub fn run(engines: &Engines, fixture: &Fixture) -> Result<GateReport, Refusal> {
    run_with(fixture, |cert| crate::check(engines, cert, fixture.kind))
}

/// Runs the gate on a checker that is not reached through [`crate::check`] (an unregistered archive sub-checker).
pub fn run_on(engines: &Engines, fixture: &Fixture, checker: &Checker) -> Result<GateReport, Refusal> {
    run_with(fixture, |cert| crate::run_checker(engines, cert, fixture.kind, checker))
}

fn run_with(
    fixture: &Fixture,
    check: impl Fn(&CertificateRef) -> Result<CheckReport, Refusal>,
) -> Result<GateReport, Refusal> {
    let inputs: Vec<Input> = fixture.inputs.iter().map(|(role, bytes)| Input { role, bytes }).collect();
    let digest = c600_digest::sha256_hex(&fixture.certificate);
    let baseline = check(&CertificateRef { digest: &digest, bytes: &fixture.certificate, inputs: &inputs })?;
    let mut problems = Vec::new();
    if !baseline.verdict.is_pass() {
        problems.push(format!("{}: the fixture does not pass: {:?}", fixture.name, baseline.verdict));
    }
    let mut outcomes = Vec::new();

    let wrong = flip(&digest);
    let report = check(&CertificateRef { digest: &wrong, bytes: &fixture.certificate, inputs: &inputs })?;
    expect_fail(&mut problems, &fixture.name, "wrong input digest", &report.verdict, None);
    outcomes.push(Outcome { mutation: "wrong input digest".into(), verdict: report.verdict });

    if fixture.wrong_start.is_none() {
        problems.push(format!("{}: the fixture names no wrong start table (KN6.5)", fixture.name));
    }
    if let Some((role, table)) = &fixture.wrong_start {
        if !fixture.inputs.iter().any(|(r, _)| r == role) {
            problems.push(format!("{}: the wrong start table replaces no input {role}", fixture.name));
        }
        let replaced: Vec<Input> = fixture
            .inputs
            .iter()
            .map(|(r, bytes)| Input { role: r, bytes: if r == role { table } else { bytes } })
            .collect();
        let report = check(&CertificateRef { digest: &digest, bytes: &fixture.certificate, inputs: &replaced })?;
        let mutation = format!("wrong start table ({role})");
        expect_fail(&mut problems, &fixture.name, &mutation, &report.verdict, Some(codes::START_TABLE_MISMATCH));
        outcomes.push(Outcome { mutation, verdict: report.verdict });
    }

    let doc = json::parse(&fixture.certificate).map_err(|_| Refusal {
        code: codes::NO_CHECKER,
        message: format!("{}: the fixture is not JSON", fixture.name),
    })?;
    let mut leaves = Vec::new();
    collect_leaves(&doc, String::new(), &mut leaves);
    let selected: Vec<String> = leaves
        .into_iter()
        .filter(|path| {
            (fixture.claims.is_empty() || fixture.claims.iter().any(|c| under(path, c)))
                && !fixture.skip.iter().any(|s| under(path, s))
        })
        .collect();
    if selected.is_empty() {
        problems.push(format!("{}: no claim leaf to tamper", fixture.name));
    }
    for path in selected {
        let mut tampered = doc.clone();
        if !tamper_at(&mut tampered, &path) {
            problems.push(format!("{}: the leaf {path} could not be tampered", fixture.name));
            continue;
        }
        let bytes = tampered.canonical().into_bytes();
        let digest = c600_digest::sha256_hex(&bytes);
        let report = check(&CertificateRef { digest: &digest, bytes: &bytes, inputs: &inputs })?;
        let mutation = format!("tampered {path}");
        expect_fail(&mut problems, &fixture.name, &mutation, &report.verdict, None);
        outcomes.push(Outcome { mutation, verdict: report.verdict });
    }
    Ok(GateReport { baseline, outcomes, problems })
}

fn expect_fail(problems: &mut Vec<String>, name: &str, mutation: &str, verdict: &Verdict, code: Option<&str>) {
    match verdict {
        Verdict::Fail { code: found, .. } if code.is_none_or(|code| code == *found) => {}
        other => problems.push(format!("{name}: {mutation} did not fail as required: {other:?}")),
    }
}

/// Whether `path` is `prefix` or lies under it.
fn under(path: &str, prefix: &str) -> bool {
    path == prefix || (path.starts_with(prefix) && matches!(path.as_bytes().get(prefix.len()), Some(b'.') | Some(b'[')))
}

/// Every scalar leaf of a document, by claim path.
pub fn collect_leaves(value: &Value, path: String, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, member) in map {
                let child = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                collect_leaves(member, child, out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_leaves(item, format!("{path}[{index}]"), out);
            }
        }
        _ => out.push(path),
    }
}

/// Changes the scalar at `path`; false when the path names no scalar.
pub fn tamper_at(value: &mut Value, path: &str) -> bool {
    let Some(leaf) = walk(value, path) else { return false };
    *leaf = tampered(leaf);
    true
}

fn walk<'v>(mut value: &'v mut Value, path: &str) -> Option<&'v mut Value> {
    let mut rest = path;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('[') {
            let close = after.find(']')?;
            let index: usize = after[..close].parse().ok()?;
            value = match value {
                Value::Array(items) => items.get_mut(index)?,
                _ => return None,
            };
            rest = &after[close + 1..];
        } else {
            let rest_key = rest.strip_prefix('.').unwrap_or(rest);
            let end = rest_key.find(['.', '[']).unwrap_or(rest_key.len());
            let key = &rest_key[..end];
            value = match value {
                Value::Object(map) => map.get_mut(key)?,
                _ => return None,
            };
            rest = &rest_key[end..];
        }
    }
    match value {
        Value::Object(_) | Value::Array(_) => None,
        _ => Some(value),
    }
}

/// A different value of the same shape.
fn tampered(value: &Value) -> Value {
    match value {
        Value::Int(n) => Value::Int(n.wrapping_add(2)),
        Value::Bool(b) => Value::Bool(!b),
        Value::Null => Value::Int(0),
        Value::Number(lexeme) => Value::Number(format!("{lexeme}1")),
        Value::String(text) => Value::String(match text.as_str() {
            "admissible" => "blocked".into(),
            "blocked" => "admissible".into(),
            _ if text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()) => flip(text),
            _ => format!("{text}~"),
        }),
        Value::Array(_) | Value::Object(_) => value.clone(),
    }
}

/// A hex digest with its first character changed.
fn flip(digest: &str) -> String {
    let mut chars: Vec<char> = digest.chars().collect();
    if let Some(first) = chars.first_mut() {
        *first = if *first == '0' { '1' } else { '0' };
    }
    chars.into_iter().collect()
}
