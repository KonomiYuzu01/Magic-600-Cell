//! CT-KN5-05's fault seam after the store's write (L12 of the KN5 attack leads; KN5.22, R-02): a panic that unwinds
//! between the write of an action and its adoption, and a panic in the display's refresh after it. The write landed, so
//! the action is never reported as failed, its retry never writes it again, and the layer, its restart and a reopen of
//! its store agree on its receipt.
//!
//! CT-KN5-05 (part: a panic after the write of a commit, an undo and a workspace completion, and a panic in the
//! display's refresh after a commit; not run: a panic inside the adoption itself, which has no seam, and the host's
//! loop that catches the panic and restarts or reopens the session).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, Caller, CommandLayer, Completion, CompletionKind, Direction, Error, Fate, Genesis,
    HeadlessFrames, History, Invocation, Kind, MemoryStore, Overlay, Receipt, Refresh, Reopen, Revision, Secret, Shell,
    Status, Store, Table, Transaction,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const NOTE_ROW: &str = "tool.full.note";

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

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// The in-memory store, which writes every transaction it is given and, once armed, panics after the write and before
/// it answers: a fault between an action's durable write and its adoption.
#[derive(Clone, Debug, Default)]
struct Panicky {
    inner: MemoryStore,
    armed: bool,
}

impl Store for Panicky {
    fn transact(&mut self, transaction: &Transaction) -> Fate {
        let fate = self.inner.transact(transaction);
        if std::mem::take(&mut self.armed) {
            panic!("the test store panics after the write landed");
        }
        fate
    }

    fn durable(&self, event: &str) -> Option<bool> {
        self.inner.durable(event)
    }

    fn begin(&mut self, genesis: &Genesis) -> Fate {
        self.inner.begin(genesis)
    }

    fn begin_epoch(&mut self, epoch: u64) -> Fate {
        self.inner.begin_epoch(epoch)
    }
}

impl Reopen for Panicky {
    fn history(&mut self) -> Result<History, Error> {
        self.inner.history()
    }
}

/// A display whose refresh panics.
struct PanickingDisplay;

impl Refresh for PanickingDisplay {
    fn refresh(&mut self, _: &Revision) -> Result<(), Error> {
        panic!("the test display panics in its refresh");
    }
}

struct Fixture {
    layer: CommandLayer<Panicky>,
    shell: Shell,
    frames: HeadlessFrames,
}

fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let layer = CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), Panicky::default(), "s1", start, protected);
    let mut f = Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() };
    let note = Overlay { module: "tools/full".into(), entry: "note".into() };
    f.layer.register_overlay(NOTE_ROW, Kind::Workspace, &[], note).unwrap();
    f
}

