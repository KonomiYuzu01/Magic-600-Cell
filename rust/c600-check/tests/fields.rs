//! The F2 and F5 number kinds of KN3.2 and ADR-L03.2 in the checker (LP2 drafts until the IR freeze, PHS.49): their
//! operations type in one field, nothing widens into or out of a field, a field has no order, an element may be a set
//! element or a map key, and it is not portable data, so a goal, an evidence sink or a snapshot read's argument takes
//! its residue instead. Hand lowerings are no evidence for the parser (milestone M7); `c600-syntax`'s `fields.rs` lowers
//! the source forms.

mod support;

use std::collections::BTreeSet;

use c600_check::{admit_built_in, Grant, Policy, Row, EXACT_REQUIRED, TYPE_MISMATCH};
use c600_ir::build::*;
use c600_ir::{ExactRole, Expr, Field, Function, Prim, Ty};
use support::{codes, mismatch, only, report, tool};

const F2: Ty = Ty::Field(Field::F2);
const F5: Ty = Ty::Field(Field::F5);
/// What a goal, an evidence sink or a snapshot read's argument takes, in a mismatch (PHS.52, 16 November 2026).
const PORTABLE: &str =
    "a type without Fn, Receipt, WordEqualityKey, TraceEqualityKey, CompressedWord, F2, F5 or a type variable";

/// `x: Int` and `y: Int`.
fn ints() -> Vec<c600_ir::Param> {
    vec![param("x", exact(Ty::Int)), param("y", exact(Ty::Int))]
}

fn f(name: &str, result: Ty, body: Expr) -> Function {
    helper(name, ints(), exact(result), body)
}

fn to_f2(e: Expr) -> Expr {
    to_field(Field::F2, e)
}

fn to_f5(e: Expr) -> Expr {
    to_field(Field::F5, e)
}

fn op(op: Prim, args: Vec<Expr>) -> Expr {
    prim(op, args)
}

#[test]
fn the_operations_type_in_one_field() {
    let x5 = || to_f5(var("x"));
    let cases = [
        ("add", F5, op(Prim::FieldAdd, vec![x5(), to_f5(var("y"))])),
        ("sub", F5, op(Prim::FieldSub, vec![x5(), to_f5(var("y"))])),
        ("neg", F5, op(Prim::FieldNeg, vec![x5()])),
        ("mul", F5, op(Prim::FieldMul, vec![x5(), op(Prim::FieldInverse, vec![x5()])])),
        ("two", F2, op(Prim::FieldAdd, vec![to_f2(var("x")), to_f2(nat(1))])),
        ("residue", Ty::Nat, op(Prim::Residue, vec![op(Prim::FieldNeg, vec![x5()])])),
        ("equal", Ty::Bool, op(Prim::ExactEq, vec![x5(), to_f5(var("y"))])),
    ];
    for (name, result, body) in cases {
        let r = report(&tool(vec![f(name, result, body)]));
        assert!(r.is_admitted(), "{name}: {:#?}", r.diagnostics);
    }
    // A function takes and returns elements like any exact value.
    let id = helper("id", vec![param("e", exact(F2))], exact(F2), op(Prim::FieldMul, vec![var("e"), var("e")]));
    let caller = f("caller", F2, call("id", vec![to_f2(var("x"))]));
    assert!(report(&tool(vec![id, caller])).is_admitted());
}

/// KN3.2's kinds are separate: no element widens into the other field, into an integer, a rational or Q(√5), or
/// from one. Each mixing is one type mismatch at its operand.
#[test]
fn nothing_widens_into_or_out_of_a_field() {
    let cases = [
        (f("mix", F2, op(Prim::FieldAdd, vec![to_f2(var("x")), to_f5(var("y"))])), "1", "F2", "F5"),
        (f("int", F2, op(Prim::FieldMul, vec![to_f2(var("x")), var("y")])), "1", "F2", "Int"),
        (f("int_first", F2, op(Prim::FieldAdd, vec![var("x"), to_f2(var("y"))])), "0", "F2 | F5", "Int"),
        (f("neg", F5, op(Prim::FieldNeg, vec![var("x")])), "0", "F2 | F5", "Int"),
        (f("result", F2, to_f5(var("x"))), "root", "F2", "F5"),
        (f("residue", Ty::Nat, op(Prim::Residue, vec![var("x")])), "0", "F2 | F5", "Int"),
        (f("exact", Ty::Int, exact_add(to_f2(var("x")), var("y"))), "0", "Nat | Int | Rat | Q5 | Budget", "F2"),
        (f("equal", Ty::Bool, op(Prim::ExactEq, vec![to_f2(var("x")), to_f5(var("y"))])), "1", "F2", "F5"),
        (
            f("ratio", F5, to_f5(Expr::Lit(c600_ir::Lit::Rat { num: "1".into(), den: "2".into() }))),
            "0",
            "Nat | Int",
            "Rat",
        ),
    ];
    for (function, path, expected, supplied) in cases {
        let name = function.name.clone();
        let d = only(&tool(vec![function]));
        assert_eq!(mismatch(&d), (TYPE_MISMATCH, name.as_str(), path, expected, supplied), "{name}");
    }
    // A budget is a cost, not a number to reduce (LG2.6).
    let budget = helper("budget", vec![param("b", exact(Ty::Budget))], exact(F2), to_f2(var("b")));
    assert_eq!(mismatch(&only(&tool(vec![budget]))), (TYPE_MISMATCH, "budget", "0", "Nat | Int", "Budget"));
}

