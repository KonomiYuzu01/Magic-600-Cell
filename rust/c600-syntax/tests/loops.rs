//! Loops in source (DET.55; LP2 draft until the IR freeze, PHS.49). `repeat(n, init, fn(acc: T) -> T { ... })` runs
//! its body `n` times, each time with `acc` bound to the value so far, from `init`; the loop's value is the last one.
//! The count bounds the iteration, so it is an exact natural (LG2.20: loop bounds accept exact inputs only, and
//! approximate data cannot bound iteration). The initial value and the body's result fit the loop's type, and `acc` is
//! bound in the body only. The run-time half is `c600-run`'s `loops.rs`.

use c600_check::{check, Diagnostic, Report, EXACTNESS_VIOLATION, TYPE_MISMATCH};
use c600_ir::{Expr as Ir, QTy, Ty};
use c600_syntax::{lower, parse, Lowered};

/// One module `tool`, which declares the clock, with the given declarations.
fn source(declarations: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule tool requires [clock];\n\n{declarations}")
}

fn lowered(declarations: &str) -> Lowered {
    let text = source(declarations);
    let file = parse(text.as_bytes());
    assert!(file.diagnostics.is_empty(), "{text}\n{:#?}", file.diagnostics);
    lower(&[file]).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

/// The lowering's refusal of a program that parses.
fn refused_at_lowering(declarations: &str) -> String {
    let text = source(declarations);
    let file = parse(text.as_bytes());
    assert!(file.diagnostics.is_empty(), "{text}\n{:#?}", file.diagnostics);
    match lower(&[file]) {
        Ok(_) => panic!("{declarations}: lowered"),
        Err(e) => e.to_string(),
    }
}

fn report(declarations: &str) -> Report {
    check(&lowered(declarations).program).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

fn admitted(declarations: &str) {
    let r = report(declarations);
    assert!(r.is_admitted(), "{declarations}: {:#?}", r.diagnostics);
}

/// The only finding of a program.
fn only(declarations: &str) -> Diagnostic {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    r.diagnostics[0].clone()
}

/// `f` with the given parameters, result type, effects and statements.
fn f(params: &str, result: &str, effects: &str, statements: &str) -> String {
    format!("fn f({params}) -> {result} effects [{effects}] {{\n    {statements}\n}}\n")
}

/// `f(params) -> Nat` with no effects, returning `repeat(count, init, body)`.
fn nat_loop(params: &str, count: &str, init: &str, body: &str) -> String {
    f(params, "Nat", "", &format!("return repeat({count}, {init}, {body});"))
}

/// The display data of `x` and `y`: a natural selected under approximate control (LG2.20).
const SHOWN: &str = "display_if(approx_less(approx53(x), approx53(y)), 1, 2)";

/// Every loop of every function of a lowered program, depth first.
fn loops(lowered: &Lowered) -> Vec<&Ir> {
    let mut out = Vec::new();
    let mut pending: Vec<&Ir> = lowered.program.modules.iter().flat_map(|m| &m.functions).map(|f| &f.body).collect();
    while let Some(e) = pending.pop() {
        if matches!(e, Ir::Repeat { .. }) {
            out.push(e);
        }
        pending.extend(e.children());
    }
    out
}

/// A loop lowers to its count at child 0, its initial value at child 1 and its body's block at child 2, with the
/// value so far named and the loop's type. The checker admits it, and so it does a count computed exactly, a loop over
/// a list and nested loops.
#[test]
fn a_loop_lowers_to_its_count_initial_value_and_body() {
    let declarations = nat_loop("n: Nat", "n", "1", "fn(acc: Nat) -> Nat { return exact_mul(acc, 2); }");
    let program = lowered(&declarations);
    let found = loops(&program);
    let [Ir::Repeat { count, init, acc, result, body }] = found.as_slice() else { panic!("{found:#?}") };
    assert_eq!((&**count, acc.as_str()), (&Ir::Var("n".into()), "acc"));
    assert_eq!(result, &QTy { ty: Ty::Nat, display: false });
    assert_eq!(found[0].children(), [&**count, &**init, &**body]);
    admitted(&declarations);
    admitted(&nat_loop("n: Nat", "exact_add(n, 1)", "n", "fn(acc: Nat) -> Nat { return acc; }"));
    admitted(&f(
        "",
        "List<Nat>",
        "",
        "return repeat(3, list<Nat>(), fn(acc: List<Nat>) -> List<Nat> { return push(acc, 1); });",
    ));
    let nested = "fn(acc: Nat) -> Nat {\n        \
                  return repeat(3, acc, fn(inner: Nat) -> Nat { return exact_add(inner, 1); });\n    }";
    let declarations = nat_loop("", "2", "0", nested);
    assert_eq!(loops(&lowered(&declarations)).len(), 2);
    admitted(&declarations);
}

/// A loop's body is a closure with one parameter of the loop's result type: a function name, another expression,
/// another arity, another parameter type and a display parameter of an exact loop are refused at lowering, and so is a
/// call of `repeat` with other than three arguments.
#[test]
fn a_loop_s_body_is_a_closure_of_one_parameter_of_its_type() {
    let declarations = |body: &str| {
        format!("fn g(x: Nat) -> Nat effects [] {{\n    return x;\n}}\n\n{}", nat_loop("", "2", "0", body))
    };
    for body in [
        "g",
        "1",
        "fn() -> Nat { return 1; }",
        "fn(a: Nat, b: Nat) -> Nat { return a; }",
        "fn(acc: Text) -> Nat { return 1; }",
        "fn(acc: Display<Nat>) -> Nat { return 1; }",
    ] {
        let message = refused_at_lowering(&declarations(body));
        assert!(
            message.contains("a loop's body is a closure with one parameter of its result type"),
            "{body}: {message}"
        );
    }
    for args in ["2, 0", "2, 0, fn(acc: Nat) -> Nat { return acc; }, 1"] {
        let message = refused_at_lowering(&f("", "Nat", "", &format!("return repeat({args});")));
        assert!(message.contains("repeat"), "{args}: {message}");
    }
}

/// The value so far is bound in the body only: the count, the initial value and the code after the loop do not see
/// it, nor the body's own locals, and it does not shadow a name in scope.
#[test]
fn the_value_so_far_is_bound_in_the_body_only() {
    let body = "fn(acc: Nat) -> Nat { return acc; }";
    for (count, init) in [("acc", "0"), ("2", "acc")] {
        let message = refused_at_lowering(&nat_loop("", count, init, body));
        assert!(message.contains("not bound"), "{count}, {init}: {message}");
    }
    let after = f("", "Nat", "", "let v = repeat(2, 0, fn(acc: Nat) -> Nat { return acc; });\n    return acc;");
    assert!(refused_at_lowering(&after).contains("not bound"));
    let local = f(
        "",
        "Nat",
        "",
        "let v = repeat(2, 0, fn(acc: Nat) -> Nat { let b = acc;\n        return b; });\n    return b;",
    );
    assert!(refused_at_lowering(&local).contains("not bound"));
    let shadow = nat_loop("acc: Nat", "2", "0", body);
    assert!(refused_at_lowering(&shadow).contains("already bound"));
}

/// The count is an exact natural (LG2.20): an approximate number, display data, a clock reading and a budget
/// observation are each one `LG2:type.exactness_violation` at the count, and an integral or text count is a type
/// mismatch there.
#[test]
fn a_loop_s_count_is_an_exact_natural() {
    let body = "fn(acc: Nat) -> Nat { return acc; }";
    for (effects, value) in [("", "approx53(x)"), ("", SHOWN), ("clock", "read_clock()"), ("budget", "used_steps()")] {
        let declarations =
            f("x: Nat, y: Nat", "Nat", effects, &format!("let d = {value};\n    return repeat(d, 0, {body});"));
        let d = only(&declarations);
        assert_eq!((d.code, d.path.as_str()), (EXACTNESS_VIOLATION, "1.0"), "{value}: {d:#?}");
        assert!(d.note.starts_with("a loop bound"), "{value}: {d:#?}");
    }
    for (params, count, supplied) in [("n: Int", "n", "Int"), ("", "\"t\"", "Text")] {
        let d = only(&nat_loop(params, count, "0", body));
        let m = d.mismatch.as_ref().unwrap_or_else(|| panic!("{count}: {d:#?}"));
        assert_eq!(
            (d.code, d.path.as_str(), m.expected.as_str(), m.supplied.as_str()),
            (TYPE_MISMATCH, "0", "Nat", supplied),
            "{count}"
        );
    }
}

/// The initial value and the body's result fit the loop's type, at their own paths, the body's after its `let`s; the
/// loop's value has the loop's type where it is used.
#[test]
fn a_loop_s_values_fit_its_type() {
    for (init, body, path) in [
        ("\"t\"", "fn(acc: Nat) -> Nat { return acc; }", "1"),
        ("0", "fn(acc: Nat) -> Nat { return \"t\"; }", "2"),
        ("0", "fn(acc: Nat) -> Nat { let b = \"t\";\n        return b; }", "2.1"),
    ] {
        let d = only(&nat_loop("", "2", init, body));
        let m = d.mismatch.as_ref().expect("a mismatch");
        assert_eq!(
            (d.code, d.path.as_str(), m.expected.as_str(), m.supplied.as_str()),
            (TYPE_MISMATCH, path, "Nat", "Text"),
            "{init}, {body}"
        );
    }
    let used = f("", "Text", "", "return repeat(2, 0, fn(acc: Nat) -> Nat { return acc; });");
    let d = only(&used);
    let m = d.mismatch.as_ref().expect("a mismatch");
    assert_eq!(
        (d.code, d.path.as_str(), m.expected.as_str(), m.supplied.as_str()),
        (TYPE_MISMATCH, "root", "Text", "Nat")
    );
}

/// A loop carries display data only by its type: an exact loop refuses a body result selected under approximate
/// control, reported at the loop as a child scope reports its body's, and a loop of `Display<Nat>` takes it, its value
/// then display data, which an exact result refuses in turn.
#[test]
fn a_loop_carries_display_data_only_by_its_type() {
    let exact = nat_loop("x: Nat, y: Nat", "2", "0", &format!("fn(acc: Nat) -> Nat {{ return {SHOWN}; }}"));
    let d = only(&exact);
    assert_eq!((d.code, d.path.as_str(), d.display_control), (EXACTNESS_VIOLATION, "root", true), "{d:#?}");
    assert!(d.note.starts_with("a loop's body"), "{d:#?}");
    let shown = format!("return repeat(2, 0, fn(acc: Display<Nat>) -> Display<Nat> {{ return {SHOWN}; }});");
    admitted(&f("x: Nat, y: Nat", "Display<Nat>", "", &shown));
    let d = only(&f("x: Nat, y: Nat", "Nat", "", &shown));
    assert_eq!((d.code, d.path.as_str()), (EXACTNESS_VIOLATION, "root"), "{d:#?}");
    assert!(d.note.starts_with("the result"), "{d:#?}");
}
