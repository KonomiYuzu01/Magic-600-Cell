//! The commit path of chapter 08 section 7 from step 3: tokens (KN5.14, KN5.15), the commit lane (KN5.17), previews
//! and their freshness (KN0.8, KN5.9 to KN5.11), undo and redo with action hints (KN5.19, LI.9), receipts and outcome
//! lookup (KN5.21 to KN5.23, LI.6, LI.30). Contract cases: CT-KN5-01 with PHS.19's preview-swap row, CT-KN5-04,
//! CT-KN5-06, CT-KN5-11 and LI.33's gate-order trace.

mod common;

use c600_kernel::codes;
use c600_run::meter::code::NOT_GRANTED;
use c600_session::disk::MemDisk;
use c600_session::{code, ActionHint, CompletionKind, Envelope, Origin, Row, Session, Status, WordSource};
use common::*;

/// An envelope from the shell with a token the shell stub minted for it now.
fn minted(session: &Session, command: &str, row: Row) -> Envelope {
    let mut e = envelope(session, command, Origin::Shell, row);
    e.token = Some(session.shell().mint(command, &e.row).unwrap_or_else(|e| panic!("no token: {e}")));
    e
}

/// Delivers an envelope that must be refused with `expected` and change nothing: not the state, not the counters,
/// not the pending preview, not the disk.
fn refused(session: &Session, disk: &MemDisk, envelope: Envelope, expected: &str) {
    let (before, preview, writes) = (session.overview(), session.preview(), disk.writes());
    let receipt = session.invoke(envelope);
    assert_eq!(code(&receipt), expected, "{receipt:?}");
    assert!(matches!(receipt.status, Status::Refused(_)), "{receipt:?}");
    assert_eq!((session.overview(), session.preview(), disk.writes()), (before, preview, writes));
}

/// CT-KN5-01 (whole), PHS.19: a preview replaced after it was presented and before the commit is refused by the token's
/// binding, and nothing changes. A new preview of the same word is still another preview. CT-ONE-04 (part: with B
/// received, a token bound to A is refused; not run: minting from the presented frame, which needs the shell and LP3's
/// view). CT-ONE-03 (part: a word commit with no fresh preview gets no token, `KN5:stale.no_fresh_preview`).
#[test]
fn a_preview_swapped_before_the_commit_is_refused() {
    let (session, disk) = session();
    let p1 = propose(&session, "p1", literal(&[1]));
    let c = minted(&session, "C", Row::Execute { preview: p1 });
    let d = minted(&session, "D", Row::Execute { preview: p1 });
    let p2 = propose(&session, "p2", literal(&[2]));
    assert_ne!(p1, p2);
    refused(&session, &disk, c, code::PREVIEW_REPLACED);

    let p3 = propose(&session, "p3", literal(&[1]));
    assert_ne!(p3, p1);
    refused(&session, &disk, d, code::PREVIEW_REPLACED);
    let e = session.shell().mint("E", &Row::Execute { preview: p1 }).unwrap_err();
    assert_eq!(e.code, code::NO_FRESH_PREVIEW, "step 1: the stub mints only for the presented, fresh preview");

    let r = shell(&session, "F", Row::Execute { preview: p3 });
    assert_eq!(committed(&r).post, session.overview().revision.digest);
}

/// The LI.33 setting: moves, a word saved in record `w`, a protection change, then proposal Q of the word in `w`.
/// Returns the session, its disk, preview p and the digest p starts from.
fn trace() -> (Session, MemDisk, u64, String) {
    let (session, disk) = session();
    commit(&session, "m1", &[1]);
    commit(&session, "m2", &[2]);
    set_record(&session, "w0", "w", letters(&[3, 4]));
    protect(&session, "pr", &[]);
    let p = propose(&session, "Q", WordSource::Record("w".into()));
    let start = digest(&session);
    (session, disk, p, start)
}

