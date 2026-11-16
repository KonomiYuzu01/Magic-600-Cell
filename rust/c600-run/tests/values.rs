//! Values, exact arithmetic, `Approx<p>` rounding and canonical output bytes (LP2, M2).
//!
//! The binary64 golden values were computed independently of this crate, with Python's correctly rounded
//! `float(Fraction)` and `float(Decimal)` at 200 digits.

use std::cmp::Ordering;

use c600_ir::Lit;
use c600_run::approx::Approx;
use c600_run::encode::{encode, Noncanonical};
use c600_run::value::{self, Value};
use num_bigint::BigInt;

fn lit(l: Lit) -> Value {
    value::literal(&l).expect("a valid literal")
}

fn ratio(n: &str, d: &str) -> (String, String) {
    (n.into(), d.into())
}

fn q5(one: (String, String), sqrt5: (String, String)) -> Value {
    lit(Lit::Q5 { one, sqrt5 })
}

fn f64_bits(v: &Value) -> Option<u64> {
    match v {
        Value::Approx(x) => x.f64_bits(),
        _ => panic!("not an approximation: {v:?}"),
    }
}

fn approx(v: &Value, p: u32) -> Value {
    Value::Approx(value::to_approx(v, p).expect("an exact number"))
}

fn bytes(v: &Value) -> Result<String, Noncanonical> {
    let mut out = Vec::new();
    encode(v, &mut out)?;
    Ok(String::from_utf8(out).expect("UTF-8"))
}

#[test]
fn exact_sums_are_canonical() {
    let half = lit(Lit::Rat { num: "1".into(), den: "2".into() });
    let third = lit(Lit::Rat { num: "2".into(), den: "6".into() });
    let sum = value::add(&half, &third).unwrap();
    assert_eq!(bytes(&sum).unwrap(), "{\"rat\":[\"5\",\"6\"]}\n");

    let phi = q5(ratio("1", "2"), ratio("1", "2"));
    let psi = q5(ratio("1", "2"), ratio("-1", "2"));
    let one = value::add(&phi, &psi).unwrap();
    assert_eq!(bytes(&one).unwrap(), "{\"q5\":[\"1\",\"0\",\"1\"]}\n");
    assert!(value::equal(&one, &Value::int(1)));
    assert_eq!(value::compare(&psi, &Value::int(0)), Some(Ordering::Less));
    assert_eq!(
        value::compare(&phi, &lit(Lit::Rat { num: "1618034".into(), den: "1000000".into() })),
        Some(Ordering::Less)
    );
    assert_eq!(
        value::compare(&phi, &lit(Lit::Rat { num: "1618033".into(), den: "1000000".into() })),
        Some(Ordering::Greater)
    );

    let big = Value::int("123456789012345678901234567890".parse::<BigInt>().unwrap());
    assert_eq!(
        bytes(&value::add(&big, &Value::int(-1)).unwrap()).unwrap(),
        "{\"int\":\"123456789012345678901234567889\"}\n"
    );
}

#[test]
fn literal_forms_are_checked() {
    for bad in ["01", "-0", "", "+1", "1.0", "1e3"] {
        assert!(value::literal(&Lit::Int(bad.into())).is_none(), "{bad}");
    }
    assert!(value::literal(&Lit::Nat("-1".into())).is_none());
    assert!(value::literal(&Lit::Rat { num: "1".into(), den: "0".into() }).is_none());
    assert!(value::literal(&Lit::Rat { num: "1".into(), den: "-2".into() }).is_none());
    assert!(value::literal(&Lit::Approx { decimal: "0.5".into(), precision: 32 }).is_none());
    assert!(value::literal(&Lit::Approx { decimal: "0.5.1".into(), precision: 53 }).is_none());
    assert!(value::literal(&Lit::Word(vec![1, 0])).is_none());
    assert!(value::literal(&Lit::Word(vec![1201])).is_none());
    assert!(value::literal(&Lit::Word(vec![-1200, 1])).is_some());
}

#[test]
fn sizes_follow_det_57() {
    assert_eq!(Value::int(0).size(), 2);
    assert_eq!(Value::int(255).size(), 2);
    assert_eq!(Value::int(256).size(), 3);
    assert_eq!(lit(Lit::Rat { num: "1".into(), den: "3".into() }).size(), 4);
    assert_eq!(Value::text("ab").size(), 8);
    assert_eq!(Value::word(&[1, -2, 3]).size(), 6);
    let r = Value::record(vec![("a", Value::Bool(true)), ("b", Value::text("xyz"))]);
    assert_eq!(r.size(), 4 + 1 + (12 + 8));
}

