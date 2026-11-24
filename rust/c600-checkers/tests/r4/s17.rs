//! Stage 17 (routing in the driver), read in place.
//!
//! - CT-LB5-04 (source, part: every `s16-rr/1` release search, `s16-pxrr/1` run and separate stop record answering
//!   not applicable with its own bounds, so a bounded stop refutes no word beyond them, and the family-R stop at 7304
//!   keeping P*'s unbeamed scope; `s17.rr` refusing a word or grip other than the recorded gate inputs before any
//!   search; not run: the RR canonical-word and five-tail gates against brute force at the recorded inputs H2 and
//!   H2b, which only the ignored tests `ct_lb5_04_stage17_rr_gate_h2` and `ct_lb5_04_stage17_rr_gate_h2b` run, with
//!   the label calculus checked against the engine in the ignored test
//!   `ct_kn6_03_stage17_label_calculus_agrees_with_the_engine`).
//! - CT-KN6-01 (source, part: `s17.macro`'s gate on RR-7304, with `gate::run`'s structural changes, and `s17.rr`
//!   failing under a wrong start table, a tampered Φ and a changed word, grip or statement; not run: `s17.rr`'s gate
//!   over its claims on H2b, which only the ignored test `ct_kn6_03_stage17_rr_gate` runs).
//! - CT-KN6-03 (source, part: stage 17's sub-checkers registered and the chosen P*** descents and RR-7304 passing in
//!   place with their check records; not run: the RR gate records H2 and H2b, which only ignored tests check).

use c600_checkers::archive::s15::grips::sfd;
use c600_checkers::archive::s15::labels::LabelTable;
use c600_checkers::archive::s17::rr::{Calculus, GATE};
use c600_checkers::archive::{self, s17};
use c600_checkers::gate::Fixture;
use c600_checkers::replay::{tree_tables, Jumbling};
use c600_checkers::{codes, Verdict};
use c600_kernel::json::Value;

use super::support::*;
use crate::common::*;

const MACRO: &str = "s17.macro";
const RR: &str = "s17.rr";
const PSTAR: &str = "s15.pstar";
const TEST_RR: &str = "research/jumbling/restoration/17-routing-in-driver/scripts/test_rr.py";

#[test]
fn ct_kn6_03_stage17_registry_is_clean() {
    assert!(archive::registry_problems(archive::registry()).is_empty());
    let ids: Vec<&str> = s17::SUBCHECKERS.iter().map(|s| s.id()).collect();
    assert_eq!(ids, [MACRO, RR]);
    for sub in s17::SUBCHECKERS {
        assert!(std::ptr::eq(archive::find(sub.id()).unwrap(), *sub));
    }
}

fn certificates(names: &[&str]) {
    for name in names {
        certificate_and_record(
            MACRO,
            &format!("{S17}/certificates/{name}.json"),
            &format!("{S17}/certificates/{name}-check.json"),
        );
    }
}

/// The chosen descents of P*** (PX3) and RR-7304 with their check records, through the macro kind from the jams.
#[test]
fn ct_lb5_04_stage17_px3_7302p_7303_7304() {
    certificates(&["PX3-7302p", "PX3-7303", "PX3-7304"]);
}

#[test]
fn ct_lb5_04_stage17_px3_7305() {
    certificates(&["PX3-7305", "PX3-7305-D8", "PX3-7305-OD3"]);
}

#[test]
fn ct_lb5_04_stage17_px3_7306_7307p() {
    certificates(&["PX3-7306", "PX3-7306-D7", "PX3-7307p"]);
}

#[test]
fn ct_lb5_04_stage17_rr_7304() {
    certificates(&["RR-7304"]);
}

fn macro_fixture() -> Fixture {
    archive_fixture(
        "RR-7304",
        MACRO,
        read(&format!("{S17}/certificates/RR-7304.json")),
        inputs("7304"),
        &[
            "table_digest",
            "start.digest",
            "start.phi",
            "macros[0].claims.digests[3]",
            "macros[0].claims.delta[7]",
            "macros[0].claims.F",
            "macros[0].claims.D",
        ],
        table("7303"),
    )
}

/// s17.macro's mutation gate on RR-7304.
#[test]
fn ct_kn6_03_stage17_macro_gate() {
    assert_eq!(assert_gate(&macro_fixture()).tampered(), 7);
}

