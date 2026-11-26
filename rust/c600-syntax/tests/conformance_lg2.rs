//! Conformance tests for chapter 02's LG2 rows in source (LP2): CT-LG2-01 to CT-LG2-11 and CT-LG2-13, each naming
//! the part it runs. The IR-level cases are `c600-check`'s (the identity tower's in its `tower.rs`); the run-time
//! halves of CT-LG2-05, CT-LG2-08, CT-LG2-11 and CT-LG2-13 are in `c600-run`'s `conformance_lg2.rs`, and the identity
//! tower's run-time halves wait for T1c's values and host rows.
//!
//! The identity tower and the domain families (T1 drafts until the IR freeze, PHS.49) are written in source as types
//! with static indices (`State<Jumbling>`, `Digest<Full, labels>`, `Verdict<Jumbling, labels, sticker_test>`), kernel
//! rows with their indices as type arguments (`project<labels>(s)`, `check<exact_pose>(s)`), built-in constructors
//! with their type as a type argument (`total<Partial<Nat>>(x)`), a module's sum types (`type Shape { ... }`), and a
//! `match` on constructor patterns. Each source test below mirrors a case of `c600-check`'s `tower.rs` and expects
//! its code and, for a mismatch, its types; where the source form differs from the hand-built IR, the test says so.
//! Menu words are an LP2 draft (LB1.8), tested in `menu_words.rs`. The IR's types and operations are matched below
//! without a wildcard, so a type or an operation added to the IR fails to compile here until its tests are written.

use c600_check::{admit, check, Grant, Policy, Profile, Report};
use c600_check::{
    AMBIENT_READ, EFFECT_UNDECLARED, EXACTNESS_VIOLATION, EXACT_REQUIRED, INCOMPATIBLE_ALPHABET, TYPE_MISMATCH,
};
use c600_check::{
    COST_UNIT_MIX, IDENTITY_MISMATCH, LEVEL_MISMATCH, MODE_MISMATCH, REACHABILITY_MISSING, VERDICT_REQUIRED,
    WORD_KEY_NOT_EXECUTABLE,
};
use c600_ir::{Capability, CostUnit, Expr as Ir, Goal, Level, Prim, Puzzle, QTy, ReachKind, Ty};
use c600_syntax::{format, lower, parse, LowerError, Lowered};

use Puzzle::{Full, Jumbling};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

/// `id<T>(x: T) -> T`.
const ID: &str = "fn id<T>(x: T) -> T effects [] {\n    return x;\n}\n\n";

/// `apply<T,E>(g, v)`, generic in a type and an effect variable.
const APPLY: &str = "fn apply<T,E>(g: fn(T) -> T effects [E], v: T) -> T effects [E] {\n    return g(v);\n}\n\n";

/// `type Shape { dot(); line(Nat); }`, as `c600-check`'s `tower.rs` declares it.
const SHAPE: &str = "type Shape {\n    dot();\n    line(Nat);\n}\n\n";

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

/// The codes of every finding, in order.
fn codes(declarations: &str) -> Vec<&'static str> {
    report(declarations).codes()
}

/// The only finding: its code and, for a type mismatch, the expected and the supplied type.
fn only(declarations: &str) -> (&'static str, Option<(String, String)>) {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    (d.code, d.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone())))
}

/// The only finding's code and path.
fn at(declarations: &str) -> (&'static str, String) {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    (r.diagnostics[0].code, r.diagnostics[0].path.clone())
}

fn mismatch(expected: &str, supplied: &str) -> (&'static str, Option<(String, String)>) {
    (TYPE_MISMATCH, Some((expected.to_string(), supplied.to_string())))
}

fn types(code: &'static str, expected: &str, supplied: &str) -> (&'static str, Option<(String, String)>) {
    (code, Some((expected.to_string(), supplied.to_string())))
}

fn refused_at_lowering(declarations: &str) -> String {
    match lowered(&tool(declarations)) {
        Ok(_) => panic!("lowered: {declarations}"),
        Err(e) => e.to_string(),
    }
}

const NONE: [&str; 0] = [];

/// A type in source: the identity tower's types with their indices by the IR's names, and the base types the tests
/// use (T1 draft).
fn src(ty: &Ty) -> String {
    let q = |q: &QTy| if q.display { format!("Display<{}>", src(&q.ty)) } else { src(&q.ty) };
    match ty {
        Ty::Piece(p) => format!("Piece<{}>", p.name()),
        Ty::Sticker(p) => format!("Sticker<{}>", p.name()),
        Ty::Slot(p) => format!("Slot<{}>", p.name()),
        Ty::State(p) => format!("State<{}>", p.name()),
        Ty::Reachability(p) => format!("Reachability<{}>", p.name()),
        Ty::Derivation(p) => format!("Derivation<{}>", p.name()),
        Ty::Digest(p, l) => format!("Digest<{}, {}>", p.name(), l.name()),
        Ty::LevelValue(p, l) => format!("LevelValue<{}, {}>", p.name(), l.name()),
        Ty::Verdict(p, l, None) => format!("Verdict<{}, {}>", p.name(), l.name()),
        Ty::Verdict(p, l, Some(g)) => format!("Verdict<{}, {}, {}>", p.name(), l.name(), g.name()),
        Ty::Partial(t) => format!("Partial<{}>", q(t)),
        Ty::Holds(p, g) => format!("Holds<{}, {}>", p.name(), g.name()),
        Ty::Cost(p, u) => format!("Cost<{}, {}>", p.name(), u.name()),
        Ty::WordEqualityKey(Full) => "WordEqualityKey".into(),
        Ty::WordEqualityKey(Jumbling) => "WordEqualityKey<Jumbling>".into(),
        Ty::Pose => "Pose".into(),
        Ty::PoseTableIndex => "PoseTableIndex".into(),
        Ty::PoseTable => "PoseTable".into(),
        Ty::Evidence => "Evidence".into(),
        Ty::Unit => "Unit".into(),
        Ty::Bool => "Bool".into(),
        Ty::Nat => "Nat".into(),
        Ty::Int => "Int".into(),
        Ty::Word => "Word".into(),
        Ty::MenuWord => "MenuWord".into(),
        Ty::List(e) => format!("List<{}>", q(e)),
        Ty::Record(fields) => {
            let fields: Vec<String> = fields.iter().map(|(n, t)| format!("{n}: {}", q(t))).collect();
            format!("record {{ {} }}", fields.join(", "))
        }
        other => panic!("no source spelling in this test for {other:?}"),
    }
}

fn exact(ty: Ty) -> QTy {
    QTy { ty, display: false }
}

/// `fn f(params) -> result effects [effects] { return body; }`.
fn f(params: &[(&str, &Ty)], result: &Ty, effects: &str, body: &str) -> String {
    let params: Vec<String> = params.iter().map(|(n, t)| format!("{n}: {}", src(t))).collect();
    format!("fn f({}) -> {} effects [{effects}] {{\n    return {body};\n}}\n", params.join(", "), src(result))
}

/// The codes of a value of type `supplied` passed as the argument of a parameter of type `expected`.
fn as_argument(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    let g = format!("fn g(x: {}) -> Unit effects [] {{\n    return ();\n}}\n\n", src(expected));
    codes(&format!("{g}{}", f(&[("a", supplied)], &Ty::Unit, "", "g(a)")))
}

/// The codes of a value of type `supplied` returned where the result type is `expected`.
fn as_result(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    codes(&f(&[("a", supplied)], expected, "", "a"))
}

/// The codes of a value of type `supplied` returned through the generic `id<T>(x: T) -> T` where the result type is
/// `expected`: substitution keeps every index (LG2.1).
fn through_generic(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    codes(&format!("{ID}{}", f(&[("a", supplied)], expected, "", "id(a)")))
}

