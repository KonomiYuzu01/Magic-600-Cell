//! `LG2:type.mismatch` (LG2.1): a value whose base type does not fit its use is a diagnostic with the expected and
//! supplied types, not malformed IR. One fault gives one finding, the base type comes first where an exactness rule
//! would also apply, and the report keeps LG4.3's order. The source-level cases are in `c600-syntax`'s lowering
//! tests, which map each diagnostic's path to its source text.

mod support;

use c600_check::{EXACTNESS_VIOLATION, EXACT_REQUIRED, INCOMPATIBLE_ALPHABET, TYPE_MISMATCH, WORD_KEY_NOT_EXECUTABLE};
use c600_ir::build::*;
use c600_ir::{ExactRole, Expr, Lit, Prim, Program, ReceiptField, Ty};
use support::{less, mismatch, only, report, tint, tool, xy};

fn boolean(b: bool) -> Expr {
    Expr::Lit(Lit::Bool(b))
}

fn text(t: &str) -> Expr {
    Expr::Lit(Lit::Text(t.into()))
}

fn nat_lit(n: &str) -> Lit {
    Lit::Nat(n.into())
}

/// A helper `f` without parameters.
fn f(result: Ty, body: Expr) -> Program {
    tool(vec![helper("f", vec![], exact(result), body)])
}

/// A command `f` without parameters.
fn cmd(result: Ty, body: Expr) -> Program {
    tool(vec![command("f", vec![], exact(result), body)])
}

/// The program's only diagnostic is a type mismatch at `path` of `function` with the given types.
fn assert_only(p: &Program, function: &str, path: &str, expected: &str, supplied: &str) {
    let d = only(p);
    assert_eq!(mismatch(&d), (TYPE_MISMATCH, function, path, expected, supplied), "{:#?}", d);
    // The exactness fields belong to the exactness codes (LG2.22).
    assert_eq!((d.value_type.as_deref(), d.display_control, d.qualifier_erasure), (None, false, false));
}

#[test]
fn an_argument_must_fit_its_parameter() {
    let g = || helper("g", vec![param("w", exact(Ty::Word))], exact(Ty::Word), var("w"));
    let named = tool(vec![g(), helper("f", vec![], exact(Ty::Word), call("g", vec![nat(1)]))]);
    assert_only(&named, "f", "0", "Word", "Nat");
    let as_value = tool(vec![g(), helper("f", vec![], exact(Ty::Word), call_value(fn_ref("g"), vec![nat(1)]))]);
    assert_only(&as_value, "f", "0", "Word", "Nat");
}

#[test]
fn a_called_value_must_be_a_function() {
    // The call's expected type lists the arguments' types; the result is unknown.
    assert_only(&f(Ty::Nat, call_value(nat(1), vec![word(&[1])])), "f", "1", "fn(Word) -> _", "Nat");
    // A function value called with another number of arguments: the count is part of its type.
    let g = closure(vec![param("w", exact(Ty::Word))], exact(Ty::Bool), boolean(true));
    let body = let_("g", g, call_value(var("g"), vec![word(&[1]), word(&[2])]));
    assert_only(&f(Ty::Bool, body), "f", "1.2", "fn(Word, Word) -> _", "fn(Word) -> Bool");
    let g = helper("g", vec![param("w", exact(Ty::Word))], exact(Ty::Word), var("w"));
    let p = tool(vec![g, helper("f", vec![], exact(Ty::Word), call_value(fn_ref("g"), vec![]))]);
    assert_only(&p, "f", "0", "fn() -> _", "fn(Word) -> Word");
}

#[test]
fn arguments_bound_to_one_type_variable_need_a_common_type() {
    let t = || exact(Ty::Var("T".into()));
    let pick = helper("pick", vec![param("a", t()), param("b", t())], t(), var("a"));
    let p = tool(vec![pick, helper("f", vec![], exact(Ty::Nat), call("pick", vec![nat(1), word(&[1])]))]);
    assert_only(&p, "f", "1", "Nat", "Word");
}

