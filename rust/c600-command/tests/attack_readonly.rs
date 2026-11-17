//! Attack test on the reads and previews (owner-requested attack of the command layer, class C: a preview or a read
//! that changes state). Rule: KN5.10 [B] and ONE.34 [D]: a preview shows an effect and changes nothing; a query, an
//! availability check, an outcome lookup and a hint read change nothing. The test drives every non-committing entry
//! the layer has from the shell and from a run, after a commit and a record write, and asks that the journal, the
//! revision, the state, the protected set, the records and both undo targets are what they were.
//!
//! Refuted on commit 4c20ce30: the test passes and stays as a regression guard.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use c600_command::{
    draft_args, Caller, CommandLayer, Direction, Error, HeadlessFrames, Invocation, MemoryStore, Provenance, Receipt,
    Secret, Shell, Status, Table, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

/// A reader that admits every provenance block: the one record here carries none worth checking.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    Arc::clone(KERNEL.get_or_init(|| {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        Arc::new(Kernel::load(&assets).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}")))
    }))
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    sent: u64,
}

impl Fixture {
    /// The solver's preview of `recipe`, built and presented; its ID.
    fn preview(&mut self, recipe: &Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        self.layer.run_jobs();
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        id
    }

    /// One envelope on a run's channel, with no token.
    fn runtime(&mut self, row: &str, args: Value) -> Receipt {
        self.sent += 1;
        let invocation = Invocation {
            command: format!("run-1/{}", self.sent),
            row: row.to_owned(),
            args,
            token: None,
            epoch: self.layer.revision().epoch,
        };
        self.layer.invoke(&Caller::Runtime { run: "run-1".into() }, invocation)
    }

    /// Everything durable the layer shows.
    fn durable(&self) -> String {
        format!(
            "{} transactions; {:?}; snapshot {}; protected {:?}; record {:?}; undo {:?}; redo {:?}",
            self.layer.store().log.len(),
            self.layer.revision(),
            self.layer.snapshot().digest(),
            self.layer.protected().orbits(),
            self.layer.record("macro").map(|record| record.payload.canonical()),
            self.layer.undo_hint(Direction::Undo),
            self.layer.undo_hint(Direction::Redo)
        )
    }
}

#[test]
fn previews_reads_and_refusals_change_nothing_durable() {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    let shell = Shell::new(Secret::fixed(SECRET), "shell");
    let mut f = Fixture { layer, shell, frames: HeadlessFrames::new(), sent: 0 };

    // A session with something to lose: one commit, one record write, one undo (so both stacks hold a target).
    let id = f.preview(&word(&[1]));
    let press = f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))]));
    let committed = f.shell.send(&mut f.layer, press.unwrap());
    assert!(matches!(committed.status, Status::Committed(_)), "{:?}", committed.status);
    let args = json::object([("payload", word(&[2])), ("provenance", text("solver")), ("record", text("macro"))]);
    let written = f.shell.invoke(&mut f.layer, RECORD_ROW, args).unwrap();
    assert!(matches!(written.status, Status::Completed(_)), "{:?}", written.status);
    let id = f.preview(&word(&[3]));
    let press = f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))]));
    assert!(matches!(f.shell.send(&mut f.layer, press.unwrap()).status, Status::Committed(_)));
    let hint = f.layer.undo_hint(Direction::Undo).expect("an undo target");
    let undone = f.shell.invoke(&mut f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
    let before = f.durable();

    // The solver's previews: presented, replaced, cancelled by the row and by Escape.
    let first = f.preview(&word(&[4]));
    let second = f.preview(&word(&[5, 6]));
    assert_ne!(first, second);
    let cancel = json::object([("preview", Value::Int(second as i64))]);
    let cancelled = f.shell.invoke(&mut f.layer, "operation.preview.cancel", cancel).unwrap();
    assert!(matches!(cancelled.status, Status::Completed(_)), "{:?}", cancelled.status);
    f.preview(&word(&[7]));
    let _ = f.shell.press(&mut f.layer, "Escape", false);
    assert_eq!(f.durable(), before, "the solver's previews and cancels");

    // A press on a preview that ended, and a second delivery of the commit's own envelope.
    let stale = f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(first as i64))]));
    if let Ok(stale) = stale {
        assert!(matches!(f.shell.send(&mut f.layer, stale).status, Status::Refused(_)));
    }
    let _ = f.layer.outcome(&committed.command, &committed.request);
    let _ = f.layer.kernel_outcome(&Caller::Shell, "shell/999", &committed.request);
    assert_eq!(f.durable(), before, "a stale press and the outcome lookups");

    // Every row's availability, and both hints.
    let rows: Vec<String> = f.layer.table().rows().map(|row| row.id.clone()).collect();
    for row in &rows {
        let _ = f.layer.availability(row);
    }
    let _ = (f.layer.undo_hint(Direction::Undo), f.layer.undo_hint(Direction::Redo));
    assert_eq!(f.durable(), before, "availability and hints");

    // A run: its preview and the preview's job, and one envelope for every row of the table with the arguments of a
    // preview. A row may refuse the arguments or the caller; none may change anything durable.
    let previewed = f.runtime("operation.preview", draft_args(&word(&[8])));
    if matches!(previewed.status, Status::Running(_)) {
        f.layer.run_jobs();
    }
    for row in &rows {
        let _ = f.runtime(row, draft_args(&word(&[9])));
        f.layer.run_jobs();
    }
    assert_eq!(f.durable(), before, "a run's envelopes for every row");
}
