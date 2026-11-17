//! The F2 and F5 number kinds of KN3.2 at run time (LP2 drafts until the IR freeze, PHS.49): exact arithmetic modulo
//! the field's prime, the reduction of an integer of any size priced by its size (DET.55: no unmetered big-integer
//! operation), the pointwise assertion that guards `field_inverse` (DET.55; `LG4:input.pointwise_failed`), the key
//! order (field, then residue) and the lack of a canonical form (FP.6 wraps only int, rat and q5). The static half is
//! `c600-check`'s `fields.rs`.

use std::cmp::Ordering;

use c600_check::check;
use c600_ir::Field;
use c600_run::encode::{encode, Noncanonical};
use c600_run::interp::{code, load, Loaded, NoHost, Options, Run};
use c600_run::meter::{self, Amounts, IntOp, Outcome, Summary, Tariffs};
use c600_run::value::{equal, key_order, Value};
use c600_syntax::{lower, parse};
use num_bigint::BigInt;

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const HEADER: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";

/// Parses, lowers, checks and loads one module `m`; the checker must accept it.
fn build(declarations: &str) -> Loaded {
    let source = format!("{HEADER}{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"))
}

fn run(loaded: &Loaded, function: &str, args: Vec<Value>) -> Run {
    loaded.run("m", function, args, &mut NoHost, &Options::new(WIDE)).unwrap_or_else(|r| panic!("{r:?}"))
}

/// The value of a completed run.
fn value(run: &Run) -> &Value {
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?} {:?}", run.summary.code, run.summary.stop);
    run.value.as_ref().expect("a completed run")
}

/// The rows of operation `op` as (steps, kernel cost, memory, released), in order.
fn rows(summary: &Summary, op: &str) -> Vec<(u128, u128, u128, u128)> {
    let suffix = format!("#{op}");
    summary
        .rows
        .iter()
        .filter(|r| r.at.ends_with(&suffix))
        .map(|r| (r.steps, r.kernel_cost, r.memory, r.released))
        .collect()
}

fn int(n: i64) -> Value {
    Value::int(n)
}

const OPS: &str =
    "export fn add(a: Int, b: Int) -> Nat effects [] {\n    return residue(field_add(to_f5(a), to_f5(b)));\n}\n\n\
    export fn sub(a: Int, b: Int) -> Nat effects [] {\n    return residue(field_sub(to_f5(a), to_f5(b)));\n}\n\n\
    export fn mul(a: Int, b: Int) -> Nat effects [] {\n    return residue(field_mul(to_f5(a), to_f5(b)));\n}\n\n\
    export fn neg(a: Int, b: Int) -> Nat effects [] {\n    return residue(field_neg(to_f5(a)));\n}\n\n\
    export fn add2(a: Int, b: Int) -> Nat effects [] {\n    return residue(field_add(to_f2(a), to_f2(b)));\n}\n\n\
    export fn mul2(a: Int, b: Int) -> Nat effects [] {\n    return residue(field_mul(to_f2(a), to_f2(b)));\n}\n\n\
    export fn inv(a: Int) -> Nat effects [] {\n    return residue(field_inverse(to_f5(a)));\n}\n\n\
    export fn inv2(a: Int) -> Nat effects [] {\n    return residue(field_inverse(to_f2(a)));\n}\n\n\
    export fn same(a: Int, b: Int) -> Bool effects [] {\n    return exact_eq(to_f5(a), to_f5(b));\n}\n";

/// Each operation is the integers' operation modulo the prime, on every pair of operands in -12..13, negative ones
/// included: their residue is in 0..p.
#[test]
fn the_operations_are_exact_modulo_the_prime() {
    let loaded = build(OPS);
    let reference: [(&str, i64, fn(i64, i64) -> i64); 6] = [
        ("add", 5, |a, b| a + b),
        ("sub", 5, |a, b| a - b),
        ("mul", 5, |a, b| a * b),
        ("neg", 5, |a, _| -a),
        ("add2", 2, |a, b| a + b),
        ("mul2", 2, |a, b| a * b),
    ];
    for (function, p, f) in reference {
        for a in -12..13 {
            for b in -12..13 {
                let r = run(&loaded, function, vec![int(a), int(b)]);
                assert!(equal(value(&r), &int(f(a, b).rem_euclid(p))), "{function}({a}, {b})");
            }
        }
    }
    for a in -12..13i64 {
        let same = run(&loaded, "same", vec![int(a), int(a + 10)]);
        assert!(equal(value(&same), &Value::Bool(true)), "{a}");
        let other = run(&loaded, "same", vec![int(a), int(a + 1)]);
        assert!(equal(value(&other), &Value::Bool(false)), "{a}");
    }
}

