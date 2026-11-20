//! The LP2 drafts `q5_parts` and `q5_of` at run time (until the IR freeze, PHS.49): a Q(√5) value's canonical integer
//! triple (ADR-L03.6, PS1.4) and the value of a triple. Together they let a library take an exact number apart and
//! build one, so an inverse, a conjugate, a quotient or an integer half is a library body over exact arithmetic, not an
//! operation of its own. Both are priced as `exact_mul` is (DET.57), and `q5_of`'s zero denominator is a pointwise
//! failure (DET.55; `LG4:input.pointwise_failed`), as `field_inverse`'s zero is. The static half is `c600-check`'s
//! `mismatch.rs`.

use std::sync::Arc;

use c600_check::check;
use c600_run::interp::{load, Loaded, NoHost, Options, Run};
use c600_run::meter::{self, Amounts, DraftTariffs, Outcome, Summary, Tariffs};
use c600_run::value::{self, equal, Quad, Ratio, Value};
use c600_syntax::{lower, parse};
use num_bigint::BigInt;

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const HEADER: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";

const OPS: &str = "export fn parts(x: Q5) -> record { a: Int, b: Int, d: Int } effects [] {\n    \
    return q5_parts(x);\n}\n\n\
    export fn parts_rat(x: Rat) -> record { a: Int, b: Int, d: Int } effects [] {\n    return q5_parts(x);\n}\n\n\
    export fn parts_int(x: Int) -> record { a: Int, b: Int, d: Int } effects [] {\n    return q5_parts(x);\n}\n\n\
    export fn of(a: Int, b: Int, d: Int) -> Q5 effects [] {\n    return q5_of(a, b, d);\n}\n\n\
    export fn inverse(x: Q5) -> Q5 effects [] {\n    \
    let p = q5_parts(x);\n    \
    let n = exact_add(exact_mul(p.a, p.a), exact_mul(-5, exact_mul(p.b, p.b)));\n    \
    return q5_of(exact_mul(p.a, p.d), exact_mul(exact_mul(-1, p.b), p.d), n);\n}\n\n\
    export fn half(n: Int) -> Int effects [] {\n    let p = q5_parts(q5_of(n, 0, 2));\n    return p.a;\n}\n";

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

/// The rows of operation `op` as (steps, kernel cost, memory), in order.
fn rows(summary: &Summary, op: &str) -> Vec<(u128, u128, u128)> {
    let suffix = format!("#{op}");
    summary.rows.iter().filter(|r| r.at.ends_with(&suffix)).map(|r| (r.steps, r.kernel_cost, r.memory)).collect()
}

fn int(n: i64) -> Value {
    Value::int(n)
}

fn q5(a: i64, b: i64, d: i64) -> Value {
    Value::Q5(Arc::new(Quad::new(a.into(), b.into(), d.into())))
}

/// A record `{ a, b, d }` of integers, its fields in that order.
fn triple(v: &Value) -> (BigInt, BigInt, BigInt) {
    let Value::Record(r) = v else { panic!("a record, not {v:?}") };
    let names: Vec<&str> = r.fields().iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(names, ["a", "b", "d"]);
    let get = |name: &str| match r.get(name) {
        Some(Value::Int(n)) => (**n).clone(),
        other => panic!("an integer field {name}, not {other:?}"),
    };
    (get("a"), get("b"), get("d"))
}

fn big(a: i64, b: i64, d: i64) -> (BigInt, BigInt, BigInt) {
    (a.into(), b.into(), d.into())
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

/// `q5_parts` gives the canonical triple: lowest terms with a positive denominator. An integer or a rational widens
/// to Q(√5) as a sum does, with a zero √5 coefficient and, for an integer, the denominator 1.
#[test]
fn the_parts_are_the_canonical_triple() {
    let loaded = build(OPS);
    let cases = [
        (q5(1, 1, 2), big(1, 1, 2)),
        (q5(-9, 2, 12), big(-9, 2, 12)),
        (q5(4, -6, -8), big(-2, 3, 4)),
        (q5(0, 3, 9), big(0, 1, 3)),
        (q5(0, 0, -7), big(0, 0, 1)),
    ];
    for (x, expected) in cases {
        let r = run(&loaded, "parts", vec![x.clone()]);
        assert_eq!(triple(value(&r)), expected, "{x:?}");
    }
    let rat = Value::Rat(Arc::new(Ratio::new(BigInt::from(-6), BigInt::from(4))));
    assert_eq!(triple(value(&run(&loaded, "parts_rat", vec![rat]))), big(-3, 0, 2));
    for n in [-7i64, 0, 1, 12] {
        assert_eq!(triple(value(&run(&loaded, "parts_int", vec![int(n)]))), big(n, 0, 1), "{n}");
    }
    let huge = BigInt::from(-3) * BigInt::from(2u32).pow(200u32);
    let r = run(&loaded, "parts_int", vec![Value::int(huge.clone())]);
    assert_eq!(triple(value(&r)), (huge, BigInt::from(0), BigInt::from(1)));
}

/// `q5_of(a, b, d)` is (a + b√5)/d in canonical form for every sign of every operand: the gcd of the three is divided
/// out and a negative denominator's sign moves to both numerators. The reference here normalises with `i64`, apart
/// from the runtime's own `Quad::new`. The value equals an integer or a rational of the same number.
#[test]
fn a_triple_builds_its_canonical_value() {
    let loaded = build(OPS);
    for a in -6i64..7 {
        for b in -6i64..7 {
            for d in (-6i64..7).filter(|d| *d != 0) {
                let r = run(&loaded, "of", vec![int(a), int(b), int(d)]);
                let Value::Q5(q) = value(&r) else { panic!("a Q5 value") };
                let g = gcd(gcd(a, b), d);
                let s = d.signum();
                assert_eq!((q.a.clone(), q.b.clone(), q.d.clone()), big(s * a / g, s * b / g, d.abs() / g), "{a} {b} {d}");
            }
        }
    }
    assert!(equal(value(&run(&loaded, "of", vec![int(6), int(0), int(-3)])), &int(-2)));
    let rat = Value::Rat(Arc::new(Ratio::new(BigInt::from(2), BigInt::from(3))));
    assert!(equal(value(&run(&loaded, "of", vec![int(-4), int(0), int(-6)])), &rat));
}

/// A zero denominator is a pointwise failure (DET.55): the predicate and the assertion run and are charged, and the run
/// stops at the assertion with `LG4:input.pointwise_failed`, before the construction, keeping the admitted charges
/// (chapter 13).
#[test]
fn a_zero_denominator_stops_at_its_assertion() {
    let loaded = build(OPS);
    for (a, b) in [(1, 2), (0, 0), (-3, 5)] {
        let r = run(&loaded, "of", vec![int(a), int(b), int(0)]);
        assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(meter::code::POINTWISE_FAILED)));
        let stop = r.summary.stop.as_ref().expect("a stop");
        assert!(stop.at.ends_with("#assert"), "{}", stop.at);
        assert_eq!(rows(&r.summary, "denominator_nonzero"), [(1, 0, 1)], "{a} {b}");
        assert_eq!(rows(&r.summary, "assert"), [(1, 0, 0)], "{a} {b}");
        assert!(rows(&r.summary, "q5_of").is_empty(), "the guarded operation does not run");
        assert!(r.value.is_none() && r.output.is_none());
    }
}

