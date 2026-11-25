//! Stepping and inspection (EV2.1, EV2.2, EV2.12; LP4-5): CT-EV2-11 and CT-LG4-07 whole, CT-EV2-02 whole, and the
//! debugger's part of CT-EV2-01 (its whole, over a command layer's digests, is `c600-tools`' `tests/debug_host.rs`).

mod support;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

use c600_debug::code::{DEBUG_WRITE_FORBIDDEN, REPLAY_DEPENDENCY_MISSING, SAFETY_STOP, STALE_DEBUG_STOP};
use c600_debug::inspect::{inspect, Class, Origin, PartialForm, SHOWN_LIMIT};
use c600_debug::session::{
    evaluate, CallResult, Coverage, Debugger, Executor, Reason, RunOptions, Source, SourceMissing, State, StepResult,
    Unavailable,
};
use c600_digest::sha256_hex;
use c600_ir::{EndTest, EndTestOutcome};
use c600_run::approx::Approx;
use c600_run::interp::Options;
use c600_run::meter::{code, Amounts, Interruption, SourceRange};
use c600_run::value::Value;
use c600_syntax::parse;
use support::*;

/// Advances `run` once from `at` under a fresh request ID.
fn advance(d: &mut Debugger, run: &str, at: &StepResult, n: &mut usize) -> StepResult {
    *n += 1;
    d.debug_step(run, &at.stop_id, &format!("{run}-{n}")).unwrap_or_else(|e| panic!("{e}"))
}

/// Advances `run` until `stop` holds of a result, which it returns, each advance under a request ID of its own.
fn until(d: &mut Debugger, run: &str, mut at: StepResult, stop: impl Fn(&StepResult) -> bool) -> StepResult {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    while !stop(&at) {
        assert!(!matches!(at.state, State::Terminal(_)), "the run ended first: {at:?}");
        let id = format!("{run}-until-{}", NEXT.fetch_add(1, Ordering::SeqCst));
        at = d.debug_step(run, &at.stop_id, &id).unwrap_or_else(|e| panic!("{e}"));
    }
    at
}

fn text(range: &SourceRange) -> &'static str {
    assert_eq!(range.digest, sha256_hex(SOURCE.as_bytes()), "a range of the executed module");
    &SOURCE[range.start..range.end]
}

fn mapped_texts(result: &StepResult) -> Vec<&'static str> {
    match &result.source {
        Source::Mapped(ranges) => ranges.iter().map(text).collect(),
        other => panic!("no source: {other:?}"),
    }
}

