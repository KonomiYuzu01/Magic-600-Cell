//! Stage 14's research-archive sub-checkers (LP5 piece R3) on the archive's records, read in place and read-only.
//!
//! Test names carry the CT ID and part (PHS.62): CT-LB5-01 (stage 14: every self-contained `t3p-*/1` claim group and
//! replay record checks; changed helper sets, precedence and input or start digests fail; replay rows keep their
//! chronological observables) and CT-KN6-03's stage 14 part (the stage's sub-checkers registered, each passing its
//! mutation gate on an archive record).
//!
//! The records are `research/jumbling/restoration/14-blocking-structure/results/`; their start tables are stage 13's,
//! found through the stage's own catalogue `results/sources.json` by state digest. The menu is I_a.

#[path = "kn6/common.rs"]
mod common;

use std::collections::BTreeMap;
use std::time::Instant;

use c600_checkers::archive::{self, s14};
use c600_checkers::gate::Fixture;
use c600_checkers::{codes, CheckReport, CheckerKind, Verdict};
use c600_kernel::json::Value;

use common::*;

const S14: &str = "research/jumbling/restoration/14-blocking-structure/results";

const T3P: CheckerKind = CheckerKind::Archive("s14.t3p");
const LITERAL: CheckerKind = CheckerKind::Archive("s14.literal");
const LOCALITY: CheckerKind = CheckerKind::Archive("s14.locality");
const LEVELS: CheckerKind = CheckerKind::Archive("s14.levels");
const BLOCKS: CheckerKind = CheckerKind::Archive("s14.blocks");
const KINDS: [CheckerKind; 5] = [T3P, LITERAL, LOCALITY, LEVELS, BLOCKS];

/// A stage 14 result file, read in place.
fn record(name: &str) -> Vec<u8> {
    read(&format!("{S14}/{name}"))
}

/// The stage 13 table with this state digest, through `results/sources.json`.
fn table(digest: &str) -> Vec<u8> {
    let sources = parse(&record("sources.json"));
    let entry = sources
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s.get("digest").and_then(Value::as_str) == Some(digest))
        .unwrap_or_else(|| panic!("no catalogued table has digest {digest}"));
    let path = entry.get("table").and_then(Value::as_str).unwrap();
    let bytes = read(path);
    assert_eq!(sha(&bytes), entry.get("table_sha256").and_then(Value::as_str).unwrap(), "{path}");
    bytes
}

/// Another catalogued table: the wrong start for every fixture here.
fn other_table() -> Vec<u8> {
    table("cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062")
}

const S7304_END: &str = "ac127dc3488832b1d1b75a043d63e84b75addd1f4e73fe7e961407aea2ff0980";

/// The six D2 ends of section 5.7, by seed, with their state digests.
const ENDS: [(&str, &str); 6] = [
    ("s7302", "2430f4cc99ea121a6d3476ec21b66fc401e277d13809ad5e1c979e6e0abe1472"),
    ("s7303", "d01dc91edd1c73da399b8acb13a57b9f526da64a5d754993f42ae02856d51889"),
    ("s7304", S7304_END),
    ("s7305", "cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062"),
    ("s7306", "ac4be82cf80e857137af957ff75438525ba950062e46a2fc5249aa0d7199d8f2"),
    ("s7307", "895b45de8a9a83604ca8bf988a3c9a287e404e59bd22a5004703d0852b252507"),
];

/// Checks a record with the I_a menu and the table of `digest`, plus further inputs.
fn check_with(kind: CheckerKind, name: &str, digest: &str, more: &[(&str, &[u8])]) -> CheckReport {
    let start = Instant::now();
    let menu = menu_record("I_a");
    let table = table(digest);
    let mut inputs: Vec<(&str, &[u8])> = vec![("table", &table), ("menu", &menu)];
    inputs.extend_from_slice(more);
    let report = run(kind, &record(name), &inputs);
    eprintln!("{name}: {:?} in {:.1} s", report.verdict, start.elapsed().as_secs_f64());
    report
}