impl Fixture {
    /// Previews `recipe`, runs its job and presents it; returns the preview's ID.
    fn presented(&mut self, recipe: &Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        id
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let id = self.presented(recipe);
        let press = self.shell.prepare(&self.layer, "operation.execute", preview_args(id)).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    fn logged(&self) -> usize {
        self.layer.store().inner.log.len()
    }
}

/// The action under test, prepared and not sent, after what it needs: a presented preview for a commit, a commit for
/// an undo, and nothing for the workspace row, whose completion the host reports.
fn prepared(f: &mut Fixture, action: &str) -> Invocation {
    match action {
        "execute" => {
            let id = f.presented(&word(&[1]));
            f.shell.prepare(&f.layer, "operation.execute", preview_args(id)).unwrap()
        }
        "undo" => {
            assert!(committed(&f.commit(&word(&[1]))));
            let hint = f.layer.undo_hint(Direction::Undo).expect("an eligible target");
            f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap()
        }
        _ => f.shell.prepare(&f.layer, NOTE_ROW, json::object([])).unwrap(),
    }
}

/// CT-KN5-05 (part), L12: the store writes a commit, an undo or a workspace completion and panics before it answers.
/// Until the restart the layer is in read-only recovery with `commit_fate`, and the lookup never answers `failed`: a
/// commit row's command is unknown (`unknown_outcome`) and the workspace row's job still running. The restart reads the
/// store's answer and adopts the action: the lookup and every retry answer the same `committed` or
/// `completed(workspace)` receipt and write nothing, the commit lane is free for the next commit, and a reopen of the
/// store answers the same receipt at the same state.
#[test]
fn a_panic_after_the_write_is_settled_by_the_restart() {
    for action in ["execute", "undo", "workspace"] {
        let mut f = fixture();
        let asked = prepared(&mut f, action);
        let (unwound, before, lookup) = if action == "workspace" {
            let started = f.shell.send(&mut f.layer, asked.clone());
            assert!(matches!(started.status, Status::Running(_)), "{action}: {:?}", started.status);
            let job = f.layer.take_program_job().expect("the row's job").job;
            let before = (f.logged(), f.layer.revision().clone());
            f.layer.store_mut().armed = true;
            let unwound = catch_unwind(AssertUnwindSafe(|| {
                f.layer.finish_program_job(job, Ok(Value::String("note".into())));
            }));
            (unwound, before, started.request)
        } else {
            let before = (f.logged(), f.layer.revision().clone());
            f.layer.store_mut().armed = true;
            let unwound = catch_unwind(AssertUnwindSafe(|| {
                f.shell.send(&mut f.layer, asked.clone());
            }));
            (unwound, before, "never recorded".to_owned())
        };
        assert!(unwound.is_err(), "{action}: the store panicked");
        assert_eq!(f.logged(), before.0 + 1, "{action}: the write landed");
        assert_eq!(f.layer.revision(), &before.1, "{action}: nothing was adopted");
        assert_eq!(f.layer.recovery().map(|e| e.code), Some(code::COMMIT_FATE), "{action}");
        assert_eq!(f.layer.availability("operation.preview").unwrap_err().code, code::READ_ONLY_RECOVERY);
        let pending = f.layer.kernel_outcome(&Caller::Shell, &asked.command, &lookup).status;
        if action == "workspace" {
            assert!(matches!(pending, Status::Running(_)), "{action}: {pending:?}");
        } else {
            assert_eq!(pending, Status::UnknownOutcome, "{action}");
        }

        f.layer.restart(Secret::fixed(SECRET));
        f.shell.restart(Secret::fixed(SECRET));
        assert!(f.layer.recovery().is_none(), "{action}");
        let answer = f.layer.invoke(&Caller::Shell, asked.clone());
        let applied = if action == "workspace" { completed(&answer) } else { committed(&answer) };
        assert!(applied, "{action}: {:?}", answer.status);
        assert_eq!(f.layer.outcome(&asked.command, &answer.request), Some(answer.clone()), "{action}");
        assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), answer, "{action}");
        assert_eq!(f.logged(), before.0 + 1, "{action}: a retry writes nothing");
        // The counters, not the whole revision: the restart's new epoch changes the revision on its own.
        let counters = |r: &Revision| (r.head.clone(), r.state, r.workspace);
        assert_ne!(counters(f.layer.revision()), counters(&before.1), "{action}: the restart adopted the action");

        assert!(committed(&f.commit(&word(&[2]))), "{action}: the commit lane is free");
        let state = f.layer.snapshot().digest().to_owned();
        let store = f.layer.store().clone();
        let mut reopened = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), store).unwrap();
        assert_eq!(reopened.outcome(&asked.command, &answer.request), Some(answer.clone()), "{action}");
        assert_eq!(reopened.invoke(&Caller::Shell, asked), answer, "{action}");
        assert_eq!(reopened.snapshot().digest(), state, "{action}");
        assert_eq!(reopened.store().inner.log.len(), before.0 + 2, "{action}");
    }
}

/// CT-KN5-05 (part), L12: the display's refresh panics after a commit the layer adopted. The commit stands: the
/// lookup and a retry answer its `committed` receipt, nothing is written again, the layer is not in recovery, and the
/// next commit goes ahead.
#[test]
fn a_panic_in_the_display_refresh_leaves_the_commit_standing() {
    let mut f = fixture();
    let id = f.presented(&word(&[1]));
    let press = f.shell.prepare(&f.layer, "operation.execute", preview_args(id)).unwrap();
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        f.shell.send_reported(&mut f.layer, press.clone(), &mut PanickingDisplay);
    }));
    assert!(unwound.is_err(), "the display panicked");
    assert!(f.layer.recovery().is_none());
    assert_eq!(f.logged(), 1);
    let answer = f.layer.invoke(&Caller::Shell, press.clone());
    assert!(committed(&answer), "{:?}", answer.status);
    assert_eq!(f.layer.outcome(&press.command, &answer.request), Some(answer.clone()));
    assert_eq!(f.logged(), 1);
    assert!(committed(&f.commit(&word(&[2]))));
    assert_eq!(f.logged(), 2);
}
