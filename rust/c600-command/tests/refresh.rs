//! KN5.22 and R-02: the shell reports a command's receipt and the display's refresh apart, so a command that applied is
//! never reported as failed. The display here is a test double whose refresh can fail, as the host's adoption of the
//! engine's bundle into LP3's view model can (`refresh_required`, RPI.9). The tests port 0.4's
//! `tests/test_native_reply_after_commit.py`: a commit whose display refresh fails is applied and says so, and a
//! command that applied nothing keeps its own outcome.
//!
//! CT-KN5-05 (part: the shell's report after a commit sent or pressed, a workspace write, a view and a file
//! publication, ONE.70's row for a refresh failure but its handles, and the refresh that follows; not run: the host's
//! loop that reports each command, its refresh over LP3's view model and renderer, an app row's own completion, the
//! runtime's handles, and the message on screen).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    draft_args, refresh_display, report, CancelReason, CommandLayer, Completion, CompletionKind, DisplayStatus, Error,
    Fate, HeadlessFrames, Invocation, Kind, MemoryStore, Overlay, Receipt, Refresh, Revision, Secret, Shell, Status,
    Table, DISPLAY_OUT_OF_DATE,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/full".into(), entry: entry.into() }
}

/// The failure the display reports: the view model refuses the engine's bundle (RPI.9).
fn injected() -> Error {
    Error::new("EV3:input.display_bundle_mismatch", "injected refresh failure")
}

/// A display whose refresh fails while `failure` is set. It records each revision it was asked to show.
#[derive(Default)]
struct TestDisplay {
    failure: Option<Error>,
    asked: Vec<Revision>,
}

impl TestDisplay {
    fn failing() -> TestDisplay {
        TestDisplay { failure: Some(injected()), asked: Vec::new() }
    }
}