/// Every nonzero element has its inverse: x · x⁻¹ = 1.
#[test]
fn every_nonzero_element_has_its_inverse() {
    let loaded = build(OPS);
    for (function, p) in [("inv", 5i64), ("inv2", 2)] {
        for a in 1..p {
            let r = run(&loaded, function, vec![int(a)]);
            let Value::Int(inverse) = value(&r) else { panic!("a residue") };
            let inverse = i64::try_from(&**inverse).unwrap();
            assert_eq!(a * inverse % p, 1, "{function}({a})");
            // The predicate, the assertion and the inverse are one step each; the assertion consumes the predicate's
            // Bool (DET.55), and the inverse drops its operand element.
            assert_eq!(rows(&r.summary, "field_nonzero"), [(1, 0, 1, 0)]);
            assert_eq!(rows(&r.summary, "assert"), [(1, 0, 0, 1)]);
            assert_eq!(rows(&r.summary, "field_inverse"), [(1, 0, 1, 1)]);
        }
    }
}

/// The inverse of zero is a pointwise failure (DET.55): the predicate and the assertion run and are charged, and the
/// run stops at the assertion with `LG4:input.pointwise_failed`, before the guarded operation, keeping the admitted
/// charges (chapter 13). Zero is any multiple of the prime.
#[test]
fn the_inverse_of_zero_stops_at_its_assertion() {
    let loaded = build(OPS);
    for (function, a) in [("inv", 0), ("inv", 10), ("inv", -5), ("inv2", 4)] {
        let r = run(&loaded, function, vec![int(a)]);
        assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(meter::code::POINTWISE_FAILED)));
        let stop = r.summary.stop.as_ref().expect("a stop");
        assert!(stop.at.ends_with("#assert"), "{}", stop.at);
        assert_eq!(rows(&r.summary, "field_nonzero"), [(1, 0, 1, 0)], "{function}({a})");
        assert_eq!(rows(&r.summary, "assert"), [(1, 0, 0, 1)], "{function}({a})");
        assert!(rows(&r.summary, "field_inverse").is_empty(), "the guarded operation does not run");
        assert!(r.value.is_none() && r.output.is_none());
    }
}

const REDUCE: &str = "export fn reduce(n: Int) -> Nat effects [] {\n    return residue(to_f5(n));\n}\n\n\
    export fn reduce2(n: Int) -> Nat effects [] {\n    return residue(to_f2(n));\n}\n";

/// `to_f2` and `to_f5` reduce an integer of any size and are priced by it: the integer row's price for adding the
/// prime, one walk over the integer's limbs (LP2 draft tariff). The field operations' other rows cost no kernel units.
#[test]
fn a_reduction_is_priced_by_the_integers_size() {
    let loaded = build(REDUCE);
    let big: BigInt = BigInt::from(10u32).pow(300u32) + 3u32;
    let cases = [
        (BigInt::from(7), 2, 1),
        (BigInt::from(-7), 3, 1),
        (big.clone(), 3, 1),
        (-big.clone(), 2, 1),
        (BigInt::from(0), 0, 0),
    ];
    let price = |n: &BigInt, prime: u128| Tariffs::TEST.integer(IntOp::Add, n.bits() as u128, prime).0;
    for (n, mod5, mod2) in cases {
        let r5 = run(&loaded, "reduce", vec![Value::int(n.clone())]);
        assert!(equal(value(&r5), &int(mod5)), "{n}");
        // The prime 5 has 3 bits, 2 has 2: one limb either way.
        assert_eq!(rows(&r5.summary, "to_f5"), [(1, price(&n, 3), 1, 0)], "{n}");
        // A residue is an integer of one magnitude byte, 2 logical bytes; the element it reads is dropped.
        assert_eq!(rows(&r5.summary, "residue"), [(1, 0, 2, 1)]);
        let r2 = run(&loaded, "reduce2", vec![Value::int(n.clone())]);
        assert!(equal(value(&r2), &int(mod2)), "{n}");
        assert_eq!(rows(&r2.summary, "to_f2"), [(1, price(&n, 2), 1, 0)], "{n}");
    }
    assert!(price(&big, 3) > price(&BigInt::from(7), 3), "a larger integer costs more");
}

/// A run that cannot pay for a reduction stops before it: its kernel cost is admitted first, and the refused request
/// debits nothing (DET.56). The entry argument's `read` is paid before, one unit per encoded byte (DET.57).
#[test]
fn a_reduction_beyond_the_kernel_budget_stops_before_it_runs() {
    let loaded = build(REDUCE);
    let n = BigInt::from(10u32).pow(3000u32);
    let mut bytes = Vec::new();
    encode(&Value::int(n.clone()), &mut bytes).unwrap();
    let read = Tariffs::TEST.decode_per_byte * bytes.len() as u128;
    let price = Tariffs::TEST.integer(IntOp::Add, n.bits() as u128, 3).0;
    let tight = Amounts { kernel_cost: read + price - 1, ..WIDE };
    let r = loaded.run("m", "reduce", vec![Value::int(n)], &mut NoHost, &Options::new(tight)).unwrap();
    assert_eq!((r.summary.outcome, r.summary.code), (Outcome::CutOff, Some(meter::code::KERNEL_COST)));
    assert!(r.summary.stop.as_ref().unwrap().at.ends_with("#to_f5"));
    assert_eq!(r.summary.used.kernel_cost, read);
}