fn assert_pass(report: &CheckReport, name: &str) {
    assert!(report.complete(), "{name}: {:?}", report.verdict);
}

/// A fixture on an archive record with the I_a menu, its table and the wrong start table.
fn archive_fixture(
    kind: CheckerKind,
    name: &str,
    digest: &str,
    more: Vec<(&str, Vec<u8>)>,
    claims: &[&str],
) -> Fixture {
    let mut inputs = vec![("table", table(digest)), ("menu", menu_record("I_a"))];
    inputs.extend(more);
    fixture(name, kind, record(name), inputs, claims, Some(("table", other_table())))
}

// ------------------------------------------------------------------------------------------------ CT-LB5-01: T3P

#[test]
fn ct_lb5_01_t3p_w3_w5_and_scan_certificates_pass() {
    for name in ["s7304-w3-certificate.json", "s7304-w5-certificate.json"] {
        assert_pass(&check_with(T3P, name, S7304_END, &[]), name);
    }
    for (seed, digest) in ENDS {
        let name = format!("{seed}-t3p-scan.certificate.json");
        let report = check_with(T3P, &name, digest, &[]);
        assert_pass(&report, &name);
        assert_eq!(report.checker.id, "s14.t3p");
    }
}

#[test]
fn ct_lb5_01_t3p_checks_match_their_records() {
    for (check, certificate) in [
        ("s7304-t3p-check-w3.json", "s7304-w3-certificate.json"),
        ("s7304-t3p-check-w5.json", "s7304-w5-certificate.json"),
    ] {
        let cert = record(certificate);
        let report = check_with(T3P, check, S7304_END, &[("certificate", &cert)]);
        assert_pass(&report, check);
        // Every stated number was compared: the trace names the per-helper and replay claims.
        for claim in ["rounds[0].helpers[0].f", "rounds[0].replay[0].digest", "rounds[0].d0", "digest_after"] {
            assert!(report.trace.iter().any(|s| s.claim == claim && s.held), "{check}: {claim}");
        }
        // Without the certificate the check record stands on its own.
        assert_pass(&check_with(T3P, check, S7304_END, &[]), check);
    }
    // A check record against another certificate fails.
    let other = record("s7304-w5-certificate.json");
    let report = check_with(T3P, "s7304-t3p-check-w3.json", S7304_END, &[("certificate", &other)]);
    assert!(report.verdict.is_fail(), "{:?}", report.verdict);
}

#[test]
fn ct_lb5_01_t3p_chains_reproduce_rounds_and_word() {
    for (seed, digest) in ENDS {
        let name = format!("{seed}-t3p-chain.json");
        let report = check_with(T3P, &name, digest, &[]);
        assert_pass(&report, &name);
        let scope = report.trace.iter().find(|s| s.claim == "scope").expect("the chain's scope is traced");
        assert!(scope.computed.contains("14/finite_search"));
    }
}

/// A W3 certificate with one field changed.
fn w3_with(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut doc = parse(&record("s7304-w3-certificate.json"));
    edit(&mut doc);
    doc.canonical().into_bytes()
}

fn round_mut(doc: &mut Value) -> &mut BTreeMap<String, Value> {
    let Value::Object(top) = doc else { panic!() };
    let Some(Value::Array(rounds)) = top.get_mut("rounds") else { panic!() };
    let Value::Object(round) = &mut rounds[0] else { panic!() };
    round
}