impl Refresh for TestDisplay {
    fn refresh(&mut self, revision: &Revision) -> Result<(), Error> {
        self.asked.push(revision.clone());
        self.failure.clone().map_or(Ok(()), Err)
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A solved session with a query, a workspace, a view and a file overlay, and room for all their jobs.
fn fixture() -> Fixture {
    let k = kernel();
    let protected = k.protected_set(&[]).unwrap();
    let start = k.solved();
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    for (row, kind) in
        [("inspect", Kind::Query), ("note", Kind::Workspace), ("view", Kind::View), ("export", Kind::File)]
    {
        layer.register_overlay(&format!("tool.full.{row}"), kind, &[], overlay(row)).unwrap();
    }
    layer.set_program_capacity(4);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    /// Previews `recipe` and presents it; returns the commit press, prepared but not sent.
    fn press(&mut self, recipe: &Value) -> Invocation {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
        assert_eq!(self.layer.run_jobs(), 1);
        let id = self.layer.preview().expect("a pending preview").id;
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let args = json::object([("preview", Value::Int(id as i64))]);
        self.shell.prepare(&self.layer, "operation.execute", args).unwrap()
    }

    fn invoke(&mut self, row: &str) -> Receipt {
        self.shell.invoke(&mut self.layer, row, json::object([])).unwrap()
    }

    /// Takes the next program job, which must run `row`, and finishes it; returns its terminal receipt.
    fn finish(&mut self, row: &str) -> Receipt {
        let job = self.layer.take_program_job().unwrap();
        assert_eq!(job.row, row);
        self.layer.finish_program_job(job.job, Ok(Value::String(row.into()))).unwrap()
    }

    /// What a refresh may not change: the revision and the durable log.
    fn fixed(&self) -> (Revision, usize) {
        (self.layer.revision().clone(), self.layer.store().log.len())
    }
}

/// CT-KN5-05, R-02, ONE.70: a refresh failure after a commit leaves the commit as it is. The report says the command
/// applied, with "Applied, display out of date"; its receipt is the one the layer's outcome lookup returns; the pending
/// preview is discarded and a query job on the replaced revision cancelled, as after any commit, while a file job keeps
/// running. Every receipt, the revision and the log equal those of the same session whose refresh works. The
/// continuation refreshes the display and sends no command, the same press sent again returns the same receipt and
/// commits nothing (ONE.71), and undo still works.
#[test]
fn a_refresh_failure_after_a_commit_reports_applied_display_out_of_date() {
    let run = |display: &mut TestDisplay| {
        let mut f = fixture();
        let jobs = [f.invoke("tool.full.inspect"), f.invoke("tool.full.export")];
        let press = f.press(&word(&[1]));
        let reported = f.shell.send_reported(&mut f.layer, press.clone(), display);
        (f, jobs, press, reported)
    };
    let mut working = TestDisplay::default();
    let (w, w_jobs, _, w_reported) = run(&mut working);
    let mut failing = TestDisplay::failing();
    let (mut f, jobs, press, reported) = run(&mut failing);

    let head = f.layer.revision().clone();
    let Status::Committed(record) = &reported.receipt.status else { panic!("{:?}", reported.receipt.status) };
    assert_eq!(record.revision, head);
    assert!(reported.applied());
    assert_eq!(reported.display, DisplayStatus::OutOfDate { revision: head.clone(), error: injected() });
    assert_eq!(reported.message(), Some(DISPLAY_OUT_OF_DATE));
    assert_eq!(failing.asked, vec![head.clone()]);
    let outcome = f.layer.outcome(&reported.receipt.command, &reported.receipt.request);
    assert_eq!(outcome.as_ref(), Some(&reported.receipt));

    // ONE.70: as after any commit.
    assert!(f.layer.preview().is_none() && f.shell.presented().is_none());
    let [query, export] = &jobs;
    let ended = f.layer.outcome(&query.command, &query.request).unwrap();
    assert_eq!(ended.status, Status::Cancelled(CancelReason::ReplacedRevision));
    assert_eq!(f.layer.outcome(&export.command, &export.request).as_ref(), Some(export));

    // The session whose refresh works differs in the display alone.
    assert_eq!((&w_reported.receipt, &w_jobs), (&reported.receipt, &jobs));
    assert_eq!(w_reported.display, DisplayStatus::Current(head.clone()));
    assert!(w_reported.applied() && w_reported.message().is_none());
    assert_eq!(w.fixed(), f.fixed());
    for receipt in &jobs {
        assert_eq!(
            w.layer.outcome(&receipt.command, &receipt.request),
            f.layer.outcome(&receipt.command, &receipt.request)
        );
    }

    // No retry: the continuation refreshes the display, and the same press commits nothing more. The continuation
    // takes the layer by shared reference and no shell, so it cannot send a command.
    let fixed = f.fixed();
    failing.failure = None;
    assert_eq!(refresh_display(&f.layer, &mut failing), DisplayStatus::Current(head.clone()));
    assert_eq!(f.fixed(), fixed);
    assert_eq!(f.shell.send(&mut f.layer, press), reported.receipt);
    assert_eq!(f.fixed(), fixed);

    // Undo by its key applies, and its report shows the display apart again.
    failing.failure = Some(injected());
    let undone = f.shell.press(&mut f.layer, "Ctrl+Z", false).unwrap().expect("a bound key");
    let undone = report(&f.layer, undone, &mut failing);
    assert!(matches!(undone.receipt.status, Status::Committed(_)));
    assert_eq!(undone.display, DisplayStatus::OutOfDate { revision: f.layer.revision().clone(), error: injected() });
    assert_eq!(f.layer.revision().digest, fixture().layer.revision().digest);
}

/// KN5.22: the same separation holds when a workspace write, a view publication or a file publication completes. A
/// refresh failure leaves each completion, its revision and the log as they are.
#[test]
fn a_completed_workspace_view_or_file_job_keeps_its_completion() {
    let mut f = fixture();
    let mut display = TestDisplay::failing();
    for (row, kind) in [
        ("tool.full.note", CompletionKind::Workspace),
        ("tool.full.view", CompletionKind::ViewPublished),
        ("tool.full.export", CompletionKind::File),
    ] {
        assert!(matches!(f.invoke(row).status, Status::Running(_)), "{row}");
        let done = f.finish(row);
        assert!(matches!(&done.status, Status::Completed(Completion { kind: k, .. }) if *k == kind), "{row}");
        let fixed = f.fixed();
        let reported = report(&f.layer, done.clone(), &mut display);
        assert!(reported.applied(), "{row}");
        assert_eq!(reported.receipt, done, "{row}");
        assert_eq!(reported.message(), Some(DISPLAY_OUT_OF_DATE), "{row}");
        assert_eq!(f.layer.outcome(&done.command, &done.request).as_ref(), Some(&done), "{row}");
        assert_eq!(f.fixed(), fixed, "{row}");
    }
    assert_eq!(display.asked.len(), 3);
}

/// 0.4's second case: a command that applied nothing keeps its own outcome, and no refresh is due. A refusal, a
/// preview's creation, a query's answer, an app row's completion and an unknown commit fate are reported as they are,
/// and the display is not asked.
#[test]
fn a_command_that_applied_nothing_keeps_its_outcome() {
    let mut f = fixture();
    let mut display = TestDisplay::failing();
    let mut quiet = |f: &mut Fixture, receipt: Receipt| {
        let reported = report(&f.layer, receipt.clone(), &mut display);
        assert_eq!(reported.receipt, receipt);
        assert_eq!(reported.display, DisplayStatus::NotDue);
        assert!(!reported.applied() && reported.message().is_none());
    };

    let preview = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
    assert!(matches!(preview.status, Status::Running(_)));
    quiet(&mut f, preview.clone());
    f.layer.run_jobs();
    let ready = f.layer.outcome(&preview.command, &preview.request).unwrap();
    assert!(matches!(ready.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. })));
    quiet(&mut f, ready);

