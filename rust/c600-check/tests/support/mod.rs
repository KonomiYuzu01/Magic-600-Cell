//! Shared helpers for the checker tests: the frozen LP1 vectors and short program builders.

#![allow(dead_code)]

pub mod json;

use c600_check::{check, Diagnostic, Report};
use c600_ir::build::*;
use c600_ir::{Expr, Function, Param, Program, Ty};
use json::Json;

/// The frozen LP1 contract vectors. The tests read them in place and never change them.
pub const VECTORS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/architecture/1.0/language/contract-vectors/vectors.json");

/// One case of an `exactness_diagnostic` vector with its expected checker code.
#[derive(Clone, Debug)]
pub struct Case {
    pub site: String,
    pub value_type: String,
    pub display_control: bool,
    pub qualifier_erasure: bool,
    pub code: String,
}

/// The cases of the boundary vector `id`, each paired with its expected checker code.
pub fn boundary_vector(id: &str) -> Vec<Case> {
    let text = std::fs::read_to_string(VECTORS).unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    let root = Json::parse(&text).unwrap_or_else(|e| panic!("vectors.json does not parse: {e}"));
    let vector = root
        .get("boundary_vectors")
        .and_then(Json::arr)
        .expect("boundary_vectors")
        .iter()
        .find(|v| v.get("id").and_then(Json::str) == Some(id))
        .unwrap_or_else(|| panic!("no boundary vector `{id}`"));
    let given = vector.get("given").expect("given");
    assert_eq!(given.get("kind").and_then(Json::str), Some("exactness_diagnostic"), "{id}");
    let cases = given.get("cases").and_then(Json::arr).expect("cases");
    let codes = vector.get("expected").and_then(|e| e.get("checker_codes")).and_then(Json::arr).expect("codes");
    assert_eq!(cases.len(), codes.len(), "{id}: one checker code per case");
    cases
        .iter()
        .zip(codes)
        .map(|(c, code)| Case {
            site: c.get("site").and_then(Json::str).expect("site").to_string(),
            value_type: c.get("value_type").and_then(Json::str).expect("value_type").to_string(),
            display_control: c.get("display_control").and_then(Json::bool).expect("display_control"),
            qualifier_erasure: c.get("qualifier_erasure").and_then(Json::bool).unwrap_or(false),
            code: code.str().expect("code").to_string(),
        })
        .collect()
}

pub fn report(p: &Program) -> Report {
    check(p).unwrap_or_else(|e| panic!("{e}"))
}

pub fn codes(p: &Program) -> Vec<&'static str> {
    report(p).codes()
}

/// The only diagnostic of a program.
pub fn only(p: &Program) -> Diagnostic {
    let r = report(p);
    assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
    r.diagnostics[0].clone()
}

/// The exactness fields of a diagnostic, as the vectors give them.
pub fn fields(d: &Diagnostic) -> (&'static str, Option<&str>, bool, bool) {
    (d.code, d.value_type.as_deref(), d.display_control, d.qualifier_erasure)
}

/// The fields of a type mismatch: code, function, path, and the expected and supplied types.
pub fn mismatch(d: &Diagnostic) -> (&'static str, &str, &str, &str, &str) {
    let m = d.mismatch.as_ref().unwrap_or_else(|| panic!("no mismatch types: {d:#?}"));
    (d.code, d.function.as_str(), d.path.as_str(), m.expected.as_str(), m.supplied.as_str())
}

/// A program with one module `tool` holding the given functions.
pub fn tool(functions: Vec<Function>) -> Program {
    program(vec![module("tool", functions)])
}

/// Two exact inputs `x` and `y`.
pub fn xy() -> Vec<Param> {
    vec![param("x", exact(Ty::Nat)), param("y", exact(Ty::Nat))]
}

/// `approx_less(x, y)` on the approximations of the exact inputs.
pub fn less() -> Expr {
    approx_less(to_approx(var("x")), to_approx(var("y")))
}

/// Pure display branches choosing two colours.
pub fn tint() -> Expr {
    display_if(less(), colour("red"), colour("blue"))
}
