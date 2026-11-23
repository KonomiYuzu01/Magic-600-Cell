//! PS1.44 and FP.37's carrier on the session store (LP4-9 draft): the command layer's recordings and playback states
//! are workspace records in the session's own journal (PS1.16), outside its action order, written through the store's
//! recorder (`JournalStore::recorder`). The tests show what reset, import and the recovery checkpoint do to them on
//! the command layer's path: they belong to the journal they were written in, and nothing there maps a playback's
//! action numbers onto another session's actions. Undo is the move history's (X1, owner decision of 23 November 2026,
//! item 2.7); EV4.15's single order would read the same here, since no layout step is involved.
//!
//! EV4.13 under LG1.3: a source conversion of a recording whose name holds line ends parses, lowers and checks.

mod common;

use c600_check::check;
use c600_command::{
    code, Direction, PlayStatus, Receipt, Recorder, Status, RECORDING_CONVERT_ROW, RECORDING_PLAY_ROW,
    RECORDING_START_ROW, RECORDING_STOP_ROW,
};
use c600_kernel::json::{self, Value};
use c600_session::disk::MemDisk;
use c600_session::{journal, JournalStore};
use c600_syntax::{lower, parse};
use common::layer::{completed, record_draft, Fixture};
use common::{manual, JOURNAL};

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn equip(mut f: Fixture) -> Fixture {
    let recorder = f.layer.store().recorder();
    f.layer.set_recorder(Box::new(recorder)).expect("the recorder reads");
    f
}

fn press(f: &mut Fixture, row: &str, args: Value) -> Receipt {
    f.shell.invoke(&mut f.layer, row, args).expect("prepared")
}

/// A session on `disk` that recorded two record writes as `r` and played them to the end.
fn played(disk: &MemDisk) -> Fixture {
    let mut f = equip(Fixture::create(Box::new(disk.clone())));
    assert!(completed(&press(&mut f, RECORDING_START_ROW, json::object([("name", text("r"))]))));
    assert!(completed(&f.write("w1", &[1], &manual())));
    assert!(completed(&f.write("w2", &[2], &manual())));
    assert!(completed(&press(&mut f, RECORDING_STOP_ROW, json::object([("recording", text("r"))]))));
    let play = press(&mut f, RECORDING_PLAY_ROW, json::object([("recording", text("r"))]));
    assert!(matches!(play.status, Status::Running(_)), "{:?}", play.status);
    f.layer.run_jobs();
    assert_eq!(f.layer.playback("r").expect("a state").status, PlayStatus::Finished);
    f
}

/// The journal's records by their `record` name, in order.
fn kinds(disk: &MemDisk) -> Vec<String> {
    let bytes = disk.durable(JOURNAL).expect("the disk holds a journal");
    let records = journal::scan(&bytes).payloads;
    let named =
        |payload: &Vec<u8>| json::parse(payload).ok().and_then(|r| r.get("record")?.as_str().map(str::to_owned));
    records.iter().map(|payload| named(payload).unwrap_or_default()).collect()
}

/// PS1.44, FP.37, PS1.16 (draft carrier): a recording and every playback-state write are `command_workspace` records
/// of the session's journal, interleaved with its actions and outside their order: the session holds four actions (two
/// recorded writes and two played ones), and the move history's undo target after the playback is the newest played
/// write, never a cursor. The recovery checkpoint, a reopen after a power loss, reads the recording and the state
/// back from the same journal.
#[test]
fn ps1_44_recordings_and_states_are_workspace_records_of_the_session_journal() {
    let disk = MemDisk::new();
    let f = played(&disk);
    let kinds = kinds(&disk);
    let count = |name: &str| kinds.iter().filter(|kind| *kind == name).count();
    assert_eq!(count("command_action"), 4);
    assert!(count("command_workspace") >= 1 + 4, "the recording and the state's writes: {kinds:?}");
    let last_action = kinds.iter().rposition(|kind| kind == "command_action").unwrap();
    assert_eq!(kinds.last().map(String::as_str), Some("command_workspace"), "the finished state follows the actions");
    assert!(last_action < kinds.len() - 1);
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(4), "the newest played write");
    let (recording, state) = (f.layer.recording("r").cloned(), f.layer.playback("r").cloned());
    drop(f);
    disk.crash();
    let g = equip(Fixture::open(Box::new(disk.clone())));
    assert_eq!((g.layer.recording("r").cloned(), g.layer.playback("r").cloned()), (recording, state));
}

