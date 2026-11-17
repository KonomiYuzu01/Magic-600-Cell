//! Menu words in source (LB1.8; LP2 draft until the IR freeze, PHS.49): `menu[cap:item, ...]` literals of the run's
//! pinned menu, each letter two naturals below 600 counted from 0, and the type `MenuWord`. They lower to the IR's
//! menu-word literal, and the checker types the word operations in the menu's alphabet and never mixes a Full word
//! with a menu word (`LB1:input.incompatible_alphabet`). The run-time halves, the pinned alphabet and the proposal to a
//! jumbling host, are `c600-run`'s `menu_words.rs`.

use c600_check::{check, Report, EFFECT_UNDECLARED, INCOMPATIBLE_ALPHABET, TYPE_MISMATCH};
use c600_ir::{Expr as Ir, Lit, Ty};
use c600_syntax::code::{INVALID_LITERAL, RECOVERY_GAP};
use c600_syntax::{format, lower, parse, Lowered, ParsedFile};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

fn source(declarations: &str) -> String {
    format!("{HEADER}{declarations}")
}

fn complete(text: &str) -> ParsedFile {
    let file = parse(text.as_bytes());
    assert!(file.diagnostics.is_empty(), "{text}\n{:#?}", file.diagnostics);
    file
}

fn lowered(declarations: &str) -> Result<Lowered, String> {
    lower(&[complete(&source(declarations))]).map_err(|e| e.to_string())
}

fn report(declarations: &str) -> Report {
    let lowered = lowered(declarations).unwrap_or_else(|e| panic!("{declarations}: {e}"));
    check(&lowered.program).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

fn admitted(declarations: &str) {
    let r = report(declarations);
    assert!(r.is_admitted(), "{declarations}: {:#?}", r.diagnostics);
}

/// The only finding's code, and the expected and supplied types of a mismatch.
fn only(declarations: &str) -> (&'static str, Option<(String, String)>) {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    (d.code, d.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone())))
}

/// The menu-word literals in the bodies of every function, depth first.
fn menu_literals(lowered: &Lowered) -> Vec<Vec<[u32; 2]>> {
    let mut out = Vec::new();
    let mut pending: Vec<&Ir> = lowered.program.modules.iter().flat_map(|m| &m.functions).map(|f| &f.body).collect();
    while let Some(e) = pending.pop() {
        if let Ir::Lit(Lit::MenuWord(letters)) = e {
            out.push(letters.clone());
        }
        pending.extend(e.children());
    }
    out
}

/// LB1.8 (LP2 draft): a menu literal lowers to its letters `[cap, item]` in written order, the empty menu word is a
/// word, and `MenuWord` is a type, also as an element of a collection.
#[test]
fn a_menu_literal_lowers_to_its_letters_in_written_order() {
    let declarations = "fn f(x: MenuWord) -> List<MenuWord> effects [] {\n    \
                        return list<MenuWord>(menu[2:5, 0:1], menu[], x, menu[599:599]);\n}\n";
    let lowered = lowered(declarations).unwrap_or_else(|e| panic!("{e}"));
    let f = &lowered.program.modules[0].functions[0];
    assert_eq!(f.params[0].ty.ty, Ty::MenuWord);
    let mut literals = menu_literals(&lowered);
    literals.sort();
    assert_eq!(literals, [vec![], vec![[2, 5], [0, 1]], vec![[599, 599]]]);
    admitted(declarations);
    admitted(
        "fn f(s: Set<MenuWord>, m: Map<MenuWord, Nat>, x: MenuWord) -> Bool effects [] {\n    \
              return contains(insert(s, x), menu[0:1]);\n}\n",
    );
}

/// LB1.8 (LP2 draft): the canonical layout writes each menu letter as `cap:item` in canonical decimals; the layout is
/// idempotent and lowers to the same program.
#[test]
fn a_menu_literal_has_a_canonical_layout() {
    let canonical = source("fn f() -> MenuWord effects [] {\n    return menu[2:5, 0:1];\n}\n");
    let written = source("fn f() -> MenuWord effects [] { return menu[ 002 : 005 ,0:01 ] ; }");
    let formatted = format(&complete(&written)).expect("a complete draft has a layout");
    assert_eq!(formatted, canonical);
    assert_eq!(format(&complete(&formatted)).as_deref(), Some(canonical.as_str()));
    let program = |text: &str| lower(&[complete(text)]).unwrap_or_else(|e| panic!("{e}")).program;
    assert_eq!(program(&written), program(&canonical));
}

/// LB1.8 (LP2 draft): a menu letter is two naturals below 600, or LX-8's fixture letter `grip:rotation`; anything
/// else is refused at its source text.
#[test]
fn a_menu_letter_out_of_range_is_refused_at_its_source_text() {
    for (literal, offending) in [
        ("menu[600:0]", "600:0"),
        ("menu[0:600]", "0:600"),
        ("menu[2:5, 1000:1]", "1000:1"),
        ("menu[0:99999999999]", "0:99999999999"),
        ("menu[120:spin]", "120:spin"),
        ("menu[2:turn]", "2:turn"),
    ] {
        let text = source(&format!("fn f() -> MenuWord effects [] {{\n    return {literal};\n}}\n"));
        let file = parse(text.as_bytes());
        let codes: Vec<&str> = file.diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, [INVALID_LITERAL, RECOVERY_GAP], "{literal}");
        let span = file.diagnostics[0].span.unwrap();
        assert_eq!(&text[span.start..span.end], offending, "{literal}");
    }
    let text = source("fn f() -> MenuWord effects [] {\n    return menu[-1:0];\n}\n");
    let file = parse(text.as_bytes());
    assert_eq!(file.diagnostics.first().map(|d| d.code), Some(INVALID_LITERAL), "{:#?}", file.diagnostics);
}

