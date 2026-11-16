//! The 600-cell-Full tool's source, the checker's verdict, the declared class and capability separation (LP3, C5,
//! acceptance item 2). Each test names the conformance parts it covers; none of those IDs runs whole here.

use std::collections::BTreeSet;

use c600_check::{admit, check, separation, Grant, Policy, Profile, Separation, NOT_GRANTED};
use c600_digest::sha256_hex;
use c600_ir::{Effect, Expr, FnKind, Lit, Ty};
use c600_syntax::{format, lower, parse};
use c600_tools::tool::{
    admit_built_in, lower_source, manifest, targets, BoundaryClass, Tool, ENTRY, MODULE, ROW, SOURCE,
};

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

    let admission = admit_built_in(&program, MODULE, ENTRY, &Grant::none(), &Policy::interim()).unwrap();
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
/// admits each variant as a well-typed program; the refusal is the separation's.
///
/// CT-LB6-03 (part, the 600-cell-Full tool: a state read beside the preview is refused under the selected mechanism).
#[test]
fn a_variant_that_also_reads_state_is_refused() {
    let snapshot = "    let best = read_snapshot<Word>(\"best_word\");\n    \
                    let r = propose(w, target(\"operation.preview\"));\n";
    let outcome = "    let r = propose(w, target(\"operation.preview\"));\n    let seen = receipt_effect(r);\n";
    for (what, lets) in [("a snapshot read", snapshot), ("a preview-outcome read", outcome)] {
        let program = lower_source(&variant("export command", "snapshot, preview", &format!("{lets}{ACK}"))).unwrap();
        assert!(check(&program).unwrap().is_admitted(), "{what}: the checker admits the program");
        let admission = admit_built_in(&program, MODULE, ENTRY, &Grant::none(), &Policy::interim()).unwrap();
        assert_eq!(admission.separation, both(), "{what}");
        assert_eq!(admission.codes(), [NOT_GRANTED], "{what}");
        assert_eq!(admission.diagnostics[0].path, "entry", "{what}");
        assert!(admission.diagnostics[0].note.contains("LG3.13"), "{what}");
        let own = admit(&program, MODULE, ENTRY, Profile::Solving, &Grant::none(), &Policy::interim()).unwrap();
        assert_eq!((own.codes(), own.separation), (vec![NOT_GRANTED], both()), "{what}");
    }

    // A program that reads state and proposes the word it read is refused twice: for the separation and because the
    // word is not the solver's.
    let chosen = "    let r = propose(read_snapshot<Word>(\"best_word\"), target(\"operation.preview\"));\n";
    let program = lower_source(&variant("export command", "snapshot, preview", &format!("{chosen}{ACK}"))).unwrap();
    let admission = admit_built_in(&program, MODULE, ENTRY, &Grant::none(), &Policy::interim()).unwrap();
    assert_eq!(admission.separation, both());
    let paths: Vec<&str> = admission.diagnostics.iter().map(|d| d.path.as_str()).collect();
    assert_eq!((admission.codes(), paths), (vec![NOT_GRANTED, NOT_GRANTED], vec!["entry", "0"]));
}

/// The built-in column prepares a preview of a word the solver named (chapter 07 section 8), only inside a command
/// (LG3.6). A variant that proposes a word it wrote itself is refused at the proposal, though it reads no state; so is
/// a hand-built body in which a `let` shadows the parameter, and a previewing entry that is a plain function.
#[test]
fn a_built_in_previews_only_the_word_the_solver_named() {
    let refused = |program: &c600_ir::Program| {
        let admission = admit_built_in(program, MODULE, ENTRY, &Grant::none(), &Policy::interim()).unwrap();
        assert_eq!(admission.separation, Separation { reads_state: false, prepares_previews: true });
        assert!(admission.codes().iter().all(|&code| code == NOT_GRANTED), "{:#?}", admission.diagnostics);
        admission.diagnostics.iter().map(|d| d.path.clone()).collect::<Vec<_>>()
    };

    let own_word = "    let r = propose(word[+1, -2], target(\"operation.preview\"));\n";
    let program = lower_source(&variant("export command", "preview", &format!("{own_word}{ACK}"))).unwrap();
    assert!(check(&program).unwrap().is_admitted());
    assert_eq!(refused(&program), ["0"]);

    let mut program = lower_source(SOURCE).unwrap();
    let entry = &mut program.modules[0].functions[0];
    let body = std::mem::replace(&mut entry.body, Expr::Lit(Lit::Unit));
    let word = Box::new(Expr::Lit(Lit::Word(vec![1, -2])));
    entry.body = Expr::Let { name: "w".into(), value: word, body: Box::new(body) };
    assert_eq!(refused(&program), ["1.0"], "the shadowed parameter is not the solver's word");

    let helper =
        lower_source(&variant("export fn", "preview", &format!("{}{ACK}", &own_word.replace("word[+1, -2]", "w"))));
    assert_eq!(refused(&helper.unwrap()), ["entry"], "a preview is prepared only inside a command (LG3.6)");
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
    assert_eq!(manifest.source_sha256, sha256_hex(SOURCE.as_bytes()));
    assert!(!manifest.principle_record);
    assert_eq!(Tool::full_600_cell().unwrap().manifest, manifest);
}