    assert!(matches!(f.invoke("tool.full.inspect").status, Status::Running(_)));
    let answer = f.finish("tool.full.inspect");
    assert!(matches!(answer.status, Status::Completed(Completion { kind: CompletionKind::Query, .. })));
    // The headless layer completes no app row, so the test builds the receipt of one from the query's.
    let Status::Completed(done) = &answer.status else { panic!("{:?}", answer.status) };
    let app = Receipt {
        status: Status::Completed(Completion { kind: CompletionKind::App, ..done.clone() }),
        ..answer.clone()
    };
    quiet(&mut f, answer);
    quiet(&mut f, app);

    let stale = json::object([("preview", Value::Int(99))]);
    let refused = f.shell.invoke(&mut f.layer, "session.undo", stale).unwrap();
    assert!(matches!(refused.status, Status::Refused(_)), "{:?}", refused.status);
    quiet(&mut f, refused);

    let press = f.press(&word(&[2]));
    f.layer.store_mut().inject.push_back(Fate::Unknown);
    let unknown = f.shell.send(&mut f.layer, press);
    assert_eq!(unknown.status, Status::UnknownOutcome);
    quiet(&mut f, unknown);
    assert!(display.asked.is_empty());
}

/// CT-LI-02 (part: each kind but `app` ends in the terminal LI.10 gives it: `committed`, and `completed` as a workspace
/// write, a query's answer, a view's publication, a file's publication and a preview's readiness; the outcome lookup a
/// shell that lost the notification makes returns each receipt unchanged; after a restart, the lookup still returns the
/// durable `commit` and `workspace` terminals and answers `unknown_outcome` for the transient ones; not run: an `app`
/// row's terminal, since the headless layer builds no app row).
#[test]
fn a_restart_keeps_the_durable_terminals_and_loses_the_transient_ones() {
    let mut f = fixture();
    let mut terminals = Vec::new();
    for (row, kind) in [
        ("tool.full.note", CompletionKind::Workspace),
        ("tool.full.inspect", CompletionKind::Query),
        ("tool.full.view", CompletionKind::ViewPublished),
        ("tool.full.export", CompletionKind::File),
    ] {
        assert!(matches!(f.invoke(row).status, Status::Running(_)), "{row}");
        let done = f.finish(row);
        assert!(matches!(&done.status, Status::Completed(Completion { kind: k, .. }) if *k == kind), "{row}");
        terminals.push((row, kind == CompletionKind::Workspace, done));
    }
    let preview = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[3]))).unwrap();
    assert_eq!(f.layer.run_jobs(), 1);
    let ready = f.layer.outcome(&preview.command, &preview.request).unwrap();
    assert!(matches!(ready.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. })));
    terminals.push(("operation.preview", false, ready));
    let press = f.press(&word(&[1]));
    let committed = f.shell.send(&mut f.layer, press);
    assert!(matches!(committed.status, Status::Committed(_)), "{:?}", committed.status);
    terminals.push(("operation.execute", true, committed));

    for (row, _, receipt) in &terminals {
        assert_eq!(f.layer.outcome(&receipt.command, &receipt.request).as_ref(), Some(receipt), "{row}");
    }
    f.layer.restart(Secret::fixed([8; 32]));
    for (row, durable, receipt) in &terminals {
        let after = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
        if *durable {
            assert_eq!(&after, receipt, "{row}");
        } else {
            assert_eq!(after.status, Status::UnknownOutcome, "{row}");
            assert_eq!((&after.command, &after.request), (&receipt.command, &receipt.request), "{row}");
        }
    }
}
