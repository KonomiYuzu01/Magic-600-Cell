//! Stage 16 (six jams), read in place.
//!
//! - CT-LB5-03 (source, part: the jam descents, routing releases and transports of section 2.2 and P**'s whole
//!   descents with their check records, each from its exact table with the jam word as prefix; SJ-E1 letter by letter
//!   as a release check outside the macro family; reduced transport matching the recorded unreduced case at 7307 and
//!   never promoted to a lemma; every T1, routing, alphabet-restricted and two-carrier record keeping its scope; not
//!   run: nothing the definition names; the I-7304 and I-7306 release traces, which only ignored tests replay, are
//!   literal records of the stage, not certificates of this definition).
//! - CT-KN6-01 (source, part: the gates of `s16.macro`, `s16.release` and `s16.reductions`, with `gate::run`'s
//!   structural changes, the wrong start table for the trace family, and `s16.mobility` failing under a tampered
//!   digest and a wrong start table; not run: `s16.mobility`'s gate over its claims, which only the ignored test
//!   `ct_kn6_03_stage16_mobility_gate` runs).
//! - CT-KN6-03 (source, part: stage 16's sub-checkers registered and its self-contained records passing in place, with
//!   the label table checked against the engine; not run: the I-7304 and I-7306 traces and the mobility records of
//!   the jams other than 7304, which only ignored tests run).

use c600_checkers::archive::{self, s16};
use c600_checkers::gate::Fixture;
use c600_checkers::{codes, CheckerKind, Verdict};
use c600_kernel::json::Value;

use super::support::*;
use crate::common::*;

const MACRO: &str = "s16.macro";
const RELEASE: &str = "s16.release";
const REDUCTIONS: &str = "s16.reductions";

#[test]
fn ct_kn6_03_stage16_registry_is_clean() {
    assert!(archive::registry_problems(archive::registry()).is_empty());
    let ids: Vec<&str> = s16::SUBCHECKERS.iter().map(|s| s.id()).collect();
    for id in [MACRO, RELEASE, REDUCTIONS, MOBILITY] {
        assert!(ids.contains(&id), "{id}");
    }
}

fn certificates(rows: &[&str]) {
    for row in rows {
        certificate_and_record(
            MACRO,
            &format!("{S16}/results/certificates/{row}.json"),
            &format!("{S16}/results/certificates/{row}-check.json"),
        );
    }
}

/// The jam descents of section 2.2, each with its check record, from its exact table with the jam word as prefix.
#[test]
fn ct_lb5_03_stage16_tame_descents_7302_7304() {
    certificates(&["SJ-D1", "SJ-D2", "SJ-D3", "I-7303", "O-SJ-D9"]);
}

#[test]
fn ct_lb5_03_stage16_tame_descents_7304_7307() {
    certificates(&["I-7304", "SJ-D4", "SJ-D5", "I-7306", "O-SJ-D7", "I-7307"]);
}

#[test]
fn ct_lb5_03_stage16_tame_descent_o_sj_d8() {
    certificates(&["O-SJ-D8"]);
}

#[test]
fn ct_lb5_03_stage16_excursion_descents_7305() {
    certificates(&["O-D1", "O-D3", "O-D4"]);
}

#[test]
fn ct_lb5_03_stage16_excursion_descents_7306() {
    certificates(&["O-D5", "O-D5c", "O-D6"]);
}

fn whole(runs: &[&str]) {
    for run in runs {
        certificate_and_record(
            MACRO,
            &format!("{S16}/results/runs/PX-{run}-cert.json"),
            &format!("{S16}/results/runs/PX-{run}-check.json"),
        );
    }
}

/// P**'s whole descents from the jams.
#[test]
fn ct_lb5_03_stage16_whole_descents_7303_7305() {
    whole(&["7303", "7305"]);
}

#[test]
fn ct_lb5_03_stage16_whole_descents_7305_d8_od3() {
    whole(&["7305-D8", "7305-OD3"]);
}

#[test]
fn ct_lb5_03_stage16_whole_descents_7306_7307() {
    whole(&["7306-D7", "7307"]);
}

fn trace(name: &str) -> c600_checkers::CheckReport {
    let record = read(&format!("{S16}/results/certificates/{name}-trace.json"));
    let seed = parse(&record).get("seed").and_then(Value::as_str).unwrap().to_owned();
    check(RELEASE, &record, &inputs(&seed))
}