/// CT-LG2-08 (part, with `ct_lg2_08_fixture_menu_letters_and_modes_do_not_lower` in `conformance_lg2.rs`): LX-8's
/// fixture letters still parse, alone, among numeric letters or as a pattern, and do not lower; the refusal says what
/// a letter of the pinned menu is.
#[test]
fn fixture_menu_letters_parse_and_do_not_lower() {
    for literal in ["menu[2:spin]", "menu[2:5, 3:unspin]", "menu[119:identity]"] {
        let declarations = format!("fn f() -> MenuWord effects [] {{\n    return {literal};\n}}\n");
        let message = lowered(&declarations).err().unwrap_or_else(|| panic!("lowered: {literal}"));
        assert!(message.contains("cap:item"), "{literal}: {message}");
    }
    let pattern = "fn f(x: MenuWord) -> Nat effects [] {\n    return match x {\n        menu[2:spin] => 1;\n        \
                   _ => 2;\n    };\n}\n";
    let message = lowered(pattern).err().expect("a fixture pattern does not lower");
    assert!(message.contains("cap:item"), "{message}");
}

/// LB1.8 (LP2 draft): the word operations build menu words from menu words, so a construction stays in the pinned
/// menu's alphabet, and a menu word has a length.
#[test]
fn the_word_operations_take_menu_words_to_menu_words() {
    admitted(
        "fn f(x: MenuWord, y: MenuWord) -> MenuWord effects [] {\n    \
         let a = concat(x, inverse(y));\n    \
         let b = compose(conjugate(a, menu[0:1]), commutator(x, y));\n    \
         return b;\n}\n",
    );
    admitted("fn f(x: MenuWord) -> Nat effects [] {\n    return length(concat(x, menu[]));\n}\n");
}

/// LB1.8 (LP2 draft): a Full word and a menu word are words of two alphabets. No word operation, collection, call,
/// return or match mixes them, and a word key and a compressed word take Full words only. Each is refused with
/// `LB1:input.incompatible_alphabet`, naming the expected and the supplied type.
#[test]
fn a_full_word_and_a_menu_word_never_meet() {
    let operations = [
        ("concat(x, w)", "MenuWord", "Word"),
        ("concat(w, x)", "Word", "MenuWord"),
        ("concat(x, word[+1])", "MenuWord", "Word"),
        ("conjugate(x, w)", "MenuWord", "Word"),
        ("commutator(w, x)", "Word", "MenuWord"),
        ("compose(x, w)", "MenuWord", "Word"),
        ("inverse(concat(w, menu[0:1]))", "Word", "MenuWord"),
        ("word_key(x)", "Word", "MenuWord"),
        ("compress(x)", "Word", "MenuWord"),
        ("list<Word>(w, x)", "Word", "MenuWord"),
    ];
    for (operation, expected, supplied) in operations {
        let declarations =
            format!("fn f(x: MenuWord, w: Word) -> Nat effects [] {{\n    let r = {operation};\n    return 1;\n}}\n");
        let found = only(&declarations);
        assert_eq!(found, (INCOMPATIBLE_ALPHABET, Some((expected.into(), supplied.into()))), "{operation}");
    }
    let programs = [
        ("fn f(x: MenuWord) -> Word effects [] {\n    return x;\n}\n", "Word", "MenuWord"),
        (
            "fn g(x: MenuWord) -> Nat effects [] {\n    return length(x);\n}\n\n\
             fn f(w: Word) -> Nat effects [] {\n    return g(w);\n}\n",
            "MenuWord",
            "Word",
        ),
        (
            "fn f(w: Word) -> Nat effects [] {\n    return match w {\n        menu[0:1] => 1;\n        _ => 2;\n    };\n}\n",
            "Word",
            "MenuWord",
        ),
        (
            "fn f(x: MenuWord) -> Nat effects [] {\n    return match x {\n        word[+1] => 1;\n        _ => 2;\n    };\n}\n",
            "MenuWord",
            "Word",
        ),
    ];
    for (declarations, expected, supplied) in programs {
        let found = only(declarations);
        assert_eq!(found, (INCOMPATIBLE_ALPHABET, Some((expected.into(), supplied.into()))), "{declarations}");
    }
}

