//! CT-EV4-14 and CT-PS1-25 (EV4.20, PS1.44) on the session store: KN5's command layer over M6's journal
//! (`JournalStore`), with the session's recorder (`JournalStore::recorder`, LP4-9 draft), which keeps recordings and
//! playback states as workspace records in the same journal, outside its action order. Each case fails the journal's
//! writes at a crash point, crashes the disk and opens the session again: the layer replays the journal, then installs
//! the recorder, which looks up every pending child's outcome first (KN5.23) and reconciles it at most once. Undo is
//! the move history's (X1, owner decision of 23 November 2026, item 2.7); EV4.15's single order would read the same
//! here, since no layout step is involved.
//!
//! The synthetic versions on the command layer alone are `c600-command`'s `tests/recording.rs`
//! (`ct_ev4_14_crash_points_recover_by_lookup_first`).

mod common;

use c600_command::{
    code, Caller, ChildOutcome, Direction, PlayStatus, PlaybackState, Receipt, Status, Stop, RECORDING_PLAY_ROW,
    RECORDING_START_ROW, RECORDING_STOP_ROW,
};
use c600_kernel::json::{self, Value};
use c600_session::disk::{Fault, MemDisk};
use c600_session::journal;
use common::layer::{completed, Fixture};
use common::{manual, JOURNAL};

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// The layer's host installs the session's recorder, which recovers every playback state it holds.
fn equip(mut f: Fixture) -> Fixture {
    let recorder = f.layer.store().recorder();
    f.layer.set_recorder(Box::new(recorder)).expect("the recorder reads");
    f
}

/// A new session on `disk` with its recorder.
fn create(disk: &MemDisk) -> Fixture {
    equip(Fixture::create(Box::new(disk.clone())))
}

/// A power loss, then the session opened again.
fn crash(f: Fixture, disk: &MemDisk) -> Fixture {
    drop(f);
    disk.crash();
    equip(Fixture::open(Box::new(disk.clone())))
}

fn press(f: &mut Fixture, row: &str, args: Value) -> Receipt {
    f.shell.invoke(&mut f.layer, row, args).expect("prepared")
}

fn play(f: &mut Fixture, name: &str, from: Option<i64>) -> Receipt {
    let mut args = vec![("recording".to_owned(), text(name))];
    args.extend(from.map(|from| ("from".to_owned(), Value::Int(from))));
    press(f, RECORDING_PLAY_ROW, Value::Object(args.into_iter().collect()))
}

fn state(f: &Fixture, name: &str) -> PlaybackState {
    f.layer.playback(name).expect("a playback state").clone()
}

/// The number of durable action records in the journal.
fn actions(disk: &MemDisk) -> usize {
    let bytes = disk.durable(JOURNAL).expect("the disk holds a journal");
    let records = journal::scan(&bytes).payloads;
    let named = |payload: &Vec<u8>| json::parse(payload).ok().and_then(|r| r.get("record").cloned());
    records.iter().filter(|payload| named(payload) == Some(text("command_action"))).count()
}

/// Records `name` around `body`.
fn record(f: &mut Fixture, name: &str, body: impl FnOnce(&mut Fixture)) {
    assert!(completed(&press(f, RECORDING_START_ROW, json::object([("name", text(name))]))));
    body(f);
    assert!(completed(&press(f, RECORDING_STOP_ROW, json::object([("recording", text(name))]))));
}

/// Two record writes, both unattended steps.
fn two_writes(f: &mut Fixture) {
    assert!(completed(&f.write("w1", &[1], &manual())));
    assert!(completed(&f.write("w2", &[2], &manual())));
}

/// CT-EV4-14 and CT-PS1-25 (synthetic, part: crash point 1, `pending` durable and the dispatch never happened; not
/// run: points 2 and 3). The pending entry's
/// frame, the first write of the job, reaches the disk while its sync reports an error and the cut fails, so the fate
/// is unknown and the layer does not dispatch. After the power loss the lookup finds no receipt for the child's
/// command: the playback stops on `unknown_outcome` with nothing dispatched, and no later play or job retries it.
#[test]
fn ct_ev4_14_pending_durable_before_dispatch() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    record(&mut f, "r", two_writes);
    assert!(matches!(play(&mut f, "r", None).status, Status::Running(_)));
    let before = actions(&disk);
    let at = disk.writes();
    disk.fail_write(at + 1, Fault::Persisted);
    disk.fail_write(at + 2, Fault::Error);
    f.layer.run_jobs();
    let pending = state(&f, "r").pending.expect("the pending entry");
    assert_eq!(actions(&disk), before, "nothing was dispatched");
    let kept = f.kept();

    let mut g = crash(f, &disk);
    let recovered = state(&g, "r");
    assert_eq!(recovered.status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)));
    assert_eq!((recovered.step, recovered.pending.is_none()), (0, true));
    let lookup = g.layer.kernel_outcome(&Caller::Shell, &pending.command, &pending.request);
    assert_eq!(lookup.status, Status::UnknownOutcome);
    g.layer.run_jobs();
    assert_eq!(code_of(&play(&mut g, "r", None)), Some(code::RECORDING_STATE), "no retry without `from`");
    assert_eq!((g.kept(), actions(&disk)), (kept, before), "no fresh retry");
    let h = crash(g, &disk);
    assert_eq!(state(&h, "r"), recovered, "recovered once");
}