#[test]
fn a_result_must_fit_its_declared_type() {
    // Reported at the returned expression: the rest of the innermost `let`.
    let body = let_("y", exact_add(var("x"), nat(1)), var("y"));
    assert_only(&tool(vec![helper("f", xy(), exact(Ty::Bool), body)]), "f", "1", "Bool", "Nat");
    // A closure's result likewise, below the closure.
    let body = let_("c", closure(vec![], exact(Ty::Bool), nat(1)), nat(0));
    assert_only(&f(Ty::Nat, body), "f", "0.0", "Bool", "Nat");
}

#[test]
fn branches_and_arms_need_a_common_type() {
    assert_only(&f(Ty::Nat, if_(boolean(true), nat(1), word(&[1]))), "f", "2", "Nat", "Word");
    let shown = tool(vec![helper("f", xy(), display(Ty::Nat), display_if(less(), nat(1), word(&[1])))]);
    assert_only(&shown, "f", "2", "Nat", "Word");
    let n = || vec![param("n", exact(Ty::Nat))];
    let arms = match_(var("n"), vec![(nat_lit("1"), word(&[1])), (nat_lit("2"), nat(2))], word(&[3]));
    assert_only(&tool(vec![helper("f", n(), exact(Ty::Word), arms)]), "f", "2", "Word", "Nat");
    let default = match_(var("n"), vec![(nat_lit("1"), word(&[1]))], nat(3));
    assert_only(&tool(vec![helper("f", n(), exact(Ty::Word), default)]), "f", "2", "Word", "Nat");
}

#[test]
fn a_pattern_must_compare_with_its_scrutinee() {
    // A pattern has no path of its own: the match is reported, with the scrutinee's type expected.
    let n = || vec![param("n", exact(Ty::Nat))];
    let text_arm = match_(var("n"), vec![(Lit::Text("a".into()), word(&[1]))], word(&[2]));
    assert_only(&tool(vec![helper("f", n(), exact(Ty::Word), text_arm)]), "f", "root", "Nat", "Text");
    // A pattern compares as an exact equality does: either side may widen to the other.
    let int_scrutinee = match_(var("i"), vec![(nat_lit("1"), word(&[1]))], word(&[2]));
    let p = tool(vec![helper("f", vec![param("i", exact(Ty::Int))], exact(Ty::Word), int_scrutinee)]);
    assert!(report(&p).is_admitted(), "{:#?}", report(&p).diagnostics);
    let int_pattern = match_(var("n"), vec![(Lit::Int("-1".into()), word(&[1]))], word(&[2]));
    assert!(report(&tool(vec![helper("f", n(), exact(Ty::Word), int_pattern)])).is_admitted());
}

/// CT-LG2-04 (part: an exact operand of an approximate operation needs an explicit conversion, and the first
/// approximate operand fixes the precision).
#[test]
fn primitive_operands_must_fit() {
    let numbers = "Nat | Int | Rat | Q5 | Budget";
    let cases = [
        (f(Ty::Nat, exact_add(nat(1), text("a"))), "1", numbers, "Text"),
        (f(Ty::Bool, prim(Prim::ExactLess, vec![boolean(true), nat(1)])), "0", numbers, "Bool"),
        (f(Ty::Bool, prim(Prim::ExactEq, vec![nat(1), word(&[1])])), "1", "Nat", "Word"),
        (f(Ty::Bool, prim(Prim::And, vec![boolean(true), nat(1)])), "1", "Bool", "Nat"),
        (f(Ty::Bool, prim(Prim::Not, vec![word(&[1])])), "0", "Bool", "Word"),
        (f(approx(), to_approx(word(&[1]))), "0", numbers, "Word"),
        (f(Ty::Int, prim(Prim::ToExact, vec![text("a")])), "0", "Approx<_> | Nat | Int | Budget", "Text"),
        (f(Ty::ApproxBool, prim(Prim::ApproxNot, vec![boolean(true)])), "0", "ApproxBool", "Bool"),
    ];
    for (p, path, expected, supplied) in cases {
        assert_only(&p, "f", path, expected, supplied);
    }
    // An exact value at an approximate operation needs an explicit conversion; the first approximate operand fixes
    // the precision (CT-LG2-04).
    let first =
        tool(vec![helper("f", xy(), exact(approx()), prim(Prim::ApproxAdd, vec![nat(1), to_approx(var("x"))]))]);
    assert_only(&first, "f", "0", "Approx<_>", "Nat");
    let second = tool(vec![helper("f", xy(), exact(Ty::ApproxBool), approx_less(to_approx(var("x")), nat(1)))]);
    assert_only(&second, "f", "1", "Approx<P53>", "Nat");
}

