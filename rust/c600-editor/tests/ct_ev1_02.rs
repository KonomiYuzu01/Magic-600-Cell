//! CT-EV1-02 (whole; source): EV1.4. Fixtures for types, effects, exactness, evidence obligations and model or menu
//! mismatches produce distinct codes and correct source ranges, and LG2.22's direct and helper condition refusals pass
//! through unchanged, distinct from its direct scalar `exact_required` case.
//!
//! The evidence obligation is the draft's one (`rust/README.md`): the module context requires a declaration to supply
//! evidence, an exact evidence sink, and the IR draft has no claim types yet.

mod support;

use c600_check::{check, Diagnostic as CheckDiagnostic};
use c600_editor::{
    code, diagnose, Envelope, Form, IdentityField, IdentitySubject, ModuleContext, NextAction, ObligationRole, Reason,
    Service, Verdict,
};
use support::{full, importer, jumble, open, text, tool, LIB};

/// The diagnostics of a draft under a context, each with the text its range selects.
fn diagnosed(source: &str, context: &ModuleContext) -> Vec<(Envelope, String)> {
    let doc = open(source);
    let d = diagnose(&doc, doc.digest(), context).unwrap();
    assert_eq!(d.subject.source, doc.digest());
    if d.diagnostics.is_empty() {
        assert!(matches!(d.verdict, Verdict::Checked(_)), "{:?}", d.verdict);
    } else {
        assert_eq!(d.verdict, Verdict::Refused, "{:#?}", d.diagnostics);
    }
    d.diagnostics
        .into_iter()
        .map(|e| {
            let range = e.span.clone().expect("every EV1.4 diagnostic has a source range");
            assert_eq!(range.digest, doc.digest());
            let selected = text(source, &range).to_owned();
            (e, selected)
        })
        .collect()
}

/// The only diagnostic: its code and the text its range selects.
fn only(source: &str, context: &ModuleContext) -> (Envelope, String) {
    let mut all = diagnosed(source, context);
    assert_eq!(all.len(), 1, "{source}\n{all:#?}");
    all.remove(0)
}

/// The checker's own finding for the same program, to show that the service forwards it unchanged.
fn checker(source: &str) -> CheckDiagnostic {
    let file = c600_syntax::parse(source.as_bytes());
    let lowered = c600_syntax::lower(&[file]).unwrap();
    let report = check(&lowered.program).unwrap();
    assert_eq!(report.diagnostics.len(), 1);
    report.diagnostics[0].clone()
}

/// Asserts that an envelope forwards the checker's finding: the same code and the same typed context.
fn forwarded(e: &Envelope, d: &CheckDiagnostic) {
    assert_eq!(e.code, d.code);
    assert_eq!((e.form, e.service, &e.next), (Form::Diagnostic, Service::Diagnose, &NextAction::EditSource));
    let Reason::Check { module, function, path, value_type, display_control, qualifier_erasure, expected, supplied } =
        &e.reason
    else {
        panic!("{e:?}")
    };
    assert_eq!((module, function, path), (&d.module, &d.function, &d.path));
    assert_eq!(
        (value_type, *display_control, *qualifier_erasure),
        (&d.value_type, d.display_control, d.qualifier_erasure)
    );
    assert_eq!(expected.as_ref(), d.mismatch.as_ref().map(|m| &m.expected));
    assert_eq!(supplied.as_ref(), d.mismatch.as_ref().map(|m| &m.supplied));
}

const TYPE: &str = "fn f() -> Int effects [] {\n    return exact_add(1/2, 1);\n}\n";
const EFFECT: &str = "fn f(w: Word) -> Word effects [] {\n    let n = read_snapshot<Nat>(\"n\");\n    return w;\n}\n";
const DIRECT_CONDITION: &str =
    "fn f(x: Nat) -> Nat effects [] {\n    return if(approx_less(approx53(x), approx53(1)), 1, 2);\n}\n";
const HELPER_CONDITION: &str = "fn less(x: Nat) -> ApproxBool effects [] {\n    return approx_less(approx53(x), \
                                approx53(1));\n}\n\nfn f(x: Nat) -> Nat effects [] {\n    return if(less(x), 1, 2);\n}\n";
