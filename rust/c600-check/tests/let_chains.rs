//! A long straight-line body (LP2): the checker checks a chain of lets in a loop, so the chain's length does not
//! deepen its recursion, whose levels have large frames in an unoptimised build, and a finding inside the chain keeps
//! the path that a nested check gives it: a let's value at `0` below it, its body at `1`.

mod support;

use c600_check::TYPE_MISMATCH;
use c600_ir::build::*;
use c600_ir::{Expr, Lit, Ty};
use support::{mismatch, report, tool};

/// `let x0 = 0; let x1 = exact_add(x0, 1); ...` for `n` lets, then `last`. Built from the inside out, without
/// recursion.
fn chain(n: usize, at: usize, bad: Expr, last: Expr) -> Expr {
    let mut body = last;
    for i in (0..n).rev() {
        let value = match i {
            i if i == at => bad.clone(),
            0 => nat(0),
            i => exact_add(var(&format!("x{}", i - 1)), nat(1)),
        };
        body = let_(&format!("x{i}"), value, body);
    }
    body
}

/// A text literal.
fn text() -> Expr {
    Expr::Lit(Lit::Text("a".into()))
}

/// Runs `f` on a thread of `kib` KiB of stack.
fn on_stack<T: Send + 'static>(kib: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new().stack_size(kib << 10).spawn(f).expect("a thread").join().expect("no overflow")
}

/// Five hundred lets check on a thread of 1 MiB, where one level of the checker's recursion per let would need many
/// times that, and their last name is in scope at the end.
#[test]
fn a_long_let_chain_checks_on_a_small_stack() {
    let admitted = on_stack(1024, || {
        let body = chain(500, usize::MAX, unit(), var("x499"));
        report(&tool(vec![helper("f", vec![], exact(Ty::Nat), body)])).is_admitted()
    });
    assert!(admitted);
}

/// A mismatch in the value of a chain's third let is reported at `1.1.0` and the operand's index, and one in the
/// chain's last expression at one `1` per let; a name bound inside the chain is out of scope after it.
#[test]
fn a_finding_in_a_let_chain_keeps_its_nested_path() {
    let bad = exact_add(var("x1"), text());
    let p = tool(vec![helper("f", vec![], exact(Ty::Nat), chain(4, 2, bad, var("x3")))]);
    let r = report(&p);
    assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
    assert_eq!(mismatch(&r.diagnostics[0]).0, TYPE_MISMATCH);
    assert_eq!(r.diagnostics[0].path, "1.1.0.1");

    let last = exact_add(var("x3"), text());
    let p = tool(vec![helper("f", vec![], exact(Ty::Nat), chain(4, usize::MAX, unit(), last))]);
    let r = report(&p);
    assert_eq!(r.diagnostics.len(), 1, "{:#?}", r.diagnostics);
    assert_eq!(r.diagnostics[0].path, "1.1.1.1.1");

    // The chain's names are in scope only inside it.
    let after: Expr = let_("y", chain(2, usize::MAX, unit(), var("x1")), var("x1"));
    let error = c600_check::check(&tool(vec![helper("f", vec![], exact(Ty::Nat), after)])).unwrap_err();
    assert!(error.to_string().contains("unbound name `x1`"), "{error}");
}
