//! The expansion kind: straight-line programs over both alphabets, expanded by a naive reference here, with replay;
//! a long program streamed; and lengths that fail before any letter is produced.

use std::time::{Duration, Instant};

use c600_checkers::{CheckerKind, Verdict};
use c600_jumble::state::State;
use c600_kernel::json::Value;
use c600_kernel::{Derivation, ProtectedSet, Snapshot};

use super::common::*;

/// A program item: a letter, or a reference `(rule, inverse, times)`.
#[derive(Clone, Copy)]
enum It {
    L(i64),
    R(usize, bool, u64),
}

fn rule_value(items: &[It]) -> Value {
    Value::Array(
        items
            .iter()
            .map(|item| match *item {
                It::L(letter) => int(letter),
                It::R(rule, inverse, times) => obj(vec![
                    ("rule", int(rule as i64)),
                    ("inverse", Value::Bool(inverse)),
                    ("times", int(times as i64)),
                ]),
            })
            .collect(),
    )
}

/// The naive expansion, held whole (fixture construction).
fn expand(rules: &[Vec<It>], rule: usize, inverse: bool, invert: &dyn Fn(i64) -> i64, out: &mut Vec<i64>) {
    let items: Vec<It> = if inverse { rules[rule].iter().rev().copied().collect() } else { rules[rule].clone() };
    for item in items {
        match item {
            It::L(letter) => out.push(if inverse { invert(letter) } else { letter }),
            It::R(r, inv, times) => {
                for _ in 0..times {
                    expand(rules, r, inv != inverse, invert, out);
                }
            }
        }
    }
}

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

fn full_rules() -> Vec<Vec<It>> {
    vec![
        vec![It::L(1), It::L(2), It::L(-3)],
        vec![It::R(0, false, 3), It::L(-5)],
        vec![It::R(1, true, 1), It::R(0, false, 2), It::L(1200)],
    ]
}

#[test]
fn ct_kn6_01_core_expansion_gate_full_with_replay() {
    let rules = full_rules();
    let mut word = Vec::new();
    expand(&rules, 2, false, &|m| -m, &mut word);
    assert_eq!(word.len(), 10 + 7);
    let bytes: Vec<u8> = word.iter().flat_map(|&m| (m as i32).to_le_bytes()).collect();
    let solved = kernel().solved();
    let start = labels_input(&solved);
    let wrong = labels_input(&full_after(&solved, &[11]));
    let end = full_after(&solved, &word);
    let cert = obj(vec![
        ("format", text("kn6-expansion/1")),
        ("alphabet", text("full")),
        ("inputs", obj(vec![("start", text(&sha(&start)))])),
        ("rules", Value::Array(rules.iter().map(|r| rule_value(r)).collect())),
        ("root", int(2)),
        ("claims", obj(vec![("length", int(word.len() as i64)), ("sha256", text(&sha(&bytes)))])),
        (
            "replay",
            obj(vec![
                ("start", obj(vec![("digest", text(solved.digest()))])),
                ("end", obj(vec![("digest", text(end.digest()))])),
            ]),
        ),
    ]);
    let f = fixture(
        "full expansion",
        CheckerKind::Expansion,
        cert.canonical().into_bytes(),
        vec![("start", start)],
        &["inputs", "claims", "replay"],
        Some(("start", wrong)),
    );
    let report = assert_gate(&f);
    assert_eq!(report.tampered(), 5);
    let span = report.baseline.trace.iter().find(|s| s.claim == "claims.sha256").and_then(|s| s.span.clone());
    assert_eq!(span.map(|s| (s.word, s.from, s.to)), Some((sha(&bytes), 0, word.len() as u64)));
}