const DIRECT_SCALAR: &str = "fn f(x: Rat) -> Rat effects [] {\n    return exact_add(approx53(x), 1/2);\n}\n";
const WITHOUT_EVIDENCE: &str = "command go(n: Nat) -> Nat effects [] {\n    return n;\n}\n";
const WITH_EVIDENCE: &str = "command go(n: Nat) -> Unit effects [] {\n    return use_evidence(n);\n}\n";
const THROUGH_A_HELPER: &str = "fn give(n: Nat) -> Unit effects [] {\n    return use_evidence(n);\n}\n\n\
                                command go(n: Nat) -> Unit effects [] {\n    return give(n);\n}\n";

/// CT-EV1-02 (types and effects): an ordinary type error and an effect error, each forwarded with its code, typed
/// context and the range of the IR node the checker names.
#[test]
fn ct_ev1_02_type_and_effect_errors() {
    let context = ModuleContext::new();
    let (e, selected) = only(&tool(TYPE), &context);
    assert_eq!((e.code, selected.as_str()), (code::TYPE_MISMATCH, "exact_add(1/2, 1)"));
    forwarded(&e, &checker(&tool(TYPE)));
    let Reason::Check { expected, supplied, .. } = &e.reason else { unreachable!() };
    assert_eq!((expected.as_deref(), supplied.as_deref()), (Some("Int"), Some("Rat")));

    let (e, selected) = only(&tool(EFFECT), &context);
    assert_eq!(e.code, code::EFFECT_UNDECLARED);
    assert_eq!(selected, "let n = read_snapshot<Nat>(\"n\");\n    return w", "the body that performs the read");
    forwarded(&e, &checker(&tool(EFFECT)));
}

/// CT-EV1-02 (exactness, LG2.22): the direct condition and the helper-return condition are both
/// `LG2:type.exactness_violation`, forwarded unchanged at the condition; the direct scalar case is
/// `LG2:type.exact_required` at the approximate operand.
#[test]
fn ct_ev1_02_exactness_refusals_pass_through_unchanged() {
    let context = ModuleContext::new();
    let (direct, selected) = only(&tool(DIRECT_CONDITION), &context);
    assert_eq!((direct.code, selected.as_str()), (code::EXACTNESS_VIOLATION, "approx_less(approx53(x), approx53(1))"));
    forwarded(&direct, &checker(&tool(DIRECT_CONDITION)));

    let (helper, selected) = only(&tool(HELPER_CONDITION), &context);
    assert_eq!((helper.code, selected.as_str()), (code::EXACTNESS_VIOLATION, "less(x)"));
    forwarded(&helper, &checker(&tool(HELPER_CONDITION)));

    let (scalar, selected) = only(&tool(DIRECT_SCALAR), &context);
    assert_eq!((scalar.code, selected.as_str()), (code::EXACT_REQUIRED, "approx53(x)"));
    forwarded(&scalar, &checker(&tool(DIRECT_SCALAR)));
    assert_ne!(scalar.code, direct.code);
}

/// CT-EV1-02 (evidence obligations): a declaration the context requires to supply evidence and that supplies none is
/// `EV1:evidence.obligation_unmet` at its name; one that supplies it, directly or through a helper, checks; an absent
/// declaration is reported at the module header. An evidence value that is open is still a value, so only the absent
/// sink is an error.
#[test]
fn ct_ev1_02_unmet_evidence_obligations() {
    let context = ModuleContext::new().with_obligation("go", ObligationRole::Evidence);
    let (e, selected) = only(&tool(WITHOUT_EVIDENCE), &context);
    assert_eq!((e.code, selected.as_str()), (code::OBLIGATION_UNMET, "go"));
    assert_eq!(
        e.reason,
        Reason::ObligationUnmet { declaration: "go".into(), role: ObligationRole::Evidence, declared: true }
    );
    assert_eq!(e.next, NextAction::SupplyObligation { declaration: "go".into() });
    assert!(diagnosed(&tool(WITH_EVIDENCE), &context).is_empty());
    assert!(diagnosed(&tool(THROUGH_A_HELPER), &context).is_empty());
    assert!(diagnosed(&tool(WITHOUT_EVIDENCE), &ModuleContext::new()).is_empty(), "no obligation, no finding");

    let absent = ModuleContext::new().with_obligation("missing", ObligationRole::Evidence);
    let (e, selected) = only(&tool(WITH_EVIDENCE), &absent);
    assert_eq!((e.code, selected.as_str()), (code::OBLIGATION_UNMET, "module tool requires [];"));
    assert!(matches!(e.reason, Reason::ObligationUnmet { declared: false, .. }));
}

