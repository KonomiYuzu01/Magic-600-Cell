//! The macro kind: stage 15's `macro-certificate/1` certificates read in place, and a synthetic certificate built from
//! the jumbling engine along the W-J I_a journal.

use c600_checkers::{codes, CheckerKind, Verdict};
use c600_jumble::nu::I_A_MENU_IDENTITY;
use c600_jumble::state::Status;
use c600_kernel::json::Value;

use super::common::*;

const S15: &str = "research/jumbling/restoration/15-lowering-machine/results/certificates";
const S13: &str = "research/jumbling/restoration/13-tutorial-runner/runs/dev";

/// The stage 15 README's D2 end tables, by chain end, and the rows that start there.
const ENDS: [(&str, &str, &[&str]); 6] = [
    ("7302", "d2-s7302/component-checkpoint/tables/00021.json", &["R01"]),
    ("7303", "d2-s7303/run2/d2-s7303/tables/00138.json", &["R02", "R03"]),
    ("7304", "d2-s7304/component/tables/00005.json", &["R04", "R05", "R06"]),
    ("7305", "d2-s7305/component/tables/00010.json", &["R07", "R08", "R09", "R10", "R11", "R12"]),
    ("7306", "d2-s7306/component-checkpoint/tables/00017.json", &["R13", "R14", "R15", "R16"]),
    ("7307", "d2-s7307/component/tables/00005.json", &["R17"]),
];

fn stage15_end(index: usize) {
    let (end, table, rows) = ENDS[index];
    let table = read(&format!("{S13}/{table}"));
    let menu = menu_record("I_a");
    for row in rows {
        let cert = read(&format!("{S15}/{row}.json"));
        let report = run(CheckerKind::Macro, &cert, &[("table", &table), ("menu", &menu)]);
        assert_eq!(report.verdict, Verdict::Pass, "{row} at chain end {end}");
        assert!(report.complete());
        // The checker's own final digest is the one the archive's check record states.
        let check = parse(&read(&format!("{S15}/{row}-check.json")));
        let last = report.trace.iter().rev().find(|s| s.claim.contains(".claims.digests[")).expect("a digest step");
        assert_eq!(Some(last.computed.as_str()), check.get("digest").and_then(Value::as_str), "{row}");
    }
}

#[test]
fn ct_kn6_03_harness_stage15_macro_certificates_pass_in_place_7302() {
    stage15_end(0);
}

#[test]
fn ct_kn6_03_harness_stage15_macro_certificates_pass_in_place_7303() {
    stage15_end(1);
}

#[test]
fn ct_kn6_03_harness_stage15_macro_certificates_pass_in_place_7304() {
    stage15_end(2);
}

#[test]
fn ct_kn6_03_harness_stage15_macro_certificates_pass_in_place_7305() {
    stage15_end(3);
}

#[test]
fn ct_kn6_03_harness_stage15_macro_certificates_pass_in_place_7306() {
    stage15_end(4);
}

#[test]
fn ct_kn6_03_harness_stage15_macro_certificates_pass_in_place_7307() {
    stage15_end(5);
}

/// R01's mutation gate on the archive's own bytes: a wrong input digest, another chain end's table, and one tampered
/// leaf of every claim family (the full per-leaf gate runs on the synthetic certificate below).
#[test]
fn ct_kn6_03_harness_r01_mutation_gate_in_place() {
    let cert = read(&format!("{S15}/R01.json"));
    let table = read(&format!("{S13}/{}", ENDS[0].1));
    let other = read(&format!("{S13}/{}", ENDS[2].1));
    let claims = [
        "table_digest",
        "start.digest",
        "start.phi",
        "macros[0].claims.supports[0].size",
        "macros[0].claims.supports[2].sha256",
        "macros[0].claims.delta[2]",
        "macros[0].claims.phi[1]",
        "macros[0].claims.digests[2]",
        "macros[0].claims.F",
        "macros[0].claims.D",
        "macros[0].claims.excursion",
    ];
    let f = fixture(
        "R01",
        CheckerKind::Macro,
        cert,
        vec![("table", table), ("menu", menu_record("I_a"))],
        &claims,
        Some(("table", other)),
    );
    let report = assert_gate(&f);
    assert_eq!(report.tampered(), claims.len());
}

