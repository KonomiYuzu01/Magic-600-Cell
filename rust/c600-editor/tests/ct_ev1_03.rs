//! CT-EV1-03 (whole; source): EV1.5 and EV1.6. Hover and definition resolve pinned dependencies; rename changes
//! resolved references only and refuses stale edit targets.

mod support;

use c600_editor::{
    apply_edits, code, definition, hover, rename, Because, DeclIdentity, Document, Exactness, HoverField,
    ModuleContext, NameKind, Permission, Reason, Resolved, Service, Stage, StaticCheck, Units, Verdict,
};
use support::{at, importer, open, text, tool, LIB};

fn context_with_lib() -> (ModuleContext, String) {
    let mut context = ModuleContext::new();
    let lib = context.add_dependency(LIB.as_bytes().to_vec(), None);
    (context, lib)
}

const BODY: &str =
    "export command go(x: Nat) -> Nat effects [] {\n    let y = lib.double(x);\n    return lib.twice(y);\n}\n";

/// CT-EV1-03 (EV1.5, hover on a pinned dependency's declaration): the declared signature and effects, exactness, the
/// identity under the dependency's digest, the checked needs, the definition range in the dependency, and the static
/// check kept apart from evidence and permission.
#[test]
fn ct_ev1_03_hover_resolves_a_pinned_dependency() {
    let (context, lib) = context_with_lib();
    let source = importer(&lib, BODY);
    let doc = open(&source);
    let h = hover(&doc, doc.digest(), at(&source, "double", 0, 2), &context).unwrap().unwrap();
    assert_eq!(h.name, "double");
    assert_eq!(text(&source, &h.range), "double");
    assert_eq!(h.resolved, Resolved::Member { digest: lib.clone() });
    assert_eq!(h.declared.as_deref(), Some("fn double(n: Nat) -> Nat effects []"));
    assert_eq!(h.declared_effects, Some(vec![]));
    assert_eq!(h.exactness, Exactness::Exact);
    assert_eq!(h.identity, Some(DeclIdentity { digest: lib.clone(), module: "lib".into(), name: "double".into() }));
    assert_eq!(h.check, StaticCheck::Checked);
    let inferred = h.inferred.as_ref().unwrap();
    assert!(inferred.effects.is_empty() && inferred.capabilities.is_empty());
    assert!(!inferred.reads_state && !inferred.prepares_previews);
    let def = h.definition.as_ref().unwrap();
    assert_eq!(def.digest, lib);
    assert_eq!(&LIB[def.start..def.end], "double");
    assert_eq!(h.permission, Permission::DecidedAtInvocation);
    assert!(h.evidence.is_empty());
    assert_eq!(h.unresolved.iter().map(|u| u.field).collect::<Vec<_>>(), [HoverField::ModelAndLevel]);
    assert!(h.subject.names(&doc));

    // The alias: its module, its header's requirements and the digest it pins.
    let h = hover(&doc, doc.digest(), at(&source, "lib.double", 0, 1), &context).unwrap().unwrap();
    assert_eq!(h.resolved, Resolved::Import { digest: lib.clone() });
    assert_eq!((h.declared.as_deref(), h.requires.clone()), (Some("module lib"), Some(vec![])));

    // A local and a parameter of the draft.
    let h = hover(&doc, doc.digest(), at(&source, "lib.twice(y)", 0, "lib.twice(".len()), &context).unwrap().unwrap();
    assert_eq!(h.resolved, Resolved::Local(NameKind::Let));
    assert_eq!(h.definition.as_ref().map(|d| text(&source, d)), Some("y"));
    assert!(h.unresolved.iter().any(|u| u.field == HoverField::Type && u.because == Because::NotReportedByChecker));
    let h = hover(&doc, doc.digest(), at(&source, "double(x)", 0, "double(".len()), &context).unwrap().unwrap();
    assert_eq!((h.resolved.clone(), h.declared.as_deref()), (Resolved::Local(NameKind::Parameter), Some("Nat")));
}