/// SJ-E1 letter by letter: Φ 1220 throughout, S, F and max D at 597 and 517 after every letter, ending at
/// `80bc4ec5…`. It stays a release check: as a macro certificate it is outside the macro family (LB5.11).
#[test]
fn ct_lb5_03_stage16_sj_e1_is_a_release_check() {
    let report = trace("SJ-E1");
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.complete());
    let last = report.trace.iter().rev().find(|s| s.claim == "trace[4].digest").unwrap();
    assert_eq!(last.computed, "80bc4ec5f36471fecf0bc6828f8f8f837caeeaed1e9be8579b90a7256c4e5434");
    // The same word as a one-macro certificate from the 7307 jam ends in a retained letter: not applicable.
    let record = parse(&read(&format!("{S16}/results/certificates/SJ-E1-trace.json")));
    let jams = parse(&jams());
    let row = jams.get("7307").unwrap();
    let mut prefix = Vec::new();
    for word in std::iter::once(row.get("chain").unwrap()).chain(row.get("macros").unwrap().as_array().unwrap()) {
        for letter in word.as_str().unwrap().split(',') {
            let (c, i) = letter.split_once(':').unwrap();
            prefix.push(letter_value(c.parse().unwrap(), i.parse().unwrap()));
        }
    }
    let events = record.get("word").unwrap().clone();
    let n = events.as_array().unwrap().len();
    let zero = |v: Value| Value::Array(vec![v; n]);
    let support = obj(vec![("size", int(0)), ("sha256", text(&"0".repeat(64)))]);
    let claims = obj(vec![
        ("supports", zero(support)),
        ("delta", zero(int(0))),
        ("phi", zero(int(1220))),
        ("digests", zero(text(&"0".repeat(64)))),
        ("excursion", int(0)),
        ("F", int(0)),
        ("D", int(0)),
    ]);
    let cert = obj(vec![
        ("format", text("macro-certificate/1")),
        ("table_digest", row.get("table_digest").unwrap().clone()),
        ("prefix", Value::Array(prefix)),
        ("start", obj(vec![("digest", row.get("end_digest").unwrap().clone()), ("phi", int(1220))])),
        ("excursion", int(0)),
        ("macros", Value::Array(vec![obj(vec![("events", events), ("claims", claims)])])),
    ]);
    let bytes = cert.canonical().into_bytes();
    let report = check(MACRO, &bytes, &inputs("7307"));
    assert!(not_applicable(&report).contains("LB5.11"), "{:?}", report.verdict);
    let report = run(CheckerKind::Macro, &bytes, &[("table", &table("7307")), ("menu", &menu_record("I_a"))]);
    assert!(matches!(report.verdict, Verdict::NotApplicable { .. }));
}

/// T2-cx: the conjugate at 7306 whose excursion needs the third term of Lemma CT, letter by letter.
#[test]
fn ct_lb5_03_stage16_t2_cx_trace() {
    let report = trace("T2-cx");
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.complete());
}

#[test]
#[ignore = "about a minute in a debug build (55 straddler sets); ct_lb5_03_stage16_sj_e1_is_a_release_check is the fast representative"]
fn ct_lb5_03_stage16_i_7304_trace() {
    let report = trace("I-7304");
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.complete());
}

#[test]
#[ignore = "about a minute in a debug build; ct_lb5_03_stage16_sj_e1_is_a_release_check is the fast representative"]
fn ct_lb5_03_stage16_i_7306_trace() {
    let report = trace("I-7306");
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.complete());
}

fn transport(name: &str, unreduced: bool) -> c600_checkers::CheckReport {
    let record = read(&format!("{S16}/results/transport/{name}.json"));
    let seed = parse(&record).get("seed").and_then(Value::as_str).unwrap().to_owned();
    let mut with = inputs(&seed);
    if unreduced {
        with.push(("unreduced", read(&format!("{S16}/results/transport/k2raw-7307.json"))));
    }
    check(REDUCTIONS, &record, &with)
}