#[test]
fn ct_lb5_01_t3p_changed_helpers_precedence_or_start_fail() {
    let menu = menu_record("I_a");
    let start = table(S7304_END);
    let inputs: [(&str, &[u8]); 2] = [("table", &start), ("menu", &menu)];
    let letter = |c: i64, i: i64| Value::Array(vec![Value::Int(c), Value::Int(i)]);
    let cases: Vec<(&str, Vec<u8>)> = vec![
        // A changed helper set: one helper dropped, another item, a helper that overlaps, the target as a helper.
        (
            "helper dropped",
            w3_with(|d| {
                round_mut(d).insert("helpers".into(), Value::Array(vec![letter(451, 55)]));
            }),
        ),
        // (451, 38) leaves no straddler at 522 but a foreign load of 114 > D = 70 (README section 5.3): P4 fails.
        (
            "helper item changed",
            w3_with(|d| {
                round_mut(d).insert("helpers".into(), Value::Array(vec![letter(344, 9), letter(451, 38)]));
            }),
        ),
        (
            "helper added",
            w3_with(|d| {
                round_mut(d)
                    .insert("helpers".into(), Value::Array(vec![letter(344, 9), letter(451, 55), letter(229, 1)]));
            }),
        ),
        (
            "target as helper",
            w3_with(|d| {
                round_mut(d).insert("helpers".into(), Value::Array(vec![letter(344, 9), letter(522, 1)]));
            }),
        ),
        (
            "repeated helper grip",
            w3_with(|d| {
                round_mut(d).insert("helpers".into(), Value::Array(vec![letter(344, 9), letter(344, 1)]));
            }),
        ),
        // A lowering item that does not lower the unit.
        (
            "retained lowering item",
            w3_with(|d| {
                round_mut(d).insert("h".into(), Value::Int(1));
            }),
        ),
        // The start digest of another state.
        (
            "start digest",
            w3_with(|d| {
                let Value::Object(top) = d else { panic!() };
                top.insert("table_digest".into(), Value::String(ENDS[0].1.into()));
            }),
        ),
    ];
    // (451, 43) instead of (451, 55) is another round of the same stop (F = 26 < D = 70): the checker decides by the
    // definitions, so it passes.
    let other = w3_with(|d| {
        round_mut(d).insert("helpers".into(), Value::Array(vec![letter(344, 9), letter(451, 43)]));
    });
    assert!(run(T3P, &other, &inputs).complete());
    for (what, bytes) in cases {
        let report = run(T3P, &bytes, &inputs);
        assert!(report.verdict.is_fail(), "{what}: {:?}", report.verdict);
        if what == "start digest" {
            assert_eq!(report.verdict.code(), Some(codes::START_TABLE_MISMATCH), "{what}");
        }
    }
    // A changed precedence: a check record stating an edge the images do not give, or an order that is not one. No
    // stored round of the stage has a precedence edge (W3, W5 and every chain round: their digraphs are empty, and the
    // reversed order of each two-helper round is still topological), so a reversed real edge is tested on the order
    // functions themselves (`t3p::tests`, CT-LB5-01).
    let mut doc = parse(&record("s7304-t3p-check-w3.json"));
    {
        let Value::Object(top) = &mut doc else { panic!() };
        let Some(Value::Array(rounds)) = top.get_mut("rounds") else { panic!() };
        let Value::Object(round) = &mut rounds[0] else { panic!() };
        round.insert("edges".into(), Value::Array(vec![letter(451, 344)]));
    }
    let report = run(T3P, doc.canonical().as_bytes(), &inputs);
    assert_eq!(report.verdict.claim(), Some("rounds[0].edges.length"), "{:?}", report.verdict);
    let mut doc = parse(&record("s7304-t3p-chain.json"));
    {
        let Value::Object(top) = &mut doc else { panic!() };
        let Some(Value::Array(rounds)) = top.get_mut("rounds") else { panic!() };
        let Value::Object(round) = &mut rounds[0] else { panic!() };
        round.insert("order".into(), Value::Array(vec![Value::Int(344)]));
    }
    let report = run(T3P, doc.canonical().as_bytes(), &inputs);
    assert_eq!(report.verdict.claim(), Some("rounds[0].order"), "{:?}", report.verdict);
    // A wrong input digest.
    let bytes = record("s7304-w3-certificate.json");
    let wrong = sha(b"another certificate");
    let refs: Vec<c600_checkers::Input> =
        inputs.iter().map(|(role, bytes)| c600_checkers::Input { role, bytes }).collect();
    let cert = c600_checkers::CertificateRef { digest: &wrong, bytes: &bytes, inputs: &refs };
    let report = c600_checkers::check(&engines(), &cert, T3P).unwrap();
    assert_eq!(report.verdict.claim(), Some("input"));
}

