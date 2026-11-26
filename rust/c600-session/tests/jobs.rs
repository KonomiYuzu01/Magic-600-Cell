//! Program jobs (KN7): budgets and their reproducibility, cut-offs, cancellation by a replaced revision, lanes under
//! concurrent submission, and pinned research jobs. Contract cases CT-KN7-01 to CT-KN7-05 and CT-KN7-07 with PHS.19's
//! budget row, CT-KN5-09, and CT-KN4-01's in-process part.

mod common;

use std::path::Path;
use std::sync::Barrier;

use c600_run::meter::code::{self as stop_code, NOT_GRANTED};
use c600_run::meter::{Amounts, Summary};
use c600_run::value::Value;
use c600_session::{
    code, CancelReason, CompletionKind, Input, JobKind, JobOutcome, JobResult, Lane, Options, Receipt, Status,
};
use common::*;

/// What a repeated run must reproduce: the outcome, the inputs it covered, the meter's summary (the step it stopped
/// at and every charge) and the value.
fn fingerprint(result: &JobResult) -> (JobOutcome, String, Vec<String>, Option<Summary>, Option<Vec<u8>>) {
    let inputs = result.inputs.iter().map(|r| r.digest.clone()).collect();
    (result.outcome.clone(), result.input.clone(), inputs, result.summary.clone(), result.value.clone())
}

/// Runs one job to its end and returns its terminal receipt and result.
fn run(session: &c600_session::Session, command: &str, spec: c600_session::JobSpec) -> (Receipt, JobResult) {
    let admitted = try_submit(session, command, spec);
    assert!(matches!(admitted.status, Status::Running(_)), "{command}: {admitted:?}");
    let receipt = finished(session, &admitted);
    (receipt, session.wait_job(admitted.job.unwrap()).unwrap())
}

fn budgeted(function: &str, args: Vec<Value>, budget: Amounts) -> c600_session::JobSpec {
    let mut spec = job(function, args, vec![Input::Current], Lane::Program, JobKind::Query);
    spec.budget = budget;
    spec
}

/// CT-KN7-01 (whole): repeated runs of one key stop at the same step, in the same engine epoch and after a reopen.
#[test]
fn repeated_runs_of_one_key_stop_at_the_same_step() {
    let (session, disk) = session();
    commit(&session, "A", &[1]);
    let budget = Amounts { steps: 10_000, ..WIDE };
    let (receipt, first) = run(&session, "J0", budgeted("count", vec![Value::int(20)], budget));
    assert_eq!(receipt.status, Status::Cancelled(CancelReason::Budget));
    assert_eq!(first.outcome, JobOutcome::CutOff(stop_code::STEPS));
    let summary = first.summary.clone().expect("a cut-off keeps its summary");
    assert!(summary.stop.is_some(), "a cut-off names the step it stopped at");
    for i in 1..3 {
        let (_, again) = run(&session, &format!("J{i}"), budgeted("count", vec![Value::int(20)], budget));
        assert_eq!(fingerprint(&again), fingerprint(&first), "run {i}");
    }
    let reopened = reopen(&disk);
    let (_, after) = run(&reopened, "J9", budgeted("count", vec![Value::int(20)], budget));
    assert_eq!(fingerprint(&after), fingerprint(&first), "after a reopen");
}