/// CT-EV1-03 (EV1.5, unresolved): without the dependency's bytes, a hover gives the known fields and why the rest is
/// unknown, with no invented check; another version of the dependency is never substituted.
#[test]
fn ct_ev1_03_hover_never_invents_a_check() {
    let (_, lib) = context_with_lib();
    let source = importer(&lib, BODY);
    let doc = open(&source);
    let other = ModuleContext::new().with_dependency(LIB.replace("exact_add(n, n)", "n").into_bytes(), None);
    for context in [ModuleContext::new(), other] {
        let h = hover(&doc, doc.digest(), at(&source, "double", 0, 0), &context).unwrap().unwrap();
        assert_eq!(h.resolved, Resolved::Member { digest: lib.clone() });
        assert_eq!(h.check, StaticCheck::NotChecked(Stage::Dependencies));
        assert!(h.declared.is_none() && h.inferred.is_none() && h.identity.is_none() && h.definition.is_none());
        let because = Because::DependencyMissing { digest: lib.clone() };
        assert!(h.unresolved.iter().any(|u| u.field == HoverField::Type && u.because == because));
        assert!(definition(&doc, doc.digest(), at(&source, "double", 0, 0), &context).unwrap().is_empty());
    }
    // A broken draft: the dependency's declaration still resolves from its own check, and the draft's own declaration
    // reports that it was not checked.
    let (context, _) = context_with_lib();
    let broken = source.replace("return lib.twice(y);", "return lib.twice(y)");
    let doc = open(&broken);
    let h = hover(&doc, doc.digest(), at(&broken, "double", 0, 0), &context).unwrap().unwrap();
    assert_eq!(h.check, StaticCheck::NotChecked(Stage::Syntax));
    assert!(h.inferred.is_some(), "the dependency closure checks on its own");
    let h = hover(&doc, doc.digest(), at(&broken, "go", 0, 0), &context).unwrap().unwrap();
    assert_eq!(h.resolved, Resolved::Declaration);
    assert!(h.unresolved.iter().any(|u| u.because == Because::NotChecked(Stage::Syntax)));
}

/// CT-EV1-03 (EV1.6, definition): a reference resolves to its binding in the draft, a dependency's declaration to its
/// range under the dependency's digest, and a reserved name to nothing.
#[test]
fn ct_ev1_03_definition_resolves_the_pinned_graph() {
    let (context, lib) = context_with_lib();
    let source = importer(&lib, BODY);
    let doc = open(&source);
    let def = |needle: &str, n: usize, shift: usize| {
        definition(&doc, doc.digest(), at(&source, needle, n, shift), &context).unwrap()
    };
    let twice = def("twice", 0, 1);
    assert_eq!(twice.len(), 1);
    assert_eq!((twice[0].digest.as_str(), &LIB[twice[0].start..twice[0].end]), (lib.as_str(), "twice"));
    let y = def("twice(y)", 0, "twice(".len());
    assert_eq!(
        (y[0].digest.as_str(), text(&source, &y[0]), y[0].start),
        (doc.digest(), "y", at(&source, "let y", 0, 4))
    );
    let alias = def("lib.twice", 0, 0);
    assert_eq!((alias[0].start, text(&source, &alias[0])), (at(&source, "import lib", 0, 7), "lib"));
    let source2 = tool("fn f(n: Nat) -> Nat effects [] {\n    return exact_add(n, 1);\n}\n");
    let doc2 = open(&source2);
    assert!(definition(&doc2, doc2.digest(), at(&source2, "exact_add", 0, 1), &context).unwrap().is_empty());
}

/// Two declarations each bind `x`; a comment and a string hold `x` too.
fn scopes() -> String {
    tool(
        "fn f(x: Nat) -> Nat effects [] {\n    let xs = exact_add(x, x);\n    return xs;\n}\n\n\
         # x stays x here\n\
         fn g(x: Nat) -> Text effects [] {\n    let t = \"x\";\n    return t;\n}\n\n\
         fn h(n: Nat) -> Nat effects [] {\n    return f(n);\n}\n",
    )
}

/// CT-EV1-03 (EV1.6, rename): renaming `f`'s parameter changes its binding and its two uses only: not `g`'s `x`, not
/// `xs`, not the comment or the string. The resulting draft is checked again.
#[test]
fn ct_ev1_03_rename_changes_resolved_references_only() {
    let source = scopes();
    let doc = open(&source);
    let context = ModuleContext::new();
    let r = rename(&doc, doc.digest(), at(&source, "x: Nat", 0, 0), "value", &context).unwrap();
    assert_eq!(r.declaration.as_ref().map(|d| (d.start, text(&source, d))), Some((at(&source, "x: Nat", 0, 0), "x")));
    let starts: Vec<usize> = r.edits.iter().map(|e| e.range.start).collect();
    assert_eq!(starts, [at(&source, "x: Nat", 0, 0), at(&source, "(x, x)", 0, 1), at(&source, "(x, x)", 0, 4)]);
    assert!(r
        .edits
        .iter()
        .all(|e| e.range.digest == doc.digest() && e.text == "value" && text(&source, &e.range) == "x"));
    assert!(matches!(r.verdict, Verdict::Checked(_)), "{:?}", r.diagnostics);
    let renamed = String::from_utf8(apply_edits(&doc, &r.edits).unwrap()).unwrap();
    assert!(renamed.contains("fn f(value: Nat)") && renamed.contains("exact_add(value, value)"));
    assert!(renamed.contains("fn g(x: Nat)") && renamed.contains("# x stays x here") && renamed.contains("\"x\""));

    // From a reference: renaming the function `f` at its call changes the declaration and the call.
    let r = rename(&doc, doc.digest(), at(&source, "f(n)", 0, 0), "first", &context).unwrap();
    let edited: Vec<&str> = r.edits.iter().map(|e| text(&source, &e.range)).collect();
    assert_eq!(edited, ["f", "f"]);
    assert_eq!(r.edits[0].range.start, at(&source, "fn f(", 0, 3));
    assert!(matches!(r.verdict, Verdict::Checked(_)));

    // A clash is not refused here: the resulting draft's check reports it.
    let r = rename(&doc, doc.digest(), at(&source, "x: Nat", 0, 0), "xs", &context).unwrap();
    assert_eq!(r.verdict, Verdict::NotChecked(Stage::Lowering));
    assert_eq!(r.diagnostics[0].code, code::IR_REJECTED);

    // A reserved name names no declaration.
    let r = rename(&doc, doc.digest(), at(&source, "exact_add", 0, 1), "plus", &context).unwrap();
    assert!(r.declaration.is_none() && r.edits.is_empty());
}

