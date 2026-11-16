//! Fault rehearsals (KN7.13, ERR.19, ONE.70, CT-ERR-14 in part; ERR.16 for the deadline). These are rehearsals of the
//! fault paths with a debug-only injector in the child, not LX-2 evidence: no decisive run, no timing measurement and
//! no Windows/DirectX claim.
//!
//! CT-RTE-10, the headless fault half: after runtime death the session keeps its journal rows, state hash and pending
//! preview, and the run gets a typed outcome (rehearsals, not LX-2's native sequences).
//!
//! The work job first proposes a word, which the session takes as its pending preview, and every fault comes after
//! the proposal was served. In each rehearsal the parent gets a typed terminal outcome; the session keeps its journal
//! rows and state hash as before the run and the pending preview the run set, nothing replays the proposal, and the
//! session stays usable through the shell gate; and a fresh child runs the next job as the interpreter does in
//! process.

#![cfg(debug_assertions)]

mod support;

use std::time::Duration;

use c600_run::meter::Outcome;
use c600_run::value::Value;
use c600_runtime::parent::{RUNTIME_LOST, SAFETY_STOP};
use c600_runtime::{run_in_process, Fault, FaultKind, Job, JobOptions, Runtime, FAULT_VARIABLE};
use support::{assert_same_end, header, job, output, runtime, Session, Token, WIDE};

/// Seven host calls: the proposal's tariff and perform, a snapshot's tariff and read, a view, and the second
/// snapshot's tariff and read.
const WORK: &str = "export fn work(w: Word) -> Nat effects [preview, snapshot, view] {\n    \
    let p = propose(w, target(\"t\"));\n    \
    let a = read_snapshot<Nat>(\"pieces\");\n    let v = publish_view(record { a = a });\n    \
    let b = read_snapshot<Nat>(\"pieces\");\n    return exact_add(a, b);\n}\n";

/// The word the work job proposes. It differs from the pending preview a fresh session holds.
const PROPOSED: [i32; 3] = [2, -9, 4];

fn work() -> Job {
    job(&format!("{}{WORK}", header("")), "work", vec![Value::word(&PROPOSED)], JobOptions::new(WIDE))
}

fn kept(session: &Session) -> (Vec<Vec<i32>>, [u8; 32], Option<Vec<i32>>) {
    (session.journal.clone(), session.state_hash(), session.preview.clone())
}

/// Holds a session whose run lost its runtime after the proposal to what it must keep: the journal rows and the
/// state hash of `before`, and the proposal as its pending preview, which the host received once.
fn assert_kept(session: &Session, before: &(Vec<Vec<i32>>, [u8; 32], Option<Vec<i32>>)) {
    assert_ne!(before.2.as_deref(), Some(&PROPOSED[..]), "the run changes the pending preview");
    assert_eq!(
        kept(session),
        (before.0.clone(), before.1, Some(PROPOSED.to_vec())),
        "the journal rows, the state hash and the pending preview the run set are kept"
    );
    assert_eq!(session.proposals, [(PROPOSED.to_vec(), "t".to_string())], "nothing replays the proposal");
}

/// Runs the work job in a child that rehearses `fault`; checks the session.
fn rehearse(fault: &str, deadline: Option<Duration>) -> (Fault, Session, Runtime) {
    let mut rt = runtime();
    rt.config_mut().env.push((FAULT_VARIABLE.into(), fault.into()));
    let mut session = Session::default();
    let before = kept(&session);
    let fault = rt.run(&work(), &mut session, deadline).expect_err("a fault");
    assert_kept(&session, &before);
    assert!(rt.child_id().is_none(), "the faulted child is gone");
    (fault, session, rt)
}

/// A fresh child runs the next job as in process, and the faulted session stays usable: the shell gate commits the
/// preview the lost run set, and the next run works on the session.
fn recovers(mut rt: Runtime, mut session: Session) {
    rt.config_mut().env.clear();
    let spawns = rt.spawns();
    let mut reference = Session::default();
    let expected = run_in_process(&work(), &mut reference, None);
    let mut fresh = Session::default();
    let got = rt.run(&work(), &mut fresh, None).unwrap_or_else(|f| panic!("{f}"));
    assert_same_end(&expected, &got);
    reference.assert_same(&fresh);
    assert_eq!(rt.spawns(), spawns + 1, "a fresh child");

    let journal = session.journal.len();
    session.confirm(Token::from_the_shell()).unwrap();
    assert_eq!(session.journal.len(), journal + 1);
    assert_eq!(session.journal.last(), Some(&PROPOSED.to_vec()));
    let next = rt.run(&work(), &mut session, None).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(output(&next), "{\"int\":\"84\"}\n");
    assert_eq!(session.preview.as_deref(), Some(&PROPOSED[..]));
}

fn lost(fault: &Fault) {
    assert_eq!(fault.kind, FaultKind::Lost, "{fault}\n{}", fault.stderr);
    assert_eq!((fault.code(), fault.outcome()), (RUNTIME_LOST, Outcome::Error));
}

