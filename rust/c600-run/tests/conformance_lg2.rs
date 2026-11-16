//! Conformance tests for chapter 02's LG2 rows at run time (LP2, M2 and M3): the run-time halves of CT-LG2-05,
//! CT-LG2-08, CT-LG2-11 and CT-LG2-13. The static halves are `c600-syntax`'s `conformance_lg2.rs` and `c600-check`'s
//! tests.

mod conformance;

use c600_ir::build::{exact, helper, module, nat, program, propose, read_receipt, target};
use c600_ir::{Expr, Lit, ReceiptField, Ty};
use c600_run::boundary::{Answer, Store};
use c600_run::interp::{code, load, NoHost, Options};
use c600_run::meter::Outcome;
use c600_run::provenance::{Author, Profile};
use c600_run::value::Value;
use conformance::*;

/// A display tint whose branch depends on an approximate comparison; the second branch costs more.
const TINT: &str = "export fn tint(x: Nat, y: Nat) -> Display<Colour> effects [] {\n    \
    return display_if(approx_less(approx53(x), approx53(y)), colour(\"red\"), \
    display_if(approx_less(approx53(y), approx53(x)), colour(\"blue\"), colour(\"grey\")));\n}\n";

/// CT-LG2-05 (part: DET.6 reproduction under form 1's terminal display isolation, LG2.20; the static mechanisms are
/// `c600-check`'s and `c600-syntax`'s CT-LG2-05 tests; not run: form 2, which is on record and not built, LG2.21): a
/// run whose display branch depends on an approximate comparison keeps its outcome, code, used budget and peak memory
/// in the reproducible payload. Repeated runs give the same summary and bytes for each input, and the branch taken is
/// what is charged.
#[test]
fn ct_lg2_05_display_isolation_keeps_the_reproducible_payload() {
    let (_, loaded) = build(&[&m(TINT)]);
    let run =
        |x: i64, y: i64| loaded.run("m", "tint", vec![Value::int(x), Value::int(y)], &mut NoHost, &Options::new(WIDE));
    let mut used = Vec::new();
    for (x, y, colour) in [(1, 2, "red"), (2, 1, "blue"), (1, 1, "grey")] {
        let first = run(x, y).unwrap();
        let again = run(x, y).unwrap();
        assert_eq!(first.summary, again.summary, "({x}, {y})");
        assert_eq!(first.output, again.output, "({x}, {y})");
        assert_eq!((first.summary.outcome, first.summary.code), (Outcome::Completed, None));
        let text = String::from_utf8(first.output.clone().unwrap()).unwrap();
        assert!(text.contains(colour), "({x}, {y}): {text}");
        used.push(first.summary.used);
    }
    // The second branch evaluates a second comparison, so its run is charged more: the payload records the branch
    // actually taken, not a fixed estimate.
    assert!(used[0].steps < used[1].steps && used[1].steps == used[2].steps, "{used:?}");
}

/// A built-in previewer, and a command that stores a word.
const PREVIEWER: &str = "export fn go(w: Word) -> Text effects [preview] {\n    \
    return receipt_id(propose(w, target(\"t\")));\n}\n\n\
    export fn keep(w: Word) -> Unit effects [workspace] {\n    return write_workspace<Word>(\"kept\", w);\n}\n";

/// CT-LG2-08 (part: the run-time half for a retained word; not run: the static mode check and menu words, which wait
/// for the jumbled puzzle's alphabet, M5, and mode changes, which are command-layer operations, LP3): a program checked
/// once is refused at invocation after the session's mode changed, the shell keeps the engine's refusal and the
/// program sees the same acknowledgement; storing a word asks the engine for nothing, so it cannot change the mode.
#[test]
fn ct_lg2_08_a_mode_change_refuses_at_invocation() {
    let source = m(PREVIEWER);
    let (mut engine, mut store) = (Engine::default(), Store::default());
    let options = Options::new(WIDE);
    let go = |engine: &mut Engine, store: &mut Store| {
        let solving = (Author::BuiltIn, Profile::Solving);
        run_as(&source, "go", solving, vec![Value::word(&[1, -2])], vec![manual()], engine, store, &options)
    };
    let before = go(&mut engine, &mut store);
    assert!(matches!(before.shell[0].answer, Answer::Previewed(_)), "{:?}", before.shell[0].answer);
    // Jumble is turned on (a command-layer operation, here the engine's state).
    engine.refuse_previews = Some(NEEDS_JUMBLE_OFF);
    let after = go(&mut engine, &mut store);
    match &after.shell[0].answer {
        Answer::Refused(refusal) => assert_eq!(refusal.code, NEEDS_JUMBLE_OFF),
        other => panic!("previewed after the mode change: {other:?}"),
    }
    assert_eq!(engine.previews, [vec![1, -2], vec![1, -2]], "each invocation is rechecked by the engine");
    assert_eq!((before.run.output, &before.run.summary), (after.run.output, &after.run.summary));
    // A stored word changes no session state.
    let hash = engine.state_hash();
    let solving = (Author::BuiltIn, Profile::Solving);
    let kept = run_as(
        &source,
        "keep",
        solving,
        vec![Value::word(&[1, -2])],
        vec![manual()],
        &mut engine,
        &mut store,
        &options,
    );
    assert_eq!(kept.run.summary.outcome, Outcome::Completed);
    assert!(store.records.contains_key(&ws("kept")));
    assert_eq!((engine.state_hash(), engine.previews.len(), engine.journal.len()), (hash, 2, 0));
}