/// The codes of an exact equality of a value of type `a` and one of type `b`.
fn compared(a: &Ty, b: &Ty) -> Vec<&'static str> {
    codes(&f(&[("a", a), ("b", b)], &Ty::Bool, "", "exact_eq(a, b)"))
}

/// The three ways a value meets a declared type, which agree on its code.
fn every_way(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    let arg = as_argument(expected, supplied);
    assert_eq!(as_result(expected, supplied), arg, "{expected:?} <- {supplied:?} as a result");
    assert_eq!(through_generic(expected, supplied), arg, "{expected:?} <- {supplied:?} through a generic");
    arg
}

fn partial(ty: Ty) -> Ty {
    Ty::Partial(Box::new(exact(ty)))
}

/// Every type of the IR draft, matched without a wildcard, true for the draft's base types. A type added to the IR
/// fails to compile here until its tests exist. The word equality key (LP2 draft) has them: `ct_lg2_11_*` and
/// `ct_lg2_13_*` below. So has the compressed word (LP2 draft): `ct_lg2_14_*` in `collections.rs` here and in
/// `c600-run`. So has the menu word (LP2 draft): `menu_words.rs` here and in `c600-run`. So has an end test's verdict
/// (LP2 draft): CT-LG2-09's unknown that never passes as false, in `end_tests.rs` here and in `c600-check` and
/// `c600-run`. So has the generator (LP2 draft; DET.61): `generators.rs` here and in `c600-run`. F2 and F5 (LP2 draft)
/// are number kinds of KN3.2, not domain families; their tests are `fields.rs` here, in `c600-check` and in
/// `c600-run`. Captured bytes (LP2 draft; DET.62) hold octets that a program reads only through `text_decode`; their
/// tests are `decode.rs` in `c600-run`. A word stream (LP2 draft; DET.63) holds a streamed word's length and literal
/// digest, no letter; its tests are `streams.rs` in `c600-run`.
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
        | Ty::WordEqualityKey(_)
        | Ty::TraceEqualityKey
        | Ty::CompressedWord
        | Ty::MenuWord
        | Ty::EndTest(_)
        | Ty::Generator
        | Ty::Field(_)
        | Ty::Bytes
        | Ty::WordStream
        | Ty::Var(_) => true,
        Ty::Record(fields) => fields.iter().all(|(_, f)| base_types_only(&f.ty)),
        Ty::Fn(f) => f.params.iter().all(|p| base_types_only(&p.ty)) && base_types_only(&f.result.ty),
        Ty::List(e) | Ty::Set(e) => base_types_only(&e.ty),
        Ty::Map(k, v) => base_types_only(&k.ty) && base_types_only(&v.ty),
        // The identity tower and the domain families (T1 drafts in `c600-ir` and `c600-check`, whose checker tests
        // are `c600-check/tests/tower.rs`). Their source forms lower (T1b): the `ct_lg2_*` tests below write each of
        // them, and `c600-syntax`'s `sum_types.rs` the declared sum types.
        Ty::Piece(_)
        | Ty::Sticker(_)
        | Ty::Slot(_)
        | Ty::Pose
        | Ty::PoseTableIndex
        | Ty::Digest(..)
        | Ty::LevelValue(..)
        | Ty::State(_)
        | Ty::PoseTable
        | Ty::Reachability(_)
        | Ty::Evidence
        | Ty::Verdict(..)
        | Ty::Partial(_)
        | Ty::Holds(..)
        | Ty::Cost(..)
        | Ty::Derivation(_)
        | Ty::Named { .. } => false,
    }
}

/// The family of every reserved operation, matched without a wildcard. Only `word_key` and `trace_key` make a key, and
/// no operation takes one back to letters (`ct_lg2_11_*`, `ct_lb1_06_*`).
fn prim_family(op: Prim) -> &'static str {
    match op {
        Prim::ExactAdd | Prim::ExactLess | Prim::ExactEq | Prim::And | Prim::Or | Prim::Not => "arithmetic",
        Prim::ToApprox(_) | Prim::ToExact | Prim::ApproxAdd | Prim::ApproxLess | Prim::ApproxAnd => "arithmetic",
        Prim::ApproxNot | Prim::ExactMul | Prim::Q5Parts | Prim::Q5Of => "arithmetic",
        Prim::Assert => "resource control",
        Prim::ToField(_) | Prim::Residue | Prim::FieldAdd | Prim::FieldSub | Prim::FieldNeg | Prim::FieldMul => {
            "arithmetic"
        }
        Prim::FieldInverse => "arithmetic",
        // A stage module's binary32 operations and `exclude`'s coverage (RPI.70; LP2 draft; tests in c600-check).
        Prim::Stage(_) => "stage arithmetic",
        Prim::Length | Prim::Index | Prim::Push | Prim::Concat => "collection",
        Prim::Contains | Prim::Insert | Prim::InsertEntry | Prim::Lookup => "collection",
        Prim::Elements(_) | Prim::Keys(_) | Prim::Entries(_) => "collection",
        Prim::Inverse | Prim::Conjugate | Prim::Commutator | Prim::Compose => "word",
        Prim::Compress | Prim::Expand => "word",
        // DET.63's streaming and the literal digest it derives (LP2 draft): words and recipes.
        Prim::WordStream | Prim::StreamDigest => "word",
        Prim::WordKey | Prim::TraceKey => "word key",
        Prim::RngDraw => "granted input or output",
        // Captured bytes into text (DET.62; LP2 draft).
        Prim::TextDecode => "decoding",
        // LG2.11's implications and LG2.6's costs (T1 drafts): `imply<g>(h)`, `to_cost<P, U>(n)`, `cost_value(c)`.
        Prim::Imply(_) => "goal",
        Prim::ToCost(..) | Prim::CostValue => "arithmetic",
    }
}