/// Reduced transport matches the recorded small unreduced case (7307, K = 2), both with R1 to R3 and with
/// minimality, and each best word replays to its stated F, D and terminal Φ.
#[test]
fn ct_lb5_03_stage16_reduced_transport_matches_the_unreduced_case() {
    for name in ["k2-7307", "k2m-7307"] {
        let report = transport(name, true);
        assert_eq!(report.verdict, Verdict::Pass, "{name}");
        assert!(report.complete());
        assert!(report.trace.iter().any(|s| s.claim == "agreement" && s.computed.ends_with("no lemma")));
    }
}

/// The agreement is not promoted to a lemma: a reduced search without its unreduced counterpart is not applicable,
/// and so is the unreduced enumeration itself.
#[test]
fn ct_lb5_03_stage16_reduced_transport_is_not_a_lemma() {
    for name in ["k2-7307", "k12-7304", "k3m-7304", "k1-7302", "n2-7304"] {
        let report = transport(name, false);
        let reason = not_applicable(&report);
        assert!(!reason.is_empty(), "{name}");
    }
    let reason = not_applicable(&transport("k2-7307", false)).to_owned();
    assert!(reason.contains("not a lemma"), "{reason}");
    let raw = not_applicable(&transport("k2raw-7307", true)).to_owned();
    assert!(raw.starts_with("an unreduced transport enumeration"), "{raw}");
}

/// Leaf and alphabet negatives keep their scope: every T1, routing-with-prize, alphabet-restricted, routing and
/// two-carrier record is not applicable, with its bounds in the reason.
#[test]
fn ct_lb5_03_stage16_leaf_and_alphabet_negatives_keep_their_scope() {
    let cases = [
        ("t1/t1-7302p", "T1 leaf search"),
        ("t1/t1-7304-23-20", "T1 leaf search"),
        ("t1/t1-7307-543", "T1 leaf search"),
        ("routing/rrp-7304-41-alphabet", "never complete"),
        ("routing/rrp-7304-41-plain", "never complete"),
        ("routing/rrpE-7306", "never complete"),
        ("routing/route-7304-23-41", "routing obligation scan"),
        ("routing/route-7307-543-597", "routing obligation scan"),
        ("dis/twocarrier-7304", "two-carrier shard"),
        ("transport/n2e800-7304", "one-helper scan"),
        ("transport/k12-7304", "transport search"),
        ("runs/px-7302", "a run of the driver P**"),
        ("runs/px-7306-D7", "a run of the driver P**"),
    ];
    for (name, scope) in cases {
        let report = check(REDUCTIONS, &read(&format!("{S16}/results/{name}.json")), &[]);
        let reason = not_applicable(&report);
        assert!(reason.contains(scope), "{name}: {reason}");
    }
}

/// s16.macro's fixture: SJ-D1 (the start is the jam end; the prefix is the jam word).
fn macro_fixture() -> Fixture {
    archive_fixture(
        "SJ-D1",
        MACRO,
        read(&format!("{S16}/results/certificates/SJ-D1.json")),
        inputs("7302"),
        &[
            "table_digest",
            "start.digest",
            "start.phi",
            "macros[0].claims.supports[0].sha256",
            "macros[0].claims.delta[1]",
            "macros[0].claims.phi[1]",
            "macros[0].claims.digests[1]",
            "macros[0].claims.D",
        ],
        table("7303"),
    )
}

#[test]
fn ct_kn6_03_stage16_macro_gate() {
    assert_eq!(assert_gate(&macro_fixture()).tampered(), 8);
}

/// s16.release's fixture: SJ-E1 with Φ, δ, digests, supports and the grip observables.
fn release_fixture() -> Fixture {
    archive_fixture(
        "SJ-E1",
        RELEASE,
        read(&format!("{S16}/results/certificates/SJ-E1-trace.json")),
        inputs("7307"),
        &[
            "trace[0].phi",
            "trace[0].grips.517.maxD",
            "trace[1].digest",
            "trace[2].support",
            "trace[3].delta",
            "trace[4].grips.597.S",
            "trace[4].grips.517.F",
        ],
        table("7306"),
    )
}

#[test]
fn ct_kn6_03_stage16_release_gate() {
    assert_eq!(assert_gate(&release_fixture()).tampered(), 7);
}

