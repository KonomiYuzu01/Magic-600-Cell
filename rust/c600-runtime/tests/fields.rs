//! LP2's draft F2 and F5 elements (KN3.2) in C3's transfer forms (draft `c600-rt1-wire-1`): an element crosses as value
//! tag 24, its field's byte and its residue's byte, and the field types as type tag 26 and the field's byte. Each
//! round-trips with one encoding; an unknown field byte and a residue at or above the prime are refused on both sides.
//! An element has no canonical form (FP.6 wraps only int, rat and q5), so as an argument it reaches the child, which
//! refuses it as the run in process does, and a result stops both runs alike; a program that outputs a residue runs
//! the same in both. Source and synthetic evidence only.

mod support;

use std::collections::BTreeSet;

use c600_ir::{Field, QTy, Ty};
use c600_run::interp::code::NONCANONICAL_VALUE;
use c600_run::meter::code::POINTWISE_FAILED;
use c600_run::meter::Outcome;
use c600_run::value::Value;
use c600_runtime::codec::{self, Unencodable};
use c600_runtime::wire::{Dec, Enc};
use c600_runtime::{Ended, JobOptions, Runtime, Stage};
use num_bigint::BigInt;
use support::{compare, header, job, ran, runtime, Session, WIDE};

/// Encodes `v`, decodes it and encodes it again: one encoding, and the same value back.
fn round_trip(v: &Value) -> Vec<u8> {
    let bytes = codec::value_bytes(v).unwrap_or_else(|e| panic!("{v:?}: {e:?}"));
    let back = codec::value_from_bytes(&bytes).unwrap_or_else(|e| panic!("{v:?}: {e}"));
    assert_eq!(format!("{v:?}"), format!("{back:?}"));
    assert_eq!(codec::value_bytes(&back).unwrap(), bytes);
    bytes
}

fn decoded(bytes: &[u8]) -> Result<Value, String> {
    codec::value_from_bytes(bytes).map_err(|m| m.to_string())
}

/// Every element of F2 and F5 round-trips with one encoding: the tag, the field's byte and the residue's byte. An
/// element is reduced when it is made, so equal elements keep equal bytes, and the two fields never share them.
#[test]
fn elements_round_trip_with_one_encoding() {
    let mut seen = BTreeSet::new();
    for (field, byte) in [(Field::F2, 0), (Field::F5, 1)] {
        for r in 0..field.modulus() {
            let bytes = round_trip(&Value::Field(field, r));
            assert_eq!(bytes, [24, byte, r]);
            assert!(seen.insert(bytes), "{field:?} {r}");
        }
    }
    // 7, -3 and 2 are one element of F5.
    for n in [7, -3, 2] {
        assert_eq!(round_trip(&Value::field(Field::F5, &BigInt::from(n))), [24, 1, 2]);
    }
    // As a set's element and a map's key.
    round_trip(&Value::set(vec![Value::Field(Field::F5, 4), Value::Field(Field::F5, 0)]));
    round_trip(&Value::map(vec![
        (Value::Field(Field::F2, 1), Value::int(1)),
        (Value::Field(Field::F2, 0), Value::Unit),
    ]));
}

/// An element outside its field has no encoding, and bytes outside the form are refused: a field byte other than 0 or
/// 1, a residue at or above the prime, a missing byte or a trailing one.
#[test]
fn elements_outside_their_form_are_refused_on_both_sides() {
    for v in [Value::Field(Field::F2, 2), Value::Field(Field::F5, 5), Value::Field(Field::F5, 255)] {
        assert_eq!(codec::value_bytes(&v), Err(Unencodable("a residue outside its field")), "{v:?}");
    }
    for (bytes, why) in [
        (&[24, 2, 0][..], "an unknown field"),
        (&[24, 255, 0], "an unknown field"),
        (&[24, 0, 2], "a residue outside its field"),
        (&[24, 1, 5], "a residue outside its field"),
        (&[24, 1, 255], "a residue outside its field"),
    ] {
        assert_eq!(decoded(bytes).err().as_deref(), Some(why), "{bytes:?}");
    }
    for bytes in [&[24][..], &[24, 1], &[24, 1, 4, 0]] {
        assert!(decoded(bytes).is_err(), "{bytes:?}");
    }
}