/// LI.33's gate-order trace, with CT-ONE-05 (whole) and CT-ONE-11 (part: refusals at steps 3, 5 and 6). An unread
/// workspace edit leaves p fresh; C commits and discards p; an undo restores the start digest at a new state; an
/// identical retry of C returns its receipt before any token check; a new invocation with C's token fails step 3; D
/// fails step 5. In separate runs, an edit of the record p read, or of the protection, makes D fail step 6. Every
/// refusal leaves the state unchanged.
#[test]
fn li_33_gate_order() {
    let (session, disk, p, start) = trace();
    let base = session.overview().revision;
    let preview = session.preview().expect("p is pending");
    assert!(preview.fresh && preview.id == p && preview.start == start);

    // K, an unread keymap save, raises workspace only; p stays fresh.
    set_record(&session, "K", "keymap", letters(&[9]));
    let after_k = session.overview().revision;
    assert_eq!(
        (after_k.state, after_k.workspace, after_k.protection),
        (base.state, base.workspace + 1, base.protection)
    );
    assert!(session.preview().unwrap().fresh);

    // Two explicit solver actions mint C and D for p; D is held.
    let c = minted(&session, "C", Row::Execute { preview: p });
    let d = minted(&session, "D", Row::Execute { preview: p });
    let c_receipt = session.invoke(c.clone());
    let record = committed(&c_receipt).clone();
    assert_eq!((record.pre.as_str(), record.revision.state), (start.as_str(), base.state + 1));
    assert_eq!(session.preview(), None, "the commit discards p");

    // U restores the start digest at a new state.
    let u = undo(&session, "U");
    assert_eq!(committed(&u).revision.state, base.state + 2);
    assert_eq!(digest(&session), start);

    // The identical retry of C answers C's receipt before any token check (LI.6).
    let order = session.overview().order;
    assert_eq!(session.invoke(c.clone()), c_receipt);
    assert_eq!(session.overview().order, order);

    // A new invocation N with C's consumed token fails step 3.
    let mut n = c.clone();
    n.command = "N".into();
    refused(&session, &disk, n, code::TOKEN_USED);

    // D passes step 3 with its unused token and fails step 5: p was discarded, and equal digests do not restore it.
    refused(&session, &disk, d, code::PREVIEW_REPLACED);

    // Separate runs before C or D: a draft edit E of the record p read, or a protection edit P.
    for edit in ["E", "P"] {
        let (session, disk, p, _) = trace();
        let d = minted(&session, "D", Row::Execute { preview: p });
        let r = if edit == "E" { set_record(&session, "E", "w", letters(&[5])) } else { protect(&session, "P", &[]) };
        assert!(matches!(r.status, Status::Completed(_)), "{r:?}");
        assert_eq!(session.preview().map(|p| p.fresh), Some(false), "{edit}");
        refused(&session, &disk, d, code::REVISION_CHANGED);
    }
}

/// CT-KN5-04 (whole), LI.6: the same command ID with other content is refused; an identical retry returns the stored
/// receipt and commits nothing more.
#[test]
fn a_retried_delivery_commits_at_most_once() {
    let (session, disk) = session();
    let p = propose(&session, "p", literal(&[1]));
    let c = minted(&session, "C", Row::Execute { preview: p });
    let first = session.invoke(c.clone());
    committed(&first);
    let (order, writes) = (session.overview().order, disk.writes());
    for _ in 0..3 {
        assert_eq!(session.invoke(c.clone()), first);
    }
    assert_eq!((session.overview().order, disk.writes()), (order, writes));

    let mut other = c.clone();
    other.row = Row::Execute { preview: p + 1 };
    refused(&session, &disk, other, code::COMMAND_ID_REUSED);
    refused(
        &session,
        &disk,
        envelope(&session, "C", Origin::Shell, Row::Propose { word: literal(&[2]) }),
        code::COMMAND_ID_REUSED,
    );
    assert_eq!(session.outcome("C", &first.request), first, "a refused reuse does not replace the stored receipt");
}

/// CT-KN5-04 (part: a duplicate while the commit is written), KN5.17: a duplicate delivered while the commit is being
/// written gets the stored provisional receipt and commits nothing; another commit is refused while the lane is taken;
/// afterwards both lookups answer the commit.
#[test]
fn a_duplicate_during_the_commit_is_not_a_second_commit() {
    let (session, disk) = session();
    let p = propose(&session, "p", literal(&[1]));
    let c = minted(&session, "C", Row::Execute { preview: p });
    // Minted now: the stub reads the session core, which the commit holds while it writes.
    let other = minted(&session, "U", Row::Undo { hint: ActionHint { order: 0, target: 0 }, confirm: false });
    let order = session.overview().order;
    disk.hold_write(disk.writes());
    let worker = {
        let (session, c) = (session.clone(), c.clone());
        std::thread::spawn(move || session.invoke(c))
    };
    disk.wait_held();
    // Only the admission state is touched here: the commit holds the session core while it writes.
    let during = session.invoke(c.clone());
    assert_eq!(during.status, Status::Accepted);
    assert_eq!(code(&session.invoke(other)), code::COMMIT_IN_FLIGHT);
    disk.release();
    let first = worker.join().expect("the commit thread");
    committed(&first);
    assert_eq!(session.overview().order, order + 1);
    assert_eq!(session.invoke(c.clone()), first);
    assert_eq!(session.outcome("C", &first.request), first);
}

