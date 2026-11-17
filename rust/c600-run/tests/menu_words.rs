//! LB1.8's literal words of one pinned menu at run time (LP2 draft; LP3's plan, section 2). A run pins at most one menu
//! alphabet, and every menu word in it is a word of that alphabet: a literal takes the pinned alphabet when it is read,
//! an argument or a host's answer of another alphabet is refused or stops the run, the word operations build words of
//! the alphabet with an inverse from the pinned menu's inverse items (LB1.9), a match compares letters, and a proposal
//! names the word's alphabet, so the engine refuses a word of another menu by the first differing field of the
//! identity blocks in KN1.4's order.

mod conformance;

use std::sync::Arc;

use c600_check::{check, separation, Policy};
use c600_ir::build::{exact, helper, menu_word, module, program, word};
use c600_ir::{Expr, Prim, Ty};
use c600_run::boundary::{Answer, Boundary, Capture, Context, Shown, Store};
use c600_run::encode::encode;
use c600_run::interp::code::{IR_REJECTED, NONCANONICAL_VALUE};
use c600_run::interp::{load, Host, NoHost, Options, Request, RowTariff, Run, Sink};
use c600_run::menu::{check_words, MenuAlphabet, MenuError, MenuIdentity, INCOMPATIBLE_ALPHABET, MENU_INDEX_LIMIT};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome;
use c600_run::provenance::{self, Author, Json, Profile};
use c600_run::value::{equal, Value};
use conformance::*;

fn identity(menu_id: &str) -> MenuIdentity {
    MenuIdentity {
        puzzle: "600-cell-jumbled".into(),
        model_id: "model-1".into(),
        menu_id: menu_id.into(),
        menu_order: "order-1".into(),
        contract_rev: "rev-1".into(),
    }
}

/// Four caps and a menu of three items: item 0 is its own inverse, items 1 and 2 are each other's.
fn alphabet(menu_id: &str) -> MenuAlphabet {
    MenuAlphabet::new(identity(menu_id), 4, vec![0, 2, 1]).expect("an alphabet")
}

fn menu(a: &MenuAlphabet, letters: &[[u32; 2]]) -> Value {
    Value::menu_word(a.shared_identity(), letters).expect("indices below 600")
}

fn pinned(a: &MenuAlphabet) -> Options<'_> {
    Options { menu: Some(a), ..Options::new(WIDE) }
}

fn letters(v: &Value) -> Vec<[u32; 2]> {
    match v {
        Value::MenuWord(w) => w.letters().to_vec(),
        other => panic!("not a menu word: {other:?}"),
    }
}

fn canonical(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode(v, &mut out).expect("a canonical value");
    out
}

fn completed(run: &Run) {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
}

fn stopped(run: &Run, code: &str) {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Error, Some(code)), "{:?}", run.summary.stop);
}

#[test]
fn an_alphabet_is_named_bounded_and_its_inverse_items_pair_up() {
    let a = alphabet("menu-a");
    assert_eq!((a.caps(), a.items(), a.inverse_items()), (4, 3, &[0, 2, 1][..]));
    assert!(a.holds([3, 2]) && !a.holds([4, 0]) && !a.holds([0, 3]));
    let x = [[0, 1], [1, 0], [3, 2]];
    assert_eq!(a.inverse_word(&x), Some(vec![[3, 1], [1, 0], [0, 2]]));
    assert_eq!(a.inverse_word(&a.inverse_word(&x).unwrap()), Some(x.to_vec()), "LB1.9: the inverse is an involution");
    assert_eq!(a.inverse_word(&[[0, 3]]), None);

    let mut unnamed = identity("menu-a");
    unnamed.contract_rev.clear();
    assert_eq!(MenuAlphabet::new(MenuIdentity::default(), 4, vec![0]), Err(MenuError::Identity("puzzle")));
    assert_eq!(MenuAlphabet::new(unnamed, 4, vec![0]), Err(MenuError::Identity("contract_rev")));
    assert!(MenuAlphabet::new(identity("m"), MENU_INDEX_LIMIT, vec![0]).is_ok());
    assert_eq!(MenuAlphabet::new(identity("m"), MENU_INDEX_LIMIT + 1, vec![0]), Err(MenuError::Caps(601)));
    assert_eq!(MenuAlphabet::new(identity("m"), 4, vec![0; 601]), Err(MenuError::Items(601)));
    assert_eq!(MenuAlphabet::new(identity("m"), 4, vec![1, 1]), Err(MenuError::Inverse { item: 0 }));
    assert_eq!(MenuAlphabet::new(identity("m"), 4, vec![3]), Err(MenuError::Inverse { item: 0 }));

    // Two blocks differ first in KN1.4's order: puzzle, model_id, menu_id, menu_order, contract_rev.
    let mut other_model = identity("menu-b");
    other_model.model_id = "model-2".into();
    assert_eq!(identity("menu-a").differing(&identity("menu-a")), None);
    assert_eq!(identity("menu-a").differing(&identity("menu-b")), Some("menu_id"));
    assert_eq!(identity("menu-a").differing(&other_model), Some("model_id"));
    let why = check_words(&menu(&alphabet("menu-b"), &[]), Some(&a)).unwrap_err();
    assert!(why.contains("menu_id"), "{why}");
}