// ------------------------------------------------------------------------------------------- CT-LB5-01: literal

#[test]
fn ct_lb5_01_literal_rows_keep_their_chronological_observables() {
    let mut names: Vec<(String, &str)> =
        ENDS.iter().map(|(seed, digest)| (format!("{seed}-t3p-chain-replay.json"), *digest)).collect();
    names.push(("s7304-peel-522-replay.json".into(), S7304_END));
    names.push(("s7304-w5-replay.json".into(), S7304_END));
    names.push(("s7307-516-14-replay.json".into(), ENDS[5].1));
    for (name, digest) in &names {
        let report = check_with(LITERAL, name, digest, &[]);
        assert_pass(&report, name);
        let rows = parse(&record(name)).get("rows").and_then(Value::as_array).unwrap().len();
        // Every row's digest was recomputed, in order.
        for k in 0..rows {
            assert!(report.trace.iter().any(|s| s.claim == format!("rows[{k}].digest") && s.held), "{name}: row {k}");
        }
    }
    // Rows out of chronological order fail: the W5 replay with its two middle rows swapped.
    let mut doc = parse(&record("s7304-w5-replay.json"));
    {
        let Value::Object(top) = &mut doc else { panic!() };
        let Some(Value::Array(rows)) = top.get_mut("rows") else { panic!() };
        rows.swap(2, 3);
    }
    let menu = menu_record("I_a");
    let start = table(S7304_END);
    let report = run(LITERAL, doc.canonical().as_bytes(), &[("table", &start), ("menu", &menu)]);
    assert_eq!(report.verdict.claim(), Some("rows[2].letter"), "{:?}", report.verdict);
}

// ------------------------------------------------------------------------------- CT-LB5-01: locality and levels

#[test]
fn ct_lb5_01_locality_recomputed_from_the_model() {
    let manifest = read("assets/manifest.json");
    let report = run(LOCALITY, &record("locality-summary.json"), &[("model", &manifest)]);
    assert_pass(&report, "locality-summary.json");
    assert!(report.trace.iter().any(|s| s.claim == "partner_counts[599]" && s.computed == "80"));
}

/// The word record behind a levels record: none for the stored ends, the chain for a chain's end, W3 and W5.
fn levels_cases() -> Vec<(String, &'static str, Option<String>)> {
    let mut cases = Vec::new();
    for (seed, digest) in ENDS {
        cases.push((format!("{seed}-levels.json"), digest, None));
        cases.push((format!("{seed}-chain-end-levels.json"), digest, Some(format!("{seed}-t3p-chain.json"))));
    }
    cases.push(("s7304-levels-w3.json".into(), S7304_END, Some("s7304-t3p-chain.json".into())));
    cases.push(("s7304-levels-w5.json".into(), S7304_END, Some("s7304-w5-replay.json".into())));
    cases
}

fn check_levels(name: &str, digest: &str, word: Option<&str>) {
    let word = word.map(record);
    let more: Vec<(&str, &[u8])> = word.iter().map(|w| ("word", &w[..])).collect();
    let report = check_with(LEVELS, name, digest, &more);
    assert_pass(&report, name);
}

#[test]
fn ct_lb5_01_levels_rebuilt_at_the_7304_stop() {
    check_levels("s7304-levels.json", S7304_END, None);
}

#[test]
#[ignore = "slow: rebuilds the 13 other level tables (about 100 s); the 7304 stop above is the fast representative"]
fn ct_lb5_01_levels_rebuilt_from_every_table_and_word() {
    for (name, digest, word) in levels_cases() {
        if name != "s7304-levels.json" {
            check_levels(&name, digest, word.as_deref());
        }
    }
}

// ------------------------------------------------------------------------ CT-LB5-01: block and holder records

