//! The jumbling tool's source, its entries, its declared class and its admission under the built-in column (LP3, C8,
//! item 1). Each test names the conformance part it covers; none of those IDs runs whole here.

use std::collections::BTreeSet;

use c600_check::{admit, check, separation, Grant, Policy, Profile, Separation, NOT_GRANTED};
use c600_digest::sha256_hex;
use c600_ir::{Effect, FnKind, Ty};
use c600_jumble_tool::tool::{
    manifest, manifests, rows, tool, CANDIDATE, CHECK_TWIST, CHECK_WORD, CLASSES, ENTRIES, MODULE, ROW, SOURCE,
};
use c600_run::boundary::Store;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::value::Value;
use c600_syntax::{format, lower, parse};
use c600_tools::tool::{admit_built_in, lower_source, targets, BoundaryClass, Manifest, Tool};

/// A module named as the tool's with `functions` as its functions.
fn module(functions: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule jumble_checker requires [];\n\n{functions}")
}

/// A command `name(params)` that returns the fixed acknowledgement of the receipt `r` its `lets` bind.
fn command(name: &str, params: &str, effects: &str, lets: &str) -> String {
    format!(
        "export command {name}({params}) -> record {{ base: Text, id: Text }} effects [{effects}] {{\n{lets}    \
         return record {{ base = receipt_base(r), id = receipt_id(r) }};\n}}\n"
    )
}

fn both() -> Separation {
    Separation { reads_state: true, prepares_previews: true }
}

fn previews_only() -> Separation {
    Separation { reads_state: false, prepares_previews: true }
}

/// The engine side of a run, counting every call: a state read answers a word, a proposal a receipt.
#[derive(Default)]
struct Counting {
    calls: usize,
    proposed: Vec<Vec<i32>>,
}

