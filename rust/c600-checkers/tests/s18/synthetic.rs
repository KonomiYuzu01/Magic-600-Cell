//! Gates 2 and 3 and the lattice replay on synthetic records: the archive's own steps take minutes each to replay, so
//! the fast mutation gates of `s18.composed`, `s18.tail_replay` and `s18.lattice_replay` run on short records built
//! here from the engine (reading B: a check of the record contract on a synthetic fixture, whose outputs enter no
//! research evidence). The archive's mutation controls (stage 18 README section 2.2) are replayed on the same records.

use c600_checkers::archive::s18;
use c600_checkers::gate;
use c600_checkers::{codes, Verdict};
use c600_jumble::state::State;
use c600_kernel::json::Value;

use super::fixtures::*;
use crate::common::*;

fn gate_on(name: &str, kind: c600_checkers::CheckerKind, s: &Synthetic) {
    let f = gate_fixture(name, kind, s.record.clone(), s.inputs.clone(), &[], &[], ("start", s.wrong_start.clone()));
    let report = gate::run(&engines(), &f).expect("a checker");
    assert!(report.passed(), "{name}: {:#?}", report.problems);
    assert!(report.tampered() >= 29, "{name}: {} leaves", report.tampered());
    // A member deleted, renamed or added, a list emptied or an element dropped: each fails.
    assert!(structural_gate(name, kind, &s.record, &s.inputs, &[], &[]) >= 60);
}

#[test]
fn ct_lb5_05_gate2_composed_mutation_gate_on_a_synthetic_four_segment_record() {
    let s = synthetic(true);
    let report = check_owned(composed(), &s.record, &s.inputs);
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace);
    // The second step's start is derived by exact replay under this checker, never presented as a stored table.
    let start = report.trace.iter().find(|t| t.claim == "steps[1].start").expect("a start step");
    assert!(start.computed.starts_with("derived: ") && start.computed.contains("LP5-3"), "{}", start.computed);
    let strict = report.trace.iter().find(|t| t.claim == "steps[0].strict_prime").expect("strict′");
    assert!(strict.computed.contains("collateral 0"), "{}", strict.computed);
    assert!(report.trace.iter().any(|t| t.claim == "steps[0].v.safe_all"));
    assert!(report.trace.iter().any(|t| t.claim == "steps[0].v_inverse.safe_all"));
    gate_on("synthetic four-segment, gate 2", composed(), &s);
}

#[test]
fn ct_lb5_05_gate3_tail_replay_mutation_gate_on_a_synthetic_three_segment_record() {
    let s = synthetic(false);
    let report = check_owned(tail_replay(), &s.record, &s.inputs);
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace);
    let scope = report.trace.iter().find(|t| t.claim == "scope").expect("a scope step");
    assert_eq!(scope.computed, "gate 3 for steps[0] to steps[1]");
    let history = report.trace.iter().find(|t| t.claim == "steps[0].history").unwrap();
    assert!(history.computed.contains("a new check"), "{}", history.computed);
    let chunk = report.trace.iter().find(|t| t.claim == "steps[0].chunks[0]").and_then(|t| t.span.clone()).unwrap();
    assert_eq!((chunk.from, chunk.to), (0, 5));
    assert!(report.trace.iter().any(|t| t.claim == "steps[1].chunks.cover" && t.held));
    gate_on("synthetic three-segment, gate 3", tail_replay(), &s);
}

#[test]
fn ct_lb5_05_gate1_expansion_mutation_gate_on_a_synthetic_record() {
    let s = synthetic(true);
    assert_eq!(check_owned(expansion(), &s.record, &s.inputs).verdict, Verdict::Pass);
    // Gate 1 cannot see a state, so the last end digest is a gate-2 claim; every other leaf is gate 1's.
    let f = gate_fixture(
        "synthetic, gate 1",
        expansion(),
        s.record.clone(),
        s.inputs.clone(),
        &[],
        &["steps[1].end_digest"],
        ("start", s.wrong_start.clone()),
    );
    let report = gate::run(&engines(), &f).expect("a checker");
    assert!(report.passed(), "{:#?}", report.problems);
    structural_gate("synthetic, gate 1", expansion(), &s.record, &s.inputs, &[], &["steps[1].end_digest"]);
}

/// Edits the synthetic record and returns its bytes.
fn edit(record: &[u8], change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut doc = parse(record);
    change(&mut doc);
    doc.canonical().into_bytes()
}