/// An element is one logical byte and has no canonical form: FP.6 wraps only int, rat and q5 (PS1.5). A run that
/// would output one stops with `PS1:input.noncanonical_value`, and an entry argument of one is refused; a program
/// outputs and stores the residue.
#[test]
fn an_element_has_no_canonical_form() {
    for v in [Value::Field(Field::F2, 1), Value::Field(Field::F5, 4)] {
        assert_eq!(v.size(), 1);
        assert_eq!(v.bound(), 0);
        let mut bytes = Vec::new();
        assert_eq!(encode(&v, &mut bytes), Err(Noncanonical));
    }
    let loaded = build(
        "export fn out(n: Int) -> F5 effects [] {\n    return to_f5(n);\n}\n\n\
         export fn id(x: F2) -> F2 effects [] {\n    return x;\n}\n",
    );
    let shown = run(&loaded, "out", vec![int(3)]);
    assert_eq!((shown.summary.outcome, shown.summary.code), (Outcome::Error, Some(code::NONCANONICAL_VALUE)));
    let refused = loaded.run("m", "id", vec![Value::Field(Field::F2, 1)], &mut NoHost, &Options::new(WIDE));
    assert_eq!(refused.map(|_| ()).unwrap_err().code, code::NONCANONICAL_VALUE);
}

/// A sink takes canonical bytes, so a workspace write or a published view of an element, alone or in a record, stops
/// at its step with `PS1:input.noncanonical_value` before the host is asked, as an output does. The checker admits the
/// write, as it admits one of a compressed word: an element holds no word (`c600-check`'s `fields.rs`).
#[test]
fn a_sink_of_an_element_stops_before_the_host_is_asked() {
    let loaded = build(
        "export fn keep(n: Int) -> Unit effects [workspace] {\n    \
         return write_workspace<F5>(\"kept\", to_f5(n));\n}\n\n\
         export fn show(n: Int) -> Unit effects [view] {\n    \
         return publish_view(record { at = n, value = to_f2(n) });\n}\n",
    );
    for (function, op) in [("keep", "emit"), ("show", "publish_view")] {
        let r = run(&loaded, function, vec![int(3)]);
        assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(code::NONCANONICAL_VALUE)), "{function}");
        assert!(r.summary.stop.as_ref().unwrap().at.ends_with(&format!("#{op}")), "{function}");
    }
}

/// The key order (LP2 draft): elements compare by their field, F2 first, then by their residue, and after every other
/// exact number. Equality needs the same field and residue.
#[test]
fn elements_order_by_field_then_residue() {
    let (f2, f5) = (|r| Value::Field(Field::F2, r), |r| Value::Field(Field::F5, r));
    assert_eq!(key_order(&f2(1), &f5(0)), Ordering::Less);
    assert_eq!(key_order(&f5(1), &f5(3)), Ordering::Less);
    assert_eq!(key_order(&f5(3), &f5(3)), Ordering::Equal);
    assert_eq!(key_order(&int(9), &f2(0)), Ordering::Less);
    assert!(equal(&f5(2), &f5(2)));
    assert!(!equal(&f2(1), &f5(1)));
    assert!(!equal(&f5(1), &int(1)));
    let loaded = build(
        "export fn count(a: Int, b: Int, c: Int) -> Nat effects [] {\n    \
         return length(set<F5>(to_f5(a), to_f5(b), to_f5(c)));\n}\n\n\
         export fn least(a: Int, b: Int, c: Int) -> Nat effects [] {\n    \
         let s = set<F5>(to_f5(a), to_f5(b), to_f5(c));\n    \
         return residue(index(elements(s, ascending), 0));\n}\n\n\
         export fn under(a: Int, b: Int) -> Int effects [] {\n    \
         return lookup(map<F2, Int>(to_f2(a), a), to_f2(b));\n}\n",
    );
    // 7, 2 and -3 are one element of F5; 1, 2 and 3 are three.
    assert!(equal(value(&run(&loaded, "count", vec![int(7), int(2), int(-3)])), &int(1)));
    assert!(equal(value(&run(&loaded, "count", vec![int(1), int(2), int(3)])), &int(3)));
    assert!(equal(value(&run(&loaded, "least", vec![int(9), int(-3), int(8)])), &int(2)));
    assert!(equal(value(&run(&loaded, "under", vec![int(5), int(-1)])), &int(5)));
    let missing = run(&loaded, "under", vec![int(5), int(2)]);
    assert_eq!(missing.summary.code, Some(meter::code::POINTWISE_FAILED));
}
