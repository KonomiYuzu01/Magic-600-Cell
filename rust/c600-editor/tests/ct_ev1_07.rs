//! CT-EV1-07 (whole; source): EV1.12. Each editor refusal exposes the event, the reason and the next action as typed
//! data, and its handling depends on no English text: every refusal the services raise is produced below, and a host
//! handler that reads only `code`, `reason` and `next` takes the same step whatever the summary says.

mod support;

use c600_editor::{
    apply_edits, code, complete, definition, diagnose, edit_declared_part, hover, rename, Document, Edit, Envelope,
    Form, IdentitySubject, ModuleContext, NextAction, PartValue, Reason, Retry, Service, SourceRange, Units, WHOLE,
};
use support::{full, importer, jumble, open, tool, LIB};

/// What a host does next, decided from typed data only.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    Reread,
    AskForPosition,
    ReopenWithUnits,
    Reencode,
    PickContext,
    SplitEdits,
    AddDependency(String),
    /// Ask for a value of the named type (EV3.7's round trip).
    AskForValue(String),
    Edit,
}

fn handle(e: &Envelope) -> Step {
    match (&e.next, e.code) {
        (NextAction::Refresh, code::STALE_EDITOR_SOURCE) => Step::Reread,
        (NextAction::ChoosePosition, code::SOURCE_RANGE_INVALID) => Step::AskForPosition,
        (NextAction::UseDeclaredUnits, code::SOURCE_RANGE_INVALID) => Step::ReopenWithUnits,
        (NextAction::Reencode, _) => Step::Reencode,
        (NextAction::SelectContext, code::IDENTITY_MISMATCH) => Step::PickContext,
        (NextAction::SeparateEdits, code::SOURCE_RANGE_INVALID) => Step::SplitEdits,
        (NextAction::SupplyDependency { digest }, _) => Step::AddDependency(digest.clone()),
        (NextAction::SupplyValue { expected }, code::TYPE_MISMATCH) => Step::AskForValue(expected.clone()),
        _ => Step::Edit,
    }
}