/// The operation family of every IR expression, matched without a wildcard. None projects except `project`, which
/// gives a partial result; none lifts labels to poses, constructs a state from raw data except `import` with a claim,
/// or turns a key into a word; an operation added to the IR fails to compile here until its tests exist.
fn family(e: &Ir) -> &'static str {
    match e {
        Ir::Lit(_) | Ir::Var(_) | Ir::FnRef { .. } | Ir::Closure { .. } | Ir::Record(_) | Ir::Field { .. } => "value",
        Ir::Collection { .. } => "value",
        Ir::Let { .. } | Ir::If { .. } | Ir::Match { .. } | Ir::DisplayIf { .. } | Ir::Call { .. } => "control",
        Ir::Child { .. } | Ir::Repeat { .. } => "control",
        Ir::Prim { op, .. } => prim_family(*op),
        Ir::ReadSnapshot { .. } | Ir::ReadSnapshotWith { .. } | Ir::Declassified { .. } => "snapshot read",
        Ir::PublishView { .. } | Ir::WriteWorkspace { .. } => "view or workspace",
        Ir::FileRead { .. } | Ir::FileWrite { .. } | Ir::RandomDraw | Ir::ReadClock => "granted input or output",
        Ir::RngInit { .. } => "granted input or output",
        // Bytes the code holds, decoded into text (DET.62, DET.65; LP2 draft).
        Ir::TextDecodeHex { .. } => "decoding",
        Ir::Propose { .. } | Ir::ReadReceipt { .. } => "preview",
        Ir::UseExact { .. } | Ir::Accounting(_) => "exact sink or accounting",
        // A sum type's construction and case, and the identity tower's kernel rows (T1 drafts; their source forms are
        // T1b's, tested below).
        Ir::Construct { .. } => "value",
        Ir::Case { .. } => "control",
        Ir::Kernel { .. } => "kernel row",
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

/// Whether every signature of a lowered program is of the draft's base types, and every expression has a family.
fn base_typed(source: &str) -> bool {
    let lowered = lowered(source).unwrap_or_else(|e| panic!("{e}"));
    assert!(expressions(&lowered).iter().all(|e| !family(e).is_empty()));
    lowered
        .program
        .modules
        .iter()
        .flat_map(|m| &m.functions)
        .all(|f| f.params.iter().all(|p| base_types_only(&p.ty.ty)) && base_types_only(&f.result.ty))
}

const EVERYTHING: &str = "edition \"6cs-draft-1\";\nmodule tool requires [file, clock, random];\n\n\
    command go(w: Word, b: Budget) -> Text effects [snapshot, workspace, preview, budget, file, random] {\n    \
    let n = read_snapshot<Nat>(\"n\");\n    let s = write_workspace<Word>(\"saved\", w);\n    \
    let f = file_write(\"out\", file_read<Nat>(\"in\"));\n    let c = read_clock();\n    let r = random_draw();\n    \
    let u = use_budget(b);\n    let k = used_steps();\n    \
    let h = child(record { steps = 8 }, fn() -> Nat { return 1; }, fn(x: Status) -> Nat { return 0; });\n    \
    return receipt_id(propose(w, target(\"t\")));\n}\n";

/// The identity tower in one program: every type constructor, every kernel row and a case, each with its family.
const TOWER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n\
    type Shape {\n    dot();\n    line(Nat);\n}\n\n\
    fn go(s: State<Jumbling>, t: PoseTable, r: Reachability<Jumbling>, w: MenuWord, h: Holds<Jumbling, exact_pose>, \
    c: Cost<Full, generator>, p: Piece<Full>, x: Sticker<Jumbling>, y: Slot<Full>, o: Pose, i: PoseTableIndex, \
    e: Evidence, z: Shape) -> Nat effects [snapshot] {\n    \
    let a = snapshot<Jumbling>();\n    let v = project<labels>(s);\n    let q = compare<poses>(a, s);\n    \
    let d = derive(s, w);\n    let m = import(t, r);\n    let k = reachability(m);\n    \
    let g = check<low_height>(s, 3);\n    let l = imply<sticker_test>(h);\n    let n = cost_value(to_cost<Full, generator>(1));\n    \
    let u = unknown<Verdict<Full, labels>>(\"not read\");\n    \
    return match v {\n        total(lv) => cost_value(c);\n        partial(_, missing) => missing.off;\n        undefined(_) => 0;\n    };\n}\n";

// CT-LG2-01: algebraic matches account for every constructor; generics and inference cannot erase a model, level,
// mode, precision, unit or effect requirement.

/// CT-LG2-01 (part: the match clause on the draft's literal matches; the constructor matches are
/// `ct_lg2_01_cases_cover_every_constructor_in_source` and the end tests' verdicts `end_tests.rs`): a literal match
/// ends with a `_` arm, so no alternative is left out, even where the literal arms happen to be exhaustive; a
/// constructor pattern on a `Nat` lowers as a case, which the checker refuses at its scrutinee.
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
    assert_eq!(at(verdict), (TYPE_MISMATCH, "0".into()));
    // Literal and constructor arms do not mix.
    let mixed =
        "fn f(v: Nat) -> Nat effects [] {\n    return match v {\n        yes(p) => 1;\n        2 => 2;\n        \
                 _ => 3;\n    };\n}\n";
    assert!(refused_at_lowering(mixed).contains("all constructor patterns or all literals"));
}

/// CT-LG2-01 (part: the source half of the constructor matches and of the puzzle, level and unit indices through
/// generics; the IR half is `c600-check`'s `tower.rs`; not run: the run-time half (T1c)). A case accounts for every
/// constructor of a declared or built-in sum type, and generics keep every puzzle, level and unit index.
#[test]
fn ct_lg2_01_cases_cover_every_constructor_in_source() {
    let shape_fn =
        |result: &str, body: &str| format!("{SHAPE}fn f(s: Shape) -> {result} effects [] {{\n    return {body};\n}}\n");
    let case = |arms: &[&str]| {
        let arms: String = arms.iter().map(|a| format!("        {a}\n")).collect();
        format!("match s {{\n{arms}    }}")
    };
    // A declared sum type: a case that names every constructor, or has a `_` arm, is admitted.
    admitted(&shape_fn("Nat", &case(&["dot() => 0;", "line(n) => n;"])));
    admitted(&shape_fn("Nat", &case(&["line(n) => n;", "_ => 0;"])));
    // A case that leaves a constructor out has no `_` arm to take it.
    let r = report(&shape_fn("Nat", &case(&["line(n) => n;"])));
    let d = &r.diagnostics[0];
    assert_eq!(r.diagnostics.len(), 1);
    let m = d.mismatch.as_ref().unwrap();
    assert_eq!(
        (d.code, d.path.as_str(), m.expected.as_str(), m.supplied.as_str()),
        (TYPE_MISMATCH, "root", "dot | line", "line")
    );
    // A pattern names a constructor of the scrutinee's type and binds each payload value.
    assert!(codes(&shape_fn("Nat", &case(&["dot() => 0;", "arc(n) => n;"]))).contains(&TYPE_MISMATCH));
    assert_eq!(codes(&shape_fn("Nat", &case(&["dot() => 0;", "line(n, m) => n;"]))), [TYPE_MISMATCH]);
    // A case on a type that is no sum type.
    let on_nat = format!("{SHAPE}fn f(n: Nat) -> Nat effects [] {{\n    return match n {{\n        dot() => 0;\n        _ => 1;\n    }};\n}}\n");
    assert_eq!(at(&on_nat), (TYPE_MISMATCH, "0".into()));
    // Construction: the constructor's payload in number and type.
    let made = |body: &str| format!("{SHAPE}fn f() -> Shape effects [] {{\n    return {body};\n}}\n");
    admitted(&made("line(2)"));
    admitted(&made("dot()"));
    assert_eq!(codes(&made("line()")), [TYPE_MISMATCH]);
    assert_eq!(at(&made("line(word[+1])")), (TYPE_MISMATCH, "0".into()));
    // A constructor the module does not declare is no name (the IR's `construct(Shape, "arc")` has no source form).
    assert!(refused_at_lowering(&made("arc(2)")).contains("`arc` is not bound"));
    // A built-in sum type: a partial result and a reachability claim.
    let pv = |body: &str| format!("fn f(v: Partial<Nat>) -> Nat effects [] {{\n    return {body};\n}}\n");
    let pcase = |arms: &[&str]| {
        let arms: String = arms.iter().map(|a| format!("        {a}\n")).collect();
        format!("match v {{\n{arms}    }}")
    };
    let all = ["total(n) => n;", "partial(n, _) => n;", "undefined(_) => 0;"];
    admitted(&pv(&pcase(&all)));
    assert_eq!(codes(&pv(&pcase(&all[..2]))), [TYPE_MISMATCH]);
    // The `missing` record of a partial result: `off`, `uncovered` and `collisions`.
    admitted(&pv(&pcase(&["total(_) => 0;", "partial(_, m) => m.collisions;", "undefined(_) => 0;"])));
    let claims: Vec<String> = ReachKind::ALL.iter().map(|k| format!("{}(_) => 1;", k.name())).collect();
    let claims: Vec<&str> = claims.iter().map(String::as_str).collect();
    let rv = |arms: &[&str]| {
        let arms: String = arms.iter().map(|a| format!("        {a}\n")).collect();
        format!("fn f(r: Reachability<Jumbling>) -> Nat effects [] {{\n    return match r {{\n{arms}    }};\n}}\n")
    };
    admitted(&rv(&claims));
    assert_eq!(codes(&rv(&claims[1..])), [TYPE_MISMATCH]);
    // Generics and inference keep the puzzle, the level and the unit.
    assert_eq!(through_generic(&Ty::Piece(Jumbling), &Ty::Piece(Full)), [IDENTITY_MISMATCH]);
    let lv = |l| Ty::LevelValue(Jumbling, l);
    assert_eq!(through_generic(&lv(Level::Poses), &lv(Level::Labels)), [LEVEL_MISMATCH]);
    let cost = |u| Ty::Cost(Full, u);
    assert_eq!(through_generic(&cost(CostUnit::Generator), &cost(CostUnit::UiAction)), [COST_UNIT_MIX]);
    assert_eq!(through_generic(&Ty::Piece(Full), &Ty::Piece(Full)), NONE);
    // A generic callable parameter keeps them too: `apply<T,E,e>(g: fn(T) -> E effects [e], x: T) -> E`.
    let apply = "fn apply<T,E,e>(g: fn(T) -> E effects [e], x: T) -> E effects [e] {\n    return g(x);\n}\n\n";
    let use_it = |result: &Ty| {
        let body = "apply(fn(x: LevelValue<Jumbling, labels>) -> Digest<Jumbling, labels> {\n        \
                    return digest(x);\n    }, v)";
        codes(&format!("{apply}{}", f(&[("v", &lv(Level::Labels))], result, "snapshot", body)))
    };
    assert_eq!(use_it(&Ty::Digest(Jumbling, Level::Labels)), NONE);
    assert_eq!(use_it(&Ty::Digest(Jumbling, Level::Poses)), [LEVEL_MISMATCH]);
    assert_eq!(use_it(&Ty::Digest(Full, Level::Labels)), [IDENTITY_MISMATCH]);
    // The mode follows the puzzle, and the alphabet the mode: a jumbling menu word stands for no Full word, through a
    // generic either.
    assert_eq!(every_way(&Ty::Word, &Ty::MenuWord), [INCOMPATIBLE_ALPHABET]);
}

/// CT-LG2-01 (part: precision, exactness, unit, effect and capability requirements through generic substitution and
/// inference; the puzzle, level and unit indices are `ct_lg2_01_cases_cover_every_constructor_in_source`; not run:
/// the run-time half (T1c)).
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
    // Capability: inference through the generic finds the file requirement but grants nothing (LG3.9); the file read
    // is an ambient read (owner, 16 November 2026, PHS.52).
    let source = format!(
        "edition \"6cs-draft-1\";\nmodule tool requires [file];\n\n{APPLY}\
         fn load(w: Word) -> Word effects [file] {{\n    let n = file_read<Nat>(\"data\");\n    return w;\n}}\n\n\
         command go(w: Word) -> Word effects [file] {{\n    return apply(load, w);\n}}\n"
    );
    let program = lowered(&source).unwrap_or_else(|e| panic!("{e}")).program;
    let run = |grant: &Grant| admit(&program, "tool", "go", Profile::ToolAuthoring, grant, &Policy::interim()).unwrap();
    assert_eq!(run(&Grant::none()).codes(), [AMBIENT_READ]);
    assert!(run(&Grant::of(&[Capability::File])).is_admitted());
}

/// CT-LG2-02 (part: the source half; the IR half is `c600-check`'s `tower.rs`; not run: the run-time half (T1c)). An
/// identity of one puzzle never stands for the other's, and a pose, a sticker-class value, a label digest and a
/// pose-table index are four types.
#[test]
fn ct_lg2_02_identities_keep_their_puzzle_and_kind() {
    let ids: [fn(Puzzle) -> Ty; 3] = [Ty::Piece, Ty::Sticker, Ty::Slot];
    for id in ids {
        assert_eq!(every_way(&id(Jumbling), &id(Full)), [IDENTITY_MISMATCH]);
        assert_eq!(every_way(&id(Full), &id(Jumbling)), [IDENTITY_MISMATCH]);
        assert_eq!(every_way(&id(Jumbling), &id(Jumbling)), NONE);
        assert_eq!(compared(&id(Full), &id(Jumbling)), [IDENTITY_MISMATCH]);
        assert_eq!(compared(&id(Full), &id(Full)), NONE);
        // A pose-table index is per revision and never an identity.
        assert_eq!(every_way(&id(Jumbling), &Ty::PoseTableIndex), [IDENTITY_MISMATCH]);
    }
    // A generic binding of two puzzles' pieces: `pick<T>(a: T, b: T) -> T`.
    let pick = "fn pick<T>(a: T, b: T) -> T effects [] {\n    return a;\n}\n\n";
    let both = f(&[("a", &Ty::Piece(Full)), ("b", &Ty::Piece(Jumbling))], &Ty::Piece(Full), "", "pick(a, b)");
    let r = report(&format!("{pick}{both}"));
    assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    let m = d.mismatch.as_ref().unwrap();
    assert_eq!(
        (d.code, d.function.as_str(), d.path.as_str(), m.expected.as_str(), m.supplied.as_str()),
        (IDENTITY_MISMATCH, "f", "1", "Piece<Full>", "Piece<Jumbling>")
    );
    // Pose, sticker class, label digest and pose-table index are not interchangeable.
    let kinds = [
        Ty::Pose,
        Ty::LevelValue(Jumbling, Level::StickerClass),
        Ty::Digest(Jumbling, Level::Labels),
        Ty::PoseTableIndex,
    ];
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            let found = as_argument(a, b);
            if i == j {
                assert_eq!(found, NONE, "{a:?}");
            } else if (a, b) == (&Ty::Pose, &Ty::PoseTableIndex) {
                assert_eq!(found, [IDENTITY_MISMATCH]);
            } else {
                assert_eq!(found, [TYPE_MISMATCH], "{a:?} <- {b:?}");
            }
        }
    }
    // Digests compare at one puzzle and one level.
    let digest = Ty::Digest;
    assert_eq!(compared(&digest(Jumbling, Level::Labels), &digest(Jumbling, Level::Labels)), NONE);
    assert_eq!(compared(&digest(Jumbling, Level::Labels), &digest(Jumbling, Level::Poses)), [LEVEL_MISMATCH]);
    assert_eq!(compared(&digest(Full, Level::Labels), &digest(Jumbling, Level::Labels)), [IDENTITY_MISMATCH]);
    assert_eq!(every_way(&digest(Full, Level::Labels), &digest(Full, Level::Position)), [LEVEL_MISMATCH]);
    // Records and collections keep the index of every element.
    let list = |ty| Ty::List(Box::new(exact(ty)));
    assert_eq!(every_way(&list(Ty::Piece(Jumbling)), &list(Ty::Piece(Full))), [IDENTITY_MISMATCH]);
    let rec = |ty| Ty::Record(vec![("piece".into(), exact(ty))]);
    assert_eq!(as_argument(&rec(Ty::Sticker(Jumbling)), &rec(Ty::Sticker(Full))), [IDENTITY_MISMATCH]);
    // The types lower as written, and the matchers above know every one of them.
    assert!(base_typed(EVERYTHING));
    assert!(!base_typed(TOWER));
    let lowered = lowered(TOWER).unwrap();
    let rows = expressions(&lowered).into_iter().filter(|e| family(e) == "kernel row").count();
    assert_eq!(rows, 7, "the rows of `snapshot`, `project`, `compare`, `derive`, `import`, `reachability`, `check`");
    assert!(check(&lowered.program).unwrap().is_admitted(), "{:#?}", check(&lowered.program).unwrap().diagnostics);
}

