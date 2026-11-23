//! The session's recorder (`c600_session::recorder::JournalRecorder`, LP4-9 draft): recordings and playback states as
//! workspace records in the session's own journal, outside its action order (EV4.20, PS1.16, PS1.30, PS1.44, FP.37).
//! Undo is not involved here. The crash cases of CT-EV4-14 and CT-PS1-25 with the command layer playing are
//! `c600-durable`'s `tests/ct_ev4_14_playback_crash.rs`, and reset, import and checkpoint are its
//! `tests/ps1_44_carrier.rs`.

mod common;

use c600_command::{
    code, ActionEntry, ChildOutcome, CommandLayer, Fate, Pending, PlayStatus, PlaybackState, RecordedStep, Recorder,
    Recording, Reopen, Secret, Stop, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_session::disk::{Disk, Fault, MemDisk};
use c600_session::journal::{self, Tail};
use c600_session::recorder::JournalRecorder;
use c600_session::JournalStore;

const SECRET: [u8; 32] = [7; 32];
const JOURNAL: &str = "journal";

fn recording(name: &str) -> Recording {
    let step = RecordedStep {
        row: "camera.rotate".into(),
        arguments: json::object([
            ("rotation", Value::String("quarter".into())),
            ("view", Value::String("main".into())),
        ]),
        word: None,
        depends: Vec::new(),
        module: None,
        action: None,
        command: "shell/1".into(),
        request: "r".into(),
        outcome: None,
    };
    Recording {
        name: name.into(),
        session: "s1".into(),
        identity: json::object([("model_id", Value::String("m".into()))]),
        started: Value::Null,
        steps: vec![step],
    }
}

fn state(step: u64) -> PlaybackState {
    let mut state = PlaybackState::new("r1#shell/2", "d", step);
    state.pending = Some(Pending { command: "r1#shell/2/play/1/0/1".into(), request: "q".into() });
    state.actions = vec![ActionEntry { recorded: 10, played: 100 }];
    state.status = PlayStatus::Stopped(Stop::Outcome(ChildOutcome::Cancelled));
    state
}

/// A new session's layer on `disk`, and the session's recorder.
fn create(disk: &MemDisk) -> (CommandLayer<JournalStore>, JournalRecorder) {
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let secret = Secret::fixed(SECRET);
    let layer = CommandLayer::new(k.clone(), Table::h08(), secret, store, "s1", k.solved(), ProtectedSet::none());
    let recorder = layer.store().recorder();
    (layer, recorder)
}

/// The session the disk holds, its store opened again as a restart opens it, and the store's recorder.
fn reopen(disk: &MemDisk) -> (JournalStore, JournalRecorder) {
    disk.end_holder();
    let store = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let recorder = store.recorder();
    (store, recorder)
}

/// The journal's records, in order, after its last whole frame.
fn records(disk: &MemDisk) -> Vec<Value> {
    let bytes = disk.durable(JOURNAL).expect("the disk holds a journal");
    let scan = journal::scan(&bytes);
    scan.payloads.iter().map(|payload| json::parse(payload).expect("a record")).collect()
}

fn named(record: &Value) -> &str {
    record.get("record").and_then(Value::as_str).unwrap_or_default()
}

/// PS1.16, PS1.44, FP.37: every write is one synced journal frame `command_workspace` with exactly `id`, `kind` and
/// `value`, and no action number, receipt or undo target: the journal's history holds no transaction for it. A
/// recording's value is a record object that the frame cites by its content digest; a playback state's is inline. The
/// last write of a name wins, and after a power loss the reopened store's recorder holds exactly what was written.
#[test]
fn writes_are_journal_frames_outside_the_action_order() {
    let disk = MemDisk::new();
    let (layer, mut recorder) = create(&disk);
    assert_eq!(recorder.load().unwrap(), Default::default(), "a new session holds nothing");
    assert_eq!(recorder.save_recording(&recording("r1")), Fate::Durable);
    assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
    assert_eq!(recorder.save_playback("r1", &state(1)), Fate::Durable);
    let held = recorder.load().unwrap();
    assert_eq!((held.recordings["r1"].clone(), held.playbacks["r1"].clone()), (recording("r1"), state(1)));

    let kept: Vec<Value> = records(&disk).into_iter().filter(|r| named(r) == "command_workspace").collect();
    assert_eq!(kept.len(), 3);
    for record in &kept {
        let fields: Vec<&str> = record.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(fields, ["id", "kind", "record", "value"], "no action, receipt or undo target");
        assert_eq!(record.get("id").and_then(Value::as_str), Some("r1"));
    }
    let digest = c600_digest::sha256_hex(recording("r1").to_json().canonical().as_bytes());
    assert_eq!(kept[0].get("kind").and_then(Value::as_str), Some("c600-recording-draft"));
    assert_eq!(kept[0].get("value"), Some(&json::object([("object", Value::String(digest.clone()))])));
    let object = disk.durable(&format!("objects/{digest}")).expect("the recording's object");
    assert_eq!(object, recording("r1").to_json().canonical().into_bytes());
    assert_eq!(kept[2].get("kind").and_then(Value::as_str), Some("c600-playback-draft"));
    assert_eq!(kept[2].get("value"), Some(&state(1).to_json()), "a playback state is inline");
    assert!(records(&disk).iter().all(|r| named(r) != "command_action"), "no action was written");

    drop((layer, recorder));
    disk.crash();
    let (mut store, mut recorder) = reopen(&disk);
    assert_eq!(recorder.load().unwrap(), held);
    assert!(store.history().unwrap().transactions.is_empty(), "the history holds no transaction for them");
}

/// A torn final frame, which a power loss during an append leaves, is cut off at open; the earlier frames hold.
#[test]
fn a_torn_frame_is_cut_off() {
    let disk = MemDisk::new();
    let (layer, mut recorder) = create(&disk);
    assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
    let durable = disk.durable(JOURNAL).unwrap();
    let mut clone = disk.clone();
    clone.append(JOURNAL, &[9, 0, 0, 0, 0xf6, 0xff]).unwrap();
    drop((layer, recorder));
    disk.crash_keeping(JOURNAL, 6);
    let (_store, mut recorder) = reopen(&disk);
    assert_eq!(recorder.load().unwrap().playbacks["r1"], state(0));
    assert_eq!(disk.durable(JOURNAL), Some(durable), "cut back to the whole frames");
}

/// A failed append that the journal cuts back is rolled back and changes nothing, and so is a recording whose object
/// was not written; an append it cannot cut back leaves the fate unknown, and the next read of the journal decides it,
/// in both directions.
#[test]
fn a_failed_write_is_rolled_back_or_decided_by_the_journal() {
    let disk = MemDisk::new();
    let (layer, mut recorder) = create(&disk);
    assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
    disk.fail_write(disk.writes(), Fault::Error);
    assert!(matches!(recorder.save_playback("r1", &state(1)), Fate::RolledBack(_)));
    assert_eq!(recorder.load().unwrap().playbacks["r1"], state(0));
    let journal = disk.durable(JOURNAL);
    disk.fail_write(disk.writes(), Fault::Error);
    assert!(matches!(recorder.save_recording(&recording("r1")), Fate::RolledBack(_)), "the object was not written");
    assert_eq!((recorder.load().unwrap().recordings.len(), disk.durable(JOURNAL)), (0, journal));

    // The frame lands durably, its sync reports an error and the cut fails: unknown, then landed.
    let at = disk.writes();
    disk.fail_write(at + 1, Fault::Persisted);
    disk.fail_write(at + 2, Fault::Error);
    assert_eq!(recorder.save_playback("r1", &state(2)), Fate::Unknown);
    assert_eq!(recorder.load().unwrap().playbacks["r1"], state(2), "the journal decides: landed");

    // Nothing lands and the cut fails: unknown, then not landed.
    let at = disk.writes();
    disk.fail_write(at, Fault::Error);
    disk.fail_write(at + 1, Fault::Error);
    assert_eq!(recorder.save_playback("r1", &state(3)), Fate::Unknown);
    assert_eq!(recorder.load().unwrap().playbacks["r1"], state(2), "the journal decides: not landed");
    assert_eq!(recorder.save_playback("r1", &state(4)), Fate::Durable);
    drop((layer, recorder));
    disk.crash();
    assert_eq!(reopen(&disk).1.load().unwrap().playbacks["r1"], state(4));
}

/// A damaged frame, or a workspace record that does not read (an unknown kind, or a recording under another name),
/// ends the journal's history there, as any record does: the reopened session is in read-only recovery, the recorder
/// holds what came before it, and every later write is refused.
#[test]
fn a_record_that_does_not_read_ends_the_history() {
    for case in ["damaged", "kind", "name"] {
        let disk = MemDisk::new();
        let (layer, mut recorder) = create(&disk);
        assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
        let before = disk.durable(JOURNAL).unwrap();
        assert_eq!(recorder.save_playback("r1", &state(1)), Fate::Durable);
        drop((layer, recorder));
        let mut bytes = disk.durable(JOURNAL).unwrap();
        let foreign = |id: &str, kind: &str, value: Value| {
            let record = json::object([
                ("id", Value::String(id.into())),
                ("kind", Value::String(kind.into())),
                ("record", Value::String("command_workspace".into())),
                ("value", value),
            ]);
            journal::frame(record.canonical().as_bytes())
        };
        match case {
            "damaged" => *bytes.last_mut().unwrap() ^= 1,
            "kind" => bytes = [before.clone(), foreign("r1", "c600-other-draft", state(1).to_json())].concat(),
            _ => bytes = [before.clone(), foreign("r2", "c600-recording-draft", recording("r1").to_json())].concat(),
        }
        disk.overwrite(JOURNAL, &bytes);
        let (mut store, mut recorder) = reopen(&disk);
        assert!(store.history().unwrap().damaged.is_some(), "{case}: the history ends early");
        let held = recorder.load().unwrap();
        assert_eq!((held.playbacks["r1"].clone(), held.recordings.len()), (state(0), 0), "{case}");
        let refused = recorder.save_playback("r1", &state(2));
        assert!(
            matches!(refused, Fate::RolledBack(ref e) if e.code == code::READ_ONLY_RECOVERY),
            "{case}: {refused:?}"
        );
        assert_eq!(journal::scan(&disk.durable(JOURNAL).unwrap()).tail == Tail::Clean, case != "damaged", "{case}");
    }
}