/// s16.reductions' fixture: the reduced 7307 record against the unreduced one.
fn reductions_fixture() -> Fixture {
    let mut with = inputs("7307");
    with.push(("unreduced", read(&format!("{S16}/results/transport/k2raw-7307.json"))));
    archive_fixture(
        "k2-7307",
        REDUCTIONS,
        read(&format!("{S16}/results/transport/k2-7307.json")),
        with,
        &[
            "digest",
            "phi",
            "best_per_target[0].terminal_phi",
            "best_per_target[1].word[0][1]",
            "best_per_target[2].F",
            "best_per_target[3].D",
        ],
        table("7302"),
    )
}

#[test]
fn ct_kn6_03_stage16_reductions_gate() {
    assert_eq!(assert_gate(&reductions_fixture()).tampered(), 6);
}

/// s16.macro, s16.release and s16.reductions under the structural mutations (a key deleted, renamed or added, an
/// array emptied, an element dropped) at their gates' claim paths: each fails.
#[test]
#[ignore = "several minutes in a debug build (one check per mutation); the three gates above are the fast cases"]
fn ct_kn6_03_stage16_structural_mutations() {
    for f in [macro_fixture(), release_fixture(), reductions_fixture()] {
        assert_fixture_structure(&f);
    }
}

/// A record of none of the stage's forms fails at the departing field rather than answer `not_applicable`.
#[test]
fn ct_kn6_03_stage16_malformed_records_fail() {
    let mut trace = parse(&read(&format!("{S16}/results/certificates/T2-cx-trace.json")));
    let Value::Object(top) = &mut trace else { unreachable!() };
    top.insert("note".into(), Value::Null);
    let report = check(RELEASE, trace.canonical().as_bytes(), &inputs("7306"));
    assert_eq!(report.verdict.claim(), Some("record"));
    let mut mobility = parse(&read(&format!("{S16}/results/mobility/7304.json")));
    let Value::Object(top) = &mut mobility else { unreachable!() };
    top.remove("summary");
    let report = check(MOBILITY, mobility.canonical().as_bytes(), &inputs("7304"));
    assert_eq!(report.verdict.claim(), Some("record"));
    let mut transport = parse(&read(&format!("{S16}/results/transport/k2-7307.json")));
    let Value::Object(top) = &mut transport else { unreachable!() };
    top.insert("format".into(), Value::String("s16-transport/1".into()));
    assert_eq!(check(REDUCTIONS, transport.canonical().as_bytes(), &[]).verdict.claim(), Some("format"));
    let report = check(REDUCTIONS, br#"{"seed":"7304","digest":"x"}"#, &[]);
    assert_eq!(report.verdict.claim(), Some("record"));
    let report = check(REDUCTIONS, br#"{"K":1,"best_per_target":"junk","raw":true}"#, &[]);
    assert_eq!(report.verdict.claim(), Some("record"));
    let mut t1 = parse(&read(&format!("{S16}/results/t1/t1-7304-23-20.json")));
    if let Value::Object(top) = &mut t1 {
        top.remove("emax");
    }
    assert_eq!(check(REDUCTIONS, t1.canonical().as_bytes(), &[]).verdict.claim(), Some("record"));
    for id in [MACRO, RELEASE, MOBILITY, REDUCTIONS] {
        assert!(matches!(check(id, b"[]", &[]).verdict, Verdict::Fail { .. }), "{id}");
    }
}

/// A wrong table fails as the wrong start table for the trace family too, before any replay.
#[test]
fn ct_kn6_03_stage16_release_wrong_table() {
    let record = read(&format!("{S16}/results/certificates/T2-cx-trace.json"));
    let mut with = inputs("7306");
    with[0] = ("table", table("7305"));
    let report = check(RELEASE, &record, &with);
    assert_eq!(report.verdict.code(), Some(codes::START_TABLE_MISMATCH));
}

const MOBILITY: &str = "s16.mobility";

fn mobility(seed: &str) -> c600_checkers::CheckReport {
    check(MOBILITY, &read(&format!("{S16}/results/mobility/{seed}.json")), &inputs(seed))
}

/// The state of a seed's D2 end table followed by its chain word, built with direct engine calls.
fn chain_end(seed: &str) -> c600_jumble::state::State {
    let ctx = jumble();
    let menu = menu("I_a");
    let doc = parse(&table(seed));
    let posed: Vec<c600_jumble::state::Pose> = doc
        .get("poses")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|m| {
            let m = c600_jumble::q5::matrix_from_json(m).unwrap();
            match ctx.kplus.index_of_matrix(&m, &ctx.model) {
                Some(k) => c600_jumble::state::Pose::K(k),
                None => c600_jumble::state::Pose::M(std::sync::Arc::new(m)),
            }
        })
        .collect();
    let per: Vec<_> = doc
        .get("pose_id")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|id| posed[id.as_i64().unwrap() as usize].clone())
        .collect();
    let mut state = c600_jumble::levels::state_of(ctx, Some(menu.clone()), &per).unwrap();
    let catalogue = parse(&jams());
    let chain = catalogue.get(seed).and_then(|r| r.get("chain")).and_then(Value::as_str).unwrap();
    let letters: Vec<(usize, usize)> = chain
        .split(',')
        .map(|l| {
            let (c, i) = l.split_once(':').unwrap();
            (c.parse().unwrap(), i.parse().unwrap())
        })
        .collect();
    apply_all(&mut state, &menu, &letters);
    state
}

