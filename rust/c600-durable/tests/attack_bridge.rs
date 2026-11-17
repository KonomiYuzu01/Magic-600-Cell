//! Attack test over the product's own bridge and M6's journal (owner-requested attack of the command layer, class A: a
//! durable change with no solver action; finding B3, which `c600-command/tests/attack_runtime.rs` shows at the layer
//! with a hand-made envelope). Here the request goes the way a program's `propose(word, target("..."))` goes after
//! the interpreter: `Host::perform` on `c600_tools::bridge::Bridge` with a `Request::Propose`, over the command layer
//! on `JournalStore`. The bridge filters no row (its own words), so the row the program names reaches the layer.
//!
//! What the test supplies beside the product's code: the registration of one language-defined workspace row that
//! takes a `draft` argument, and the host step that serves the admitted job with one record. No host in the tree
//! serves program jobs yet, so this is what the layer and the bridge allow once one does. Whether the checker lets a
//! program name such a row as a proposal target was not examined.
//!
//! The test uses a fresh memory disk and no other session.

mod common;

use c600_command::{
    code, CommandLayer, Error, JobOutput, Kind, Overlay, Provenance, RecordValue, Secret, Status, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_run::interp::{Host, Request};
use c600_session::disk::MemDisk;
use c600_session::journal;
use c600_session::JournalStore;
use c600_tools::bridge::Bridge;

const ROW: &str = "tool.notes.save";

/// A reader that admits every provenance block: the records here carry none worth checking.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

/// H-08 with the layer's drafts and one language-defined workspace row whose argument is a draft.
fn table() -> Table {
    let mut table = Table::h08().with_drafts();
    let overlay = Overlay { module: "notes".into(), entry: "save".into() };
    table.register_overlay(ROW, Kind::Workspace, &[("draft", "Draft")], overlay).expect("the row registers");
    table
}

/// The journal's action records, in order.
fn actions(disk: &MemDisk) -> Vec<Value> {
    let bytes = disk.durable(common::JOURNAL).expect("the disk holds a journal");
    let scan = journal::scan(&bytes);
    let records = scan.payloads.iter().map(|payload| json::parse(payload).expect("a record"));
    records.filter(|record| record.get("record").and_then(Value::as_str) == Some("command_action")).collect()
}

/// Finding B3 over the bridge (class A, attacker 1). Rule: ONE.52 [B] "A workspace change by a program happens inside
/// a command the solver invokes, as one undoable action"; KN4.4 [D] "Workspace records change only through workspace
/// commands". No solver action happens in this test: no press, no shell invocation, no token. A run's proposal names
/// the workspace row as its target; the bridge sends it on the run's channel; the layer admits a program job; the job's
/// completion is written to the journal.
///
/// Observed on 4c20ce30: the bridge's `perform` answers `running`, the journal on the disk gains one `command_action`
/// record of kind `workspace` whose caller is `{"runtime": "run-1"}`, and after a reopen the record `note` holds the
/// run's text.
///
/// Fixed: a run reaches no workspace-kind row, so the layer refuses the bridge's invocation with
/// `LG3:capability.not_granted`, the bridge answers the boundary with that code, and nothing is journaled.
#[test]
fn b3_a_proposal_that_names_a_workspace_row_writes_nothing_durable() {
    let disk = MemDisk::new();
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let secret = Secret::fixed([7; 32]);
    let mut layer = CommandLayer::new(k.clone(), table(), secret, store, "s1", k.solved(), ProtectedSet::none());
    layer.set_provenance(Box::new(Reader));
    assert!(actions(&disk).is_empty());

    // The run's side: one proposal, as the interpreter hands it to its host.
    let answer = {
        let mut bridge = Bridge::new(&mut layer, "run-1");
        let request = Request::Propose { word: &[1], target: ROW, request: b"" };
        let answer = bridge.perform(&request);
        assert_eq!(bridge.sent.len(), 1);
        assert!(bridge.sent[0].invocation.token.is_none());
        (answer.is_ok(), bridge.sent[0].receipt.status.clone())
    };
    // The host's side: it serves the job the layer admitted.
    if let (true, Status::Running(job)) = &answer {
        let program = layer.take_program_job().expect("the job the run started");
        assert_eq!(program.job, *job);
        let note = RecordValue { payload: Value::String("written by the run".into()), provenance: "run".into() };
        let records = [("note".to_owned(), note)].into_iter().collect();
        let output = JobOutput { value: Value::Null, views: Vec::new(), records };
        layer.finish_program_job_with(*job, Ok(output));
    }
    drop(layer);
    assert_eq!(answer.1.code(), Some(code::NOT_GRANTED), "the bridge answered {answer:?}");

    let written = actions(&disk);
    let reopened = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let reopened = CommandLayer::open(common::kernel(), table(), Secret::fixed([9; 32]), reopened);
    let note = reopened.as_ref().ok().and_then(|layer| layer.record("note")).map(|record| record.payload.canonical());
    assert!(
        written.is_empty() && note.is_none(),
        "the bridge answered {answer:?}; the journal holds {} action record(s): {:?}; after a reopen the record \
         `note` is {note:?}",
        written.len(),
        written.iter().map(Value::canonical).collect::<Vec<_>>()
    );
}
