//! CT-EV1-01 (whole; source): EV1.1 and EV1.2. An incomplete declaration yields recovered spans and completion
//! context but no runnable module: parse returns the recovered structure and its diagnostics, completion reads the
//! recovered scope inside the broken declaration, diagnose gives no checked verdict, lowering refuses the draft, and
//! every response names exactly the input it answers.

mod support;

use c600_editor::{
    code, complete, diagnose, parse, CandidateKind, CompletionContext, Document, ModuleContext, NameKind, OutlineKind,
    Reason, Service, Stage, Units, Verdict,
};
use c600_syntax::ast::{BindingKind, DeclKind};
use support::{at, open, text, tool};

/// A complete helper, then a command broken in its body: a `let` that parsed, and a `return` cut off mid-name.
fn draft() -> String {
    tool(
        "fn helper(a: Nat) -> Nat effects [] {\n    return a;\n}\n\n\
         command go(w: Word, n: Nat) -> Nat effects [] {\n    let m = helper(n);\n    return he",
    )
}

/// CT-EV1-01 (EV1.2, recovered spans): parse returns the complete declaration, a recovery hole for the broken one
/// with what was read of it (kind, name, parameters with their types, the `let`), and the gap's diagnostic at the
/// hole's range under the source digest.
#[test]
fn ct_ev1_01_an_incomplete_declaration_has_recovered_spans() {
    let source = draft();
    let doc = open(&source);
    let parsed = parse(&doc, c600_syntax::EDITION);
    assert!(!parsed.is_complete());
    assert_eq!(parsed.file.declarations().map(|d| d.name.text.as_str()).collect::<Vec<_>>(), ["helper"]);
    assert_eq!(parsed.file.holes.len(), 1);
    let hole = parsed.file.holes[0];
    assert_eq!(&source[hole.start..hole.end], &source[at(&source, "command go", 0, 0)..]);

    let recovered = &parsed.file.recovered[0];
    assert_eq!(recovered.kind, Some(DeclKind::Command));
    assert_eq!(recovered.name.as_ref().map(|n| n.text.as_str()), Some("go"));
    let bindings: Vec<(&str, BindingKind)> =
        recovered.bindings.iter().map(|b| (b.name.text.as_str(), b.kind)).collect();
    assert_eq!(bindings, [("w", BindingKind::Parameter), ("n", BindingKind::Parameter), ("m", BindingKind::Let)]);
    let m = &recovered.bindings[2];
    assert_eq!(m.from, at(&source, "helper(n);", 0, "helper(n);".len()), "a let is visible after its `;`");
    assert_eq!(m.to, hole.end, "an unclosed scope ends at the hole's end");
    assert!(recovered.bindings[0].ty.is_some(), "a parameter keeps its declared type");

    let codes: Vec<&str> = parsed.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, [code::UNEXPECTED_TOKEN, code::RECOVERY_GAP]);
    let gap = &parsed.diagnostics[1];
    assert_eq!(gap.reason, Reason::RecoveryGap);
    assert_eq!(gap.service, Service::Parse);
    let span = gap.span.as_ref().unwrap();
    assert_eq!((span.digest.as_str(), span.start, span.end), (doc.digest(), hole.start, hole.end));

    let outline: Vec<(OutlineKind, Option<&str>)> =
        parsed.outline.iter().map(|o| (o.kind, o.name.as_deref())).collect();
    assert_eq!(outline, [(OutlineKind::Fn, Some("helper")), (OutlineKind::Hole, Some("go"))]);
    assert_eq!(parsed.outline[1].recovered_kind, Some(OutlineKind::Command));
}