/// CT-DET-08 (part: the session's derived-query cache: each of five budget cases run cold, warm and after an eviction
/// keeps its outcome, stop, every charge and its value, so a cache hit is charged as a miss; not run: a value store
/// and the eviction of its values, which no crate builds yet).
/// CT-LG4-04 (part: a derived-query cache hit and a recomputation after an eviction charge the same as a cold run;
/// not run: the wall-clock safety stop's halves, in `persist_run.rs` (the record) and `c600-runtime`'s `faults.rs`
/// (the deadline), and the four resources' separate exhaustion, in `c600-run`'s `conformance_lg4.rs`).
/// CT-KN7-02 (whole) and PHS.19's budget row: each declared-budget case, run cold, warm from the derived-query cache
/// and again after an eviction, stops at the same step with the same coverage and charges. A cache hit is charged as a
/// miss (KN7.8).
#[test]
fn a_cache_hit_and_a_recomputation_charge_the_same() {
    let (session, _disk) = session();
    commit(&session, "A", &[1, 2, 3]);
    let state_query = 254; // one unit per 1,024 slots of the 259,800, rounded up
    let cases = [
        ("wide", WIDE, None, true),
        ("kernel", Amounts { kernel_cost: 2 * state_query - 1, ..WIDE }, Some(stop_code::KERNEL_COST), true),
        ("output", Amounts { output: 1, ..WIDE }, Some(stop_code::OUTPUT), true),
        ("steps", Amounts { steps: 5, ..WIDE }, Some(stop_code::STEPS), false),
        ("memory", Amounts { memory: 1, ..WIDE }, Some(stop_code::MEMORY), false),
    ];
    for (name, budget, cut, reaches_diff) in cases {
        session.evict_cache();
        let (receipt, cold) = run(&session, &format!("{name}.cold"), budgeted("compare", Vec::new(), budget));
        if reaches_diff {
            assert_eq!(session.cached_queries(), 1, "{name}: the cold run computed and cached its diff");
        }
        let (_, warm) = run(&session, &format!("{name}.warm"), budgeted("compare", Vec::new(), budget));
        session.evict_cache();
        let (_, evicted) = run(&session, &format!("{name}.evicted"), budgeted("compare", Vec::new(), budget));
        assert_eq!(fingerprint(&warm), fingerprint(&cold), "{name}: warm");
        assert_eq!(fingerprint(&evicted), fingerprint(&cold), "{name}: after an eviction");
        match cut {
            None => assert!(matches!(receipt.status, Status::Completed(_)), "{name}: {receipt:?}"),
            Some(code) => {
                assert_eq!(receipt.status, Status::Cancelled(CancelReason::Budget), "{name}");
                assert_eq!(cold.outcome, JobOutcome::CutOff(code), "{name}");
            }
        }
    }
}

/// CT-KN7-03 (whole), KN7.4, KN7.11: a cut-off job yields no negative claim. It has no value, unlike a completed job
/// whose answer is empty, and unlike a job that never ran, which has no result at all.
#[test]
fn a_cut_off_job_yields_no_negative_claim() {
    let (session, _disk) = session();
    let (receipt, empty) = run(&session, "done", budgeted("diff", Vec::new(), WIDE));
    assert!(matches!(&receipt.status, Status::Completed(c) if c.kind == CompletionKind::Query));
    assert_eq!(empty.value, Some(nat(0)), "the solved state differs from itself nowhere: an empty answer");

    let (receipt, cut) = run(&session, "cut", budgeted("diff", Vec::new(), Amounts { kernel_cost: 1, ..WIDE }));
    assert_eq!(receipt.status, Status::Cancelled(CancelReason::Budget));
    assert_eq!(cut.outcome, JobOutcome::CutOff(stop_code::KERNEL_COST));
    assert_eq!(cut.value, None, "a cut-off claims nothing");
    assert_eq!(cut.summary.as_ref().and_then(|s| s.code), Some(stop_code::KERNEL_COST));

    let refused = try_submit(&session, "none", job("diff", Vec::new(), Vec::new(), Lane::Program, JobKind::Query));
    assert!(
        matches!(refused.status, Status::Refused(_)) && refused.job.is_none(),
        "a job that never ran has no result"
    );
}