/// CT-LG2-03 (part: the source half; the IR half is `c600-check`'s `tower.rs`; not run: the run-time half (T1c),
/// where a read of labels off the lattice returns `partial` or `undefined`). A projection's partial result is cased,
/// nothing lifts a coarser level to a finer one, and a class is no equality.
#[test]
fn ct_lg2_03_projection_is_partial_and_nothing_lifts() {
    let project = |level: Level, of: Puzzle| {
        let s = Ty::State(of);
        f(&[("s", &s)], &partial(Ty::LevelValue(of, level)), "snapshot", &format!("project<{}>(s)", level.name()))
    };
    for level in Level::ALL {
        // The jumbling puzzle holds every level; 600-cell-Full holds the labels and the position only.
        assert_eq!(codes(&project(level, Jumbling)), NONE, "{level:?}");
        let found = codes(&project(level, Full));
        if level.held_by(Full) {
            assert_eq!(found, NONE, "{level:?}");
        } else {
            assert_eq!(found, [LEVEL_MISMATCH], "{level:?}");
        }
    }
    // The partial result is no level value: it is used through a case.
    let s = Ty::State(Jumbling);
    let value = Ty::LevelValue(Jumbling, Level::Labels);
    let direct = f(&[("s", &s)], &value, "snapshot", "project<labels>(s)");
    assert_eq!(
        only(&direct),
        types(LEVEL_MISMATCH, "LevelValue<Jumbling, labels>", "Partial<LevelValue<Jumbling, labels>>")
    );
    let digest = Ty::Digest(Jumbling, Level::Labels);
    assert_eq!(codes(&f(&[("s", &s)], &digest, "snapshot", "digest(project<labels>(s))")), [LEVEL_MISMATCH]);
    let one = "fn one(d: Digest<Jumbling, labels>) -> Nat effects [] {\n    return 1;\n}\n\n";
    let cased = "match project<labels>(s) {\n        total(v) => one(digest(v));\n        \
                 partial(_, m) => m.off;\n        undefined(_) => 0;\n    }";
    admitted(&format!("{one}{}", f(&[("s", &s)], &Ty::Nat, "snapshot", cased)));
    // No map leads back to a finer level, and a level value is no state, so nothing projects it to the poses.
    let lifted = f(&[("v", &value)], &partial(Ty::LevelValue(Jumbling, Level::Poses)), "snapshot", "project<poses>(v)");
    assert_eq!(codes(&lifted), [REACHABILITY_MISSING]);
    assert_eq!(every_way(&Ty::LevelValue(Jumbling, Level::Poses), &value), [LEVEL_MISMATCH]);
    // A class is no equality: level values have no exact equality, and a comparison at the labels gives a labels
    // verdict, which no poses verdict takes.
    assert_eq!(compared(&value, &value), [TYPE_MISMATCH]);
    let compare = |level: Level, result: Ty| {
        let ab = [("a", &s), ("b", &s)];
        codes(&f(&ab, &result, "snapshot", &format!("compare<{}>(a, b)", level.name())))
    };
    let verdict = |l| Ty::Verdict(Jumbling, l, None);
    assert_eq!(compare(Level::Labels, partial(verdict(Level::Labels))), NONE);
    // The IR's declared row type has no source form: the row's own type meets the declared result, a level apart.
    assert_eq!(compare(Level::Labels, partial(verdict(Level::Poses))), [LEVEL_MISMATCH]);
    assert_eq!(every_way(&verdict(Level::Poses), &verdict(Level::Labels)), [LEVEL_MISMATCH]);
    // States of two puzzles never compare, and 600-cell-Full compares at its own levels only.
    let mixed = f(&[("a", &Ty::State(Full)), ("b", &s)], &Ty::Unit, "snapshot", "compare<labels>(a, b)");
    assert_eq!(at(&mixed), (IDENTITY_MISMATCH, "1".into()));
    let fulls = f(&[("a", &Ty::State(Full)), ("b", &Ty::State(Full))], &Ty::Unit, "snapshot", "compare<poses>(a, b)");
    assert_eq!(at(&fulls), (LEVEL_MISMATCH, "0".into()));
    // A projection names its level, which lowering reads as an index; a level is no type.
    let unnamed = f(&[("s", &s)], &Ty::Unit, "snapshot", "project(s)");
    assert!(refused_at_lowering(&unnamed).contains("a level"));
    assert!(refused_at_lowering("fn f(l: labels) -> Nat effects [] {\n    return 1;\n}\n").contains("type argument"));
    for name in ["lift", "labels"] {
        let declaration = format!("fn f(w: Word) -> Word effects [] {{\n    return {name}(w);\n}}\n");
        assert!(refused_at_lowering(&declaration).contains("not bound"), "{name}");
    }
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

/// CT-LG2-06 (part: the budget half on the draft's single `Budget` unit; the puzzle and unit indices of menu letters,
/// retained generators and UI actions are `ct_lg2_06_costs_keep_their_puzzle_and_unit` and the seconds half is
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

/// CT-LG2-06 (part: the source half of the puzzle and unit indices; the IR half is `c600-check`'s `tower.rs`; not run:
/// the run-time half (T1c); the budget half is `ct_lg2_06_a_budget_mixes_with_integers_only` and the seconds half
/// `ct_lg2_06_a_clock_reading_is_no_deterministic_budget`). Costs add, order and compare only with costs of their
/// puzzle and unit.
#[test]
fn ct_lg2_06_costs_keep_their_puzzle_and_unit() {
    use CostUnit::*;
    let c = Ty::Cost;
    let op =
        |op: &str, a: &Ty, b: &Ty, result: &Ty| codes(&f(&[("a", a), ("b", b)], result, "", &format!("{op}(a, b)")));
    let same = c(Full, Generator);
    assert_eq!(op("exact_add", &same, &same, &same), NONE);
    assert_eq!(op("exact_less", &same, &same, &Ty::Bool), NONE);
    assert_eq!(op("exact_eq", &same, &same, &Ty::Bool), NONE);
    for other in [c(Full, MenuLetter), c(Full, UiAction), c(Jumbling, Generator), Ty::Nat] {
        assert_eq!(op("exact_add", &same, &other, &Ty::Unit), [COST_UNIT_MIX], "{other:?}");
        assert_eq!(op("exact_less", &same, &other, &Ty::Bool), [COST_UNIT_MIX], "{other:?}");
        assert_eq!(op("exact_eq", &same, &other, &Ty::Bool), [COST_UNIT_MIX], "{other:?}");
        assert_eq!(every_way(&same, &other), [COST_UNIT_MIX], "{other:?}");
    }
    // A count first: the refusal is at the count.
    assert_eq!(at(&f(&[("a", &Ty::Nat), ("b", &same)], &Ty::Unit, "", "exact_add(a, b)")), (COST_UNIT_MIX, "0".into()));
    // A product of costs is no cost.
    assert_eq!(op("exact_mul", &same, &same, &Ty::Unit), [TYPE_MISMATCH, TYPE_MISMATCH]);
    // A cost starts from an exact count and gives one back.
    admitted(&f(&[], &same, "", "to_cost<Full, generator>(3)"));
    admitted(&f(&[("a", &same)], &Ty::Nat, "", "cost_value(a)"));
    let shown =
        "fn f(n: Display<Nat>) -> Cost<Full, generator> effects [] {\n    return to_cost<Full, generator>(n);\n}\n";
    let r = report(shown);
    assert!(r.diagnostics.iter().any(|d| (d.code, d.path.as_str()) == (EXACTNESS_VIOLATION, "0")), "{r:#?}");
    assert_eq!(codes(&f(&[], &c(Full, MenuLetter), "", "to_cost<Full, generator>(3)")), [COST_UNIT_MIX]);
    // The indices are static: a cost names its puzzle and its unit, in this order.
    for wrong in ["to_cost<generator, Full>(3)", "to_cost<Full>(3)", "to_cost(3)"] {
        assert!(refused_at_lowering(&f(&[], &same, "", wrong)).contains("a puzzle and a cost unit"), "{wrong}");
    }
    assert!(refused_at_lowering("fn f(c: Cost<Full>) -> Nat effects [] {\n    return 1;\n}\n").contains("cost unit"));
}

/// CT-LG2-07 (part: the source half; the IR half is `c600-check`'s `tower.rs`; not run: the run-time half (T1c)). Raw
/// pose data enters as a state only through `import` with a reachability claim, and a jumbling claim is no claim
/// about 600-cell-Full.
#[test]
fn ct_lg2_07_raw_pose_data_is_no_state() {
    let state = Ty::State(Jumbling);
    let record = Ty::Record(vec![("poses".into(), exact(Ty::PoseTable))]);
    for raw in [Ty::PoseTable, record, Ty::LevelValue(Jumbling, Level::Poses)] {
        assert_eq!(every_way(&state, &raw), [REACHABILITY_MISSING], "{raw:?}");
    }
    let import = |claim: &Ty, body: &str| f(&[("t", &Ty::PoseTable), ("r", claim)], &state, "snapshot", body);
    let claim = Ty::Reachability(Jumbling);
    admitted(&import(&claim, "import(t, r)"));
    // Without a claim, form validation establishes no reachability.
    assert_eq!(at(&import(&claim, "import(t)")), (REACHABILITY_MISSING, "root".into()));
    assert_eq!(at(&import(&Ty::Evidence, "import(t, r)")), (REACHABILITY_MISSING, "1".into()));
    assert_eq!(at(&import(&Ty::Reachability(Full), "import(t, r)")), (IDENTITY_MISMATCH, "1".into()));
    assert!(refused_at_lowering(&import(&claim, "import(t, r, r)")).contains("`import` takes 2 arguments"));
    // A jumbling witness does not assert retained reachability.
    assert_eq!(every_way(&Ty::Reachability(Full), &Ty::Reachability(Jumbling)), [IDENTITY_MISMATCH]);
    let claim_of = |of: Puzzle, declared: Puzzle| {
        codes(&f(&[("s", &Ty::State(of))], &Ty::Reachability(declared), "snapshot", "reachability(s)"))
    };
    assert_eq!(claim_of(Full, Full), NONE);
    // The IR's declared row type has no source form: the row's own type meets the declared result, a puzzle apart.
    assert_eq!(claim_of(Jumbling, Full), [IDENTITY_MISMATCH]);
    admitted(&f(&[], &Ty::State(Full), "snapshot", "snapshot<Full>()"));
    assert_eq!(codes(&f(&[], &Ty::State(Full), "snapshot", "snapshot<Jumbling>()")), [IDENTITY_MISMATCH]);
    // A program constructs a claim from evidence it was given, never the evidence, a certificate or a derivation.
    admitted(&f(&[("e", &Ty::Evidence)], &claim, "", "witnessed<Reachability<Jumbling>>(e)"));
    assert_eq!(codes(&f(&[], &Ty::Evidence, "", "yes<Evidence>()")), [TYPE_MISMATCH]);
    let holds = Ty::Holds(Jumbling, Goal::ExactPose);
    assert_eq!(codes(&f(&[], &holds, "", "yes<Holds<Jumbling, exact_pose>>()")), [TYPE_MISMATCH]);
    let derived = "derived<Derivation<Jumbling>>(s)";
    assert_eq!(codes(&f(&[("s", &state)], &Ty::Derivation(Jumbling), "", derived)), [TYPE_MISMATCH]);
    // Evidence has no constructor name to call: `evidence` is no built-in constructor.
    assert!(refused_at_lowering(&f(&[], &Ty::Evidence, "", "evidence<Evidence>()")).contains("type arguments"));
}

/// CT-LG2-08 (part, static: a state's mode is its puzzle's, Jumble off for 600-cell-Full and on for the jumbling
/// puzzle, so the IR draft has no mode type, and `derive`'s word of the other puzzle's alphabet is
/// `LG2:type.mode_mismatch` in `ct_lg2_11_derive_takes_literal_words_only`; LX-8's fixture menu letters do not lower;
/// menu words of the run's pinned menu lower as an LP2 draft, `menu_words.rs`; the run-time refusal at invocation is
/// `c600-run`'s `ct_lg2_08_a_mode_change_refuses_at_invocation` and the command layer's, after a real mode change, is
/// in `c600-command`'s `jumble_on.rs`).
#[test]
fn ct_lg2_08_fixture_menu_letters_and_mode_types_do_not_lower() {
    let fixture = refused_at_lowering("fn f() -> Word effects [] {\n    return menu[2:spin];\n}\n");
    assert!(fixture.contains("cap:item"), "{fixture}");
    for ty in ["Mode", "Revision"] {
        let message = refused_at_lowering(&format!("fn f(p: {ty}) -> Int effects [] {{\n    return 1;\n}}\n"));
        assert!(message.contains(&format!("unknown type `{ty}`")), "{ty}: {message}");
    }
}

/// A verdict of the jumbling puzzle at the labels, of `check<sticker_test>(s)`.
fn sticker_verdict() -> Ty {
    Ty::Verdict(Jumbling, Level::Labels, Some(Goal::StickerTest))
}

/// CT-LG2-09 (part: the source half; the IR half is `c600-check`'s `tower.rs`; LB3.2's end tests' verdicts (LP2
/// draft) run unknown that never passes as false in `end_tests.rs`; not run: the run-time half (T1c)). Unknown never
/// passes as false, an off-lattice projection never passes as unknown, and a goal never weakens its level.
#[test]
fn ct_lg2_09_unknown_is_no_false_and_goals_keep_their_level() {
    let v = sticker_verdict();
    let over = |result: &Ty, body: &str| f(&[("v", &v)], result, "", body);
    // A verdict is no boolean, and a boolean no verdict.
    assert_eq!(codes(&over(&Ty::Nat, "if(v, 1, 0)")), [VERDICT_REQUIRED]);
    assert_eq!(every_way(&Ty::Bool, &v), [VERDICT_REQUIRED]);
    assert_eq!(every_way(&v, &Ty::Bool), [VERDICT_REQUIRED]);
    assert_eq!(compared(&v, &v), [VERDICT_REQUIRED]);
    // A status is no condition either.
    let status = only("fn f() -> Nat effects [budget] {\n    return if(budget_status(), 1, 2);\n}\n");
    assert_eq!(status, mismatch("Bool", "Status"));
    // A `_` arm takes `unknown` alone or not at all.
    let case = |arms: &[&str]| {
        let arms: String = arms.iter().map(|a| format!("        {a}\n")).collect();
        over(&Ty::Nat, &format!("match v {{\n{arms}    }}"))
    };
    let (yes, no, unknown) = ("yes(_) => 1;", "no(_) => 0;", "unknown(_) => 2;");
    admitted(&case(&[yes, no, unknown]));
    admitted(&case(&[yes, no, "_ => 2;"]));
    admitted(&case(&[unknown, "_ => 0;"]));
    assert_eq!(at(&case(&[yes, "_ => 0;"])), (VERDICT_REQUIRED, "root".into()));
    assert_eq!(codes(&case(&[yes, unknown])), [TYPE_MISMATCH]);
    // A `yes` of a goal's check carries the goal's certificate.
    let holds = Ty::Holds(Jumbling, Goal::StickerTest);
    let cert = "match v {\n        yes(h) => h;\n        no(_) => h0;\n        unknown(_) => h0;\n    }";
    admitted(&f(&[("v", &v), ("h0", &holds)], &holds, "", cert));
    // A goal's check gives its verdict, which a program cases.
    let checked =
        "match check<sticker_test>(s) {\n        yes(_) => 1;\n        no(_) => 0;\n        unknown(_) => 2;\n    }";
    admitted(&f(&[("s", &Ty::State(Jumbling))], &Ty::Nat, "snapshot", checked));
    // A program makes an unknown verdict with its reason, never a `yes` without evidence.
    admitted(&f(&[], &Ty::Verdict(Full, Level::Labels, None), "", "unknown<Verdict<Full, labels>>(\"not read\")"));
    assert_eq!(
        codes(&f(&[], &Ty::Verdict(Full, Level::Labels, None), "", "yes<Verdict<Full, labels>>(\"y\")")),
        [TYPE_MISMATCH]
    );
    // An off-lattice projection is no unknown: a partial verdict is no verdict, and its case has no `unknown`.
    let pv = partial(Ty::Verdict(Jumbling, Level::Labels, None));
    let over_partial = |result: &Ty, body: &str| codes(&f(&[("v", &pv)], result, "", body));
    assert_eq!(over_partial(&Ty::Nat, "if(v, 1, 0)"), [LEVEL_MISMATCH]);
    assert_eq!(over_partial(&Ty::Verdict(Jumbling, Level::Labels, None), "v"), [LEVEL_MISMATCH]);
    let off = "match v {\n        unknown(_) => 2;\n        _ => 0;\n    }";
    assert_eq!(over_partial(&Ty::Nat, off), [TYPE_MISMATCH]);
    // A goal never weakens its level: a certificate or a verdict of another goal stands for none of this one's.
    let goal = |g| Ty::Holds(Jumbling, g);
    assert_eq!(every_way(&goal(Goal::ExactPose), &goal(Goal::StickerTest)), [LEVEL_MISMATCH]);
    assert_eq!(every_way(&goal(Goal::StickerTest), &goal(Goal::ExactPose)), [LEVEL_MISMATCH]);
    let exact_pose = Ty::Verdict(Jumbling, Level::Poses, Some(Goal::ExactPose));
    assert_eq!(every_way(&exact_pose, &v), [LEVEL_MISMATCH]);
}

/// The codes of `imply` from a certificate of `from` to `to`, of a state of `puzzle`.
fn implied(puzzle: Puzzle, from: Goal, to: Goal) -> Vec<&'static str> {
    let bound = if to.takes_bound() { ", 3" } else { "" };
    let body = format!("imply<{}>(h{bound})", to.name());
    codes(&f(&[("h", &Ty::Holds(puzzle, from))], &Ty::Holds(puzzle, to), "", &body))
}

