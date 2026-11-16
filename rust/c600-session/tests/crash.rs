//! Crash consistency of the session store (the proactive crash-consistency task for M6). One durable action is one
//! frame: after a crash at any point a reopen finds the session wholly before or wholly after the action, writable
//! unless a frame is damaged, and answering what the journal proves (R-06, KN5.21 to KN5.23, ONE.70, PS1.29). The
//! cases: unsynced frames found by a reopen, torn and zero-filled appends, every prefix of a journal, every damaged
//! byte, failures during recovery, lost state objects, identical retries after a restart, `Session::create` on a disk
//! that holds a journal, and seeded live histories that must replay. Store reads and writes are counted, never timed.

mod common;

use c600_digest::sha256_hex;
use c600_run::provenance::Json;
use c600_session::disk::{Fault, MemDisk};
use c600_session::journal::{self, Tail};
use c600_session::{
    code, ActionHint, At, CompletionKind, Options, Origin, Overview, Receipt, Row, Session, Status, WordSource,
};
use common::*;

/// A fresh memory disk whose journal holds these bytes durably, and which holds no state object.
fn disk_with(journal: &[u8]) -> MemDisk {
    let disk = MemDisk::new();
    disk.overwrite("journal", journal);
    disk
}

/// The end offsets of a journal's whole frames.
fn ends(bytes: &[u8]) -> Vec<usize> {
    let mut at = 0;
    journal::scan(bytes)
        .payloads
        .iter()
        .map(|payload| {
            at += 8 + payload.len() + 32;
            at
        })
        .collect()
}

/// Whether a receipt reports a durable action: a commit, or a completed workspace action.
fn is_durable(receipt: &Receipt) -> bool {
    match &receipt.status {
        Status::Committed(_) => true,
        Status::Completed(c) => c.kind == CompletionKind::Workspace,
        _ => false,
    }
}

/// R-06, KN5.22: a reopen that finds an action's frame written but not synced syncs it before it answers from it, so
/// a power loss after the reopen cannot take back what the reopen served.
#[test]
fn a_reopen_serves_only_what_it_made_durable() {
    for kind in kinds() {
        let (session, disk) = session();
        (kind.setup)(&session);
        let frames = ends(&disk.durable("journal").unwrap()).len();
        // The frame's sync fails, and so does the cut that would prove the action absent.
        disk.fail_from(disk.writes() + writes_of(&kind) - 1);
        let receipt = (kind.act)(&session);
        assert_eq!(receipt.status, Status::UnknownOutcome, "{}", kind.name);

        disk.heal();
        let first = reopen(&disk);
        let served = first.outcome("act", &receipt.request);
        assert!(is_durable(&served), "{}: {served:?}", kind.name);
        assert_eq!(ends(&disk.durable("journal").unwrap()).len(), frames + 1, "{}: the reopen synced it", kind.name);

        disk.crash();
        let second = reopen(&disk);
        assert_eq!(second.outcome("act", &receipt.request), served, "{}", kind.name);
        assert_eq!(durable(&second), durable(&first), "{}", kind.name);
    }
}

/// The journal after a kind's setup, the frame its action appends, and the durable views before and after the
/// action, from a clean run.
fn appended(kind: &Kind) -> (Vec<u8>, Vec<u8>, Overview, Overview) {
    let (session, disk) = session();
    (kind.setup)(&session);
    let (before, view) = (disk.durable("journal").unwrap(), durable(&session));
    let receipt = (kind.act)(&session);
    assert!(is_durable(&receipt), "{}: {receipt:?}", kind.name);
    let after = disk.durable("journal").unwrap();
    assert!(after.starts_with(&before));
    let frame = after[before.len()..].to_vec();
    (before, frame, view, durable(&session))
}