fn set(doc: &mut Value, step: usize, key: &str, value: Value) {
    if let Value::Object(map) = doc {
        if let Some(Value::Array(steps)) = map.get_mut("steps") {
            if let Value::Object(step) = &mut steps[step] {
                step.insert(key.to_owned(), value);
            }
        }
    }
}

fn remove(doc: &mut Value, step: usize, key: &str) {
    if let Value::Object(map) = doc {
        if let Some(Value::Array(steps)) = map.get_mut("steps") {
            if let Value::Object(step) = &mut steps[step] {
                step.remove(key);
            }
        }
    }
}

#[test]
fn ct_lb5_05_archive_mutation_controls_fail_and_the_record_passes() {
    // The controls of the archive's own gate (stage 18 README section 2.2): an empty unit, a lattice piece in a unit,
    // wrong counts, a removed preparation, two adjacent letters of V′ swapped, the other step form and a wrong start
    // table must each be rejected, and the unmutated record accepted.
    let s = synthetic(true);
    let l = letters();
    let both = [expansion(), composed()];
    for kind in both {
        assert_eq!(check_owned(kind, &s.record, &s.inputs).verdict, Verdict::Pass);
    }
    let fails = |kind, record: &[u8], inputs: &[(String, Vec<u8>)]| {
        let report = check_owned(kind, record, inputs);
        assert!(report.verdict.is_fail(), "{:?}", report.verdict);
        report.verdict
    };
    // An empty unit, and a lattice piece in a unit (the size kept right by dropping a real member).
    fails(expansion(), &s.record, &replace(&s.inputs, "01-a/unit.json", b"{\"pieces\": []}".to_vec()));
    let unit = parse(&s.inputs.iter().find(|(r, _)| r == "01-a/unit.json").unwrap().1);
    let mut pieces: Vec<i64> =
        unit.get("pieces").and_then(Value::as_array).unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
    let lattice = (0..PIECES as i64).find(|p| {
        !pieces.contains(p) && {
            let start = parse(&s.inputs.iter().find(|(r, _)| r == "start").unwrap().1);
            State::from_record(jumble(), Some(ia()), &start).unwrap().is_lattice_piece(*p as usize)
        }
    });
    pieces[0] = lattice.unwrap();
    let unit = format!("{{\"pieces\": [{}]}}", pieces.iter().map(i64::to_string).collect::<Vec<_>>().join(", "));
    let verdict = fails(composed(), &s.record, &replace(&s.inputs, "01-a/unit.json", unit.into_bytes()));
    assert_eq!(verdict.claim(), Some("steps[0].unit"));
    // Wrong counts.
    for (key, delta) in [("off_before", 1), ("off_after", -1), ("letters", 2), ("X", 1), ("V", 1), ("unit_size", 1)] {
        let record = edit(&s.record, |doc| {
            let value = parse(&s.record).get("steps").and_then(|v| v.as_array().map(|a| a[0].clone())).unwrap();
            set(doc, 0, key, int(value.get(key).and_then(Value::as_i64).unwrap() + delta));
        });
        fails(expansion(), &record, &s.inputs);
    }
    // Counts whose length X + 2 + copies·V overflows fail the length claim; they never panic or wrap.
    let record = edit(&s.record, |doc| {
        set(doc, 0, "X", int(i64::MAX));
        set(doc, 0, "V", int(i64::MAX));
    });
    assert_eq!(fails(expansion(), &record, &s.inputs).claim(), Some("steps[0].letters"));
    // A removed preparation: no x file, X = 0 and no hash, the length adjusted; W then claims to start where X ended.
    let record = edit(&s.record, |doc| {
        set(doc, 0, "X", int(0));
        set(doc, 0, "letters", int(2 + 2 * 2));
        remove(doc, 0, "x_sha256");
    });
    let inputs: Vec<_> = s.inputs.iter().filter(|(r, _)| r != "01-a/x.json.gz").cloned().collect();
    assert_eq!(fails(expansion(), &record, &inputs).claim(), Some("steps[0].w.input_digest"));
    // Two adjacent letters of V′ swapped: the stored hash no longer holds.
    let w_out = parse(&s.inputs.iter().find(|(r, _)| r == "01-a/w.json").unwrap().1);
    let w_out = w_out.get("output_digest").and_then(Value::as_str).unwrap().to_owned();
    let swapped = format!("{{\"word\": {}, \"input_digest\": \"{w_out}\"}}\n", dumps(&[l.v[1], l.v[0]]));
    let verdict = fails(expansion(), &s.record, &replace(&s.inputs, "01-a/v.json.gz", gzip_stored(swapped.as_bytes())));
    assert_eq!(verdict.claim(), Some("steps[0].v.sha256"));
    // The other step form: the counts no longer add up; with them adjusted, the shortened chain is not accepted
    // without rechecking its changed input, and gate 2 finds the changed end.
    let record = edit(&s.record, |doc| {
        if let Value::Object(map) = doc {
            map.insert("form".to_owned(), text("three-segment"));
        }
    });
    assert_eq!(fails(expansion(), &record, &s.inputs).claim(), Some("steps[0].letters"));
    let record = edit(&record, |doc| {
        for step in 0..2 {
            set(doc, step, "letters", int(1 + 2 + 2));
        }
    });
    assert_eq!(check_owned(expansion(), &record, &s.inputs).verdict, Verdict::Pass);
    let verdict = fails(composed(), &record, &s.inputs);
    assert_eq!(verdict.claim(), Some("steps[0].end_digest"));
    // A wrong start table.
    for kind in both {
        let verdict = fails(kind, &s.record, &replace(&s.inputs, "start", s.wrong_start.clone()));
        assert_eq!(verdict.code(), Some(codes::START_TABLE_MISMATCH));
    }
}

