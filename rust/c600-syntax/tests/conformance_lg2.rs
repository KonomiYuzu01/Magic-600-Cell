//! Conformance tests for chapter 02's LG2 rows in source (LP2): CT-LG2-01 to CT-LG2-11 and CT-LG2-13, each naming
//! the part it runs. The IR-level cases are `c600-check`'s; the run-time halves of CT-LG2-05, CT-LG2-08, CT-LG2-11
//! and CT-LG2-13 are in `c600-run`'s `conformance_lg2.rs`.
//!
//! The IR draft has no identity tower, no domain families and no menu words yet (`rust/README.md`, M7). Where a row
//! needs them, the test runs as a guard: such source does not lower, and the IR's types and operations are matched
//! without a wildcard, so a family added in M4 or M5 fails to compile here until its tests are written.

use c600_check::WORD_KEY_NOT_EXECUTABLE;
use c600_check::{admit, check, Grant, Policy, Profile, Report};
use c600_check::{EFFECT_UNDECLARED, EXACTNESS_VIOLATION, EXACT_REQUIRED, NOT_GRANTED, TYPE_MISMATCH};
use c600_ir::{Capability, Expr as Ir, Prim, Ty};
use c600_syntax::{lower, parse, LowerError, Lowered};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

/// `id<T>(x: T) -> T`.
const ID: &str = "fn id<T>(x: T) -> T effects [] {\n    return x;\n}\n\n";

/// `apply<T,E>(g, v)`, generic in a type and an effect variable.
const APPLY: &str = "fn apply<T,E>(g: fn(T) -> T effects [E], v: T) -> T effects [E] {\n    return g(v);\n}\n\n";

fn tool(declarations: &str) -> String {
    format!("{HEADER}{declarations}")
}

fn lowered(source: &str) -> Result<Lowered, LowerError> {
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    lower(&[file])
}

fn report(declarations: &str) -> Report {
    let lowered = lowered(&tool(declarations)).unwrap_or_else(|e| panic!("{declarations}: {e}"));
    check(&lowered.program).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

fn admitted(declarations: &str) {
    let r = report(declarations);
    assert!(r.is_admitted(), "{declarations}: {:#?}", r.diagnostics);
}

/// The only finding: its code and, for a type mismatch, the expected and the supplied type.
fn only(declarations: &str) -> (&'static str, Option<(String, String)>) {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    (d.code, d.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone())))
}

fn mismatch(expected: &str, supplied: &str) -> (&'static str, Option<(String, String)>) {
    (TYPE_MISMATCH, Some((expected.to_string(), supplied.to_string())))
}

fn refused_at_lowering(declarations: &str) -> String {
    match lowered(&tool(declarations)) {
        Ok(_) => panic!("lowered: {declarations}"),
        Err(e) => e.to_string(),
    }
}

/// Each type, as a parameter type, does not lower, and the refusal names the milestone the type waits for.
fn assert_types_wait(types: &[&str], waits_for: &str) {
    for ty in types {
        let message = refused_at_lowering(&format!("fn f(p: {ty}) -> Int effects [] {{\n    return 1;\n}}\n"));
        assert!(message.contains(waits_for), "{ty}: {message}");
    }
}

/// Every type of the IR draft, matched without a wildcard. A domain family added to the IR (a piece, a pose, a
/// sticker class, a state, a verdict, a goal or an equality key) fails to compile here until its CT-LG2 tests exist.
/// The word equality key (LP2 draft) has them: `ct_lg2_11_*` and `ct_lg2_13_*` below.
fn base_types_only(ty: &Ty) -> bool {
    match ty {
        Ty::Unit
        | Ty::Bool
        | Ty::Nat
        | Ty::Int
        | Ty::Rat
        | Ty::Q5
        | Ty::Text
        | Ty::Word
        | Ty::Target
        | Ty::Budget
        | Ty::Status
        | Ty::Colour
        | Ty::Approx(_)
        | Ty::ApproxBool
        | Ty::Receipt
        | Ty::WordEqualityKey
        | Ty::Var(_) => true,
        Ty::Record(fields) => fields.iter().all(|(_, f)| base_types_only(&f.ty)),
        Ty::Fn(f) => f.params.iter().all(|p| base_types_only(&p.ty)) && base_types_only(&f.result.ty),
        Ty::List(e) => base_types_only(&e.ty),
    }
}

/// The family of every reserved operation, matched without a wildcard. Only `word_key` makes a key, and no operation
/// takes one back to letters (`ct_lg2_11_*`).
fn prim_family(op: Prim) -> &'static str {
    match op {
        Prim::ExactAdd | Prim::ExactLess | Prim::ExactEq | Prim::And | Prim::Or | Prim::Not => "arithmetic",
        Prim::ToApprox(_) | Prim::ToExact | Prim::ApproxAdd | Prim::ApproxLess | Prim::ApproxAnd => "arithmetic",
        Prim::ApproxNot => "arithmetic",
        Prim::Length | Prim::Index | Prim::Push | Prim::Concat => "collection",
        Prim::Inverse | Prim::Conjugate | Prim::Commutator | Prim::Compose => "word",
        Prim::WordKey => "word key",
    }
}