/// CT-LG2-10 (part: the source half; the IR half is `c600-check`'s `tower.rs`; not run: the run-time half (T1c), with
/// the goal fixtures). `imply` follows LG2.11's sourced chain and checkpoint implications and nothing else; equal
/// labels do not require equal poses, and a label checkpoint supplies no retained word.
#[test]
fn ct_lg2_10_imply_follows_the_sourced_chain() {
    use Goal::*;
    for from in Goal::ALL {
        for to in Goal::ALL {
            let found = implied(Jumbling, from, to);
            if from.implies(to) {
                assert_eq!(found, NONE, "{from:?} => {to:?}");
            } else {
                assert_eq!(found, [LEVEL_MISMATCH], "{from:?} => {to:?}");
            }
        }
    }
    let note = |from: Goal, to: Goal| {
        let body = format!("imply<{}>(h)", to.name());
        let r = report(&f(&[("h", &Ty::Holds(Jumbling, from))], &Ty::Holds(Jumbling, to), "", &body));
        assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
        r.diagnostics[0].note.clone()
    };
    assert!(note(LabelCheckpoint, RetainedCheckpoint).contains("supplies no retained word"));
    assert!(note(StickerTest, LabelCheckpoint).contains("use check(label_checkpoint, s)"));
    // The sticker test holds of 600-cell-Full states; the goals it implies are defined on jumbling states only.
    assert_eq!(implied(Full, StickerTest, StickerTest), NONE);
    assert_eq!(implied(Full, StickerTest, LatticePlacement), [IDENTITY_MISMATCH]);
    // The low-height bound is an exact `Nat`, and `imply` to the low-height goal takes it.
    let low = |n: &str| {
        format!(
            "fn f(h: Holds<Jumbling, exact_pose>, n: {n}) -> Holds<Jumbling, low_height> effects [] {{\n    \
                 return imply<low_height>(h, n);\n}}\n"
        )
    };
    let r = report(&low("Display<Nat>"));
    assert!(r.diagnostics.iter().any(|d| (d.code, d.path.as_str()) == (EXACTNESS_VIOLATION, "1")), "{r:#?}");
    assert_eq!(at(&low("Int")), (TYPE_MISMATCH, "1".into()));
    let unbounded =
        f(&[("h", &Ty::Holds(Jumbling, ExactPose))], &Ty::Holds(Jumbling, LowHeight), "", "imply<low_height>(h)");
    assert!(refused_at_lowering(&unbounded).contains("`imply<low_height>` takes 2 arguments"));
    // Equal labels do not require equal poses: a labels verdict is no poses verdict, and a label checkpoint's
    // certificate implies no exact pose.
    let poses = Ty::Verdict(Jumbling, Level::Poses, Some(ExactPose));
    assert_eq!(every_way(&poses, &Ty::Verdict(Jumbling, Level::Labels, Some(LabelCheckpoint))), [LEVEL_MISMATCH]);
    assert_eq!(implied(Jumbling, LabelCheckpoint, ExactPose), [LEVEL_MISMATCH]);
    // A label checkpoint's certificate is no word.
    assert_eq!(as_argument(&Ty::Word, &Ty::Holds(Jumbling, LabelCheckpoint)), [TYPE_MISMATCH]);
    // `check<g>(s)` reads the goal at its own level, on the puzzles the goal is defined on.
    for goal in Goal::ALL {
        for puzzle in Puzzle::ALL {
            let bound = if goal.takes_bound() { ", 3" } else { "" };
            let verdict = Ty::Verdict(puzzle, goal.level(), Some(goal));
            let body = format!("check<{}>(s{bound})", goal.name());
            let found = codes(&f(&[("s", &Ty::State(puzzle))], &verdict, "snapshot", &body));
            if goal.puzzles().contains(&puzzle) {
                assert_eq!(found, NONE, "{goal:?} on {puzzle:?}");
            } else {
                assert_eq!(found, [IDENTITY_MISMATCH], "{goal:?} on {puzzle:?}");
            }
        }
    }
    let unbounded = f(&[("s", &Ty::State(Jumbling))], &Ty::Unit, "snapshot", "check<low_height>(s)");
    assert!(refused_at_lowering(&unbounded).contains("`check<low_height>` takes 2 arguments"));
    // There is no goal type of its own: a goal is an index.
    let message = refused_at_lowering("fn f(g: ExactPose) -> Nat effects [] {\n    return 1;\n}\n");
    assert!(message.contains("unknown type `ExactPose`"), "{message}");
}

