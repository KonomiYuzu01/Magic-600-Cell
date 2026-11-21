//! Lists, word operations, sets, maps and compressed words in source (LP2 drafts until the IR freeze, PHS.49).
//! Candidate A calls them like functions: `list<T>(...)`, `length`, `index`, `push`, `concat`, `inverse`, `conjugate`,
//! `commutator`, `compose`, `word_key`, `set<T>(...)`, `map<K, V>(...)`, `contains`, `insert`, `lookup`, `elements`,
//! `keys` and `entries` with a named order, `ascending` or `descending`, and `compress` and `expand`. They lower to the
//! IR's collection form and primitives, and the checker applies LG2.13, LG2.15, LG2.16, LG2.19, LG2.20 and LG4.3 to
//! them. The run-time halves are `c600-run`'s `collections.rs`; the key's CT-LG2-11 and CT-LG2-13 halves are in
//! `conformance_lg2.rs`.

use c600_check::{check, Report, EXACTNESS_VIOLATION, TYPE_MISMATCH};
use c600_ir::{Expr as Ir, Order, Prim, QTy, Ty};
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
/// collection index; not run: form 2, LG2.21; sets and maps are `ct_lg2_05_set_elements_and_map_keys_are_exact`): a
/// list with a display element is display-qualified as a whole, and its length, an element or an equality cannot
/// extract an exact result; an index is an exact input.
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
/// word domain, the draft's `Word` of literal signed generators, the empty word included, and give a `Word`;
/// validation is the kernel's, the jumbling fixtures under LG2.14's four hypotheses are `c600-jumble-tool`'s `lb3.rs`
/// and `c600-kernel`'s `load.rs`, and the run-time totality half is `c600-run`'s `collections.rs`): anything else is
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

fn exact(ty: Ty) -> Box<QTy> {
    Box::new(QTy { ty, display: false })
}

/// Sets and maps lower to the IR's collection form with `Set<T>` and `Map<K, V>` types and one primitive per
/// operation; a listing carries its named order in its primitive (LG4.3).
#[test]
fn sets_and_maps_lower_to_the_draft_forms() {
    let source = "fn f(s: Set<Nat>, m: Map<Text, Word>) -> List<Nat> effects [] {\n    \
                  let a = contains(insert(s, 4), 4);\n    \
                  let b = contains(insert(m, \"x\", word[+1]), \"x\");\n    \
                  let c = lookup(m, \"x\");\n    \
                  let d = keys(m, descending);\n    \
                  let e = entries(m, ascending);\n    \
                  let g = exact_add(length(set<Nat>(1, 2)), length(map<Text, Word>(\"y\", word[])));\n    \
                  return elements(s, ascending);\n}\n";
    let lowered = lowered(source).unwrap_or_else(|e| panic!("{e}"));
    let f = &lowered.program.modules[0].functions[0];
    assert_eq!(f.params[0].ty.ty, Ty::Set(exact(Ty::Nat)));
    assert_eq!(f.params[1].ty.ty, Ty::Map(exact(Ty::Text), exact(Ty::Word)));
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
        Prim::Contains,
        Prim::Elements(Order::Ascending),
        Prim::Entries(Order::Ascending),
        Prim::ExactAdd,
        Prim::Insert,
        Prim::InsertEntry,
        Prim::Keys(Order::Descending),
        Prim::Length,
        Prim::Lookup,
    ];
    assert_eq!(ops, want);
    let collections: Vec<(&Ty, usize)> = all
        .iter()
        .filter_map(|e| match e {
            Ir::Collection { ty, items } => Some((ty, items.len())),
            _ => None,
        })
        .collect();
    assert_eq!(collections.len(), 2, "{collections:?}");
    assert!(collections.contains(&(&Ty::Set(exact(Ty::Nat)), 2)), "{collections:?}");
    assert!(collections.contains(&(&Ty::Map(exact(Ty::Text), exact(Ty::Word)), 2)), "{collections:?}");
    admitted(source);
}