/// Every refusal the services raise, with its label, the service, the reason's kind and the host's step.
fn refusals() -> Vec<(&'static str, Envelope, Service, &'static str, Step)> {
    let source = tool("fn f(x: Nat) -> Nat effects [] {\n    return x;\n}\n");
    let doc = open(&source);
    let other = open(&source.replace("return x", "return  x"));
    let utf16 = Document::open("a\u{1F600}", Units::Utf16);
    let invalid = Document::open(vec![0x61, 0xFF, 0x62], Units::Utf8);
    let context = ModuleContext::new();
    let old = doc.digest();
    let mut all = vec![
        (
            "unknown units",
            Document::open_declared("x", "bytes").unwrap_err(),
            Service::Open,
            "UndeclaredUnits",
            Step::ReopenWithUnits,
        ),
        ("range stale", utf16.range(old, 0, 0).unwrap_err(), Service::Range, "StaleSource", Step::Reread),
        (
            "range outside",
            utf16.range(utf16.digest(), 0, 9).unwrap_err(),
            Service::Range,
            "OutOfBounds",
            Step::AskForPosition,
        ),
        (
            "range inside a pair",
            utf16.offset(utf16.digest(), 2).unwrap_err(),
            Service::Range,
            "InsideScalar",
            Step::AskForPosition,
        ),
        (
            "range reversed",
            utf16.range(utf16.digest(), 1, 0).unwrap_err(),
            Service::Range,
            "Reversed",
            Step::AskForPosition,
        ),
        (
            "range undeclared",
            utf16.host_in(&SourceRange::new(utf16.digest(), 0, 1), Units::Utf8).unwrap_err(),
            Service::Range,
            "UndeclaredUnits",
            Step::ReopenWithUnits,
        ),
        (
            "range unpinned",
            invalid.offset(invalid.digest(), 0).unwrap_err(),
            Service::Range,
            "NotPinned",
            Step::Reencode,
        ),
        (
            "complete stale",
            complete(&other, old, 0, &context).unwrap_err(),
            Service::Complete,
            "StaleSource",
            Step::Reread,
        ),
        (
            "complete outside",
            complete(&doc, old, 9999, &context).unwrap_err(),
            Service::Complete,
            "OutOfBounds",
            Step::AskForPosition,
        ),
        (
            "diagnose stale",
            diagnose(&other, old, &context).unwrap_err(),
            Service::Diagnose,
            "StaleSource",
            Step::Reread,
        ),
        ("hover stale", hover(&other, old, 0, &context).unwrap_err(), Service::Hover, "StaleSource", Step::Reread),
        (
            "hover unpinned",
            hover(&invalid, invalid.digest(), 0, &context).unwrap_err(),
            Service::Hover,
            "NotPinned",
            Step::Reencode,
        ),
        (
            "definition stale",
            definition(&other, old, 0, &context).unwrap_err(),
            Service::Definition,
            "StaleSource",
            Step::Reread,
        ),
        (
            "definition outside",
            definition(&doc, old, 9999, &context).unwrap_err(),
            Service::Definition,
            "OutOfBounds",
            Step::AskForPosition,
        ),
        (
            "rename stale",
            rename(&other, old, 0, "y", &context).unwrap_err(),
            Service::Rename,
            "StaleSource",
            Step::Reread,
        ),
        (
            "rename outside",
            rename(&doc, old, 9999, "y", &context).unwrap_err(),
            Service::Rename,
            "OutOfBounds",
            Step::AskForPosition,
        ),
    ];
    let edit = |s, e| Edit { range: SourceRange::new(old, s, e), text: "y".into() };
    all.push((
        "edits stale",
        apply_edits(&other, &[edit(0, 1)]).unwrap_err(),
        Service::ApplyEdits,
        "StaleSource",
        Step::Reread,
    ));
    all.push((
        "edits outside",
        apply_edits(&doc, &[edit(0, 9999)]).unwrap_err(),
        Service::ApplyEdits,
        "OutOfBounds",
        Step::AskForPosition,
    ));
    all.push((
        "edits overlap",
        apply_edits(&doc, &[edit(0, 4), edit(2, 6)]).unwrap_err(),
        Service::ApplyEdits,
        "OverlappingEdits",
        Step::SplitEdits,
    ));
    let unpinned_edit = Edit { range: SourceRange::new(invalid.digest(), 0, 0), text: "y".into() };
    all.push((
        "edits unpinned",
        apply_edits(&invalid, &[unpinned_edit]).unwrap_err(),
        Service::ApplyEdits,
        "NotPinned",
        Step::Reencode,
    ));
    // Completion reads types under one identity: a context whose claims differ refuses, and so does a member list of
    // a dependency that claims another identity.
    let mismatched = ModuleContext::new().with_session(full(), vec![jumble("a")]);
    all.push((
        "complete identity",
        complete(&doc, old, 0, &mismatched).unwrap_err(),
        Service::Complete,
        "IdentityMismatch",
        Step::PickContext,
    ));
    let mut dependency = ModuleContext::new().with_session(full(), vec![full()]);
    let lib = dependency.add_dependency(LIB.as_bytes().to_vec(), Some(vec![jumble("a")]));
    let importing = importer(&lib, "fn f(n: Nat) -> Nat effects [] {\n    return lib.double(n);\n}\n");
    let importing_doc = open(&importing);
    let member = importing.find("lib.double").unwrap() + 4;
    let refused = complete(&importing_doc, importing_doc.digest(), member, &dependency).unwrap_err();
    if let Reason::IdentityMismatch { subject, .. } = &refused.reason {
        assert_eq!(subject, &IdentitySubject::Dependency { alias: "lib".into(), digest: lib.clone() });
    }
    all.push(("member identity", refused, Service::Complete, "IdentityMismatch", Step::PickContext));
    // The declared-part round trip (EV3.7): an undeclared field, and a value of another type than a declared one's.
    let nat = PartValue::Int("1".into());
    all.push((
        "round trip undeclared",
        edit_declared_part(&doc, old, "f", "x", &nat, &context).unwrap_err(),
        Service::EditDeclaredPart,
        "RoundTripUndeclared",
        Step::Edit,
    ));
    let declared = open(&tool("# @editable parameter\nfn p() -> Nat effects [] {\n    return 1;\n}\n"));
    let text = PartValue::Text("one".into());
    all.push((
        "round trip type",
        edit_declared_part(&declared, declared.digest(), "p", WHOLE, &text, &context).unwrap_err(),
        Service::EditDeclaredPart,
        "ValueType",
        Step::AskForValue("Nat".into()),
    ));
    all
}

