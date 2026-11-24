//! Deterministic replay and the cost profile (EV2.3, EV2.4, EV2.7, EV2.9; LP4-6): CT-EV2-03, CT-EV2-04, CT-EV2-05 and
//! CT-EV2-08 whole, and CT-EV2-10 whole under the replay contract the owner settled on 23 November 2026 (chapter 09's
//! DET.7 as amended under PHS.52, and overlay OV-1 for chapter 05's part).

mod support;

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use c600_debug::code::{
    DEBUG_WRITE_FORBIDDEN, IDENTITY_MISMATCH, REPLAY_DEPENDENCY_MISSING, REPLAY_MISMATCH, SAFETY_STOP,
};
use c600_debug::profile::{Alphabet, WordCost};
use c600_debug::replay::{Comparison, Event, Kind, Recorded, Replayer};
use c600_debug::session::{Coverage, Debugger, State};
use c600_debug::Identity;
use c600_digest::sha256_hex;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::{code, Amounts, Outcome};
use c600_run::value::Value;
use support::*;

fn replayer() -> Replayer {
    let mut replayer = Replayer::new(identity());
    replayer.hold(SOURCE.as_bytes());
    replayer
}

fn word() -> Value {
    Value::Word(Arc::from(&[3, -7][..]))
}

/// CT-EV2-03 (whole): a complete reproducible record matches its exact bytes, outcome, coverage and charges on
/// repeated headless replays.
#[test]
fn ct_ev2_03_a_complete_record_matches_on_repeated_replays() {
    let (mut engine, _) = Engine::new();
    let (record, run) = capture("main-5", "main", vec![Value::int(5)], WIDE, &mut engine);
    assert_eq!(run.output.as_deref(), Some(&b"{\"int\":\"47\"}\n"[..]));
    let replayer = replayer();
    let first = replayer.replay_run(&record).unwrap();
    assert_eq!(first.comparison, Comparison::Match);
    assert_eq!(first.outcome, Some(Outcome::Completed));
    assert_eq!(first.kind, Kind::Reexecution, "a run with reads and no effects is re-executed whole");
    assert_eq!(first.profile.used, run.summary.used);
    assert_eq!(record.result.coverage, Coverage::Complete);
    assert!(first.evidence_references.contains(&format!("module:{}", sha256_hex(SOURCE.as_bytes()))));
    for _ in 0..3 {
        let again = replayer.replay_run(&record).unwrap();
        assert_eq!(again.comparison, Comparison::Match);
        assert_eq!(again.profile, first.profile);
        assert_eq!(again.evidence_references, first.evidence_references);
    }
    // A cut-off record keeps its coverage and matches as a cut-off.
    let (mut engine, _) = Engine::new();
    let (cut, _) = capture("cut", "main", vec![Value::int(5)], Amounts { steps: 9, ..WIDE }, &mut engine);
    assert_eq!((cut.result.outcome, cut.result.coverage), (Outcome::CutOff, Coverage::WithinBudget));
    assert_eq!(replayer.replay_run(&cut).unwrap().comparison, Comparison::Match);
}

/// CT-EV2-04 (whole): a missing module and another model or a reversed menu order report their own blockers, and no
/// replacement file is evaluated.
#[test]
fn ct_ev2_04_preflight_reports_its_blockers() {
    let (mut engine, _) = Engine::new();
    let (record, _) = capture("main-5", "main", vec![Value::int(5)], WIDE, &mut engine);
    let digest = sha256_hex(SOURCE.as_bytes());

    // A missing module, with a file of the same module name and other bytes held: still missing, by its digest.
    let mut replacing = Replayer::new(identity());
    replacing.hold(SOURCE.replace("exact_add(b, probe())", "exact_add(probe(), b)").as_bytes());
    for replayer in [Replayer::new(identity()), replacing] {
        let blocked = replayer.replay_run(&record).unwrap_err();
        assert_eq!(blocked.code, REPLAY_DEPENDENCY_MISSING);
        assert!(blocked.detail.contains(&digest), "{}", blocked.detail);
    }

    // Another model.
    let mut other = replayer();
    other.identity.model = "model-2".into();
    let blocked = other.replay_run(&record).unwrap_err();
    assert_eq!(blocked.code, IDENTITY_MISMATCH);
    assert!(blocked.detail.contains("model_id") && blocked.detail.contains("model-2"), "{}", blocked.detail);

    // A jumbling record whose menu order is reversed on this side.
    let menu = |order: &str| Identity {
        puzzle: "600-cell-Jumble".into(),
        model: "model-1".into(),
        menu: Some("menu-1".into()),
        menu_order: Some(order.into()),
        contract: Some("contract-1".into()),
    };
    let mut jumbled = record.clone();
    jumbled.key.input.identity = menu("order-ab");
    let mut reversed = replayer();
    reversed.identity = menu("order-ba");
    let blocked = reversed.replay_run(&jumbled).unwrap_err();
    assert_eq!(blocked.code, IDENTITY_MISMATCH);
    assert!(blocked.detail.starts_with("the record's identity differs in menu_order"), "{}", blocked.detail);

    // Other blockers keep their own names: an edition, cost units, a level.
    let mut edition = record.clone();
    edition.key.edition = "6cs-draft-0".into();
    assert_eq!(replayer().replay_run(&edition).unwrap_err().code, REPLAY_DEPENDENCY_MISSING);
    let mut units = record.clone();
    units.key.cost_units = "measured-1".into();
    assert!(replayer().replay_run(&units).unwrap_err().detail.contains("measured-1"));
    // The record is unchanged by every refusal and stays inspectable.
    assert_eq!(record.result.output.as_deref(), Some(&b"{\"int\":\"47\"}\n"[..]));
}