const LITERAL: &str = "export fn w() -> MenuWord effects [] {\n    return menu[3:2, 0:1];\n}\n\n\
    export fn outside() -> MenuWord effects [] {\n    return menu[4:0];\n}\n";

/// A literal takes the pinned alphabet when it is read, so its output names the menu. With no pinned alphabet, or with
/// a letter outside it, the run stops at the literal.
#[test]
fn a_literal_takes_the_pinned_alphabet_and_a_letter_outside_it_stops_the_run() {
    let (_, loaded) = build(&[&m(LITERAL)]);
    let a = alphabet("menu-a");
    let ran = loaded.run("m", "w", vec![], &mut NoHost, &pinned(&a)).unwrap();
    completed(&ran);
    assert!(equal(ran.value.as_ref().expect("a value"), &menu(&a, &[[3, 2], [0, 1]])));
    let output = "{\"letters\":[[3,2],[0,1]],\"menu\":{\"contract_rev\":\"rev-1\",\"menu_id\":\"menu-a\",\
        \"menu_order\":\"order-1\",\"model_id\":\"model-1\",\"puzzle\":\"600-cell-jumbled\"}}\n";
    assert_eq!(ran.output.as_deref(), Some(output.as_bytes()));

    stopped(&loaded.run("m", "w", vec![], &mut NoHost, &Options::new(WIDE)).unwrap(), INCOMPATIBLE_ALPHABET);
    stopped(&loaded.run("m", "outside", vec![], &mut NoHost, &pinned(&a)).unwrap(), INCOMPATIBLE_ALPHABET);
}

const ECHO: &str = "export fn echo(w: MenuWord) -> MenuWord effects [] {\n    return w;\n}\n\n\
    export fn count(l: List<MenuWord>) -> Nat effects [] {\n    return length(l);\n}\n";

/// An argument that holds a menu word of another alphabet, or one with a letter outside the pinned alphabet, or any
/// menu word where the run pins none, is refused before the run starts; so is a word whose block names no alphabet,
/// which has no canonical form.
#[test]
fn an_argument_of_another_alphabet_or_outside_the_pinned_one_is_refused() {
    let (_, loaded) = build(&[&m(ECHO)]);
    let (a, b) = (alphabet("menu-a"), alphabet("menu-b"));
    completed(&loaded.run("m", "echo", vec![menu(&a, &[[1, 2]])], &mut NoHost, &pinned(&a)).unwrap());
    let refused = |entry: &str, arg: Value, options: &Options| {
        loaded.run("m", entry, vec![arg], &mut NoHost, options).err().map(|r| r.code)
    };
    assert_eq!(refused("echo", menu(&b, &[[1, 2]]), &pinned(&a)), Some(INCOMPATIBLE_ALPHABET));
    assert_eq!(refused("echo", menu(&a, &[[4, 0]]), &pinned(&a)), Some(INCOMPATIBLE_ALPHABET));
    assert_eq!(refused("echo", menu(&a, &[[1, 2]]), &Options::new(WIDE)), Some(INCOMPATIBLE_ALPHABET));
    let mixed = Value::list(vec![menu(&a, &[[0, 0]]), menu(&b, &[[0, 0]])]);
    assert_eq!(refused("count", mixed, &pinned(&a)), Some(INCOMPATIBLE_ALPHABET));
    let unnamed = Value::menu_word(&Arc::new(MenuIdentity::default()), &[[0, 0]]).expect("bounded");
    assert_eq!(refused("echo", unnamed, &pinned(&a)), Some(NONCANONICAL_VALUE));
}