/// The operation family of every IR expression, matched without a wildcard. None projects between levels, lifts
/// labels to poses, constructs a state from raw data or turns a key into a word; an operation added to the IR fails
/// to compile here until its tests exist.
fn family(e: &Ir) -> &'static str {
    match e {
        Ir::Lit(_) | Ir::Var(_) | Ir::FnRef { .. } | Ir::Closure { .. } | Ir::Record(_) | Ir::Field { .. } => "value",
        Ir::Collection { .. } => "value",
        Ir::Let { .. } | Ir::If { .. } | Ir::Match { .. } | Ir::DisplayIf { .. } | Ir::Call { .. } => "control",
        Ir::Prim { op, .. } => prim_family(*op),
        Ir::ReadSnapshot { .. } | Ir::Declassified { .. } => "snapshot read",
        Ir::PublishView { .. } | Ir::WriteWorkspace { .. } => "view or workspace",
        Ir::FileRead { .. } | Ir::FileWrite { .. } | Ir::RandomDraw | Ir::ReadClock => "granted input or output",
        Ir::Propose { .. } | Ir::ReadReceipt { .. } => "preview",
        Ir::UseExact { .. } | Ir::Accounting(_) => "exact sink or accounting",
    }
}

/// Every expression of every function of a lowered program, depth first.
fn expressions(lowered: &Lowered) -> Vec<&Ir> {
    let mut out = Vec::new();
    let mut pending: Vec<&Ir> = lowered.program.modules.iter().flat_map(|m| &m.functions).map(|f| &f.body).collect();
    while let Some(e) = pending.pop() {
        out.push(e);
        pending.extend(e.children());
    }
    out
}

/// A program that uses every surface construct the draft lowers: its types are base types and its operations are
/// the families above.
fn assert_guard(source: &str) {
    let lowered = lowered(source).unwrap_or_else(|e| panic!("{e}"));
    for module in &lowered.program.modules {
        for f in &module.functions {
            assert!(f.params.iter().all(|p| base_types_only(&p.ty.ty)) && base_types_only(&f.result.ty));
        }
    }
    assert!(expressions(&lowered).iter().all(|e| !family(e).is_empty()));
}

const EVERYTHING: &str = "edition \"6cs-draft-1\";\nmodule tool requires [file, clock, random];\n\n\
    command go(w: Word, b: Budget) -> Text effects [snapshot, workspace, preview, budget, file, random] {\n    \
    let n = read_snapshot<Nat>(\"n\");\n    let s = write_workspace<Word>(\"saved\", w);\n    \
    let f = file_write(\"out\", file_read<Nat>(\"in\"));\n    let c = read_clock();\n    let r = random_draw();\n    \
    let u = use_budget(b);\n    let k = used_steps();\n    return receipt_id(propose(w, target(\"t\")));\n}\n";

// CT-LG2-01: algebraic matches account for every constructor; generics and inference cannot erase a model, level,
// mode, precision, unit or effect requirement.