/// CT-EV2-05 (whole, synthetic): a planted difference reports `EV2:evidence.replay_mismatch` with its field and
/// position; a safety-stopped record stays incomplete, with no negative claim.
#[test]
fn ct_ev2_05_planted_differences_and_a_safety_stop() {
    let (mut engine, _) = Engine::new();
    let (record, _) = capture("main-5", "main", vec![Value::int(5)], WIDE, &mut engine);
    let replayer = replayer();
    let mismatch = |planted: &c600_debug::replay::RunRecord| match replayer.replay_run(planted).unwrap().comparison {
        Comparison::Mismatch(m) => m,
        other => panic!("{other:?}"),
    };

    let mut output = record.clone();
    output.result.output = Some(b"{\"int\":\"48\"}\n".to_vec());
    let m = mismatch(&output);
    assert_eq!((m.code, m.field), (REPLAY_MISMATCH, "output"));
    let verdict = replayer.replay_run(&output).unwrap().verdict().unwrap_err();
    assert_eq!(verdict.code, REPLAY_MISMATCH);

    let mut value = record.clone();
    value.result.value = Some(Value::int(48));
    assert_eq!(mismatch(&value).field, "result");

    let mut rows = record.clone();
    rows.result.rows[3].kernel_cost += 1;
    let m = mismatch(&rows);
    assert_eq!(m.field, "rows");
    assert!(m.position.as_deref().unwrap().starts_with("row 3 at "), "{m:?}");

    let mut used = record.clone();
    used.result.used.steps += 1;
    assert_eq!(mismatch(&used).field, "used");

    // A differing captured input changes the computation, and the result differs from the record.
    let mut input = record.clone();
    for event in &mut input.events {
        if let Event::Perform { answer, .. } = event {
            *answer = Ok(Value::int(41));
        }
    }
    assert_eq!(mismatch(&input).field, "result");
    // A trace that names another request differs at that event.
    let mut trace = record.clone();
    if let Event::Tariff { request: c600_debug::replay::RequestKey::Snapshot { query, .. }, .. } = &mut trace.events[0]
    {
        *query = "edges".into();
    }
    let m = mismatch(&trace);
    assert_eq!((m.field, m.position.as_deref()), ("trace", Some("event 0")));

    // A wall-clock safety stop: incomplete, not run, and no negative claim.
    let mut stopped = record.clone();
    stopped.result = Recorded::safety_stopped(Amounts { steps: 7, ..Amounts::default() });
    let replay = replayer.replay_run(&stopped).unwrap();
    assert_eq!(replay.comparison, Comparison::Incomplete { code: SAFETY_STOP });
    assert_eq!((replay.outcome, replay.kind), (None, Kind::NotRun));
    assert_eq!(replay.verdict().unwrap_err().code, SAFETY_STOP);
    assert_eq!(replay.profile.used.steps, 7);
    assert_eq!(replay.profile.stop, Some((SAFETY_STOP, "safety_stop")));
    assert_eq!(stopped.result.coverage, Coverage::Open);
}

/// An engine that computes `pieces` once and answers later rows from its cache; its tariff is declared either way.
struct Caching {
    cache: HashMap<String, Value>,
    computed: usize,
    delay: Option<Duration>,
}

