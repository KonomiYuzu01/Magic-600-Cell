//! The recorder on a disk (`c600_session::recorder`, LP4-9 draft): recordings and playback states in their own file of
//! frames, outside the journal's action order (EV4.20, PS1.30, PS1.44). The crash cases of CT-EV4-14 and CT-PS1-25,
//! with the command layer over the journal, are `c600-durable`'s `tests/ct_ev4_14_playback_crash.rs`.

use c600_command::{
    ActionEntry, ChildOutcome, Fate, Pending, PlayStatus, PlaybackState, RecordedStep, Recorder, Recording, Stop,
};
use c600_kernel::json::{self, Value};
use c600_session::disk::{Disk, Fault, MemDisk};
use c600_session::recorder::{DiskRecorder, RECORDER};

const PERSISTENCE_FORMAT: &str = "PS1:input.persistence_format";

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

fn open(disk: &MemDisk) -> DiskRecorder {
    DiskRecorder::open(Box::new(disk.clone())).expect("the recorder opens")
}

/// Every write is one synced frame; the last write of a name wins; after a power loss a new recorder on the disk holds
/// exactly what was written, and a missing file holds nothing.
#[test]
fn writes_survive_a_power_loss() {
    let disk = MemDisk::new();
    let mut recorder = open(&disk);
    assert_eq!(recorder.load().unwrap(), Default::default(), "no file, nothing held");
    assert_eq!(recorder.save_recording(&recording("r1")), Fate::Durable);
    assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
    assert_eq!(recorder.save_playback("r1", &state(1)), Fate::Durable);
    let held = recorder.load().unwrap();
    assert_eq!((held.recordings["r1"].clone(), held.playbacks["r1"].clone()), (recording("r1"), state(1)));
    drop(recorder);
    disk.crash();
    assert_eq!(open(&disk).load().unwrap(), held);
}

/// A torn final frame, which a power loss during an append leaves, is cut off at open; the earlier frames hold.
#[test]
fn a_torn_frame_is_cut_off() {
    let disk = MemDisk::new();
    let mut recorder = open(&disk);
    assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
    let durable = disk.durable(RECORDER).unwrap();
    let mut clone = disk.clone();
    clone.append(RECORDER, &[9, 0, 0, 0, 0xf6, 0xff]).unwrap();
    drop(recorder);
    disk.crash_keeping(RECORDER, 6);
    assert_eq!(open(&disk).load().unwrap().playbacks["r1"], state(0));
    assert_eq!(disk.durable(RECORDER), Some(durable), "cut back to the whole frames");
}

/// A failed append that the recorder cuts back is rolled back and changes nothing; one it cannot cut back leaves the
/// fate unknown, and the next read of the file decides it, in both directions.
#[test]
fn a_failed_write_is_rolled_back_or_decided_by_the_file() {
    let disk = MemDisk::new();
    let mut recorder = open(&disk);
    assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
    disk.fail_write(disk.writes(), Fault::Error);
    assert!(matches!(recorder.save_playback("r1", &state(1)), Fate::RolledBack(_)));
    assert_eq!(recorder.load().unwrap().playbacks["r1"], state(0));

    // The frame lands durably, its sync reports an error and the cut fails: unknown, then landed.
    let at = disk.writes();
    disk.fail_write(at + 1, Fault::Persisted);
    disk.fail_write(at + 2, Fault::Error);
    assert_eq!(recorder.save_playback("r1", &state(2)), Fate::Unknown);
    assert_eq!(recorder.load().unwrap().playbacks["r1"], state(2), "the file decides: landed");

    // Nothing lands and the cut fails: unknown, then not landed.
    let at = disk.writes();
    disk.fail_write(at, Fault::Error);
    disk.fail_write(at + 1, Fault::Error);
    assert_eq!(recorder.save_playback("r1", &state(3)), Fate::Unknown);
    assert_eq!(recorder.load().unwrap().playbacks["r1"], state(2), "the file decides: not landed");
    assert_eq!(recorder.save_playback("r1", &state(4)), Fate::Durable);
    drop(recorder);
    disk.crash();
    assert_eq!(open(&disk).load().unwrap().playbacks["r1"], state(4));
}

/// A damaged frame, or a frame that does not read as a record, refuses the recorder with
/// `PS1:input.persistence_format` and every later write, and nothing is cut.
#[test]
fn a_damaged_file_is_refused() {
    let disk = MemDisk::new();
    let mut recorder = open(&disk);
    assert_eq!(recorder.save_playback("r1", &state(0)), Fate::Durable);
    let mut bytes = disk.durable(RECORDER).unwrap();
    bytes[10] ^= 1; // a byte of the frame's payload, which its checksum no longer matches
    disk.overwrite(RECORDER, &bytes);
    let mut damaged = open(&disk);
    assert_eq!(damaged.load().unwrap_err().code, PERSISTENCE_FORMAT);
    assert!(matches!(damaged.save_playback("r1", &state(1)), Fate::RolledBack(_)));
    assert_eq!(disk.durable(RECORDER), Some(bytes), "nothing is cut or written");

    let foreign = MemDisk::new();
    let frame = c600_session::journal::frame(br#"{"record":"other"}"#);
    foreign.overwrite(RECORDER, &frame);
    assert_eq!(open(&foreign).load().unwrap_err().code, PERSISTENCE_FORMAT);
}