fn code_of(receipt: &Receipt) -> Option<&'static str> {
    receipt.status.code()
}

/// CT-EV4-14 and CT-PS1-25 (synthetic, part: crash point 2, the child's completion durable and the cursor's advance
/// not;
/// not run: points 1 and 3). The job's writes are the pending entry's frame (append and sync), the child's action
/// frame (its record value's
/// object is the recorded write's, already on the disk) and the advance's frame; every write fails from the advance's
/// append on, so its fate is unknown and the next step is never dispatched. After the power loss the lookup finds the
/// record write's receipt in the journal: the playback advances once, maps the recorded action to the played one and,
/// with no live job, stops with `KN5:resource_limit.engine_lost` (draft); a second open advances nothing more. An
/// unrelated invocation leaves the cursor, a play continues from it, and the cursor is never an undo target.
#[test]
fn ct_ev4_14_child_durable_before_the_advance() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    record(&mut f, "r", two_writes);
    let recorded = f.layer.recording("r").unwrap().steps[0].action.expect("the recorded action");
    assert!(matches!(play(&mut f, "r", None).status, Status::Running(_)));
    let before = actions(&disk);
    disk.fail_from(disk.writes() + 4);
    f.layer.run_jobs();
    assert_eq!(actions(&disk), before + 1, "the first child's action is durable, and no other");
    let played = f.layer.undo_hint(Direction::Undo).expect("the played write").target;
    assert_eq!(played, recorded + 2, "the first step played as the third action");

    let g = crash(f, &disk);
    let recovered = state(&g, "r");
    assert_eq!((recovered.step, recovered.pending.is_none()), (1, true), "advanced once by the lookup");
    assert_eq!(recovered.played(recorded), Some(played), "the map entry");
    assert_eq!(recovered.status, PlayStatus::Stopped(Stop::Code(code::ENGINE_LOST.to_owned())));
    let kept = g.kept();
    let mut h = crash(g, &disk);
    assert_eq!(state(&h, "r"), recovered, "a second open advances nothing");
    assert_eq!(h.kept(), kept);

    assert!(completed(&h.write("x", &[5], &manual())), "an unrelated invocation");
    assert_eq!(state(&h, "r").step, 1, "leaves the cursor");
    assert!(matches!(play(&mut h, "r", None).status, Status::Running(_)));
    h.layer.run_jobs();
    let finished = state(&h, "r");
    assert_eq!(finished.status, PlayStatus::Finished);
    assert_eq!(h.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(played + 2), "the newest write");
    let i = crash(h, &disk);
    assert_eq!(state(&i, "r"), finished);
}

/// CT-EV4-14 and CT-PS1-25 (synthetic, part: crash point 3, during a manual continuation; not run: points 1 and 2).
/// The playback waits at the recorded preview,
/// a user step; the step action writes `pending` durably and the power fails before the shell sends the row. After
/// the open the lookup finds no receipt: the playback stops on `unknown_outcome`, nothing was committed or previewed,
/// and only an explicit `from` opens a new playback, which waits at the same step.
#[test]
fn ct_ev4_14_during_a_manual_continuation() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    record(&mut f, "c", |f| {
        f.commit(&[1]);
    });
    assert!(matches!(play(&mut f, "c", None).status, Status::Running(_)));
    f.layer.run_jobs();
    assert_eq!(state(&f, "c").status, PlayStatus::Waiting(0));
    let (row, args) = f.layer.step_arguments("c").unwrap();
    let invocation = f.shell.prepare(&f.layer, &row, args).unwrap();
    f.layer.begin_step("c", &invocation).unwrap();
    let kept = f.kept();

    let mut g = crash(f, &disk);
    let stopped = state(&g, "c");
    assert_eq!(stopped.status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)));
    assert_eq!(g.kept(), kept, "nothing committed or written");
    assert_eq!(code_of(&play(&mut g, "c", None)), Some(code::RECORDING_STATE));
    assert!(matches!(play(&mut g, "c", Some(0)).status, Status::Running(_)));
    g.layer.run_jobs();
    let rewound = state(&g, "c");
    assert_ne!(rewound.playback, stopped.playback, "a new playback");
    assert_eq!((rewound.step, rewound.status.clone()), (0, PlayStatus::Waiting(0)));
    let h = crash(g, &disk);
    assert_eq!(state(&h, "c"), rewound, "the explicit `from` is durable");
}
