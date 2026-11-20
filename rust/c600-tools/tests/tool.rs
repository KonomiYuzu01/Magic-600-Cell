//! The 600-cell-Full tool's source, the checker's verdict, the declared class and capability separation (LP3, C5,
//! acceptance item 2). Each test names the conformance parts it covers; none of those IDs runs whole here.

use std::collections::BTreeSet;

use c600_check::{
    admit, admit_built_in, check, separation, Grant, Policy, Profile, Row, Separation, EFFECT_UNDECLARED, NOT_GRANTED,
};
use c600_digest::sha256_hex;
use c600_ir::{Effect, Expr, FnKind, Lit, Ty};
use c600_run::boundary::Store;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::value::Value;
use c600_syntax::{format, lower, parse};
use c600_tools::tool::{
    class_rows, lower_source, manifest, targets, BoundaryClass, Manifest, Tool, ENTRY, MODULE, ROW, SOURCE,
};

/// The shipped tool's class: the prepare-a-preview row of its manifest's row.
fn class() -> BTreeSet<Row> {
    class_rows(&manifest())
}

/// The shipped class with `rows` added: a fixture that tests another rule lists the rows it uses, so that each refusal
/// shown is the rule under test.
fn wider(rows: &[Row]) -> BTreeSet<Row> {
    let mut class = class();
    class.extend(rows.iter().cloned());
    class
}

/// The tool with `body` in place of its entry's body and `effects` as its effect list.
fn variant(head: &str, effects: &str, body: &str) -> String {
    format!(
        "edition \"6cs-draft-1\";\nmodule full_600_cell requires [];\n\n{head} preview_word(w: Word) -> record {{ base: \
         Text, id: Text }} effects [{effects}] {{\n{body}}}\n"
    )
}

const ACK: &str = "    return record { base = receipt_base(r), id = receipt_id(r) };\n";

fn both() -> Separation {
    Separation { reads_state: true, prepares_previews: true }
}