/// CT-EV1-03 (EV1.6, stale targets): a rename request for older bytes and edits applied to changed bytes are both
/// refused as `EV1:stale.editor_source`, and nothing is applied.
#[test]
fn ct_ev1_03_rename_refuses_stale_edit_targets() {
    let source = scopes();
    let doc = open(&source);
    let context = ModuleContext::new();
    let r = rename(&doc, doc.digest(), at(&source, "x: Nat", 0, 0), "value", &context).unwrap();
    let changed = Document::open(source.replace("return xs;", "return x;").into_bytes(), Units::Utf8);
    let refused = apply_edits(&changed, &r.edits).unwrap_err();
    assert_eq!((refused.code, refused.service), (code::STALE_EDITOR_SOURCE, Service::ApplyEdits));
    assert_eq!(
        refused.reason,
        Reason::StaleSource { requested: doc.digest().to_owned(), held: changed.digest().to_owned() }
    );
    let refused = rename(&changed, doc.digest(), at(&source, "x: Nat", 0, 0), "value", &context).unwrap_err();
    assert_eq!((refused.code, refused.service), (code::STALE_EDITOR_SOURCE, Service::Rename));
    // Overlapping edits are refused as invalid ranges, with nothing applied.
    let mut overlapping = r.edits.clone();
    overlapping.push(overlapping[0].clone());
    assert_eq!(apply_edits(&doc, &overlapping).unwrap_err().code, code::SOURCE_RANGE_INVALID);
}

/// CT-EV1-03 (EV1.6, a dependency's declaration): renaming `lib.double` proposes edits to `lib`'s bytes under `lib`'s
/// digest (its declaration and its own uses, not `helper`), to the draft's reference, and re-pins the draft's import to
/// the edited bytes' new digest, so no dependency is rewritten under its old digest. The draft checks against the
/// edited closure.
#[test]
fn ct_ev1_03_rename_never_rewrites_a_dependency_under_its_old_digest() {
    let (context, lib) = context_with_lib();
    let source = importer(&lib, BODY);
    let doc = open(&source);
    let r = rename(&doc, doc.digest(), at(&source, "double", 0, 0), "twofold", &context).unwrap();
    let declaration = r.declaration.as_ref().unwrap();
    assert_eq!((declaration.digest.as_str(), &LIB[declaration.start..declaration.end]), (lib.as_str(), "double"));
    let lib_edits: Vec<&str> = r.edits_for(&lib).map(|e| &LIB[e.range.start..e.range.end]).collect();
    assert_eq!(lib_edits, ["double", "double", "double"]);
    let new_lib =
        String::from_utf8(apply_edits(&open(LIB), &r.edits_for(&lib).cloned().collect::<Vec<_>>()).unwrap()).unwrap();
    assert!(new_lib.contains("export fn twofold(") && new_lib.contains("twofold(twofold(n))"));
    let new_digest = c600_syntax::sha256_hex(new_lib.as_bytes());
    assert_eq!(r.changed.iter().find(|(old, _)| *old == lib).map(|(_, new)| new.as_str()), Some(new_digest.as_str()));

    let root: Vec<(&str, &str)> =
        r.edits_for(doc.digest()).map(|e| (text(&source, &e.range), e.text.as_str())).collect();
    assert_eq!(root, [(lib.as_str(), new_digest.as_str()), ("double", "twofold")]);
    assert!(matches!(r.verdict, Verdict::Checked(_)), "{:?}", r.diagnostics);
    let renamed =
        String::from_utf8(apply_edits(&doc, &r.edits_for(doc.digest()).cloned().collect::<Vec<_>>()).unwrap()).unwrap();
    assert!(renamed.contains(&format!("import lib \"{new_digest}\";")) && renamed.contains("lib.twofold(x)"));
}
