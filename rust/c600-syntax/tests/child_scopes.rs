//! Child scopes in source (DET.60; LP2 draft until the IR freeze, PHS.49). `child(caps, body, stop branch)` runs
//! `body`, a closure without parameters, as a serial subrun under the requested caps: a record of literal naturals
//! named `steps`, `kernel_cost`, `memory` and `output`, any of them omitted. When the body stops on a cap or an error
//! of its own, the stop branch, a closure with the body's result type and no parameter or one `Status` parameter,
//! runs in the parent. A child needs the `budget` effect. Its body borrows the names in scope, but no display data,
//! and it evaluates no display, reads no display accounting or clock and calls no function value: exact inputs alone
//! then fix its budget use and its outcome, which continues into exact computation (LG2.20). The stop branch runs in
//! the parent and is checked as the parent's code. The run-time half is `c600-run`'s `child_scopes.rs`.

use c600_check::{check, Diagnostic, Report, EFFECT_UNDECLARED, EXACTNESS_VIOLATION, TYPE_MISMATCH};
use c600_ir::{ChildCaps, Expr as Ir, QTy, Ty};
use c600_syntax::{lower, parse, Lowered};

/// One module `tool` with the given capability list and declarations.
fn source(requires: &str, declarations: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule tool requires [{requires}];\n\n{declarations}")
}