#[test]
fn ct_lb5_05_gate3_historically_incomplete_steps_are_never_passed() {
    // A record whose first step is T6's step 4 or a rule stage answers not_applicable for gate 3, read in place and
    // before any replay: the archive stopped those replays at the freeze, and no check here completes them.
    let rule = read(&format!("{RULE}/steps.json"));
    let report = check_owned(tail_replay(), &rule, &chain_inputs(RULE, false));
    match &report.verdict {
        Verdict::NotApplicable { reason } => {
            assert!(reason.contains(s18::tail_replay::HISTORICAL_CHECK_INCOMPLETE), "{reason}");
            for name in ["rule stage 1", "rule stage 2", "rule stage 3"] {
                assert!(reason.contains(name), "{reason}");
            }
        }
        other => panic!("{other:?}"),
    }
    // The record is read in its exact form first: a malformed rule record fails, it does not answer not_applicable.
    let inputs = chain_inputs(RULE, false);
    assert!(structural_gate("rule-stop steps.json, gate 3", tail_replay(), &rule, &inputs, &["steps[1]"], &[]) > 30);
    // T6's step 4 alone, as a one-step record from its recorded start.
    let t6 = parse(&read(&format!("{T6}/steps.json")));
    let step4 = t6.get("steps").and_then(Value::as_array).unwrap()[3].clone();
    let start = step4.get("start_digest").cloned().unwrap();
    let record = obj(vec![("start_digest", start), ("steps", Value::Array(vec![step4]))]).canonical().into_bytes();
    let inputs: Vec<_> = chain_inputs(T6, false)
        .into_iter()
        .filter(|(r, _)| r == "menu" || r == "start" || r.starts_with("04-91-14/"))
        .collect();
    let report = check_owned(tail_replay(), &record, &inputs);
    assert!(
        matches!(&report.verdict, Verdict::NotApplicable { reason } if reason.contains("T6 step 4")),
        "{:?}",
        report.verdict
    );
    // The T6 record's gate 3 covers steps 1 to 3 only. The scope is decided before any replay; here a wrong start
    // table stops the check right after it.
    let inputs = replace(&chain_inputs(T6, false), "start", pinned(BEFORE_STOP_TABLE, BEFORE_STOP_TABLE_SHA256));
    let report = check_owned(tail_replay(), &read(&format!("{T6}/steps.json")), &inputs);
    assert_eq!(report.verdict.code(), Some(codes::START_TABLE_MISMATCH));
    let scope = report.trace.iter().find(|t| t.claim == "scope").expect("a scope step");
    assert_eq!(
        scope.computed,
        "gate 3 for steps[0] to steps[2]; not applicable: steps[3] = T6 step 4 (04-91-14): historical_check_incomplete"
    );
}

