//! The checkpoints kind on both puzzles: label digests along a 600-cell-Full word, and digests, lattice flags, π's
//! labels and ν along a jumbling word that leaves the lattice at its last letter.

use c600_checkers::{CheckerKind, Verdict};
use c600_jumble::levels::{self, Partial};
use c600_jumble::nu;
use c600_jumble::state::State;
use c600_kernel::json::Value;
use c600_kernel::{Derivation, ProtectedSet, Snapshot};

use super::common::*;

/// The 600-cell-Full state after `letters`, by the kernel's replay (fixture construction).
fn full_after(base: &Snapshot, letters: &[i64]) -> Snapshot {
    let mut replay = kernel().replay(base, &ProtectedSet::none()).unwrap();
    replay.push(letters).unwrap();
    match replay.finish() {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(_) => panic!("refused"),
    }
}

fn labels_input(state: &Snapshot) -> Vec<u8> {
    obj(vec![("labels", Value::Array(state.labels().iter().map(|&l| int(l as i64)).collect()))])
        .canonical()
        .into_bytes()
}

const FULL_WORD: [i64; 8] = [5, -17, 1200, 3, 3, -1199, 42, 8];

fn full_certificate(word: Value, inputs: Vec<(&str, Value)>) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let solved = kernel().solved();
    let start = labels_input(&solved);
    let wrong = labels_input(&full_after(&solved, &[7]));
    let checkpoints: Vec<Value> = [2usize, 5, 8]
        .iter()
        .map(|&at| obj(vec![("at", int(at as i64)), ("labels", text(full_after(&solved, &FULL_WORD[..at]).digest()))]))
        .collect();
    let mut bindings = vec![("start", text(&sha(&start)))];
    bindings.extend(inputs);
    let cert = obj(vec![
        ("format", text("kn6-checkpoints/1")),
        ("puzzle", text("600-cell-Full")),
        ("inputs", obj(bindings)),
        ("start", obj(vec![("digest", text(solved.digest()))])),
        ("word", word),
        ("checkpoints", Value::Array(checkpoints)),
    ]);
    (cert.canonical().into_bytes(), start, wrong)
}

#[test]
fn ct_kn6_01_core_checkpoints_gate_full() {
    let word = Value::Array(FULL_WORD.iter().map(|&m| int(m)).collect());
    let (cert, start, wrong) = full_certificate(word, vec![]);
    let f = fixture(
        "600-cell-Full checkpoints",
        CheckerKind::Checkpoints,
        cert,
        vec![("start", start)],
        &["inputs", "start", "checkpoints[0].labels", "checkpoints[1].labels", "checkpoints[2].labels"],
        Some(("start", wrong)),
    );
    let report = assert_gate(&f);
    assert_eq!(report.tampered(), 5);
    // Each checkpoint's trace step cites its span of the word by digest and positions.
    let spans: Vec<_> = report.baseline.trace.iter().filter_map(|s| s.span.clone()).collect();
    assert_eq!(spans.iter().map(|s| (s.from, s.to)).collect::<Vec<_>>(), [(0, 2), (2, 5), (5, 8)]);
}

#[test]
fn ct_kn6_01_core_checkpoints_word_from_an_input() {
    let bytes: Vec<u8> = FULL_WORD.iter().flat_map(|&m| (m as i32).to_le_bytes()).collect();
    let word = obj(vec![
        ("input", text("word")),
        ("encoding", text("i32le")),
        ("letters", int(FULL_WORD.len() as i64)),
        ("sha256", text(&sha(&bytes))),
    ]);
    let (cert, start, _) = full_certificate(word, vec![("word", text(&sha(&bytes)))]);
    let report = run(CheckerKind::Checkpoints, &cert, &[("start", &start), ("word", &bytes)]);
    assert_eq!(report.verdict, Verdict::Pass);
    // A word file longer than its declared letters fails before it is read as letters.
    let longer: Vec<u8> = bytes.iter().copied().chain(1i32.to_le_bytes()).collect();
    let mut doc = parse(&cert);
    let Value::Object(top) = &mut doc else { unreachable!() };
    top.insert("inputs".into(), obj(vec![("start", text(&sha(&start))), ("word", text(&sha(&longer)))]));
    let report = run(CheckerKind::Checkpoints, doc.canonical().as_bytes(), &[("start", &start), ("word", &longer)]);
    assert_eq!(report.verdict.claim(), Some("word.letters"));
}