#[test]
fn approximations_round_to_nearest_even() {
    let third = lit(Lit::Rat { num: "1".into(), den: "3".into() });
    assert_eq!(f64_bits(&approx(&third, 53)), Some(0x3fd5555555555555));
    assert_eq!(f64_bits(&approx(&third, 24)), Some(0x3fd5555560000000));
    match value::to_approx(&third, 113).unwrap() {
        Approx { mantissa, exponent, negative: false, .. } => {
            assert_eq!(mantissa, 6923062478046436838040661772293461);
            assert_eq!(exponent, -114);
        }
        other => panic!("{other:?}"),
    }

    let phi = q5(ratio("1", "2"), ratio("1", "2"));
    assert_eq!(f64_bits(&approx(&phi, 53)), Some(0x3ff9e3779b97f4a8));
    let neg_phi = q5(ratio("-1", "2"), ratio("-1", "2"));
    assert_eq!(f64_bits(&approx(&neg_phi, 53)), Some(0xbff9e3779b97f4a8));
    let inv_phi = q5(ratio("-1", "2"), ratio("1", "2"));
    assert_eq!(f64_bits(&approx(&inv_phi, 53)), Some(0x3fe3c6ef372fe950));
    let root5_7 = q5(ratio("0", "1"), ratio("1", "7"));
    assert_eq!(f64_bits(&approx(&root5_7, 53)), Some(0x3fd471ad441b60c0));

    let tenth = lit(Lit::Approx { decimal: "0.1".into(), precision: 53 });
    let fifth = lit(Lit::Approx { decimal: "2e-1".into(), precision: 53 });
    assert_eq!(f64_bits(&tenth), Some(0x3fb999999999999a));
    let (Value::Approx(a), Value::Approx(b)) = (&tenth, &fifth) else { unreachable!() };
    assert_eq!(a.add(*b).f64_bits(), Some(0x3fd3333333333334));
    assert_eq!(a.compare(b), Ordering::Less);
    assert_eq!(a.add(*b).add(Approx { negative: true, ..a.add(*b) }).f64_bits(), Some(0));

    let tie = Value::int((BigInt::from(1) << 53usize) + 1);
    assert_eq!(f64_bits(&approx(&tie, 53)), Some(0x4340000000000000));
    let smallest = lit(Lit::Rat { num: "1".into(), den: (BigInt::from(1) << 1074usize).to_string() });
    assert_eq!(f64_bits(&approx(&smallest, 53)), Some(1));
    let below = lit(Lit::Rat { num: "1".into(), den: (BigInt::from(1) << 1080usize).to_string() });
    assert_eq!(f64_bits(&approx(&below, 53)), None, "below the binary64 range is not publishable");
    assert_eq!(bytes(&approx(&below, 53)), Err(Noncanonical));
    assert_eq!(f64_bits(&approx(&third, 113)), None, "a 113-bit third is not a binary64 number");
}

#[test]
fn canonical_bytes_sort_and_escape() {
    let v = Value::record(vec![
        ("zeta", Value::Unit),
        ("alpha", Value::text("a\"b\\c\n\u{1}é")),
        ("mid", Value::record(vec![("y", Value::Bool(false)), ("x", Value::word(&[3, -1200]))])),
        ("num", approx(&Value::int(1), 53)),
    ]);
    assert_eq!(
        bytes(&v).unwrap(),
        "{\"alpha\":\"a\\\"b\\\\c\\n\\u0001é\",\"mid\":{\"x\":[3,-1200],\"y\":false},\
         \"num\":{\"f64\":\"3ff0000000000000\"},\"zeta\":null}\n"
    );
    assert!(bytes(&v).unwrap().len() as u128 <= v.bound() + 1);
    let dup = Value::record(vec![("k", Value::Unit), ("k", Value::Unit)]);
    assert_eq!(bytes(&dup), Err(Noncanonical));
    assert_eq!(bytes(&Value::Fn(0)), Err(Noncanonical));
}

#[test]
fn deep_values_drop_and_compare_without_recursion() {
    let mut a = Value::Unit;
    let mut b = Value::Unit;
    for _ in 0..200_000 {
        a = Value::record(vec![("x", a)]);
        b = Value::record(vec![("x", b)]);
    }
    assert!(value::equal(&a, &b));
    let mut out = Vec::new();
    encode(&a, &mut out).unwrap();
    assert_eq!(out.len(), 200_000 * 6 + 4 + 1);
}
