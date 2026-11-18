//! LB2's exact completion as library code (chapter 03's row "Residuals and exact completion"): the source of
//! `completion.exact_completion` and its catalogue row. CT-LB2-04 (part: the entry's source, its checker summary and
//! its catalogue row; the whole run on the command layer is `c600-tools/tests/exact_completion.rs`). A pass is source
//! evidence.

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::{
    completion_sha256, lower_source, residuals_and_exact_completion, COMPLETION_EXPORTS, COMPLETION_MODULE,
    COMPLETION_ROWS, COMPLETION_SOURCE,
};
use c600_syntax::{format, parse};

/// The exact-completion source parses with no diagnostic, is in the formatter's canonical layout with LF line endings,
/// and lowers to one module, `completion`, that requests no capability and exports `exact_completion` only. The
/// checker reports no diagnostic. The entry's summary has the effect `snapshot` and no capability, effect variable or
/// unresolved call; it reads state through snapshot reads of the rows `positions` and `labels` and no other row, so
/// under capability separation it reads state and prepares no preview. The catalogue row's Effect field is the
/// checker's effects, and its digest is the source's.
#[test]
fn the_exact_completion_source_checks_and_reads_two_rows() {
    let file = parse(COMPLETION_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(COMPLETION_SOURCE), "the source is in the canonical layout");
    assert!(!COMPLETION_SOURCE.contains('\r') && COMPLETION_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(COMPLETION_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, COMPLETION_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, COMPLETION_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let summary = report.summary(COMPLETION_MODULE, "exact_completion").expect("a summary of exact_completion");
    assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]));
    assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
    assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]));
    let rows: BTreeSet<Row> = COMPLETION_ROWS.iter().map(|row| Row::Read(row.to_string())).collect();
    assert_eq!(summary.rows, rows);
    let separated = separation(summary, &Policy::interim());
    assert_eq!(separated, Separation { reads_state: true, prepares_previews: false });

    let row = residuals_and_exact_completion();
    assert_eq!(row.entry, "Residuals and exact completion");
    assert!(row.level.starts_with("Full labels and positions"));
    assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), summary.effects, "Effect: snapshot");
    assert!(row.backing.contains("`completion.exact_completion`"));
    assert!(row.notes.contains("empty candidate lists, matching colours or a buffer"));
    assert_eq!((row.module, row.functions), (COMPLETION_MODULE, &COMPLETION_EXPORTS[..]));
    assert_eq!(row.source_sha256, sha256_hex(COMPLETION_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, completion_sha256());
}