#[test]
fn ct_lb5_01_coheld_and_per_grip_rows_recomputed_at_the_7304_stop() {
    for name in ["s7304-coheld.json", "s7304-end-caps.json", "s7304-jam.json"] {
        assert_pass(&check_with(BLOCKS, name, S7304_END, &[]), name);
    }
}

#[test]
#[ignore = "slow (about 45 s): ct_kn6_03_stage14_gate_blocks_moves passes the same record in full as its baseline"]
fn ct_lb5_01_single_letters_at_the_holders_of_522() {
    let report = check_with(BLOCKS, "s7304-moves-522.json", S7304_END, &[]);
    assert_pass(&report, "s7304-moves-522.json");
    assert!(report.trace.iter().any(|s| s.claim == "anatomy[0].moves[117].landed_units.length" && s.held));
}

// --------------------------------------------------------------------- CT-LB5-01: finite searches, out of scope

#[test]
fn ct_lb5_01_finite_searches_are_not_applicable_with_their_scope() {
    let menu = menu_record("I_a");
    let start = table(S7304_END);
    let mut names = vec![
        "s7304-t3p-plus.json".to_owned(),
        "s7304-t3p-pairs-229-a.json".to_owned(),
        "s7304-t3p-pairs-229-b.json".to_owned(),
        "s7304-t3p-scan-w3.json".to_owned(),
        "s7304-t3p-scan-w5.json".to_owned(),
        "s7304-retained.json".to_owned(),
        "s7304-retained-deep.json".to_owned(),
        "s7304-retained-check.json".to_owned(),
        "s7304-retained-deep-check.json".to_owned(),
        "s7304-peel-522.json".to_owned(),
    ];
    for (seed, _) in ENDS {
        names.push(format!("{seed}-t3p-scan.json"));
        names.push(format!("{seed}-t3p-chain-end-scan.json"));
    }
    for name in &names {
        for kind in KINDS {
            let report = run(kind, &record(name), &[("table", &start), ("menu", &menu)]);
            let Verdict::NotApplicable { reason } = &report.verdict else {
                panic!("{name} under {kind:?}: {:?}", report.verdict)
            };
            assert!(
                ["finite search", "beam search", "peel search"].iter().any(|s| reason.contains(s)),
                "{name}: {reason}"
            );
        }
    }
    // The input catalogues assert no result: no sub-checker reads them as its kind.
    for name in ["sources.json", "states.json"] {
        for kind in KINDS {
            let report = run(kind, &record(name), &[("table", &start), ("menu", &menu)]);
            assert!(matches!(report.verdict, Verdict::NotApplicable { .. }), "{name}: {:?}", report.verdict);
        }
    }
}

/// A record with one edit.
fn edited(name: &str, edit: impl FnOnce(&mut BTreeMap<String, Value>)) -> Vec<u8> {
    let mut doc = parse(&record(name));
    let Value::Object(top) = &mut doc else { panic!() };
    edit(top);
    doc.canonical().into_bytes()
}

fn first_of<'v>(top: &'v mut BTreeMap<String, Value>, key: &str) -> &'v mut BTreeMap<String, Value> {
    let Some(Value::Array(list)) = top.get_mut(key) else { panic!() };
    let Value::Object(first) = &mut list[0] else { panic!() };
    first
}

