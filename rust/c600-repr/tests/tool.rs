//! The representations tool: its 600 Cell Script source, its manifest and class, the review of the class against the
//! exact entry definitions and the row set, and the form of a review record (LP3, C6, item 5). The tests named after
//! CT-BND-15 and CT-LB6-03 cover the representations tool's part of each; neither ID runs whole here.

use std::collections::BTreeSet;

use c600_check::{
    admit_built_in, check, separation, Grant, Policy, Row as GrantRow, Separation, EFFECT_UNDECLARED, NOT_GRANTED,
};
use c600_command::{Kind, Table};
use c600_digest::sha256_hex;
use c600_ir::{Effect, FnKind, Program};
use c600_repr::tool::{
    admit, check_record, class_rows, entry_digest, manifest, review_class, review_source, row_within_view_aid, Entry,
    Finding, Manifest, RecordFault, ReviewRecord, Rule, ENTRIES, MODULE, ROWS, SOURCE, UNEXPRESSED,
};
use c600_syntax::{format, lower, parse};
use c600_tools::tool::{lower_source, targets, BoundaryClass};

fn program() -> Program {
    lower_source(SOURCE).expect("the shipped source lowers")
}

/// The shipped module with more functions after it, and its manifest with more entries.
fn variant(tail: &str, extra: &[Entry]) -> (Program, Manifest) {
    let source = format!("{SOURCE}\n{tail}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    let program = lower_source(&source).expect("the variant lowers");
    let mut entries = ENTRIES.to_vec();
    entries.extend_from_slice(extra);
    (program, Manifest { entries, source_sha256: sha256_hex(source.as_bytes()), ..manifest() })
}

/// The rules of the findings about `entry`, in the review's order.
fn rules(findings: &[Finding], entry: &str) -> Vec<Rule> {
    findings.iter().filter(|f| f.entry.as_deref() == Some(entry)).map(|f| f.rule).collect()
}

/// The tool's source parses with no diagnostic, is in the formatter's canonical layout (edition `6cs-draft-1`, syntax
/// candidate A) and lowers to the IR draft: one module that requests no capability, with the six exported entries of
/// the manifest. The built-in column admits every entry with no capability granted; no entry prepares a preview or
/// names a proposal target, and the entries' effects are the ones their rows need. Every row has an entry, `set.save`
/// included, since the IR draft gained lists.
#[test]
fn the_source_parses_lowers_and_every_entry_is_admitted_under_the_built_in_column() {
    let file = parse(SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(file.edition.as_deref(), Some("6cs-draft-1"));
    assert_eq!(format(&file).as_deref(), Some(SOURCE), "the source is in canonical layout");
    assert!(!SOURCE.contains('\r'), "LF line endings");
    let lowered = lower(&[file]).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(lowered.source(MODULE).expect("a source map").digest, manifest().source_sha256);
    let [module] = &lowered.program.modules[..] else { panic!("one module") };
    assert_eq!(module.name, MODULE);
    assert!(module.capabilities.is_empty());
    let defined: Vec<(&str, FnKind, bool)> =
        module.functions.iter().map(|f| (f.name.as_str(), f.kind, f.exported)).collect();
    let declared: Vec<(&str, FnKind, bool)> = ENTRIES.iter().map(|e| (e.name, e.kind, true)).collect();
    assert_eq!(defined, declared);

    let program = program();
    let report = check(&program).expect("the IR version is supported");
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let reads = |entry: &str| {
        let summary = report.summary(MODULE, entry).expect("a summary");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert!(!summary.effects.contains(&Effect::PreparePreview), "{entry}");
        separation(summary, &Policy::interim())
    };
    for entry in &ENTRIES {
        let admission = admit(&program, MODULE, entry.name).expect("the IR version is supported");
        assert!(admission.is_admitted(), "{}: {:#?}", entry.name, admission.diagnostics);
        assert!(admission.open_cells.is_empty(), "{}", entry.name);
        assert_eq!(admission.separation, reads(entry.name));
        assert!(!admission.separation.prepares_previews, "{}", entry.name);
        assert_eq!(targets(&program, MODULE, entry.name), Ok(BTreeSet::new()), "{}: no proposal target", entry.name);
    }
    // Capability separation (LQ21): the two entries that read state prepare nothing, and the others read nothing.
    let reading: Vec<&str> = ENTRIES.iter().map(|e| e.name).filter(|name| reads(name).reads_state).collect();
    assert_eq!(reading, ["filter_counts", "compare_cells"]);

    // Every row has an entry or a stated reason for none, and since the IR draft gained lists no row lacks an entry.
    let served: BTreeSet<&str> = ENTRIES.iter().map(|e| e.row).chain(UNEXPRESSED.iter().map(|(row, _)| *row)).collect();
    assert_eq!(served, ROWS.iter().copied().collect::<BTreeSet<_>>());
    assert_eq!((ENTRIES.len(), UNEXPRESSED.len()), (ROWS.len(), 0));
}

/// CT-BND-15 (part, the representations tool: the class is declared, the row set lies within the class, and the form
/// of a review record names every entry digest). The manifest declares the one class `view_aid`, one of BND.27's. Each
/// of the six rows is a query, view or workspace row of the stored H-08 table that takes no user-action token and
/// needs no commit preview, and the mechanical review of the class finds nothing outside it; a commit row, a row the
/// table does not have, an entry on a row outside the row set and another class are each found. A record names the
/// tool, the class, the source digest, every entry's digest and the row set, and a record for other source bytes is
/// refused, and so is a record whose reviewer is blank. The review of source bytes passes only the bytes the manifest
/// names: other bytes that lower to an equal program are found, and so are bytes that do not lower. Not run: BM5's
/// review has not happened, so no review record of the shipped tool exists; the record
/// checked here carries a fixture reviewer and is no review. The row predicate reads a row's kind, token and preview
/// rule only: that a query row prepares a commit preview (`operation.preview`) is found by the entry review, not by
/// the row set.
#[test]
fn ct_bnd_15_representations_part_class_declared_rows_within_class_and_record_names_entry_digests() {
    let manifest = manifest();
    assert_eq!(manifest.class, BoundaryClass::ViewAid);
    assert!(BoundaryClass::ALL.contains(&manifest.class));
    assert_eq!(manifest.class.name(), "view_aid");
    assert_eq!((manifest.module, &manifest.rows[..]), (MODULE, &ROWS[..]));
    assert_eq!(manifest.source_sha256, sha256_hex(SOURCE.as_bytes()));
    assert!(!manifest.principle_record, "no PrincipleRecord exists (LB6.5)");

    let table = Table::h08();
    for row in &manifest.rows {
        let stored = table.row(row).unwrap_or_else(|| panic!("{row} is a stored row"));
        assert!(row_within_view_aid(stored), "{row}");
        assert!(matches!(stored.kind, Kind::Query | Kind::View | Kind::Workspace), "{row}");
        assert!(!stored.takes_token(), "{row}");
    }
    assert!(manifest.entries.iter().all(|e| manifest.rows.contains(&e.row)));
    let program = program();
    assert_eq!(review_class(&program, &manifest, &table).expect("reviewed"), Vec::new());

    // The review of source bytes: only the bytes the manifest names, and they must lower.
    let source_rules = |source: &str| {
        review_source(source, &manifest, &table).expect("reviewed").iter().map(|f| f.rule).collect::<Vec<_>>()
    };
    assert_eq!(source_rules(SOURCE), []);
    assert_eq!(source_rules(&format!("{SOURCE}\n")), [Rule::SourceDigest], "an equal program from other bytes");
    assert_eq!(source_rules("module"), [Rule::SourceDigest, Rule::SourceUnlowered]);

    // What the review finds when the row set leaves the class.
    for commit in ["operation.execute", "session.undo", "twist.apply"] {
        let mut wider = manifest.clone();
        wider.rows.push(commit);
        let findings = review_class(&program, &wider, &table).expect("reviewed");
        let [finding] = &findings[..] else { panic!("{findings:?}") };
        assert_eq!((finding.rule, finding.row.as_deref()), (Rule::RowOutsideClass, Some(commit)));
    }
    let mut unknown = manifest.clone();
    unknown.rows.push("filter.no_such_row");
    let findings = review_class(&program, &unknown, &table).expect("reviewed");
    assert_eq!(findings.iter().map(|f| f.rule).collect::<Vec<_>>(), [Rule::RowOutsideTool]);
    let mut narrower = manifest.clone();
    narrower.rows.retain(|row| *row != "filter.apply");
    let findings = review_class(&program, &narrower, &table).expect("reviewed");
    assert_eq!(rules(&findings, "apply_filter"), [Rule::RowOutsideTool]);
    let other_class = Manifest { class: BoundaryClass::Analysis, ..manifest.clone() };
    let findings = review_class(&program, &other_class, &table).expect("reviewed");
    assert_eq!(findings.iter().map(|f| f.rule).collect::<Vec<_>>(), [Rule::ClassNotReviewed]);

    // The record's form: it names every entry digest, and an unfilled form is no record.
    let form = ReviewRecord::form(&manifest);
    assert_eq!(check_record(&form, &manifest), [RecordFault::NoReviewer]);
    assert_eq!(form.entries.keys().map(String::as_str).collect::<BTreeSet<_>>(), ENTRIES.map(|e| e.name).into());
    for (entry, digest) in &form.entries {
        assert_eq!(digest, &entry_digest(&manifest.source_sha256, MODULE, entry));
        assert!(digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()), "{digest}");
    }
    assert_eq!(form.entries.values().collect::<BTreeSet<_>>().len(), ENTRIES.len(), "one digest per entry");
    let filled = ReviewRecord { reviewer: "fixture reviewer; no BM5 review exists".to_owned(), ..form.clone() };
    assert_eq!(check_record(&filled, &manifest), Vec::new());
    let blank = ReviewRecord { reviewer: " \t ".to_owned(), ..form.clone() };
    assert_eq!(check_record(&blank, &manifest), [RecordFault::NoReviewer], "a blank name is no reviewer");

    // A record names the exact source: for other source bytes every entry digest differs.
    let edited = format!("{SOURCE}\n");
    let other = Manifest { source_sha256: sha256_hex(edited.as_bytes()), ..manifest.clone() };
    let faults = check_record(&filled, &other);
    assert!(faults.contains(&RecordFault::Source), "{faults:?}");
    for entry in &ENTRIES {
        assert!(faults.contains(&RecordFault::EntryDigest(entry.name.to_owned())), "{faults:?}");
    }
    // A record that leaves an entry out, names one the tool does not have, or names other rows or another class.
    let mut partial = filled.clone();
    partial.entries.remove("apply_filter");
    assert_eq!(check_record(&partial, &manifest), [RecordFault::EntryNotNamed("apply_filter".to_owned())]);
    let mut extra = filled.clone();
    extra.entries.insert("stash".to_owned(), entry_digest(&manifest.source_sha256, MODULE, "stash"));
    assert_eq!(check_record(&extra, &manifest), [RecordFault::EntryUnknown("stash".to_owned())]);
    let mut rows = filled.clone();
    rows.rows.insert("operation.execute".to_owned());
    assert_eq!(check_record(&rows, &manifest), [RecordFault::Rows]);
    let class = ReviewRecord { class: "analysis".to_owned(), ..filled };
    assert_eq!(check_record(&class, &manifest), [RecordFault::Class]);
}

/// CT-LB6-03 (part, the representations tool: the declared class is reviewed against the exact entry definitions, and
/// LX-3's storage-laundering and state-based target selection cases are refused under capability separation and at the
/// preview sink; not run: BM5's review and its record, when the tools ship). The shipped entries pass the review. A
/// variant that stores a word read from state is refused by the built-in column at the write, because a stored word too
/// must be one the solver named (LP2's column), though the workspace sink refuses nothing on labels (BND.26); the class
/// review finds it as well: a view aid stores what the solver named. The variants that preview the stored word, read
/// directly or through a declassifier, are refused with `LG3:capability.not_granted` at the entry and at the proposal,
/// and with the declassifier enabled still at the proposal; a variant that reads its target from state is refused at
/// the entry twice, for the separation and for a row the checker cannot name. The column's checks here use a class that
/// also lists the variants' rows, so that each refusal shown is the rule under test; under the tool's own class
/// ([`class_rows`]) the store's rows are refused too. The review finds each as choosing a target. An exported function
/// the manifest does not list is found, so no entry escapes the review by being left out of the manifest. Not run:
/// LX-3's correction solve, which is the 600-cell-Full tool's case, and any run of a variant; the shipped entries run
/// through the command and source routes in `served_rows.rs`.
#[test]
fn ct_lb6_03_representations_part_class_reviewed_against_entries_and_storage_laundering_refused() {
    let table = Table::h08();
    let shipped = program();
    assert_eq!(review_class(&shipped, &manifest(), &table).expect("reviewed"), Vec::new());
    let report = check(&shipped).expect("checked");
    for entry in &ENTRIES {
        let summary = report.summary(MODULE, entry.name).expect("a summary");
        if summary.effects.contains(&Effect::ChangeWorkspace) {
            assert!(summary.state_reads.is_empty(), "{} stores only what it was given", entry.name);
            assert_eq!((entry.kind, table.row(entry.row).map(|r| r.kind)), (FnKind::Command, Some(Kind::Workspace)));
        }
    }

    let tail = "export command stash() -> Unit effects [snapshot, workspace] {\n    \
                return write_workspace<Word>(\"kept\", read_snapshot<Word>(\"best_word\"));\n}\n\n\
                export command relay() -> Text effects [snapshot, preview] {\n    \
                let r = propose(read_snapshot<Word>(\"kept\"), target(\"operation.preview\"));\n    \
                return receipt_id(r);\n}\n\n\
                export command relay_declassified() -> Text effects [snapshot, preview] {\n    \
                let r = propose(declassified<Word>(\"kept\"), target(\"operation.preview\"));\n    \
                return receipt_id(r);\n}\n\n\
                export command chooses(w: Word) -> Text effects [snapshot, preview] {\n    \
                let r = propose(w, read_snapshot<Target>(\"structure.compare\"));\n    \
                return receipt_id(r);\n}\n";
    let extra = [
        Entry { name: "stash", row: "filter.apply", kind: FnKind::Command },
        Entry { name: "relay", row: "filter.preview", kind: FnKind::Command },
        Entry { name: "relay_declassified", row: "filter.preview", kind: FnKind::Command },
        Entry { name: "chooses", row: "structure.compare", kind: FnKind::Command },
    ];
    let (program, wider) = variant(tail, &extra);
    assert!(check(&program).expect("checked").is_admitted(), "each variant is a well-typed program");

    // The class with the variants' rows added.
    let rows = BTreeSet::from([
        GrantRow::Read("best_word".into()),
        GrantRow::Read("kept".into()),
        GrantRow::Workspace("kept".into()),
        GrantRow::Declassified("kept".into()),
        GrantRow::Preview("operation.preview".into()),
    ]);
    let rows: BTreeSet<GrantRow> = class_rows().union(&rows).cloned().collect();

    // The store: refused by the column at the write (the body's root), found by the class review. Under the tool's own
    // class its snapshot read and its workspace record are rows the class does not list, two more refusals at `entry`.
    let stash = admit_built_in(&program, MODULE, "stash", &rows, &Grant::none(), &Policy::interim()).expect("admitted");
    let found: Vec<(&str, &str)> = stash.diagnostics.iter().map(|d| (d.path.as_str(), d.code)).collect();
    assert_eq!(found, [("root", NOT_GRANTED)], "{:#?}", stash.diagnostics);
    assert_eq!(stash.separation, Separation { reads_state: true, prepares_previews: false });
    let own_class = admit(&program, MODULE, "stash").expect("admitted or refused");
    let found: Vec<&str> = own_class.diagnostics.iter().map(|d| d.path.as_str()).collect();
    assert_eq!((own_class.codes(), found), (vec![NOT_GRANTED; 3], vec!["entry", "entry", "root"]));

    // The uses: refused under the selected mechanism.
    let both = Separation { reads_state: true, prepares_previews: true };
    let previews_only = Separation { reads_state: false, prepares_previews: true };
    for (entry, policy, separation, paths) in [
        ("relay", Policy::interim(), both, vec!["entry", "0"]),
        ("relay_declassified", Policy::interim(), both, vec!["entry", "0"]),
        ("relay_declassified", Policy::enabling(&["kept"]), previews_only, vec!["0"]),
        ("chooses", Policy::interim(), both, vec!["entry", "entry"]),
    ] {
        let admission = admit_built_in(&program, MODULE, entry, &rows, &Grant::none(), &policy).expect("admitted");
        let found: Vec<&str> = admission.diagnostics.iter().map(|d| d.path.as_str()).collect();
        assert_eq!((admission.separation, found), (separation, paths), "{entry}");
        assert!(admission.codes().iter().all(|&code| code == NOT_GRANTED), "{entry}: {:#?}", admission.diagnostics);
    }

    // The review of the declared class against these entry definitions.
    let findings = review_class(&program, &wider, &table).expect("reviewed");
    assert_eq!(rules(&findings, "stash"), [Rule::ColumnRefusal, Rule::StoresStateDerived]);
    for entry in ["relay", "relay_declassified", "chooses"] {
        assert_eq!(rules(&findings, entry), [Rule::ColumnRefusal, Rule::ChoosesTarget], "{entry}");
    }
    for entry in &ENTRIES {
        assert_eq!(rules(&findings, entry.name), [], "{} is as it was", entry.name);
    }
    // The manifest of the wider module is not the shipped one: its entry digests differ, so no record of the shipped
    // tool covers it.
    let record = ReviewRecord { reviewer: "fixture reviewer".to_owned(), ..ReviewRecord::form(&manifest()) };
    assert!(check_record(&record, &wider).contains(&RecordFault::Source));

    // A view that writes a workspace record is no well-typed program, and the review reports the checker's finding.
    let writes = "export view writes() -> Unit effects [snapshot, view, workspace] {\n    \
                  return write_workspace<Nat>(\"filter.apply\", 1);\n}\n";
    let (program, wider) = variant(writes, &[Entry { name: "writes", row: "filter.apply", kind: FnKind::View }]);
    let report = check(&program).expect("checked");
    assert_eq!(report.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>(), [EFFECT_UNDECLARED]);
    let findings = review_class(&program, &wider, &table).expect("reviewed");
    assert_eq!(rules(&findings, "writes"), [Rule::CheckerDiagnostic, Rule::WorkspaceOutsideRow]);

    // The same variants left out of the manifest: each export is found as unlisted, and nothing else is.
    let (program, unlisted) = variant(tail, &[]);
    let findings = review_class(&program, &unlisted, &table).expect("reviewed");
    let found: BTreeSet<(Rule, Option<&str>)> = findings.iter().map(|f| (f.rule, f.entry.as_deref())).collect();
    let names = ["stash", "relay", "relay_declassified", "chooses"];
    let expected: BTreeSet<(Rule, Option<&str>)> = names.map(|name| (Rule::EntryUnlisted, Some(name))).into();
    assert_eq!((findings.len(), found), (4, expected));
}