/// CT-LG2-01 (part: the match clause on the draft's literal matches; not run: matches over algebraic constructors,
/// since the IR draft has no data types or verdict constructors yet, M4): a match ends with a `_` arm, so no
/// alternative is left out, even where the literal arms happen to be exhaustive; a verdict match does not lower.
#[test]
fn ct_lg2_01_a_match_accounts_for_every_alternative() {
    for declaration in [
        "fn f(x: Nat) -> Nat effects [] {\n    return match x {\n        1 => 2;\n    };\n}\n",
        "fn f(b: Bool) -> Nat effects [] {\n    return match b {\n        true => 1;\n        false => 2;\n    };\n}\n",
    ] {
        let message = refused_at_lowering(declaration);
        assert!(message.contains("`_` arm"), "{declaration}: {message}");
    }
    admitted(
        "fn f(b: Bool) -> Nat effects [] {\n    return match b {\n        true => 1;\n        _ => 2;\n    };\n}\n",
    );
    let verdict =
        "fn f(v: Nat) -> Text effects [] {\n    return match v {\n        yes(p) => \"y\";\n        _ => \"n\";\n    \
                   };\n}\n";
    assert!(refused_at_lowering(verdict).contains("M4"));
}

/// CT-LG2-01 (part: precision, exactness, unit, effect and capability requirements through generic substitution and
/// inference; not run: model, level and mode indices, which the IR draft does not have yet, M4, M5 and LP3; the unit
/// half covers `Budget` only, since puzzle-indexed costs wait for M5).
#[test]
fn ct_lg2_01_generics_and_inference_keep_precision_unit_and_effect() {
    // Precision: substitution keeps P24, which does not fit a P53 result.
    let narrow = format!("{ID}fn f(x: Nat) -> Approx<P53> effects [] {{\n    return id(approx24(x));\n}}\n");
    assert_eq!(only(&narrow), mismatch("Approx<P53>", "Approx<P24>"));
    // Exactness: an approximate value does not come back exact through a generic.
    let erased = format!("{ID}fn f(x: Nat) -> Int effects [] {{\n    return id(approx53(x));\n}}\n");
    assert!([EXACTNESS_VIOLATION, EXACT_REQUIRED].contains(&only(&erased).0));
    // Unit: a budget stays a budget, which mixes with integers only (LG2.6).
    let budget = format!("{ID}fn f(b: Budget) -> Rat effects [] {{\n    return exact_add(id(b), 1/2);\n}}\n");
    assert_eq!(only(&budget), mismatch("Nat | Int | Budget", "Rat"));
    // Effect: instantiating the effect variable with a snapshot reader keeps the read (LG3.2).
    let probe =
        "fn probe(w: Word) -> Word effects [snapshot] {\n    let n = read_snapshot<Nat>(\"n\");\n    return w;\n}\n\n";
    let call = |effects: &str| {
        format!("{APPLY}{probe}fn f(w: Word) -> Word effects [{effects}] {{\n    return apply(probe, w);\n}}\n")
    };
    assert_eq!(only(&call("")).0, EFFECT_UNDECLARED);
    admitted(&call("snapshot"));
    // Capability: inference through the generic finds the file requirement but grants nothing (LG3.9).
    let source = format!(
        "edition \"6cs-draft-1\";\nmodule tool requires [file];\n\n{APPLY}\
         fn load(w: Word) -> Word effects [file] {{\n    let n = file_read<Nat>(\"data\");\n    return w;\n}}\n\n\
         command go(w: Word) -> Word effects [file] {{\n    return apply(load, w);\n}}\n"
    );
    let program = lowered(&source).unwrap_or_else(|e| panic!("{e}")).program;
    let run = |grant: &Grant| admit(&program, "tool", "go", Profile::ToolAuthoring, grant, &Policy::interim()).unwrap();
    assert_eq!(run(&Grant::none()).codes(), [NOT_GRANTED]);
    assert!(run(&Grant::of(&[Capability::File])).is_admitted());
}

/// CT-LG2-02 (part, guard only: the IR draft has no identity tower or domain families, so a Full piece, a jumbling
/// piece, a pose, a sticker class, a label digest and a pose-table index cannot be written yet; the substitution and
/// unification refusals wait for M4 and M5).
#[test]
fn ct_lg2_02_identity_types_wait_for_the_domain_families() {
    let types = ["Piece", "Piece<Full>", "Piece<Jumbled>", "Pose", "StickerClass", "LabelDigest", "PoseTableIndex"];
    assert_types_wait(&types, "M4, M5");
    assert_types_wait(&["Word<Full,Retained>"], "M4, M5");
    assert_guard(EVERYTHING);
}

