//! Lines of history and snapshots (CT-KN4-04). The engine owns the journal's branches with exactly one active line: a
//! commit after an undo keeps the old line, and a line switch is a row of kind `commit`. Programs may view and compare
//! other lines through opened snapshots and cannot create, switch or change a line.

mod common;

use c600_kernel::codes;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::value::Value;
use c600_session::{code, At, Input, JobKind, JobOutcome, Lane, Origin, Row, Status};
use common::*;

/// A commit after an undo starts a new line and keeps the old one; exactly one line is active; a switch is a commit
/// row that needs the shell's token, moves the state to the other line's head, and is undone like any commit.
#[test]
fn a_commit_after_an_undo_keeps_the_old_line() {
    let (session, disk) = session();
    let a = committed(&commit(&session, "A", &[1])).clone();
    undo(&session, "U1");
    let b = committed(&commit(&session, "B", &[2])).clone();
    let o = session.overview();
    assert_eq!((o.lines, o.active_line), (2, 1), "the old line stays and the new one is active");
    assert_eq!(o.revision.digest, b.post);

    assert!(Row::SwitchLine { line: 0 }.is_commit(), "a line switch is a row of kind commit");
    let bare = session.invoke(envelope(&session, "S0", Origin::Shell, Row::SwitchLine { line: 0 }));
    assert_eq!(code(&bare), code::TOKEN_INVALID, "a switch without a token is refused");
    let missing = shell(&session, "S1", Row::SwitchLine { line: 2 });
    assert_eq!(code(&missing), codes::NAMED_OBJECT_INVALID);

    let (order, state) = (o.order, o.revision.state);
    let switched = shell(&session, "S", Row::SwitchLine { line: 0 });
    let record = committed(&switched);
    assert_eq!((record.pre.as_str(), record.post.as_str()), (b.post.as_str(), a.post.as_str()));
    let o = session.overview();
    assert_eq!((o.lines, o.active_line), (2, 0));
    assert_eq!((o.order, o.revision.state, o.revision.digest.as_str()), (order + 1, state + 1, a.post.as_str()));

    let back = committed(&undo(&session, "U2")).clone();
    assert_eq!(back.post, b.post, "the switch is undone like any commit");
    assert_eq!(session.overview().active_line, 1);
    let reopened = reopen(&disk);
    assert_eq!(durable(&reopened), durable(&session), "lines and the active line are durable");
}

/// A program job compares two lines through opened snapshots: one of the old line's head and one of the current
/// state. Its answer is the number of slots where the two differ, and nothing about the lines changes.
#[test]
fn a_program_job_compares_two_lines_through_snapshots() {
    let (session, _disk) = session();
    let a = committed(&commit(&session, "A", &[1])).clone();
    undo(&session, "U");
    let b = committed(&commit(&session, "B", &[2])).clone();
    let before = session.overview();

    let old_line = session.open_snapshot(At::Revision(a.revision.clone())).expect("the old line's head");
    let current = session.open_snapshot(At::Current).expect("the current state");
    assert_eq!((old_line.state().digest(), current.state().digest()), (a.post.as_str(), b.post.as_str()));
    let differ = old_line.state().labels().iter().zip(current.state().labels()).filter(|(x, y)| x != y).count();
    assert!(differ > 0);

    let inputs = vec![Input::Handle(old_line.clone()), Input::Handle(current.clone())];
    let admitted = try_submit(&session, "J", job("diff", Vec::new(), inputs, Lane::Program, JobKind::Query));
    let done = finished(&session, &admitted);
    assert!(matches!(done.status, Status::Completed(_)), "{done:?}");
    let result = session.wait_job(admitted.job.unwrap()).unwrap();
    assert_eq!(result.outcome, JobOutcome::Completed);
    assert_eq!(result.value, Some(nat(differ as u64)));
    assert_eq!(result.input, a.post, "the result is bound to its first input");
    assert_eq!(result.inputs, vec![a.revision.clone(), current.revision.clone()]);

    let after = session.overview();
    assert_eq!(after, before, "a comparison changes no line");
}