/// ONE.70, KN5.23: an append cut off at any byte, or cut off and followed by zero bytes as a file system that extends
/// a file before its data arrives shows it, is a torn append: the reopened session is wholly before the action,
/// writable, and its journal is cut back durably. A whole frame is wholly after. Every length is scanned; sessions
/// are opened at the edges of each part of the frame.
#[test]
fn a_torn_or_zero_filled_append_recovers_wholly_before_or_after() {
    for kind in kinds() {
        let (before, frame, view_before, view_after) = appended(&kind);
        let (len, frames) = (frame.len(), ends(&before).len());
        let body = len - 32;
        let tail = |k: usize, zeros: usize| {
            let mut bytes = [&before[..], &frame[..k]].concat();
            bytes.resize(bytes.len() + zeros, 0);
            // The bytes hold the whole frame when the zeros stand where the frame's own last bytes are zero.
            (bytes, zeros >= len - k && frame[k..].iter().all(|&b| b == 0))
        };

        for k in 0..=len {
            for zeros in [0, len - k, len - k + 4096] {
                let (bytes, whole) = tail(k, zeros);
                let scan = journal::scan(&bytes);
                let expected = if k + zeros == 0 {
                    (frames, Tail::Clean)
                } else if !whole {
                    (frames, Tail::Torn)
                } else if zeros == len - k {
                    (frames + 1, Tail::Clean)
                } else {
                    (frames + 1, Tail::Torn)
                };
                assert_eq!((scan.payloads.len(), scan.tail), expected, "{} byte {k} zeros {zeros}", kind.name);
            }
        }

        let samples =
            [(1, 0), (9, 0), (body, len - body), (body + 5, len - body - 5), (len - 1, 0), (0, 4096), (len, 4096)];
        for (k, zeros) in samples {
            let case = format!("{} byte {k} zeros {zeros}", kind.name);
            let (bytes, whole) = tail(k, zeros);
            let disk = disk_with(&bytes);
            let reopened = reopen(&disk);
            assert_eq!(reopened.overview().read_only, None, "{case}");
            if whole {
                assert_eq!(durable(&reopened), view_after, "{case}");
                assert_eq!(disk.durable("journal").unwrap(), [&before[..], &frame[..]].concat(), "{case}");
                continue;
            }
            assert_eq!(durable(&reopened), view_before, "{case}");
            assert_eq!(disk.durable("journal").unwrap(), before, "{case}: the cut is durable");
            let next = set_record(&reopened, "next", "n", letters(&[1]));
            assert!(is_durable(&next), "{case}: {next:?}");
            assert_eq!(ends(&disk.durable("journal").unwrap()).len(), frames + 1, "{case}");
        }
    }
}

/// A history over the seven kinds of durable action: its disk, the receipt of each action, and the durable view
/// after each frame (the first frame is the session's header).
fn history() -> (MemDisk, Vec<Receipt>, Vec<Overview>) {
    let (session, disk) = session();
    let steps: [fn(&Session) -> Receipt; 10] = [
        |s| commit(s, "a", &[1]),
        |s| commit(s, "b", &[2]),
        |s| undo(s, "u1"),
        |s| redo(s, "r1"),
        |s| undo(s, "u2"),
        |s| commit(s, "c", &[3]),
        |s| shell(s, "s", Row::SwitchLine { line: 0 }),
        |s| shell(s, "k", Row::Checkpoint { name: "k".into() }),
        |s| set_record(s, "w", "r", letters(&[1, 2])),
        |s| protect(s, "p", &[0]),
    ];
    let mut views = vec![durable(&session)];
    let mut receipts = Vec::new();
    for step in steps {
        let receipt = step(&session);
        assert!(is_durable(&receipt), "{receipt:?}");
        receipts.push(receipt);
        views.push(durable(&session));
    }
    (disk, receipts, views)
}

/// ONE.70, PS1.29, KN5.22: any prefix of a journal opens as the session after its last whole frame, writable, with the
/// receipts of the actions in it and `unknown_outcome` for the rest. A prefix inside the header holds no session:
/// opening it is refused, and the disk takes a new session. Every length is scanned; sessions are opened at every
/// frame boundary and inside every frame.
#[test]
fn every_journal_prefix_opens_at_its_last_whole_frame() {
    let (disk, receipts, views) = history();
    let full = disk.durable("journal").unwrap();
    let ends = ends(&full);
    assert_eq!(ends.len(), views.len());
    let whole = |length: usize| ends.iter().filter(|&&end| end <= length).count();
    for length in 0..=full.len() {
        let scan = journal::scan(&full[..length]);
        let tail = if length == 0 || ends.contains(&length) { Tail::Clean } else { Tail::Torn };
        assert_eq!((scan.payloads.len(), scan.tail), (whole(length), tail), "length {length}");
    }

    let mut lengths = vec![0, 5, ends[0] - 1];
    for &end in &ends {
        lengths.push(end);
        if end < full.len() {
            lengths.push(end + 9);
        }
    }
    for length in lengths {
        let disk = disk_with(&full[..length]);
        let frames = whole(length);
        if frames == 0 {
            let refused = Session::open(kernel(), Box::new(disk.clone()), Options::default()).err();
            assert_eq!(refused.map(|e| e.code), Some(code::UNKNOWN_SESSION), "length {length}");
            Session::create(kernel(), Box::new(disk.clone()), "s1", Options::default()).expect("a new session");
            continue;
        }
        let reopened = reopen(&disk);
        assert_eq!(reopened.overview().read_only, None, "length {length}");
        assert_eq!(durable(&reopened), views[frames - 1], "length {length}");
        for (i, receipt) in receipts.iter().enumerate() {
            let found = reopened.outcome(&receipt.command, &receipt.request);
            if i + 1 < frames {
                assert_eq!(&found, receipt, "length {length}");
            } else {
                assert_eq!(found.status, Status::UnknownOutcome, "length {length}: {}", receipt.command);
            }
        }
    }
}