/// CT-EV1-02 (model and menu mismatches): a session identity outside the draft's claims is
/// `KN1:input.identity_mismatch` at the module header, naming the first differing field (the model for Full against
/// jumbling, the menu order for a reversed menu); a dependency whose claims differ is the same code at the import.
/// Nothing is reinterpreted for the session.
#[test]
fn ct_ev1_02_model_and_menu_identity_mismatches() {
    let source = tool(WITH_EVIDENCE);
    let model = ModuleContext::new().with_session(full(), vec![jumble("ascending")]);
    let (e, selected) = only(&source, &model);
    assert_eq!((e.code, selected.as_str()), (code::IDENTITY_MISMATCH, "module tool requires [];"));
    let Reason::IdentityMismatch { subject, field, expected, held } = &e.reason else { panic!("{e:?}") };
    assert_eq!((subject, *field), (&IdentitySubject::Module, IdentityField::Puzzle));
    assert_eq!((expected.as_deref(), held.as_deref()), (Some("full"), Some("jumble")));
    assert_eq!(e.next, NextAction::SelectContext);

    let menu = ModuleContext::new().with_session(jumble("descending"), vec![full(), jumble("ascending")]);
    let (e, _) = only(&source, &menu);
    let Reason::IdentityMismatch { field, expected, held, .. } = &e.reason else { panic!("{e:?}") };
    assert_eq!(*field, IdentityField::MenuOrder, "matching a model digest does not waive the menu order");
    assert_eq!((expected.as_deref(), held.as_deref()), (Some("descending"), Some("ascending")));
    assert!(diagnosed(&source, &ModuleContext::new().with_session(full(), vec![jumble("a"), full()])).is_empty());

    // A dependency that claims another menu order, at the root's import.
    let mut context = ModuleContext::new().with_session(jumble("descending"), vec![jumble("descending")]);
    let lib = context.add_dependency(LIB.as_bytes().to_vec(), Some(vec![jumble("ascending")]));
    let source = importer(&lib, "fn f(n: Nat) -> Nat effects [] {\n    return lib.double(n);\n}\n");
    let (e, selected) = only(&source, &context);
    assert_eq!(e.code, code::IDENTITY_MISMATCH);
    assert_eq!(selected, format!("import lib \"{lib}\";"));
    let Reason::IdentityMismatch { subject, field, .. } = &e.reason else { panic!("{e:?}") };
    assert_eq!(subject, &IdentitySubject::Dependency { alias: "lib".into(), digest: lib.clone() });
    assert_eq!(*field, IdentityField::MenuOrder);
}

/// CT-EV1-02 (distinct codes): one code per fixture kind, and the exactness codes stay LG2.22's two.
#[test]
fn ct_ev1_02_the_fixture_kinds_have_distinct_codes() {
    let plain = ModuleContext::new();
    let evidence = ModuleContext::new().with_obligation("go", ObligationRole::Evidence);
    let identity = ModuleContext::new().with_session(full(), vec![jumble("ascending")]);
    let kinds = [
        ("type", only(&tool(TYPE), &plain).0.code),
        ("effect", only(&tool(EFFECT), &plain).0.code),
        ("exactness", only(&tool(DIRECT_CONDITION), &plain).0.code),
        ("exact scalar", only(&tool(DIRECT_SCALAR), &plain).0.code),
        ("evidence", only(&tool(WITHOUT_EVIDENCE), &evidence).0.code),
        ("identity", only(&tool(WITH_EVIDENCE), &identity).0.code),
    ];
    let codes: std::collections::BTreeSet<&str> = kinds.iter().map(|(_, c)| *c).collect();
    assert_eq!(codes.len(), kinds.len(), "{kinds:?}");
    assert_eq!(only(&tool(HELPER_CONDITION), &plain).0.code, kinds[2].1);
}
