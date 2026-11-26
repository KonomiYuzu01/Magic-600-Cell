//! LP2 conformance tests of chapter 08's recovery rows against the session store (chapter 12, section 15.3):
//! CT-ONE-08, CT-ONE-11, CT-ONE-20, CT-ONE-21 and CT-ONE-22. Each test's comment names its ID first, then whether the
//! whole definition runs, or which part runs and what does not and why (PHS.62, PHS.63). Failures are injected on the
//! memory disk; an engine restart is the session dropped and reopened under a new epoch.

mod common;

use c600_kernel::{codes, Derivation, Derived, ProtectedSet, Snapshot};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::value::Value;
use c600_session::disk::{Fault, MemDisk};
use c600_session::journal;
use c600_session::{
    code, At, CompletionKind, Envelope, Input, JobKind, JobOutcome, Lane, Origin, Overview, PreviewInfo, Receipt, Row,
    Session, Status, WordSource,
};
use common::*;

/// An envelope from the shell with a token the shell stub minted for it now.
fn minted(session: &Session, command: &str, row: Row) -> Envelope {
    let mut e = envelope(session, command, row);
    e.token = Some(session.shell().mint(command, &e.row).unwrap_or_else(|e| panic!("no token: {e}")));
    e
}

/// The number of whole frames in the durable journal.
fn frames(disk: &MemDisk) -> usize {
    journal::scan(&disk.durable("journal").expect("a journal")).payloads.len()
}

/// The kernel's derivation of a word from a base state, with nothing protected.
fn derive(base: &Snapshot, letters: &[i32]) -> Derived {
    let wide: Vec<i64> = letters.iter().map(|&m| i64::from(m)).collect();
    let recipe = kernel().word(&wide).expect("a legal word");
    match kernel().derive(base, &recipe, &ProtectedSet::none()).expect("a derivation") {
        Derivation::Derived(d) => d,
        Derivation::Refused(r) => panic!("the word was refused: {r:?}"),
    }
}

/// CT-ONE-08 (whole): a derived state's word, proposed after the head moved, shows its effect at the current revision,
/// and the derived state is not read (ONE.26, KN5.10, KN5.13). The kernel derives the word on a snapshot of the head;
/// another move commits; the word is proposed as a literal, which is all a proposal carries. The preview starts at the
/// new head and ends where the kernel takes the word from there, not at the derived state, which is dropped before
/// the proposal so that nothing can read it; the commit publishes that end.
#[test]
fn ct_one_08_a_derived_word_shows_its_effect_at_the_current_revision() {
    const WORD: [i32; 2] = [2, 3];
    let (session, _disk) = session();
    commit(&session, "m1", &[1]);
    let base = session.open_snapshot(At::Current).expect("a snapshot of the head");
    let stale = derive(base.state(), &WORD);
    assert_eq!(stale.base, base.revision.digest);
    let (stale_end, stale_word) = (stale.state.digest().to_owned(), stale.word.clone());
    drop(stale);

    commit(&session, "m2", &[5]);
    let head = session.open_snapshot(At::Current).expect("a snapshot of the new head");
    assert_ne!(head.revision.digest, base.revision.digest, "the head moved after the derivation");
    let now = derive(head.state(), &WORD);
    assert_ne!(now.state.digest(), stale_end, "from the new head the word ends elsewhere");

    let p = propose(&session, "w", literal(&WORD));
    let shown = session.preview().expect("the preview of the word");
    assert_eq!((shown.id, shown.fresh), (p, true));
    assert_eq!(shown.literal, stale_word, "the same literal word");
    assert_eq!(shown.start, head.revision.digest, "the word replays at the current revision");
    assert_eq!(shown.end.as_deref(), Some(now.state.digest()), "the preview shows what the word does now");
    assert_ne!(shown.end.as_deref(), Some(stale_end.as_str()), "the derived state is not the preview's end");

    let executed = shell(&session, "x", Row::Execute { preview: p });
    assert_eq!(committed(&executed).pre, head.revision.digest);
    assert_eq!(committed(&executed).post, now.state.digest());
    assert_ne!(digest(&session), stale_end, "the derived state was not adopted");
}

/// Everything a refused commit must leave as it was: the overview with every revision counter and the state digest,
/// the pending preview with its digests and freshness, the number of writes, and the durable journal.
fn everything(session: &Session, disk: &MemDisk) -> (Overview, Option<PreviewInfo>, u64, Option<Vec<u8>>) {
    (session.overview(), session.preview(), disk.writes(), disk.durable("journal"))
}