/// A gzip member of stored DEFLATE blocks (fixture construction; RFC 1951 section 3.2.4, RFC 1952).
fn gzip_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
    let chunks: Vec<&[u8]> = if data.is_empty() { vec![&[][..]] } else { data.chunks(65_535).collect() };
    for (k, chunk) in chunks.iter().enumerate() {
        out.push(u8::from(k + 1 == chunks.len()));
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&c600_kernel::archive::crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

/// Retained letters keep every piece on the lattice; the last letter, a jumble, leaves it.
const JUMBLE_WORD: [(usize, usize); 4] = [(0, 1), (17, 5), (300, 11), (45, 20)];

fn jumbling_certificate(gzip: bool) -> (Vec<u8>, Vec<(&'static str, Vec<u8>)>, Vec<u8>) {
    let menu = menu("I_a");
    let start = State::new(Some(menu.clone()));
    let mut wrong = start.clone();
    apply_all(&mut wrong, &menu, &[(9, 2)]);
    let mut checkpoints = Vec::new();
    let mut state = start.clone();
    for (done, at) in [(0usize, 2usize), (2, 3), (3, 4)] {
        apply_all(&mut state, &menu, &JUMBLE_WORD[done..at]);
        let labels = match levels::labels(jumble(), &state).unwrap() {
            Partial::Total(labels) => text(&labels.digest()),
            _ => Value::Null,
        };
        let nu = match nu::value(jumble(), &state).unwrap() {
            Partial::Total(v) => Value::Array(v.values().iter().map(|&n| int(n)).collect()),
            Partial::Undefined(reason) => text(&reason),
            Partial::Partial { .. } => panic!("ν is total or undefined"),
        };
        checkpoints.push(obj(vec![
            ("at", int(at as i64)),
            ("digest", text(&state.digest(jumble()))),
            ("lattice", text(&state.lattice_digest())),
            ("labels", labels),
            ("nu", nu),
        ]));
    }
    let menu_bytes = menu_record("I_a");
    let start_bytes = state_record(&start);
    let codes: Vec<u8> = JUMBLE_WORD.iter().flat_map(|&(c, i)| ((60 * c + i) as u16).to_le_bytes()).collect();
    let mut bindings = vec![("menu", text(&sha(&menu_bytes))), ("start", text(&sha(&start_bytes)))];
    let mut inputs = vec![("menu", menu_bytes), ("start", start_bytes)];
    let word = if gzip {
        let file = gzip_stored(&codes);
        bindings.push(("word", text(&sha(&file))));
        inputs.push(("word", file));
        obj(vec![
            ("input", text("word")),
            ("encoding", text("ia16-gzip")),
            ("letters", int(JUMBLE_WORD.len() as i64)),
            ("sha256", text(&sha(&codes))),
        ])
    } else {
        Value::Array(JUMBLE_WORD.iter().map(|&(c, i)| letter_value(c, i)).collect())
    };
    let cert = obj(vec![
        ("format", text("kn6-checkpoints/1")),
        ("puzzle", text("jumbling")),
        ("inputs", obj(bindings)),
        ("menu", obj(vec![("identity", text(&menu.identity)), ("order", text(&menu.order))])),
        ("start", obj(vec![("digest", text(&start.digest(jumble())))])),
        ("word", word),
        ("checkpoints", Value::Array(checkpoints)),
    ]);
    (cert.canonical().into_bytes(), inputs, state_record(&wrong))
}

#[test]
fn ct_kn6_01_core_checkpoints_gate_jumbling() {
    let (cert, inputs, wrong) = jumbling_certificate(false);
    let doc = parse(&cert);
    // ν is total on the lattice and undefined after the jumble; π's labels likewise.
    let checkpoints = doc.get("checkpoints").and_then(Value::as_array).unwrap();
    assert!(checkpoints[1].get("nu").and_then(Value::as_array).is_some_and(|v| v.len() == 45));
    assert_eq!(checkpoints[2].get("nu").and_then(Value::as_str), Some(nu::OFF_LATTICE));
    assert_eq!(checkpoints[2].get("labels"), Some(&Value::Null));
    let mut f = fixture(
        "jumbling checkpoints",
        CheckerKind::Checkpoints,
        cert,
        inputs,
        &["inputs", "menu", "start", "checkpoints"],
        Some(("start", wrong)),
    );
    for k in 0..3 {
        f.skip.push(format!("checkpoints[{k}].at"));
        // ν is tampered in its first and last coordinates.
        for c in 1..44 {
            f.skip.push(format!("checkpoints[{k}].nu[{c}]"));
        }
    }
    // A checkpoint's ν, labels and lattice are optional parts of kn6-checkpoints/1: one left out claims less.
    for k in 0..3 {
        f.optional.extend(["nu", "labels", "lattice"].map(|part| format!("checkpoints[{k}].{part}")));
    }
    let report = assert_gate(&f);
    assert!(report.tampered() >= 16, "{}", report.tampered());
}

#[test]
fn ct_kn6_01_core_checkpoints_ia16_gzip_word() {
    let (cert, inputs, _) = jumbling_certificate(true);
    let inputs: Vec<(&str, &[u8])> = inputs.iter().map(|(r, b)| (*r, b.as_slice())).collect();
    let report = run(CheckerKind::Checkpoints, &cert, &inputs);
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.inputs.iter().any(|b| b.role == "word" && b.identity.iter().any(|(k, _)| k == "decoded_sha256")));
}