/// The full label table agrees with the engine's own classification of single cuts at a chain end (inside and
/// straddler sets at blocked and clean grips).
#[test]
fn ct_kn6_03_stage16_label_table_agrees_with_the_engine() {
    use c600_checkers::archive::s15::labels::LabelTable;
    let mut state = chain_end("7307");
    let table = LabelTable::build(jumble(), &state, "test").unwrap();
    for x in [0usize, 10, 94, 102, 266, 435, 516, 543, 597, 599] {
        let cl = state.classify(jumble(), x, true, false, None).unwrap();
        assert_eq!(table.inside[x], cl.inside.unwrap(), "inside {x}");
        let set = state.straddler_set(jumble(), x, None).unwrap();
        assert_eq!(table.straddle[x], set.pieces, "straddlers {x}");
    }
}

/// Unit mobility at the 7304 jam: 11 high units, 586 high pieces, 7 frozen and 4 carried whole, every holder and
/// level recomputed from the full label table.
#[test]
fn ct_lb5_03_stage16_mobility_7304() {
    let report = mobility("7304");
    assert_eq!(report.verdict, Verdict::Pass, "{:?}", report.verdict);
    assert!(report.complete());
}

#[test]
#[ignore = "about a minute per jam in a debug build (the full label table); ct_lb5_03_stage16_mobility_7304 is the fast representative"]
fn ct_lb5_03_stage16_mobility_other_jams() {
    for seed in ["7302", "7303", "7305", "7306", "7307"] {
        let report = mobility(seed);
        assert_eq!(report.verdict, Verdict::Pass, "{seed}: {:?}", report.verdict);
        assert!(report.complete());
    }
}

/// s16.mobility's mutation gate on the 7304 record.
#[test]
#[ignore = "several minutes in a debug build (one label table per mutation); ct_kn6_03_stage16_mobility_early_mutations runs the fast mutations"]
fn ct_kn6_03_stage16_mobility_gate() {
    let f = mobility_fixture();
    assert_eq!(assert_gate(&f).tampered(), 8);
    assert_fixture_structure(&f);
}

fn mobility_fixture() -> Fixture {
    archive_fixture(
        "mobility-7304",
        MOBILITY,
        read(&format!("{S16}/results/mobility/7304.json")),
        inputs("7304"),
        &[
            "digest",
            "units[0].size",
            "units[2].whole_holders[1]",
            "units[0].own[0].level",
            "units[0].own[0].S",
            "units[5].own[1].maxD",
            "units[2].clean_whole[0]",
            "summary.frozen_pieces",
        ],
        table("7305"),
    )
}

/// The mobility mutations that fail before the label table: a tampered digest and a wrong table.
#[test]
fn ct_kn6_03_stage16_mobility_early_mutations() {
    let record = read(&format!("{S16}/results/mobility/7304.json"));
    let mut doc = parse(&record);
    let Value::Object(top) = &mut doc else { unreachable!() };
    top.insert("digest".into(), text(&"0".repeat(64)));
    let report = check(MOBILITY, doc.canonical().as_bytes(), &inputs("7304"));
    assert_eq!(report.verdict.claim(), Some("digest"));
    let mut with = inputs("7304");
    with[0] = ("table", table("7305"));
    assert_eq!(check(MOBILITY, &record, &with).verdict.code(), Some(codes::START_TABLE_MISMATCH));
}
