//! LB2's "Named macro analysis and comparison" as library code (chapter 03's row of that name): the source of the
//! module `macros` and its catalogue row, without a host. The entries run whole, through the command layer, in
//! `c600-tools/tests/macros.rs`. A pass is source evidence.

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::{
    lower_source, macros_sha256, named_macro_analysis_and_comparison, MACROS_EXPORTS, MACROS_MODULE, MACROS_ROWS,
    MACROS_SOURCE,
};
use c600_syntax::{format, parse};

/// The source parses with no diagnostic, is in the formatter's canonical layout with LF line endings, and lowers to one
/// module, `macros`, that requests no capability and exports the two entries. The checker reports no diagnostic.
/// Both entries have the effect `snapshot`: `macro_report` reads `effect` and `replay`, and `macro_comparison` reads
/// `effect`, so under capability separation each reads state and prepares no preview. The catalogue row lists both,
/// its Effect field is the checker's effects of its functions together, its notes say that it chooses no macro, and
/// its digest is the source's.
#[test]
fn the_macros_source_checks_and_reads_its_rows() {
    let file = parse(MACROS_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(MACROS_SOURCE), "the source is in the canonical layout");
    assert!(!MACROS_SOURCE.contains('\r') && MACROS_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(MACROS_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, MACROS_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, MACROS_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let mut read = BTreeSet::new();
    for entry in MACROS_EXPORTS {
        let summary = report.summary(MACROS_MODULE, entry).expect("a summary");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{entry}");
        assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{entry}");
        let own: &[&str] = match entry {
            "macro_report" => &["effect", "replay"],
            _ => &["effect"],
        };
        let rows: BTreeSet<Row> = own.iter().map(|row| Row::Read(row.to_string())).collect();
        assert_eq!(summary.rows, rows, "{entry}");
        read.extend(own.iter().copied());
        let separated = separation(summary, &Policy::interim());
        assert_eq!(separated, Separation { reads_state: true, prepares_previews: false }, "{entry}");
    }
    assert_eq!(read, MACROS_ROWS.into_iter().collect(), "the rows the entries read");

    let row = named_macro_analysis_and_comparison();
    assert_eq!(row.entry, "Named macro analysis and comparison");
    assert_eq!((row.module, row.functions), (MACROS_MODULE, &MACROS_EXPORTS[..]));
    assert_eq!(row.effects, [Effect::ReadSnapshot]);
    assert!(MACROS_EXPORTS.iter().all(|f| row.backing.contains(&format!("`macros.{f}`"))), "Backing");
    assert!(MACROS_ROWS.iter().all(|r| row.backing.contains(&format!("`{r}`"))), "Backing names the rows");
    assert!(row.notes.contains("without choosing one") && row.notes.contains("Nothing is ranked, selected or applied"));
    assert_eq!(row.source_sha256, sha256_hex(MACROS_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, macros_sha256());
}