/// The ID of a proposal of `word`, in hand-built IR that the checker would refuse.
fn proposing(word: Expr) -> c600_ir::Program {
    let mut f = helper("f", vec![], exact(Ty::Text), read_receipt(propose(word, target("t")), ReceiptField::Id));
    f.exported = true;
    program(vec![module("m", vec![f])])
}

/// CT-LG2-11 (part: the run-time half of the proposal binding; not run: execution, derivation and protection
/// bindings, which wait for M4, and the `WordEqualityKey` type, LG2.13, which the IR draft does not have yet): a
/// proposal that reaches the interpreter with anything but a literal word stops with `LG4:input.ir_rejected` before the
/// host sees a request, although the loader does not re-run the checker; a literal word reaches the host letter for
/// letter.
#[test]
fn ct_lg2_11_a_proposal_runs_a_literal_word_only() {
    for word in [Expr::Lit(Lit::Text("w".into())), nat(1), Expr::Lit(Lit::Tag("k".into())), Expr::Lit(Lit::Unit)] {
        let loaded = load(&proposing(word.clone())).unwrap_or_else(|r| panic!("{r:?}"));
        let mut engine = Engine::default();
        let r = loaded.run("m", "f", vec![], &mut engine, &Options::new(WIDE)).unwrap();
        assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(code::IR_REJECTED)), "{word:?}");
        assert!(engine.previews.is_empty(), "{word:?}");
    }
    let loaded = load(&proposing(Expr::Lit(Lit::Word(vec![3, -1, 2])))).unwrap();
    let mut engine = Engine::default();
    let r = loaded.run("m", "f", vec![], &mut engine, &Options::new(WIDE)).unwrap();
    assert_eq!(r.summary.outcome, Outcome::Completed, "{:?} {:?}", r.summary.code, r.summary.stop);
    assert_eq!(engine.previews, [vec![3, -1, 2]]);
    assert_eq!(
        r.output.as_deref(),
        Some(
            &b"\"preview-1\"
"[..]
        )
    );
}

/// CT-LG2-13 (part: the empty-word half at run time; not run: the key's domain test at a straddling cap, which waits
/// for LG2.13's key type and the jumbled puzzle, M5): an inverse pair is unequal to the empty word, and a proposal of
/// the pair reaches the engine, or a capture, with both letters.
#[test]
fn ct_lg2_13_the_empty_word_is_no_witness_for_an_inverse_pair() {
    let source = m("export fn same() -> Bool effects [] {\n    return exact_eq(word[+1, -1], word[]);\n}\n\n\
                    export fn pair() -> Text effects [preview] {\n    \
                    return receipt_id(propose(word[+1, -1], target(\"t\")));\n}\n");
    let (_, loaded) = build(&[&source]);
    let same = loaded.run("m", "same", vec![], &mut NoHost, &Options::new(WIDE)).unwrap();
    assert!(matches!(same.value, Some(Value::Bool(false))), "{:?}", same.value);
    let (mut engine, mut store) = (Engine::default(), Store::default());
    let options = Options::new(WIDE);
    let solving = (Author::BuiltIn, Profile::Solving);
    let previewed = run_as(&source, "pair", solving, vec![], vec![], &mut engine, &mut store, &options);
    assert_eq!(previewed.shell[0].word, [1, -1]);
    assert_eq!(engine.previews, [vec![1, -1]]);
    let authoring = (Author::Own, Profile::ToolAuthoring);
    let captured = run_as(&source, "pair", authoring, vec![], vec![], &mut engine, &mut store, &options);
    assert_eq!(captured.captures[0].word, [1, -1]);
    assert_eq!(engine.previews.len(), 1, "a capture builds no preview");
}