/// CT-EV2-11 (whole): duplicate step requests across a suspended kernel call neither dispatch nor charge it again;
/// completion stops at the continuation with the call's result. Stale stops are `EV2:stale.debug_stop`.
#[test]
fn ct_ev2_11_a_suspended_call_is_dispatched_and_charged_once() {
    let (lowered, loaded) = build(&[SOURCE]);
    let (quick, _) = Engine::new();
    let mut quick = quick;
    let plain = loaded.run("m", "main", vec![Value::int(5)], &mut quick, &Options::new(WIDE)).unwrap();

    let (mut engine, seen) = Engine::new();
    let (release, waiting) = mpsc::channel();
    engine.release = Some(waiting);
    let mut d = Debugger::new();
    let first = open(&mut d, "r", "main", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    assert!(matches!(first.state, State::Stopped { completed: None }));
    assert_eq!((first.position.step, first.stop_id.as_str()), (0, "0/entry#read"));

    // Advance to the host row: the advance that dispatches it answers `waiting(call)` at the awaiting stop, with the
    // call site's source and no return value.
    let mut n = 0;
    let mut at = first.clone();
    let (before, waiting) = loop {
        let next = advance(&mut d, "r", &at, &mut n);
        if let State::Waiting { .. } = next.state {
            break (at, next);
        }
        at = next;
    };
    let dispatcher = format!("r-{n}");
    let State::Waiting { call } = &waiting.state else { unreachable!() };
    let call: u128 = call.parse().unwrap();
    assert_eq!(waiting.stop_id, format!("{call}/await"));
    assert_eq!(waiting.position.step, call);
    assert_eq!(call, before.position.step + 1);
    assert_eq!(mapped_texts(&waiting), ["read_snapshot<Nat>(\"pieces\")", "probe()"]);
    assert!(waiting.values.is_empty(), "probe has no locals and the call no value yet");
    assert_eq!(seen.performed.load(Ordering::SeqCst), 1);

    // Duplicates return the latest result and advance nothing; a new ID naming the awaiting stop joins the call.
    for _ in 0..3 {
        assert_eq!(d.debug_step("r", &before.stop_id, &dispatcher).unwrap(), waiting);
    }
    assert_eq!(d.debug_step("r", &waiting.stop_id, "join").unwrap(), waiting);
    // A new ID naming an obsolete stop, an ID reused with another stop and an ID reused with another run are stale.
    assert_eq!(d.debug_step("r", &first.stop_id, "fresh").unwrap_err().code, STALE_DEBUG_STOP);
    assert_eq!(d.debug_step("r", &first.stop_id, &dispatcher).unwrap_err().code, STALE_DEBUG_STOP);
    let (other, _) = Engine::new();
    let elsewhere = open(&mut d, "s", "pure", vec![Value::int(1)], other, mapped(&lowered, WIDE), &loaded);
    assert_eq!(d.debug_step("s", &elsewhere.stop_id, &dispatcher).unwrap_err().code, STALE_DEBUG_STOP);
    assert_eq!(seen.performed.load(Ordering::SeqCst), 1, "nothing dispatched again");

    // Completion stops at the continuation with the call's result.
    release.send(()).unwrap();
    let done = d.wait("r").unwrap();
    let State::Stopped { completed: Some(CallResult { call: c, result: Ok(reference) }) } = &done.state else {
        panic!("{done:?}")
    };
    assert_eq!(c, &call.to_string());
    assert_eq!(done.position.step, call, "the continuation is the position after the call's row");
    let result = d.inspect_value("r", reference).unwrap();
    assert!(matches!(result.value, Value::Int(ref n) if **n == 40.into()));
    // The dispatching request and the one that joined answer the continuation; a late ID naming the completed call's
    // awaiting stop joins it too. Still one dispatch.
    assert_eq!(d.debug_step("r", &before.stop_id, &dispatcher).unwrap(), done);
    assert_eq!(d.debug_step("r", &waiting.stop_id, "join").unwrap(), done);
    assert_eq!(d.debug_step("r", &waiting.stop_id, "late").unwrap(), done);
    assert_eq!(seen.performed.load(Ordering::SeqCst), 1);

    let end = until(&mut d, "r", done, |r| matches!(r.state, State::Terminal(_)));
    let State::Terminal(terminal) = &end.state else { unreachable!() };
    assert_eq!(terminal.used, plain.summary.used, "stepping and waiting charge nothing of their own");
    assert_eq!(d.profile("r").unwrap().used, plain.summary.used);
    assert_eq!(seen.performed.load(Ordering::SeqCst), 1);
    // A request at the end changes nothing.
    assert_eq!(d.debug_step("r", "end", "after").unwrap().state, end.state);
}

/// CT-EV2-11 (whole): a missing source map, missing source bytes and a missing step mapping stay distinct.
#[test]
fn ct_ev2_11_missing_source_map_bytes_and_step_mapping_stay_distinct() {
    let (lowered, loaded) = build(&[SOURCE]);
    let inside = |r: &StepResult| r.position.at.starts_with("m.main:0 > m.add:");
    let mut d = Debugger::new();

    let (engine, _) = Engine::new();
    let first = open(&mut d, "mapped", "main", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    let mapped_stop = until(&mut d, "mapped", first, inside);
    assert!(matches!(mapped_stop.source, Source::Mapped(_)));

    let (engine, _) = Engine::new();
    let first = open(&mut d, "no-map", "main", vec![Value::int(5)], engine, RunOptions::new(WIDE), &loaded);
    let unmapped = until(&mut d, "no-map", first, inside);
    assert_eq!(unmapped.source, Source::Unavailable(SourceMissing::SourceMap));

    let (engine, _) = Engine::new();
    let mut no_bytes = target("main");
    no_bytes.sources.clear();
    let execute = evaluate(
        loaded.clone(),
        "m".into(),
        "main".into(),
        vec![Value::int(5)],
        Box::new(engine),
        mapped(&lowered, WIDE),
    );
    let first = d.open("no-bytes", no_bytes, execute).unwrap();
    let bytes_missing = until(&mut d, "no-bytes", first, inside);
    assert_eq!(bytes_missing.source, Source::Unavailable(SourceMissing::SourceBytes));
    assert_eq!(bytes_missing.position, mapped_stop.position, "the same stop, without its bytes");

    let (engine, seen) = Engine::new();
    let mut process = target("main");
    process.executor = Executor::Unmapped("runtime process".into());
    let execute = evaluate(
        loaded.clone(),
        "m".into(),
        "main".into(),
        vec![Value::int(5)],
        Box::new(engine),
        mapped(&lowered, WIDE),
    );
    let none = d.open("process", process, execute).unwrap();
    assert_eq!(none.state, State::Unavailable(Unavailable::StepMappingMissing));
    let again = d.debug_step("process", &none.stop_id, "p1").unwrap();
    assert_eq!(again.state, State::Unavailable(Unavailable::StepMappingMissing));
    assert_eq!(seen.performed.load(Ordering::SeqCst), 0, "nothing ran");

    let kinds: BTreeSet<String> =
        [&mapped_stop.source, &unmapped.source, &bytes_missing.source].iter().map(|s| format!("{s:?}")).collect();
    assert_eq!(kinds.len(), 3);
    // An unknown run or value reference is a missing dependency, not an empty answer.
    assert_eq!(d.debug_step("absent", "0/entry#read", "x").unwrap_err().code, REPLAY_DEPENDENCY_MISSING);
    assert_eq!(d.inspect_value("mapped", "9/none#0").unwrap_err().code, REPLAY_DEPENDENCY_MISSING);
}

/// CT-EV2-11 (whole): every terminal reason stays distinct, with the meter's real use and its coverage; none is an
/// empty negative.
#[test]
fn ct_ev2_11_every_terminal_reason_stays_distinct() {
    let (lowered, loaded) = build(&[SOURCE]);
    let mut d = Debugger::new();
    let run_out = |d: &mut Debugger, run: &str, first: StepResult| {
        until(d, run, first, |r| matches!(r.state, State::Terminal(_)))
    };
    let mut ends = Vec::new();

    let (engine, _) = Engine::new();
    let first = open(&mut d, "done", "pure", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    let done = run_out(&mut d, "done", first);
    let value = done.values.iter().find(|v| v.name == "result").expect("the run's value");
    let shown = d.inspect_value("done", &value.reference).unwrap();
    assert_eq!(shown.shown.text, "{\"int\":\"14\"}");
    ends.push(done);

    let (engine, _) = Engine::new();
    let limits = Amounts { steps: 6, ..WIDE };
    let first = open(&mut d, "cut", "pure", vec![Value::int(5)], engine, mapped(&lowered, limits), &loaded);
    ends.push(run_out(&mut d, "cut", first));

    let (mut engine, _) = Engine::new();
    engine.refuse = true;
    let first = open(&mut d, "error", "main", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    let error = run_out(&mut d, "error", first);
    ends.push(error);

    let (engine, _) = Engine::new();
    let first = open(&mut d, "cancel", "pure", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    let mut n = 0;
    let second = advance(&mut d, "cancel", &first, &mut n);
    let third = advance(&mut d, "cancel", &second, &mut n);
    ends.push(d.cancel("cancel", Interruption::ByUser).unwrap());
    assert_eq!(third.position.step, 2);

    let (engine, _) = Engine::new();
    let first = open(&mut d, "safety", "pure", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    let second = advance(&mut d, "safety", &first, &mut n);
    let third = advance(&mut d, "safety", &second, &mut n);
    let safety = d.safety_stop("safety").unwrap();
    // The safety-stopped run stays ended: a later step, even its old stop, does not resume it.
    assert_eq!(d.debug_step("safety", &third.stop_id, "late").unwrap_err().code, STALE_DEBUG_STOP);
    assert_eq!(d.debug_step("safety", "end", "late-end").unwrap().state, safety.state);
    ends.push(safety);

    let got: Vec<_> = ends
        .iter()
        .map(|r| match &r.state {
            State::Terminal(t) => (t.reason, t.code, t.coverage, t.used.steps),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        got,
        [
            (Reason::Completed, None, Coverage::Complete, got[0].3),
            (Reason::CutOff, Some(code::STEPS), Coverage::WithinBudget, 6),
            (Reason::Error, Some(code::NOT_GRANTED), Coverage::Open, got[2].3),
            (Reason::Cancelled, Some(code::CANCELLED_BY_USER), Coverage::Open, 2),
            (Reason::SafetyStop, Some(SAFETY_STOP), Coverage::Open, 2),
        ]
    );
    assert!(got.iter().all(|g| g.3 > 0), "every end carries the meter's real use: {got:?}");
    let reasons: BTreeSet<String> = got.iter().map(|g| format!("{:?}{:?}", g.0, g.1)).collect();
    assert_eq!(reasons.len(), 5);
    // Only a completed run has a value; the others name their stop.
    assert!(ends[1..].iter().all(|r| r.values.is_empty()));
    assert!(matches!(&ends[1].source, Source::Mapped(r) if r.len() == 1));
    assert_eq!(d.profile("safety").unwrap().stop, Some((SAFETY_STOP, "safety_stop")));
}

/// CT-LG4-07 (whole): stepping and output linkage resolve the executed module's source, compiled call sites
/// included, while an invalid draft stays separate.
#[test]
fn ct_lg4_07_stepping_resolves_the_executed_modules_source() {
    let (lowered, loaded) = build(&[SOURCE]);
    let mut d = Debugger::new();
    let (engine, _) = Engine::new();
    let first = open(&mut d, "r", "main", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    let entry = until(&mut d, "r", first, |r| r.position.at.starts_with("m.main:0 > m.add:"));
    assert_eq!(mapped_texts(&entry), ["x", "add(a, 2)"]);
    let deeper = until(&mut d, "r", entry, |r| r.position.at.ends_with("#exact_add"));
    assert_eq!(mapped_texts(&deeper), ["exact_add(x, y)", "add(a, 2)"]);

    // Output linkage: a run cut off inside the callee names the executed module's range of its stop.
    let (engine, _) = Engine::new();
    let limits = Amounts { steps: deeper.position.step, ..WIDE };
    let first = open(&mut d, "cut", "main", vec![Value::int(5)], engine, mapped(&lowered, limits), &loaded);
    let end = until(&mut d, "cut", first, |r| matches!(r.state, State::Terminal(_)));
    assert_eq!(mapped_texts(&end), ["exact_add(x, y)"]);

    // An invalid draft of the same module stays separate: it has no lowering and no map, its digest is not the
    // executed one, and a session holding only its bytes cannot show the executed ranges in its text.
    let draft = SOURCE.replace("let s = exact_add(x, y);", "let s = exact_add(x, y)");
    assert!(!parse(draft.as_bytes()).diagnostics.is_empty());
    let Source::Mapped(ranges) = &deeper.source else { unreachable!() };
    assert!(ranges.iter().all(|r| r.digest != sha256_hex(draft.as_bytes())));
    let mut drafted = target("main");
    drafted.sources.clear();
    drafted.sources.insert(sha256_hex(draft.as_bytes()), std::sync::Arc::from(draft.as_bytes()));
    let (engine, _) = Engine::new();
    let execute = evaluate(
        loaded.clone(),
        "m".into(),
        "main".into(),
        vec![Value::int(5)],
        Box::new(engine),
        mapped(&lowered, WIDE),
    );
    let first = d.open("draft", drafted, execute).unwrap();
    let in_draft = until(&mut d, "draft", first, |r| r.position.at.ends_with("#exact_add"));
    assert_eq!(in_draft.position, deeper.position);
    assert_eq!(in_draft.source, Source::Unavailable(SourceMissing::SourceBytes));
}

/// CT-EV2-02 (whole): exact values, declared display precision, partial results and unknown verdicts stay distinct;
/// a cut display says so, and an inspection keeps the value's identity and level.
#[test]
fn ct_ev2_02_inspection_keeps_value_classes_apart() {
    let (lowered, loaded) = build(&[SOURCE]);
    let mut d = Debugger::new();
    let (engine, _) = Engine::new();
    let first = open(&mut d, "r", "main", vec![Value::int(5)], engine, mapped(&lowered, WIDE), &loaded);
    let entry = until(&mut d, "r", first, |r| r.position.at.starts_with("m.main:0 > m.add:"));
    let x = entry.values.iter().find(|v| v.name == "x").expect("add's parameter");
    let seen = d.inspect_value("r", &x.reference).unwrap();
    assert_eq!((seen.ty.as_str(), &seen.class, seen.shown.text.as_str()), ("Int", &Class::Exact, "{\"int\":\"5\"}"));
    assert_eq!(seen.origin.function.as_deref(), Some("add"));
    assert_eq!(seen.origin.name, "x");

    let origin =
        || Origin { stop: "s".into(), name: "v".into(), slot: None, module: None, function: None, source: None };
    let input = pinned();
    let exact = inspect(&Value::int(3), origin(), &input);
    let approx =
        inspect(&Value::Approx(Approx { precision: 53, negative: false, mantissa: 3, exponent: 0 }), origin(), &input);
    let partial =
        inspect(&Value::record(vec![("result", Value::tag("partial")), ("n", Value::int(3))]), origin(), &input);
    let total = inspect(&Value::record(vec![("result", Value::tag("total")), ("n", Value::int(3))]), origin(), &input);
    let unknown = inspect(&Value::EndTest(EndTest::StickerTest, EndTestOutcome::Unestablished), origin(), &input);
    let fails = inspect(&Value::EndTest(EndTest::StickerTest, EndTestOutcome::Fails), origin(), &input);
    assert_eq!(exact.class, Class::Exact);
    assert_eq!(approx.class, Class::Approximate { precision: 53 });
    assert_eq!(approx.ty, "Approx<53>");
    assert_eq!(partial.class, Class::Partial(PartialForm::Partial));
    assert_eq!(total.class, Class::Partial(PartialForm::Total));
    assert_eq!(unknown.class, Class::Verdict { test: "sticker_test", outcome: "unestablished" });
    assert_ne!(unknown.class, fails.class, "an unestablished verdict is never a failure");
    let classes: BTreeSet<String> =
        [&exact, &approx, &partial, &unknown].iter().map(|i| format!("{:?}", i.class)).collect();
    assert_eq!(classes.len(), 4);

    // A cut display says it is cut.
    let long = inspect(&Value::text(&"é".repeat(SHOWN_LIMIT)), origin(), &input);
    assert!(long.shown.truncated && long.shown.text.len() <= SHOWN_LIMIT);
    assert!(!exact.shown.truncated);
    // A word keeps the pinned identity at level `labels`; a number carries none.
    let word = inspect(&Value::Word(std::sync::Arc::from(&[1, -2, 3][..])), origin(), &input);
    assert_eq!((word.identity.as_ref(), word.level.as_deref()), (Some(&identity()), Some("labels")));
    assert_eq!((exact.identity, exact.level), (None, None));
}

/// CT-EV2-01 (the debugger's part): stepping over a workspace write, a file write, a view and a preview refuses each
/// with `EV2:capability.debug_write_forbidden` at its site; the host behind the debugger never sees one.
#[test]
fn ct_ev2_01_part_stepping_over_effect_sites_writes_nothing() {
    let (lowered, loaded) = build(&[SOURCE]);
    let mut d = Debugger::new();
    for (run, function, arg) in [
        ("save", "save", Value::Word(std::sync::Arc::from(&[1, 2][..]))),
        ("show", "show", Value::int(1)),
        ("preview", "preview", Value::Word(std::sync::Arc::from(&[1, 2][..]))),
    ] {
        let (engine, seen) = Engine::new();
        let first = open(&mut d, run, function, vec![arg], engine, mapped(&lowered, WIDE), &loaded);
        let end = until(&mut d, run, first, |r| matches!(r.state, State::Terminal(_)));
        let State::Terminal(t) = &end.state else { unreachable!() };
        assert_eq!((t.reason, t.code), (Reason::Error, Some(DEBUG_WRITE_FORBIDDEN)), "{run}");
        assert!(seen.sinks.lock().unwrap().is_empty(), "{run}: a sink reached the engine");
        let performed = seen.performed.load(Ordering::SeqCst);
        assert_eq!(performed, usize::from(run == "show"), "{run}: only the snapshot read passes");
    }
}