/// CT-LG2-11 (part: the source half of the derivation binding and of the key's puzzle; the IR half is `c600-check`'s
/// `tower.rs`; not run: the run-time half (T1c), and an execution binding, which no program has (a program cannot
/// execute)). A derivation takes a literal word of its puzzle's alphabet, never a word equality key, and keys of two
/// puzzles do not compare.
#[test]
fn ct_lg2_11_derive_takes_literal_words_only() {
    let derive = |of: Puzzle, w: &Ty| {
        let r = report(&f(&[("s", &Ty::State(of)), ("w", w)], &Ty::Derivation(of), "snapshot", "derive(s, w)"));
        assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
        let d = &r.diagnostics[0];
        (d.code, d.path.clone(), d.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone())))
    };
    let pair = |a: &str, b: &str| Some((a.to_string(), b.to_string()));
    assert_eq!(
        derive(Full, &Ty::WordEqualityKey(Full)),
        (WORD_KEY_NOT_EXECUTABLE, "1".into(), pair("Word", "WordEqualityKey<Full>"))
    );
    assert_eq!(
        derive(Jumbling, &Ty::WordEqualityKey(Jumbling)),
        (WORD_KEY_NOT_EXECUTABLE, "1".into(), pair("MenuWord", "WordEqualityKey<Jumbling>"))
    );
    // The alphabet follows the puzzle: Jumble off for 600-cell-Full, on for the jumbling puzzle (LG2.8).
    let (code, path, _) = derive(Full, &Ty::MenuWord);
    assert_eq!((code, path.as_str()), (MODE_MISMATCH, "1"));
    assert_eq!(derive(Jumbling, &Ty::Word).0, MODE_MISMATCH);
    // A literal word is a word: `derive` of a literal Full word lowers and checks.
    admitted(&f(&[("s", &Ty::State(Full))], &Ty::Derivation(Full), "snapshot", "derive(s, word[+1, -2])"));
    admitted(&f(&[("s", &Ty::State(Jumbling))], &Ty::Derivation(Jumbling), "snapshot", "derive(s, menu[2:5])"));
    // A derivation is cased: `derived` carries the state, `refused` its record.
    let sw = [("s", &Ty::State(Full)), ("w", &Ty::Word)];
    let cased = "match derive(s, w) {\n        derived(_) => 0;\n        refused(r) => r.prefix;\n    }";
    admitted(&f(&sw, &Ty::Nat, "snapshot", cased));
    assert_eq!(codes(&f(&sw, &Ty::State(Full), "snapshot", "derive(s, w)")), [TYPE_MISMATCH]);
    // A key keeps its word's puzzle.
    let key = |w: &Ty, result: &Ty| codes(&f(&[("w", w)], result, "", "word_key(w)"));
    assert_eq!(key(&Ty::Word, &Ty::WordEqualityKey(Full)), NONE);
    assert_eq!(key(&Ty::MenuWord, &Ty::WordEqualityKey(Jumbling)), NONE);
    assert_eq!(key(&Ty::MenuWord, &Ty::WordEqualityKey(Full)), [IDENTITY_MISMATCH]);
    assert_eq!(every_way(&Ty::WordEqualityKey(Jumbling), &Ty::WordEqualityKey(Full)), [IDENTITY_MISMATCH]);
    assert_eq!(compared(&Ty::WordEqualityKey(Full), &Ty::WordEqualityKey(Jumbling)), [IDENTITY_MISMATCH]);
    assert_eq!(compared(&Ty::WordEqualityKey(Full), &Ty::WordEqualityKey(Full)), NONE);
}