/// The draft's refusals for sets and maps in source: `set` and `map` name their types, `Set` takes one type argument
/// and `Map` two, a set element and a map key are exact in their type (LG2.20), a collection holds no function, a map
/// takes its keys and values in pairs, a listing names its order (LG4.3), and the names are reserved.
#[test]
fn malformed_sets_and_maps_do_not_lower() {
    let refused = [
        ("fn f() -> Set<Nat> effects [] {\n    return set(1);\n}\n", "one type argument"),
        ("fn f() -> Map<Nat, Nat> effects [] {\n    return map<Nat>(1, 2);\n}\n", "two type arguments"),
        ("fn f(m: Map<Nat>) -> Nat effects [] {\n    return 1;\n}\n", "two type arguments"),
        ("fn f(s: Set<Nat, Nat>) -> Nat effects [] {\n    return 1;\n}\n", "one type argument"),
        ("fn f() -> Map<Nat, Nat> effects [] {\n    return map<Nat, Nat>(1, 2, 3);\n}\n", "in pairs"),
        ("fn f(s: Set<Display<Nat>>) -> Nat effects [] {\n    return 1;\n}\n", "LG2.20"),
        ("fn f(m: Map<Approx<P53>, Nat>) -> Nat effects [] {\n    return 1;\n}\n", "LG2.20"),
        ("fn f(s: Set<Receipt>) -> Nat effects [] {\n    return 1;\n}\n", "LG2.20"),
        ("fn f<T>(s: Set<T>) -> Nat effects [] {\n    return 1;\n}\n", "LG2.20"),
        ("fn f(s: Set<List<Display<Nat>>>) -> Nat effects [] {\n    return 1;\n}\n", "LG2.20"),
        ("fn f() -> Set<Unit> effects [] {\n    return set<Unit>();\n}\n", "LG2.20"),
        ("fn f(m: Map<Nat, fn(Nat) -> Nat effects []>) -> Nat effects [] {\n    return 1;\n}\n", "no function"),
        ("fn f(s: Set<Nat>) -> List<Nat> effects [] {\n    return elements(s);\n}\n", "elements"),
        ("fn f(s: Set<Nat>) -> List<Nat> effects [] {\n    return elements(s, sideways);\n}\n", "names its order"),
        ("fn f(m: Map<Nat, Nat>, o: Nat) -> List<Nat> effects [] {\n    return keys(m, o);\n}\n", "names its order"),
        ("fn f() -> Nat effects [] {\n    let ascending = 1;\n    return ascending;\n}\n", "ascending"),
        ("fn keys(s: Set<Nat>) -> Nat effects [] {\n    return 1;\n}\n", "keys"),
    ];
    for (declaration, says) in refused {
        let message = match lowered(declaration) {
            Ok(_) => panic!("lowered: {declaration}"),
            Err(e) => e,
        };
        assert!(message.contains(says), "{declaration}: {message}");
    }
    // A map's value need not be a key: it may be display, approximate or a record of either.
    admitted("fn f(c: Display<Colour>) -> Map<Nat, Display<Colour>> effects [] {\n    return map<Nat, Display<Colour>>(1, c);\n}\n");
}

/// CT-LG2-05 (part: aggregates under form 1 on the LP2 draft's sets and maps, LG2.19's container row and LG2.20's
/// exact collection index; not run: form 2, LG2.21): a set element and a map key are exact inputs, so that their key
/// order is defined, and a display key is refused where it is written, looked up or inserted; a map with a display
/// value is display-qualified as a whole, and its length, its keys, a membership or a looked-up value cannot extract
/// an exact result from it.
#[test]
fn ct_lg2_05_set_elements_and_map_keys_are_exact() {
    // Exact sets and maps give exact memberships, values, lengths and listings.
    admitted("fn f(s: Set<Nat>, x: Nat) -> Bool effects [] {\n    return contains(insert(s, x), x);\n}\n");
    admitted("fn f(m: Map<Text, Nat>) -> Nat effects [] {\n    return exact_add(lookup(m, \"a\"), length(m));\n}\n");
    admitted(
        "fn f(m: Map<Text, Nat>) -> List<Text> effects [] {\n    return keys(insert(m, \"b\", 2), ascending);\n}\n",
    );
    // A display key is refused as an element, a probe or an insertion.
    for declaration in [
        "fn f(c: Display<Nat>) -> Display<Set<Nat>> effects [] {\n    return set<Nat>(c);\n}\n",
        "fn f(c: Display<Nat>) -> Display<Map<Nat, Nat>> effects [] {\n    return map<Nat, Nat>(c, 1);\n}\n",
        "fn f(s: Set<Nat>, c: Display<Nat>) -> Display<Bool> effects [] {\n    return contains(s, c);\n}\n",
        "fn f(m: Map<Nat, Nat>, c: Display<Nat>) -> Display<Nat> effects [] {\n    return lookup(m, c);\n}\n",
        "fn f(s: Set<Nat>, c: Display<Nat>) -> Display<Set<Nat>> effects [] {\n    return insert(s, c);\n}\n",
    ] {
        assert_eq!(codes(declaration), [EXACTNESS_VIOLATION], "{declaration}");
    }
    // A display value makes the whole map display, whether the map's type names it or not.
    let shown = "fn f(c: Display<Colour>) -> Map<Nat, Colour> effects [] {\n    return map<Nat, Colour>(1, c);\n}\n";
    assert_eq!(codes(shown), [EXACTNESS_VIOLATION]);
    admitted(
        "fn f(c: Display<Colour>) -> Display<Map<Nat, Colour>> effects [] {\n    return map<Nat, Colour>(1, c);\n}\n",
    );
    // Its length, its keys, a membership or a looked-up value cannot extract an exact result.
    for declaration in [
        "fn f(m: Map<Nat, Display<Colour>>) -> Nat effects [] {\n    return length(m);\n}\n",
        "fn f(m: Map<Nat, Display<Colour>>) -> List<Nat> effects [] {\n    return keys(m, ascending);\n}\n",
        "fn f(m: Map<Nat, Display<Colour>>) -> Bool effects [] {\n    return contains(m, 1);\n}\n",
        "fn f(m: Display<Map<Nat, Colour>>) -> Colour effects [] {\n    return lookup(m, 1);\n}\n",
    ] {
        assert_eq!(codes(declaration), [EXACTNESS_VIOLATION], "{declaration}");
    }
    admitted("fn f(m: Map<Nat, Display<Colour>>) -> Display<Nat> effects [] {\n    return length(m);\n}\n");
    // A key must fit the key type, and a listing the collection it lists.
    let (code, types) = only("fn f(m: Map<Nat, Text>) -> Text effects [] {\n    return lookup(m, \"1\");\n}\n");
    assert_eq!((code, types), (TYPE_MISMATCH, Some(("Nat".into(), "Text".into()))));
    let (code, types) = only("fn f(s: Set<Nat>) -> List<Nat> effects [] {\n    return keys(s, ascending);\n}\n");
    assert_eq!((code, types), (TYPE_MISMATCH, Some(("Map<_, _>".into(), "Set<Nat>".into()))));
}