/// A synthetic `macro-certificate/1` from the W-J I_a journal: the table is the state after 165 letters, the prefix
/// the next letter, and the macro the two letters after it, a Φ-neutral letter and a jumble that lowers Φ. The claims
/// are computed here with direct engine calls. With `unit`, the macro names a unit the jumble lowers whole.
fn synthetic(unit: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let menu = menu("I_a");
    assert_eq!(menu.identity, I_A_MENU_IDENTITY);
    let letters = wj_letters();
    let base = wj_state(&menu, 165);
    let wrong = wj_state(&menu, 164);
    let mut state = base.clone();
    apply_all(&mut state, &menu, &letters[165..166]);
    let start = (state.digest(jumble()), phi(&state));
    let events = &letters[166..168];
    let at_start = state.clone();
    let (mut supports, mut delta, mut phis, mut digests) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut f, mut d, mut now, mut peak, mut total) = (0i64, 0i64, start.1, 0i64, 0i64);
    let mut unit_piece = None;
    for (e, &(cap, item)) in events.iter().enumerate() {
        let before = state.clone();
        let outcome = state.apply(jumble(), &twist(&menu, cap, item), None, &[]).unwrap();
        assert_eq!(outcome.status, Status::Admissible);
        let after = phi(&state);
        if e == events.len() - 1 {
            let cls = c600_jumble::observe::tables(jumble()).unwrap().cls(jumble(), cap).unwrap();
            for &p in &outcome.inside {
                let (h, b) = height(&before, p as usize);
                if h >= 2 && b != cls {
                    f += 1;
                }
                if h >= 4 && b == cls && height(&state, p as usize).0 == h - 2 {
                    d += 1;
                }
            }
            // A unit: a piece whose pose group at the macro's start lies wholly in the cap and is lowered whole.
            let mut groups = vec![Vec::new(); at_start.poses().len()];
            for p in 0..PIECES {
                groups[at_start.pose_ids()[p] as usize].push(p);
            }
            unit_piece = groups.iter().filter(|members| !members.is_empty()).find_map(|members| {
                let whole = height(&at_start, members[0]).0 >= 4
                    && members.iter().all(|&q| {
                        let h = height(&before, q).0;
                        outcome.inside.binary_search(&(q as u32)).is_ok() && h >= 4 && height(&state, q).0 == h - 2
                    });
                whole.then_some(members[0])
            });
        }
        let (size, hash) = support(&outcome.inside);
        supports.push(obj(vec![("size", int(size as i64)), ("sha256", text(&hash))]));
        delta.push(int(after - now));
        phis.push(int(after));
        digests.push(text(&state.digest(jumble())));
        total += after - now;
        peak = peak.max(total);
        now = after;
    }
    assert!(total < 0, "the synthetic macro descends");
    assert_eq!(total, 2 * (f - d), "Lemma Φ on the synthetic macro");
    let mut spec = vec![
        ("events", Value::Array(events.iter().map(|&(c, i)| letter_value(c, i)).collect())),
        (
            "claims",
            obj(vec![
                ("supports", Value::Array(supports)),
                ("delta", Value::Array(delta)),
                ("phi", Value::Array(phis)),
                ("digests", Value::Array(digests)),
                ("excursion", int(peak)),
                ("F", int(f)),
                ("D", int(d)),
            ]),
        ),
    ];
    if unit {
        spec.push(("unit_piece", int(unit_piece.expect("a unit the jumble lowers whole") as i64)));
    }
    let cert = obj(vec![
        ("format", text("macro-certificate/1")),
        ("table_digest", text(&base.digest(jumble()))),
        ("prefix", Value::Array(letters[165..166].iter().map(|&(c, i)| letter_value(c, i)).collect())),
        ("start", obj(vec![("digest", text(&start.0)), ("phi", int(start.1))])),
        ("excursion", int(0)),
        ("macros", Value::Array(vec![obj(spec)])),
    ]);
    (cert.canonical().into_bytes(), state_record(&base), state_record(&wrong))
}