/// CT-LG2-03 (part, guard only: levels, projections and labels wait for the identity tower, M4 and M5; no operation of
/// the IR draft projects between levels or lifts labels to poses, and there is no class to treat as equality yet).
#[test]
fn ct_lg2_03_no_projection_or_lift_exists_before_the_identity_tower() {
    assert_types_wait(&["Labels", "Projection", "Lattice", "Level"], "M4, M5");
    for name in ["project", "lift", "labels"] {
        let declaration = format!("fn f(w: Word) -> Word effects [] {{\n    return {name}(w);\n}}\n");
        let message = refused_at_lowering(&declaration);
        assert!(message.contains("not bound"), "{name}: {message}");
    }
    assert_guard(EVERYTHING);
}

/// CT-LG2-04 (part: the source half; the IR half is `c600-check`'s `ct_lg2_04_*` in `exactness.rs`, and the row is
/// whole with them): an exact-to-approximate conversion is explicit and names a precision; there is no implicit or
/// unnamed one, and a direct conversion back, a declared exact result and an ordinary-boolean helper are refused.
#[test]
fn ct_lg2_04_conversions_in_source() {
    for p in [24, 53, 113] {
        admitted(&format!("fn f(x: Nat) -> Approx<P{p}> effects [] {{\n    return approx{p}(x);\n}}\n"));
    }
    assert_eq!(only("fn f(x: Nat) -> Approx<P53> effects [] {\n    return x;\n}\n").0, TYPE_MISMATCH);
    for declaration in [
        "fn f(x: Nat) -> Approx<P53> effects [] {\n    return approx(x);\n}\n",
        "fn f(x: Nat) -> Approx<P53> effects [] {\n    return approx64(x);\n}\n",
    ] {
        assert!(refused_at_lowering(declaration).contains("not bound"), "{declaration}");
    }
    assert!(refused_at_lowering("fn f(x: Nat) -> Approx<P64> effects [] {\n    return 1;\n}\n").contains("P53"));
    for declaration in [
        "fn f(x: Nat) -> Int effects [] {\n    return to_exact(approx53(x));\n}\n",
        "fn f(a: Approx<P53>) -> Int effects [] {\n    return a;\n}\n",
        "fn f(a: Approx<P53>, b: Approx<P53>) -> Bool effects [] {\n    return approx_less(a, b);\n}\n",
        "fn f(x: Nat, y: Nat) -> Word effects [] {\n    \
         return if(approx_less(approx53(x), approx53(y)), word[+1], word[+2]);\n}\n",
    ] {
        let code = only(declaration).0;
        assert!([EXACTNESS_VIOLATION, EXACT_REQUIRED].contains(&code), "{declaration}: {code}");
    }
}

// CT-LG2-05 is `c600-check`'s (`exactness.rs`, `vectors.rs`) and `lowering.rs`'s; its DET.6 half runs in `c600-run`.

/// CT-LG2-06 (part: the budget half on the draft's single `Budget` unit; not run: puzzle and unit indices for menu
/// letters, retained generators (M5) and UI actions (LP3); the seconds half is
/// `ct_lg2_06_a_clock_reading_is_no_deterministic_budget`): a budget mixes with integers only, in addition, ordering
/// and budget use, and an approximate value is no budget.
#[test]
fn ct_lg2_06_a_budget_mixes_with_integers_only() {
    admitted("command f(b: Budget) -> Unit effects [budget] {\n    return use_budget(exact_add(b, 1));\n}\n");
    admitted("fn f(b: Budget, n: Int) -> Bool effects [] {\n    return exact_less(b, n);\n}\n");
    let refused = [
        ("fn f(b: Budget) -> Bool effects [] {\n    return exact_less(b, 1/2);\n}\n", "Nat | Int | Budget", "Rat"),
        (
            "fn f(b: Budget) -> Q5 effects [] {\n    return exact_add(q5(1/2, 1/2), b);\n}\n",
            "Nat | Int | Rat | Q5",
            "Budget",
        ),
        ("command f() -> Unit effects [budget] {\n    return use_budget(1/2);\n}\n", "Nat | Int | Budget", "Rat"),
    ];
    for (declaration, expected, supplied) in refused {
        assert_eq!(only(declaration), mismatch(expected, supplied), "{declaration}");
    }
    let approx = only("command f(x: Nat) -> Unit effects [budget] {\n    return use_budget(approx53(x));\n}\n").0;
    assert!([EXACTNESS_VIOLATION, EXACT_REQUIRED].contains(&approx), "{approx}");
}

