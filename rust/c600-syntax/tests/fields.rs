//! The F2 and F5 number kinds of KN3.2 in source (LP2 drafts until the IR freeze, PHS.49). Candidate A calls their
//! operations like functions: `to_f2` and `to_f5` reduce an integer, `field_add`, `field_sub`, `field_neg`,
//! `field_mul` and `field_inverse` work in one field, and `residue` gives an element's residue as a `Nat`. `F2` and
//! `F5` are base types. There is no F2 or F5 literal (LG1.6): an element comes only from a reduction. The checker's
//! half is `c600-check`'s `fields.rs`, and the run-time half is `c600-run`'s `fields.rs`.

use c600_check::{check, Report, TYPE_MISMATCH};
use c600_ir::{Expr as Ir, Field, Prim, Ty};
use c600_syntax::{lower, parse, Lowered};

const HEADER: &str = "edition \"6cs-draft-1\";\nmodule tool requires [];\n\n";

fn lowered(declarations: &str) -> Result<Lowered, String> {
    let source = format!("{HEADER}{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    lower(&[file]).map_err(|e| e.to_string())
}

fn report(declarations: &str) -> Report {
    let lowered = lowered(declarations).unwrap_or_else(|e| panic!("{declarations}: {e}"));
    check(&lowered.program).unwrap_or_else(|e| panic!("{declarations}: {e}"))
}

fn admitted(declarations: &str) {
    let r = report(declarations);
    assert!(r.is_admitted(), "{declarations}: {:#?}", r.diagnostics);
}

fn codes(declarations: &str) -> Vec<&'static str> {
    report(declarations).diagnostics.iter().map(|d| d.code).collect()
}

/// The only finding's code, and the expected and supplied types of a mismatch.
fn only(declarations: &str) -> (&'static str, Option<(String, String)>) {
    let r = report(declarations);
    assert_eq!(r.diagnostics.len(), 1, "{declarations}: {:#?}", r.diagnostics);
    let d = &r.diagnostics[0];
    (d.code, d.mismatch.as_ref().map(|m| (m.expected.clone(), m.supplied.clone())))
}

/// Every expression of every function, depth first.
fn expressions(lowered: &Lowered) -> Vec<&Ir> {
    let mut out = Vec::new();
    let mut pending: Vec<&Ir> = lowered.program.modules.iter().flat_map(|m| &m.functions).map(|f| &f.body).collect();
    while let Some(e) = pending.pop() {
        out.push(e);
        pending.extend(e.children());
    }
    out
}

/// Each operation lowers to its primitive, and `F2` and `F5` name the field types.
#[test]
fn the_field_operations_lower_to_their_primitives() {
    let source = "fn f(x: Int, e: F5, t: F2) -> Nat effects [] {\n    \
                  let a = field_add(to_f5(x), field_sub(e, field_neg(e)));\n    \
                  let b = field_mul(a, field_inverse(to_f5(1)));\n    \
                  let c = exact_eq(field_add(to_f2(x), t), to_f2(0));\n    \
                  return residue(b);\n}\n";
    let lowered = lowered(source).unwrap_or_else(|e| panic!("{e}"));
    let f = &lowered.program.modules[0].functions[0];
    assert_eq!(f.params[1].ty.ty, Ty::Field(Field::F5));
    assert_eq!(f.params[2].ty.ty, Ty::Field(Field::F2));
    let mut ops: Vec<Prim> = expressions(&lowered)
        .iter()
        .filter_map(|e| match e {
            Ir::Prim { op, .. } => Some(*op),
            _ => None,
        })
        .collect();
    ops.sort_by_key(|op| format!("{op:?}"));
    ops.dedup();
    let want = [
        Prim::ExactEq,
        Prim::FieldAdd,
        Prim::FieldInverse,
        Prim::FieldMul,
        Prim::FieldNeg,
        Prim::FieldSub,
        Prim::Residue,
        Prim::ToField(Field::F2),
        Prim::ToField(Field::F5),
    ];
    assert_eq!(ops, want);
    admitted(source);
}

/// There is no F2 or F5 literal (LG1.6): a number literal is never an element, and an element of one field is not one
/// of the other. A reduction names its field.
#[test]
fn there_is_no_element_literal() {
    let cases = [
        ("fn f() -> F5 effects [] {\n    return 3;\n}\n", "F5", "Nat"),
        ("fn f(e: F2) -> F2 effects [] {\n    return field_add(e, 1);\n}\n", "F2", "Nat"),
        ("fn f() -> F5 effects [] {\n    return to_f2(3);\n}\n", "F5", "F2"),
    ];
    for (declaration, expected, supplied) in cases {
        let (code, mismatch) = only(declaration);
        assert_eq!(code, TYPE_MISMATCH, "{declaration}");
        assert_eq!(mismatch, Some((expected.into(), supplied.into())), "{declaration}");
    }
    admitted("fn f() -> F5 effects [] {\n    return to_f5(3);\n}\n");
}

/// The draft's refusals in source: the operations take their arity and no type argument, their names are reserved,
/// and `F2` and `F5` are the only finite fields.
#[test]
fn malformed_field_operations_do_not_lower() {
    let refused = [
        ("fn f(x: Int) -> F5 effects [] {\n    return to_f5(x, x);\n}\n", "to_f5"),
        ("fn f(e: F5) -> F5 effects [] {\n    return field_add(e);\n}\n", "field_add"),
        ("fn f(e: F5) -> F5 effects [] {\n    return field_inverse(e, e);\n}\n", "field_inverse"),
        ("fn f(e: F5) -> Nat effects [] {\n    return residue<F5>(e);\n}\n", "no type argument"),
        ("fn f(x: Int) -> Int effects [] {\n    let residue = x;\n    return residue;\n}\n", "residue"),
        ("fn to_f2(x: Int) -> Int effects [] {\n    return x;\n}\n", "to_f2"),
        ("fn f(x: F3) -> Nat effects [] {\n    return 1;\n}\n", "F3"),
    ];
    for (declaration, says) in refused {
        let message = match lowered(declaration) {
            Ok(_) => panic!("lowered: {declaration}"),
            Err(e) => e,
        };
        assert!(message.contains(says), "{declaration}: {message}");
    }
}

/// An element may be a set element or a map key (LP2 draft), but `exact_less` refuses it: a finite field has no order.
#[test]
fn an_element_is_a_key_without_an_order() {
    admitted("fn f(x: Int) -> Set<F2> effects [] {\n    return set<F2>(to_f2(x), to_f2(1));\n}\n");
    admitted("fn f(x: Int) -> Map<F5, Int> effects [] {\n    return map<F5, Int>(to_f5(x), x);\n}\n");
    let less = "fn f(x: Int) -> Bool effects [] {\n    return exact_less(to_f5(x), to_f5(1));\n}\n";
    assert_eq!(codes(less), [TYPE_MISMATCH, TYPE_MISMATCH]);
}
