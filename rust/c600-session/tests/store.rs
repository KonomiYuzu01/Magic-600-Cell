//! The store: failures at each write (CT-KN4-02, R-06, KN5.23), a status that reads no storage (CT-KN4-03, R-13), and
//! the journal's torn and damaged tails (ONE.70). Store reads are counted, never timed.

mod common;

use c600_session::disk::{Fault, FsDisk, MemDisk};
use c600_session::journal::{self, Tail};
use c600_session::{code, At, Options, Row, Session, Status};
use common::*;

/// A clean run of a kind: the number of writes the action makes and the durable view after it.
fn clean_run(kind: &Kind) -> (u64, c600_session::Overview) {
    let (session, disk) = session();
    (kind.setup)(&session);
    let start = disk.writes();
    let receipt = (kind.act)(&session);
    assert!(matches!(receipt.status, Status::Committed(_) | Status::Completed(_)), "{}: {receipt:?}", kind.name);
    (disk.writes() - start, durable(&session))
}

/// CT-KN4-02 (whole): one failure at any write of any kind of durable action is a proved rollback. The receipt is
/// `failed` with `KN4:resource_limit.disk_full`, the session is wholly as before, and so is the session a reopen finds,
/// with or without a crash after the failure.
#[test]
fn one_failed_write_rolls_the_action_back_whole() {
    for kind in kinds() {
        let (writes, after) = clean_run(&kind);
        assert_eq!(writes, writes_of(&kind), "{}", kind.name);
        for index in 0..writes {
            for fault in [Fault::Error, Fault::Partial(5), Fault::Persisted] {
                for crash in [false, true] {
                    let case = format!("{} write {index} {fault:?} crash {crash}", kind.name);
                    let (session, disk) = session();
                    (kind.setup)(&session);
                    let before = session.overview();
                    disk.fail_write(disk.writes() + index, fault);
                    let receipt = (kind.act)(&session);
                    assert!(matches!(receipt.status, Status::Failed(_)), "{case}: {receipt:?}");
                    assert_eq!(code(&receipt), code::DISK_FULL, "{case}");
                    assert_eq!(session.overview(), before, "{case}");
                    assert_eq!(session.outcome("act", &receipt.request), receipt, "{case}");

                    if crash {
                        disk.crash();
                    } else {
                        disk.heal();
                    }
                    let reopened = reopen(&disk);
                    let mut expected = before.clone();
                    expected.revision.epoch = 0;
                    expected.pending_preview = None;
                    assert_eq!(durable(&reopened), expected, "{case}");
                    assert_ne!(durable(&reopened), after, "{case}");
                }
            }
        }
    }
}

/// CT-KN4-02 (part: a rollback that fails too), KN5.23: when the rollback fails too, nothing proves the fate. The
/// receipt is `unknown_outcome`, the session refuses every row until it is reopened, and the reopened session is wholly
/// before the action or wholly after it: after it exactly when the whole frame reached what the reopen reads. Each of
/// the three kinds of failure is tried at the append and at its sync.
#[test]
fn an_unproved_fate_is_unknown_and_read_only_until_reopened() {
    for kind in kinds() {
        let (writes, after) = clean_run(&kind);
        let (append, sync) = (writes - 2, writes - 1);
        for index in [append, sync] {
            for fault in [Fault::Error, Fault::Partial(5), Fault::Persisted] {
                for crash in [false, true] {
                    let case = format!("{} write {index} {fault:?} crash {crash}", kind.name);
                    let (session, disk) = session();
                    (kind.setup)(&session);
                    let before = durable(&session);
                    let at = disk.writes() + index;
                    disk.fail_write(at, fault);
                    disk.fail_from(at + 1);
                    let persisted = fault == Fault::Persisted;
                    let receipt = (kind.act)(&session);
                    assert_eq!(receipt.status, Status::UnknownOutcome, "{case}");
                    assert_eq!(session.overview().read_only, Some(code::COMMIT_FATE), "{case}");
                    assert_eq!(session.overview().revision.digest, before.revision.digest, "{case}");
                    assert_eq!(session.outcome("act", &receipt.request).status, Status::UnknownOutcome, "{case}");

                    disk.heal();
                    let writes_before = disk.writes();
                    let refused = shell(&session, "next", Row::Propose { word: literal(&[3]) });
                    assert_eq!(code(&refused), code::READ_ONLY_RECOVERY, "{case}");
                    let refused = set_record(&session, "next2", "r2", letters(&[3]));
                    assert_eq!(code(&refused), code::READ_ONLY_RECOVERY, "{case}");
                    assert_eq!(disk.writes(), writes_before, "{case}: a read-only session writes nothing");

                    let whole_frame_read = if crash { persisted && index == sync } else { persisted || index == sync };
                    if crash {
                        disk.crash();
                    } else {
                        disk.heal();
                    }
                    let reopened = reopen(&disk);
                    assert_eq!(reopened.overview().read_only, None, "{case}");
                    let expected = if whole_frame_read { &after } else { &before };
                    assert_eq!(&durable(&reopened), expected, "{case}");
                }
            }
        }
    }
}