#[test]
fn ct_kn6_01_core_expansion_gate_ia16_inverse_returns_to_start() {
    let menu = menu("I_a");
    let inverse = |code: i64| -> i64 {
        let (cap, item) = (code / 60, (code % 60) as usize);
        let transposed = c600_jumble::q5::transpose(&menu.items[item].1);
        cap * 60 + menu.items.iter().position(|(_, m)| *m == transposed).unwrap() as i64
    };
    // r1 = r0 r0 J and the root r1 r1⁻¹: retained letters and one jumble, then their inverses, back to the start.
    let rules = vec![
        vec![It::L(60), It::L(5 * 60 + 2)],
        vec![It::R(0, false, 2), It::L(7 * 60 + 20)],
        vec![It::R(1, false, 1), It::R(1, true, 1)],
    ];
    let mut word = Vec::new();
    expand(&rules, 2, false, &inverse, &mut word);
    assert_eq!(word.len(), 10);
    let bytes: Vec<u8> = word.iter().flat_map(|&c| (c as u16).to_le_bytes()).collect();
    let solved = State::new(Some(menu.clone()));
    let mut end = solved.clone();
    apply_all(&mut end, &menu, &word.iter().map(|&c| ((c / 60) as usize, (c % 60) as usize)).collect::<Vec<_>>());
    assert_eq!(end.digest(jumble()), solved.digest(jumble()));
    let mut wrong = solved.clone();
    apply_all(&mut wrong, &menu, &[(3, 4)]);
    let menu_bytes = menu_record("I_a");
    let start = state_record(&solved);
    let cert = obj(vec![
        ("format", text("kn6-expansion/1")),
        ("alphabet", text("ia16")),
        ("inputs", obj(vec![("menu", text(&sha(&menu_bytes))), ("start", text(&sha(&start)))])),
        ("menu", obj(vec![("identity", text(&menu.identity)), ("order", text(&menu.order))])),
        ("rules", Value::Array(rules.iter().map(|r| rule_value(r)).collect())),
        ("root", int(2)),
        ("claims", obj(vec![("length", int(word.len() as i64)), ("sha256", text(&sha(&bytes)))])),
        (
            "replay",
            obj(vec![
                ("start", obj(vec![("digest", text(&solved.digest(jumble())))])),
                ("end", obj(vec![("digest", text(&end.digest(jumble())))])),
            ]),
        ),
    ]);
    let f = fixture(
        "ia16 expansion",
        CheckerKind::Expansion,
        cert.canonical().into_bytes(),
        vec![("menu", menu_bytes), ("start", start)],
        &["inputs", "menu", "claims", "replay"],
        Some(("start", state_record(&wrong))),
    );
    let report = assert_gate(&f);
    assert_eq!(report.tampered(), 8);
}

fn program(rules: Value, length: i64, digest: &str) -> Vec<u8> {
    obj(vec![
        ("format", text("kn6-expansion/1")),
        ("alphabet", text("full")),
        ("inputs", obj(vec![])),
        ("rules", rules),
        ("root", int(2)),
        ("claims", obj(vec![("length", int(length)), ("sha256", text(digest))])),
    ])
    .canonical()
    .into_bytes()
}

#[test]
fn ct_kn6_01_core_expansion_streams_a_long_word() {
    // 1,000,000 letters from three short rules; the checker never holds the word.
    let rules = Value::Array(vec![
        rule_value(&[It::L(7), It::L(-8)]),
        rule_value(&[It::R(0, false, 1000)]),
        rule_value(&[It::R(1, true, 500)]),
    ]);
    let mut hash = c600_digest::Sha256::new();
    for _ in 0..500 * 1000 {
        hash.update(&8i32.to_le_bytes());
        hash.update(&(-7i32).to_le_bytes());
    }
    let digest = c600_digest::hex(&hash.finish());
    let report = run(CheckerKind::Expansion, &program(rules, 1_000_000, &digest), &[]);
    assert_eq!(report.verdict, Verdict::Pass);
}

#[test]
fn ct_kn6_01_core_expansion_length_fails_before_expansion() {
    // 2^62 · 2^3 letters overflow 64 bits; 2^60 letters claimed as 2^60 + 2 fail at the length. Neither expands.
    let overflow = Value::Array(vec![
        rule_value(&[It::L(1)]),
        rule_value(&[It::R(0, false, 1 << 62)]),
        rule_value(&[It::R(1, false, 8)]),
    ]);
    let long = Value::Array(vec![
        rule_value(&[It::L(1)]),
        rule_value(&[It::R(0, false, 1 << 30)]),
        rule_value(&[It::R(1, false, 1 << 30)]),
    ]);
    let started = Instant::now();
    let report = run(CheckerKind::Expansion, &program(overflow, 0, &sha(b"")), &[]);
    assert_eq!(report.verdict.claim(), Some("claims.length"));
    let report = run(CheckerKind::Expansion, &program(long, (1 << 60) + 2, &sha(b"")), &[]);
    assert_eq!(report.verdict.claim(), Some("claims.length"));
    assert!(report.trace.iter().all(|s| s.span.is_none()));
    assert!(started.elapsed() < Duration::from_secs(60));
    // A reference forward is refused, so every program is acyclic.
    let forward =
        Value::Array(vec![rule_value(&[It::R(1, false, 1)]), rule_value(&[It::L(1)]), rule_value(&[It::L(2)])]);
    let report = run(CheckerKind::Expansion, &program(forward, 1, &sha(b"")), &[]);
    assert_eq!(report.verdict.claim(), Some("rules[0][0].rule"));
}