/// ONE.70, KN5.23: a damaged byte anywhere in a frame opens the session read-only at the last whole frame before it,
/// and the journal is never cut. Damage in the header frame leaves no session: opening is refused, and so is a new
/// session on the disk, before anything is written. Every byte is scanned; sessions are opened for each part of a
/// few frames.
#[test]
fn every_damaged_byte_opens_read_only_at_the_frame_before() {
    let (disk, _, views) = history();
    let full = disk.durable("journal").unwrap();
    let ends = ends(&full);
    let start = |frame: usize| if frame == 0 { 0 } else { ends[frame - 1] };
    for (frame, &end) in ends.iter().enumerate() {
        for at in start(frame)..end {
            let mut bytes = full.clone();
            flip(&mut bytes, at);
            let scan = journal::scan(&bytes);
            assert_eq!((scan.payloads.len(), scan.tail), (frame, Tail::Damaged(start(frame) as u64)), "byte {at}");
        }
    }

    for frame in [0, 1, 6, ends.len() - 1] {
        let (from, to) = (start(frame), ends[frame]);
        // The length, its complement, the payload and the checksum.
        for at in [from, from + 5, from + 11, to - 1] {
            let case = format!("frame {frame} byte {at}");
            let mut bytes = full.clone();
            flip(&mut bytes, at);
            let disk = disk_with(&bytes);
            if frame == 0 {
                let refused = Session::open(kernel(), Box::new(disk.clone()), Options::default()).err();
                assert_eq!(refused.map(|e| e.code), Some(code::UNKNOWN_SESSION), "{case}");
                let writes = disk.writes();
                let refused = Session::create(kernel(), Box::new(disk.clone()), "s2", Options::default()).err();
                assert_eq!(refused.map(|e| e.code), Some(code::PERSISTENCE_FORMAT), "{case}");
                assert_eq!(disk.writes(), writes, "{case}: nothing was written");
            } else {
                let reopened = reopen(&disk);
                let mut expected = views[frame - 1].clone();
                expected.read_only = Some(code::COMMIT_FATE);
                assert_eq!(durable(&reopened), expected, "{case}");
                let refused = set_record(&reopened, "next", "n", letters(&[1]));
                assert_eq!(code(&refused), code::READ_ONLY_RECOVERY, "{case}");
            }
            assert_eq!(disk.durable("journal").unwrap(), bytes, "{case}: damage is never cut");
        }
    }
}

/// ONE.70: recovery that fails is done again whole. When cutting a torn tail or syncing the cut fails, opening fails
/// with `KN4:resource_limit.disk_full`; the next open, after a power loss or not, finds the same session and makes
/// the cut durable.
#[test]
fn a_failed_recovery_is_done_again_whole() {
    let (session, disk) = session();
    commit(&session, "a", &[1]);
    let (before, journal_before) = (durable(&session), disk.durable("journal").unwrap());
    drop(session);
    let torn = [&journal_before[..], &journal::frame(b"{\"next\":1}")[..20]].concat();
    // Write 0 is the cut, write 1 its sync.
    for (failing, crash) in [(0, false), (0, true), (1, false), (1, true)] {
        let case = format!("write {failing} crash {crash}");
        let disk = disk_with(&torn);
        disk.fail_write(failing, Fault::Error);
        let refused = Session::open(kernel(), Box::new(disk.clone()), Options::default()).err();
        assert_eq!(refused.map(|e| e.code), Some(code::DISK_FULL), "{case}");
        if crash {
            disk.crash();
        } else {
            disk.heal();
        }
        let reopened = reopen(&disk);
        assert_eq!(durable(&reopened), before, "{case}");
        assert_eq!(disk.durable("journal").unwrap(), journal_before, "{case}");
    }
}

