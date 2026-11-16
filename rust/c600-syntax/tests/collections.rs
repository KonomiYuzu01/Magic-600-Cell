//! Lists and word operations in source (LP2 drafts until the IR freeze, PHS.49). Candidate A calls them like
//! functions: `list<T>(...)`, `length`, `index`, `push`, `concat`, `inverse`, `conjugate`, `commutator`, `compose` and
//! `word_key`. They lower to the IR's collection form and primitives, and the checker applies LG2.13, LG2.15, LG2.19
//! and LG2.20 to them. The run-time halves are `c600-run`'s `collections.rs`; the key's CT-LG2-11 and CT-LG2-13 halves
//! are in `conformance_lg2.rs`.

use c600_check::{check, Report, EXACTNESS_VIOLATION, TYPE_MISMATCH};
use c600_ir::{Expr as Ir, Prim, Ty};
use c600_syntax::{lower, parse, Lowered};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

fn lowered(declarations: &str) -> Result<Lowered, String> {
    let source = format!("{HEADER}{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    lower(&[file]).map_err(|e| e.to_string())
}

fn report(declarations: &str) -> Report {
    let lowered = lowered(declarations).unwrap_or_else(|e| panic!("{declarations}: {e}"));
    check(&lowered.program).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

fn admitted(declarations: &str) {
    let r = report(declarations);
    assert!(r.is_admitted(), "{declarations}: {:#?}", r.diagnostics);
}

fn codes(declarations: &str) -> Vec<&'static str> {
    report(declarations).diagnostics.iter().map(|d| d.code).collect()
}

/// The only finding's code, and the expected and supplied types of a mismatch.
fn only(declarations: &str) -> (&'static str, Option<(String, String)>) {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    (d.code, d.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone())))
}

/// Every expression of every function, depth first.
fn expressions(lowered: &Lowered) -> Vec<&Ir> {
    let mut out = Vec::new();
    let mut pending: Vec<&Ir> = lowered.program.modules.iter().flat_map(|m| &m.functions).map(|f| &f.body).collect();
    while let Some(e) = pending.pop() {
        out.push(e);
        pending.extend(e.children());
    }
    out
}

/// The draft's surface lowers to one collection form and one primitive per operation, with `List<T>` and
/// `WordEqualityKey` as types.
#[test]
fn lists_and_word_operations_lower_to_the_draft_forms() {
    let source = "fn f(xs: List<Word>, w: Word, k: WordEqualityKey) -> List<Nat> effects [] {\n    \
                  let a = length(push(concat(xs, list<Word>(w)), inverse(w)));\n    \
                  let b = length(compose(conjugate(w, w), commutator(w, w)));\n    \
                  let c = exact_eq(word_key(w), k);\n    return list<Nat>(a, index(list<Nat>(b), 0));\n}\n";
    let lowered = lowered(source).unwrap_or_else(|e| panic!("{e}"));
    let f = &lowered.program.modules[0].functions[0];
    assert_eq!(f.params[0].ty.ty, Ty::List(Box::new(c600_ir::QTy { ty: Ty::Word, display: false })));
    assert_eq!(f.params[2].ty.ty, Ty::WordEqualityKey);
    let all = expressions(&lowered);
    let mut ops: Vec<Prim> = all
        .iter()
        .filter_map(|e| match e {
            Ir::Prim { op, .. } => Some(*op),
            _ => None,
        })
        .collect();
    ops.sort_by_key(|op| format!("{op:?}"));
    ops.dedup();
    let want = [
        Prim::Commutator,
        Prim::Compose,
        Prim::Concat,
        Prim::Conjugate,
        Prim::ExactEq,
        Prim::Index,
        Prim::Inverse,
        Prim::Length,
        Prim::Push,
        Prim::WordKey,
    ];
    assert_eq!(ops, want);
    let lists: Vec<(&Ty, usize)> = all
        .iter()
        .filter_map(|e| match e {
            Ir::Collection { ty, items } => Some((ty, items.len())),
            _ => None,
        })
        .collect();
    assert!(lists.iter().all(|(ty, _)| matches!(ty, Ty::List(_))), "{lists:?}");
    let mut sizes: Vec<usize> = lists.iter().map(|(_, n)| *n).collect();
    sizes.sort();
    assert_eq!(sizes, [1, 1, 2], "{lists:?}");
    admitted(source);
}

/// The draft's refusals in source: `list` names its element type, `List` takes one type argument and holds no
/// function, the operations take their arity, and their names are reserved.
#[test]
fn malformed_lists_and_word_operations_do_not_lower() {
    let refused = [
        ("fn f() -> List<Nat> effects [] {\n    return list(1);\n}\n", "one type argument"),
        ("fn f(xs: List<Nat, Nat>) -> Nat effects [] {\n    return 1;\n}\n", "one type argument"),
        ("fn f(xs: List) -> Nat effects [] {\n    return 1;\n}\n", "one type argument"),
        ("fn f(xs: List<fn(Nat) -> Nat effects []>) -> Nat effects [] {\n    return 1;\n}\n", "no function"),
        ("fn f(w: Word) -> Word effects [] {\n    return inverse(w, w);\n}\n", "inverse"),
        ("fn f(w: Word) -> Word effects [] {\n    return conjugate(w);\n}\n", "conjugate"),
        ("fn f(w: Word) -> Word effects [] {\n    return inverse<Word>(w);\n}\n", "no type argument"),
        ("fn f(w: Word) -> Word effects [] {\n    let length = w;\n    return length;\n}\n", "length"),
        ("fn word_key(w: Word) -> Word effects [] {\n    return w;\n}\n", "word_key"),
    ];
    for (declaration, says) in refused {
        let message = match lowered(declaration) {
            Ok(_) => panic!("lowered: {declaration}"),
            Err(e) => e,
        };
        assert!(message.contains(says), "{declaration}: {message}");
    }
}

/// CT-LG2-05 (part: aggregates under form 1 on the LP2 draft's lists, LG2.19's container row and LG2.20's exact
/// collection index; not run: maps and sets, which are not built, and form 2, LG2.21): a list with a display element
/// is display-qualified as a whole, and its length, an element or an equality cannot extract an exact result; an
/// index is an exact input.
#[test]
fn ct_lg2_05_a_list_keeps_the_display_qualifier_of_its_elements() {
    // Exact lists give exact lengths, elements and equalities.
    admitted("fn f(xs: List<Nat>, i: Nat) -> Nat effects [] {\n    return exact_add(length(xs), index(xs, i));\n}\n");
    admitted("fn f(xs: List<Nat>) -> Bool effects [] {\n    return exact_eq(xs, push(xs, 1));\n}\n");
    admitted("fn f() -> List<Word> effects [] {\n    return concat(list<Word>(word[+1]), list<Word>());\n}\n");
    // A display element makes the whole list display, whether the list's type names it or not.
    for declaration in [
        "fn f(c: Display<Colour>) -> List<Colour> effects [] {\n    return list<Colour>(c);\n}\n",
        "fn f(c: Display<Colour>) -> List<Colour> effects [] {\n    return push(list<Colour>(), c);\n}\n",
        "fn f(xs: List<Display<Nat>>) -> List<Nat> effects [] {\n    return concat(xs, list<Nat>(1));\n}\n",
    ] {
        assert_eq!(codes(declaration), [EXACTNESS_VIOLATION], "{declaration}");
    }
    admitted("fn f(c: Display<Colour>) -> Display<List<Colour>> effects [] {\n    return list<Colour>(c);\n}\n");
    admitted(
        "fn f(c: Display<Colour>) -> List<Display<Colour>> effects [] {\n    return list<Display<Colour>>(c);\n}\n",
    );
    // Length, indexing and equality cannot extract an exact result from it.
    for declaration in [
        "fn f(xs: List<Display<Nat>>) -> Nat effects [] {\n    return length(xs);\n}\n",
        "fn f(xs: List<Display<Nat>>) -> Nat effects [] {\n    return index(xs, 0);\n}\n",
        "fn f(c: Display<Colour>) -> Nat effects [] {\n    return length(list<Colour>(c));\n}\n",
        "fn f(xs: Display<List<Nat>>) -> Nat effects [] {\n    return index(xs, 0);\n}\n",
    ] {
        assert_eq!(codes(declaration), [EXACTNESS_VIOLATION], "{declaration}");
    }
    let equal = codes("fn f(xs: List<Display<Nat>>) -> Bool effects [] {\n    return exact_eq(xs, xs);\n}\n");
    assert!(!equal.is_empty() && equal.iter().all(|&c| c == EXACTNESS_VIOLATION), "{equal:?}");
    admitted("fn f(xs: List<Display<Nat>>) -> Display<Nat> effects [] {\n    return length(xs);\n}\n");
    // An exact collection index takes exact inputs only (LG2.20), and a natural number only.
    let display_index = "fn f(xs: List<Nat>, i: Display<Nat>) -> Display<Nat> effects [] {\n    \
                         return index(xs, i);\n}\n";
    assert_eq!(codes(display_index), [EXACTNESS_VIOLATION]);
    let (code, types) = only("fn f(xs: List<Nat>) -> Nat effects [] {\n    return index(xs, \"0\");\n}\n");
    assert_eq!((code, types), (TYPE_MISMATCH, Some(("Nat".into(), "Text".into()))));
    let (code, types) = only("fn f(xs: Word) -> Nat effects [] {\n    return index(xs, 0);\n}\n");
    assert_eq!((code, types), (TYPE_MISMATCH, Some(("List<_>".into(), "Word".into()))));
    // An element must fit the list's element type.
    let (code, types) = only("fn f() -> List<Nat> effects [] {\n    return list<Nat>(1, \"2\");\n}\n");
    assert_eq!((code, types), (TYPE_MISMATCH, Some(("Nat".into(), "Text".into()))));
}

/// CT-LG2-12 (part, the language half: R-11's inverse, conjugate, commutator and composition accept the validated
/// word domain, the draft's `Word` of literal signed generators, the empty word included, and give a `Word`; not run:
/// validation, which is the kernel's, and the jumbling fixtures under LG2.14's four hypotheses, which wait for the
/// jumbled puzzle's alphabet, M5; the run-time totality half is `c600-run`'s `collections.rs`): anything else is
/// refused, and a display word stays display, so it cannot become an exact word (LG2.19).
#[test]
fn ct_lg2_12_word_operations_accept_the_word_domain() {
    let ops = ["inverse(x)", "conjugate(x, y)", "commutator(x, y)", "compose(x, y)", "concat(x, y)"];
    for op in ops {
        admitted(&format!("fn f(x: Word, y: Word) -> Word effects [] {{\n    return {op};\n}}\n"));
        let empty = op.replace('x', "word[]").replace('y', "word[]");
        admitted(&format!("fn f() -> Word effects [] {{\n    return {empty};\n}}\n"));
        for (ty, supplied) in [("Nat", "Nat"), ("Text", "Text"), ("List<Word>", "List<Word>"), ("Target", "Target")] {
            let declaration = format!("fn f(x: {ty}, y: Word) -> Word effects [] {{\n    return {op};\n}}\n");
            if op == "concat(x, y)" && ty == "List<Word>" {
                continue;
            }
            let (code, types) = only(&declaration);
            assert_eq!(code, TYPE_MISMATCH, "{declaration}");
            assert_eq!(types.map(|(_, s)| s), Some(supplied.to_string()), "{declaration}");
        }
        let shown = format!("fn f(x: Display<Word>, y: Word) -> Word effects [] {{\n    return {op};\n}}\n");
        assert_eq!(codes(&shown), [EXACTNESS_VIOLATION], "{shown}");
        admitted(&format!("fn f(x: Display<Word>, y: Word) -> Display<Word> effects [] {{\n    return {op};\n}}\n"));
    }
    // A word's exact length is a natural number (LG2.16).
    admitted("fn f(w: Word) -> Nat effects [] {\n    return length(w);\n}\n");
    // Words and lists do not mix.
    let (code, _) = only("fn f(w: Word, xs: List<Word>) -> Word effects [] {\n    return concat(w, xs);\n}\n");
    assert_eq!(code, TYPE_MISMATCH);
}
