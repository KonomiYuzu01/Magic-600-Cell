//! The three known limits of M1, leads from the Fable attack (owner decision, 15 November 2026): each attack program
//! with the frozen rules' verdict. Each test names the part of a conformance row it covers.

mod support;

use c600_check::{EFFECT_UNDECLARED, EXACTNESS_VIOLATION, EXACT_REQUIRED};
use c600_ir::build::*;
use c600_ir::{Expr, Function, Prim, Ty};
use support::{codes, fields, less, only, report, tool, xy};

const V: &str = EXACTNESS_VIOLATION;

/// `prepare() -> Unit`, preparing a preview.
fn prepare() -> Function {
    helper("prepare", vec![], exact(Ty::Unit), let_("r", propose(word(&[4]), target("t")), unit()))
}

/// `view show(f: () -> Unit ! e) -> Unit`, calling `f`.
fn show() -> Function {
    let f = fn_ty(vec![], exact(Ty::Unit), bound(&[], &["e"]));
    view("show", vec![param("f", exact(f))], exact(Ty::Unit), call_value(var("f"), vec![]))
}

fn go(body: Expr) -> Function {
    command("go", vec![], exact(Ty::Unit), body)
}

// Limit 1: an effect variable of a view's callable parameter instantiated at a call site with a workspace or preview
// effect. LG3.5 and LG3.6: views have no workspace-write or preview effect, including through callees; BND.83 gives
// `LG3:effect.undeclared`. The view's body is checked once, so the call site reports the instantiation.

/// CT-LG3-03 (part: a view's effect variable instantiated with a preview or workspace effect at the call site, in IR).
#[test]
fn limit_1_a_view_effect_variable_takes_no_workspace_or_preview_effect() {
    let d = only(&tool(vec![prepare(), show(), go(call("show", vec![fn_ref("prepare")]))]));
    assert_eq!((d.code, d.function.as_str(), d.path.as_str()), (EFFECT_UNDECLARED, "go", "root"));
    let writes = closure(vec![], exact(Ty::Unit), write_workspace("k", exact(Ty::Nat), nat(1)));
    let d = only(&tool(vec![show(), go(call("show", vec![writes]))]));
    assert_eq!((d.code, d.function.as_str(), d.path.as_str()), (EFFECT_UNDECLARED, "go", "root"));
    // Through a function value of the view, and through a record field.
    let by_value = go(call_value(fn_ref("show"), vec![fn_ref("prepare")]));
    assert_eq!(codes(&tool(vec![prepare(), show(), by_value])), [EFFECT_UNDECLARED]);
    let held = fn_ty(vec![], exact(Ty::Unit), bound(&[], &["e"]));
    let holder = record_ty(vec![("f", exact(held))]);
    let in_record =
        view("show", vec![param("r", exact(holder))], exact(Ty::Unit), call_value(field(var("r"), "f"), vec![]));
    let p = tool(vec![prepare(), in_record, go(call("show", vec![record(vec![("f", fn_ref("prepare"))])]))]);
    assert_eq!(codes(&p), [EFFECT_UNDECLARED]);
    // A view effect is a view's own; a helper's effect variable may take a preview inside a command.
    let shows = closure(vec![], exact(Ty::Unit), publish_view(nat(1)));
    assert!(report(&tool(vec![show(), go(call("show", vec![shows]))])).is_admitted());
    let run = helper("run", show().params, exact(Ty::Unit), call_value(var("f"), vec![]));
    assert!(report(&tool(vec![prepare(), run, go(call("run", vec![fn_ref("prepare")]))])).is_admitted());
}

fn record_ty(fields: Vec<(&str, c600_ir::QTy)>) -> Ty {
    Ty::Record(fields.into_iter().map(|(n, t)| (n.to_string(), t)).collect())
}

// Limit 2: recursion through a callable parameter whose bound has no effect variable. LG2.20: approximate data cannot
// bound recursion; the call under approximate control is `LG2:type.exactness_violation` with display control.

/// CT-LG2-05 (part: LG2.20, recursion through a callable passed to a parameter with a closed bound).
#[test]
fn limit_2_recursion_through_a_closed_bound_is_traced() {
    let thunk = fn_ty(vec![], display(Ty::Nat), bound(&[], &[]));
    let apply = helper("apply", vec![param("f", exact(thunk))], display(Ty::Nat), call_value(var("f"), vec![]));
    let again = helper("again", vec![], display(Ty::Nat), call("count", vec![to_approx(nat(1)), to_approx(nat(2))]));
    let ab = vec![param("a", exact(approx())), param("b", exact(approx()))];
    let steered = call("apply", vec![fn_ref("again")]);
    let count = helper("count", ab, display(Ty::Nat), display_if(approx_less(var("a"), var("b")), steered, nat(0)));
    let d = only(&tool(vec![apply, again, count]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("count", "1", (V, Some("Nat"), true, false)));
}

// Limit 3: the approximate-control bit of a function value. LG2.22's precedence: a value selected under approximate
// control at an exact operand is `LG2:type.exactness_violation` with display control, not `exact_required`.

fn pick() -> Function {
    helper("pick", xy(), exact(approx()), display_if(less(), to_approx(var("x")), to_approx(var("y"))))
}

fn add(picked: Expr) -> Function {
    helper("add", xy(), exact(Ty::Nat), exact_add(picked, nat(1)))
}

/// CT-LG2-05 (part: LG2.22's precedence through function values and closures).
#[test]
fn limit_3_a_function_value_keeps_its_control_bit() {
    let args = || vec![var("x"), var("y")];
    let d = only(&tool(vec![pick(), add(call_value(fn_ref("pick"), args()))]));
    assert_eq!((d.function.as_str(), d.path.as_str(), fields(&d)), ("add", "0", (V, Some("Approx"), true, false)));
    let held = let_("p", fn_ref("pick"), call_value(var("p"), args()));
    let d = only(&tool(vec![pick(), add(held)]));
    assert_eq!((d.path.as_str(), fields(&d)), ("0", (V, Some("Approx"), true, false)));
    let chooser = closure(xy(), exact(approx()), display_if(less(), to_approx(var("x")), to_approx(var("y"))));
    let d = only(&tool(vec![add(call_value(chooser, args()))]));
    assert_eq!((d.path.as_str(), fields(&d)), ("0", (V, Some("Approx"), true, false)));
}

/// CT-LG2-05 (part: the limit that stays: a callable known only by its declared type). The refusal holds; its code
/// is the direct approximate scalar's, because a declared function type has no place for the control bit until the
/// IR freezes (PHS.49).
#[test]
fn limit_3_a_declared_callable_type_cannot_carry_the_control_bit() {
    let f = fn_ty(vec![exact(Ty::Nat), exact(Ty::Nat)], exact(approx()), bound(&[], &[]));
    let mut params = xy();
    params.push(param("f", exact(f)));
    let called = Expr::Prim { op: Prim::ExactAdd, args: vec![call_value(var("f"), vec![var("x"), var("y")]), nat(1)] };
    let d = only(&tool(vec![helper("add", params, exact(Ty::Nat), called)]));
    assert_eq!((d.path.as_str(), d.code), ("0", EXACT_REQUIRED));
}