/// CT-LG2-06 (part: "seconds cannot be a deterministic budget", LG2.6, LG3.8, DET.18): a clock reading is
/// display-qualified, because seconds are a display unit only, so a clock grant cannot make it a budget: `use_budget`
/// of a reading is `LG2:type.exactness_violation`, while a display result may still show it. The checker typed a
/// reading as an exact `Nat` until this test found it (16 November 2026).
#[test]
fn ct_lg2_06_a_clock_reading_is_no_deterministic_budget() {
    let codes = |command: &str| {
        let source = format!("edition \"6cs-draft-1\";\nmodule tool requires [clock];\n\n{command}");
        let program = lowered(&source).unwrap_or_else(|e| panic!("{e}")).program;
        let grant = Grant::of(&[Capability::Clock]);
        admit(&program, "tool", "go", Profile::Research, &grant, &Policy::interim()).unwrap().codes()
    };
    let budget = "command go() -> Unit effects [budget, clock] {\n    return use_budget(read_clock());\n}\n";
    assert_eq!(codes(budget), [EXACTNESS_VIOLATION], "a clock grant makes seconds a deterministic budget");
    let shown = "command go() -> Display<Nat> effects [clock] {\n    return read_clock();\n}\n";
    assert!(codes(shown).is_empty(), "a display result shows a clock reading");
}

/// CT-LG2-07 (part, guard only: states, derived states, pose tables and reachability claims wait for the domain
/// families, M4 and M5; a snapshot read gives base-typed data, never a state).
#[test]
fn ct_lg2_07_no_state_enters_from_raw_data_before_the_domain_families() {
    assert_types_wait(&["State", "DerivedState", "PoseTable", "ReachabilityClaim", "Witness"], "M4, M5");
    let message =
        refused_at_lowering("fn f() -> Nat effects [snapshot] {\n    return read_snapshot<State>(\"s\");\n}\n");
    assert!(message.contains("M4, M5"), "{message}");
    assert_guard(EVERYTHING);
}

/// CT-LG2-08 (part, static guard: menu words and the session mode wait for the jumbled puzzle's alphabet, M5, and the
/// command layer, LP3; the run-time refusal at invocation is `c600-run`'s
/// `ct_lg2_08_a_mode_change_refuses_at_invocation`).
#[test]
fn ct_lg2_08_menu_words_and_modes_do_not_lower_yet() {
    assert!(refused_at_lowering("fn f() -> Word effects [] {\n    return menu[2:spin];\n}\n").contains("M5"));
    assert_types_wait(&["Mode", "Revision"], "M4, M5");
}

/// CT-LG2-09 (part, guard only: verdicts, partial projections and goal levels wait for the domain families, M4; the
/// draft has exact booleans only, and an approximate comparison or a status is no condition).
#[test]
fn ct_lg2_09_verdicts_wait_and_a_condition_is_an_exact_boolean() {
    assert_types_wait(&["Verdict", "Unknown", "Goal"], "M4");
    let verdict = "fn f(v: Nat) -> Bool effects [] {\n    return match v {\n        \
                   unknown(r) => false;\n        _ => true;\n    };\n}\n";
    assert!(refused_at_lowering(verdict).contains("M4"));
    let status = only("fn f() -> Nat effects [budget] {\n    return if(budget_status(), 1, 2);\n}\n");
    assert_eq!(status, mismatch("Bool", "Status"));
}

/// CT-LG2-10 (part, guard only: goals, their levels and checkpoints wait for the domain families, M4; the draft has no
/// implication between goals to check).
#[test]
fn ct_lg2_10_goals_wait_for_the_domain_families() {
    let goals = ["Goal", "ExactPose", "StickerTest", "LatticePlacement", "RetainedCheckpoint", "LabelCheckpoint"];
    assert_types_wait(&goals, "M4, M5");
}

/// CT-LG2-11 (part: the proposal binding statically; not run: execution and protection bindings, which wait for M4;
/// the key's half is `ct_lg2_11_a_word_equality_key_is_never_literal_letters` and the run-time half is `c600-run`'s
/// `ct_lg2_11_a_proposal_runs_a_literal_word_only`): `propose` takes a literal word and nothing else.
#[test]
fn ct_lg2_11_a_proposal_takes_a_literal_word_only() {
    for (argument, supplied) in
        [("\"w\"", "Text"), ("1", "Nat"), ("tag(\"k\")", "Status"), ("record { w = word[+1] }", "")]
    {
        let declaration =
            format!("fn f() -> Receipt effects [preview] {{\n    return propose({argument}, target(\"t\"));\n}}\n");
        let (code, types) = only(&declaration);
        assert_eq!(code, TYPE_MISMATCH, "{argument}");
        let (expected, got) = types.expect("mismatch types");
        assert_eq!(expected, "Word", "{argument}");
        assert!(supplied.is_empty() || got == supplied, "{argument}: {got}");
    }
}