#[test]
fn ct_lb5_05_lattice_replay_mutation_gate_on_a_synthetic_run() {
    // The run record's own form with two orbit steps cut from the first lattice word's first 60 letters, played here
    // from the stored start table (T6's end); the rest of the record (its phases, acceptance and configuration) is the
    // archive's.
    let menu = ia();
    let table = pinned(T6_END_TABLE, T6_END_TABLE_SHA256);
    let word = c600_kernel::archive::gunzip(&read(&format!("{RUN}/words/00000.ia16.gz")), 1 << 24).unwrap();
    let mut state = table_state(&table, &menu);
    assert_eq!(state.digest(jumble()), T6_END_DIGEST);
    let mut doc = parse(&pinned(&format!("{RUN}/run.json"), RUN_SHA256));
    let templates: Vec<Value> = doc.get("steps").and_then(Value::as_array).unwrap()[..2].to_vec();
    let mut steps = Vec::new();
    let mut inputs = vec![("menu".to_owned(), menu_record("I_a")), ("start-table.json".to_owned(), table.clone())];
    let mut before = T6_END_DIGEST.to_owned();
    for (k, range) in [(0usize, 0..30usize), (1, 30..60)] {
        let bytes = word[2 * range.start..2 * range.end].to_vec();
        for pair in bytes.chunks_exact(2) {
            let code = u16::from_le_bytes([pair[0], pair[1]]) as usize;
            apply(&mut state, &menu, (code / 60, code % 60));
        }
        let after = state.digest(jumble());
        let point = |digest: &str| obj(vec![("digest", text(digest)), ("off", int(0)), ("heights", obj(vec![]))]);
        let file = format!("words/p{k}.ia16");
        let mut step = templates[k].clone();
        set_at(&mut step, "word", text(&file));
        set_at(&mut step, "letters", int(30));
        set_at(&mut step, "word_sha256", text(&sha(&bytes)));
        set_at(&mut step, "before", point(&before));
        set_at(&mut step, "after", point(&after));
        steps.push(step);
        inputs.push((file, bytes));
        before = after;
    }
    set_at(&mut doc, "steps", Value::Array(steps));
    set_at(&mut doc, "phases.lattice.steps", int(2));
    set_at(&mut doc, "phases.lattice.twists", int(60));
    set_at(&mut doc, "totals.lattice_twists", int(60));
    let record = doc.canonical().into_bytes();
    let report = check_owned(lattice_replay(), &record, &inputs);
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace);
    let fresh = report.trace.iter().find(|t| t.claim == "fresh_check").unwrap();
    assert!(fresh.computed.contains("check.jsonl is not in the repository"), "{}", fresh.computed);
    // Every leaf but the producer's configuration and the timings, which are not claims, and the claims s18.lattice_run
    // checks against the members this replay does not read: the trees (and the last factor count, which only its tree
    // fixes), the report and the bulk phase's end table by file.
    let skip = [
        "config",
        "steps[0].seconds",
        "steps[1].seconds",
        "totals.seconds",
        "steps[0].tree",
        "steps[1].tree",
        "steps[1].factor_count",
        "phases.lattice.report",
        "phases.bulk.end.table",
    ];
    let wrong = read(&format!("{RUN}/tables/00052.json"));
    let f = gate_fixture(
        "synthetic lattice run",
        lattice_replay(),
        record.clone(),
        inputs.clone(),
        &[],
        &skip,
        ("start-table.json", wrong),
    );
    let report = gate::run(&engines(), &f).expect("a checker");
    assert!(report.passed(), "{:#?}", report.problems);
    assert!(structural_gate("synthetic lattice run", lattice_replay(), &record, &inputs, &[], &skip) > 200);
}

#[test]
fn ct_lb5_05_malformed_records_fail_and_only_unchecked_records_are_not_applicable() {
    // A record that departs from a sub-checker's form fails, naming the field; only the records the plan leaves
    // unchecked (static-tail units, whole-cap outputs) answer not_applicable.
    let s = synthetic(true);
    let chain = [expansion(), composed(), tail_replay()];
    for kind in chain {
        for key in ["steps", "start_digest"] {
            let record = edit(&s.record, |doc| {
                if let Value::Object(map) = doc {
                    map.remove(key);
                }
            });
            let report = check_owned(kind, &record, &s.inputs);
            assert!(report.verdict.is_fail(), "{kind:?} without {key}: {:?}", report.verdict);
        }
        for bytes in [&b"not a record"[..], b"[]", b"{}", b"ELIGIBILITY short"] {
            let report = check_owned(kind, bytes, &s.inputs);
            assert!(report.verdict.is_fail(), "{kind:?}: {:?}", report.verdict);
        }
    }
    // Each sub-checker's form against the other's: a steps.json record is no run record and a run record no
    // steps.json record, each failing at its format.
    let run = pinned(&format!("{RUN}/run.json"), RUN_SHA256);
    for kind in chain {
        assert_eq!(check_owned(kind, &run, &[]).verdict.claim(), Some("format"), "{kind:?}");
    }
    for kind in [lattice_run(), lattice_replay()] {
        assert_eq!(check_owned(kind, &s.record, &[]).verdict.claim(), Some("format"), "{kind:?}");
    }
}
