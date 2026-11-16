//! The parts of CT-EV3-07 and of CT-LB2-03's work-sheet part that M6's session store shows through its public API.
//!
//! - CT-EV3-07 (EV3.2, EV4.15): an explicit workspace command joins the durable action order; a view cannot write
//!   workspace state or make undo stale.
//! - CT-LB2-03, its work-sheet part (LB7.2, KN0.8): reusing a named work sheet invalidates the old preview.
//!
//! Waits for the KN5 split (crate documentation): EV3.2's view rows, which M6 does not have, so a view is stood in for
//! by what it stands on in M6 (a query job, an opened snapshot, a preview and a program's attempts to write); LB7.2's
//! work-sheet structure, with its named roles, their re-resolution and a fresh exact analysis.

mod common;

use c600_kernel::codes::NAMED_OBJECT_INVALID;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::value::Value;
use c600_session::code::{NO_FRESH_PREVIEW, REVISION_CHANGED, UNDO_SUPERSEDED};
use c600_session::{At, JobKind, Origin, Row, Status, WordSource};
use common::*;

fn sheet() -> WordSource {
    WordSource::Record("sheet".into())
}

/// CT-EV3-07, its durable-order part (EV4.15): an explicit workspace command is a durable action in the session's one
/// order of actions. It takes the next order between two commits and supersedes an undo hint taken before it; a new
/// session replays the same order with the same receipts (KN5.23); undo steps back through the three actions in their
/// order, and undoing the workspace action removes the record.
#[test]
fn ct_ev3_07_an_explicit_workspace_command_joins_the_durable_action_order() {
    let (session, disk) = session();
    let first = commit(&session, "c1", &[1]);
    let hint = session.overview().undo.expect("an action to undo");
    let set = set_record(&session, "w1", "sheet", &[1, 2]);
    let stale = shell(&session, "u0", Row::Undo { hint, confirm: false });
    assert_eq!(code(&stale), UNDO_SUPERSEDED, "the workspace action superseded the hint (LI.9)");
    let second = commit(&session, "c2", &[2]);
    assert_eq!((committed(&first).event, action(&set), committed(&second).event), (1, 2, 3));
    assert_eq!(session.overview().order, 3);

    let before = durable(&session);
    drop(session);
    disk.crash();
    let session = reopen(&disk);
    assert_eq!(durable(&session), before);
    for receipt in [&first, &set, &second] {
        assert_eq!(session.outcome(&receipt.command, &receipt.request), *receipt, "{}", receipt.command);
    }

    let mut targets = Vec::new();
    for n in 1..=3 {
        let hint = session.overview().undo.expect("an action to undo");
        targets.push(hint.target);
        let undone = shell(&session, &format!("u{n}"), Row::Undo { hint, confirm: false });
        assert_eq!(committed(&undone).event, 3 + n);
        if hint.target == action(&set) {
            let refused = shell(&session, &format!("p{n}"), Row::Propose { word: sheet() });
            assert_eq!(code(&refused), NAMED_OBJECT_INVALID, "the record is gone");
        }
    }
    assert_eq!(targets, [3, 2, 1]);
    assert_eq!(session.overview().undo, None);
}

/// CT-EV3-07, its view part (EV3.2, LB7.2): a view cannot write workspace state or make undo stale. M6 has no view
/// rows; the test uses what a view stands on in M6. A program's workspace write is refused from the program origin
/// and from a job (ONE.77), and the reads a view makes (a query job, an opened snapshot and a preview) are no durable
/// action: the order, the undo and redo hints, the revision and the journal stay as they were, and the hint taken
/// before them still undoes the latest action.
#[test]
fn ct_ev3_07_a_view_cannot_write_the_workspace_or_make_undo_stale() {
    let (session, disk) = session();
    commit(&session, "c1", &[1]);
    set_record(&session, "w1", "sheet", &[1, 2]);
    let before = session.overview();
    let journal = disk.durable(JOURNAL);
    let hint = before.undo.expect("an action to undo");

    let row = Row::WorkspaceSet { record: "sheet".into(), payload: word_payload(&[2]), provenance: manual() };
    assert_eq!(code(&from_program(&session, "v1", row)), NOT_GRANTED);
    let keep = shell(&session, "v2", Row::JobSubmit(job("keep", vec![Value::word(&[2])], JobKind::Query)));
    assert_eq!(code(&finished(&session, &keep)), NOT_GRANTED);

    let query = shell(&session, "v3", Row::JobSubmit(job("state", Vec::new(), JobKind::Query)));
    assert!(matches!(finished(&session, &query).status, Status::Completed(_)));
    session.open_snapshot(At::Current).expect("a snapshot");
    propose(&session, "v4", sheet());

    let after = session.overview();
    assert_eq!((after.order, after.undo, after.redo), (before.order, before.undo, before.redo));
    assert_eq!(after.revision, before.revision);
    assert_eq!(disk.durable(JOURNAL), journal, "nothing was written");

    let undone = shell(&session, "u1", Row::Undo { hint, confirm: false });
    assert_eq!(committed(&undone).event, before.order + 1);
}

/// CT-LB2-03, its work-sheet part (LB7.2, KN0.8): reusing a named work sheet invalidates the old preview. A preview of
/// the word a record holds stays fresh when another record is set and goes stale when its own record is set again: no
/// token is minted for it, and a token minted before the reuse is refused at execute. A new proposal reads the sheet's
/// new content. A new session holds no preview, and a proposal there reads the stored sheet.
#[test]
fn ct_lb2_03_reusing_a_named_work_sheet_invalidates_the_old_preview() {
    let (session, disk) = session();
    set_record(&session, "w1", "sheet", &[1, 2]);
    let old = propose(&session, "p1", sheet());
    let execute = Row::Execute { preview: old };
    let early = session.shell().mint("x1", &execute).expect("a token for a fresh preview");

    set_record(&session, "w2", "other", &[3]);
    assert!(session.preview().expect("the preview").fresh, "a record the preview did not read");

    set_record(&session, "w3", "sheet", &[2, 1]);
    let info = session.preview().expect("the preview is still pending");
    assert_eq!(info.id, old);
    assert!(!info.fresh);
    assert_eq!(session.shell().mint("x2", &execute).expect_err("a stale preview").code, NO_FRESH_PREVIEW);
    let mut delivery = envelope(&session, "x1", Origin::Shell, execute);
    delivery.token = Some(early);
    assert_eq!(code(&session.invoke(delivery)), REVISION_CHANGED);
    assert_eq!(session.overview().order, 3, "nothing was committed");

    let literal_of = |command: &str, word: WordSource| {
        propose(&session, command, word);
        session.preview().expect("a preview").literal
    };
    let reused = literal_of("p2", sheet());
    assert_eq!(reused, literal_of("p3", literal(&[2, 1])));
    assert_ne!(reused, literal_of("p4", literal(&[1, 2])));

    drop(session);
    disk.crash();
    let session = reopen(&disk);
    assert_eq!(session.preview(), None);
    propose(&session, "p5", sheet());
    assert_eq!(session.preview().expect("a preview").literal, reused);
}