/// CT-EV1-01 (EV1.2, completion context): completion inside the broken declaration reads its recovered scope: the
/// parameters and the `let` that parsed, the module's functions and the reserved names, filtered by the prefix, with
/// the identifier as the replace range. The `let` is not visible inside its own value.
#[test]
fn ct_ev1_01_completion_reads_the_recovered_scope() {
    let source = draft();
    let doc = open(&source);
    let context = ModuleContext::new();
    let end = source.len();
    let c = complete(&doc, doc.digest(), end, &context).unwrap();
    assert_eq!(c.context, CompletionContext::Expression);
    assert_eq!(c.prefix, "he");
    assert_eq!(text(&source, &c.replace), "he");
    let labels: Vec<&str> = c.candidates.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["helper"]);
    assert_eq!(c.candidates[0].kind, CandidateKind::Declaration);
    assert_eq!(c.candidates[0].ty.as_deref(), Some("fn helper(a: Nat) -> Nat effects []"));

    // With an empty prefix after `return `, the recovered locals are candidates.
    let edited = source.replace("return he", "return ");
    let doc = open(&edited);
    let c = complete(&doc, doc.digest(), edited.len(), &context).unwrap();
    let locals: Vec<(&str, CandidateKind)> = c
        .candidates
        .iter()
        .filter(|c| matches!(c.kind, CandidateKind::Local(_) | CandidateKind::Declaration))
        .map(|c| (c.label.as_str(), c.kind))
        .collect();
    assert_eq!(
        locals,
        [
            ("go", CandidateKind::Declaration),
            ("helper", CandidateKind::Declaration),
            ("m", CandidateKind::Local(NameKind::Let)),
            ("n", CandidateKind::Local(NameKind::Parameter)),
            ("w", CandidateKind::Local(NameKind::Parameter)),
        ]
    );
    assert_eq!(c.candidates.iter().find(|c| c.label == "w").and_then(|c| c.ty.as_deref()), Some("Word"));
    assert!(c.replace.is_insertion());
    assert!(c.candidates.iter().any(|c| c.label == "read_snapshot" && c.kind == CandidateKind::Builtin));

    // Inside the `let`'s own value, `m` is not visible yet.
    let inside = at(&source, "helper(n)", 0, "helper(".len());
    let doc = open(&source);
    let c = complete(&doc, doc.digest(), inside, &context).unwrap();
    assert!(c.candidates.iter().any(|c| c.label == "n"));
    assert!(!c.candidates.iter().any(|c| c.label == "m"), "{:?}", c.candidates);
}

/// CT-EV1-01 (EV1.2, no runnable module): diagnose gives no checked verdict and no candidate for activation, the
/// product lowering refuses the draft, and a complete draft of the same declaration checks.
#[test]
fn ct_ev1_01_an_incomplete_declaration_gives_no_runnable_module() {
    let source = draft();
    let doc = open(&source);
    let context = ModuleContext::new();
    let d = diagnose(&doc, doc.digest(), &context).unwrap();
    assert_eq!(d.verdict, Verdict::NotChecked(Stage::Syntax));
    assert!(d.diagnostics.iter().any(|e| e.code == code::RECOVERY_GAP));
    let file = c600_syntax::parse(source.as_bytes());
    assert!(c600_syntax::lower(&[file]).is_err(), "a draft with a recovery hole does not lower");

    let complete_source = source.replace("return he", "return helper(m);\n}\n");
    let doc = open(&complete_source);
    let d = diagnose(&doc, doc.digest(), &context).unwrap();
    assert!(matches!(d.verdict, Verdict::Checked(_)), "{:?}", d.diagnostics);
}

/// CT-EV1-01 (EV1.1): every response names its input, the source digest, the edition and the context's digest; a
/// response is never attached to other bytes, and a request built against older bytes is refused as stale.
#[test]
fn ct_ev1_01_responses_name_their_input() {
    let source = draft();
    let older = open(&source);
    let newer = open(&source.replace("return he", "return h"));
    let context = ModuleContext::new();

    let parsed = parse(&older, c600_syntax::EDITION);
    assert!(parsed.subject.names(&older) && !parsed.subject.names(&newer));
    assert_eq!(parsed.subject.edition, c600_syntax::EDITION);
    let c = complete(&older, older.digest(), source.len(), &context).unwrap();
    assert_eq!(c.subject.source, older.digest());
    assert_eq!(c.subject.context.as_deref(), Some(context.digest().as_str()));
    assert!(c.candidates.iter().all(|c| c.replace.digest == older.digest()));

    let refused = complete(&newer, older.digest(), 0, &context).unwrap_err();
    assert_eq!(refused.code, code::STALE_EDITOR_SOURCE);
    let refused = diagnose(&newer, older.digest(), &context).unwrap_err();
    assert_eq!(refused.code, code::STALE_EDITOR_SOURCE);

    // A digest names bytes, not a file: the same bytes opened twice are one input.
    let again = Document::open(source.as_bytes().to_vec(), Units::Utf16);
    assert!(parsed.subject.names(&again));
}