/// CT-LG1-03 (part: the identity tower's source forms (T1 drafts) keep the canonical layout: a program written with
/// other spacing formats to one text, which formats to itself and lowers to the same program).
#[test]
fn ct_lg1_03_the_towers_forms_format_canonically() {
    let canonical = format(&parse(TOWER.as_bytes())).expect("a complete parse formats");
    // Type arguments join with `,` alone, as every type's do in the layout (`Map<Nat,Text>`).
    for form in [
        "type Shape {\n    dot();\n    line(Nat);\n}\n",
        "Holds<Jumbling,exact_pose>",
        "project<labels>(s)",
        "to_cost<Full,generator>(1)",
        "unknown<Verdict<Full,labels>>(",
        "        partial(_, missing) => missing.off;\n",
    ] {
        assert!(canonical.contains(form), "{form}\n{canonical}");
    }
    assert_eq!(format(&parse(canonical.as_bytes())).as_deref(), Some(canonical.as_str()));
    let spaced = TOWER
        .replace("<labels>", "< labels >")
        .replace("State<Jumbling>,", "State< Jumbling > ,")
        .replace("partial(_, missing)", "partial( _ ,missing )")
        .replace("    dot();\n    line(Nat);\n", "dot( ) ; line( Nat ) ;\n");
    assert_ne!(spaced, TOWER);
    let reparsed = parse(spaced.as_bytes());
    assert!(reparsed.diagnostics.is_empty(), "{:#?}", reparsed.diagnostics);
    assert_eq!(format(&reparsed).as_deref(), Some(canonical.as_str()));
    assert!(reparsed.same_program(&parse(canonical.as_bytes())));
    assert_eq!(lowered(&spaced).unwrap().program, lowered(&canonical).unwrap().program);
}