/// The tool's source parses with no diagnostic, is in the formatter's canonical layout and lowers to the IR draft: one
/// module that requests no capability, with one exported command that takes one `Word`.
#[test]
fn the_source_parses_and_lowers_to_one_exported_command() {
    let file = parse(SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(file.edition.as_deref(), Some("6cs-draft-1"));
    assert_eq!(format(&file).as_deref(), Some(SOURCE), "the source is in canonical layout");
    let lowered = lower(&[file]).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(lowered.source(MODULE).expect("a source map").digest, manifest().source_sha256);
    let [module] = &lowered.program.modules[..] else { panic!("one module") };
    assert_eq!(module.name, MODULE);
    assert!(module.capabilities.is_empty());
    let [entry] = &module.functions[..] else { panic!("one function") };
    assert_eq!((entry.name.as_str(), entry.kind, entry.exported), (ENTRY, FnKind::Command, true));
    let [param] = &entry.params[..] else { panic!("one parameter") };
    assert_eq!((param.name.as_str(), &param.ty.ty, param.ty.display), ("w", &Ty::Word, false));
}

/// Acceptance item 2, the checker's verdict. The checker admits the tool with no diagnostic. Its entry prepares
/// previews and does nothing else: its only effect is `preview`, it requests no capability, calls nothing, reads no
/// state-derived data and writes no workspace record or file (LG3.13, BND.39). The built-in column admits it in the
/// solving profile. `c600-check`'s own `Profile::Solving`, the column of a solver's own tool, refuses it as it refuses
/// every previewing program until the joint setting; tool authoring admits it.
///
/// CT-LB6-03 (part, the 600-cell-Full tool: the checker half of the review of its exact entry definition).
#[test]
fn the_checker_admits_the_tool_and_it_only_prepares_previews() {
    let program = lower_source(SOURCE).unwrap();
    let report = check(&program).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let summary = report.summary(MODULE, ENTRY).expect("the entry's summary");
    assert_eq!(summary.effects, BTreeSet::from([Effect::PreparePreview]));
    assert!(summary.capabilities.is_empty() && summary.state_reads.is_empty() && summary.exact_sinks.is_empty());
    assert!(summary.vars.is_empty() && summary.unresolved.is_empty() && summary.reaches.is_empty());
    let only_previews = Separation { reads_state: false, prepares_previews: true };
    assert_eq!(separation(summary, &Policy::interim()), only_previews);

    let admission = admit_built_in(&program, MODULE, ENTRY, &class(), &Grant::none(), &Policy::interim()).unwrap();
    assert!(admission.is_admitted(), "{:#?}", admission.diagnostics);
    assert_eq!((admission.separation, admission.open_cells.len()), (only_previews, 0));
    let tool = Tool::full_600_cell().expect("the shipped tool loads");
    assert_eq!(tool.admission, admission);

    let own = admit(&program, MODULE, ENTRY, Profile::Solving, &Grant::none(), &Policy::interim()).unwrap();
    assert_eq!(own.codes(), [NOT_GRANTED], "the own-tool column prepares no preview until the joint setting");
    let authoring = admit(&program, MODULE, ENTRY, Profile::ToolAuthoring, &Grant::none(), &Policy::interim());
    assert!(authoring.unwrap().is_admitted());
}

/// Acceptance item 2, capability separation (LQ21, LG3.13, BND.39). A variant of the tool that also reads state is
/// refused in the solving profile with `LG3:capability.not_granted`, although the word it proposes is still the
/// solver's: a snapshot read, and a read of its own preview's effect, which is state-derived (BND.42). The checker
/// admits each variant as a well-typed program; the refusal is the separation's. The class here also lists the read's
/// row, so that the separation is the only refusal; under the shipped tool's class the read is also a row the class
/// does not list (BND.57), a second refusal at `entry`.
///
/// CT-LB6-03 (part, the 600-cell-Full tool: a state read beside the preview is refused under the selected mechanism).
#[test]
fn a_variant_that_also_reads_state_is_refused() {
    let snapshot = "    let best = read_snapshot<Word>(\"best_word\");\n    \
                    let r = propose(w, target(\"operation.preview\"));\n";
    let outcome = "    let r = propose(w, target(\"operation.preview\"));\n    let seen = receipt_effect(r);\n";
    let reads = wider(&[Row::Read("best_word".into()), Row::Outcome]);
    for (what, lets) in [("a snapshot read", snapshot), ("a preview-outcome read", outcome)] {
        let program = lower_source(&variant("export command", "snapshot, preview", &format!("{lets}{ACK}"))).unwrap();
        assert!(check(&program).unwrap().is_admitted(), "{what}: the checker admits the program");
        let admission = admit_built_in(&program, MODULE, ENTRY, &reads, &Grant::none(), &Policy::interim()).unwrap();
        assert_eq!(admission.separation, both(), "{what}");
        assert_eq!(admission.codes(), [NOT_GRANTED], "{what}");
        assert_eq!(admission.diagnostics[0].path, "entry", "{what}");
        assert!(admission.diagnostics[0].note.contains("LG3.13"), "{what}");
        let shipped = admit_built_in(&program, MODULE, ENTRY, &class(), &Grant::none(), &Policy::interim()).unwrap();
        let paths: Vec<&str> = shipped.diagnostics.iter().map(|d| d.path.as_str()).collect();
        assert_eq!((shipped.codes(), paths), (vec![NOT_GRANTED; 2], vec!["entry"; 2]), "{what}");
        assert!(shipped.diagnostics[1].note.contains("BND.57"), "{what}: the class lists no such row");
        let own = admit(&program, MODULE, ENTRY, Profile::Solving, &Grant::none(), &Policy::interim()).unwrap();
        assert_eq!((own.codes(), own.separation), (vec![NOT_GRANTED], both()), "{what}");
    }

    // A program that reads state and proposes the word it read is refused twice: for the separation and because the
    // word is not the solver's.
    let chosen = "    let r = propose(read_snapshot<Word>(\"best_word\"), target(\"operation.preview\"));\n";
    let program = lower_source(&variant("export command", "snapshot, preview", &format!("{chosen}{ACK}"))).unwrap();
    let admission = admit_built_in(&program, MODULE, ENTRY, &reads, &Grant::none(), &Policy::interim()).unwrap();
    assert_eq!(admission.separation, both());
    let paths: Vec<&str> = admission.diagnostics.iter().map(|d| d.path.as_str()).collect();
    assert_eq!((admission.codes(), paths), (vec![NOT_GRANTED, NOT_GRANTED], vec!["entry", "0"]));
}

/// The built-in column prepares a preview of a word the solver named (chapter 07 section 8), only inside a command
/// (LG3.6). A variant that proposes a word it wrote itself is refused at the proposal with
/// `LG3:capability.not_granted`, though it reads no state; so is a hand-built body in which a `let` shadows the
/// parameter. A previewing entry that is a plain function is refused with `LG3:effect.undeclared` at `entry`: the
/// effect occurs only inside a command (LG3.6).
#[test]
fn a_built_in_previews_only_the_word_the_solver_named() {
    let refused = |program: &c600_ir::Program| {
        let admission = admit_built_in(program, MODULE, ENTRY, &class(), &Grant::none(), &Policy::interim()).unwrap();
        assert_eq!(admission.separation, Separation { reads_state: false, prepares_previews: true });
        admission.diagnostics.iter().map(|d| (d.path.clone(), d.code)).collect::<Vec<_>>()
    };
    let at = |path: &str, code: &'static str| vec![(path.to_owned(), code)];

    let own_word = "    let r = propose(word[+1, -2], target(\"operation.preview\"));\n";
    let program = lower_source(&variant("export command", "preview", &format!("{own_word}{ACK}"))).unwrap();
    assert!(check(&program).unwrap().is_admitted());
    assert_eq!(refused(&program), at("0", NOT_GRANTED));

    let mut program = lower_source(SOURCE).unwrap();
    let entry = &mut program.modules[0].functions[0];
    let body = std::mem::replace(&mut entry.body, Expr::Lit(Lit::Unit));
    let word = Box::new(Expr::Lit(Lit::Word(vec![1, -2])));
    entry.body = Expr::Let { name: "w".into(), value: word, body: Box::new(body) };
    assert_eq!(refused(&program), at("1.0", NOT_GRANTED), "the shadowed parameter is not the solver's word");

    let helper =
        lower_source(&variant("export fn", "preview", &format!("{}{ACK}", own_word.replace("word[+1, -2]", "w"))));
    let found = refused(&helper.unwrap());
    assert_eq!(found, at("entry", EFFECT_UNDECLARED), "a preview is prepared only inside a command (LG3.6)");
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
            _ => Ok(Value::word(&[3, -4])),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        self.calls += 1;
        Ok(())
    }
}

