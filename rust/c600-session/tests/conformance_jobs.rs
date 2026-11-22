//! LP2 conformance tests of chapter 08 against the session store (chapter 12 section 15.3): CT-ONE-06 (a cached read
//! result after an undo), CT-ONE-07 (a long read while commits land) and the session-owned rows of CT-ONE-12 (what
//! each state change does to the things ONE.49 names that are open at that moment).

mod common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use c600_check::check;
use c600_digest::sha256_hex;
use c600_run::interp::{load, Loaded};
use c600_run::meter::Summary;
use c600_run::value::Value;
use c600_session::{
    code, At, CancelReason, CompletionKind, Input, JobKind, JobOutcome, JobResult, JobSpec, Lane, Options, Origin,
    Receipt, Row, Session, SnapshotHandle, Status, WordSource,
};
use c600_syntax::{lower, parse};
use common::*;

/// The programs these tests add to the shared test programs, in the same module `m`. `steady` runs `n` rounds; each
/// round does `2^spin` calls of work, then reads its first input's `state`, `moved` and `diff` and counts the answers
/// equal to the expected ones, so it returns `3 * n` only when every read agrees. `late_moved` does its work first and
/// reads `moved` last.
const MORE: &str = "
fn same(a: Nat, b: Nat) -> Nat effects [] {
    return match exact_eq(a, b) {
        true => 1;
        _ => 0;
    };
}