/// CT-KN5-06 (whole), R-10: what is committed and journalled is the preview's own word. A change the caller makes after
/// the preview, to its own copy or to the record the preview read, changes nothing committed; a changed record makes
/// the preview stale instead.
#[test]
fn a_mutation_after_the_preview_changes_nothing_committed() {
    let (session, disk) = session();
    let mut word = vec![1, 2, 3];
    let p = propose(&session, "p", WordSource::Literal(word.clone()));
    let end = session.preview().unwrap().end.expect("a clear preview");
    word.clear();
    word.push(7);
    assert_eq!(committed(&shell(&session, "C", Row::Execute { preview: p })).post, end);

    set_record(&session, "w1", "w", letters(&[4, 5]));
    let q = propose(&session, "q", WordSource::Record("w".into()));
    let q_end = session.preview().unwrap().end.unwrap();
    let c2 = minted(&session, "C2", Row::Execute { preview: q });
    set_record(&session, "w2", "w", letters(&[6]));
    refused(&session, &disk, c2, code::REVISION_CHANGED);

    let r = propose(&session, "r", WordSource::Record("w".into()));
    let r_end = session.preview().unwrap().end.unwrap();
    assert_ne!(r_end, q_end);
    shell(&session, "C3", Row::Execute { preview: r });
    set_record(&session, "w3", "w", letters(&[1, 1, 1]));
    let reopened = reopen(&disk);
    assert_eq!(digest(&reopened), r_end, "the journal holds the literal letters, not the record's name");
}

/// CT-KN5-11 (whole), KN5.19, LI.9: an undo whose hint names a superseded action, or a stale order, is refused as
/// stale.
#[test]
fn an_undo_naming_a_superseded_action_is_refused() {
    let (session, disk) = session();
    commit(&session, "a", &[1]);
    let stale = session.overview().undo.unwrap();
    commit(&session, "b", &[2]);
    refused(&session, &disk, minted(&session, "u1", Row::Undo { hint: stale, confirm: false }), code::UNDO_SUPERSEDED);

    // The right target with a stale order: a checkpoint, which is not undoable, moved the order on.
    let hint = session.overview().undo.unwrap();
    shell(&session, "k", Row::Checkpoint { name: "k".into() });
    assert_eq!(session.overview().undo.unwrap().target, hint.target);
    refused(&session, &disk, minted(&session, "u2", Row::Undo { hint, confirm: false }), code::UNDO_SUPERSEDED);

    committed(&undo(&session, "u3"));
    let redo_hint = session.overview().redo.unwrap();
    set_record(&session, "w", "r", letters(&[1]));
    assert_eq!(session.overview().redo, None, "a new action clears the redo stack");
    let r = minted(&session, "r1", Row::Redo { hint: redo_hint, confirm: false });
    refused(&session, &disk, r, code::UNDO_SUPERSEDED);
}

/// KN5.20, LI.26: one undo order across the puzzle and the workspace. A workspace action is undone like a move, and
/// releasing a protected orbit needs confirmation, by undo as by `protection.set`.
#[test]
fn undo_and_redo_cover_moves_records_and_protection() {
    let (session, disk) = session();
    let solved = digest(&session);
    commit(&session, "a", &[1]);
    let moved = digest(&session);
    set_record(&session, "w", "r", letters(&[2]));
    protect(&session, "p", &[0, 1]);
    assert_eq!(session.overview().protected, vec![0, 1]);

    let release = Row::ProtectionSet { orbits: vec![0], confirm: false };
    refused(&session, &disk, envelope(&session, "p2", Origin::Shell, release), code::CONFIRMATION_REQUIRED);
    let hint = session.overview().undo.unwrap();
    refused(&session, &disk, minted(&session, "u0", Row::Undo { hint, confirm: false }), code::CONFIRMATION_REQUIRED);
    committed(&shell(&session, "u1", Row::Undo { hint, confirm: true }));
    assert_eq!(session.overview().protected, Vec::<usize>::new());

    let workspace = session.overview().revision.workspace;
    committed(&undo(&session, "u2"));
    assert_eq!(session.overview().revision.workspace, workspace + 1, "a workspace undo advances workspace");
    committed(&undo(&session, "u3"));
    assert_eq!(digest(&session), solved);
    assert_eq!(session.overview().undo, None);

    committed(&redo(&session, "r1"));
    assert_eq!(digest(&session), moved);
    committed(&redo(&session, "r2"));
    committed(&redo(&session, "r3"));
    assert_eq!(session.overview().protected, vec![0, 1], "a redo that protects needs no confirmation");

    let reopened = reopen(&disk);
    assert_eq!(durable(&reopened), durable(&session));
}