/// The bounded records keep their bounds: every RR release search, P*** run and transport stop is
/// `not_applicable`, with its own bounds in the reason, and the family-R stop at 7304 keeps P*'s unbeamed scope.
#[test]
fn ct_lb5_04_stage17_bounded_records_keep_their_bounds() {
    let reason = |id: &'static str, path: &str| not_applicable(&check(id, &read(path), &[])).to_owned();
    let rr = reason(RR, &format!("{S17}/rr-7304-23-41.json"));
    assert!(rr.contains("family RR") && rr.contains("carrier actions 1 to 3") && rr.contains("pairs 23:41"), "{rr}");
    let mut names: Vec<String> = ["rr-n2-134-172", "rr-n2-463-458", "rr-n2-other"].map(String::from).to_vec();
    for entry in std::fs::read_dir(repository().join(format!("{S17}/stop-7304-438/rr-n3"))).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        names.push(format!("rr-n3/{}", name.trim_end_matches(".json")));
    }
    assert_eq!(names.len(), 22);
    for name in &names {
        let reason = reason(RR, &format!("{S17}/stop-7304-438/{name}.json"));
        assert!(reason.contains("a negative for these bounds only, not for all words"), "{name}: {reason}");
        assert!(reason.contains("after a prefix of 17 letters"), "{name}: {reason}");
    }
    for name in [
        "pxrr-7302p",
        "pxrr-7303",
        "pxrr-7304",
        "pxrr-7305",
        "pxrr-7305-D8",
        "pxrr-7305-OD3",
        "pxrr-7306",
        "pxrr-7306-D7",
        "pxrr-7307p",
    ] {
        let reason = reason(RR, &format!("{S17}/runs/{name}.json"));
        assert!(reason.starts_with("a run of the driver P***") && reason.contains("status"), "{name}: {reason}");
    }
    for (name, k) in [("x1", "K 1, minimal false"), ("x2", "K 2, minimal false"), ("x3-minimal", "K 3, minimal true")] {
        let reason = reason(RR, &format!("{S17}/stop-7304-438/{name}.json"));
        assert!(reason.starts_with("a transport stop of family X") && reason.contains(k), "{name}: {reason}");
    }
    let stop = reason(PSTAR, &format!("{S17}/stop-7304-438/r.json"));
    assert!(stop.contains("unbeamed stop") && stop.contains("[\"R\"]") && stop.contains("not for all words"), "{stop}");
}

/// A record of none of the stage's forms fails at the departing field rather than answer `not_applicable`.
#[test]
fn ct_kn6_03_stage17_malformed_records_fail() {
    let base = parse(&read(&format!("{S17}/runs/pxrr-7306.json")));
    let edited = |f: &dyn Fn(&mut std::collections::BTreeMap<String, Value>)| {
        let mut doc = base.clone();
        if let Value::Object(top) = &mut doc {
            f(top);
        }
        doc.canonical().into_bytes()
    };
    let unknown = edited(&|top| {
        top.insert("format".into(), Value::String("s16-pxrr/9".into()));
    });
    assert_eq!(check(RR, &unknown, &[]).verdict.claim(), Some("format"));
    let missing = edited(&|top| {
        top.remove("segments");
    });
    assert_eq!(check(RR, &missing, &[]).verdict.claim(), Some("record"));
    let extra = edited(&|top| {
        top.insert("note".into(), Value::Null);
    });
    assert_eq!(check(RR, &extra, &[]).verdict.claim(), Some("record"));
    assert_eq!(check(MACRO, &extra, &inputs("7306")).verdict.claim(), Some("format"));
}

// ------------------------------------------------------------------------------------------------- the RR gate

/// The 7306 jam's configuration followed by `letters`, built with direct engine calls.
fn jam_end(seed: &str, letters: &[(usize, usize)]) -> c600_jumble::state::State {
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
    let row = catalogue.get(seed).unwrap();
    let mut word: Vec<&str> = row.get("chain").and_then(Value::as_str).unwrap().split(',').collect();
    for m in row.get("macros").and_then(Value::as_array).unwrap() {
        word.extend(m.as_str().unwrap().split(','));
    }
    let mut all: Vec<(usize, usize)> = word
        .iter()
        .map(|l| {
            let (c, i) = l.split_once(':').unwrap();
            (c.parse().unwrap(), i.parse().unwrap())
        })
        .collect();
    all.extend_from_slice(letters);
    apply_all(&mut state, &menu, &all);
    state
}