impl Host for Counting {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        self.calls += 1;
        Ok(RowTariff { kernel_cost: 1, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.calls += 1;
        match request {
            Request::Propose { word, .. } => {
                self.proposed.push(word.to_vec());
                Ok(Value::receipt(vec![("id", Value::text("preview-1"))]))
            }
            _ => Ok(Value::word(&[3, 17])),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        self.calls += 1;
        Ok(())
    }
}

/// Loads `source` as a built-in tool whose entry is `entry` and, if it loads, runs it on the word `[0, 12]`. Returns
/// the load's error, the engine side and the store.
fn attempt(source: &str, entry: &'static str) -> (Option<String>, Counting, Store) {
    let base = manifest(CHECK_WORD).expect("the tool's manifest");
    let manifest = Manifest { entry, source_sha256: sha256_hex(source.as_bytes()), ..base };
    let (mut engine, mut store) = (Counting::default(), Store::default());
    let error = match Tool::new(source, manifest) {
        Ok(tool) => {
            let ran = tool.run(&[0, 12], "attempt", "base", &mut engine, &mut store).expect("the run starts");
            assert_eq!(ran.shell.len(), 1);
            None
        }
        Err(error) => Some(error),
    };
    (error, engine, store)
}

/// CT-LB6-03 (part, the jumbling tool: the mechanical half of the review of its declared class against its exact
/// entry definitions. Not run: the review itself and its record, which wait for BM5, and the `PrincipleRecord`, which
/// LB6.5 says no candidate has).
///
/// The source parses with no diagnostic, is in the formatter's canonical layout and lowers to one module that
/// requests no capability, whose functions are exactly the manifest's entries. The declared class is LB6.5's class of
/// the "Named twist/word checker and kept checks" row, `checking`. Each entry is an exported command of one `Word`
/// parameter whose only effect is `preview`: it requests no capability, calls nothing, reads no state-derived data and
/// writes nothing (LG3.13, BND.39), so what it previews is the literal word the solver named. The built-in column
/// admits each entry in the solving profile; the own-tool column refuses it, as it refuses every previewing program
/// until the joint setting. A run of each entry proposes the named word once, as given, and the proposal's block
/// names the digest of the source that ran.
#[test]
fn ct_lb6_03_the_declared_class_matches_each_exact_entry_definition() {
    let file = parse(SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(file.edition.as_deref(), Some("6cs-draft-1"));
    assert_eq!(format(&file).as_deref(), Some(SOURCE), "the source is in canonical layout");
    let lowered = lower(&[file]).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(lowered.source(MODULE).expect("a source map").digest, sha256_hex(SOURCE.as_bytes()));
    let [module] = &lowered.program.modules[..] else { panic!("one module") };
    assert_eq!(module.name, MODULE);
    assert!(module.capabilities.is_empty());
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ENTRIES, "the module's functions are the manifest's entries");
    assert_eq!((CANDIDATE, &CLASSES[..]), ("Named twist/word checker and kept checks", &[BoundaryClass::Checking][..]));

    let program = lower_source(SOURCE).unwrap();
    let report = check(&program).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let source = sha256_hex(SOURCE.as_bytes());
    for (entry, function) in ENTRIES.into_iter().zip(&module.functions) {
        assert_eq!((function.kind, function.exported), (FnKind::Command, true), "{entry}");
        let [param] = &function.params[..] else { panic!("{entry}: one parameter") };
        assert_eq!((&param.ty.ty, param.ty.display), (&Ty::Word, false), "{entry}");
        let summary = report.summary(MODULE, entry).expect("the entry's summary");
        assert_eq!(summary.effects, BTreeSet::from([Effect::PreparePreview]), "{entry}");
        assert!(summary.capabilities.is_empty() && summary.state_reads.is_empty(), "{entry}");
        assert!(summary.exact_sinks.is_empty() && summary.vars.is_empty(), "{entry}");
        assert!(summary.unresolved.is_empty() && summary.reaches.is_empty(), "{entry}");
        assert_eq!(separation(summary, &Policy::interim()), previews_only(), "{entry}");

        let admission = admit_built_in(&program, MODULE, entry, &Grant::none(), &Policy::interim()).unwrap();
        assert!(admission.is_admitted(), "{entry}: {:#?}", admission.diagnostics);
        assert_eq!((admission.separation, admission.open_cells.len()), (previews_only(), 0), "{entry}");
        let loaded = tool(entry).expect("the shipped entry loads");
        assert_eq!(loaded.admission, admission, "{entry}");
        let own = admit(&program, MODULE, entry, Profile::Solving, &Grant::none(), &Policy::interim()).unwrap();
        assert_eq!(own.codes(), [NOT_GRANTED], "{entry}: the own-tool column prepares no preview");

        let (mut engine, mut store) = (Counting::default(), Store::default());
        let ran = loaded.run(&[0, 12, 0, 45], "c8", "base", &mut engine, &mut store).expect("the run starts");
        assert_eq!(ran.shell.len(), 1, "{entry}");
        assert_eq!((engine.calls, &engine.proposed[..]), (2, &[vec![0, 12, 0, 45]][..]), "{entry}: one proposal");
        let named = store.blocks.values().filter(|block| String::from_utf8_lossy(block).contains(&source)).count();
        assert_eq!(named, 1, "{entry}: the proposal's block names the digest of the source that ran");
    }
}

/// CT-LB6-03 (part, the jumbling tool: LX-3's correction-solve, state-based target selection and storage-laundering
/// cases, written as entries of this tool's module, are refused under chapter 07's selected mechanism, capability
/// separation, and never run. Not run: the frozen LX-3 corpus itself, which is C4's, and the cases the IR draft
/// cannot express: it has no word operations, so a word chosen from state is a word read from state).
///
/// - Correction solve: an entry reads the state-derived correction combination and proposes it.
/// - State-based target selection: an entry picks its target, or one of two words the solver named, by a state read.
/// - Storage laundering: an entry keeps a state-derived word in a workspace record and another proposes the record,
///   read directly or through a declassifier, enabled or not.
///
/// The checker admits each module as a well-typed program. The built-in column refuses each previewing entry with
/// `LG3:capability.not_granted`: at `entry` for reading state beside the preview (LG3.13, BND.39), and at the proposal
/// when the word is not a parameter as given. The keeping entry prepares no preview and is admitted, so the refusal
/// is the relay's. A refused module never loads as a tool: no run starts and the engine is never called.
#[test]
fn ct_lb6_03_lx3_cases_are_refused_under_capability_separation() {
    let preview = "target(\"operation.preview\")";
    let correction_solve = module(&command(
        "solve_corrections",
        "",
        "snapshot, preview",
        &format!("    let r = propose(read_snapshot<Word>(\"correction_combination\"), {preview});\n"),
    ));
    let target_selection = module(&format!(
        "{}\n{}",
        command(
            "pick_target",
            "w: Word",
            "snapshot, preview",
            &format!(
                "    let r = propose(w, if(read_snapshot<Bool>(\"cap_is_blocked\"), {preview}, \
                 target(\"operation.preview.cancel\")));\n"
            ),
        ),
        command(
            "pick_word",
            "a: Word, b: Word",
            "snapshot, preview",
            &format!("    let r = propose(if(read_snapshot<Bool>(\"lowers_n\"), a, b), {preview});\n"),
        ),
    ));
    let laundering = module(&format!(
        "export command keep() -> Unit effects [snapshot, workspace] {{\n    \
         let w = read_snapshot<Word>(\"lowering_word\");\n    let a = write_workspace<Word>(\"kept_check\", w);\n    \
         return ();\n}}\n\n{}\n{}",
        command(
            "relay",
            "",
            "snapshot, preview",
            &format!("    let r = propose(read_snapshot<Word>(\"kept_check\"), {preview});\n"),
        ),
        command(
            "relay_declassified",
            "",
            "snapshot, preview",
            &format!("    let r = propose(declassified<Word>(\"kept_check\"), {preview});\n"),
        ),
    ));

    let enabled = Policy::enabling(&["kept_check"]);
    let cases: [(&str, &String, &'static str, Policy, Separation, &[&str]); 6] = [
        ("correction solve", &correction_solve, "solve_corrections", Policy::interim(), both(), &["entry", "0"]),
        ("target chosen from state", &target_selection, "pick_target", Policy::interim(), both(), &["entry"]),
        ("word chosen from state", &target_selection, "pick_word", Policy::interim(), both(), &["entry", "0"]),
        ("laundering through a record", &laundering, "relay", Policy::interim(), both(), &["entry", "0"]),
        ("a declassified record", &laundering, "relay_declassified", Policy::interim(), both(), &["entry", "0"]),
        ("an enabled declassifier", &laundering, "relay_declassified", enabled, previews_only(), &["0"]),
    ];
    for (what, source, entry, policy, separation, paths) in cases {
        let program = lower_source(source).unwrap_or_else(|error| panic!("{what}: {error}"));
        let report = check(&program).unwrap();
        assert!(report.is_admitted(), "{what}: the checker admits the program: {:#?}", report.diagnostics);
        let admission = admit_built_in(&program, MODULE, entry, &Grant::none(), &policy).unwrap();
        let found: Vec<&str> = admission.diagnostics.iter().map(|d| d.path.as_str()).collect();
        assert_eq!((admission.separation, &found[..]), (separation, paths), "{what}");
        assert!(admission.codes().iter().all(|&code| code == NOT_GRANTED), "{what}: {:#?}", admission.diagnostics);
        let own = admit(&program, MODULE, entry, Profile::Solving, &Grant::none(), &policy).unwrap();
        assert!(!own.is_admitted(), "{what}: the own-tool column refuses it too");

        let (error, engine, store) = attempt(source, entry);
        let error = error.unwrap_or_else(|| panic!("{what}: the module loaded as a tool"));
        assert!(error.contains("not admitted") && error.contains(NOT_GRANTED), "{what}: {error}");
        assert_eq!((engine.calls, store.blocks.len()), (0, 0), "{what}: nothing ran");
    }
    // A target chosen from state is no literal row either, so the entry has no row set to declare.
    assert!(targets(&lower_source(&target_selection).unwrap(), MODULE, "pick_target").is_err());

    // The keeping entry reads state and prepares no preview: the built-in column admits it, and it proposes nothing.
    let program = lower_source(&laundering).unwrap();
    let keep = admit_built_in(&program, MODULE, "keep", &Grant::none(), &Policy::interim()).unwrap();
    assert!(keep.is_admitted(), "{:#?}", keep.diagnostics);
    assert_eq!(keep.separation, Separation { reads_state: true, prepares_previews: false });
    assert_eq!(targets(&program, MODULE, "keep"), Ok(BTreeSet::new()));
}

/// CT-BND-15 (part, the jumbling tool: the tool declares a class, and its row set is the set of rows its entries'
/// proposals name. Not run: the review of the row set against the class's row list, which BND.57 leaves to the
/// class's list and BM5's review, and the review record naming the entry digest: no review record exists, and the
/// manifest carries the digest such a record will name).
///
/// BND.37 and BND.27: every entry's manifest declares the class `checking`, one of BND.27's classes, with LB6.5's
/// candidate row and no `PrincipleRecord`. The row set is `operation.preview`, the row every entry's proposal names.
/// The manifest names the SHA-256 of the source, and an entry loads only with a manifest that names its source and
/// its rows; a name that is no entry has no manifest.
#[test]
fn ct_bnd_15_the_tool_declares_its_class_and_its_row_set() {
    let program = lower_source(SOURCE).unwrap();
    let manifests = manifests();
    assert_eq!(manifests.iter().map(|m| m.entry).collect::<Vec<_>>(), ENTRIES);
    assert_eq!(rows(), BTreeSet::from([ROW]));
    for manifest in &manifests {
        let entry = manifest.entry;
        assert_eq!(manifest.classes, [BoundaryClass::Checking], "{entry}");
        assert!(manifest.classes.iter().all(|class| BoundaryClass::ALL.contains(class)), "{entry}");
        assert_eq!(manifest.classes.iter().map(|class| class.name()).collect::<Vec<_>>(), ["checking"], "{entry}");
        assert_eq!((manifest.module, manifest.candidate, manifest.rows), (MODULE, CANDIDATE, &[ROW][..]), "{entry}");
        let named: BTreeSet<String> = manifest.rows.iter().map(|row| row.to_string()).collect();
        assert_eq!(targets(&program, MODULE, entry), Ok(named), "{entry}: the row set is the entry's");
        assert_eq!(manifest.source_sha256, sha256_hex(SOURCE.as_bytes()), "{entry}");
        assert!(!manifest.principle_record, "{entry}");
        assert_eq!(&tool(entry).expect("the entry loads").manifest, manifest, "{entry}");

        let other_source = Manifest { source_sha256: sha256_hex(b"another module"), ..manifest.clone() };
        assert!(Tool::new(SOURCE, other_source).is_err(), "{entry}");
        let other_rows = Manifest { rows: &["operation.execute"], ..manifest.clone() };
        assert!(Tool::new(SOURCE, other_rows).err().is_some_and(|error| error.contains("rows")), "{entry}");
    }
    assert_eq!(
        (manifest(CHECK_TWIST).map(|m| m.entry), manifest(CHECK_WORD).map(|m| m.entry)),
        (Some(CHECK_TWIST), Some(CHECK_WORD))
    );
    assert!(manifest("preview_word").is_none() && tool("preview_word").is_err());
}
