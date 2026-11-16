//! The six rows of the representations tool at the command layer (LP3 plan, C6 item 7): `rows::NEEDS` against the
//! stored H-08 table, the shipped entries and the layer's refusals. Nothing here builds a row; the layer has the one
//! implementation of tokens, receipts, undo and outcomes, and it refuses all six as not built.

mod common;

use std::collections::BTreeSet;

use c600_check::check;
use c600_command::{code, Kind, Overlay, Table};
use c600_ir::Effect;
use c600_repr::rows::{COMMON, NEEDS};
use c600_repr::tool::{ENTRIES, MODULE, ROWS, SOURCE, UNEXPRESSED};
use c600_tools::tool::lower_source;

use common::*;

/// The requirement list names the tool's six rows, each as H-08 stores it: kind, lane, undo class, preview rule,
/// gates and arguments with their types, none of them a user-action token. A row's entry is one of the shipped entries, bound to that row, and the one
/// row without an entry is the one the tool lists as not expressible. Covers no conformance ID: it keeps the list of
/// item 7 from drifting from the stored table.
#[test]
fn the_row_requirements_name_the_stored_rows_as_h08_has_them() {
    let table = Table::h08();
    assert_eq!(NEEDS.iter().map(|n| n.row).collect::<BTreeSet<_>>(), ROWS.iter().copied().collect::<BTreeSet<_>>());
    assert_eq!(NEEDS.len(), ROWS.len());
    assert!(COMMON.iter().all(|need| !need.is_empty()));
    for need in &NEEDS {
        let row = table.row(need.row).unwrap_or_else(|| panic!("{} is a stored row", need.row));
        assert!(row.overlay.is_none(), "{}", need.row);
        assert_eq!((row.kind, row.lane, row.undo, row.preview), (need.kind, need.lane, need.undo, need.preview));
        assert_eq!(row.gates, need.gates, "{}", need.row);
        let stored: Vec<(&str, &str)> = row.arguments.iter().map(|(name, ty)| (name.as_str(), ty.as_str())).collect();
        assert_eq!(stored, need.arguments, "{}", need.row);
        assert!(!row.takes_token(), "{}: H-08 gives none of the six a user-action token", need.row);
        for text in [need.receipt, need.undo_entry, need.layer] {
            assert!(!text.is_empty(), "{}", need.row);
        }
        match need.entry {
            Some(name) => {
                let entry = ENTRIES.iter().find(|e| e.name == name).unwrap_or_else(|| panic!("{name} is shipped"));
                assert_eq!(entry.row, need.row);
            }
            None => assert!(UNEXPRESSED.iter().any(|(row, _)| *row == need.row), "{}", need.row),
        }
    }
}

/// The effect class each requirement names is the one the checker infers for the row's entry (LG3.1, LG3.2), so the
/// list and the shipped source cannot disagree. A row without an entry has only its declared class. Covers no
/// conformance ID.
#[test]
fn the_effect_class_of_each_row_is_the_one_its_entry_has() {
    let program = lower_source(SOURCE).expect("the shipped source lowers");
    let report = check(&program).expect("checked");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>());
    for need in &NEEDS {
        let declared: BTreeSet<Effect> = need.effects.iter().copied().collect();
        assert_eq!(declared.len(), need.effects.len(), "{}", need.row);
        assert!(!declared.contains(&Effect::PreparePreview), "{}: no row of the tool prepares a preview", need.row);
        assert_eq!(
            declared.contains(&Effect::ChangeWorkspace),
            need.kind == Kind::Workspace,
            "{}: a workspace effect exactly on a workspace row",
            need.row
        );
        if let Some(entry) = need.entry {
            let summary = report.summary(MODULE, entry).expect("a summary");
            assert_eq!(summary.effects, declared, "{}", need.row);
        }
    }
}

/// The command layer builds none of the six rows and nothing else may stand in for one. Its availability answer and
/// its receipt for an invocation, from the shell and from a running program, carry the layer's not-built code; an
/// overlay under a stored row's ID is refused as a conflict; and none of it moves the revision, the durable log or
/// the undo target of a committed action. Covers no conformance ID in full: it is the evidence for item 7 that the
/// rows are listed, not built.
#[test]
fn the_layer_refuses_all_six_rows_and_an_overlay_cannot_replace_one() {
    let mut session = session();
    session.commit(&[1]);
    let before = session.fixed();
    for (i, need) in NEEDS.iter().enumerate() {
        let unavailable = session.layer.availability(need.row).expect_err("not available");
        assert_eq!(unavailable.code, code::ROW_NOT_BUILT, "{}", need.row);

        let args = placeholder_args(session.layer.table(), need.row);
        let refused = match session.shell.invoke(&mut session.layer, need.row, args) {
            Ok(receipt) => receipt.status.code(),
            Err(error) => Some(error.code),
        };
        assert_eq!(refused, Some(code::ROW_NOT_BUILT), "{} from the shell", need.row);

        let receipt = session.layer.invoke(session.runtime_call(&format!("rt/{i}"), need.row));
        assert_eq!(receipt.status.code(), Some(code::ROW_NOT_BUILT), "{} from a program", need.row);

        let overlay = Overlay { module: MODULE.to_owned(), entry: need.entry.unwrap_or("filter_counts").to_owned() };
        let conflict = session.layer.register_overlay(need.row, need.kind, &[], overlay).expect_err("a stored ID");
        assert_eq!(conflict.code, code::OVERLAY_ROW_CONFLICT, "{}", need.row);

        assert!(session.layer.take_program_job().is_none(), "{} started no program", need.row);
        assert_eq!(session.fixed(), before, "{}", need.row);
    }
    assert_eq!(session.layer.table().row("filter.apply").map(|row| row.overlay.is_none()), Some(true));
}