fn reason_kind(r: &Reason) -> &'static str {
    match r {
        Reason::StaleSource { .. } => "StaleSource",
        Reason::OutOfBounds { .. } => "OutOfBounds",
        Reason::InsideScalar { .. } => "InsideScalar",
        Reason::Reversed { .. } => "Reversed",
        Reason::UndeclaredUnits { .. } => "UndeclaredUnits",
        Reason::NotPinned { .. } => "NotPinned",
        Reason::OverlappingEdits { .. } => "OverlappingEdits",
        Reason::Encoding { .. } => "Encoding",
        Reason::Syntax => "Syntax",
        Reason::RecoveryGap => "RecoveryGap",
        Reason::EditionUnsupported { .. } => "EditionUnsupported",
        Reason::Check { .. } => "Check",
        Reason::Lowering { .. } => "Lowering",
        Reason::DependencyMissing { .. } => "DependencyMissing",
        Reason::IdentityMismatch { .. } => "IdentityMismatch",
        Reason::ObligationUnmet { .. } => "ObligationUnmet",
        Reason::RoundTripUndeclared { .. } => "RoundTripUndeclared",
        Reason::ValueType { .. } => "ValueType",
    }
}

/// CT-EV1-07 (refusals): each refusal names the service that refused (the event), a typed reason and a typed next
/// action, has form `refusal`, no fabricated span, and the category's retry; the host's step follows from that data.
#[test]
fn ct_ev1_07_every_refusal_carries_event_reason_and_next_action() {
    let all = refusals();
    let kinds: std::collections::BTreeSet<&str> = all.iter().map(|(_, _, _, k, _)| *k).collect();
    assert_eq!(kinds.len(), 10, "every refusal reason the services raise: {kinds:?}");
    for (label, e, service, kind, step) in &all {
        assert_eq!(e.form, Form::Refusal, "{label}");
        assert_eq!(e.service, *service, "{label}");
        assert_eq!(reason_kind(&e.reason), *kind, "{label}: {e:?}");
        assert!(e.span.is_none(), "{label}");
        assert!(e.recoverable(), "{label}");
        let retry = if e.category() == "stale" { Retry::AfterRefresh } else { Retry::AfterUserChange };
        assert_eq!(e.retry(), retry, "{label}");
        assert_eq!(&handle(e), step, "{label}");
        assert!(!e.summary.is_empty(), "{label}: a summary exists for display");
    }
}

/// CT-EV1-07 (no English text): the same refusals with their summaries replaced by other text, or removed, are
/// handled the same way, and every typed field is unchanged.
#[test]
fn ct_ev1_07_handling_does_not_depend_on_text() {
    for (label, e, _, _, step) in refusals() {
        for replacement in ["", "Résumé: ne rien faire", "retry automatically now"] {
            let mut changed = e.clone();
            changed.summary = replacement.to_owned();
            assert_eq!(handle(&changed), step, "{label}");
            assert_eq!(
                (changed.code, changed.form, changed.service, &changed.reason, &changed.next, &changed.span),
                (e.code, e.form, e.service, &e.reason, &e.next, &e.span),
                "{label}"
            );
        }
    }
}

/// CT-EV1-07 (diagnostics): diagnostics carry the same typed data: a syntax gap, a missing dependency, an identity
/// mismatch and a checker finding each have their reason and next action.
#[test]
fn ct_ev1_07_diagnostics_carry_typed_next_actions() {
    let broken = tool("fn f(x: Nat) -> Nat effects [] {\n    return\n");
    let doc = open(&broken);
    let d = diagnose(&doc, doc.digest(), &ModuleContext::new()).unwrap();
    assert!(d.diagnostics.iter().all(|e| e.form == Form::Diagnostic && e.next == NextAction::EditSource));
    assert!(d.diagnostics.iter().any(|e| e.reason == Reason::RecoveryGap));

    let missing = "a".repeat(64);
    let importing = importer(&missing, "fn f(n: Nat) -> Nat effects [] {\n    return n;\n}\n");
    let doc = open(&importing);
    let context = ModuleContext::new().with_session(full(), vec![jumble("a")]);
    let d = diagnose(&doc, doc.digest(), &context).unwrap();
    let steps: Vec<Step> = d.diagnostics.iter().map(handle).collect();
    assert_eq!(steps, [Step::AddDependency(missing.clone()), Step::PickContext]);
    assert_eq!(d.diagnostics[0].reason, Reason::DependencyMissing { alias: "lib".into(), digest: missing });
}