/// CT-LG2-14 (part, the language half's static rules: `CompressedWord` is a type of its own, `compress` and `expand`
/// lower to their primitives, and the checker gives a compressed word an exact `Nat` length and joins two without
/// expansion; not run: the run-time half, `c600-run`'s `collections.rs`, and the streaming, CT-LG4-03's
/// `streams.rs`): a compressed word is never a `Word`, has no exact equality and is no key (LP2 draft), and an expansion takes
/// exact natural positions.
#[test]
fn ct_lg2_14_a_compressed_word_is_a_type_of_its_own() {
    let source = "fn f(w: Word, c: CompressedWord) -> record { n: Nat, w: Word } effects [] {\n    \
                  let d = concat(compress(w), c);\n    return record { n = length(d), w = expand(d, 0, 1) };\n}\n";
    let joined = lowered(source).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(joined.program.modules[0].functions[0].params[1].ty.ty, Ty::CompressedWord);
    let mut ops: Vec<Prim> = expressions(&joined)
        .iter()
        .filter_map(|e| match e {
            Ir::Prim { op, .. } => Some(*op),
            _ => None,
        })
        .collect();
    ops.sort_by_key(|op| format!("{op:?}"));
    assert_eq!(ops, [Prim::Compress, Prim::Concat, Prim::Expand, Prim::Length]);
    admitted(source);
    // A compressed word is not a word, and a word is not a compressed word.
    for (declaration, expected, supplied) in [
        ("fn f(c: CompressedWord) -> Word effects [] {\n    return inverse(c);\n}\n", "Word", "CompressedWord"),
        ("fn f(w: Word) -> Word effects [] {\n    return expand(w, 0, 1);\n}\n", "CompressedWord", "Word"),
        (
            "fn f(c: CompressedWord) -> CompressedWord effects [] {\n    return compress(c);\n}\n",
            "Word",
            "CompressedWord",
        ),
        ("fn f(c: CompressedWord) -> Word effects [] {\n    return c;\n}\n", "Word", "CompressedWord"),
    ] {
        let (code, types) = only(declaration);
        assert_eq!((code, types), (TYPE_MISMATCH, Some((expected.into(), supplied.into()))), "{declaration}");
    }
    let (code, _) = only("fn f(w: Word, c: CompressedWord) -> Word effects [] {\n    return concat(w, c);\n}\n");
    assert_eq!(code, TYPE_MISMATCH);
    // No exact equality, alone or held in a list or a record.
    for operand in ["CompressedWord", "List<CompressedWord>", "record { c: CompressedWord }"] {
        let declaration = format!("fn f(c: {operand}) -> Bool effects [] {{\n    return exact_eq(c, c);\n}}\n");
        let (code, types) = only(&declaration);
        assert_eq!(code, TYPE_MISMATCH, "{declaration}");
        assert_eq!(types.map(|(e, _)| e), Some("a type without CompressedWord".to_string()), "{declaration}");
    }
    // An expansion's position and count are exact natural numbers (LG2.20).
    let (code, types) = only("fn f(c: CompressedWord) -> Word effects [] {\n    return expand(c, \"0\", 1);\n}\n");
    assert_eq!((code, types), (TYPE_MISMATCH, Some(("Nat".into(), "Text".into()))));
    let shown = "fn f(c: CompressedWord, i: Display<Nat>) -> Display<Word> effects [] {\n    \
                 return expand(c, 0, i);\n}\n";
    assert_eq!(codes(shown), [EXACTNESS_VIOLATION]);
    // It is no set element or map key.
    for declaration in [
        "fn f(s: Set<CompressedWord>) -> Nat effects [] {\n    return 1;\n}\n",
        "fn f(m: Map<CompressedWord, Nat>) -> Nat effects [] {\n    return 1;\n}\n",
        "fn f(c: CompressedWord) -> Nat effects [] {\n    return length(set<CompressedWord>(c));\n}\n",
    ] {
        let message = match lowered(declaration) {
            Ok(_) => panic!("lowered: {declaration}"),
            Err(e) => e,
        };
        assert!(message.contains("compressed word (LG2.20)"), "{declaration}: {message}");
    }
}