#[test]
fn ct_lb5_01_malformed_records_fail_and_are_never_unchecked() {
    let menu = menu_record("I_a");
    let start = table(S7304_END);
    let inputs: [(&str, &[u8]); 2] = [("table", &start), ("menu", &menu)];
    let cases: Vec<(&str, CheckerKind, Vec<u8>)> = vec![
        (
            "co-held groups emptied",
            BLOCKS,
            edited("s7304-coheld.json", |t| {
                t.insert("groups".into(), Value::Array(vec![]));
            }),
        ),
        (
            "co-held group key renamed",
            BLOCKS,
            edited("s7304-coheld.json", |t| {
                let g = first_of(t, "groups");
                let v = g.remove("own").unwrap();
                g.insert("owned".into(), v);
            }),
        ),
        (
            "co-held top-level key added",
            BLOCKS,
            edited("s7304-coheld.json", |t| {
                t.insert("note".into(), Value::Null);
            }),
        ),
        (
            "co-held top-level key dropped",
            BLOCKS,
            edited("s7304-coheld.json", |t| {
                t.remove("digest");
            }),
        ),
        (
            "anatomy top-level key dropped",
            BLOCKS,
            edited("s7304-jam.json", |t| {
                t.remove("phi");
            }),
        ),
        (
            "replay with a fourth key",
            LITERAL,
            edited("s7304-w5-replay.json", |t| {
                t.insert("note".into(), Value::Null);
            }),
        ),
        (
            "check after a prefix word",
            T3P,
            edited("s7304-t3p-check-w3.json", |t| {
                let letter = Value::Array(vec![Value::Int(1), Value::Int(2)]);
                t.insert("prefix_word".into(), Value::Array(vec![letter]));
            }),
        ),
        (
            "unknown stage 14 format",
            T3P,
            edited("s7304-w3-certificate.json", |t| {
                t.insert("format".into(), Value::String("t3p-certificate/2".into()));
            }),
        ),
        (
            "levels without its timings",
            LEVELS,
            edited("s7304-levels.json", |t| {
                t.remove("setup");
            }),
        ),
        (
            "formatless record of no form",
            LITERAL,
            edited("s7304-coheld.json", |t| {
                let v = t.remove("groups").unwrap();
                t.insert("units".into(), v);
            }),
        ),
        // A chain without its format has a replay record's `word`, `table` and `rows` among others: it is no replay
        // record, so the T3P sub-checker does not leave it to the literal one.
        (
            "chain without its format",
            T3P,
            edited("s7304-t3p-chain.json", |t| {
                t.remove("format");
            }),
        ),
        // A replay record with two of the peel record's fields is no peel search.
        (
            "replay with a peel's fields",
            LITERAL,
            edited("s7304-w5-replay.json", |t| {
                t.insert("keep".into(), Value::Array(vec![]));
                t.insert("first".into(), Value::Array(vec![]));
            }),
        ),
    ];
    for (what, kind, bytes) in &cases {
        let report = run(*kind, bytes, &inputs);
        assert!(report.verdict.is_fail(), "{what}: {:?}", report.verdict);
    }
    // A well-formed record of another sub-checker's form is not_applicable, naming the one that checks it.
    let report = run(T3P, &record("s7304-w5-replay.json"), &inputs);
    let Verdict::NotApplicable { reason } = &report.verdict else { panic!("{:?}", report.verdict) };
    assert!(reason.contains("s14.literal"), "{reason}");
    // A format no stage 14 form has, a damaged one or another stage's, fails at the format in every sub-checker.
    for format in ["chain/1", "t3p_chain/1", "macro-certificate/1"] {
        let other = edited("s7304-t3p-chain.json", |t| {
            t.insert("format".into(), Value::String(format.into()));
        });
        for kind in KINDS {
            let Verdict::Fail { claim, .. } = &run(kind, &other, &inputs).verdict else { panic!("{format} {kind:?}") };
            assert_eq!(claim, "format", "{format} {kind:?}");
        }
    }
}

#[test]
fn ct_lb5_01_check_after_a_prefix_word_replays_it_from_the_table() {
    // The W5 check's second round, stated after its first round's word as a prefix: the prefix is replayed letter by
    // letter from the table, and the state it reaches is the round's start.
    let w5 = parse(&record("s7304-t3p-check-w5.json"));
    let rounds = w5.get("rounds").and_then(Value::as_array).unwrap();
    let prefix: Vec<Value> = rounds[0]
        .get("replay")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|r| r.get("letter").unwrap().clone())
        .collect();
    let bytes = edited("s7304-t3p-check-w5.json", |t| {
        let Some(Value::Array(list)) = t.get_mut("rounds") else { panic!() };
        list.remove(0);
        t.insert("prefix_word".into(), Value::Array(prefix.clone()));
    });
    let menu = menu_record("I_a");
    let start = table(S7304_END);
    let report = run(T3P, &bytes, &[("table", &start), ("menu", &menu)]);
    assert!(report.complete(), "{:?}", report.verdict);
    assert!(report.trace.iter().any(|s| s.claim == "prefix_word" && s.held));
    // Another prefix reaches another state: the table with it is the wrong start.
    let mut wrong = prefix.clone();
    wrong.pop();
    let bytes = edited("s7304-t3p-check-w5.json", |t| {
        let Some(Value::Array(list)) = t.get_mut("rounds") else { panic!() };
        list.remove(0);
        t.insert("prefix_word".into(), Value::Array(wrong));
    });
    let report = run(T3P, &bytes, &[("table", &start), ("menu", &menu)]);
    assert_eq!(report.verdict.code(), Some(codes::START_TABLE_MISMATCH), "{:?}", report.verdict);
}