/// CT-ONE-03 (part: a commit row without a token, or with a forged, foreign, misdirected or changed one; not run: a
/// live twist, which waits for A6's exact rotation requests, and a crossing commit, which waits for M5's jumbled
/// puzzle), KN5.14, KN5.15: a commit row needs a genuine token of this session, minted for this invocation, used once.
#[test]
fn commit_rows_need_a_genuine_unused_token() {
    let (session, disk) = session();
    commit(&session, "a", &[1]);
    let hint = session.overview().undo.unwrap();
    let row = Row::Undo { hint, confirm: false };

    refused(&session, &disk, envelope(&session, "u0", Origin::Shell, row.clone()), code::TOKEN_INVALID);
    let mut forged = minted(&session, "u1", row.clone());
    forged.token.as_mut().unwrap().mac[0] ^= 1;
    refused(&session, &disk, forged, code::TOKEN_INVALID);
    let (other, _) = common::session();
    let mut foreign = envelope(&session, "u2", Origin::Shell, row.clone());
    foreign.token = Some(other.shell().mint("u2", &row).unwrap());
    refused(&session, &disk, foreign, code::TOKEN_INVALID);

    let mut misdirected = envelope(&session, "u4", Origin::Shell, row.clone());
    misdirected.token = Some(session.shell().mint("u3", &row).unwrap());
    refused(&session, &disk, misdirected, code::TOKEN_MISMATCH);
    let mut changed = minted(&session, "u5", row.clone());
    changed.row = Row::Undo { hint, confirm: true };
    refused(&session, &disk, changed, code::TOKEN_MISMATCH);

    let e = session.shell().mint("k", &Row::Checkpoint { name: "k".into() }).unwrap_err();
    assert_eq!(e.code, code::TOKEN_INVALID, "the stub mints only for commit rows");
    committed(&shell(&session, "u6", row));
}

/// CT-ONE-03 (part: a token bound to another row), KN5.14: a token binds the row it was minted for. An undo's token
/// does not carry an execute, and an execute's token does not carry an undo, though both are genuine and unused.
#[test]
fn a_token_fits_only_the_row_it_was_minted_for() {
    let (session, disk) = session();
    commit(&session, "a", &[1]);
    let p = propose(&session, "p", literal(&[2]));
    let undo_row = Row::Undo { hint: session.overview().undo.unwrap(), confirm: false };
    let execute_row = Row::Execute { preview: p };

    let mut e = envelope(&session, "x1", Origin::Shell, execute_row.clone());
    e.token = Some(session.shell().mint("x1", &undo_row).unwrap());
    refused(&session, &disk, e, code::TOKEN_MISMATCH);
    let mut u = envelope(&session, "x2", Origin::Shell, undo_row);
    u.token = Some(session.shell().mint("x2", &execute_row).unwrap());
    refused(&session, &disk, u, code::TOKEN_MISMATCH);
    committed(&shell(&session, "x3", execute_row));
}

/// LI.6, KN5.14: a token for a commit row without a preview binds the `state` and `protection` revisions the shell
/// resolved the action against. A commit, a protection change, or an undo back to the same digest at a new state
/// makes it stale; a workspace record write does not.
#[test]
fn a_token_without_a_preview_binds_the_revisions_it_was_resolved_at() {
    let (session, disk) = session();
    commit(&session, "a", &[1]);
    undo(&session, "u");
    commit(&session, "b", &[2]);
    let switch = || Row::SwitchLine { line: 0 };

    let s1 = minted(&session, "s1", switch());
    commit(&session, "c", &[3]);
    refused(&session, &disk, s1, code::REVISION_CHANGED);

    let s2 = minted(&session, "s2", switch());
    protect(&session, "p", &[]);
    refused(&session, &disk, s2, code::REVISION_CHANGED);

    let s3 = minted(&session, "s3", switch());
    let before = digest(&session);
    commit(&session, "d", &[4]);
    undo(&session, "u2");
    assert_eq!(digest(&session), before, "the same digest at a new state");
    refused(&session, &disk, s3, code::REVISION_CHANGED);

    let s4 = minted(&session, "s4", switch());
    set_record(&session, "w", "r", letters(&[5]));
    committed(&session.invoke(s4));
}