/// The LP2 drafts `exact_mul` and `assert` (DET.57, DET.55): a product widens along `Nat`, `Int`, `Rat` and `Q5` as a
/// sum does but takes no budget, since a product of costs is no cost; an assertion takes a boolean and is of value
/// `Unit`.
#[test]
fn a_product_and_an_assertion_take_their_operands() {
    let product = |a: Ty, b: Ty, result: Ty| {
        let ab = vec![param("a", exact(a)), param("b", exact(b))];
        tool(vec![helper("f", ab, exact(result), exact_mul(var("a"), var("b")))])
    };
    let widened = [(Ty::Nat, Ty::Nat, Ty::Nat), (Ty::Nat, Ty::Int, Ty::Int), (Ty::Int, Ty::Rat, Ty::Rat)];
    for (a, b, result) in widened.into_iter().chain([(Ty::Rat, Ty::Q5, Ty::Q5)]) {
        let p = product(a, b, result);
        assert!(report(&p).is_admitted(), "{:#?}", report(&p).diagnostics);
    }
    assert_only(&product(Ty::Nat, Ty::Int, Ty::Nat), "f", "root", "Nat", "Int");
    let numbers = "Nat | Int | Rat | Q5";
    assert_only(&product(Ty::Budget, Ty::Nat, Ty::Nat), "f", "0", numbers, "Budget");
    assert_only(&f(Ty::Nat, exact_mul(nat(1), text("a"))), "f", "1", numbers, "Text");
    let assertion = |predicate: Expr, result: Ty| f(result, prim(Prim::Assert, vec![predicate]));
    assert!(report(&assertion(boolean(false), Ty::Unit)).is_admitted());
    assert_only(&assertion(nat(1), Ty::Unit), "f", "0", "Bool", "Nat");
    assert_only(&assertion(boolean(true), Ty::Bool), "f", "root", "Bool", "Unit");
}

#[test]
fn a_field_read_needs_a_record_with_that_field() {
    assert_only(&f(Ty::Nat, field(nat(1), "a")), "f", "0", "record { a: _, .. }", "Nat");
    let other = field(record(vec![("b", nat(1))]), "a");
    assert_only(&f(Ty::Nat, other), "f", "0", "record { a: _, .. }", "record { b: Nat }");
}