// ----------------------------------------------------------------------------- CT-KN6-03: stage 14 registration

#[test]
fn ct_kn6_03_stage14_registered_with_their_own_identities() {
    assert!(archive::registry_problems(archive::registry()).is_empty());
    let ids: Vec<&str> = s14::SUBCHECKERS.iter().map(|s| s.id()).collect();
    assert_eq!(ids, ["s14.t3p", "s14.literal", "s14.locality", "s14.levels", "s14.blocks"]);
    let mut sources: Vec<String> = s14::SUBCHECKERS.iter().map(|s| s.checker.sources.identity().source).collect();
    sources.sort();
    sources.dedup();
    assert_eq!(sources.len(), 5);
    for sub in s14::SUBCHECKERS {
        let named: Vec<&str> = sub.checker.sources.files.iter().map(|(n, _)| *n).collect();
        assert!(named.contains(&"c600-checkers/src/archive/s14/common.rs"), "{}", sub.id());
        assert_eq!(archive::find(sub.id()).map(|s| s.stage), Some(14));
    }
}

// ------------------------------------------------------------------------------ CT-KN6-03: stage 14 mutation gates

#[test]
fn ct_kn6_03_stage14_gate_t3p_certificate() {
    let f = archive_fixture(
        T3P,
        "s7304-w5-certificate.json",
        S7304_END,
        vec![],
        &["table_digest", "rounds[0].helpers[1][1]", "rounds[1].x", "rounds[1].helpers[0][0]"],
    );
    let report = assert_gate(&f);
    assert_eq!(report.baseline.checker.id, "s14.t3p");
}

#[test]
fn ct_kn6_03_stage14_gate_t3p_check() {
    let f = archive_fixture(
        T3P,
        "s7304-t3p-check-w3.json",
        S7304_END,
        vec![("certificate", record("s7304-w3-certificate.json"))],
        &[
            "rounds[0].F",
            "rounds[0].helpers[1].f",
            "rounds[0].helpers[0].inside_size",
            "rounds[0].intersections[0].size",
            "rounds[0].replay[2].digest",
            "rounds[0].replay[1].n4",
            "rounds[0].unit_size",
            "rounds[0].hypotheses.P4",
            "rounds[0].edges",
            "rounds[0].order",
            "rounds[0].dropped_identities",
            "digest_after",
        ],
    );
    let report = assert_gate(&f);
    for change in ["deleted rounds[0].edges", "emptied rounds[0].order", "tampered rounds[0].order[1]"] {
        assert!(report.outcomes.iter().any(|o| o.mutation == change), "{change}");
    }
}

#[test]
fn ct_kn6_03_stage14_gate_t3p_chain() {
    let mut f = archive_fixture(
        T3P,
        "s7302-t3p-chain.json",
        ENDS[0].1,
        vec![],
        &[
            "start_phi",
            "rounds[1].D",
            "rounds[0].digest_after",
            "rounds[0].order",
            "rounds[0].h",
            "rounds[0].x",
            "rounds[0].helpers",
            "rounds[1].x",
            "rounds[1].helpers[0][1]",
            "word[2][1]",
            "end_n4",
            "stop.phi",
        ],
    );
    f.skip.push("rounds[0].statuses_before".into());
    assert_gate(&f);
}