/// Loads `source` as a built-in tool whose entry is `entry` and, if it loads, runs it on the word `[1, -2]`. Returns
/// the load's error, the engine side and the store.
fn attempt(source: &str, entry: &'static str) -> (Option<String>, Counting, Store) {
    let manifest = Manifest { entry, source_sha256: sha256_hex(source.as_bytes()), ..manifest() };
    let (mut engine, mut store) = (Counting::default(), Store::default());
    let error = match Tool::new(source, manifest) {
        Ok(tool) => {
            let ran = tool.run(&[1, -2], "attempt", "base", &mut engine, &mut store).expect("the run starts");
            assert_eq!(ran.shell.len(), 1);
            None
        }
        Err(error) => Some(error),
    };
    (error, engine, store)
}

/// Leads L1 and L3 of the LP3 trap analysis (BND.39, BND.57, LG3.10). The built-in column, author `built_in` in the
/// solving profile, refuses an entry that reads a snapshot and proposes the word it read (the IR draft has no word
/// operations, so a word read from state is the word such an entry can compute), and it refuses the relay of such a
/// word through a workspace record at both ends: entry `keep` reads state and writes the word to a record, refused at
/// the write because a stored word too must be one the solver named (LP2's built-in column), and entry `relay` reads
/// the record and proposes it, refused at the proposal. Entry `relay_declassified` reads the record through a
/// declassifier: under the interim policy that is a state read, and with the declassifier enabled the word is still
/// not the one the solver named. The class here lists every row the fixture uses, so each refusal shown is the rule
/// under test; each is `LG3:capability.not_granted`. A refused module, and a module with one checker diagnostic, never
/// loads as a tool, so no run starts and the engine is never called. Through the same path the shipped tool proposes
/// the solver's word once, and the proposal's provenance block names the digest of the source bytes that ran: the
/// producer's module digest is computed in [`Tool::new`] from those bytes, and a manifest naming other bytes is refused
/// (`the_tool_declares_its_class_and_its_rows`).
///
/// CT-LB6-03 (part, the 600-cell-Full tool: a word from state, read directly or through a workspace record, is refused
/// under the selected mechanism, and a refused or unchecked module never runs).
#[test]
fn a_refused_built_in_never_runs_and_the_named_word_previews() {
    let read_and_propose = variant(
        "export command",
        "snapshot, preview",
        &format!("    let r = propose(read_snapshot<Word>(\"best_word\"), target(\"operation.preview\"));\n{ACK}"),
    );
    let relay = format!(
        "edition \"6cs-draft-1\";\nmodule full_600_cell requires [];\n\n\
         export command keep() -> Unit effects [snapshot, workspace] {{\n    \
         let w = read_snapshot<Word>(\"best_word\");\n    let a = write_workspace<Word>(\"kept\", w);\n    \
         return ();\n}}\n\n\
         export command relay() -> record {{ base: Text, id: Text }} effects [snapshot, preview] {{\n    \
         let r = propose(read_snapshot<Word>(\"kept\"), target(\"operation.preview\"));\n{ACK}}}\n\n\
         export command relay_declassified() -> record {{ base: Text, id: Text }} effects [snapshot, preview] {{\n    \
         let r = propose(declassified<Word>(\"kept\"), target(\"operation.preview\"));\n{ACK}}}\n"
    );
    let program = lower_source(&relay).unwrap();
    let report = check(&program).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let rows = wider(&[
        Row::Read("best_word".into()),
        Row::Read("kept".into()),
        Row::Workspace("kept".into()),
        Row::Declassified("kept".into()),
    ]);
    // `keep` reads state and prepares no preview; the built-in column refuses it at the write (path `1.0`, the second
    // `let`'s value), because the word it stores is not one the solver named.
    let keep = admit_built_in(&program, MODULE, "keep", &rows, &Grant::none(), &Policy::interim()).unwrap();
    let found: Vec<(&str, &str)> = keep.diagnostics.iter().map(|d| (d.path.as_str(), d.code)).collect();
    assert_eq!(found, [("1.0", NOT_GRANTED)], "{:#?}", keep.diagnostics);
    assert_eq!(keep.separation, Separation { reads_state: true, prepares_previews: false });
    // With the declassifier enabled, the declassified read is no state read, and the word is still not the solver's.
    let enabled = Policy::enabling(&["kept"]);
    let previews_only = Separation { reads_state: false, prepares_previews: true };
    for (entry, policy, separation, paths) in [
        ("relay", Policy::interim(), both(), vec!["entry", "0"]),
        ("relay_declassified", Policy::interim(), both(), vec!["entry", "0"]),
        ("relay_declassified", enabled, previews_only, vec!["0"]),
    ] {
        let admission = admit_built_in(&program, MODULE, entry, &rows, &Grant::none(), &policy).unwrap();
        let found: Vec<&str> = admission.diagnostics.iter().map(|d| d.path.as_str()).collect();
        assert_eq!((admission.separation, found), (separation, paths), "{entry}");
        assert!(admission.codes().iter().all(|&code| code == NOT_GRANTED), "{entry}: {:#?}", admission.diagnostics);
    }

    // One checker diagnostic: the entry returns a `Text` where its signature says a record.
    let unchecked = variant(
        "export command",
        "preview",
        "    let r = propose(w, target(\"operation.preview\"));\n    return receipt_id(r);\n",
    );
    assert_eq!(check(&lower_source(&unchecked).unwrap()).unwrap().diagnostics.len(), 1);

    for (what, source, entry) in [
        ("reads a snapshot and proposes it", &read_and_propose, ENTRY),
        ("relays a state read through a workspace record", &relay, "relay"),
        ("relays a record through a declassifier", &relay, "relay_declassified"),
        ("has one checker diagnostic", &unchecked, ENTRY),
    ] {
        let (error, engine, store) = attempt(source, entry);
        let error = error.unwrap_or_else(|| panic!("{what}: the module loaded as a tool"));
        assert!(error.contains("not admitted"), "{what}: {error}");
        assert_eq!((engine.calls, store.blocks.len()), (0, 0), "{what}: nothing ran");
    }
    assert!(attempt(&read_and_propose, ENTRY).0.unwrap().contains(NOT_GRANTED));

    let (error, engine, store) = attempt(SOURCE, ENTRY);
    assert_eq!(error, None);
    assert_eq!((engine.calls, engine.proposed), (2, vec![vec![1, -2]]), "one tariff and one proposal");
    let source = sha256_hex(SOURCE.as_bytes());
    let named = store.blocks.values().filter(|block| String::from_utf8_lossy(block).contains(&source)).count();
    assert_eq!(named, 1, "the proposal's block names the digest of the source that ran");
}