fn round(s: Nat, m: Nat, d: Nat, spin: Nat) -> Nat effects [snapshot] {
    let work = tree(0, spin);
    let a = same(read_snapshot<Nat>(\"state\"), s);
    let b = same(read_snapshot<Nat>(\"moved\"), m);
    let c = same(read_snapshot<Nat>(\"diff\"), d);
    return match exact_less(0, work) {
        true => exact_add(exact_add(a, b), c);
        _ => 0;
    };
}

fn rounds(i: Nat, n: Nat, s: Nat, m: Nat, d: Nat, spin: Nat) -> Nat effects [snapshot] {
    let here = round(s, m, d, spin);
    let rest = match exact_less(exact_add(i, 1), n) {
        true => rounds(exact_add(i, 1), n, s, m, d, spin);
        _ => 0;
    };
    return exact_add(here, rest);
}

export fn steady(n: Nat, s: Nat, m: Nat, d: Nat, spin: Nat) -> Nat effects [snapshot] {
    return rounds(0, n, s, m, d, spin);
}

export fn late_moved(spin: Nat) -> Nat effects [snapshot] {
    let work = tree(0, spin);
    let m = read_snapshot<Nat>(\"moved\");
    return match exact_less(0, work) {
        true => m;
        _ => 0;
    };
}
";

/// The rounds of CT-ONE-07's long job and the work before each round's reads: 2^14 calls, about 0.3 s in a debug
/// build, so the job runs for seconds.
const ROUNDS: u64 = 12;
const ROUND_WORK: u64 = 14;
/// The work a late job of CT-ONE-12 does before its one read: 2^16 calls, about a second in a debug build, so it is
/// still running when the state moves.
const LATE_WORK: u64 = 16;

/// The shared test programs and [`MORE`], checked and loaded once.
fn loaded() -> Arc<Loaded> {
    static LOADED: OnceLock<Arc<Loaded>> = OnceLock::new();
    Arc::clone(LOADED.get_or_init(|| {
        let source = format!("{HEADER}{PROGRAMS}{MORE}");
        let file = parse(source.as_bytes());
        assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
        let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
        let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        Arc::new(load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}")))
    }))
}

/// A job of one exported function of [`loaded`].
fn more(function: &str, args: Vec<Value>, inputs: Vec<Input>, lane: Lane, kind: JobKind) -> JobSpec {
    JobSpec {
        program: loaded(),
        program_digest: "conformance-programs".to_owned(),
        module: "m".to_owned(),
        function: function.to_owned(),
        args,
        inputs,
        lane,
        kind,
        budget: WIDE,
    }
}

/// What a result must reproduce to count as the same read: the outcome, the inputs it covered, the meter's summary
/// (every charge) and the value. The request digest is left out: it names the submission, which differs per run.
fn fingerprint(result: &JobResult) -> (JobOutcome, String, Vec<String>, Option<Summary>, Option<Vec<u8>>) {
    let inputs = result.inputs.iter().map(|r| r.digest.clone()).collect();
    (result.outcome.clone(), result.input.clone(), inputs, result.summary.clone(), result.value.clone())
}

/// Runs one job to its end and returns its terminal receipt and result.
fn run(session: &Session, command: &str, spec: JobSpec) -> (Receipt, JobResult) {
    let admitted = try_submit(session, command, spec);
    assert!(matches!(admitted.status, Status::Running(_)), "{command}: {admitted:?}");
    let receipt = finished(session, &admitted);
    (receipt, session.wait_job(admitted.job.expect("an admitted job")).expect("the job's result"))
}

/// The terminal receipt of a job that cannot end by itself unless it is cancelled: one that is not cancelled within a
/// minute fails the test instead of hanging it.
fn cancelled(session: &Session, admitted: &Receipt) -> Receipt {
    let job = admitted.job.expect("an admitted job");
    let deadline = Instant::now() + Duration::from_secs(60);
    while session.job_result(job).expect("a job of this epoch").is_none() {
        assert!(Instant::now() < deadline, "the {} job is still running", admitted.command);
        std::thread::sleep(Duration::from_millis(5));
    }
    session.outcome(&admitted.command, &admitted.request)
}

/// `Digest<labels>` of the bytes a handle reads: SHA-256 of its labels as little-endian `i32`s.
fn label_digest(handle: &SnapshotHandle) -> String {
    let bytes: Vec<u8> = handle.state().labels().iter().flat_map(|label| label.to_le_bytes()).collect();
    sha256_hex(&bytes)
}

/// The slots whose label is not the slot's own: the job host's `moved` answer.
fn moved_slots(labels: &[i32]) -> u64 {
    labels.iter().enumerate().filter(|&(slot, &label)| slot as i32 != label).count() as u64
}

/// Sets a flag when dropped, so a failing assertion still stops the reader thread it guards.
struct StopOnDrop<'a>(&'a AtomicBool);

impl Drop for StopOnDrop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// CT-ONE-06 (part: the session's half; the view model's half is `c600-view/tests/view_rows.rs`'s): a read result is
/// valid
/// wherever its inputs are equal, and a cache hit is charged as a miss (ONE.10, KN7.3, KN7.8). A query at A computes
/// and caches its whole-state diff. After a commit to B and an undo back to A's digest under a newer revision, the
/// session gives A's result again at the undo's revision and refuses it at B's; a run at the undo takes its diff from
/// the cache (no new entry, where B's own diff was one) and is charged exactly as the cold miss was, and so is a
/// recomputation after an eviction.
#[test]
fn ct_one_06_a_cached_result_is_shown_after_an_undo_and_charged_as_a_miss() {
    let compare = || job("compare", Vec::new(), vec![Input::Current], Lane::Program, JobKind::Query);
    let (session, _disk) = session();
    committed(&commit(&session, "A", &[1, 2, 3]));
    let a = session.overview().revision;
    session.evict_cache();

    let (receipt, cold) = run(&session, "cold", compare());
    assert!(matches!(&receipt.status, Status::Completed(c) if c.kind == CompletionKind::Query), "{receipt:?}");
    assert_eq!(session.cached_queries(), 1, "the cold run computed and cached its diff");
    assert_eq!(cold.inputs, vec![a.clone()]);
    // One unit per 1,024 slots of the 259,800, rounded up, for each whole-state query; the additions and the encoding
    // of the answer are priced on top.
    let charged = cold.summary.as_ref().expect("a completed run keeps its summary").used;
    assert!(charged.kernel_cost >= 2 * 254 + 1, "the cold run pays `diff`, `moved` and `inputs`: {charged:?}");

    committed(&commit(&session, "B", &[4]));
    let b = session.overview().revision;
    assert_ne!(b.digest, a.digest);
    assert_eq!(session.job_result_at(cold.job, &b).unwrap_err().code, code::JOB_INPUT_DIGEST, "not shown at B");
    run(&session, "at.b", compare());
    assert_eq!(session.cached_queries(), 2, "B's diff has other inputs: a miss and a new entry");

    committed(&undo(&session, "U"));
    let back = session.overview().revision;
    assert_eq!(back.digest, a.digest, "the undo returns to A's digest");
    assert!(back.state > b.state && back.head != b.head, "under a newer revision (ONE.9): {back:?}");
    assert_eq!(session.job_result_at(cold.job, &back).unwrap(), cold, "A's result is shown again after the undo");

    let (_, warm) = run(&session, "warm", compare());
    assert_eq!(session.cached_queries(), 2, "the run after the undo took its diff from the cache");
    assert_eq!(fingerprint(&warm), fingerprint(&cold), "the hit after the undo is charged as the cold miss was");
    assert_eq!(warm.inputs, vec![back.clone()], "the new result names the revision shown now");
    assert_eq!(session.job_result_at(warm.job, &back).unwrap(), warm);
    assert_eq!(session.job_result_at(warm.job, &a).unwrap(), warm, "and it is valid at A, whose inputs are equal");

    session.evict_cache();
    let (_, evicted) = run(&session, "evicted", compare());
    assert_eq!(session.cached_queries(), 1, "after the eviction the diff was computed again");
    assert_eq!(fingerprint(&evicted), fingerprint(&cold), "a recomputation is charged as the cold miss was");
}

/// CT-ONE-07 (whole): a long read keeps giving the same bytes while commits land (ONE.15, ONE.18, ONE.19). A thread
/// reads one handle pass after pass, from before the first of five state changes (two commits, two undos and a redo)
/// until after the last; every pass hashes to the opened digest while the head moves, and changes land during passes.
/// A pinned job reads `state`, `moved` and `diff` through the same handle, is in flight across every change and gets
/// the opened answers in every round, including a diff it computes again from the handle after the last change.
#[test]
fn ct_one_07_a_long_read_keeps_giving_the_same_bytes_while_commits_land() {
    let (session, _disk) = session();
    committed(&commit(&session, "A", &[1, 2, 3]));
    let held = session.open_snapshot(At::Current).expect("a snapshot of the current revision");
    let opened = held.revision.clone();
    assert_eq!(label_digest(&held), opened.digest, "the bytes a handle reads hash to its revision's digest");
    let solved = kernel().solved();
    let labels = held.state().labels();
    let unlike = labels.iter().zip(solved.labels()).filter(|(x, y)| x != y).count() as u64;
    let args = [ROUNDS, opened.state, moved_slots(labels), unlike, ROUND_WORK].into_iter().map(Value::int).collect();
    session.evict_cache();
    let spec = more("steady", args, vec![Input::Handle(held.clone())], Lane::Program, JobKind::Pinned);
    let admitted = try_submit(&session, "long", spec);
    let job = admitted.job.unwrap_or_else(|| panic!("the long job was not admitted: {admitted:?}"));

    let stop = AtomicBool::new(false);
    let landed = AtomicUsize::new(0);
    let passes = AtomicUsize::new(0);
    let mut heads = Vec::new();
    let reads = std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            let mut reads = Vec::new();
            loop {
                let stopping = stop.load(Ordering::SeqCst);
                let before = landed.load(Ordering::SeqCst);
                let read = label_digest(&held);
                let after = landed.load(Ordering::SeqCst);
                reads.push((before, read, after));
                passes.fetch_add(1, Ordering::SeqCst);
                if stopping {
                    return reads;
                }
            }
        });
        let _stop = StopOnDrop(&stop);
        // Both readers are under way before the first change: one whole pass, and the job's first diff, now cached.
        let deadline = Instant::now() + Duration::from_secs(120);
        while passes.load(Ordering::SeqCst) == 0 || session.cached_queries() == 0 {
            assert!(Instant::now() < deadline, "the reader or the job made no read");
            std::thread::sleep(Duration::from_millis(2));
        }
        let changes: [(&str, fn(&Session) -> Receipt); 5] = [
            ("commit B", |s| commit(s, "B", &[4])),
            ("undo B", |s| undo(s, "U1")),
            ("redo B", |s| redo(s, "R1")),
            ("commit C", |s| commit(s, "C", &[5])),
            ("undo C", |s| undo(s, "U2")),
        ];
        for (name, change) in changes {
            committed(&change(&session));
            landed.fetch_add(1, Ordering::SeqCst);
            heads.push(digest(&session));
            let running = session.job_result(job).unwrap().is_none();
            assert!(running, "the job ended before the change `{name}` landed; raise ROUNDS");
        }
        // From here on, a diff the job reads is computed again from its handle's labels.
        session.evict_cache();
        assert!(session.job_result(job).unwrap().is_none(), "the job ended before the eviction; raise ROUNDS");
        stop.store(true, Ordering::SeqCst);
        reader.join().expect("the reader thread")
    });

    assert_ne!(heads[0], opened.digest, "the head moved away from the opened revision");
    // The head's state answers `moved` and `diff` differently, so a read that followed the head would show.
    let now = session.open_snapshot(At::Current).expect("a snapshot of the head");
    assert_ne!(now.revision.digest, opened.digest);
    let now_unlike = now.state().labels().iter().zip(solved.labels()).filter(|(x, y)| x != y).count() as u64;
    assert_ne!((moved_slots(now.state().labels()), now_unlike), (moved_slots(labels), unlike));
    assert_eq!(reads.first().map(|r| r.0), Some(0), "the long read began before the first change");
    assert_eq!(reads.last().map(|r| r.0), Some(5), "the long read went on after the last change");
    let straddled = reads.iter().filter(|(before, _, after)| before < after).count();
    assert!(straddled >= 1, "no change landed during a pass: {} passes", reads.len());
    for (i, (before, read, after)) in reads.iter().enumerate() {
        assert_eq!(read, &opened.digest, "pass {i}, between change {before} and change {after}, read other bytes");
    }

    let receipt = finished(&session, &admitted);
    assert!(matches!(&receipt.status, Status::Completed(c) if c.kind == CompletionKind::Query), "{receipt:?}");
    let result = session.wait_job(job).unwrap();
    assert_eq!(result.value, Some(nat(3 * ROUNDS)), "every read through the handle gave the opened state's answers");
    assert_eq!((&result.input, &result.inputs), (&opened.digest, &vec![opened.clone()]));
    assert_eq!(session.cached_queries(), 1, "the job read the diff again after every change had landed");
}

#[derive(Clone, Copy, Debug)]
enum Change {
    Execute,
    Undo,
    Redo,
    SwitchLine,
}

/// Opens at once every thing of ONE.49 the session owns, makes one state change and checks what it did to each.
fn one_49(change: Change) {
    let name = format!("{change:?}");
    let (session, _disk) = session_with(Options { program_capacity: 3 });
    // The draft comes first: a workspace write after the history would empty the redo stack.
    let draft = set_record(&session, "draft", "draft", letters(&[5, 6]));
    assert!(matches!(&draft.status, Status::Completed(c) if c.kind == CompletionKind::Workspace), "{draft:?}");
    match change {
        Change::Execute => {
            committed(&commit(&session, "A", &[1]));
        }
        Change::Undo => {
            committed(&commit(&session, "A", &[1]));
            committed(&commit(&session, "B", &[2]));
        }
        Change::Redo => {
            committed(&commit(&session, "A", &[1]));
            committed(&commit(&session, "B", &[2]));
            committed(&undo(&session, "U"));
        }
        Change::SwitchLine => {
            // Two letters on the second line: one letter moves as many slots as another.
            committed(&commit(&session, "A", &[1]));
            committed(&undo(&session, "U"));
            committed(&commit(&session, "C", &[2, 3]));
            let overview = session.overview();
            assert_eq!((overview.lines, overview.active_line), (2, 1), "C started a second line");
        }
    }

    propose(&session, "draft.shown", WordSource::Record("draft".to_owned()));
    let draft_word = session.preview().expect("the draft's preview").literal;
    let pending = propose(&session, "pending", literal(&[4]));
    let pending_end = session.preview().and_then(|p| p.end).expect("the pending preview's end state");
    let unused = session.shell().mint("late", &Row::Execute { preview: pending }).expect("a token for the preview");
    let held = session.open_snapshot(At::Current).expect("a snapshot");
    let old = session.overview().revision;
    let query = |command: &str, lane: Lane| {
        try_submit(&session, command, job("long_query", Vec::new(), vec![Input::Current], lane, JobKind::Query))
    };
    let exclusive = query("query.exclusive", Lane::Exclusive);
    let program = query("query.program", Lane::Program);
    let late = |command: &str, kind: JobKind| {
        let args = vec![Value::int(LATE_WORK)];
        try_submit(&session, command, more("late_moved", args, vec![Input::Current], Lane::Program, kind))
    };
    let file = late("file", JobKind::File);
    let pinned = late("pinned", JobKind::Pinned);
    for admitted in [&exclusive, &program, &file, &pinned] {
        assert!(matches!(admitted.status, Status::Running(_)), "{name}: {admitted:?}");
    }

    let receipt = match change {
        Change::Execute => shell(&session, "change", Row::Execute { preview: pending }),
        Change::Undo => undo(&session, "change"),
        Change::Redo => redo(&session, "change"),
        Change::SwitchLine => shell(&session, "change", Row::SwitchLine { line: 0 }),
    };
    let new = committed(&receipt).revision.clone();
    for admitted in [&file, &pinned] {
        let running = session.job_result(admitted.job.unwrap()).unwrap().is_none();
        assert!(running, "{name}: the {} job ended before the state moved; raise LATE_WORK", admitted.command);
    }
    assert_eq!(session.overview().revision, new, "{name}");
    assert_eq!(new.state, old.state + 1, "{name}: one new state revision");
    assert_ne!(new.digest, old.digest, "{name}: the state moved");

    // Open snapshot: it stays readable and unchanged (ONE.18).
    assert_eq!(held.revision, old, "{name}: the handle names the revision it was opened at");
    assert_eq!(label_digest(&held), old.digest, "{name}: the handle reads the old state's bytes");

    // Pending preview: an execute commits it, any other change discards it (ONE.40).
    match change {
        Change::Execute => assert_eq!(new.digest, pending_end, "{name}: the committed preview's state is the state"),
        _ => assert_ne!(new.digest, pending_end, "{name}: the preview's state was not committed"),
    }
    assert_eq!(session.overview().pending_preview, None, "{name}: no preview is pending");
    let again = session.shell().mint("again", &Row::Execute { preview: pending }).err().map(|e| e.code);
    assert_eq!(again, Some(code::NO_FRESH_PREVIEW), "{name}: the preview can no longer be committed");

    // Unused token: it dies with the preview it binds (chapter 08 section 7, step 5).
    let mut delivery = envelope(&session, "late", Row::Execute { preview: pending });
    delivery.token = Some(unused);
    let refused = session.invoke(Origin::Shell, delivery);
    assert_eq!(code(&refused), code::PREVIEW_REPLACED, "{name}: the unused token was refused");
    assert_eq!(session.overview().revision, new, "{name}: the refusal changed nothing");

    // Draft: it stays, and it offers a fresh preview at the new state (ONE.35, ONE.40).
    assert_eq!(new.workspace, old.workspace, "{name}: no workspace record changed");
    let offered = propose(&session, "draft.again", WordSource::Record("draft".to_owned()));
    let preview = session.preview().expect("the draft's new preview");
    assert!(preview.id == offered && preview.fresh, "{name}: {preview:?}");
    assert_eq!((preview.literal, preview.start), (draft_word, new.digest.clone()), "{name}: the draft is as it was");

    // Queries reading the replaced revision, in the exclusive and the program lane: cancelled, with no negative claim
    // (KN7.4, KN5.18).
    for admitted in [&exclusive, &program] {
        let receipt = cancelled(&session, admitted);
        assert_eq!(receipt.status, Status::Cancelled(CancelReason::ReplacedRevision), "{name}: {}", admitted.command);
        let result = session.wait_job(admitted.job.unwrap()).unwrap();
        assert_eq!(result.outcome, JobOutcome::Cancelled(CancelReason::ReplacedRevision), "{name}");
        assert_eq!(result.value, None, "{name}: a cancelled query yields no answer");
        assert_eq!(result.inputs, vec![old.clone()], "{name}");
    }

    // File job and pinned job: they finish on their own input, which they read after the state moved (DT-M1 Q5 (a),
    // KN7.5). The two states answer `moved` differently, so a read of the new state would show.
    let before = moved_slots(held.state().labels());
    let after = moved_slots(session.open_snapshot(At::Current).unwrap().state().labels());
    assert_ne!(before, after, "{name}: the old and the new state must answer `moved` differently");
    for (admitted, kind) in [(&file, CompletionKind::File), (&pinned, CompletionKind::Query)] {
        let receipt = finished(&session, admitted);
        match &receipt.status {
            Status::Completed(c) => assert_eq!((c.kind, &c.revision), (kind, &old), "{name}: {}", admitted.command),
            other => panic!("{name}: the {} job did not finish: {other:?}", admitted.command),
        }
        let result = session.wait_job(admitted.job.unwrap()).unwrap();
        assert_eq!(result.value, Some(nat(before)), "{name}: the {} job read its own input", admitted.command);
        assert_eq!(result.input, old.digest, "{name}");
    }
    // The pinned result reaches the newer revision only by a re-run (KN7.5, KN7.3).
    let pinned_job = pinned.job.unwrap();
    assert_eq!(session.job_result_at(pinned_job, &new).unwrap_err().code, code::JOB_INPUT_DIGEST, "{name}");
    assert!(session.job_result_at(pinned_job, &old).is_ok(), "{name}");
    let spec = more("late_moved", vec![Value::int(0)], vec![Input::Current], Lane::Program, JobKind::Pinned);
    let (_, rerun) = run(&session, "pinned.rerun", spec);
    assert_eq!(rerun.value, Some(nat(after)), "{name}: the re-run reads the new state");
    assert_eq!(session.job_result_at(rerun.job, &new).unwrap(), rerun, "{name}");

    // The held handle is still readable by a job once the state has moved.
    let spec = job("moved", Vec::new(), vec![Input::Handle(held.clone())], Lane::Program, JobKind::Query);
    let (_, through) = run(&session, "through", spec);
    assert_eq!(through.value, Some(nat(before)), "{name}: a job reads the held handle's old state");
    assert_eq!(session.overview().jobs_running, 0, "{name}");
}

/// CT-ONE-12 (part: an execute, driven headless, with ONE.49's session-owned things open at once: an open snapshot,
/// queries reading the replaced revision in the exclusive and the program lane, a file job, a pinned job, the pending
/// preview (here the one committed), an unused token that binds it and a draft record; not run: the row for latest-wins
/// jobs, whose one H-08 row no crate builds (LP3's host); the job building a preview runs in
/// `c600-command/tests/served_rows.rs`, and the view job, the per-revision fact on screen, the adopted bundle and a
/// derived state in `c600-tools/tests/views.rs` and `c600-input/tests/twist.rs`).
#[test]
fn ct_one_12_an_execute_acts_on_each_open_thing_as_one_49_says() {
    one_49(Change::Execute);
}

/// CT-ONE-12 (part: an undo, driven headless, with ONE.49's session-owned things open at once: an open snapshot,
/// queries reading the replaced revision in the exclusive and the program lane, a file job, a pinned job, a pending
/// preview (discarded), an unused token that binds it and a draft record; not run: the row for latest-wins jobs, whose
/// one H-08 row no crate builds (LP3's host); the job building a preview runs in `c600-command/tests/served_rows.rs`,
/// and the view job, the per-revision fact on screen, the adopted bundle and a derived state in
/// `c600-tools/tests/views.rs` and `c600-input/tests/twist.rs`).
#[test]
fn ct_one_12_an_undo_acts_on_each_open_thing_as_one_49_says() {
    one_49(Change::Undo);
}

/// CT-ONE-12 (part: a redo, driven headless, with ONE.49's session-owned things open at once: an open snapshot, queries
/// reading the replaced revision in the exclusive and the program lane, a file job, a pinned job, a pending preview
/// (discarded), an unused token that binds it and a draft record; not run: the row for latest-wins jobs, whose one H-08
/// row no crate builds (LP3's host); the job building a preview runs in `c600-command/tests/served_rows.rs`, and the
/// view job, the per-revision fact on screen, the adopted bundle and a derived state in `c600-tools/tests/views.rs` and
/// `c600-input/tests/twist.rs`).
#[test]
fn ct_one_12_a_redo_acts_on_each_open_thing_as_one_49_says() {
    one_49(Change::Redo);
}

/// CT-ONE-12 (part: a line switch, driven headless, with ONE.49's session-owned things open at once: an open snapshot,
/// queries reading the replaced revision in the exclusive and the program lane, a file job, a pinned job, a pending
/// preview (discarded), an unused token that binds it and a draft record; not run: the row for latest-wins jobs, whose
/// one H-08 row no crate builds (LP3's host); the job building a preview runs in `c600-command/tests/served_rows.rs`,
/// and the view job, the per-revision fact on screen, the adopted bundle and a derived state in
/// `c600-tools/tests/views.rs` and `c600-input/tests/twist.rs`).
#[test]
fn ct_one_12_a_line_switch_acts_on_each_open_thing_as_one_49_says() {
    one_49(Change::SwitchLine);
}