/// The library's derivations run as written: the inverse of (a + b√5)/d is q5_of(a·d, −b·d, a² − 5b²), so x · x⁻¹ = 1
/// and φ⁻¹ = φ − 1; an even integer's half is the first part of q5_of(n, 0, 2).
#[test]
fn an_inverse_and_a_half_are_library_bodies() {
    let loaded = build(OPS);
    let phi = q5(1, 1, 2);
    let inverse = run(&loaded, "inverse", vec![phi.clone()]);
    assert!(equal(value(&inverse), &q5(-1, 1, 2)));
    for x in [phi, q5(-9, 2, 12), q5(3, -1, 1), q5(5, 0, 7), q5(-4, 6, 8), q5(0, -2, 3)] {
        let inverse = run(&loaded, "inverse", vec![x.clone()]);
        let product = value::mul(&x, value(&inverse)).expect("two numbers");
        assert!(equal(&product, &int(1)), "{x:?}");
    }
    for n in (-10i64..11).step_by(2) {
        assert!(equal(value(&run(&loaded, "half", vec![int(n)])), &int(n / 2)), "{n}");
    }
    let even = BigInt::from(2u32).pow(201u32);
    let r = run(&loaded, "half", vec![Value::int(-even.clone())]);
    assert!(equal(value(&r), &Value::int(-even / 2)));
}

/// Both are priced as `exact_mul` is (DET.57). `q5_parts` costs `field_per_unit_pair` per byte of its operand widened
/// to Q(√5) and reserves the record exactly: a 4-byte tag and the three integers. `q5_of` costs what the product of
/// a + b√5 and 1/d costs and reserves what that unreduced product could need, which settles on the canonical value.
#[test]
fn the_rows_are_priced_as_products() {
    let loaded = build(OPS);
    // (1, 1, 2) is three two-byte integers: 6 units, and 4 + 6 bytes for the record.
    let r = run(&loaded, "parts", vec![q5(1, 1, 2)]);
    assert_eq!(rows(&r.summary, "q5_parts"), [(1, 6, 10)]);
    assert_eq!((r.summary.live_before_cleanup, r.summary.live_after_cleanup), (0, 0));
    // (2, 4, 1) times (1, 0, 6): 6 · 6 units; the unreduced (2, 4, 6) reserves 2 + 2 + 2 bytes.
    let r = run(&loaded, "of", vec![int(2), int(4), int(6)]);
    assert_eq!(rows(&r.summary, "denominator_nonzero"), [(1, 0, 1)]);
    assert_eq!(rows(&r.summary, "assert"), [(1, 0, 0)]);
    assert_eq!(rows(&r.summary, "q5_of"), [(1, 36, 6)]);
    assert_eq!((r.summary.live_before_cleanup, r.summary.live_after_cleanup), (0, 0));
    let (t, d) = (Tariffs::TEST, DraftTariffs::TEST);
    let n = BigInt::from(7u32).pow(150u32);
    let wide = [Value::int(-n.clone()), Value::int(n.clone() * 3u32), Value::int(n.clone() * -2)];
    let r = run(&loaded, "of", wide.to_vec());
    let (kernel, memory) = value::q5_of_price(&wide[0], &wide[1], &wide[2], &t, &d).expect("three integers");
    assert_eq!(rows(&r.summary, "q5_of"), [(1, kernel, memory)]);
    assert!(kernel > 36, "a larger operand costs more");
    let Value::Q5(q) = value(&r) else { panic!("a Q5 value") };
    assert_eq!((q.a.clone(), q.b.clone(), q.d.clone()), (BigInt::from(1), BigInt::from(-3), BigInt::from(2)));
    let x = Value::Q5(Arc::new(Quad::new(n.clone(), -n.clone() * 5u32, BigInt::from(3))));
    let r = run(&loaded, "parts", vec![x.clone()]);
    let (kernel, memory) = value::q5_parts_price(&x, &d).expect("a number");
    assert_eq!(rows(&r.summary, "q5_parts"), [(1, kernel, memory)]);
    assert_eq!(memory, 4 + x.size());
}