/// CT-LG2-11 (part: a proposal takes a Word, in IR, and refuses a `WordEqualityKey` or LB1.10's `TraceEqualityKey`
/// (LP2 drafts) with `LG2:type.word_key_not_executable`, as a word operation and a stored word do; not run: the other
/// bindings, which wait for M4).
#[test]
fn state_and_preview_operations_take_their_types() {
    assert_only(&cmd(Ty::Unit, write_workspace("r", exact(Ty::Word), nat(1))), "f", "0", "Word", "Nat");
    assert_only(&cmd(Ty::Receipt, propose(nat(1), target("t"))), "f", "0", "Word", "Nat");
    for (op, name) in [(Prim::WordKey, "WordEqualityKey"), (Prim::TraceKey, "TraceEqualityKey")] {
        let key = || prim(op, vec![word(&[1, -1])]);
        let keyed = [
            (cmd(Ty::Receipt, propose(key(), target("t"))), "0"),
            (f(Ty::Word, prim(Prim::Inverse, vec![key()])), "0"),
            (cmd(Ty::Unit, write_workspace("r", exact(Ty::Word), key())), "0"),
            (f(Ty::Word, key()), "root"),
        ];
        for (p, path) in &keyed {
            let d = only(p);
            let (code, function, at, expected, supplied) = mismatch(&d);
            assert_eq!((code, function, at, expected, supplied), (WORD_KEY_NOT_EXECUTABLE, "f", *path, "Word", name));
        }
    }
    // A trace key takes a Full word or a menu word (LB1.10: a menu word's key is under the run's `jumble-locality/1`),
    // and a word key a Full word only, so a menu word is of another alphabet there.
    let menu = || Expr::Lit(Lit::MenuWord(vec![[0, 1]]));
    let keyed = report(&f(Ty::TraceEqualityKey, prim(Prim::TraceKey, vec![menu()])));
    assert!(keyed.diagnostics.is_empty(), "{:#?}", keyed.diagnostics);
    let d = only(&f(Ty::WordEqualityKey, prim(Prim::WordKey, vec![menu()])));
    let (code, _, at, expected, supplied) = mismatch(&d);
    assert_eq!((code, at, expected, supplied), (INCOMPATIBLE_ALPHABET, "0", "Word", "MenuWord"));
    assert_only(&cmd(Ty::Receipt, propose(word(&[1]), nat(2))), "f", "1", "Target", "Nat");
    assert_only(&cmd(Ty::Text, read_receipt(nat(1), ReceiptField::Id)), "f", "0", "Receipt", "Nat");
    assert_only(&cmd(Ty::Unit, use_exact(ExactRole::Budget, word(&[1]))), "f", "0", "Nat | Int | Budget", "Word");
    // A read whose type does not fit what a workspace write of the program stores, reported once at the read.
    let keep = command("keep", vec![], exact(Ty::Unit), write_workspace("r", exact(Ty::Nat), nat(1)));
    let look = view("look", vec![], exact(Ty::Word), read_snapshot("r", exact(Ty::Word)));
    assert_only(&tool(vec![keep, look]), "look", "root", "Nat", "Word");
}

/// Sets and maps in IR (LP2 drafts): an operation takes the collection it names and a key that fits its key type. A
/// set element or a map key of a type without a canonical key order, or a map with an unpaired key, is malformed IR
/// (LG2.20), not a diagnostic.
#[test]
fn set_and_map_operations_take_their_collections_and_keys() {
    let nats = || set(exact(Ty::Nat), vec![nat(1), nat(2)]);
    let names = || map(exact(Ty::Text), exact(Ty::Nat), vec![(text("a"), nat(1))]);
    let empty = || list(exact(Ty::Nat), vec![]);
    assert_only(&f(Ty::Bool, prim(Prim::Contains, vec![empty(), nat(1)])), "f", "0", "Set<_> | Map<_, _>", "List<Nat>");
    assert_only(&f(Ty::Nat, prim(Prim::Lookup, vec![nats(), nat(1)])), "f", "0", "Map<_, _>", "Set<Nat>");
    assert_only(&f(Ty::Nat, prim(Prim::Lookup, vec![names(), nat(1)])), "f", "1", "Text", "Nat");
    assert_only(&f(Ty::Bool, prim(Prim::Contains, vec![nats(), text("1")])), "f", "1", "Nat", "Text");
    let pair = prim(Prim::InsertEntry, vec![names(), text("b"), text("2")]);
    assert_only(&f(Ty::Map(exact(Ty::Text).into(), exact(Ty::Nat).into()), pair), "f", "2", "Nat", "Text");
    let malformed = [
        set(exact(Ty::Receipt), vec![]),
        set(c600_ir::QTy { ty: Ty::Nat, display: true }, vec![]),
        map(exact(Ty::Unit), exact(Ty::Nat), vec![]),
        Expr::Collection { ty: Ty::Map(exact(Ty::Nat).into(), exact(Ty::Nat).into()), items: vec![nat(1)] },
    ];
    for body in malformed {
        assert!(c600_check::check(&f(Ty::Unit, body.clone())).is_err(), "{body:?}");
    }
}