/// No runtime request creates, switches or changes a line: a program delivers no commit row and no session row, and
/// a job that asks for a preview or a workspace write is refused by its host.
#[test]
fn no_runtime_request_changes_a_line() {
    let (session, disk) = session();
    commit(&session, "A", &[1]);
    undo(&session, "U");
    commit(&session, "B", &[2]);
    let (before, writes) = (session.overview(), disk.writes());

    for (command, row) in
        [("p1", Row::SwitchLine { line: 0 }), ("p2", Row::Undo { hint: before.undo.unwrap(), confirm: false })]
    {
        // Even with a genuine token the shell minted, a program's commit row is refused.
        let mut e = envelope(&session, command, Origin::Program, row);
        e.token = Some(session.shell().mint(command, &e.row).expect("a token"));
        assert_eq!(code(&session.invoke(e)), code::COMMIT_ROW_REFUSED, "{command}");
    }
    let checkpoint = from_program(&session, "p3", Row::Checkpoint { name: "k".into() });
    assert_eq!(code(&checkpoint), NOT_GRANTED);

    for (command, function) in [("j1", "offer"), ("j2", "keep")] {
        let spec = job(function, vec![Value::word(&[1])], vec![Input::Current], Lane::Program, JobKind::Query);
        let done = finished(&session, &try_submit(&session, command, spec));
        assert_eq!(code(&done), NOT_GRANTED, "{function}");
    }
    let mut after = session.overview();
    after.jobs_running = before.jobs_running;
    assert_eq!(after, before);
    assert_eq!(disk.writes(), writes, "no runtime request wrote anything");
}

/// A snapshot stays as it was opened while the session moves on. A checkpoint's snapshot is read from its stored
/// state object, after a reopen too; a damaged object is not used, and the state is replayed along the checkpoint's
/// path (PS1.29). Only revisions this session reached in this engine epoch open.
#[test]
fn snapshots_stay_as_they_were_opened() {
    let (session, disk) = session();
    let a = committed(&commit(&session, "A", &[1])).clone();
    let held = session.open_snapshot(At::Current).unwrap();
    shell(&session, "K", Row::Checkpoint { name: "k".into() });
    commit(&session, "B", &[2]);
    commit(&session, "C", &[3]);
    assert_eq!(held.state().digest(), a.post, "a held snapshot does not move");
    let again = session.open_snapshot(At::Revision(a.revision.clone())).unwrap();
    assert_eq!(again.state().labels(), held.state().labels());
    let checkpoint = session.open_snapshot(At::Checkpoint("k".into())).unwrap();
    assert_eq!(checkpoint.state().digest(), a.post);

    let mut forged = a.revision.clone();
    forged.state = 99;
    assert_eq!(session.open_snapshot(At::Revision(forged)).unwrap_err().code, code::REVISION_GONE);
    let mut other = a.revision.clone();
    other.session = "s2".into();
    assert_eq!(session.open_snapshot(At::Revision(other)).unwrap_err().code, code::UNKNOWN_SESSION);
    let missing = session.open_snapshot(At::Checkpoint("none".into())).unwrap_err();
    assert_eq!(missing.code, codes::NAMED_OBJECT_INVALID);

    let reopened = reopen(&disk);
    let old_epoch = reopened.open_snapshot(At::Revision(a.revision.clone())).unwrap_err();
    assert_eq!(old_epoch.code, code::EPOCH_CHANGED);
    let stale = Input::Handle(held.clone());
    let refused = try_submit(&reopened, "J", job("state", Vec::new(), vec![stale], Lane::Program, JobKind::Query));
    assert_eq!(code(&refused), code::EPOCH_CHANGED, "a snapshot of another epoch is no job input");
    let checkpoint = reopened.open_snapshot(At::Checkpoint("k".into())).expect("the checkpoint after a reopen");
    assert_eq!(checkpoint.state().digest(), a.post);

    let object = format!("objects/{}", a.post);
    let mut bytes = disk.durable(&object).expect("the checkpoint's state object");
    bytes[0] ^= 0x01;
    disk.overwrite(&object, &bytes);
    let replayed = reopened.open_snapshot(At::Checkpoint("k".into())).expect("the checkpoint from its path");
    assert_eq!(replayed.state().labels(), held.state().labels(), "the damaged object is not used");
}
