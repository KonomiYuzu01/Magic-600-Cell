//! The admissibility kind on a synthetic certificate along the W-J I_a journal: admissible letters with their moved
//! sets, a blocked letter with its straddler, the end digest and a survey of two grips.

use c600_checkers::{CheckerKind, Verdict};
use c600_jumble::q5::{Vec4, Q5};
use c600_jumble::state::{State, Status};
use c600_kernel::json::Value;

use super::common::*;

fn point(p: &Vec4) -> Value {
    Value::Array(p.iter().map(Q5::to_json).collect())
}

/// The first cap, from 0 up, whose cut a piece straddles, with its straddler (fixture construction).
fn blocked_cap(state: &mut State) -> (usize, Value) {
    for cap in 0..600 {
        let classified = state.classify(jumble(), cap, false, true, None).unwrap();
        if let Some(cert) = classified.straddle {
            let straddler = obj(vec![
                ("piece", int(cert.piece as i64)),
                ("below", point(&cert.point_below)),
                ("above", point(&cert.point_above)),
            ]);
            return (cap, straddler);
        }
    }
    panic!("no blocked cap");
}

fn synthetic() -> (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>) {
    let menu = menu("I_a");
    let letters = wj_letters();
    let start = wj_state(&menu, 110);
    let wrong = wj_state(&menu, 109);
    let mut state = start.clone();
    let mut steps = Vec::new();
    for &(cap, item) in &letters[110..116] {
        let outcome = state.apply(jumble(), &twist(&menu, cap, item), None, &[]).unwrap();
        assert_eq!(outcome.status, Status::Admissible);
        let (size, hash) = support(&outcome.inside);
        steps.push(obj(vec![
            ("letter", letter_value(cap, item)),
            ("status", text("admissible")),
            ("moved", obj(vec![("size", int(size as i64)), ("sha256", text(&hash))])),
        ]));
    }
    let (blocked, straddler) = blocked_cap(&mut state);
    steps.push(obj(vec![
        ("letter", letter_value(blocked, 12)),
        ("status", text("blocked")),
        ("straddler", straddler.clone()),
    ]));
    let next = letters[116].0;
    let survey = vec![
        obj(vec![("grip", int(next as i64)), ("status", text("admissible"))]),
        obj(vec![("grip", int(blocked as i64)), ("status", text("blocked")), ("straddler", straddler)]),
    ];
    let menu_bytes = menu_record("I_a");
    let start_bytes = state_record(&start);
    let cert = obj(vec![
        ("format", text("kn6-admissibility/1")),
        ("inputs", obj(vec![("menu", text(&sha(&menu_bytes))), ("start", text(&sha(&start_bytes)))])),
        ("menu", obj(vec![("identity", text(&menu.identity)), ("order", text(&menu.order))])),
        ("start", obj(vec![("digest", text(&start.digest(jumble())))])),
        ("steps", Value::Array(steps)),
        ("end", obj(vec![("digest", text(&state.digest(jumble())))])),
        ("survey", Value::Array(survey)),
    ]);
    (cert.canonical().into_bytes(), menu_bytes, start_bytes, state_record(&wrong))
}

#[test]
fn ct_kn6_01_core_admissibility_gate_synthetic() {
    let (cert, menu, start, wrong) = synthetic();
    let mut f = fixture(
        "synthetic admissibility",
        CheckerKind::Admissibility,
        cert,
        vec![("menu", menu), ("start", start)],
        &["inputs", "menu", "start", "steps", "end", "survey"],
        Some(("start", wrong)),
    );
    // Letters and grips are the certificate's subject, not its claims; a straddler's points are tampered in their first
    // coordinate only.
    for k in 0..7 {
        f.skip.push(format!("steps[{k}].letter"));
    }
    for k in 0..2 {
        f.skip.push(format!("survey[{k}].grip"));
    }
    for base in ["steps[6].straddler", "survey[1].straddler"] {
        for side in ["below", "above"] {
            for c in 1..4 {
                f.skip.push(format!("{base}.{side}[{c}]"));
            }
            f.skip.push(format!("{base}.{side}[0][1]"));
            f.skip.push(format!("{base}.{side}[0][2]"));
        }
    }
    // The survey is optional, and so is each of its entries; the last step is blocked and moves nothing, so the word
    // without it is another word with the same end.
    f.optional = ["survey", "survey[1]", "steps[6]"].map(String::from).to_vec();
    let report = assert_gate(&f);
    assert!(report.tampered() >= 25, "{}", report.tampered());
}

#[test]
fn ct_kn6_01_core_admissibility_status_is_recomputed() {
    let (cert, menu, start, _) = synthetic();
    let inputs = [("menu", menu.as_slice()), ("start", start.as_slice())];
    assert_eq!(run(CheckerKind::Admissibility, &cert, &inputs).verdict, Verdict::Pass);
    // The blocked letter claimed admissible: the engine's exact classification refuses it.
    let mut doc = parse(&cert);
    let Value::Object(top) = &mut doc else { unreachable!() };
    let Some(Value::Array(steps)) = top.get_mut("steps") else { unreachable!() };
    let Value::Object(step) = &mut steps[6] else { unreachable!() };
    step.remove("straddler");
    step.insert("status".into(), text("admissible"));
    step.insert("moved".into(), obj(vec![("size", int(0)), ("sha256", text(&sha(b"")))]));
    let report = run(CheckerKind::Admissibility, doc.canonical().as_bytes(), &inputs);
    assert_eq!(report.verdict.claim(), Some("steps[6].status"));
}