/// Delivers an envelope that one step of the commit path must refuse with `expected`, checks that the refusal left
/// everything as it was, and returns the envelope with its receipt for a retry later.
fn refused_at(step: u8, session: &Session, disk: &MemDisk, envelope: Envelope, expected: &str) -> (Envelope, Receipt) {
    let before = everything(session, disk);
    let receipt = session.invoke(Origin::Shell, envelope.clone());
    assert!(matches!(&receipt.status, Status::Refused(e) if e.code == expected), "step {step}: {receipt:?}");
    assert_eq!(everything(session, disk), before, "step {step}: the refusal changed something");
    (envelope, receipt)
}

/// CT-ONE-11 (part: an `operation.execute` refused at steps 1, 3, 4, 5, 6 and 8 of chapter 08 section 7, and one
/// failed by step 9's proved rollback, leaves the overview with every counter and digest, the pending preview, the
/// write count and the durable journal as they were, and exactly one later commit succeeds; not run here: step 7
/// (`KN5:consistency.state_hash_mismatch`), which this store cannot reach because step 6 refuses every head change
/// first (`rust/README.md`, M6), and a refused live twist (ONE.46), because `twist.apply` is not an M6 row; both run in
/// `c600-command`'s `twist_apply.rs`). Steps 2 and 10
/// have no refusal. Step 4 holds the first commit's write on the memory disk; while it is held, the commit holds the
/// session core, so the refusal is compared on the disk, and the whole view is compared after the rollback.
#[test]
fn ct_one_11_a_commit_refused_at_each_step_changes_nothing() {
    let (session, disk) = session();
    commit(&session, "m1", &[1]);
    let mut refusals = Vec::new();

    // Step 1: the stub mints only for the presented, fresh preview.
    let p = propose(&session, "p", literal(&[2]));
    let before = everything(&session, &disk);
    let e = session.shell().mint("s1", &Row::Execute { preview: p + 1 }).unwrap_err();
    assert_eq!(e.code, code::NO_FRESH_PREVIEW, "step 1");
    assert_eq!(everything(&session, &disk), before, "step 1");

    // Step 3: no token, and a token whose authenticator fails.
    let tokenless = envelope(&session, "s3.none", Row::Execute { preview: p });
    refusals.push(refused_at(3, &session, &disk, tokenless, code::TOKEN_INVALID));
    let mut forged = minted(&session, "s3.forged", Row::Execute { preview: p });
    forged.token.as_mut().expect("a token").mac[0] ^= 1;
    refusals.push(refused_at(3, &session, &disk, forged, code::TOKEN_INVALID));

    // Step 5: a genuine token delivered under another command ID. Step 3 saw it, so it is used from then on.
    let genuine = minted(&session, "s5.mismatch", Row::Execute { preview: p });
    let mut elsewhere = genuine.clone();
    elsewhere.command = "s5.elsewhere".into();
    refusals.push(refused_at(5, &session, &disk, elsewhere, code::TOKEN_MISMATCH));
    refusals.push(refused_at(3, &session, &disk, genuine, code::TOKEN_USED));

    // Step 5: the token's preview was replaced.
    let for_p = minted(&session, "s5.replaced", Row::Execute { preview: p });
    let q = propose(&session, "q", literal(&[3]));
    refusals.push(refused_at(5, &session, &disk, for_p, code::PREVIEW_REPLACED));

    // Step 6: a protection change ends the preview's freshness; step 1 then mints nothing for it.
    let for_q = minted(&session, "s6", Row::Execute { preview: q });
    protect(&session, "pr", &[]);
    refusals.push(refused_at(6, &session, &disk, for_q, code::REVISION_CHANGED));
    let before = everything(&session, &disk);
    let e = session.shell().mint("s1.stale", &Row::Execute { preview: q }).unwrap_err();
    assert_eq!(e.code, code::NO_FRESH_PREVIEW, "step 1 for a stale preview");
    assert_eq!(everything(&session, &disk), before, "step 1 for a stale preview");

    // Step 8: every orbit protected; the preview has no end, and the recheck refuses it.
    let every: Vec<i64> = (0..35).collect();
    protect(&session, "all", &every);
    let r = propose(&session, "r", literal(&[2]));
    assert_eq!(session.preview().expect("a preview").end, None, "a protected orbit would move");
    let for_r = minted(&session, "s8", Row::Execute { preview: r });
    refusals.push(refused_at(8, &session, &disk, for_r, codes::ORBIT_WOULD_MOVE));
    protect(&session, "free", &[]);

    // Steps 4 and 9: a second commit arrives while the first is being written, and the first's append fails.
    let s = propose(&session, "s", literal(&[2]));
    let first = minted(&session, "s9", Row::Execute { preview: s });
    // Minted now: the stub reads the session core, which the commit holds while it writes.
    let second = minted(&session, "s4", Row::Execute { preview: s });
    let before = everything(&session, &disk);
    let at = disk.writes();
    disk.hold_write(at);
    disk.fail_write(at, Fault::Error);
    let worker = {
        let (session, first) = (session.clone(), first.clone());
        std::thread::spawn(move || session.invoke(Origin::Shell, first))
    };
    disk.wait_held();
    let (writes, journal) = (disk.writes(), disk.durable("journal"));
    let busy = session.invoke(Origin::Shell, second.clone());
    assert!(matches!(&busy.status, Status::Refused(e) if e.code == code::COMMIT_IN_FLIGHT), "step 4: {busy:?}");
    assert_eq!((disk.writes(), disk.durable("journal")), (writes, journal), "step 4 wrote");
    disk.release();
    let failed = worker.join().expect("the commit thread");
    assert!(matches!(&failed.status, Status::Failed(e) if e.code == code::DISK_FULL), "step 9: {failed:?}");
    let after = everything(&session, &disk);
    assert_eq!((&after.0, &after.1, &after.3), (&before.0, &before.1, &before.3), "steps 4 and 9 changed something");
    refusals.push((second, busy));
    refusals.push((first, failed));

    // Exactly one later commit: the preview the refusals left commits once, and no refused delivery commits late.
    let shown = session.preview().expect("the preview the refusals left");
    assert_eq!((shown.id, shown.fresh), (s, true));
    let (order, journalled) = (session.overview().order, frames(&disk));
    let c = shell(&session, "c", Row::Execute { preview: s });
    let record = committed(&c);
    assert_eq!((record.pre.as_str(), Some(record.post.as_str())), (shown.start.as_str(), shown.end.as_deref()));
    for (envelope, receipt) in refusals {
        assert_eq!(session.invoke(Origin::Shell, envelope), receipt, "a retry gets the stored answer and nothing else");
    }
    assert_eq!((session.overview().order, frames(&disk)), (order + 1, journalled + 1), "exactly one commit");
    assert_eq!(digest(&session), record.post);
}

