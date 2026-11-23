//! CT-EV4-14 and CT-PS1-25 (EV4.20, PS1.44) on the session store: KN5's command layer over M6's journal
//! (`JournalStore`), with the recorder on a disk (`c600_session::recorder::DiskRecorder`, LP4-9 draft) on a memory disk
//! of its own, so a test can fail and crash the recorder's writes apart from the journal's. Each case crashes both
//! disks and opens the session again: the layer replays the journal, then installs the recorder, which looks up every
//! pending child's outcome first (KN5.23) and reconciles it at most once. Undo is the move history's (X1, owner
//! decision of 23 November 2026, item 2.7); EV4.15's single order would read the same here, since no layout step is
//! involved.
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
use c600_session::recorder::DiskRecorder;
use common::layer::{completed, Fixture};
use common::{manual, JOURNAL};

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// The layer's host installs the recorder on `recorder`, which recovers every playback state it holds.
fn equip(mut f: Fixture, recorder: &MemDisk) -> Fixture {
    let opened = DiskRecorder::open(Box::new(recorder.clone())).expect("the recorder opens");
    f.layer.set_recorder(Box::new(opened)).expect("the recorder reads");
    f
}

/// A new session on `disk` with the recorder on `recorder`.
fn create(disk: &MemDisk, recorder: &MemDisk) -> Fixture {
    equip(Fixture::create(Box::new(disk.clone())), recorder)
}

/// A power loss on both disks, then the session opened again.
fn crash(f: Fixture, disk: &MemDisk, recorder: &MemDisk) -> Fixture {
    drop(f);
    disk.crash();
    recorder.crash();
    equip(Fixture::open(Box::new(disk.clone())), recorder)
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

/// CT-EV4-14 and CT-PS1-25 (crash point 1: `pending` is durable, the dispatch never happened). The recorder's pending
/// frame reaches the disk while its sync reports an error and the cut fails, so the fate is unknown and the layer does
/// not dispatch. After the power loss the lookup finds no receipt for the child's command: the playback stops on
/// `unknown_outcome` with nothing dispatched, and no later play or job retries it.
#[test]
fn ct_ev4_14_pending_durable_before_dispatch() {
    let (disk, rdisk) = (MemDisk::new(), MemDisk::new());
    let mut f = create(&disk, &rdisk);
    record(&mut f, "r", two_writes);
    assert!(matches!(play(&mut f, "r", None).status, Status::Running(_)));
    let journal = disk.durable(JOURNAL);
    let at = rdisk.writes();
    rdisk.fail_write(at + 1, Fault::Persisted);
    rdisk.fail_write(at + 2, Fault::Error);
    f.layer.run_jobs();
    let pending = state(&f, "r").pending.expect("the pending entry");
    assert_eq!(disk.durable(JOURNAL), journal, "nothing was dispatched");
    let kept = f.kept();

    let mut g = crash(f, &disk, &rdisk);
    let recovered = state(&g, "r");
    assert_eq!(recovered.status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)));
    assert_eq!((recovered.step, recovered.pending.is_none()), (0, true));
    let lookup = g.layer.kernel_outcome(&Caller::Shell, &pending.command, &pending.request);
    assert_eq!(lookup.status, Status::UnknownOutcome);
    g.layer.run_jobs();
    assert_eq!(code_of(&play(&mut g, "r", None)), Some(code::RECORDING_STATE), "no retry without `from`");
    assert_eq!(g.kept(), kept, "no fresh retry");
    let h = crash(g, &disk, &rdisk);
    assert_eq!(state(&h, "r"), recovered, "recovered once");
}

fn code_of(receipt: &Receipt) -> Option<&'static str> {
    receipt.status.code()
}

/// CT-EV4-14 and CT-PS1-25 (crash point 2: the child's completion is durable in the journal, the cursor's advance is
/// not). The recorder fails from the advance on, so the advance's fate is unknown and the next step is never
/// dispatched. After the power loss the lookup finds the record write's receipt in the journal: the playback advances
/// once, maps the recorded action to the played one and, with no live job, stops with
/// `KN5:resource_limit.engine_lost` (draft); a second open advances nothing more. An unrelated invocation leaves the
/// cursor, a play continues from it, and the cursor is never an undo target.
#[test]
fn ct_ev4_14_child_durable_before_the_advance() {
    let (disk, rdisk) = (MemDisk::new(), MemDisk::new());
    let mut f = create(&disk, &rdisk);
    record(&mut f, "r", two_writes);
    let recorded = f.layer.recording("r").unwrap().steps[0].action.expect("the recorded action");
    assert!(matches!(play(&mut f, "r", None).status, Status::Running(_)));
    rdisk.fail_from(rdisk.writes() + 2);
    f.layer.run_jobs();
    let played = f.layer.undo_hint(Direction::Undo).expect("the played write").target;
    assert_eq!(played, recorded + 2, "the first step played as the third action");

    let g = crash(f, &disk, &rdisk);
    let recovered = state(&g, "r");
    assert_eq!((recovered.step, recovered.pending.is_none()), (1, true), "advanced once by the lookup");
    assert_eq!(recovered.played(recorded), Some(played), "the map entry");
    assert_eq!(recovered.status, PlayStatus::Stopped(Stop::Code(code::ENGINE_LOST.to_owned())));
    let kept = g.kept();
    let mut h = crash(g, &disk, &rdisk);
    assert_eq!(state(&h, "r"), recovered, "a second open advances nothing");
    assert_eq!(h.kept(), kept);

    assert!(completed(&h.write("x", &[5], &manual())), "an unrelated invocation");
    assert_eq!(state(&h, "r").step, 1, "leaves the cursor");
    assert!(matches!(play(&mut h, "r", None).status, Status::Running(_)));
    h.layer.run_jobs();
    let finished = state(&h, "r");
    assert_eq!(finished.status, PlayStatus::Finished);
    assert_eq!(h.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(played + 2), "the newest write");
    let i = crash(h, &disk, &rdisk);
    assert_eq!(state(&i, "r"), finished);
}

/// CT-EV4-14 and CT-PS1-25 (crash point 3: during a manual continuation). The playback waits at the recorded preview,
/// a user step; the step action writes `pending` durably and the power fails before the shell sends the row. After
/// the open the lookup finds no receipt: the playback stops on `unknown_outcome`, nothing was committed or previewed,
/// and only an explicit `from` opens a new playback, which waits at the same step.
#[test]
fn ct_ev4_14_during_a_manual_continuation() {
    let (disk, rdisk) = (MemDisk::new(), MemDisk::new());
    let mut f = create(&disk, &rdisk);
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

    let mut g = crash(f, &disk, &rdisk);
    let stopped = state(&g, "c");
    assert_eq!(stopped.status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)));
    assert_eq!(g.kept(), kept, "nothing committed or written");
    assert_eq!(code_of(&play(&mut g, "c", None)), Some(code::RECORDING_STATE));
    assert!(matches!(play(&mut g, "c", Some(0)).status, Status::Running(_)));
    g.layer.run_jobs();
    let rewound = state(&g, "c");
    assert_ne!(rewound.playback, stopped.playback, "a new playback");
    assert_eq!((rewound.step, rewound.status.clone()), (0, PlayStatus::Waiting(0)));
    let h = crash(g, &disk, &rdisk);
    assert_eq!(state(&h, "c"), rewound, "the explicit `from` is durable");
}
