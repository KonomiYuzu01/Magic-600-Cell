//! LB2's "Net and Strict protection" as library code (chapter 03's row of that name): the source of the module
//! `protection` and its catalogue row. CT-LB2-02 (part: the entries' source, their checker summaries, their catalogue
//! row and the report's coverage rule on the row's answers without a host; the whole, through the command layer, is
//! `c600-tools/tests/ct_lb2_02.rs`). A pass is source evidence.

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::{
    lower_source, net_and_strict_protection, protection_sha256, PROTECTION_CHECKS, PROTECTION_EXPORTS,
    PROTECTION_LOCKS, PROTECTION_MODULE, PROTECTION_ROWS, PROTECTION_SOURCE,
};
use c600_syntax::{format, parse};

const INCOMPLETE: &str = "LB2:protection.incomplete_coverage";

/// The source parses with no diagnostic, is in the formatter's canonical layout with LF line endings, and lowers to one
/// module, `protection`, that requests no capability and exports the six entries. The checker reports no diagnostic.
/// Every entry has the effect `snapshot`: `position_lock` reads `labels`, `identity_lock` and `block_lock` read
/// `positions`, and the three checks read `protection`, so under capability separation each reads state and prepares no
/// preview. The catalogue row lists all six, its Effect field is the checker's effects of its functions together, its
/// notes carry LB2.3's error code, and its digest is the source's.
#[test]
fn the_protection_source_checks_and_reads_its_rows() {
    let file = parse(PROTECTION_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(PROTECTION_SOURCE), "the source is in the canonical layout");
    assert!(!PROTECTION_SOURCE.contains('\r') && PROTECTION_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(PROTECTION_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, PROTECTION_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, PROTECTION_EXPORTS);
    let together: BTreeSet<&str> = PROTECTION_LOCKS.iter().chain(&PROTECTION_CHECKS).copied().collect();
    assert_eq!(together, PROTECTION_EXPORTS.into_iter().collect(), "locks and checks split the exports");
    assert_eq!(PROTECTION_LOCKS.len() + PROTECTION_CHECKS.len(), PROTECTION_EXPORTS.len());

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let mut read = BTreeSet::new();
    for entry in PROTECTION_EXPORTS {
        let summary = report.summary(PROTECTION_MODULE, entry).expect("a summary");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{entry}");
        assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{entry}");
        let own = match entry {
            "position_lock" => "labels",
            "identity_lock" | "block_lock" => "positions",
            _ => "protection",
        };
        assert_eq!(summary.rows, BTreeSet::from([Row::Read(own.into())]), "{entry}");
        read.insert(own);
        let separated = separation(summary, &Policy::interim());
        assert_eq!(separated, Separation { reads_state: true, prepares_previews: false }, "{entry}");
    }
    assert_eq!(read, PROTECTION_ROWS.into_iter().collect(), "the rows the entries read");

    let row = net_and_strict_protection();
    assert_eq!(row.entry, "Net and Strict protection");
    assert_eq!((row.module, row.functions), (PROTECTION_MODULE, &PROTECTION_EXPORTS[..]));
    assert_eq!(row.effects, [Effect::ReadSnapshot]);
    assert!(PROTECTION_EXPORTS.iter().all(|f| row.backing.contains(&format!("`protection.{f}`"))), "Backing");
    assert!(row.notes.contains(INCOMPLETE) && row.notes.contains("never a safe verdict"));
    assert_eq!(row.source_sha256, sha256_hex(PROTECTION_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, protection_sha256());
}