/// BND.37 and BND.27: the tool declares its class in its module manifest, the class is one of BND.27's, and the
/// manifest's row set is the set of rows the entry's proposals name. No `PrincipleRecord` and no review record exist.
///
/// CT-BND-15 (part, the 600-cell-Full tool: the class is declared and the row set is the entry's; the class-to-row
/// review and the review record naming the entry digest wait for BM5).
#[test]
fn the_tool_declares_its_class_and_its_rows() {
    let manifest = manifest();
    assert_eq!(manifest.classes, [BoundaryClass::Analysis, BoundaryClass::Checking]);
    assert!(manifest.classes.iter().all(|class| BoundaryClass::ALL.contains(class)));
    let names: Vec<&str> = manifest.classes.iter().map(|class| class.name()).collect();
    assert_eq!(names, ["analysis", "checking"]);
    assert_eq!((manifest.module, manifest.entry, manifest.rows), (MODULE, ENTRY, &[ROW][..]));
    let program = lower_source(SOURCE).unwrap();
    let rows: BTreeSet<String> = manifest.rows.iter().map(|row| row.to_string()).collect();
    assert_eq!(targets(&program, MODULE, ENTRY), Ok(rows));
    assert_eq!(class_rows(&manifest), BTreeSet::from([Row::Preview(ROW.to_owned())]), "the class's rows (draft)");
    assert_eq!(manifest.source_sha256, sha256_hex(SOURCE.as_bytes()));
    assert!(!manifest.principle_record);
    assert_eq!(Tool::full_600_cell().unwrap().manifest, manifest);

    // A tool loads only with a manifest that names its source, whose class lists the row its proposals name, and whose
    // rows are exactly the rows its proposals name.
    let other_source = Manifest { source_sha256: sha256_hex(b"another module"), ..manifest.clone() };
    assert!(Tool::new(SOURCE, other_source).is_err());
    let extra_row = Manifest { rows: &[ROW, "operation.execute"], ..manifest.clone() };
    let error = Tool::new(SOURCE, extra_row).err().expect("one row too many");
    assert!(error.contains("do not name the manifest's rows"), "{error}");
    let other_rows = Manifest { rows: &["operation.execute"], ..manifest };
    let error = Tool::new(SOURCE, other_rows).err().expect("the class lists no row the entry uses");
    assert!(error.contains("not admitted") && error.contains("BND.57"), "{error}");
}