/// CT-ONE-20 (part: a program job that ends as `error`, here because its host refuses the proposal the job tries to
/// make, leaves the pending preview fresh and the state and journal unchanged, and a token minted afterwards commits
/// that preview; the death of the runtime process against a fake session is `c600-runtime/tests/faults.rs`'s; not run:
/// that death against the real session store, which `c600-session` does not link to `c600-runtime` (LP3's shell
/// integration); M6 tests `KN7:resource_limit.runtime_lost` through the terminal mapping only (`rust/README.md`, M6)).
#[test]
fn ct_one_20_a_job_ending_in_error_leaves_the_pending_preview_committable() {
    let (session, disk) = session();
    commit(&session, "m1", &[1]);
    let p = propose(&session, "p", literal(&[2]));
    let shown = session.preview().expect("the pending preview");
    assert!(shown.fresh);
    let (before, journal) = (session.overview(), disk.durable("journal"));

    let spec = job("offer", vec![Value::word(&[3])], vec![Input::Current], Lane::Program, JobKind::File);
    let admitted = try_submit(&session, "j", spec);
    let id = match admitted.status {
        Status::Running(id) => id,
        ref other => panic!("the job was not admitted: {other:?}"),
    };
    let result = session.wait_job(id).expect("the job's result");
    assert!(matches!(&result.outcome, JobOutcome::Error(e) if e.code == NOT_GRANTED), "{:?}", result.outcome);
    let ended = finished(&session, &admitted);
    assert!(matches!(&ended.status, Status::Failed(e) if e.code == NOT_GRANTED), "{ended:?}");

    assert_eq!(session.preview(), Some(shown.clone()), "the pending preview stays, fresh");
    assert_eq!((session.overview(), disk.durable("journal")), (before, journal), "the state and journal are unchanged");

    let executed = shell(&session, "x", Row::Execute { preview: p });
    let record = committed(&executed);
    assert_eq!((record.pre.as_str(), Some(record.post.as_str())), (shown.start.as_str(), shown.end.as_deref()));
    assert_eq!(digest(&session), record.post);
}