/// CT-LG2-11 (part: the `WordEqualityKey` type and `word_key` (LP2 drafts) in source, against the proposal binding,
/// the word operations (derivation) and a `Word` parameter; not run: execution and protection bindings, which wait
/// for M4, and a key indexed by puzzle and identity, which waits for the domain families, M4 and M5): a key is never
/// accepted where literal letters are required, and no source form converts it back.
#[test]
fn ct_lg2_11_a_word_equality_key_is_never_literal_letters() {
    let key = "word_key(word[+1, -2])";
    let uses = [
        format!("fn f() -> Receipt effects [preview] {{\n    return propose({key}, target(\"t\"));\n}}\n"),
        format!("fn f() -> Word effects [] {{\n    return inverse({key});\n}}\n"),
        format!("fn f() -> Word effects [] {{\n    return compose(word[+1], {key});\n}}\n"),
        format!("fn g(w: Word) -> Word effects [] {{\n    return w;\n}}\n\nfn f() -> Word effects [] {{\n    return g({key});\n}}\n"),
        format!("fn f() -> Word effects [] {{\n    return {key};\n}}\n"),
        format!("fn f() -> Unit effects [workspace] {{\n    return write_workspace<Word>(\"w\", {key});\n}}\n"),
    ];
    for declaration in &uses {
        let (code, types) = only(declaration);
        assert_eq!(code, WORD_KEY_NOT_EXECUTABLE, "{declaration}");
        assert_eq!(types, Some(("Word".to_string(), "WordEqualityKey".to_string())), "{declaration}");
    }
    admitted("fn f(w: Word) -> WordEqualityKey effects [] {\n    return word_key(inverse(inverse(w)));\n}\n");
    admitted("fn f(a: WordEqualityKey, b: WordEqualityKey) -> Bool effects [] {\n    return exact_eq(a, b);\n}\n");
    // A key of a key is no key: `word_key` takes literal letters.
    let (code, _) = only("fn f(w: Word) -> WordEqualityKey effects [] {\n    return word_key(word_key(w));\n}\n");
    assert_eq!(code, WORD_KEY_NOT_EXECUTABLE);
    for literal in ["word[]", "\"k\"", "1"] {
        let declaration = format!("fn f() -> WordEqualityKey effects [] {{\n    return {literal};\n}}\n");
        assert_eq!(only(&declaration).0, TYPE_MISMATCH, "{literal}");
    }
}

/// CT-LG2-13 (part: the empty-word half statically, with the Full key (LP2 draft); not run: the jumbling key and its
/// domain test at a straddling cap, which wait for the jumbled puzzle, M5; the run-time halves are `c600-run`'s
/// `ct_lg2_13_the_empty_word_is_no_witness_for_an_inverse_pair` and
/// `ct_lg2_13_the_full_key_cancels_the_pair_not_the_witness`): an inverse pair is a literal word of its own, unequal
/// to the empty word, and comparing keys gives a boolean, never a word to propose.
#[test]
fn ct_lg2_13_an_inverse_pair_is_not_the_empty_word() {
    admitted("fn f() -> Bool effects [] {\n    return exact_eq(word_key(word[+1, -1]), word_key(word[]));\n}\n");
    let source = tool("fn f() -> Bool effects [] {\n    return exact_eq(word[+1, -1], word[]);\n}\n");
    let lowered = lowered(&source).unwrap_or_else(|e| panic!("{e}"));
    assert!(check(&lowered.program).unwrap().is_admitted());
    let words: Vec<_> = expressions(&lowered)
        .into_iter()
        .filter_map(|e| match e {
            Ir::Lit(c600_ir::Lit::Word(w)) => Some(w.clone()),
            _ => None,
        })
        .collect();
    assert!(words.contains(&vec![1, -1]) && words.contains(&vec![]), "{words:?}");
}
