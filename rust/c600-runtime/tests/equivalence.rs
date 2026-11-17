//! The child against the interpreter in process (RTE.21, ONE.69): the same jobs give the same values, outputs and
//! run summaries (rows, charges, stop reasons and positions), and the caller's host sees the same calls in the same
//! order. Source and synthetic evidence only: no timing is measured.

mod support;

use std::sync::Arc;
use std::time::Duration;

use c600_check::Profile;
use c600_ir::Capability;
use c600_run::interp::{code, CancelFlag};
use c600_run::meter::code::{CANCELLED_BY_USER, KERNEL_COST, MEMORY, NOT_GRANTED, OUTPUT, STEPS};
use c600_run::meter::{Amounts, Interruption, Outcome};
use c600_run::value::Value;
use c600_runtime::{run_in_process, Ended, JobOptions, Stage};
use num_bigint::BigInt;
use support::{assert_same_end, compare, header, job, output, ran, runtime, Session, WIDE};

const PURE: &str = "fn add(x: Nat, y: Nat) -> Nat effects [] {\n    return exact_add(x, y);\n}\n\n\
    export fn main(a: Nat) -> Nat effects [] {\n    return add(a, 2);\n}\n\n\
    export fn sums() -> record { phi: Q5, r: Rat, less: Bool, same: Bool } effects [] {\n    \
    let one = exact_add(q5(1/2, 1/2), q5(1/2, -1/2));\n    \
    return record { phi = one, r = exact_add(1/2, 1/3), less = exact_less(1/3, 1/2), same = exact_eq(one, 1) };\n}\n\n\
    export fn sum(x: Nat) -> Nat effects [] {\n    let a = 1;\n    let b = 1000;\n    \
    let f = fn(z: Nat) -> Nat { return exact_add(z, exact_add(b, a)); };\n    return f(x);\n}\n\n\
    fn shade(n: Nat) -> Colour effects [] {\n    \
    return match exact_less(n, 3) {\n        true => colour(\"blue\");\n        _ => colour(\"navy\");\n    };\n}\n\n\
    export fn pick(x: Nat, y: Nat) -> Display<Colour> effects [] {\n    \
    return display_if(approx_less(approx53(x), approx53(y)), colour(\"red\"), shade(x));\n}\n\n\
    export fn keep(x: Nat) -> Nat effects [] {\n    let a = x;\n    let b = exact_add(a, 1);\n    return a;\n}\n\n\
    export fn down(n: Nat) -> Nat effects [] {\n    return down(exact_add(n, 1));\n}\n\n\
    export fn show(x: Nat) -> Unit effects [view] {\n    return publish_view(record { b = x, a = \"\u{e9}\\n\" });\n}\n";

const SNAPSHOT: &str = "fn probe() -> Nat effects [snapshot] {\n    return read_snapshot<Nat>(\"pieces\");\n}\n\n\
    export fn main() -> Nat effects [snapshot] {\n    return exact_add(probe(), 1);\n}\n\n\
    export fn small() -> Nat effects [snapshot] {\n    return read_snapshot<Nat>(\"small\");\n}\n";

const TOOL: &str = "export fn save(w: Word) -> Unit effects [workspace, file] {\n    \
    let a = write_workspace<Word>(\"saved\", w);\n    let b = file_write(\"saved.6cw\", w);\n    return ();\n}\n\n\
    export fn relay() -> Unit effects [snapshot, workspace, file] {\n    \
    let a = read_snapshot<Word>(\"best_word\");\n    let b = file_read<Word>(\"saved.6cw\");\n    \
    return write_workspace<Word>(\"relayed\", b);\n}\n";

const PREVIEWER: &str = "export fn go(w: Word) -> Text effects [preview] {\n    \
    return receipt_id(propose(w, target(\"t\")));\n}\n";

const RANDOM: &str = "export fn two() -> record { a: Nat, b: Nat } effects [random] {\n    \
    return record { a = random_draw(), b = random_draw() };\n}\n";

fn pure() -> String {
    format!("{}{PURE}", header(""))
}