/// Rehearsal: the caller kills the child from inside its second `perform`, the first snapshot read.
#[test]
fn rehearsal_child_killed_mid_run() {
    let mut rt = runtime();
    rt.start().unwrap();
    let killer = rt.killer().unwrap();
    let mut session = Session::default();
    let before = kept(&session);
    let mut performs = 0;
    session.on_perform = Some(Box::new(move || {
        performs += 1;
        if performs == 2 {
            killer.kill();
        }
    }));
    let fault = rt.run(&work(), &mut session, None).expect_err("a fault");
    lost(&fault);
    session.on_perform = None;
    assert_kept(&session, &before);
    assert_eq!(session.log.len(), 4, "the host saw the calls before the loss and nothing after");
    recovers(rt, session);
}

/// Rehearsal: the child panics before its third call.
#[test]
fn rehearsal_child_panics() {
    let (fault, session, rt) = rehearse("panic@2", None);
    lost(&fault);
    assert_eq!(fault.exit, Some(101));
    assert!(fault.stderr.contains("fault rehearsal: panic"), "{}", fault.stderr);
    assert_eq!(session.log.len(), 2);
    recovers(rt, session);
}

/// Rehearsal: the child aborts. Windows reports an abort as a nonzero exit status; on Unix the abort is a signal, so
/// the child reports no exit status.
#[test]
fn rehearsal_child_aborts() {
    let (fault, session, rt) = rehearse("abort@3", None);
    lost(&fault);
    if cfg!(windows) {
        assert!(fault.exit.is_some_and(|code| code != 0), "{:?}", fault.exit);
    } else {
        assert_eq!(fault.exit, None, "an abort is a signal, not an exit status");
    }
    assert_eq!(session.log.len(), 3);
    recovers(rt, session);
}

/// Rehearsal: the child exits with a status of its own.
#[test]
fn rehearsal_child_exits() {
    let (fault, session, rt) = rehearse("exit@4", None);
    lost(&fault);
    assert_eq!(fault.exit, Some(3));
    assert_eq!(session.log.len(), 4);
    recovers(rt, session);
}

/// Rehearsal: the child's output closes after its last call, before the result.
#[test]
fn rehearsal_child_closes_its_output_before_the_result() {
    let (fault, session, rt) = rehearse("close@7", None);
    lost(&fault);
    assert_eq!(fault.exit, Some(0));
    assert_eq!(session.log.len(), 7, "every call was served; the result never came");
    recovers(rt, session);
}

/// Rehearsal: the child's output breaks off inside a frame.
#[test]
fn rehearsal_child_breaks_off_a_frame() {
    let (fault, session, rt) = rehearse("truncate@2", None);
    lost(&fault);
    assert!(fault.detail.contains("inside a frame"), "{fault}");
    recovers(rt, session);
}

/// Rehearsal: the child writes a frame that is not a message, then waits.
#[test]
fn rehearsal_child_writes_garbage() {
    let (fault, session, rt) = rehearse("garbage@2", None);
    assert_eq!(fault.kind, FaultKind::Malformed, "{fault}");
    assert_eq!((fault.code(), fault.outcome()), (RUNTIME_LOST, Outcome::Error));
    recovers(rt, session);
}

/// Rehearsal: the child announces a frame above the bound, then waits; nothing is allocated for it.
#[test]
fn rehearsal_child_announces_an_oversized_frame() {
    let (fault, session, rt) = rehearse("oversized@2", None);
    assert_eq!(fault.kind, FaultKind::Oversized, "{fault}");
    assert_eq!(fault.code(), RUNTIME_LOST);
    recovers(rt, session);
}

/// Rehearsal: the child hangs; the run's wall-clock safety stop ends it as cut off (ERR.16). The deadline is a
/// timeout of the rehearsal, not a measurement.
#[test]
fn rehearsal_child_hangs_past_the_deadline() {
    let (fault, session, rt) = rehearse("hang@2", Some(Duration::from_secs(2)));
    assert_eq!(fault.kind, FaultKind::Deadline, "{fault}");
    assert_eq!((fault.code(), fault.outcome()), (SAFETY_STOP, Outcome::CutOff));
    recovers(rt, session);
}

/// A child that died between runs is replaced before the next run is sent.
#[test]
fn a_child_lost_between_runs_is_replaced() {
    let mut rt = runtime();
    let first = rt.run(&work(), &mut Session::default(), None).unwrap();
    assert_eq!(output(&first), "{\"int\":\"84\"}\n");
    rt.killer().unwrap().kill();
    let second = rt.run(&work(), &mut Session::default(), None).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(output(&second), "{\"int\":\"84\"}\n");
    assert_eq!(rt.spawns(), 2);
}

/// The rehearsal variable in the parent's own environment never reaches a child: the parent removes it.
#[test]
fn an_inherited_rehearsal_variable_is_removed() {
    std::env::set_var(FAULT_VARIABLE, "panic@0");
    let result = runtime().run(&work(), &mut Session::default(), None);
    std::env::remove_var(FAULT_VARIABLE);
    assert_eq!(output(&result.unwrap_or_else(|f| panic!("{f}"))), "{\"int\":\"84\"}\n");
}
