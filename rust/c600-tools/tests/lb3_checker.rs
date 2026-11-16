//! The checker half of CT-LB3-07, as far as `c600-check`'s public API decides it on 600 Cell Script source (LP3,
//! C5's next piece, the last item). The fixtures are programs of this test, not tools of the crate. The other half
//! (the facts themselves, per revision and per named cap) waits for M5's jumbling engine (C8).
//!
//! CT-LB3-01 has no checker half today: the checker and the IR know no end test, so no fixture can tell a retained
//! checkpoint, a label checkpoint and a sticker test apart, and it is not built here.

use std::collections::BTreeSet;

use c600_check::{
    admit, admit_built_in, check, separation, Grant, Policy, Profile, Row, Separation, StateRead, NOT_GRANTED,
};
use c600_tools::tool::lower_source;

const MODULE: &str = "jumble_inspector";

/// Four inspector entries. `outcome_table` previews two words and returns both outcomes; `check_named_word` previews
/// the solver's word and answers the fixed acknowledgement's two fields; `check_and_skip` also previews a word of
/// its own; `recommend` returns a word it read from state.
const INSPECTOR: &str = "edition \"6cs-draft-1\";\nmodule jumble_inspector requires [];\n\n\
export command outcome_table(a: Word, b: Word) -> record { first: Nat, second: Nat } effects [snapshot, preview] {\n    \
let ra = propose(a, target(\"operation.preview\"));\n    \
let rb = propose(b, target(\"operation.preview\"));\n    \
return record { first = receipt_n(ra), second = receipt_n(rb) };\n}\n\n\
export command check_named_word(w: Word) -> record { base: Text, id: Text } effects [preview] {\n    \
let r = propose(w, target(\"operation.preview\"));\n    \
return record { base = receipt_base(r), id = receipt_id(r) };\n}\n\n\
export command check_and_skip(w: Word) -> record { base: Text, id: Text } effects [preview] {\n    \
let r = propose(w, target(\"operation.preview\"));\n    \
let next = propose(word[+3], target(\"operation.preview\"));\n    \
return record { base = receipt_base(next), id = receipt_id(next) };\n}\n\n\
export fn recommend() -> Word effects [snapshot] {\n    \
return read_snapshot<Word>(\"lowering_word\");\n}\n";

/// CT-LB3-07, the checker half (LB3.5: no outcome table over items, no selection by the method; LG3.13 and BND.42:
/// a preview's outcome is state-derived data). All four entries are well typed, so every refusal below is the
/// admission's, with `LG3:capability.not_granted`.
///
/// - An entry that returns a table of preview outcomes reads state and prepares previews: its summary holds the
///   outcome read, capability separation reports both, and both the solving column and the built-in column refuse it.
/// - A named-word check that answers only its word, the fixed acknowledgement's two fields, prepares a preview and
///   reads nothing; the built-in column admits it with no open cell.
/// - A named-word check that also previews a word the solver did not name (a next-cap skip or a correction
///   combination would be one) is refused by the built-in column at that proposal.
///
/// Not decided by the checker, so not built: a recommendation. `recommend` returns a word read from state through an
/// entry that prepares no preview; the solving column leaves it to the open cell BND.59 and the built-in column
/// admits it when the class lists its read. The checker has no rule that tells a fact about the solver's named object
/// from a selection the method made, and the IR draft has no list type, so an all-items table can only be written as a
/// fixed record as above. The fixture's class lists every row its entries use (preview, outcome read, snapshot read),
/// so each refusal shown is the rule under test and not a missing row (BND.57).
#[test]
fn ct_lb3_07_checker_half_outcome_tables_and_named_word_checks() {
    let program = lower_source(INSPECTOR).expect("the fixture lowers");
    let report = check(&program).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let (none, interim) = (Grant::none(), Policy::interim());
    let facts = |entry: &str| separation(report.summary(MODULE, entry).expect("the entry's summary"), &interim);
    let solving = |entry: &str| admit(&program, MODULE, entry, Profile::Solving, &none, &interim).unwrap();
    let class =
        BTreeSet::from([Row::Preview("operation.preview".into()), Row::Outcome, Row::Read("lowering_word".into())]);
    let built_in = |entry: &str| admit_built_in(&program, MODULE, entry, &class, &none, &interim).unwrap();
    let both = Separation { reads_state: true, prepares_previews: true };
    let previews = Separation { reads_state: false, prepares_previews: true };
    let reads = Separation { reads_state: true, prepares_previews: false };

    let table = report.summary(MODULE, "outcome_table").unwrap();
    assert_eq!(table.state_reads, BTreeSet::from([StateRead::Outcome]));
    assert_eq!(facts("outcome_table"), both);
    assert_eq!((solving("outcome_table").codes(), solving("outcome_table").separation), (vec![NOT_GRANTED], both));
    let refused = built_in("outcome_table");
    assert_eq!(refused.separation, both);
    assert!(!refused.is_admitted() && refused.codes().iter().all(|&code| code == NOT_GRANTED));
    assert!(refused.diagnostics.iter().any(|diagnostic| diagnostic.path == "entry"), "{:#?}", refused.diagnostics);

    assert!(report.summary(MODULE, "check_named_word").unwrap().state_reads.is_empty());
    assert_eq!(facts("check_named_word"), previews);
    let admitted = built_in("check_named_word");
    assert!(admitted.is_admitted() && admitted.open_cells.is_empty(), "{:#?}", admitted.diagnostics);

    assert_eq!(facts("check_and_skip"), previews);
    let skipped = built_in("check_and_skip");
    assert!(!skipped.is_admitted() && skipped.codes().iter().all(|&code| code == NOT_GRANTED));
    assert!(skipped.diagnostics.iter().all(|diagnostic| diagnostic.path != "entry"), "{:#?}", skipped.diagnostics);

    // The part the checker does not decide: a recommendation through a reading entry.
    assert_eq!(facts("recommend"), reads);
    let open = solving("recommend");
    assert_eq!((open.codes(), open.open_cells), (Vec::<&str>::new(), vec!["BND.59"]));
    assert!(built_in("recommend").is_admitted());
}