/// How a test loses or damages a checkpoint's state object; it returns the disk to reopen.
type Damage = fn(&MemDisk, &str) -> MemDisk;

/// PS1.29, PS3.5: a checkpoint never depends on its state object alone. When the object is lost, emptied or damaged,
/// the checkpoint still opens, replayed along its path from the start to its digest, and a later checkpoint of the
/// same state writes the object again; an intact object is not written twice. A checkpoint whose object write failed
/// part-way is refused, cites nothing, and the next one writes the object whole.
#[test]
fn a_lost_state_object_is_rewritten_or_replayed() {
    let damages: [(&str, Damage); 3] = [
        ("lost", |disk, _| disk_with(&disk.durable("journal").unwrap())),
        ("emptied", |disk, object| {
            disk.overwrite(object, &[]);
            disk.clone()
        }),
        ("damaged", |disk, object| {
            let mut bytes = disk.durable(object).unwrap();
            bytes[7] ^= 0x10;
            disk.overwrite(object, &bytes);
            disk.clone()
        }),
    ];
    for (case, damage) in damages {
        let (session, disk) = session();
        let a = committed(&commit(&session, "A", &[1])).clone();
        shell(&session, "K", Row::Checkpoint { name: "k".into() });
        let object = format!("objects/{}", a.post);
        assert_eq!(sha256_hex(&disk.durable(&object).unwrap()), a.post, "{case}: an object is named by its digest");
        drop(session);

        let disk = damage(&disk, &object);
        let reopened = reopen(&disk);
        let opened = reopened.open_snapshot(At::Checkpoint("k".into())).expect("the checkpoint, replayed");
        assert_eq!(opened.state().digest(), a.post, "{case}");
        let writes = disk.writes();
        let again = shell(&reopened, "K2", Row::Checkpoint { name: "k2".into() });
        assert!(is_durable(&again), "{case}: {again:?}");
        assert_eq!(disk.writes() - writes, 4, "{case}: the object is written again");
        assert_eq!(sha256_hex(&disk.durable(&object).unwrap()), a.post, "{case}");
        let writes = disk.writes();
        shell(&reopened, "K3", Row::Checkpoint { name: "k3".into() });
        assert_eq!(disk.writes() - writes, 2, "{case}: an intact object is not written twice");
        let reopened = reopen(&disk);
        for name in ["k", "k2", "k3"] {
            let opened = reopened.open_snapshot(At::Checkpoint(name.into())).expect("the checkpoint");
            assert_eq!(opened.state().digest(), a.post, "{case}: {name}");
        }
    }

    let (session, disk) = session();
    let a = committed(&commit(&session, "A", &[1])).clone();
    disk.fail_write(disk.writes(), Fault::Partial(100));
    assert_eq!(code(&shell(&session, "K", Row::Checkpoint { name: "k".into() })), code::DISK_FULL);
    assert_eq!(session.overview().checkpoints, 0);
    let again = shell(&session, "K2", Row::Checkpoint { name: "k".into() });
    assert!(is_durable(&again), "{again:?}");
    assert_eq!(sha256_hex(&disk.durable(&format!("objects/{}", a.post)).unwrap()), a.post);
}

