//! Completion contexts beyond CT-EV1-01's (EV1.3; no CT ID of its own): each kind of position gets the candidates of
//! its kind, a suggestion carries its replace range and declared data, and missing context stays explicit.

mod support;

use c600_editor::{complete, CandidateKind, Completion, CompletionContext, Missing, ModuleContext};
use support::{at, importer, open, text, tool, LIB};

fn at_offset(source: &str, offset: usize, context: &ModuleContext) -> Completion {
    let doc = open(source);
    complete(&doc, doc.digest(), offset, context).unwrap()
}

fn labels(c: &Completion) -> Vec<&str> {
    c.candidates.iter().map(|c| c.label.as_str()).collect()
}

const SOURCE: &str = "fn f<T>(r: record { a: Nat, b: Text }, t: T) -> Nat effects [snapshot] {\n    \
                      # a comment\n    let s = \"text\";\n    return r.a;\n}\n";

#[test]
fn completion_contexts_follow_the_position() {
    let source = tool(SOURCE);
    let context = ModuleContext::new();
    let c = |needle: &str, shift: usize| at_offset(&source, at(&source, needle, 0, shift), &context);

    assert_eq!(c("# a comment", 3).context, CompletionContext::Comment);
    assert!(c("# a comment", 3).candidates.is_empty());
    assert_eq!(c("\"text\"", 2).context, CompletionContext::Text);
    assert_eq!(c("Nat, b", 0).context, CompletionContext::Type);
    let types = c("Nat, b", 1);
    assert_eq!(labels(&types), ["Nat"]);
    assert_eq!(text(&source, &types.replace), "Nat");
    assert!(c("T) ->", 0)
        .candidates
        .iter()
        .any(|c| c.label == "T" && c.kind == CandidateKind::Local(c600_editor::NameKind::Generic)));
    let effects = c("snapshot]", 4);
    assert_eq!(effects.context, CompletionContext::Effects);
    assert_eq!(labels(&effects), ["snapshot"]);
    let fields = c("r.a;", 2);
    assert_eq!(fields.context, CompletionContext::Field);
    assert_eq!(labels(&fields), ["a", "b"], "the position is before `a`, so the prefix is empty");
    let fields = c("r.a;", 3);
    assert_eq!(labels(&fields), ["a"]);
    assert_eq!(fields.candidates[0].ty.as_deref(), Some("Nat"));
    assert_eq!(c("let s", 4).context, CompletionContext::NewName);
    assert_eq!(c("module tool requires [", "module tool requires [".len()).context, CompletionContext::Capabilities);
    assert_eq!(at_offset(&source, source.len(), &context).context, CompletionContext::Declaration);
    let decl = at_offset(&format!("{source}\nco"), source.len() + 3, &context);
    assert_eq!(labels(&decl), ["command", "contract"]);
    let statement = c("return r.a", 0);
    assert!(labels(&statement).contains(&"return") && labels(&statement).contains(&"let"));
}

#[test]
fn member_completion_reads_the_pinned_dependency_or_says_it_is_missing() {
    let mut context = ModuleContext::new();
    let lib = context.add_dependency(LIB.as_bytes().to_vec(), None);
    let source = importer(&lib, "fn f(n: Nat) -> Nat effects [] {\n    return lib.d(n);\n}\n");
    let member = at(&source, "lib.d", 0, 5);
    let c = at_offset(&source, member, &context);
    assert_eq!(c.context, CompletionContext::Member { alias: "lib".into(), digest: lib.clone() });
    assert_eq!(labels(&c), ["double"]);
    assert_eq!(c.candidates[0].kind, CandidateKind::Member);
    assert_eq!(c.candidates[0].identity.as_ref().map(|i| i.digest.as_str()), Some(lib.as_str()));
    let all = at_offset(&source.replace("lib.d(", "lib.("), member - 1, &context);
    assert_eq!(labels(&all), ["double", "twice"], "only exported declarations, never `helper`");

    let c = at_offset(&source, member, &ModuleContext::new());
    assert!(c.candidates.is_empty());
    assert_eq!(c.missing, [Missing::Dependency { alias: "lib".into(), digest: lib }]);
}