/// `F2` and `F5` round-trip as types with one encoding each, also as a set's element and a map's key, and a field byte
/// other than 0 or 1 is refused.
#[test]
fn field_types_round_trip_with_one_encoding() {
    let q = |ty: Ty| QTy { ty, display: false };
    let types = [
        Ty::Field(Field::F2),
        Ty::Field(Field::F5),
        Ty::Set(Box::new(q(Ty::Field(Field::F5)))),
        Ty::Map(Box::new(q(Ty::Field(Field::F2))), Box::new(q(Ty::Int))),
    ];
    let mut seen = BTreeSet::new();
    for t in &types {
        let mut e = Enc::new();
        codec::encode_ty(t, &mut e).unwrap_or_else(|x| panic!("{t:?}: {x:?}"));
        let mut d = Dec::new(&e.bytes);
        let back = codec::decode_ty(&mut d).unwrap_or_else(|x| panic!("{t:?}: {x}"));
        d.end().unwrap();
        assert_eq!(&back, t);
        assert!(seen.insert(e.bytes), "{t:?} shares an encoding");
    }
    let one = |bytes: &[u8]| codec::decode_ty(&mut Dec::new(bytes)).map_err(|m| m.to_string());
    assert_eq!(one(&[26, 0]), Ok(Ty::Field(Field::F2)));
    assert_eq!(one(&[26, 1]), Ok(Ty::Field(Field::F5)));
    assert_eq!(one(&[26, 2]), Err("an unknown field".to_string()));
    assert!(one(&[26]).is_err());
}

const FIELDS: &str = "\
    export fn quotient(a: Int, b: Int) -> Nat effects [] {\n    \
    return residue(field_mul(to_f5(a), field_inverse(to_f5(b))));\n}\n\n\
    export fn element(a: Int) -> F5 effects [] {\n    return to_f5(a);\n}\n\n\
    export fn given(e: F2) -> Nat effects [] {\n    return residue(e);\n}\n";

/// A program over F5 runs in the child as in process: its charges, its result, the inverse of zero's pointwise
/// failure and the output stop of an element. An element given as an argument reaches the child, which refuses it as
/// the run in process does.
#[test]
fn field_programs_cross_the_process_unchanged() {
    let mut rt = runtime();
    let source = format!("{}{FIELDS}", header(""));
    let run = |rt: &mut Runtime, entry: &str, args: Vec<Value>| {
        compare(rt, &job(&source, entry, args, JobOptions::new(WIDE)), |_: &mut Session| {}).0
    };
    // 7 is 2 in F5 and 3's inverse is 2: their quotient is 4; -1 is 4, so its quotient is 3. A large integer reduces
    // as a small one.
    for (a, want) in [(BigInt::from(7), 4), (BigInt::from(10u32).pow(40u32) + 7u32, 4), (BigInt::from(-1), 3)] {
        let end = run(&mut rt, "quotient", vec![Value::int(a.clone()), Value::int(3)]);
        let got = ran(&end).value.as_ref().expect("a value");
        assert!(codec::same_value(got, &Value::int(want)), "{a}: {got:?}");
    }
    let end = run(&mut rt, "quotient", vec![Value::int(1), Value::int(10)]);
    assert_eq!((ran(&end).summary.outcome, ran(&end).summary.code), (Outcome::Error, Some(POINTWISE_FAILED)));
    let end = run(&mut rt, "element", vec![Value::int(3)]);
    assert_eq!((ran(&end).summary.outcome, ran(&end).summary.code), (Outcome::Error, Some(NONCANONICAL_VALUE)));
    match run(&mut rt, "given", vec![Value::Field(Field::F2, 1)]) {
        Ended::Refused(r) => assert_eq!((r.stage, r.codes), (Stage::Start, vec![NONCANONICAL_VALUE.to_string()])),
        other => panic!("{other:#?}"),
    }
}