/// KN5.21, KN5.14, LI.30: an identical retry delivered after a restart, still naming the old epoch, gets the receipt
/// the journal proves before any epoch or token check, and writes nothing. A transient row's receipt does not
/// survive the restart, so its retry is refused as stale; other content under a durable command ID is a reuse.
#[test]
fn a_retry_after_restart_returns_the_durable_receipt() {
    let (session, disk) = session();
    let preview = propose(&session, "p", literal(&[1]));
    let mut c = envelope(&session, "C", Origin::Shell, Row::Execute { preview });
    c.token = Some(session.shell().mint("C", &c.row).unwrap());
    let c_receipt = session.invoke(c.clone());
    committed(&c_receipt);
    let row = Row::WorkspaceSet { record: "r".into(), payload: letters(&[2]), provenance: manual() };
    let w = envelope(&session, "W", Origin::Shell, row);
    let w_receipt = session.invoke(w.clone());
    let mut u =
        envelope(&session, "U", Origin::Shell, Row::Undo { hint: session.overview().undo.unwrap(), confirm: false });
    u.token = Some(session.shell().mint("U", &u.row).unwrap());
    let u_receipt = session.invoke(u.clone());
    committed(&u_receipt);
    let q = envelope(&session, "Q", Origin::Shell, Row::Propose { word: literal(&[2]) });
    session.invoke(q.clone());
    let view = durable(&session);

    disk.crash();
    let reopened = reopen(&disk);
    let writes = disk.writes();
    assert_eq!(reopened.invoke(c.clone()), c_receipt);
    assert_eq!(reopened.invoke(w), w_receipt);
    assert_eq!(reopened.invoke(u), u_receipt);
    assert_eq!(code(&reopened.invoke(q)), code::EPOCH_CHANGED);
    let mut other = c;
    other.row = Row::Execute { preview: preview + 1 };
    assert_eq!(code(&reopened.invoke(other)), code::COMMAND_ID_REUSED);
    assert_eq!(disk.writes(), writes, "a retry writes nothing");
    assert_eq!(durable(&reopened), view);
}

/// KN5.23, LI.8: `Session::create` never guesses a rollback. When the header's fate is unknown it says so, and the
/// header it may have left opens; a proved rollback leaves a disk that takes the session. A disk that holds a whole
/// or a damaged frame is refused before anything is written, also behind a torn tail; the torn remnant of a header
/// alone is cut, and the new session starts clean.
#[test]
fn create_is_honest_about_the_header() {
    let create = |disk: &MemDisk, id: &str| Session::create(kernel(), Box::new(disk.clone()), id, Options::default());

    // Write 0 appends the header, write 1 syncs it, and write 2 would cut it.
    let disk = MemDisk::new();
    disk.fail_write(1, Fault::Persisted);
    disk.fail_from(2);
    assert_eq!(create(&disk, "s1").err().map(|e| e.code), Some(code::COMMIT_FATE));
    disk.crash();
    assert_eq!(reopen(&disk).id(), "s1", "the header the unknown fate left opens");

    let disk = MemDisk::new();
    disk.fail_write(0, Fault::Error);
    assert_eq!(create(&disk, "s1").err().map(|e| e.code), Some(code::DISK_FULL));
    create(&disk, "s1").expect("the disk takes the session after a proved rollback");

    let (session, disk) = session();
    commit(&session, "a", &[1]);
    drop(session);
    let journal = disk.durable("journal").unwrap();
    let mut damaged = journal.clone();
    flip(&mut damaged, journal.len() - 1);
    let torn = [&journal[..], &journal::frame(b"{\"next\":1}")[..9]].concat();
    for (case, bytes) in [("whole", &journal), ("damaged", &damaged), ("torn", &torn)] {
        disk.overwrite("journal", bytes);
        let writes = disk.writes();
        assert_eq!(create(&disk, "s2").err().map(|e| e.code), Some(code::PERSISTENCE_FORMAT), "{case}");
        assert_eq!(disk.writes(), writes, "{case}: nothing was written");
        assert_eq!(&disk.durable("journal").unwrap(), bytes, "{case}");
    }

    let header = ends(&journal)[0];
    let disk = disk_with(&journal[..header - 20]);
    create(&disk, "s3").expect("a new session over a torn header");
    let scan = journal::scan(&disk.durable("journal").unwrap());
    assert_eq!((scan.payloads.len(), scan.tail), (1, Tail::Clean));
    assert_eq!(reopen(&disk).id(), "s3");
}

/// A small deterministic generator (xorshift64*), so every run builds the same histories without a crate.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len() as u64) as usize]
    }
}

/// The record and checkpoint names of the generated histories, one of each with text a frame must escape.
const RECORDS: [&str; 3] = ["r", "w", "odd \"record\" \\ \t é 😀"];
const CHECKPOINTS: [&str; 3] = ["k", "k2", "odd \"checkpoint\" \\ \n ü"];