/// Exact numbers, records, closures, `display_if` with approximate comparison, and big integers give identical
/// results; one child serves every run.
#[test]
fn pure_programs_give_identical_runs() {
    let mut rt = runtime();
    let source = pure();
    let big = Value::int(BigInt::from(1u8) << 200);
    for (entry, args) in [
        ("main", vec![Value::int(5)]),
        ("main", vec![big]),
        ("sums", vec![]),
        ("sum", vec![Value::int(5)]),
        ("pick", vec![Value::int(1), Value::int(2)]),
        ("pick", vec![Value::int(5), Value::int(2)]),
        ("keep", vec![Value::int(5)]),
    ] {
        let (end, _) = compare(&mut rt, &job(&source, entry, args, JobOptions::new(WIDE)), |_| {});
        assert_eq!(ran(&end).summary.outcome, Outcome::Completed, "{entry}");
    }
    let (end, _) = compare(&mut rt, &job(&source, "main", vec![Value::int(5)], JobOptions::new(WIDE)), |_| {});
    assert_eq!(output(&end), "{\"int\":\"7\"}\n");
    assert_eq!(rt.spawns(), 1, "one child serves every run");
}

/// A run that publishes a view reaches the caller's host exactly once, with the same bytes.
#[test]
fn a_published_view_reaches_the_host_once() {
    let mut rt = runtime();
    let (end, session) = compare(&mut rt, &job(&pure(), "show", vec![Value::int(5)], JobOptions::new(WIDE)), |_| {});
    assert_eq!(ran(&end).summary.outcome, Outcome::Completed);
    assert_eq!(session.views, [b"{\"a\":\"\xc3\xa9\\n\",\"b\":{\"int\":\"5\"}}\n".to_vec()]);
    assert_eq!(session.log.len(), 1);
}

/// A run that writes a workspace record and a granted file, and one that reads them back through the host.
#[test]
fn workspace_records_and_files_are_written_once() {
    let mut rt = runtime();
    let source = format!("{}{TOOL}", header("file"));
    let options = JobOptions { grant: vec![Capability::File], ..JobOptions::new(WIDE) };
    let (end, session) =
        compare(&mut rt, &job(&source, "save", vec![Value::word(&[1, -2, 3])], options.clone()), |_| {});
    assert_eq!(ran(&end).summary.outcome, Outcome::Completed);
    assert_eq!(session.workspace.len(), 1);
    assert_eq!(session.workspace[0].0, "saved");
    assert_eq!(session.written.len(), 1);
    assert_eq!(session.written[0].0, "saved.6cw");
    let (end, session) = compare(&mut rt, &job(&source, "relay", vec![], options), |_| {});
    assert_eq!(ran(&end).summary.outcome, Outcome::Completed);
    assert_eq!(session.workspace.len(), 1);
    assert_eq!(session.workspace[0].0, "relayed");
}

/// A run that proposes a word: the host receives the proposal once, takes it as the session's pending preview, and
/// the receipt crosses back. The journal is the session's and stays as it was.
#[test]
fn a_proposed_word_reaches_the_host_once() {
    let mut rt = runtime();
    let source = format!("{}{PREVIEWER}", header(""));
    let (end, session) =
        compare(&mut rt, &job(&source, "go", vec![Value::word(&[1, -2, 3])], JobOptions::new(WIDE)), |_| {});
    assert_eq!(output(&end), "\"preview-1\"\n");
    assert_eq!(session.proposals, [(vec![1, -2, 3], "t".to_string())]);
    assert_eq!(session.preview, Some(vec![1, -2, 3]));
    assert_eq!(session.journal, Session::default().journal);

    // In a solving session the same program is refused at admission on both sides (LG3.16).
    let solving = JobOptions { profile: Profile::Solving, ..JobOptions::new(WIDE) };
    let (end, session) = compare(&mut rt, &job(&source, "go", vec![Value::word(&[1])], solving), |_| {});
    match end {
        Ended::Refused(r) => assert_eq!((r.stage, r.codes), (Stage::Admit, vec![NOT_GRANTED.to_string()])),
        other => panic!("{other:#?}"),
    }
    assert!(session.log.is_empty());
}

/// Every budget stops at the same operation, phase and position with the same charges.
#[test]
fn limits_stop_at_the_same_operation() {
    let mut rt = runtime();
    let source = pure();
    let limited = |limits: Amounts| JobOptions::new(limits);
    for (entry, args, limits, code) in [
        ("down", vec![Value::int(0)], Amounts { steps: 1_000, ..WIDE }, STEPS),
        ("main", vec![Value::int(5)], Amounts { kernel_cost: 5, ..WIDE }, KERNEL_COST),
        ("main", vec![Value::int(5)], Amounts { memory: 40, ..WIDE }, MEMORY),
        ("show", vec![Value::int(5)], Amounts { output: 10, ..WIDE }, OUTPUT),
    ] {
        let (end, _) = compare(&mut rt, &job(&source, entry, args, limited(limits)), |_| {});
        let summary = &ran(&end).summary;
        assert_eq!((summary.outcome, summary.code), (Outcome::CutOff, Some(code)), "{entry}");
        let stop = summary.stop.as_ref().expect("a stop");
        // Entry operations have no source; a stop inside the program names its range.
        assert!(stop.at.starts_with("entry#") || stop.source.is_some(), "{entry}: {stop:?}");
    }
    // The quiet meter keeps no rows; a deep recursion still stops at the same place.
    let quiet = JobOptions { record: false, source_ranges: false, ..limited(Amounts { steps: 400_000, ..WIDE }) };
    let (end, _) = compare(&mut rt, &job(&source, "down", vec![Value::int(0)], quiet), |_| {});
    let summary = &ran(&end).summary;
    assert!(summary.rows.is_empty());
    assert_eq!(summary.stop.as_ref().map(|s| s.at.matches(" > ").count()), Some(99_999));
}