/// CT-KN7-04 (whole), DT-M1's XM4 with a long program job: a commit cancels the queries on the old revision; file jobs
/// finish on their original input, and a file job that does not end keeps running until the user stops it. ONE.45: the
/// commit is not refused because jobs run.
#[test]
fn a_commit_cancels_queries_on_the_old_revision_and_file_jobs_finish() {
    let (session, _disk) = session_with(Options { program_capacity: 4 });
    let start = session.overview().revision;
    let query =
        try_submit(&session, "Q", job("long_query", Vec::new(), vec![Input::Current], Lane::Exclusive, JobKind::Query));
    let endless =
        try_submit(&session, "F1", job("long_query", Vec::new(), vec![Input::Current], Lane::Program, JobKind::File));
    let file = try_submit(
        &session,
        "F2",
        job("count", vec![Value::int(12)], vec![Input::Current], Lane::Program, JobKind::File),
    );
    for admitted in [&query, &endless, &file] {
        assert!(matches!(admitted.status, Status::Running(_)), "{admitted:?}");
    }
    // The short file job may already be done; the two long jobs still run.
    assert!(session.overview().jobs_running >= 2);

    let post = committed(&commit(&session, "C", &[1])).post.clone();
    assert_eq!(finished(&session, &query).status, Status::Cancelled(CancelReason::ReplacedRevision));
    let cancelled = session.wait_job(query.job.unwrap()).unwrap();
    assert_eq!((cancelled.outcome, cancelled.value), (JobOutcome::Cancelled(CancelReason::ReplacedRevision), None));

    let done = finished(&session, &file);
    assert!(matches!(&done.status, Status::Completed(c) if c.kind == CompletionKind::File), "{done:?}");
    let result = session.wait_job(file.job.unwrap()).unwrap();
    assert_eq!((result.input.as_str(), result.value), (start.digest.as_str(), Some(nat(4096))));

    let endless_job = endless.job.unwrap();
    assert_eq!(session.job_result(endless_job).unwrap(), None, "the commit did not cancel the file job");
    assert!(matches!(stop(&session, "stop", endless_job).status, Status::Completed(_)));
    assert_eq!(finished(&session, &endless).status, Status::Cancelled(CancelReason::ByUser));
    let o = session.overview();
    assert_eq!((o.revision.digest.as_str(), o.jobs_running), (post.as_str(), 0));
}

/// CT-KN5-09 (whole), ONE.51: undo and redo succeed while program jobs run; the query on the replaced revision is
/// cancelled and the file job is not.
#[test]
fn undo_succeeds_while_a_program_job_runs() {
    let (session, _disk) = session();
    commit(&session, "A", &[1]);
    let query =
        try_submit(&session, "Q", job("long_query", Vec::new(), vec![Input::Current], Lane::Program, JobKind::Query));
    let file =
        try_submit(&session, "F", job("long_query", Vec::new(), vec![Input::Current], Lane::Exclusive, JobKind::File));
    assert!(matches!(undo(&session, "U").status, Status::Committed(_)));
    assert_eq!(finished(&session, &query).status, Status::Cancelled(CancelReason::ReplacedRevision));
    assert!(matches!(redo(&session, "R").status, Status::Committed(_)));
    assert_eq!(session.job_result(file.job.unwrap()).unwrap(), None);
    stop(&session, "stop", file.job.unwrap());
    assert_eq!(finished(&session, &file).status, Status::Cancelled(CancelReason::ByUser));
}

/// Submits `count` jobs of `lane` at once from as many threads and returns their admission receipts.
fn race(session: &c600_session::Session, prefix: &str, lane: Lane, count: usize) -> Vec<Receipt> {
    let barrier = Barrier::new(count);
    std::thread::scope(|scope| {
        let threads: Vec<_> = (0..count)
            .map(|i| {
                let barrier = &barrier;
                scope.spawn(move || {
                    let spec = job("long_query", Vec::new(), vec![Input::Current], lane, JobKind::Query);
                    barrier.wait();
                    try_submit(session, &format!("{prefix}{i}"), spec)
                })
            })
            .collect();
        threads.into_iter().map(|t| t.join().expect("a submitting thread")).collect()
    })
}

fn admitted(receipts: &[Receipt]) -> Vec<Receipt> {
    receipts.iter().filter(|r| matches!(r.status, Status::Running(_))).cloned().collect()
}

fn lane_full(receipts: &[Receipt]) -> usize {
    receipts.iter().filter(|r| matches!(&r.status, Status::Refused(e) if e.code == code::LANE_FULL)).count()
}