/// One generated action under command ID `id`, with the proposal it needs under `{id}.p`: a move of a literal word or
/// of a word read from a record, an undo, a redo, a line switch, a checkpoint, a record write or a protection change.
/// Stale hints, unknown lines, records without letters, unconfirmed releases and protected orbits make some of them
/// refused. Any other durable action clears the redo stack, so a redo comes every other time one is possible.
/// Returns every receipt.
fn act(session: &Session, rng: &mut Rng, id: &str) -> Vec<Receipt> {
    let o = session.overview();
    let choice = if o.redo.is_some() && rng.below(2) == 0 { 4 } else { rng.below(9) };
    match choice {
        0..=2 => {
            let word = if choice == 2 {
                WordSource::Record(rng.pick(&RECORDS).to_owned())
            } else {
                let letters = (0..1 + rng.below(2))
                    .map(|_| {
                        let m = 1 + rng.below(24) as i32;
                        if rng.below(2) == 0 {
                            m
                        } else {
                            -m
                        }
                    })
                    .collect();
                WordSource::Literal(letters)
            };
            let proposed = shell(session, &format!("{id}.p"), Row::Propose { word });
            let preview = match &proposed.status {
                Status::Completed(c) => c.result.as_ref().and_then(|r| r.preview),
                _ => None,
            };
            let mut receipts = vec![proposed];
            if let Some(preview) = preview {
                receipts.push(shell(session, id, Row::Execute { preview }));
            }
            receipts
        }
        3 | 4 => {
            let redo = choice == 4;
            let hint = if redo { o.redo } else { o.undo }.unwrap_or(ActionHint { order: o.order, target: 0 });
            let confirm = rng.below(2) == 0;
            let row = if redo { Row::Redo { hint, confirm } } else { Row::Undo { hint, confirm } };
            vec![shell(session, id, row)]
        }
        5 => vec![shell(session, id, Row::SwitchLine { line: rng.below(o.lines as u64 + 1) as usize })],
        6 => vec![shell(session, id, Row::Checkpoint { name: rng.pick(&CHECKPOINTS).to_owned() })],
        7 => {
            let payload = if rng.below(4) == 0 { Json::Int(7) } else { letters(&[1 + rng.below(24) as i64]) };
            let record = rng.pick(&RECORDS);
            vec![set_record(session, id, record, payload)]
        }
        _ => {
            let orbits = (0..rng.below(3)).map(|_| rng.below(35) as i64).collect();
            vec![shell(session, id, Row::ProtectionSet { orbits, confirm: rng.below(2) == 0 })]
        }
    }
}

/// Opens a second session on the live session's disk and requires it to match: writable, the same durable view, the
/// receipt of every durable action issued, and `unknown_outcome` for every other command.
fn replays(live: &Session, disk: &MemDisk, issued: &[Receipt], case: &str) {
    let reopened = reopen(disk);
    assert_eq!(reopened.overview().read_only, None, "{case}");
    assert_eq!(durable(&reopened), durable(live), "{case}");
    for receipt in issued {
        let found = reopened.outcome(&receipt.command, &receipt.request);
        if is_durable(receipt) {
            assert_eq!(&found, receipt, "{case}");
        } else {
            assert_eq!(found.status, Status::UnknownOutcome, "{case}: {}", receipt.command);
        }
    }
}

/// What reads and proposals tell about a session's contents: each checkpoint's digest and the literal of each
/// record's word, or the code that refuses them.
fn contents(session: &Session, tag: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for name in CHECKPOINTS {
        seen.push(match session.open_snapshot(At::Checkpoint(name.into())) {
            Ok(opened) => opened.state().digest().to_owned(),
            Err(e) => e.code.to_owned(),
        });
    }
    for name in RECORDS {
        let read =
            shell(session, &format!("{tag}.read {name}"), Row::Propose { word: WordSource::Record(name.into()) });
        seen.push(match &read.status {
            Status::Completed(_) => session.preview().expect("the record's preview").literal,
            _ => code(&read).to_owned(),
        });
    }
    seen
}

/// Walks the undo stack to its bottom, then the redo stack to its top, with confirmation, and returns each step's
/// post digest or the code that stops the walk.
fn walk(session: &Session, tag: &str) -> Vec<String> {
    let mut seen = Vec::new();
    for redo in [false, true] {
        for n in 0.. {
            let o = session.overview();
            let Some(hint) = (if redo { o.redo } else { o.undo }) else { break };
            let row = if redo { Row::Redo { hint, confirm: true } } else { Row::Undo { hint, confirm: true } };
            let step = shell(session, &format!("{tag}.walk {redo} {n}"), row);
            match &step.status {
                Status::Committed(record) => seen.push(record.post.clone()),
                _ => {
                    seen.push(code(&step).to_owned());
                    break;
                }
            }
        }
    }
    seen
}