/// CT-ONE-21 (part: an engine restart, here the session dropped and reopened under a new epoch, ends the pending
/// preview and a snapshot handle, keeps the draft record and the durable commit and workspace receipts, answers
/// `unknown_outcome` for the lost proposal, refusal and job receipts, and revives no permission: the unused token is
/// refused in its old envelope, re-addressed to the new epoch, and against a new preview of the same word at the same
/// revision, which only a token minted after the restart commits; not run: the value store's handles and derived
/// values of ONE.70, which the session store does not hold: the command layer's value store holds them, and the hosts'
/// `tower_rows.rs` tests in `c600-tools` and `c600-jumble-tool` run them). ONE.17's other side, a stored `RevisionRef`
/// that still resolves, is the next test.
#[test]
fn ct_one_21_a_restart_ends_previews_and_handles_and_revives_no_permission() {
    let (session, disk) = session();
    let moved = commit(&session, "C", &[1]);
    let draft = set_record(&session, "W", "draft", letters(&[2, 3]));
    assert!(matches!(draft.status, Status::Completed(_)), "{draft:?}");
    let handle = session.open_snapshot(At::Current).expect("a snapshot handle");
    let proposal = shell(&session, "Q", Row::Propose { word: WordSource::Record("draft".into()) });
    let p = match &proposal.status {
        Status::Completed(c) => c.result.as_ref().and_then(|r| r.preview).expect("a preview ID"),
        other => panic!("the draft was not proposed: {other:?}"),
    };
    let shown = session.preview().expect("the pending preview");
    let unused = minted(&session, "X", Row::Execute { preview: p });
    let refusal = session.invoke(Origin::Shell, envelope(&session, "R", Row::Execute { preview: p }));
    assert_eq!(code(&refusal), code::TOKEN_INVALID);
    let spec = job("state", Vec::new(), vec![Input::Handle(handle.clone())], Lane::Program, JobKind::Query);
    let read = try_submit(&session, "J", spec);
    assert!(matches!(finished(&session, &read).status, Status::Completed(_)));
    let (old_epoch, view) = (session.epoch(), durable(&session));
    drop(session);

    let reopened = reopen(&disk);
    assert_ne!(reopened.epoch(), old_epoch, "a new engine epoch");
    let writes = disk.writes();
    assert_eq!(reopened.preview(), None, "the preview ended");
    assert_eq!(durable(&reopened), view, "the state, its counters and the draft are recovered");

    let stale = job("state", Vec::new(), vec![Input::Handle(handle)], Lane::Program, JobKind::Query);
    assert_eq!(code(&try_submit(&reopened, "J2", stale)), code::EPOCH_CHANGED, "the handle ended");

    assert_eq!(reopened.outcome("C", &moved.request), moved);
    assert_eq!(reopened.outcome("W", &draft.request), draft);
    for lost in [&proposal, &refusal, &read] {
        assert_eq!(reopened.outcome(&lost.command, &lost.request).status, Status::UnknownOutcome, "{}", lost.command);
    }

    let none = reopened.shell().mint("X1", &Row::Execute { preview: p }).unwrap_err();
    assert_eq!(none.code, code::NO_FRESH_PREVIEW, "a fresh preview is needed");
    assert_eq!(code(&reopened.invoke(Origin::Shell, unused.clone())), code::EPOCH_CHANGED);
    let mut readdressed = unused.clone();
    readdressed.epoch = reopened.epoch();
    assert_eq!(code(&reopened.invoke(Origin::Shell, readdressed)), code::TOKEN_INVALID);
    assert_eq!(disk.writes(), writes, "nothing above wrote");

    let p2 = propose(&reopened, "Q2", WordSource::Record("draft".into()));
    let again = reopened.preview().expect("the draft's new preview");
    assert_eq!((&again.literal, &again.start, &again.end), (&shown.literal, &shown.start, &shown.end));
    let mut revived = unused;
    revived.epoch = reopened.epoch();
    revived.command = "X3".into();
    revived.row = Row::Execute { preview: p2 };
    assert_eq!(code(&reopened.invoke(Origin::Shell, revived)), code::TOKEN_INVALID, "the old token revives nothing");
    assert_eq!(reopened.preview(), Some(again.clone()));

    let executed = shell(&reopened, "X4", Row::Execute { preview: p2 });
    assert_eq!(Some(committed(&executed).post.clone()), shown.end);
}