/// CT-DET-12 (part: one-job admission is atomic: eight concurrent submissions to a one-job lane admit one and refuse
/// seven, a held key starts nothing more, and the program lane never passes its capacity; not run: other jobs'
/// scheduling varied around a fixed run's payload and coverage, and child completion order).
/// CT-KN7-05 (whole), R-16: concurrent invocations and a held key cannot pass a one-job limit, and the program lane
/// admits no more than its declared capacity. A held key's repeats either carry the first command ID and get its
/// receipt, or carry new ones and find the lane full.
/// CT-LI-03 (part: the session's exclusive and program lanes under concurrent submission; not run: all six lanes at
/// once in the command layer).
#[test]
fn concurrent_submissions_cannot_pass_a_lane_limit() {
    let (session, _disk) = session();
    let receipts = race(&session, "X", Lane::Exclusive, 8);
    let exclusive = admitted(&receipts);
    assert_eq!((exclusive.len(), lane_full(&receipts)), (1, 7));
    let first = exclusive[0].clone();
    for _ in 0..3 {
        let held = try_submit(
            &session,
            &first.command,
            job("long_query", Vec::new(), vec![Input::Current], Lane::Exclusive, JobKind::Query),
        );
        assert_eq!(held, first, "a held key returns the first receipt and starts nothing");
    }
    let other = try_submit(
        &session,
        &first.command,
        job("count", vec![Value::int(1)], vec![Input::Current], Lane::Exclusive, JobKind::Query),
    );
    assert_eq!(code(&other), code::COMMAND_ID_REUSED);
    assert_eq!(session.overview().jobs_running, 1);

    let capacity = Options::default().program_capacity;
    let receipts = race(&session, "P", Lane::Program, capacity + 3);
    let program = admitted(&receipts);
    assert_eq!((program.len(), lane_full(&receipts)), (capacity, 3));
    assert_eq!(session.overview().jobs_running, capacity + 1);

    for (i, receipt) in program.iter().chain([&first]).enumerate() {
        stop(&session, &format!("stop{i}"), receipt.job.unwrap());
        assert_eq!(finished(&session, receipt).status, Status::Cancelled(CancelReason::ByUser));
    }
    assert_eq!(session.overview().jobs_running, 0);
    let freed = race(&session, "Y", Lane::Exclusive, 1);
    assert!(matches!(freed[0].status, Status::Running(_)), "a stopped job frees its lane");
    stop(&session, "stop.last", freed[0].job.unwrap());
    finished(&session, &freed[0]);

    let unknown = stop(&session, "stop.unknown", c600_session::JobId { epoch: session.epoch(), n: 999 });
    assert_eq!(code(&unknown), code::RESULT_GONE);
}

/// CT-KN7-07 (whole), KN7.5, KN7.3: after a commit, a digest-pinned job finishes on its original input. Its result
/// applies to a revision with that input's digest, an undo back to it included, and never to another revision without a
/// re-run. CT-ONE-24 (part: the pinned job finishes after a commit and its result is refused against the newer
/// revision; the view model refusing it at the newer revision runs in `c600-durable/tests/ct_one_24.rs`; not run: a
/// host that binds a job's result to a view, LP3).
#[test]
fn a_pinned_job_finishes_on_its_input_and_binds_its_result_to_it() {
    let (session, _disk) = session();
    commit(&session, "A", &[1]);
    let start = session.overview().revision;
    let endless = try_submit(
        &session,
        "P1",
        job("long_query", Vec::new(), vec![Input::Current], Lane::Exclusive, JobKind::Pinned),
    );
    let pinned =
        try_submit(&session, "P2", job("moved", Vec::new(), vec![Input::Current], Lane::Program, JobKind::Pinned));
    commit(&session, "C", &[2]);
    let newer = session.overview().revision;

    assert!(matches!(&finished(&session, &pinned).status, Status::Completed(c) if c.kind == CompletionKind::Query));
    let job_id = pinned.job.unwrap();
    let result = session.wait_job(job_id).unwrap();
    assert_eq!(result.input, start.digest, "the pinned job ran on its original input");
    assert_eq!(session.job_result_at(job_id, &newer).unwrap_err().code, code::JOB_INPUT_DIGEST);
    assert_eq!(session.job_result_at(job_id, &start).unwrap(), result);

    undo(&session, "U");
    let back = session.overview().revision;
    assert_ne!(back.state, start.state);
    assert_eq!(session.job_result_at(job_id, &back).unwrap(), result, "an equal digest after an undo is a match");

    redo(&session, "R");
    let current = session.overview().revision;
    let (_, rerun) =
        run(&session, "P3", job("moved", Vec::new(), vec![Input::Current], Lane::Program, JobKind::Pinned));
    assert_eq!(rerun.input, current.digest, "a re-run binds the result to the newer revision");
    assert_ne!(rerun.value, result.value);

    assert_eq!(session.job_result(endless.job.unwrap()).unwrap(), None, "commits, undo and redo left it running");
    stop(&session, "stop", endless.job.unwrap());
    assert_eq!(finished(&session, &endless).status, Status::Cancelled(CancelReason::ByUser));
}

