//! Shared test setup: the engines loaded once from `assets/`, repository files read in place (read-only), and the
//! fixture builders. A builder computes a fixture's claims with direct engine calls written here, apart from the
//! checkers, so a fixture is never produced by the code it tests.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_checkers::gate::{self, Fixture, GateReport};
use c600_checkers::{check, CertificateRef, CheckReport, CheckerKind, Engines, Input};
use c600_jumble::model::NP;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| Kernel::load(&repository().join("assets")).expect("the model in assets/ loads"))
}

pub fn jumble() -> &'static Context {
    static CONTEXT: OnceLock<Context> = OnceLock::new();
    CONTEXT.get_or_init(|| Context::load(&repository().join("assets")).expect("the exact model loads"))
}

pub fn engines() -> Engines<'static> {
    Engines { kernel: Some(kernel()), jumble: Some(jumble()) }
}

/// A repository file, read in place (read-only).
pub fn read(relative: &str) -> Vec<u8> {
    std::fs::read(repository().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

pub fn parse(bytes: &[u8]) -> Value {
    json::parse(bytes).expect("JSON")
}

pub fn sha(bytes: &[u8]) -> String {
    c600_digest::sha256_hex(bytes)
}

/// A JSON object from pairs.
pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect::<BTreeMap<_, _>>())
}

pub fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

pub fn int(n: i64) -> Value {
    Value::Int(n)
}

pub fn letter_value(cap: usize, item: usize) -> Value {
    Value::Array(vec![int(cap as i64), int(item as i64)])
}

/// The exact menu record of a W-J fixture (`I_a`, `I_b`, `S4`), as canonical JSON bytes.
pub fn menu_record(name: &str) -> Vec<u8> {
    static CACHE: OnceLock<std::sync::Mutex<BTreeMap<String, Vec<u8>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(bytes) = cache.lock().unwrap().get(name) {
        return bytes.clone();
    }
    let fixture = parse(&read(&format!("research/jumbling/fixtures/wj-{name}.json")));
    let bytes = fixture.get("menu").expect("a menu").canonical().into_bytes();
    cache.lock().unwrap().insert(name.to_owned(), bytes.clone());
    bytes
}

pub fn menu(name: &str) -> Arc<Menu> {
    Arc::new(Menu::from_record(jumble(), &parse(&menu_record(name))).expect("the menu record"))
}

/// The applied letters of the W-J I_a journal (`research/jumbling/fixtures/wj-I_a.json`), as `[grip, item]`.
pub fn wj_letters() -> &'static [(usize, usize)] {
    static LETTERS: OnceLock<Vec<(usize, usize)>> = OnceLock::new();
    LETTERS.get_or_init(|| {
        let fixture = parse(&read("research/jumbling/fixtures/wj-I_a.json"));
        let records = fixture.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).unwrap();
        records
            .iter()
            .map(|rec| {
                let grip = rec.get("grip").and_then(Value::as_i64).unwrap() as usize;
                let label = rec.get("params").and_then(|p| p.get("label")).and_then(Value::as_str).unwrap();
                (grip, label.trim_start_matches("I_a[").trim_end_matches(']').parse().unwrap())
            })
            .collect()
    })
}

/// The twist of `[cap, item]` through the engine's own menu transport.
pub fn twist(menu: &Menu, cap: usize, item: usize) -> c600_jumble::twist::Twist {
    menu.for_cap(jumble(), cap).expect("the menu at the cap")[item].clone()
}

/// Applies letters that must all be admissible.
pub fn apply_all(state: &mut State, menu: &Menu, letters: &[(usize, usize)]) {
    for &(cap, item) in letters {
        let outcome = state.apply(jumble(), &twist(menu, cap, item), None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "[{cap}, {item}]");
    }
}

/// The state after the first `n` letters of the W-J I_a journal.
pub fn wj_state(menu: &Arc<Menu>, n: usize) -> State {
    let mut state = State::new(Some(menu.clone()));
    apply_all(&mut state, menu, &wj_letters()[..n]);
    state
}

/// A state record, as canonical JSON bytes.
pub fn state_record(state: &State) -> Vec<u8> {
    state.record(jumble()).canonical().into_bytes()
}

/// The count and SHA-256 of sorted piece IDs as int64.
pub fn support(pieces: &[u32]) -> (u64, String) {
    let mut sorted = pieces.to_vec();
    sorted.sort_unstable();
    let bytes: Vec<u8> = sorted.iter().flat_map(|&p| (p as i64).to_le_bytes()).collect();
    (sorted.len() as u64, sha(&bytes))
}

/// Left height and branch of a piece.
pub fn height(state: &State, piece: usize) -> (i64, i64) {
    let facts = c600_jumble::observe::tables(jumble()).unwrap().facts(jumble(), state.pose_of(piece)).unwrap();
    assert_eq!(facts.dr, 0);
    (facts.dl, facts.bl)
}

/// Φ = Σ max(0, h − 2).
pub fn phi(state: &State) -> i64 {
    let tables = c600_jumble::observe::tables(jumble()).unwrap();
    let mut counts = vec![0i64; state.poses().len()];
    for &id in state.pose_ids() {
        counts[id as usize] += 1;
    }
    state.poses().iter().zip(&counts).map(|(p, n)| n * (tables.facts(jumble(), p).unwrap().dl - 2).max(0)).sum()
}

/// Checks `certificate` with these inputs, stated to have its own digest.
pub fn run(kind: CheckerKind, certificate: &[u8], inputs: &[(&str, &[u8])]) -> CheckReport {
    let inputs: Vec<Input> = inputs.iter().map(|(role, bytes)| Input { role, bytes }).collect();
    let digest = sha(certificate);
    check(&engines(), &CertificateRef { digest: &digest, bytes: certificate, inputs: &inputs }, kind)
        .expect("a checker")
}

/// Runs the mutation gate and fails the test with every broken expectation.
pub fn assert_gate(fixture: &Fixture) -> GateReport {
    let report = gate::run(&engines(), fixture).expect("a checker");
    assert!(report.passed(), "{}: {:#?}", fixture.name, report.problems);
    assert!(report.tampered() > 0);
    report
}

/// A fixture with owned inputs.
pub fn fixture(
    name: &str,
    kind: CheckerKind,
    certificate: Vec<u8>,
    inputs: Vec<(&str, Vec<u8>)>,
    claims: &[&str],
    wrong_start: Option<(&str, Vec<u8>)>,
) -> Fixture {
    Fixture {
        name: name.to_owned(),
        kind,
        certificate,
        inputs: inputs.into_iter().map(|(r, b)| (r.to_owned(), b)).collect(),
        claims: claims.iter().map(|c| (*c).to_owned()).collect(),
        skip: Vec::new(),
        wrong_start: wrong_start.map(|(r, b)| (r.to_owned(), b)),
    }
}

/// The number of pieces.
pub const PIECES: usize = NP;