impl Host for Caching {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if let Some(delay) = self.delay {
            std::thread::sleep(delay);
        }
        let Request::Snapshot { query, .. } = request else { return Err(code::NOT_GRANTED) };
        if let Some(v) = self.cache.get(*query) {
            return Ok(v.clone());
        }
        self.computed += 1;
        let v = Value::int(40);
        self.cache.insert((*query).to_owned(), v.clone());
        Ok(v)
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

/// CT-EV2-08 (whole): cached and uncached runs have the same deterministic charges, and elapsed seconds never change
/// the stop step.
#[test]
fn ct_ev2_08_cached_and_slow_runs_charge_and_stop_the_same() {
    let mut engine = Caching { cache: HashMap::new(), computed: 0, delay: None };
    let (uncached, _) = capture("first", "main", vec![Value::int(5)], WIDE, &mut engine);
    let (cached, _) = capture("first", "main", vec![Value::int(5)], WIDE, &mut engine);
    assert_eq!(engine.computed, 1, "the second run was answered from the cache");
    assert_eq!(cached.result.used, uncached.result.used);
    assert_eq!(cached.result.rows, uncached.result.rows);
    let replayer = replayer();
    assert_eq!(replayer.replay_run(&cached).unwrap().profile, replayer.replay_run(&uncached).unwrap().profile);

    // A budget that cuts the run off after the host row: a slow host and a quick one stop at the same step, with the
    // same profile; only the elapsed display time differs.
    let (lowered, loaded) = build(&[SOURCE]);
    let limits = Amounts { kernel_cost: uncached.result.used.kernel_cost - 1, ..WIDE };
    let mut d = Debugger::new();
    let mut ends = Vec::new();
    for (run, delay) in [("quick", None), ("slow", Some(Duration::from_millis(40)))] {
        let engine = Caching { cache: HashMap::new(), computed: 0, delay };
        let execute = c600_debug::session::evaluate(
            loaded.clone(),
            "m".into(),
            "main".into(),
            vec![Value::int(5)],
            Box::new(engine),
            mapped(&lowered, limits),
        );
        let mut at = d.open(run, target("main"), execute).unwrap();
        let mut n = 0;
        while !matches!(at.state, State::Terminal(_)) {
            n += 1;
            at = d.debug_step(run, &at.stop_id, &format!("{run}{n}")).unwrap();
        }
        let profile = d.profile(run).unwrap();
        ends.push((at, profile));
    }
    let (quick, slow) = (&ends[0], &ends[1]);
    let State::Terminal(t) = &quick.0.state else { unreachable!() };
    assert_eq!(t.code, Some(code::KERNEL_COST));
    assert_eq!((&quick.0.position, &quick.0.state), (&slow.0.position, &slow.0.state), "the same stop step and stop");
    let same_budget = c600_debug::profile::CostProfile { budget: quick.1.budget.clone(), ..slow.1.clone() };
    assert_eq!(quick.1, same_budget, "the same profile under each run's own budget name, elapsed time aside");
    assert!(slow.1.elapsed.unwrap() >= Duration::from_millis(40));
    assert_eq!(quick.1.stop, Some((code::KERNEL_COST, "kernel_cost")));
}

/// CT-EV2-10 (whole): replay inspects a record's effects without executing them, refuses an effect the record does
/// not hold instead of writing, and tells effect inspection from a complete re-execution (DET.7 as amended).
#[test]
fn ct_ev2_10_effects_are_inspected_never_executed() {
    let replayer = replayer();

    let (mut engine, seen) = Engine::new();
    let (saved, _) = capture("save", "save", vec![word()], WIDE, &mut engine);
    assert_eq!(seen.sinks.lock().unwrap().len(), 2, "the recorded run wrote twice");
    let replay = replayer.replay_run(&saved).unwrap();
    assert_eq!(replay.comparison, Comparison::Match);
    assert_eq!(replay.kind, Kind::EffectInspection { inspected: 2 });
    assert_eq!(seen.sinks.lock().unwrap().len(), 2, "replay reached no sink");

    let (mut engine, seen) = Engine::new();
    let (previewed, _) = capture("preview", "preview", vec![word()], WIDE, &mut engine);
    let replay = replayer.replay_run(&previewed).unwrap();
    assert_eq!(replay.comparison, Comparison::Match);
    assert_eq!(replay.kind, Kind::EffectInspection { inspected: 1 });
    assert_eq!(replay.profile.words, [WordCost { alphabet: Alphabet::RetainedGenerators, letters: 2 }]);
    assert_eq!(seen.performed.load(Ordering::SeqCst), 1, "the preview was built once, when recorded");

    let (mut engine, _) = Engine::new();
    let (pure, _) = capture("pure", "pure", vec![Value::int(5)], WIDE, &mut engine);
    assert_eq!(replayer.replay_run(&pure).unwrap().kind, Kind::Reexecution);

    // An effect the record does not hold would need a live write: refused, and nothing is written.
    let mut uncaptured = saved.clone();
    uncaptured.events.clear();
    let refused = replayer.replay_run(&uncaptured).unwrap_err();
    assert_eq!(refused.code, DEBUG_WRITE_FORBIDDEN);

    // A differing effect is a mismatch at its trace position.
    let mut planted = saved.clone();
    for event in &mut planted.events {
        if let Event::Sink { bytes, .. } = event {
            bytes.push(b' ');
            break;
        }
    }
    let Comparison::Mismatch(m) = replayer.replay_run(&planted).unwrap().comparison else { panic!() };
    assert_eq!((m.code, m.field, m.position.as_deref()), (REPLAY_MISMATCH, "effects", Some("event 0")));
}