/// Snapshot reads cross with their tariffs; a host refusal, static or the host's own, is the run's stop code; a
/// host value over its declared bound stops the run.
#[test]
fn snapshot_reads_and_host_refusals_cross() {
    let mut rt = runtime();
    let source = format!("{}{SNAPSHOT}", header(""));
    let (end, session) = compare(&mut rt, &job(&source, "main", vec![], JobOptions::new(WIDE)), |_| {});
    assert_eq!(output(&end), "{\"int\":\"43\"}\n");
    assert_eq!(session.log.len(), 2, "one tariff and one read");

    let (end, _) = compare(&mut rt, &job(&source, "small", vec![], JobOptions::new(WIDE)), |_| {});
    assert_eq!(ran(&end).summary.code, Some(code::HOST_EXHAUSTED));

    for refusal in [NOT_GRANTED, "TEST:host.refused_by_the_session"] {
        let (end, _) =
            compare(&mut rt, &job(&source, "main", vec![], JobOptions::new(WIDE)), |s| s.refuse = Some(refusal));
        let summary = &ran(&end).summary;
        assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(refusal)));
    }
}

const READER: &str = "export fn guarded(w: Word) -> Nat effects [snapshot] {\n    \
    return exact_add(read_snapshot<Nat>(\"pieces\", w), 1);\n}\n";

/// A snapshot read with an argument (LP2 draft) crosses as `request.snapshot_with`: the caller's host is asked for
/// the tariff and the read once each, with the argument's canonical bytes, and the answer crosses back.
#[test]
fn a_snapshot_read_with_an_argument_crosses_with_its_bytes() {
    let mut rt = runtime();
    let source = format!("{}{READER}", header(""));
    let word = Value::word(&[1, -2]);
    let (end, session) = compare(&mut rt, &job(&source, "guarded", vec![word.clone()], JobOptions::new(WIDE)), |_| {});
    assert_eq!(output(&end), "{\"int\":\"43\"}\n");
    let mut bytes = Vec::new();
    c600_run::encode::encode(&word, &mut bytes).expect("a canonical word");
    let arg = format!("arg: {bytes:?}");
    assert_eq!(session.log.len(), 2, "one tariff and one read");
    assert!(session.log.iter().all(|l| l.contains("SnapshotWith") && l.contains(&arg)), "{:?}", session.log);
}

/// Seeded draws give the same values; without a seed the draw is refused at the same operation.
#[test]
fn seeded_draws_are_identical() {
    let mut rt = runtime();
    let source = format!("{}{RANDOM}", header("random"));
    let seeded = JobOptions {
        seed: Some((vec![7u8; 32], vec!["test".into(), "two".into()])),
        grant: vec![Capability::Randomness],
        ..JobOptions::new(WIDE)
    };
    let (end, _) = compare(&mut rt, &job(&source, "two", vec![], seeded), |_| {});
    assert_eq!(ran(&end).summary.outcome, Outcome::Completed);
    let unseeded = JobOptions { grant: vec![Capability::Randomness], ..JobOptions::new(WIDE) };
    let (end, _) = compare(&mut rt, &job(&source, "two", vec![], unseeded), |_| {});
    assert_eq!(ran(&end).summary.code, Some(NOT_GRANTED));
}