/// Reset, import and checkpoint on the command layer's path (AGENTS.md: reset and import transactional and
/// recoverable). The layer builds none of `session.reset`, `session.checkpoint.save`, `session.checkpoint.restore`,
/// `file.log.import` and `file.macro.import`: each is refused with `KN5:input.row_not_built` and writes nothing, so the
/// journal, and the recordings and states in it, stay byte for byte. A reset that starts a new session needs a disk
/// with no journal: the old session's disk is refused before anything is written, and a new session on an empty disk
/// holds no recording and no playback state, so no `actions` map of the old session's action numbers can apply to it.
#[test]
fn reset_import_and_checkpoint_leave_the_recorder_as_it_was() {
    let disk = MemDisk::new();
    let mut f = played(&disk);
    let journal = disk.durable(JOURNAL);
    let held = f.layer.store().recorder().load().unwrap();
    for row in ["session.reset", "session.checkpoint.save", "session.checkpoint.restore", "file.log.import"] {
        assert_eq!(f.layer.availability(row).unwrap_err().code, code::ROW_NOT_BUILT, "{row}");
        if let Ok(receipt) = f.shell.invoke(&mut f.layer, row, json::object([])) {
            assert!(receipt.status.code().is_some(), "{row}: {:?}", receipt.status);
        }
        assert_eq!(disk.durable(JOURNAL), journal, "{row} wrote nothing");
    }
    assert_eq!(f.layer.availability("file.macro.import").unwrap_err().code, code::ROW_NOT_BUILT);
    assert_eq!(f.layer.store().recorder().load().unwrap(), held, "the recorder holds what it held");

    drop(f);
    disk.end_holder();
    let refused = JournalStore::create(Box::new(disk.clone())).err().expect("the old session's disk is refused");
    assert_eq!(refused.code, code::PERSISTENCE_FORMAT);
    assert_eq!(disk.durable(JOURNAL), journal, "nothing written");

    let fresh = MemDisk::new();
    let mut g = equip(Fixture::create(Box::new(fresh.clone())));
    let empty = g.layer.store().recorder().load().unwrap();
    assert!(empty.recordings.is_empty() && empty.playbacks.is_empty(), "a new session holds no recording or state");
    let play = press(&mut g, RECORDING_PLAY_ROW, json::object([("recording", text("r"))]));
    assert_eq!(play.status.code(), Some(code::NAMED_OBJECT_INVALID));
}

/// EV4.13 under LG1.3: a recording whose name holds CR, LF, `#`, NEL, U+2028 and U+2029 converts to a module that the
/// front end parses with no diagnostic, lowers and checks; its one exported entry is `replay`.
#[test]
fn a_source_conversion_parses_whatever_the_recordings_name_holds() {
    let name = "r\r\n# x\u{85}\u{2028}\u{2029}end";
    let mut f = equip(Fixture::create(Box::new(MemDisk::new())));
    assert!(completed(&f.write("word", &[1, 2], &manual())));
    assert!(completed(&press(&mut f, RECORDING_START_ROW, json::object([("name", text(name))]))));
    let id = f.preview(record_draft("word"));
    let commit = f.press(id);
    assert!(matches!(f.shell.send(&mut f.layer, commit).status, Status::Committed(_)));
    assert!(completed(&press(&mut f, RECORDING_STOP_ROW, json::object([("recording", text(name))]))));
    let args = json::object([("recording", text(name)), ("target", text("source"))]);
    assert!(completed(&press(&mut f, RECORDING_CONVERT_ROW, args)));
    let record = f.layer.record(&format!("source/recording/{name}")).expect("the source record");
    let source = record.payload.get("source").and_then(|s| s.get("text")).and_then(Value::as_str).unwrap().to_owned();
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}\n{source}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|error| panic!("{error}\n{source}"));
    check(&lowered.program).unwrap_or_else(|error| panic!("{error}\n{source}"));
    let [module] = &lowered.program.modules[..] else { panic!("one module") };
    let exported: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exported, ["replay"]);
}