#[test]
fn ct_kn6_03_stage14_gate_literal() {
    let f = archive_fixture(LITERAL, "s7307-516-14-replay.json", ENDS[5].1, vec![], &[]);
    let mut f = f;
    f.skip.push("table".into());
    let report = assert_gate(&f);
    assert!(report.tampered() >= 18, "{}", report.tampered());
}

#[test]
fn ct_kn6_03_stage14_gate_locality() {
    let manifest = read("assets/manifest.json");
    let text = String::from_utf8(manifest.clone()).unwrap();
    let id = parse(&manifest).get("model_id").and_then(Value::as_str).unwrap().to_owned();
    let other = text.replace(&id, &sha(b"another model")).into_bytes();
    let f = Fixture {
        name: "locality-summary.json".into(),
        kind: LOCALITY,
        certificate: record("locality-summary.json"),
        inputs: vec![("model".into(), manifest)],
        claims: vec![
            "R2".into(),
            "NN".into(),
            "kappa".into(),
            "partner_count".into(),
            "equal_partner_counts".into(),
            "partner_counts[17]".into(),
            "shells[7].delta".into(),
            "shells[7].interacts".into(),
            "shells[30].grips[0]".into(),
        ],
        skip: vec![],
        wrong_start: Some(("model".into(), other)),
        optional: vec![],
    };
    assert_gate(&f);
}

#[test]
fn ct_kn6_03_stage14_gate_levels() {
    let f = archive_fixture(
        LEVELS,
        "s7304-levels.json",
        S7304_END,
        vec![],
        &["levels[540]", "histogram.3", "high_units[1].own_holders[0].level", "lambda_witness.piece"],
    );
    assert_gate(&f);
}

#[test]
fn ct_kn6_03_stage14_gate_blocks() {
    let f = archive_fixture(
        BLOCKS,
        "s7304-jam.json",
        S7304_END,
        vec![],
        &[
            "targets[1]",
            "phi",
            "anatomy[1].straddlers",
            "anatomy[1].units[0].on_cut",
            "anatomy[1].units[0].clean_pure_own[0]",
            "anatomy[1].units_with_clean_holder",
            "anatomy[1].foreign_load.h2-b1",
            "anatomy[1].clean",
        ],
    );
    let report = assert_gate(&f);
    assert_eq!(report.baseline.checker.id, "s14.blocks");
}

#[test]
fn ct_kn6_03_stage14_gate_blocks_coheld() {
    let f = archive_fixture(
        BLOCKS,
        "s7304-coheld.json",
        S7304_END,
        vec![],
        &["digest", "groups[1].holders", "groups[1].own", "groups[1].coheld_foreign", "groups[1].member_holder_rows"],
    );
    let report = assert_gate(&f);
    assert!(report.outcomes.iter().any(|o| o.mutation == "added an unknown key to the certificate"));
}

#[test]
fn ct_kn6_03_stage14_gate_blocks_end_caps() {
    let f = archive_fixture(
        BLOCKS,
        "s7304-end-caps.json",
        S7304_END,
        vec![],
        &[
            "digest",
            "caps[42].clean_covers",
            "caps[42].freed_by_one_retained_twist[0]",
            "caps[16].by_height",
            "caps[16].inside.h2-own",
            "caps[16].pure",
        ],
    );
    assert_gate(&f);
}

/// The moves record checks 118 letters in order; its gate changes the first entry and the first with a landed unit,
/// so each mutation stops early.
#[test]
fn ct_kn6_03_stage14_gate_blocks_moves() {
    let f = archive_fixture(
        BLOCKS,
        "s7304-moves-522.json",
        S7304_END,
        vec![],
        &["anatomy[0].moves[0]", "anatomy[0].moves[11].landed_units"],
    );
    let report = assert_gate(&f);
    assert!(report.baseline.trace.iter().any(|s| s.claim == "anatomy[0].moves[117].landed_units.length" && s.held));
}