/// CT-ONE-21 (part: the line ONE.17 draws through "ends handles": the handle ends with its engine epoch, but the
/// `RevisionRef` stored from it is opened again after the restart, under the new epoch, reads the same state and is a
/// job input there, because a different epoch does not invalidate a read; chapter 13 gives `KN2:stale.epoch_changed` to
/// transient handles and previews, and "a durable RevisionRef can still be resolved". A stored reference to an event
/// the store no longer holds is `KN2:stale.revision_gone`; not run here: the rest of the row, which the test above
/// runs).
#[test]
fn ct_one_21_a_stored_revision_ref_still_resolves_after_a_restart() {
    let (session, disk) = session();
    commit(&session, "C", &[1]);
    let handle = session.open_snapshot(At::Current).expect("a snapshot handle");
    let stored = handle.revision.clone();
    drop(session);

    let reopened = reopen(&disk);
    let stale = job("state", Vec::new(), vec![Input::Handle(handle.clone())], Lane::Program, JobKind::Query);
    assert_eq!(code(&try_submit(&reopened, "J", stale)), code::EPOCH_CHANGED, "the handle ended");
    assert_eq!(digest(&reopened), stored.digest, "the head is the stored reference's event");
    let again = reopened.open_snapshot(At::Revision(stored.clone()));
    let again = again.unwrap_or_else(|e| panic!("ONE.17: the stored reference resolves after the restart: {e}"));
    assert_eq!(again.state().digest(), stored.digest);
    assert_eq!(again.state().labels(), handle.state().labels());
    let mut expected = stored.clone();
    expected.epoch = reopened.epoch();
    assert_eq!(again.revision, expected, "the same revision, under the new epoch");
    let read = job("state", Vec::new(), vec![Input::Handle(again)], Lane::Program, JobKind::Query);
    let admitted = try_submit(&reopened, "J2", read);
    assert!(matches!(finished(&reopened, &admitted).status, Status::Completed(_)), "a job reads the reopened snapshot");

    // D commits after the restart, and its frame is damaged before the next one: the store no longer holds D.
    let d = committed(&commit(&reopened, "D", &[2])).revision.clone();
    drop(reopened);
    let mut bytes = disk.durable("journal").expect("a journal");
    let last = bytes.len() - 1;
    flip(&mut bytes, last);
    disk.overwrite("journal", &bytes);
    let recovered = reopen(&disk);
    assert_eq!(recovered.overview().read_only, Some(code::COMMIT_FATE), "the damage was found");
    assert_eq!(recovered.open_snapshot(At::Revision(d)).unwrap_err().code, code::REVISION_GONE, "D is gone");
    let kept = recovered.open_snapshot(At::Revision(stored.clone())).expect("the earlier reference still resolves");
    assert_eq!(kept.state().digest(), stored.digest);
}

/// A history in which every M6 commit row would commit: a draft record of letters, two lines, and both an undo and a
/// redo available.
fn history(session: &Session) {
    set_record(session, "d", "draft", letters(&[4, 5]));
    commit(session, "x", &[1]);
    undo(session, "ux");
    commit(session, "y", &[2]);
    commit(session, "z", &[3]);
    undo(session, "uz");
    let o = session.overview();
    assert!(o.undo.is_some() && o.redo.is_some() && o.lines == 2, "{o:?}");
}

/// The commit rows without a preview, each with a token minted now at the current revision.
fn steps(session: &Session) -> Vec<Envelope> {
    let o = session.overview();
    vec![
        minted(session, "undo", Row::Undo { hint: o.undo.expect("an undo"), confirm: false }),
        minted(session, "redo", Row::Redo { hint: o.redo.expect("a redo"), confirm: false }),
        minted(session, "switch", Row::SwitchLine { line: 0 }),
    ]
}