/// CT-LG2-11 (part: the proposal binding statically; not run: an execution binding, which no program has (a program
/// cannot execute); the protection read is in `c600-run`'s `tests/snapshot_arg.rs`, the key's half is
/// `ct_lg2_11_a_word_equality_key_is_never_literal_letters`, the derivation binding's source half is
/// `ct_lg2_11_derive_takes_literal_words_only` and the run-time half is `c600-run`'s
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
/// the word operations (derivation) and a `Word` parameter; the key indexed by its word's puzzle and `derive`'s
/// binding are `ct_lg2_11_derive_takes_literal_words_only`; not run: an execution binding, which no program has): a
/// key is never accepted where literal letters are required, and no source form converts it back.
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
        assert_eq!(types, Some(("Word".to_string(), "WordEqualityKey<Full>".to_string())), "{declaration}");
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

/// CT-LB1-06 (part: LB1.10's `TraceEqualityKey` and `trace_key` (LP2 drafts) in source, against the proposal binding,
/// the word operations, a `Word` parameter and the word key; the run-time halves are `c600-run`'s `ct_lb1_06_*`, Full
/// and jumbling, and the model's jumbling relation is `c600-jumble`'s): a trace key is never literal letters and never
/// a word key, and no source form converts one into the other.
#[test]
fn ct_lb1_06_a_trace_key_is_neither_letters_nor_a_word_key() {
    let key = "trace_key(word[+1, -2])";
    let uses = [
        format!(
            "fn f() -> Receipt effects [preview] {{
    return propose({key}, target(\"t\"));
}}
"
        ),
        format!(
            "fn f() -> Word effects [] {{
    return inverse({key});
}}
"
        ),
        format!(
            "fn f() -> Word effects [] {{
    return compose(word[+1], {key});
}}
"
        ),
        format!(
            "fn g(w: Word) -> Word effects [] {{
    return w;
}}

fn f() -> Word effects [] {{
    return g({key});
}}
"
        ),
        format!(
            "fn f() -> Word effects [] {{
    return {key};
}}
"
        ),
        format!(
            "fn f() -> Unit effects [workspace] {{
    return write_workspace<Word>(\"w\", {key});
}}
"
        ),
        "fn f(w: Word) -> WordEqualityKey effects [] {
    return word_key(trace_key(w));
}
"
        .to_string(),
    ];
    for declaration in &uses {
        let (code, types) = only(declaration);
        assert_eq!(code, WORD_KEY_NOT_EXECUTABLE, "{declaration}");
        assert_eq!(types, Some(("Word".to_string(), "TraceEqualityKey".to_string())), "{declaration}");
    }
    admitted(
        "fn f(w: Word) -> TraceEqualityKey effects [] {
    return trace_key(inverse(w));
}
",
    );
    admitted(
        "fn f(a: TraceEqualityKey, b: TraceEqualityKey) -> Bool effects [] {
    return exact_eq(a, b);
}
",
    );
    // A trace key takes literal letters, not a word key; and neither key is the other.
    let (code, types) = only(
        "fn f(w: Word) -> TraceEqualityKey effects [] {
    return trace_key(word_key(w));
}
",
    );
    assert_eq!(
        (code, types),
        (WORD_KEY_NOT_EXECUTABLE, Some(("Word".to_string(), "WordEqualityKey<Full>".to_string())))
    );
    for declaration in [
        "fn f(w: Word) -> WordEqualityKey effects [] {
    return trace_key(w);
}
",
        "fn f(w: Word) -> TraceEqualityKey effects [] {
    return word_key(w);
}
",
        "fn f(w: Word) -> Bool effects [] {
    return exact_eq(word_key(w), trace_key(w));
}
",
    ] {
        assert_eq!(only(declaration).0, TYPE_MISMATCH, "{declaration}");
    }
}

/// CT-LG2-13 (part: the empty-word half statically, with the Full key (LP2 draft); the jumbling key and its domain
/// test at a straddling cap are `c600-jumble-tool`'s `lb3.rs`; the run-time halves are `c600-run`'s
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