/// Ends a history: a reopen replays it, a copy of its journal without state objects holds the same contents and
/// walks the same undo and redo chain, and the journal the walk extended replays too.
fn finish(live: &Session, disk: &MemDisk, issued: &[Receipt], case: &str) {
    replays(live, disk, issued, case);
    let copy = reopen(&disk_with(&disk.durable("journal").unwrap()));
    assert_eq!(contents(&copy, "copy"), contents(live, "live"), "{case}");
    assert_eq!(walk(&copy, "copy"), walk(live, "live"), "{case}");
    let reopened = reopen(disk);
    assert_eq!(reopened.overview().read_only, None, "{case}: after the walk");
    assert_eq!(durable(&reopened), durable(live), "{case}: after the walk");
}

const HISTORIES: u64 = 4;
const ACTIONS: u64 = 16;
/// A second session replays the journal after every this many durable actions, and at the end.
const EVERY: u64 = 4;
/// The rows of the seven kinds of durable action, as their frames name them.
const ROWS: [&str; 7] = [
    "operation.execute",
    "session.undo",
    "session.redo",
    "session.switch_line",
    "session.checkpoint",
    "workspace.set",
    "protection.set",
];

/// ONE.70, KN5.23: every frame the live path writes replays. Seeded histories over all seven kinds, refusals
/// included, and a scripted one with the cases most likely to differ (undo of a protection change, a switch to the
/// active line, a record removed by undo, odd command, record and checkpoint names) are reopened along the way; the
/// reopen is never read-only and matches the live session in its durable view, receipts, contents and undo chain.
/// The seeded histories must write all seven kinds before their final walks.
#[test]
fn every_live_history_replays() {
    let mut rng = Rng(0x6c70_325f_6d36_0001);
    let mut written = std::collections::BTreeSet::new();
    for h in 0..HISTORIES {
        let (session, disk) = session();
        let (mut issued, mut durable_actions) = (Vec::new(), 0);
        for i in 0..ACTIONS {
            let id = if i % 5 == 4 { format!("odd \"{h}\" \\ é 😀 {i}") } else { format!("{h}.{i}") };
            let receipts = act(&session, &mut rng, &id);
            if receipts.iter().any(is_durable) {
                durable_actions += 1;
                if durable_actions % EVERY == 0 {
                    issued.extend(receipts);
                    replays(&session, &disk, &issued, &format!("history {h} action {i}"));
                    continue;
                }
            }
            issued.extend(receipts);
        }
        assert!(durable_actions > 0, "history {h} wrote nothing");
        let journal = String::from_utf8_lossy(&disk.durable("journal").unwrap()).into_owned();
        written.extend(ROWS.into_iter().filter(|row| journal.contains(&format!("\"{row}\""))));
        finish(&session, &disk, &issued, &format!("history {h}"));
    }
    assert_eq!(written.len(), ROWS.len(), "the seeded histories wrote only {written:?}");

    let (session, disk) = session();
    let odd = "odd \"id\" \\ \u{1} é 😀";
    let mut issued = vec![
        protect(&session, "p1", &[0, 1]),
        shell(&session, "u0", Row::Undo { hint: session.overview().undo.unwrap(), confirm: false }),
    ];
    assert_eq!(code(&issued[1]), code::CONFIRMATION_REQUIRED, "an undo that releases needs confirmation");
    issued.push(shell(&session, "u1", Row::Undo { hint: session.overview().undo.unwrap(), confirm: true }));
    issued.push(redo(&session, "r1"));
    issued.push(shell(&session, "p2", Row::ProtectionSet { orbits: Vec::new(), confirm: true }));
    issued.push(set_record(&session, "w1", RECORDS[2], letters(&[1, 2])));
    issued.push(set_record(&session, "w2", RECORDS[2], letters(&[3])));
    issued.push(undo(&session, "u2"));
    issued.push(undo(&session, "u3"));
    issued.push(commit(&session, "m1", &[1]));
    let active = session.overview().active_line;
    issued.push(shell(&session, "s1", Row::SwitchLine { line: active }));
    issued.push(shell(&session, odd, Row::Checkpoint { name: CHECKPOINTS[2].into() }));
    issued.push(commit(&session, &format!("{odd} m2"), &[2]));
    for receipt in &issued[2..] {
        assert!(is_durable(receipt), "{receipt:?}");
    }
    replays(&session, &disk, &issued, "scripted, before the end");
    finish(&session, &disk, &issued, "scripted");
}