/// LB1.8 (LP2 draft): the two alphabets never meet inside a type either. A call's argument, a result, a collection's
/// element, a map's value, a pushed element, a concatenation, an equality, two match arms or a callable whose types
/// differ only by a menu word where a Full word stands, or the reverse, is refused with
/// `LB1:input.incompatible_alphabet`, naming the whole types. Types that differ in anything else as well stay a type
/// mismatch.
#[test]
fn a_full_word_and_a_menu_word_never_meet_inside_a_type() {
    let lists = ("List<Word>", "List<MenuWord>");
    let cases = [
        (
            "fn g(xs: List<Word>) -> Nat effects [] {\n    return length(xs);\n}\n\n\
             fn f(x: MenuWord) -> Nat effects [] {\n    return g(list<MenuWord>(x));\n}\n",
            lists,
        ),
        ("fn f(x: MenuWord) -> List<Word> effects [] {\n    return list<MenuWord>(x);\n}\n", lists),
        (
            "fn f(x: MenuWord) -> record { w: Word } effects [] {\n    return record { w = x };\n}\n",
            ("record { w: Word }", "record { w: MenuWord }"),
        ),
        ("fn f(x: MenuWord) -> Nat effects [] {\n    let r = list<List<Word>>(list<MenuWord>(x));\n    return 1;\n}\n", lists),
        (
            "fn f(m: Map<Nat, List<Word>>, x: MenuWord) -> Nat effects [] {\n    \
             let r = insert(m, 1, list<MenuWord>(x));\n    return 1;\n}\n",
            lists,
        ),
        (
            "fn f(x: MenuWord) -> Nat effects [] {\n    let r = push(list<List<Word>>(), list<MenuWord>(x));\n    \
             return 1;\n}\n",
            lists,
        ),
        (
            "fn f(w: Word, x: MenuWord) -> Nat effects [] {\n    let r = concat(list<Word>(w), list<MenuWord>(x));\n    \
             return 1;\n}\n",
            lists,
        ),
        (
            "fn f(w: Word, x: MenuWord) -> Bool effects [] {\n    return exact_eq(list<Word>(w), list<MenuWord>(x));\n}\n",
            lists,
        ),
        (
            "fn f(n: Nat, w: Word, x: MenuWord) -> Nat effects [] {\n    let r = match n {\n        \
             0 => list<Word>(w);\n        _ => list<MenuWord>(x);\n    };\n    return 1;\n}\n",
            lists,
        ),
        (
            "fn h(x: MenuWord) -> Nat effects [] {\n    return length(x);\n}\n\n\
             fn apply(g: fn(Word) -> Nat effects [], w: Word) -> Nat effects [] {\n    return g(w);\n}\n\n\
             fn f(w: Word) -> Nat effects [] {\n    return apply(h, w);\n}\n",
            ("fn(Word) -> Nat", "fn(MenuWord) -> Nat"),
        ),
    ];
    for (declarations, (expected, supplied)) in cases {
        let found = only(declarations);
        assert_eq!(found, (INCOMPATIBLE_ALPHABET, Some((expected.into(), supplied.into()))), "{declarations}");
    }
    let others = [
        (
            "fn f(x: MenuWord) -> List<Nat> effects [] {\n    return list<MenuWord>(x);\n}\n",
            ("List<Nat>", "List<MenuWord>"),
        ),
        (
            "fn f(x: MenuWord) -> record { n: Nat, w: Word } effects [] {\n    \
             return record { n = \"t\", w = x };\n}\n",
            ("record { n: Nat, w: Word }", "record { n: Text, w: MenuWord }"),
        ),
    ];
    for (declarations, (expected, supplied)) in others {
        let found = only(declarations);
        assert_eq!(found, (TYPE_MISMATCH, Some((expected.into(), supplied.into()))), "{declarations}");
    }
}

/// LB1.8 (LP2 draft): a menu word, named or literal, is proposed like a Full word: a preview, with the `preview`
/// effect.
#[test]
fn a_menu_word_is_proposed_with_the_preview_effect() {
    admitted("fn f(x: MenuWord) -> Receipt effects [preview] {\n    return propose(x, target(\"t\"));\n}\n");
    admitted("fn f() -> Receipt effects [preview] {\n    return propose(menu[0:1, 3:2], target(\"t\"));\n}\n");
    let (code, _) = only("fn f(x: MenuWord) -> Receipt effects [] {\n    return propose(x, target(\"t\"));\n}\n");
    assert_eq!(code, EFFECT_UNDECLARED);
}

/// LB1.8 (LP2 draft): a match compares a menu word with menu literals by their letters.
#[test]
fn a_match_on_a_menu_word_takes_menu_literals() {
    admitted(
        "fn f(x: MenuWord) -> Nat effects [] {\n    return match x {\n        menu[0:1] => 1;\n        menu[] => 0;\n        \
         _ => 2;\n    };\n}\n",
    );
}