/// Read-only recovery as CT-ONE-22 states it: the commit rows and proposals are refused with the read-only code, the
/// status, snapshots and read jobs answer, a job that would write gets no write, and nothing is written.
fn read_only_holds(route: &str, session: &Session, disk: &MemDisk, commits: Vec<Envelope>) {
    let before = session.overview();
    assert_eq!(before.read_only, Some(code::COMMIT_FATE), "{route}");
    let (writes, journal) = (disk.writes(), disk.durable("journal"));

    for envelope in commits {
        let row = envelope.row.name();
        let r = session.invoke(Origin::Shell, envelope);
        assert!(matches!(&r.status, Status::Refused(e) if e.code == code::READ_ONLY_RECOVERY), "{route}: {row}: {r:?}");
    }
    for (command, word) in [("propose.literal", literal(&[4])), ("propose.draft", WordSource::Record("draft".into()))] {
        let r = shell(session, command, Row::Propose { word });
        let refused = matches!(&r.status, Status::Refused(e) if e.code == code::READ_ONLY_RECOVERY);
        assert!(refused, "{route}: {command}: {r:?}");
    }

    let now = session.open_snapshot(At::Current).expect("a snapshot of the current revision");
    assert_eq!(now.revision.digest, before.revision.digest, "{route}");
    let moved = now.state().labels().iter().enumerate().filter(|&(slot, &label)| slot as i32 != label).count() as u64;
    assert!(moved > 0, "{route}: the state is not solved");
    for (command, input) in [("read.current", Input::Current), ("read.handle", Input::Handle(now.clone()))] {
        let spec = job("moved", Vec::new(), vec![input], Lane::Program, JobKind::Query);
        let admitted = try_submit(session, command, spec);
        assert!(matches!(admitted.status, Status::Running(_)), "{route}: {command}: {admitted:?}");
        let done = finished(session, &admitted);
        let answered = matches!(&done.status, Status::Completed(c) if c.kind == CompletionKind::Query);
        assert!(answered, "{route}: {command}: {done:?}");
        let result = session.wait_job(admitted.job.expect("a job")).expect("its result");
        assert_eq!(result.value, Some(nat(moved)), "{route}: {command}");
    }
    let spec = job("keep", vec![Value::word(&[1])], vec![Input::Current], Lane::Program, JobKind::File);
    let write = try_submit(session, "write", spec);
    let ended = if write.job.is_some() { finished(session, &write) } else { write };
    let stopped = matches!(ended.status, Status::Refused(_) | Status::Failed(_));
    assert!(stopped, "{route}: a job that would write: {ended:?}");

    assert_eq!(session.overview(), before, "{route}: the status");
    assert_eq!((disk.writes(), disk.durable("journal")), (writes, journal), "{route}: read-only recovery wrote");
}

/// CT-ONE-22 (part: read-only recovery, entered by a commit whose fate is unknown and by damage found at open, refuses
/// the proposal of a literal and of a draft and every M6 commit row (`operation.execute`, `session.undo`,
/// `session.redo`, `session.switch_line`) with `KN5:unknown_outcome.read_only_recovery`, writes nothing, and answers
/// the status, snapshots, an outcome lookup and read jobs, while a job that would write gets no write (ONE.70: jobs
/// are admitted only for reads); not run: a program's proposal, because M6 grants none (KN5.12 stays open), and
/// `twist.apply`, which is not an M6 row). After the damage the session is a restart, so no preview is pending and
/// none can be built: the stub mints no `operation.execute` there (step 1).
#[test]
fn ct_one_22_read_only_recovery_refuses_proposals_and_commits_and_answers_reads() {
    {
        let (session, disk) = session();
        history(&session);
        let p = propose(&session, "p", literal(&[4]));
        let fated = minted(&session, "fated", Row::Execute { preview: p });
        let mut commits = vec![minted(&session, "execute", Row::Execute { preview: p })];
        commits.extend(steps(&session));
        let at = disk.writes();
        disk.fail_write(at, Fault::Error);
        disk.fail_from(at + 1);
        let fate = session.invoke(Origin::Shell, fated);
        assert_eq!(fate.status, Status::UnknownOutcome);
        disk.heal();
        assert_eq!(session.outcome("fated", &fate.request).status, Status::UnknownOutcome, "the lookup answers");
        read_only_holds("unknown fate", &session, &disk, commits);
    }
    {
        let (session, disk) = session();
        history(&session);
        set_record(&session, "spare", "spare", letters(&[1]));
        let mut bytes = disk.durable("journal").expect("a journal");
        let last = bytes.len() - 1;
        flip(&mut bytes, last);
        disk.overwrite("journal", &bytes);
        drop(session);
        let reopened = reopen(&disk);
        let none = reopened.shell().mint("execute", &Row::Execute { preview: 1 }).unwrap_err();
        assert_eq!(none.code, code::NO_FRESH_PREVIEW);
        let commits = steps(&reopened);
        read_only_holds("damage", &reopened, &disk, commits);
    }
}