/// A finite field has no order: `exact_less` refuses its elements, as it refuses any operand that is not an exact
/// number.
#[test]
fn a_field_has_no_order() {
    let less = f("less", Ty::Bool, op(Prim::ExactLess, vec![to_f5(var("x")), to_f5(var("y"))]));
    let r = report(&tool(vec![less]));
    let found: Vec<_> = r.diagnostics.iter().map(|d| (d.code, d.path.as_str())).collect();
    assert_eq!(found, [(TYPE_MISMATCH, "0"), (TYPE_MISMATCH, "1")]);
}

/// An approximation does not reduce into a field: the conversion's operand is exact, as an exact addition's is.
#[test]
fn an_approximation_does_not_reduce_into_a_field() {
    let reduced = f("reduced", F2, to_f2(to_approx(var("x"))));
    let added = f("added", Ty::Int, exact_add(to_approx(var("x")), nat(1)));
    assert_eq!(codes(&tool(vec![reduced])), [EXACT_REQUIRED]);
    assert_eq!(codes(&tool(vec![added])), [EXACT_REQUIRED]);
}

/// An element may be a set element or a map key (LP2 draft): its key order is the field, then the residue.
#[test]
fn an_element_may_be_a_set_element_or_a_map_key() {
    let s = set(exact(F2), vec![to_f2(var("x")), to_f2(var("y"))]);
    let m = map(exact(F5), exact(Ty::Int), vec![(to_f5(var("x")), var("x")), (to_f5(var("y")), var("y"))]);
    let cases = [
        f("elements", Ty::Set(Box::new(exact(F2))), s),
        f("keys", Ty::Map(Box::new(exact(F5)), Box::new(exact(Ty::Int))), m),
    ];
    for function in cases {
        let r = report(&tool(vec![function]));
        assert!(r.is_admitted(), "{:#?}", r.diagnostics);
    }
}

/// A goal, an evidence sink or a snapshot read's argument takes portable data with a canonical encoding (owner
/// decision of 16 November 2026 under PHS.52). FP.6 has no form for an F2 or F5 element, so they refuse one, also
/// inside a record or a collection, and take its residue.
#[test]
fn a_goal_evidence_or_snapshot_argument_takes_the_residue() {
    let pair = || record(vec![("at", var("x")), ("value", to_f5(var("y")))]);
    let cases = [
        command("goal", ints(), exact(Ty::Unit), use_exact(ExactRole::Goal, to_f2(var("x")))),
        command("evidence", ints(), exact(Ty::Unit), use_exact(ExactRole::Evidence, pair())),
        helper("read", ints(), exact(Ty::Nat), read_snapshot_with("q", to_f5(var("x")), exact(Ty::Nat))),
    ];
    for function in cases {
        let name = function.name.clone();
        let d = only(&tool(vec![function]));
        let m = mismatch(&d);
        assert_eq!((m.0, m.3), (TYPE_MISMATCH, PORTABLE), "{name}");
    }
    let residue = || op(Prim::Residue, vec![to_f2(var("x"))]);
    let admitted = [
        command("goal", ints(), exact(Ty::Unit), use_exact(ExactRole::Goal, residue())),
        helper("read", ints(), exact(Ty::Nat), read_snapshot_with("q", residue(), exact(Ty::Nat))),
    ];
    for function in admitted {
        let r = report(&tool(vec![function]));
        assert!(r.is_admitted(), "{:#?}", r.diagnostics);
    }
}

/// An element is plain data for the built-in column's rules: it holds no word, so a write of one is not a word write.
/// The write itself stops at run time with `PS1:input.noncanonical_value`, since the element has no canonical form,
/// as for a compressed word; a program stores its residue.
#[test]
fn an_element_holds_no_word() {
    let class: BTreeSet<Row> = [Row::Workspace("kept".into())].into_iter().collect();
    let write = |ty: Ty, value: Expr| {
        let go =
            command("go", vec![param("n", exact(Ty::Int))], exact(Ty::Unit), write_workspace("kept", exact(ty), value));
        tool(vec![go.exported()])
    };
    for (ty, value) in [(F5, to_f5(var("n"))), (Ty::Set(Box::new(exact(F2))), set(exact(F2), vec![to_f2(var("n"))]))] {
        let p = write(ty.clone(), value);
        let a = admit_built_in(&p, "tool", "go", &class, &Grant::none(), &Policy::interim()).unwrap();
        assert!(a.is_admitted(), "{ty:?}: {:#?}", a.diagnostics);
    }
}