/// Refusals before the first operation are the same at every stage.
#[test]
fn refusals_before_the_first_operation_are_identical() {
    let mut rt = runtime();
    let refused = |end: Ended| match end {
        Ended::Refused(r) => r,
        other => panic!("{other:#?}"),
    };
    let broken = format!("{}export fn f( -> Nat effects [] {{\n    return 1;\n}}\n", header(""));
    let (end, _) = compare(&mut rt, &job(&broken, "f", vec![], JobOptions::new(WIDE)), |_| {});
    assert_eq!(refused(end).stage, Stage::Parse);

    let (end, _) = compare(&mut rt, &job(&pure(), "absent", vec![], JobOptions::new(WIDE)), |_| {});
    assert_eq!(refused(end).stage, Stage::Admit);

    let random = format!("{}{RANDOM}", header("random"));
    let (end, _) = compare(&mut rt, &job(&random, "two", vec![], JobOptions::new(WIDE)), |_| {});
    let r = refused(end);
    assert_eq!((r.stage, r.codes), (Stage::Admit, vec![NOT_GRANTED.to_string()]));

    let (end, _) = compare(&mut rt, &job(&pure(), "main", vec![], JobOptions::new(WIDE)), |_| {});
    let r = refused(end);
    assert_eq!((r.stage, r.codes), (Stage::Start, vec![code::IR_REJECTED.to_string()]));

    let (end, _) = compare(&mut rt, &job(&pure(), "main", vec![Value::Fn(0)], JobOptions::new(WIDE)), |_| {});
    let r = refused(end);
    assert_eq!((r.stage, r.codes), (Stage::Start, vec![code::NONCANONICAL_VALUE.to_string()]));
    assert_eq!(rt.spawns(), 1);
}

/// DET.56, KN7.6: an interruption the caller raises from inside a host call stops both executors at the same
/// operation; the next run on the same child is not interrupted.
#[test]
fn an_interruption_from_inside_a_host_call_stops_at_the_same_operation() {
    let source = format!("{}{SNAPSHOT}", header(""));
    let j = job(&source, "main", vec![], JobOptions::new(WIDE));

    let flag = Arc::new(CancelFlag::default());
    let mut here = Session::default();
    let raise = Arc::clone(&flag);
    here.on_perform = Some(Box::new(move || raise.cancel(Interruption::ByUser)));
    let a = run_in_process(&j, &mut here, Some(&*flag));

    let mut rt = runtime();
    rt.start().unwrap();
    let canceller = rt.canceller().unwrap();
    let mut there = Session::default();
    there.on_perform = Some(Box::new(move || assert!(canceller.cancel(Interruption::ByUser), "a run in progress")));
    let b = rt.run(&j, &mut there, None).unwrap();
    assert_same_end(&a, &b);
    here.assert_same(&there);
    let summary = &ran(&b).summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Cancelled, Some(CANCELLED_BY_USER)));
    assert_eq!(summary.stop.as_ref().map(|s| s.phase), Some("interruption"));

    let c = rt.run(&j, &mut Session::default(), None).unwrap();
    assert_eq!(output(&c), "{\"int\":\"43\"}\n");
    assert_eq!(rt.spawns(), 1);
}

/// A long pure job: it makes no host call, so only an interruption ends it before its safety stop.
const SPIN: &str = "fn spin(n: Nat, top: Nat) -> Nat effects [] {\n    \
    return match exact_less(n, top) {\n        \
    true => exact_add(spin(exact_add(n, 1), top), spin(exact_add(n, 1), top));\n        _ => 0;\n    };\n}\n\n\
    export fn forever() -> Nat effects [] {\n    return spin(0, 64);\n}\n";

/// DET.56, KN7.6: a cancel sent as soon as a run is in progress is never lost, however it interleaves with the
/// child's start of the run. Each round raises the cancel from another thread the moment the run is in progress; the
/// job has no host call, so nothing else can end it before the safety stop, which is a timeout of the test and not a
/// measurement. Every round ends cancelled, on the same child.
#[test]
fn a_cancel_right_after_the_start_is_never_lost() {
    const ROUNDS: u32 = 200;
    let j =
        job(&format!("{}{SPIN}", header("")), "forever", vec![], JobOptions { record: false, ..JobOptions::new(WIDE) });
    let mut rt = runtime();
    rt.start().unwrap();
    let canceller = rt.canceller().unwrap();
    assert!(!canceller.cancel(Interruption::ByUser), "no run in progress");
    for round in 0..ROUNDS {
        let c = canceller.clone();
        let raise = std::thread::spawn(move || {
            while !c.cancel(Interruption::ByUser) {
                std::thread::yield_now();
            }
        });
        let end = rt.run(&j, &mut Session::default(), Some(Duration::from_secs(60)));
        let end = end.unwrap_or_else(|f| panic!("round {round}: {f}"));
        let summary = &ran(&end).summary;
        assert_eq!((summary.outcome, summary.code), (Outcome::Cancelled, Some(CANCELLED_BY_USER)), "round {round}");
        raise.join().unwrap();
    }
    assert_eq!(rt.spawns(), 1);
    assert!(!canceller.cancel(Interruption::ByUser), "no run in progress");
}