/// The label calculus agrees with the engine at H2's configuration and after two retained letters there: S, F and
/// max D at the routing grips and at grips around them, and the clean grips everywhere.
#[test]
#[ignore = "about a minute in a debug build (two full label tables); ct_kn6_03_stage17_rr_early_mutations is the fast representative"]
fn ct_kn6_03_stage17_label_calculus_agrees_with_the_engine() {
    let ctx = jumble();
    let menu = menu("I_a");
    let state = jam_end("7306", &[(561, 1), (598, 4)]);
    let tables = tree_tables(ctx, "test").unwrap();
    let j = Jumbling::new(ctx, menu.clone(), state.clone());
    let labels = LabelTable::build(ctx, &j.state, "test").unwrap();
    let mut calc = Calculus::new(&j, tables, &labels, "test").unwrap();
    let mut probe = Jumbling::new(ctx, menu, state);
    let grips = [536usize, 598, 561, 233, 240, 90, 370, 351, 384, 485, 526, 0, 299, 599];
    let compare = |calc: &mut Calculus, probe: &mut Jumbling| {
        for x in grips {
            let (s, f, d) = calc.metric(x).unwrap();
            let engine = sfd(probe, tables, x, "test").unwrap();
            assert_eq!((u64::from(s), u64::from(f), u64::from(d)), engine, "grip {x}");
        }
    };
    compare(&mut calc, &mut probe);
    for x in 0..600 {
        assert_eq!(calc.clean(x), labels.clean(x), "grip {x}");
    }
    for (g, i) in [(536, 11), (233, 6)] {
        calc.apply(g, i).unwrap();
        let twist = probe.twist(g, i, "test").unwrap();
        assert_eq!(probe.apply(&twist, "test").unwrap().status, c600_jumble::state::Status::Admissible);
    }
    let after = LabelTable::build(ctx, &probe.state, "test").unwrap();
    for x in 0..600 {
        assert_eq!(calc.clean(x), after.clean(x), "grip {x} after two letters");
    }
    compare(&mut calc, &mut probe);
    calc.undo();
    calc.undo();
    for x in 0..600 {
        assert_eq!(calc.clean(x), labels.clean(x), "grip {x} after undo");
    }
}

/// The gate inputs as `test_rr.py` records them: D7, H2's routing word and released grip, H2b's prefix, its last
/// letter and released grip, the jam's Φ and the lowering at 370 right after H2b's release. Each line is read from the
/// test in place, so a certificate here states nothing the test does not.
fn gate_lines() -> String {
    let source = String::from_utf8(read(TEST_RR)).unwrap();
    for line in [
        "D7 = [(561, 1), (598, 4), (233, 6), (536, 11), (370, 15)]  # O-SJ-D7 as certified in stage 16",
        "expect(phi0 == 1490, 'the 7306 jam has phi 1490')",
        "expect(walker.go(D7[:2]) is not None and rs.ns[536] == 0, 'H2 the routing word releases 536')",
        "expect(brute and found == brute, f'H2 tails {found} brute force {brute}')",
        "pre = [D7[0], D7[1], D7[3], D7[2], (240, 3)]",
        "expect(walker.go(pre) is not None and rs.ns[90] > 0, 'H2b 90 is blocked')",
        "expect(walker.go(pre + [(240, 9)]) is not None and rs.ns[90] == 0, 'H2b 240:9 releases 90')",
        "expect(found.get(370) == (1268, 1), f'H2b the lowering at 370 right after the release: {found}')",
        "expect(found == brute, f'H2b tails {found} brute force {brute}')",
        "state = C.load(7306, save_labels=tmp/'jam7306.npz')",
    ] {
        assert!(source.contains(line), "test_rr.py no longer states: {line}");
    }
    source
}

