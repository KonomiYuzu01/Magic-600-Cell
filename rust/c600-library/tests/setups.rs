//! LB2's "Guarded setups and buffers" as library code (chapter 03's row of that name): the source of the module
//! `guarded_setups` and its catalogue row. CT-BND-18 (part: the entry's source, its checker summary and its catalogue
//! row; the whole, through the command layer and the tool's host bridge, is `c600-tools/tests/ct_bnd_18.rs`). A pass
//! is source evidence.

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::{
    guarded_setups_and_buffers, lower_source, setups_sha256, SETUPS_EXPORTS, SETUPS_MODULE, SETUPS_ROWS, SETUPS_SOURCE,
};
use c600_syntax::{format, parse};

/// The source parses with no diagnostic, is in the formatter's canonical layout with LF line endings, and lowers to one
/// module, `guarded_setups`, that requests no capability and exports `setups_for`. The checker reports no diagnostic.
/// The entry has the effect `snapshot` and reads `setups`, so under capability separation it reads state and prepares
/// no preview. The catalogue row lists the entry, its Effect field is the checker's effects, its notes say that the
/// solver names the target and chooses among the setups, and its digest is the source's.
#[test]
fn the_setups_source_checks_and_reads_its_row() {
    let file = parse(SETUPS_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(SETUPS_SOURCE), "the source is in the canonical layout");
    assert!(!SETUPS_SOURCE.contains('\r') && SETUPS_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(SETUPS_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, SETUPS_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, SETUPS_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    for entry in SETUPS_EXPORTS {
        let summary = report.summary(SETUPS_MODULE, entry).expect("a summary");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{entry}");
        assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{entry}");
        let rows: BTreeSet<Row> = SETUPS_ROWS.iter().map(|row| Row::Read(row.to_string())).collect();
        assert_eq!(summary.rows, rows, "{entry}");
        let separated = separation(summary, &Policy::interim());
        assert_eq!(separated, Separation { reads_state: true, prepares_previews: false }, "{entry}");
    }

    let row = guarded_setups_and_buffers();
    assert_eq!(row.entry, "Guarded setups and buffers");
    assert_eq!((row.module, row.functions), (SETUPS_MODULE, &SETUPS_EXPORTS[..]));
    assert_eq!(row.effects, [Effect::ReadSnapshot]);
    assert!(row.backing.contains("`guarded_setups.setups_for`") && row.backing.contains("`setups`"), "Backing");
    assert!(row.notes.contains("the solver names") && row.notes.contains("no rank"), "Notes");
    assert_eq!(row.source_sha256, sha256_hex(SETUPS_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, setups_sha256());
}