fn lowered(declarations: &str) -> Lowered {
    let text = source("clock", declarations);
    let file = parse(text.as_bytes());
    assert!(file.diagnostics.is_empty(), "{text}\n{:#?}", file.diagnostics);
    lower(&[file]).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

/// The lowering's refusal of a program that parses.
fn refused_at_lowering(declarations: &str) -> String {
    let text = source("clock", declarations);
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

/// `f`, declared with `effects`, returning a child scope with the given caps, body block and stop-branch block.
fn child_in(params: &str, effects: &str, caps: &str, body: &str, otherwise: &str) -> String {
    format!(
        "fn f({params}) -> Nat effects [{effects}] {{\n    return child(record {{ {caps} }}, fn() -> Nat {{ {body} }}, \
         fn(s: Status) -> Nat {{ {otherwise} }});\n}}\n"
    )
}

/// `f(x: Nat, y: Nat)` with the budget effect, returning a child of eight steps with the given blocks.
fn child(body: &str, otherwise: &str) -> String {
    child_in("x: Nat, y: Nat", "budget", "steps = 8", body, otherwise)
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

/// A child lowers to its caps, its result type, its body at child 0 and its stop branch at child 1 with the status
/// bound; an omitted cap stays omitted, to be resolved when the child starts. The checker admits it.
#[test]
fn a_child_lowers_to_its_caps_body_and_stop_branch() {
    let declarations = child_in(
        "x: Nat",
        "budget",
        "steps = 8, memory = 64",
        "return exact_add(x, 1);",
        "return match s {\n        tag(\"cut_off\") => 1;\n        _ => 2;\n    };",
    );
    let lowered = lowered(&declarations);
    let found: Vec<_> = expressions(&lowered).into_iter().filter(|e| matches!(e, Ir::Child { .. })).collect();
    let [Ir::Child { caps, result, body, status, otherwise }] = found.as_slice() else { panic!("{found:#?}") };
    let want = ChildCaps { steps: Some(8), kernel_cost: None, memory: Some(64), output: None };
    assert_eq!((*caps, result, status.as_deref()), (want, &QTy { ty: Ty::Nat, display: false }, Some("s")));
    assert_eq!(found[0].children(), [&**body, &**otherwise]);
    admitted(&declarations);
    admitted(&child_in("x: Nat", "budget", "", "return x;", "return 0;"));
    let all = "steps = 1, kernel_cost = 2, memory = 3, output = 340282366920938463463374607431768211455";
    admitted(&child_in("x: Nat", "budget", all, "return x;", "return 0;"));
}

/// A child's caps are a record of literal naturals, each named once; another expression, an unknown name, a repeated
/// name, a negative or fractional literal, a name in scope and a natural beyond 128 bits are refused at lowering.
#[test]
fn a_child_s_caps_are_literal_naturals() {
    let caps = |caps: &str| {
        format!(
            "fn f(x: Nat) -> Nat effects [budget] {{\n    return child({caps}, fn() -> Nat {{ return 1; }}, \
             fn(s: Status) -> Nat {{ return 0; }});\n}}\n"
        )
    };
    for bad in [
        "8",
        "x",
        "record { time = 3 }",
        "record { steps = 1, steps = 2 }",
        "record { steps = -1 }",
        "record { steps = 1/2 }",
        "record { steps = x }",
        "record { steps = exact_add(1, 2) }",
        "record { steps = 340282366920938463463374607431768211456 }",
    ] {
        let message = refused_at_lowering(&caps(bad));
        assert!(message.contains("a child scope's caps are a record of literal naturals"), "{bad}: {message}");
    }
}

/// A child's body is a closure without parameters and its stop branch a closure with the body's result type and no
/// parameter or one exact `Status` parameter; a function name, another arity, another parameter type and another
/// result type are refused at lowering, and so is a call of `child` with other than three arguments.
#[test]
fn a_child_s_body_and_stop_branch_are_closures_of_their_own_shape() {
    let call = |body: &str, otherwise: &str| {
        format!(
            "fn g() -> Nat effects [] {{\n    return 1;\n}}\n\n\
             fn f(x: Nat) -> Nat effects [budget] {{\n    \
             return child(record {{ steps = 8 }}, {body}, {otherwise});\n}}\n"
        )
    };
    let otherwise = "fn(s: Status) -> Nat { return 0; }";
    for body in ["g", "fn(y: Nat) -> Nat { return y; }", "1"] {
        let message = refused_at_lowering(&call(body, otherwise));
        assert!(message.contains("a child scope's body is a closure without parameters"), "{body}: {message}");
    }
    let body = "fn() -> Nat { return 1; }";
    for otherwise in [
        "g",
        "fn(s: Nat) -> Nat { return 0; }",
        "fn(s: Display<Status>) -> Nat { return 0; }",
        "fn(s: Status, t: Status) -> Nat { return 0; }",
        "fn(s: Status) -> Text { return \"t\"; }",
        "fn() -> Text { return \"t\"; }",
    ] {
        let message = refused_at_lowering(&call(body, otherwise));
        assert!(message.contains("no parameter or one `Status` parameter"), "{otherwise}: {message}");
    }
    for args in ["record { steps = 8 }, fn() -> Nat { return 1; }", "record { }"] {
        let declaration = format!("fn f() -> Nat effects [budget] {{\n    return child({args});\n}}\n");
        let message = refused_at_lowering(&declaration);
        assert!(message.contains("child"), "{args}: {message}");
    }
}

/// A stop branch without a parameter names no status: it lowers with none bound, and the checker admits it.
#[test]
fn a_stop_branch_may_name_no_status() {
    let declarations = "fn f(x: Nat) -> Nat effects [budget] {\n    return child(record { steps = 8 }, \
                        fn() -> Nat { return x; }, fn() -> Nat { return 0; });\n}\n";
    let lowered = lowered(declarations);
    let found: Vec<_> = expressions(&lowered).into_iter().filter(|e| matches!(e, Ir::Child { .. })).collect();
    let [Ir::Child { status, .. }] = found.as_slice() else { panic!("{found:#?}") };
    assert_eq!(*status, None);
    admitted(declarations);
    let unbound = declarations.replace("fn() -> Nat { return 0; }", "fn() -> Nat { return s; }");
    assert!(refused_at_lowering(&unbound).contains("not bound"));
}

/// The status is bound in the stop branch only: the body and the code after the child do not see it, and it does not
/// shadow a name in scope.
#[test]
fn the_status_is_bound_in_the_stop_branch_only() {
    let message = refused_at_lowering(&child("return s;", "return 0;"));
    assert!(message.contains("not bound"), "{message}");
    let after = "fn f() -> Nat effects [budget] {\n    let v = child(record { steps = 8 }, fn() -> Nat { return 1; }, \
                 fn(s: Status) -> Nat { return 0; });\n    return match s {\n        tag(\"error\") => 1;\n        \
                 _ => v;\n    };\n}\n";
    assert!(refused_at_lowering(after).contains("not bound"));
    let shadow = child_in("s: Nat", "budget", "steps = 8", "return s;", "return 0;");
    assert!(refused_at_lowering(&shadow).contains("already bound"));
}

/// A child needs the `budget` effect: its caps spend the enclosing budgets (DET.60).
#[test]
fn a_child_needs_the_budget_effect() {
    let d = only(&child_in("x: Nat", "", "steps = 8", "return x;", "return 0;"));
    assert_eq!(d.code, EFFECT_UNDECLARED, "{d:#?}");
}

/// The body's result and the stop branch's result fit the child's result type, at their own paths.
#[test]
fn a_child_s_results_fit_its_type() {
    let d = only(&child("return \"t\";", "return 0;"));
    let m = d.mismatch.as_ref().expect("a mismatch");
    assert_eq!(
        (d.code, d.path.as_str(), m.expected.as_str(), m.supplied.as_str()),
        (TYPE_MISMATCH, "0", "Nat", "Text")
    );
    let d = only(&child("return 1;", "return s;"));
    let m = d.mismatch.as_ref().expect("a mismatch");
    assert_eq!(
        (d.code, d.path.as_str(), m.expected.as_str(), m.supplied.as_str()),
        (TYPE_MISMATCH, "1", "Nat", "Status")
    );
}

/// The body evaluates no display, reads no display accounting or clock and calls no function value, directly or
/// through a named function: each is one `LG2:exactness.violation` under display control at the child, since its
/// budget use would not be fixed by exact inputs alone and the child's outcome continues into exact computation
/// (LG2.20, DET.60; LP2 draft). The same operations in the stop branch, which runs in the parent, are admitted.
#[test]
fn a_child_s_body_has_no_display_cost() {
    let shown = "let d = display_if(approx_less(approx53(x), approx53(y)), 1, 2);\n        return x;";
    let functions = "fn shows(x: Nat, y: Nat) -> Nat effects [] {\n    \
                     let d = display_if(approx_less(approx53(x), approx53(y)), 1, 2);\n    return x;\n}\n\n";
    for (extra, params, effects, body) in [
        ("", "x: Nat, y: Nat", "budget", shown),
        ("", "x: Nat, y: Nat", "budget", "let k = used_steps();\n        return x;"),
        ("", "x: Nat, y: Nat", "budget, clock", "let k = read_clock();\n        return x;"),
        ("", "c: fn() -> Nat effects []", "budget", "return c();"),
        (functions, "x: Nat, y: Nat", "budget", "return shows(x, y);"),
    ] {
        let declarations = format!("{extra}{}", child_in(params, effects, "steps = 8", body, "return 0;"));
        let r = report(&declarations);
        let found: Vec<_> = r.diagnostics.iter().filter(|d| d.note.contains("a child scope's body")).collect();
        assert_eq!(found.len(), 1, "{body}: {:#?}", r.diagnostics);
        let d = found[0];
        assert_eq!((d.code, d.path.as_str(), d.display_control), (EXACTNESS_VIOLATION, "0", true), "{body}");
        assert_eq!(r.diagnostics.len(), 1, "{body}: {:#?}", r.diagnostics);
        let parent = format!("{extra}{}", child_in(params, effects, "steps = 8", "return 1;", body));
        admitted(&parent);
    }
}

/// A body that borrows display data from the enclosing scope is refused, once per name; the stop branch may.
#[test]
fn a_child_s_body_borrows_no_display_data() {
    let declarations = "fn f(x: Nat, y: Nat) -> Nat effects [budget] {\n    \
                        let d = display_if(approx_less(approx53(x), approx53(y)), 1, 2);\n    \
                        return child(record { steps = 8 }, fn() -> Nat { let e = d;\n        return x; }, \
                        fn(s: Status) -> Nat { return 0; });\n}\n";
    let r = report(declarations);
    let found: Vec<_> = r.diagnostics.iter().filter(|d| d.note.contains("borrows display data")).collect();
    assert_eq!(found.len(), 1, "{:#?}", r.diagnostics);
    assert!(found[0].note.starts_with("`d`"), "{:#?}", found[0]);
    assert_eq!((found[0].code, found[0].display_control), (EXACTNESS_VIOLATION, true));
    let parent = declarations
        .replace("let e = d;\n        return x; }", "return x; }")
        .replace("fn(s: Status) -> Nat { return 0; }", "fn(s: Status) -> Nat { let e = d;\n        return 0; }");
    admitted(&parent);
}

/// Nested children: an inner body's display cost is reported once, at the inner child, and the outer body that holds
/// the inner child is not reported again for it.
#[test]
fn a_nested_child_reports_its_display_cost_once() {
    let declarations = "fn f(x: Nat) -> Nat effects [budget] {\n    \
                        return child(record { steps = 8 }, fn() -> Nat {\n        \
                        return child(record { steps = 4 }, fn() -> Nat { let k = used_steps();\n            \
                        return x; }, fn(t: Status) -> Nat { return 1; });\n    }, \
                        fn(s: Status) -> Nat { return 0; });\n}\n";
    let r = report(declarations);
    assert_eq!(r.codes(), [EXACTNESS_VIOLATION], "{:#?}", r.diagnostics);
    assert_eq!(r.diagnostics[0].path, "0.0", "{:#?}", r.diagnostics);
    let clean = declarations.replace("let k = used_steps();\n            ", "");
    admitted(&clean);
}