/// KN5.15, ONE.77: a program delivers no commit row, even with a genuine token, and no session row but its jobs.
#[test]
fn a_program_delivers_no_commit_and_no_session_row() {
    let (session, disk) = session();
    commit(&session, "a", &[1]);
    let p = propose(&session, "p", literal(&[2]));
    let hint = session.overview().undo.unwrap();
    for (command, row) in [
        ("x1", Row::Execute { preview: p }),
        ("x2", Row::Undo { hint, confirm: true }),
        ("x3", Row::SwitchLine { line: 0 }),
    ] {
        let mut e = envelope(&session, command, Origin::Program, row.clone());
        e.token = Some(session.shell().mint(command, &row).unwrap());
        refused(&session, &disk, e, code::COMMIT_ROW_REFUSED);
    }
    for (command, row) in [
        ("y1", Row::Propose { word: literal(&[3]) }),
        ("y2", Row::Checkpoint { name: "k".into() }),
        ("y3", Row::WorkspaceSet { record: "r".into(), payload: letters(&[1]), provenance: manual() }),
        ("y4", Row::ProtectionSet { orbits: vec![0], confirm: true }),
    ] {
        refused(&session, &disk, envelope(&session, command, Origin::Program, row), NOT_GRANTED);
    }
}

/// KN5.9 to KN5.11: a protection conflict still gets a preview, which execute refuses at step 8; a mechanically
/// refused word builds none and leaves the pending preview as it was.
#[test]
fn previews_follow_kn5_9_to_kn5_11() {
    let (session, disk) = session();
    let p = propose(&session, "p", literal(&[1]));
    let r = shell(&session, "bad", Row::Propose { word: literal(&[1201]) });
    assert_eq!(code(&r), codes::RECIPE_SHAPE);
    assert_eq!(session.preview().map(|i| (i.id, i.fresh)), Some((p, true)));

    let every: Vec<i64> = (0..35).collect();
    protect(&session, "all", &every);
    let q = propose(&session, "q", literal(&[1]));
    assert_eq!(session.preview().unwrap().end, None, "a preview that would move a protected orbit has no end state");
    refused(&session, &disk, minted(&session, "C", Row::Execute { preview: q }), codes::ORBIT_WOULD_MOVE);
    let net = propose(&session, "n", literal(&[1, -1]));
    let r = shell(&session, "C2", Row::Execute { preview: net });
    assert_eq!(committed(&r).pre, committed(&r).post, "a word whose net map is the identity commits under protection");

    let receipt = shell(&session, "q2", Row::Propose { word: literal(&[2]) });
    assert!(matches!(&receipt.status, Status::Completed(c) if c.kind == CompletionKind::PreviewReady));
}

/// ONE.75 with the active protection: a switch whose path would move a protected orbit is refused.
#[test]
fn a_switch_keeps_the_active_protection() {
    let (session, disk) = session();
    commit(&session, "a", &[1]);
    undo(&session, "u");
    commit(&session, "b", &[2]);
    let every: Vec<i64> = (0..35).collect();
    protect(&session, "all", &every);
    refused(&session, &disk, minted(&session, "s", Row::SwitchLine { line: 0 }), codes::ORBIT_WOULD_MOVE);
}

/// CT-ONE-21 (part: durable receipts survive a restart, a lost transient outcome answers `unknown_outcome`, and a
/// delivery from the old epoch is refused), KN5.21 to KN5.23, LI.30: a durable action's receipt is rebuilt from the
/// journal and survives a reopen whole; a transient receipt does not, and the lookup reports missing knowledge as
/// `unknown_outcome`. A delivery from the old epoch is refused as stale.
#[test]
fn outcomes_are_looked_up_across_a_reopen() {
    let (session, disk) = session();
    let c = commit(&session, "C", &[1]);
    let w = set_record(&session, "W", "r", letters(&[2]));
    let k = shell(&session, "K", Row::Checkpoint { name: "k".into() });
    let q = shell(&session, "Q", Row::Propose { word: literal(&[2]) });
    let u = undo(&session, "U");
    let old = envelope(&session, "X", Origin::Shell, Row::Propose { word: literal(&[3]) });

    let reopened = reopen(&disk);
    assert_ne!(reopened.epoch(), session.epoch());
    for receipt in [&c, &w, &k, &u] {
        assert_eq!(&reopened.outcome(&receipt.command, &receipt.request), receipt);
    }
    assert_eq!(reopened.outcome("Q", &q.request).status, Status::UnknownOutcome);
    assert_eq!(code(&reopened.outcome("C", "another request")), code::COMMAND_ID_REUSED);
    assert_eq!(code(&reopened.invoke(old)), code::EPOCH_CHANGED);
    let mut stranger = envelope(&reopened, "Y", Origin::Shell, Row::Propose { word: literal(&[3]) });
    stranger.session = "s2".into();
    assert_eq!(code(&reopened.invoke(stranger)), code::UNKNOWN_SESSION);
}