/// Answers every literal read with the value it holds.
struct Answering(Value);

impl Host for Answering {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, r: &Request) -> Result<Value, &'static str> {
        match r {
            Request::Snapshot { .. } => Ok(self.0.clone()),
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

const READ: &str =
    "export fn best() -> MenuWord effects [snapshot] {\n    return read_snapshot<MenuWord>(\"best\");\n}\n";

/// A host's answer that holds a menu word of another alphabet, or a letter outside the pinned one, stops the run after
/// the read's row: the host answered, and the program never sees the word.
#[test]
fn a_host_answer_of_another_alphabet_stops_the_run_after_its_row() {
    let (_, loaded) = build(&[&m(READ)]);
    let (a, b) = (alphabet("menu-a"), alphabet("menu-b"));
    let ran = |answer: Value| loaded.run("m", "best", vec![], &mut Answering(answer), &pinned(&a)).unwrap();
    let ok = ran(menu(&a, &[[2, 1]]));
    completed(&ok);
    assert_eq!(letters(ok.value.as_ref().expect("a value")), [[2, 1]]);
    for answer in [menu(&b, &[[2, 1]]), menu(&a, &[[2, 3]])] {
        let run = ran(answer);
        stopped(&run, INCOMPATIBLE_ALPHABET);
        assert!(run.value.is_none());
        assert!(run.summary.rows.iter().any(|r| r.at.ends_with("#kernel")), "{:?}", run.summary.rows);
    }
}

const OPS: &str = "export fn ops(x: MenuWord, y: MenuWord) -> record { cat: MenuWord, inv: MenuWord, \
    conj: MenuWord, comm: MenuWord, comp: MenuWord, len: Nat } effects [] {\n    \
    return record { cat = concat(x, y), inv = inverse(x), conj = conjugate(x, y), comm = commutator(x, y), \
    comp = compose(x, y), len = length(x) };\n}\n";

/// LB1.8's constructors on menu words: concatenation, inverse, conjugate and commutator (and composition, which
/// concatenates as on words), each a word of the pinned alphabet, priced as on words per letter built. The inverse
/// reverses the letters, each at its own cap with the item of the inverse rotation.
#[test]
fn the_word_operations_build_words_of_the_pinned_alphabet() {
    let (_, loaded) = build(&[&m(OPS)]);
    let a = alphabet("menu-a");
    let (x, y) = (menu(&a, &[[0, 1], [1, 0]]), menu(&a, &[[2, 2]]));
    let options = pinned(&a);
    let ran = loaded.run("m", "ops", vec![x, y], &mut NoHost, &options).unwrap();
    completed(&ran);
    let Some(Value::Record(r)) = &ran.value else { panic!("a record: {:?}", ran.value) };
    let field = |name: &str| r.fields().iter().find(|(k, _)| k.as_str() == name).map(|(_, v)| v.clone()).unwrap();
    let (xi, yi) = ([[1, 0], [0, 2]], [[2, 1]]);
    assert_eq!(letters(&field("cat")), [[0, 1], [1, 0], [2, 2]]);
    assert_eq!(letters(&field("comp")), [[0, 1], [1, 0], [2, 2]]);
    assert_eq!(letters(&field("inv")), xi);
    assert_eq!(letters(&field("conj")), [&[[0, 1], [1, 0], [2, 2]][..], &xi].concat());
    assert_eq!(letters(&field("comm")), [&[[0, 1], [1, 0], [2, 2]][..], &xi, &yi].concat());
    assert!(equal(&field("len"), &Value::int(2)));
    assert!(equal(&field("cat"), &menu(&a, &[[0, 1], [1, 0], [2, 2]])), "the result names the pinned alphabet");

    let per_letter = options.tariffs.expand_per_letter;
    let row = |op: &str| ran.summary.rows.iter().find(|r| r.at.ends_with(&format!("#{op}"))).expect(op).clone();
    let built = [("concat", 3), ("inverse", 2), ("conjugate", 5), ("commutator", 6), ("compose", 3)];
    for (op, letters) in built {
        assert_eq!(row(op).kernel_cost, per_letter * letters, "{op}");
    }
}

/// A Full word and a menu word never meet in one operation: the checker refuses the mix with LB1's code, and the same
/// IR built by hand stops the run before the step.
#[test]
fn a_full_word_and_a_menu_word_never_meet_in_one_operation() {
    let mix = Expr::Prim { op: Prim::Concat, args: vec![menu_word(&[[0, 1]]), word(&[1])] };
    let hand = program(vec![module("m", vec![helper("mix", vec![], exact(Ty::MenuWord), mix).exported()])]);
    let report = check(&hand).unwrap();
    assert!(report.codes().contains(&INCOMPATIBLE_ALPHABET), "{:#?}", report.diagnostics);
    let a = alphabet("menu-a");
    let run = load(&hand).unwrap().run("m", "mix", vec![], &mut NoHost, &pinned(&a)).unwrap();
    stopped(&run, IR_REJECTED);
}

/// A menu letter is two indices below 600 (`MENU_INDEX_LIMIT`). The parser refuses a larger one at its source text. In
/// IR built by hand the checker admits it, since it bounds no literal's letters, as for a Full word's, and loading
/// refuses it before any run starts.
#[test]
fn a_literal_letter_of_600_or_more_is_refused_when_the_ir_is_loaded() {
    for outside in [[MENU_INDEX_LIMIT, 0], [0, MENU_INDEX_LIMIT]] {
        let body = menu_word(&[[0, 1], outside]);
        let hand = program(vec![module("m", vec![helper("w", vec![], exact(Ty::MenuWord), body).exported()])]);
        assert!(check(&hand).unwrap().is_admitted(), "{outside:?}");
        let refused = load(&hand).err().unwrap_or_else(|| panic!("loaded: {outside:?}"));
        assert_eq!(refused.code, IR_REJECTED, "{outside:?}: {}", refused.detail);
    }
}

const WHICH: &str = "export fn which(w: MenuWord) -> Nat effects [] {\n    return match w {\n        \
    menu[0:1] => 1;\n        menu[0:1, 1:0] => 2;\n        _ => 0;\n    };\n}\n";

/// A match arm's menu-word literal compares letters: the pattern is a literal of the pinned alphabet.
#[test]
fn a_match_compares_a_menu_word_by_its_letters() {
    let (_, loaded) = build(&[&m(WHICH)]);
    let a = alphabet("menu-a");
    for (letters, arm) in [(&[[0, 1]][..], 1), (&[[0, 1], [1, 0]][..], 2), (&[[0, 2]][..], 0), (&[][..], 0)] {
        let ran = loaded.run("m", "which", vec![menu(&a, letters)], &mut NoHost, &pinned(&a)).unwrap();
        completed(&ran);
        assert!(equal(ran.value.as_ref().expect("a value"), &Value::int(arm)), "{letters:?}");
    }
}

/// The engine side of a jumbling session: it previews a menu word of its own alphabet and refuses one of another, and
/// a Full word needs Jumble off. It records every menu-word request it is handed.
struct Jumbling {
    alphabet: MenuIdentity,
    previews: Vec<(Vec<[u32; 2]>, MenuIdentity, Vec<u8>)>,
}

impl Host for Jumbling {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 1 << 20 })
    }

    fn perform(&mut self, r: &Request) -> Result<Value, &'static str> {
        match *r {
            Request::ProposeMenu { letters, menu, request, .. } => {
                self.previews.push((letters.to_vec(), menu.clone(), request.to_vec()));
                match menu.differing(&self.alphabet) {
                    Some(_) => Err(IDENTITY_MISMATCH),
                    None => Ok(Value::receipt(vec![("base", Value::text("rev-7")), ("id", Value::text("preview-1"))])),
                }
            }
            Request::Propose { .. } => Err(NEEDS_JUMBLE_OFF),
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

/// Runs an entry of `source` through the boundary adapter, with a hand-entered word as its input.
fn through(
    source: &str,
    entry: &str,
    (author, profile): (Author, Profile),
    args: Vec<Value>,
    engine: &mut dyn Host,
    options: &Options,
) -> (Run, Vec<Shown>, Vec<Capture>) {
    let (lowered, loaded) = build(&[source]);
    let report = check(&lowered.program).unwrap();
    let reads_state = separation(report.summary("m", entry).expect("the entry"), &Policy::interim()).reads_state;
    let context = Context {
        run: "run".into(),
        base: "rev-7".into(),
        producer: producer(source, entry, author, profile),
        reads_state,
        inputs: vec![manual()],
        pins: pins(),
    };
    let mut store = Store::default();
    let mut boundary = Boundary::new(engine, &mut store, context);
    let run = loaded.run("m", entry, args, &mut boundary, options).unwrap_or_else(|r| panic!("{r:?}"));
    (run, boundary.shell, boundary.captures)
}

const GO: &str = "export fn go(w: MenuWord) -> Text effects [preview] {\n    \
    return receipt_id(propose(w, target(\"t\")));\n}\n";

/// A menu-word proposal hands the engine the letters, the identity block of their alphabet and the canonical bytes of
/// `{target, word}`, the word with its letters and its menu. In a solving session the shell keeps the menu word and the
/// engine's answer: a preview in the session's own menu, a refusal in another. In tool authoring it is captured.
#[test]
fn a_menu_word_is_proposed_with_the_identity_block_of_its_alphabet() {
    let source = m(GO);
    let a = alphabet("menu-a");
    let w = menu(&a, &[[1, 2], [0, 0]]);
    let solving = (Author::BuiltIn, Profile::Solving);
    for (session, previewed) in [("menu-a", true), ("menu-b", false)] {
        let mut engine = Jumbling { alphabet: identity(session), previews: vec![] };
        let (run, shell, captures) = through(&source, "go", solving, vec![w.clone()], &mut engine, &pinned(&a));
        completed(&run);
        let [(sent, block, request)] = &engine.previews[..] else { panic!("{:?}", engine.previews) };
        assert_eq!((&sent[..], block), (&[[1, 2], [0, 0]][..], a.identity()));
        let Some(Json::Object(record)) = provenance::parse(request) else { panic!("canonical request bytes") };
        assert_eq!(record.keys().collect::<Vec<_>>(), ["target", "word"]);
        let Json::Object(word) = &record["word"] else { panic!("{:?}", record["word"]) };
        assert_eq!(word.keys().collect::<Vec<_>>(), ["letters", "menu"]);

        let [shown] = &shell[..] else { panic!("{shell:?}") };
        assert!(shown.word.is_empty() && captures.is_empty());
        let shown_word = shown.menu.as_ref().expect("the menu word");
        assert_eq!((shown_word.letters(), shown_word.identity()), (&[[1, 2], [0, 0]][..], a.identity()));
        match (&shown.answer, previewed) {
            (Answer::Previewed(_), true) => {}
            (Answer::Refused(refusal), false) => assert_eq!(refusal.code, IDENTITY_MISMATCH),
            (other, _) => panic!("{session}: {other:?}"),
        }
    }

    let mut engine = Jumbling { alphabet: identity("menu-a"), previews: vec![] };
    let authoring = (Author::Own, Profile::ToolAuthoring);
    let (run, shell, captures) = through(&source, "go", authoring, vec![w], &mut engine, &pinned(&a));
    completed(&run);
    assert!(engine.previews.is_empty() && shell.is_empty());
    let [capture] = &captures[..] else { panic!("{captures:?}") };
    assert!(capture.word.is_empty());
    assert_eq!(capture.menu.as_ref().map(|w| w.letters().to_vec()), Some(vec![[1, 2], [0, 0]]));
}

const KEEP: &str = "export fn keep(w: MenuWord) -> Unit effects [workspace] {\n    \
    return write_workspace<MenuWord>(\"kept\", w);\n}\n";

/// CT-LG2-08 (part: a stored menu word asks the engine for nothing, so it never changes the session mode; not run: the
/// static mode check, which waits for the session mode in the IR, and mode changes, which are command-layer
/// operations, LP3): the store keeps the word with the name of its alphabet, and the engine sees no request.
#[test]
fn ct_lg2_08_a_stored_menu_word_asks_the_engine_for_nothing() {
    let source = m(KEEP);
    let a = alphabet("menu-a");
    let w = menu(&a, &[[3, 1]]);
    let (mut engine, mut store) = (Engine::default(), Store::default());
    let solving = (Author::BuiltIn, Profile::Solving);
    let ran = run_as(&source, "keep", solving, vec![w.clone()], vec![manual()], &mut engine, &mut store, &pinned(&a));
    completed(&ran.run);
    assert!(engine.previews.is_empty() && engine.journal.is_empty() && ran.shell.is_empty());
    assert_eq!(store.records[&ws("kept")].bytes, canonical(&w));
}