/// CT-KN4-03 (whole), R-13: the status answers from memory. It reads no storage at 0, 50, 200 and 400 checkpoints, and
/// opening a session reads its journal once, whatever the history holds.
#[test]
fn status_reads_no_storage_at_any_history_size() {
    let (session, disk) = session();
    let mut made = 0;
    for (round, size) in [0usize, 50, 200, 400].into_iter().enumerate() {
        commit(&session, &format!("move{round}"), &[round as i32 + 1]);
        while made < size {
            let receipt = shell(&session, &format!("k{made}"), Row::Checkpoint { name: format!("k{made}") });
            assert!(matches!(receipt.status, Status::Completed(_)), "{receipt:?}");
            made += 1;
        }
        let reads = disk.reads();
        let overview = session.overview();
        assert_eq!(overview.checkpoints, size);
        assert_eq!(disk.reads(), reads, "the status read storage at {size} checkpoints");

        let reads = disk.reads();
        let reopened = reopen(&disk);
        assert_eq!(disk.reads(), reads + 1, "opening read more than the journal at {size} checkpoints");
        let reads = disk.reads();
        assert_eq!(durable(&reopened), durable(&session));
        assert_eq!(disk.reads(), reads, "the reopened status read storage at {size} checkpoints");
    }
}

/// A journal that ends in part of a frame, as an append torn by a crash leaves it.
fn torn(disk: &MemDisk, keep: usize) {
    let mut bytes = disk.durable("journal").expect("a journal");
    let next = journal::frame(b"{\"next\":1}");
    bytes.extend_from_slice(&next[..keep]);
    disk.overwrite("journal", &bytes);
}

/// ONE.70: opening cuts a torn final frame and syncs the cut; the session is the one before the torn append.
#[test]
fn a_torn_final_frame_is_cut_at_open() {
    for keep in [1, 7, 8, 20] {
        let (session, disk) = session();
        commit(&session, "c1", &[1]);
        let before = durable(&session);
        let length = disk.durable("journal").unwrap().len();
        torn(&disk, keep);
        assert_eq!(journal::scan(&disk.durable("journal").unwrap()).tail, Tail::Torn);
        let reopened = reopen(&disk);
        assert_eq!(reopened.overview().read_only, None);
        assert_eq!(durable(&reopened), before);
        assert_eq!(disk.durable("journal").unwrap().len(), length, "the cut is durable");
        assert!(matches!(commit(&reopened, "c2", &[2]).status, Status::Committed(_)));
    }
}

/// ONE.70, KN5.23: any other damage opens the session read-only at the last whole frame before it. A damaged length
/// is damage, not a torn append, even in the last frame.
#[test]
fn damage_opens_read_only() {
    let (session, disk) = session();
    commit(&session, "c1", &[1]);
    let at_first = durable(&session);
    commit(&session, "c2", &[2]);
    let journal = disk.durable("journal").unwrap();
    let frames = journal::scan(&journal);
    assert_eq!((frames.payloads.len(), frames.tail), (3, Tail::Clean));
    // The offset of the last frame: the header frame, then the first commit.
    let last = journal.len() - (8 + frames.payloads[2].len() + 32);

    let damaged = |offset: usize| {
        let mut bytes = journal.clone();
        flip(&mut bytes, offset);
        disk.overwrite("journal", &bytes);
        let reopened = reopen(&disk);
        assert_eq!(reopened.overview().read_only, Some(code::COMMIT_FATE), "byte {offset}");
        assert_eq!(durable(&reopened).revision, at_first.revision, "byte {offset}");
        let refused = shell(&reopened, "next", Row::Propose { word: literal(&[3]) });
        assert_eq!(code(&refused), code::READ_ONLY_RECOVERY);
        assert_eq!(disk.durable("journal").unwrap().len(), journal.len(), "byte {offset}: damage is never cut");
    };
    damaged(last); // the length
    damaged(last + 5); // its complement
    damaged(last + 8 + 3); // the payload
    damaged(journal.len() - 1); // the checksum
}

/// A session on a directory: written, reopened and read back through the file system.
#[test]
fn a_session_on_a_directory_reopens() {
    let root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("c600-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let disk = FsDisk::new(&root).expect("a store directory");
    let session = Session::create(kernel(), Box::new(disk), "fs1", Options::default()).expect("created");
    commit(&session, "c1", &[1, 2]);
    shell(&session, "k1", Row::Checkpoint { name: "k1".into() });
    set_record(&session, "w1", "r", letters(&[5]));
    let before = durable(&session);
    drop(session);

    let reopened =
        Session::open(kernel(), Box::new(FsDisk::new(&root).unwrap()), Options::default()).expect("reopened");
    assert_eq!(durable(&reopened), before);
    let checkpoint = reopened.open_snapshot(At::Checkpoint("k1".into())).expect("the checkpoint's state object");
    assert_eq!(checkpoint.state().digest(), before.revision.digest);
    drop(reopened);
    std::fs::remove_dir_all(&root).expect("the directory is removed");
}