#[test]
fn one_fault_gives_one_finding() {
    // The misfit operand's value is unknown from there on: the sum, the comparison, the condition and the result take
    // it without a finding of their own.
    let sum = exact_add(word(&[1]), nat(1));
    let body = if_(prim(Prim::ExactLess, vec![sum, nat(2)]), nat(1), nat(2));
    assert_only(&f(Ty::Nat, body), "f", "0.0.0", "Nat | Int | Rat | Q5 | Budget", "Word");
    // Through a generic call.
    let t = || exact(Ty::Var("T".into()));
    let id = helper("id", vec![param("a", t())], t(), var("a"));
    let p = tool(vec![id, helper("f", vec![], exact(Ty::Bool), call("id", vec![field(nat(1), "a")]))]);
    assert_only(&p, "f", "0.0", "record { a: _, .. }", "Nat");
    // Display branches without a common type select no value whose exactness could be checked at an exact goal.
    let goal = use_exact(ExactRole::Goal, display_if(less(), word(&[1]), nat(1)));
    assert_only(&tool(vec![command("f", xy(), exact(Ty::Unit), goal)]), "f", "0.2", "Word", "Nat");
    // A conversion has its own result type, so a later fault of that value is a finding of its own (LG2.22).
    let converted = tool(vec![helper("f", vec![], exact(Ty::Nat), exact_add(to_approx(boolean(true)), nat(1)))]);
    let found: Vec<_> = report(&converted).diagnostics.iter().map(|d| (d.code, d.path.clone())).collect();
    assert_eq!(found, [(EXACT_REQUIRED, "0".to_string()), (TYPE_MISMATCH, "0.0".to_string())]);
    // A budget that is no integer, set under approximate control: a fault of the value and a fault of the place.
    let budget = display_if(less(), use_exact(ExactRole::Budget, word(&[1])), unit());
    let found: Vec<_> = report(&tool(vec![command("f", xy(), exact(Ty::Unit), budget)]))
        .diagnostics
        .iter()
        .map(|d| (d.code, d.path.clone(), d.value_type.clone()))
        .collect();
    assert_eq!(
        found,
        [(EXACTNESS_VIOLATION, "1".to_string(), Some("Word".to_string())), (TYPE_MISMATCH, "1.0".to_string(), None)]
    );
}

#[test]
fn where_both_apply_the_base_type_comes_first() {
    // A display colour or an approximate scalar at an ordinary if is no Bool: the mismatch is the finding, and the
    // value's exactness is not checked. An ApproxBool condition fits and stays an exactness violation (CT-LG2-05).
    let colour = tool(vec![helper("f", xy(), exact(Ty::Nat), if_(tint(), nat(1), nat(2)))]);
    assert_only(&colour, "f", "0", "Bool", "Colour");
    let scalar = tool(vec![helper("f", xy(), exact(Ty::Nat), if_(to_approx(var("x")), nat(1), nat(2)))]);
    assert_only(&scalar, "f", "0", "Bool", "Approx<P53>");
}

#[test]
fn lg4_3_mismatches_and_exactness_findings_share_one_order() {
    // Sorted by module, function and path, whatever the code; `g` comes first because it is declared first.
    let g = helper("g", vec![], exact(Ty::Bool), nat(1));
    let body = if_(nat(1), exact_add(to_approx(var("x")), nat(1)), field(nat(2), "a"));
    let r = report(&tool(vec![g, helper("f", xy(), exact(Ty::Nat), body)]));
    let found: Vec<_> = r.diagnostics.iter().map(|d| (d.code, d.function.as_str(), d.path.as_str())).collect();
    assert_eq!(
        found,
        [
            (TYPE_MISMATCH, "g", "root"),
            (TYPE_MISMATCH, "f", "0"),
            (EXACT_REQUIRED, "f", "1.0"),
            (TYPE_MISMATCH, "f", "2.0")
        ]
    );
    assert!(r.diagnostics[2].mismatch.is_none() && r.diagnostics[2].value_type.as_deref() == Some("Approx"));
}