/// CT-KN4-01 (part: a program job in the engine's process; not run: an operating-system check that the runtime's
/// child opens no session file; its source scan is in `c600-runtime`): a program job has no row that opens a
/// session file. A file read or write, a workspace write and a proposal are each refused by the job's host, and the
/// store is not touched.
#[test]
fn a_program_job_opens_no_session_file() {
    let (session, disk) = session();
    commit(&session, "A", &[1]);
    let (reads, writes) = (disk.reads(), disk.writes());
    let word = || vec![Value::word(&[1])];
    for (function, args) in [("peek", Vec::new()), ("save", word()), ("keep", word()), ("offer", word())] {
        let admitted =
            try_submit(&session, function, job(function, args, vec![Input::Current], Lane::Program, JobKind::File));
        let done = finished(&session, &admitted);
        assert_eq!(code(&done), NOT_GRANTED, "{function}");
    }
    assert_eq!((disk.reads(), disk.writes()), (reads, writes), "no job touched the store");
}

/// The first segments of the `std::` paths in Rust source with its line comments removed. A brace group gives the
/// first segment of each of its items.
fn std_modules(source: &str) -> Vec<String> {
    let ident = |s: &str| s.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>();
    let code: Vec<&str> = source.lines().map(|line| line.split("//").next().unwrap_or("")).collect();
    let code = code.join("\n");
    let mut found = Vec::new();
    let mut rest = code.as_str();
    while let Some(at) = rest.find("std::") {
        let inside_a_name = rest[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_');
        rest = &rest[at + "std::".len()..];
        if inside_a_name {
            continue;
        }
        let Some(group) = rest.trim_start().strip_prefix('{') else {
            found.push(ident(rest));
            continue;
        };
        let (mut depth, mut item) = (0usize, String::new());
        for c in group.chars() {
            match c {
                '{' => depth += 1,
                '}' if depth == 0 => break,
                '}' => depth -= 1,
                ',' if depth == 0 => found.push(ident(&std::mem::take(&mut item))),
                _ => {}
            }
            if depth > 0 || !matches!(c, ',' | '{' | '}') {
                item.push(c);
            }
        }
        if !ident(&item).is_empty() {
            found.push(ident(&item));
        }
    }
    found
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// CT-KN4-01 (part: `c600-run`'s source; not run: an operating-system check of the runtime's child, whose source scan
/// is in `c600-runtime`'s `ct_kn4_01`): `c600-run` makes no file-system
/// call of its own. No path of its source reaches the standard library's file system, paths, environment, I/O,
/// operating system, network or processes.
#[test]
fn the_interpreter_makes_no_file_system_call_of_its_own() {
    let sample = std_modules("use std::{fs, io::{self, Write}};\nlet x = std::path::Path::new(\"a\"); // std::net\n");
    assert_eq!(sample, ["fs", "io", "path"], "the scan reads groups and skips comments");

    const DENIED: [&str; 7] = ["fs", "path", "env", "io", "os", "net", "process"];
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("c600-run").join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.len() >= 8, "the interpreter's sources were found");
    let mut seen = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file).expect("a source file");
        for module in std_modules(&source) {
            assert!(!DENIED.contains(&module.as_str()), "{} uses std::{module}", file.display());
            seen.push(module);
        }
    }
    assert!(seen.iter().any(|m| m == "sync"), "the scan found the interpreter's std paths");
}