fn gate_certificate(word: &[(usize, usize)], released: usize, stated: Option<(usize, i64, usize)>) -> Vec<u8> {
    let letters = Value::Array(
        word.iter().map(|&(c, i)| Value::Array(vec![Value::Int(c as i64), Value::Int(i as i64)])).collect(),
    );
    let stated = match stated {
        None => Value::Null,
        Some((target, phi, length)) => Value::Object(
            [
                ("target".to_owned(), Value::Int(target as i64)),
                ("best".to_owned(), Value::Array(vec![Value::Int(phi), Value::Int(length as i64)])),
            ]
            .into_iter()
            .collect(),
        ),
    };
    let doc = Value::Object(
        [
            ("format", text(GATE)),
            ("seed", text("7306")),
            ("word", letters),
            ("released", Value::Int(released as i64)),
            ("phi", Value::Int(1490)),
            ("lowers", Value::Bool(true)),
            ("stated", stated),
            ("agree", Value::Bool(true)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect(),
    );
    doc.canonical().into_bytes()
}

const D7: [(usize, usize); 5] = [(561, 1), (598, 4), (233, 6), (536, 11), (370, 15)];

/// H2: at the 7306 jam after D7[:2] (536 released).
fn h2() -> Vec<u8> {
    gate_lines();
    gate_certificate(&D7[..2], 536, None)
}

/// H2b: after D7[0], D7[1], D7[3], D7[2], 240:3 (90 blocked), then 240:9 (90 released).
fn h2b() -> Vec<u8> {
    gate_lines();
    let word = [D7[0], D7[1], D7[3], D7[2], (240, 3), (240, 9)];
    gate_certificate(&word, 90, Some((370, 1268, 1)))
}

/// H2: the five tails give the brute force's best (terminal Φ, length) at every target.
#[test]
#[ignore = "about two minutes in a debug build (the label table and the brute force); ct_kn6_03_stage17_rr_early_mutations is the fast representative"]
fn ct_lb5_04_stage17_rr_gate_h2() {
    let report = check(RR, &h2(), &inputs("7306"));
    assert_eq!(report.verdict, Verdict::Pass, "{:?}", report.trace.last());
    assert!(report.complete());
    assert!(report.trace.iter().any(|s| s.claim == "agree" && s.held));
}

/// H2b: 90 blocked before 240:9 and released after it; the lowering at 370 at once reaches Φ 1268; the tails agree.
#[test]
#[ignore = "a few minutes in a debug build (the brute force at H2b); ct_kn6_03_stage17_rr_early_mutations is the fast representative"]
fn ct_lb5_04_stage17_rr_gate_h2b() {
    let report = check(RR, &h2b(), &inputs("7306"));
    assert_eq!(report.verdict, Verdict::Pass, "{:?}", report.trace.last());
    assert!(report.trace.iter().any(|s| s.claim == "stated" && s.held && s.computed == "370:[1268,1]"));
    assert!(report.trace.iter().any(|s| s.claim == "released.before" && s.held));
}

fn rr_fixture() -> Fixture {
    archive_fixture(
        "H2b",
        RR,
        h2b(),
        inputs("7306"),
        &["phi", "released", "lowers", "stated.target", "stated.best", "agree"],
        table("7305"),
    )
}

/// s17.rr's mutation gate on H2b, and the structural mutations at its claim paths.
#[test]
#[ignore = "about half an hour in a debug build (the brute force once per mutation); ct_kn6_03_stage17_rr_early_mutations runs the fast ones"]
fn ct_kn6_03_stage17_rr_gate() {
    let f = rr_fixture();
    assert_eq!(assert_gate(&f).tampered(), 7);
    assert_fixture_structure(&f);
}

/// The RR gate's mutations that fail before any search: a wrong table, a tampered Φ, a word that does not release
/// its grip, and a letter that is not retained.
#[test]
fn ct_kn6_03_stage17_rr_early_mutations() {
    let mut with = inputs("7306");
    with[0] = ("table", table("7305"));
    assert_eq!(check(RR, &h2(), &with).verdict.code(), Some(codes::START_TABLE_MISMATCH));
    let mut doc = parse(&h2());
    if let Value::Object(top) = &mut doc {
        top.insert("phi".into(), Value::Int(1492));
    }
    assert_eq!(check(RR, doc.canonical().as_bytes(), &inputs("7306")).verdict.claim(), Some("phi"));
    // Only the recorded inputs are checked: another word or released grip is refused before any replay.
    let report = check(RR, &gate_certificate(&D7[..1], 536, None), &inputs("7306"));
    assert_eq!(report.verdict.claim(), Some("word"));
    let report = check(RR, &gate_certificate(&[(561, 1), (598, 14)], 536, None), &inputs("7306"));
    assert_eq!(report.verdict.claim(), Some("word"));
    let report = check(RR, &gate_certificate(&D7[..2], 538, None), &inputs("7306"));
    assert_eq!(report.verdict.claim(), Some("released"));
    let mut doc = parse(&h2b());
    if let Value::Object(top) = &mut doc {
        top.insert("stated".into(), Value::Object(Default::default()));
    }
    assert_eq!(check(RR, doc.canonical().as_bytes(), &inputs("7306")).verdict.claim(), Some("stated"));
}

/// s17.macro under the structural mutations at its gate's claim paths.
#[test]
#[ignore = "about a minute in a debug build (one check per mutation); ct_kn6_03_stage17_macro_gate is the fast gate"]
fn ct_kn6_03_stage17_macro_structural_mutations() {
    assert_fixture_structure(&macro_fixture());
}
