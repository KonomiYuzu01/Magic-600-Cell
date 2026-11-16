//! Leads L1 and L2 from Fable's attack on the M7 parser and lowering (16 November 2026) that reach the checker, each
//! with the frozen rules' verdict, in IR. Each test names the part of a conformance row it covers.

mod support;

use c600_check::{EXACTNESS_VIOLATION, EXACT_REQUIRED};
use c600_ir::build::*;
use c600_ir::{Expr, Function, Lit, Prim, QTy, Ty};
use support::{fields, only, report, tool};

fn approx_lit(decimal: &str) -> Lit {
    Lit::Approx { decimal: decimal.into(), precision: 53 }
}

/// `cmd store() -> Unit`, writing `value` to the workspace record `r` at type `ty`.
fn store(ty: QTy, value: Expr) -> Function {
    command("store", vec![], exact(Ty::Unit), write_workspace("r", ty, value))
}

/// The type variable `T`.
fn t() -> QTy {
    exact(Ty::Var("T".into()))
}

fn exact_eq(a: Expr, b: Expr) -> Expr {
    prim(Prim::ExactEq, vec![a, b])
}

// L1: a storage read at a type variable. LG2.19's storage row: decoding retains the qualifier; a type variable takes
// what it stands for and keeps its qualifier. LG2.22's precedence gives the code.

/// CT-LG2-01 (part: a generic storage read cannot erase the stored approximate qualifier).
#[test]
fn l1_a_read_at_a_type_variable_keeps_an_approximate_qualifier() {
    let stored = store(exact(approx()), Expr::Lit(approx_lit("1.5")));
    let peek = helper("peek", vec![], exact(Ty::Bool), exact_eq(read_snapshot("r", t()), nat(1)));
    let d = only(&tool(vec![stored, peek]));
    assert_eq!((d.function.as_str(), d.path.as_str()), ("peek", "0"));
    assert_eq!((d.code, d.display_control, d.qualifier_erasure), (EXACT_REQUIRED, false, false));
    // In a record field: the field read is the stored approximate scalar.
    let held = Ty::Record(vec![("a".into(), exact(approx()))]);
    let stored = store(exact(held), record(vec![("a", Expr::Lit(approx_lit("1.5")))]));
    let slot = exact(Ty::Record(vec![("a".into(), t())]));
    let peek = helper("peek", vec![], exact(Ty::Bool), exact_eq(field(read_snapshot("r", slot), "a"), nat(1)));
    let d = only(&tool(vec![stored, peek]));
    assert_eq!((d.function.as_str(), d.path.as_str(), d.code), ("peek", "0", EXACT_REQUIRED));
    // An exact stored value stays exact.
    let stored = store(exact(Ty::Nat), nat(1));
    let peek = helper("peek", vec![], exact(Ty::Bool), exact_eq(read_snapshot("r", t()), nat(1)));
    assert!(report(&tool(vec![stored, peek])).is_admitted());
}

/// CT-LG2-05 (part: a display value through storage and a generic read cannot steer a match).
#[test]
fn l1_a_read_at_a_type_variable_keeps_a_display_qualifier() {
    let shown = display_if(approx_less(to_approx(nat(1)), to_approx(nat(2))), nat(1), nat(2));
    let stored = store(display(Ty::Nat), shown);
    let peek = helper(
        "peek",
        vec![],
        exact(Ty::Word),
        match_(read_snapshot("r", t()), vec![(Lit::Nat("1".into()), word(&[1]))], word(&[2])),
    );
    let d = only(&tool(vec![stored, peek]));
    assert_eq!((d.function.as_str(), d.path.as_str()), ("peek", "0"));
    assert_eq!(fields(&d), (EXACTNESS_VIOLATION, Some("T"), false, true));
}

// L2: an approximate literal as a match pattern. LG1.7: an approximate literal is never read as exact; a pattern
// compares with the scrutinee as an exact equality does, so the literal is an exact operand.

/// CT-LG1-05 (part: an approximate literal remains a different kind, also as a match pattern).
#[test]
fn l2_an_approximate_literal_is_no_match_pattern() {
    let arms = vec![(approx_lit("1.0"), word(&[1]))];
    let f = helper("f", vec![param("n", exact(Ty::Nat))], exact(Ty::Word), match_(var("n"), arms, word(&[2])));
    let d = only(&tool(vec![f]));
    assert_eq!((d.function.as_str(), d.path.as_str()), ("f", "root"));
    assert_eq!((d.code, d.value_type.as_deref()), (EXACT_REQUIRED, Some("Approx")));
    // An exact literal pattern stays admitted.
    let arms = vec![(Lit::Nat("1".into()), word(&[1]))];
    let f = helper("f", vec![param("n", exact(Ty::Nat))], exact(Ty::Word), match_(var("n"), arms, word(&[2])));
    assert!(report(&tool(vec![f])).is_admitted());
}