#[test]
fn ct_kn6_01_core_macro_gate_synthetic() {
    let (cert, table, wrong) = synthetic(false);
    // Every scalar leaf except the certificate's subject (its format and letters) and the allowance, which a larger
    // value only weakens.
    let mut f = fixture(
        "synthetic macro",
        CheckerKind::Macro,
        cert,
        vec![("table", table), ("menu", menu_record("I_a"))],
        &[],
        Some(("table", wrong)),
    );
    f.skip = ["format", "prefix", "macros[0].events", "excursion"].map(String::from).to_vec();
    // macro-certificate/1 does not require a macro: an empty list claims nothing about one.
    f.optional.push("macros".into());
    let report = assert_gate(&f);
    // Two supports of two fields, two deltas, phis and digests, the excursion, F and D, the table digest and the
    // start's digest and Φ.
    assert_eq!(report.tampered(), 4 + 6 + 3 + 3);
}

#[test]
fn ct_kn6_01_core_macro_unit_claims() {
    let (cert, table, _) = synthetic(true);
    let inputs = [("table", table.as_slice()), ("menu", &menu_record("I_a")[..])];
    let report = run(CheckerKind::Macro, &cert, &inputs);
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.trace.iter().any(|s| s.claim == "macros[0].unit_piece" && s.computed == "lowered by 2"));
    // A unit piece that is not high fails its claim.
    let mut doc = parse(&cert);
    let Value::Object(top) = &mut doc else { unreachable!() };
    let Some(Value::Array(macros)) = top.get_mut("macros") else { unreachable!() };
    let Value::Object(spec) = &mut macros[0] else { unreachable!() };
    spec.insert("unit_piece".into(), int(0));
    let report = run(CheckerKind::Macro, doc.canonical().as_bytes(), &inputs);
    assert_eq!(report.verdict.claim(), Some("macros[0].unit_piece"));
}

#[test]
fn ct_kn6_01_core_macro_scope_and_inputs() {
    let (cert, table, _) = synthetic(false);
    let menu_a = menu_record("I_a");
    // LB5.11: a macro whose last event is a retained letter is outside the kind.
    let mut doc = parse(&cert);
    let Value::Object(top) = &mut doc else { unreachable!() };
    let Some(Value::Array(macros)) = top.get_mut("macros") else { unreachable!() };
    let Value::Object(spec) = &mut macros[0] else { unreachable!() };
    let (cap, item) = wj_letters()[166];
    spec.insert("events".into(), Value::Array(vec![letter_value(cap, item), letter_value(91, 3)]));
    let report = run(CheckerKind::Macro, doc.canonical().as_bytes(), &[("table", &table), ("menu", &menu_a)]);
    assert!(matches!(report.verdict, Verdict::NotApplicable { .. }), "{:?}", report.verdict);
    // Another format is outside the kind.
    let other = br#"{"format": "macro-check/1"}"#;
    assert!(matches!(run(CheckerKind::Macro, other, &[]).verdict, Verdict::NotApplicable { .. }));
    // No table, and another menu, fail.
    let report = run(CheckerKind::Macro, &cert, &[("menu", &menu_a)]);
    assert_eq!(report.verdict.code(), Some(codes::START_TABLE_MISMATCH));
    let report = run(CheckerKind::Macro, &cert, &[("table", &table), ("menu", &menu_record("I_b"))]);
    assert_eq!(report.verdict.claim(), Some("menu.identity"));
}
