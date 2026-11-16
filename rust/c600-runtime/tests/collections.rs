//! LP2's draft collections cross the process boundary in C3's transfer forms (draft `c600-rt1-wire-1`): lists, word
//! keys, sets and maps round-trip with exactly one encoding each. Lists, sets and maps cross as results, arguments,
//! a host call's type and its answer, a word key as an answer, and the run in the child equals the run in process.
//! A set or map outside the canonical key order, a map entry that is not a `{ key, value }` record and a word key
//! that is not freely reduced are refused on both sides. A compressed word has no transfer form until `c600-run`
//! lends its parts: both executors refuse it as an argument, and as an answer it stops the child's run only (crate
//! draft 15). Source and synthetic evidence only.

mod support;

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_ir::{QTy, Ty};
use c600_run::interp::code::NONCANONICAL_VALUE;
use c600_run::interp::RowTariff;
use c600_run::meter::Outcome;
use c600_run::value::{entry, Compressed, List, Ratio, Value};
use c600_runtime::codec::{self, Unencodable};
use c600_runtime::wire::{Dec, Enc};
use c600_runtime::{run_in_process, Ended, JobOptions, Runtime, Stage};
use support::{compare, header, job, ran, runtime, Session, WIDE};

fn rat(num: i64, den: i64) -> Value {
    Value::Rat(Arc::new(Ratio::new(num.into(), den.into())))
}

fn key(letters: &[i32]) -> Value {
    Value::WordEqualityKey(letters.into())
}

/// Encodes `v`, decodes it and encodes it again: one encoding, and the same value back.
fn round_trip(v: &Value) -> Vec<u8> {
    let bytes = codec::value_bytes(v).unwrap_or_else(|e| panic!("{v:?}: {e:?}"));
    let back = codec::value_from_bytes(&bytes).unwrap_or_else(|e| panic!("{v:?}: {e}"));
    assert_eq!(format!("{v:?}"), format!("{back:?}"));
    assert_eq!(codec::value_bytes(&back).unwrap(), bytes);
    bytes
}

fn decoded(f: impl Fn(&mut Enc)) -> Result<Value, String> {
    let mut e = Enc::new();
    f(&mut e);
    codec::value_from_bytes(&e.bytes).map_err(|m| m.to_string())
}

/// A natural number below 256 as the encoding writes it: the integer tag, a plus sign and one magnitude byte.
fn nat(e: &mut Enc, n: u8) {
    e.u8(2);
    e.u8(1);
    e.bytes(&[n]);
}

fn assert_same(got: &Value, want: &Value) {
    assert!(codec::same_value(got, want), "{got:?}\n/ {want:?}");
}

/// Every collection form round-trips with exactly one encoding; the written order and the repetitions of a set or a
/// map never reach its bytes.
#[test]
fn collections_round_trip_with_one_encoding() {
    let values = vec![
        Value::list(vec![]),
        Value::list(vec![Value::int(2), Value::int(1), Value::int(2)]),
        Value::list(vec![Value::list(vec![Value::word(&[1, -2])]), Value::word(&[])]),
        key(&[]),
        key(&[1, -2, 1200]),
        Value::set(vec![]),
        Value::set(vec![Value::int(2), Value::int(1)]),
        Value::set(vec![Value::text("a"), rat(1, 2), Value::list(vec![])]),
        Value::map(vec![]),
        Value::map(vec![
            (Value::int(2), Value::text("b")),
            (Value::int(1), Value::record(vec![("x", Value::set(vec![Value::int(7)]))])),
        ]),
        Value::map(vec![(Value::text("k"), Value::map(vec![(Value::int(1), key(&[3]))]))]),
    ];
    let mut seen = BTreeSet::new();
    for v in &values {
        assert!(seen.insert(round_trip(v)), "{v:?} shares an encoding");
    }
    // A word and its key, and a list and a set of the same elements, stay apart.
    assert_ne!(round_trip(&Value::word(&[1, 3])), round_trip(&key(&[1, 3])));
    assert_ne!(round_trip(&Value::list(vec![Value::int(1)])), round_trip(&Value::set(vec![Value::int(1)])));

    let three = |items: [i64; 3]| Value::set(items.iter().map(|n| Value::int(*n)).collect());
    assert_eq!(round_trip(&three([3, 1, 3])), round_trip(&Value::set(vec![Value::int(1), Value::int(3)])));
    assert_eq!(round_trip(&three([2, 3, 1])), round_trip(&three([1, 2, 3])));
    let written = Value::map(vec![
        (Value::int(2), Value::text("b")),
        (Value::int(1), Value::text("a")),
        (Value::int(2), Value::text("c")),
    ]);
    let latest = Value::map(vec![(Value::int(1), Value::text("a")), (Value::int(2), Value::text("c"))]);
    assert_eq!(round_trip(&written), round_trip(&latest));
    // Of an `Int` and a `Rat` of one number, the first in the key order stays whichever was written first.
    assert_eq!(round_trip(&Value::set(vec![rat(2, 1), Value::int(2)])), round_trip(&Value::set(vec![Value::int(2)])));
}

/// A set or a map outside the canonical key order, a map entry that is not a `{ key, value }` record, a word key that
/// is not freely reduced and a compressed word are refused when encoded. The same faults written by hand are refused
/// when decoded, and their canonical controls decode.
#[test]
fn collections_outside_their_canonical_form_are_refused_on_both_sides() {
    let set = |items: Vec<Value>| Value::Set(Arc::new(List::new(items)));
    let map = |items: Vec<Value>| Value::Map(Arc::new(List::new(items)));
    let unit = |k: i64| entry(Value::int(k), Value::Unit);
    let order = "a set outside the canonical key order";
    let map_order = "a map outside the canonical key order";
    let not_entry = "a map entry that is not a { key, value } record";
    for (v, why) in [
        (set(vec![Value::int(2), Value::int(1)]), order),
        // An `Int` and a `Rat` of one number are one key.
        (set(vec![Value::int(1), rat(1, 1)]), order),
        (map(vec![unit(2), unit(1)]), map_order),
        (map(vec![unit(1), entry(Value::int(1), Value::Bool(true))]), map_order),
        (map(vec![Value::int(1)]), not_entry),
        (map(vec![Value::record(vec![("value", Value::Unit), ("key", Value::int(1))])]), not_entry),
        (map(vec![Value::record(vec![("key", Value::int(1)), ("value", Value::Unit), ("x", Value::Unit)])]), not_entry),
        (key(&[1, -1]), "a word key that is not freely reduced"),
        (key(&[2, 1201]), "a letter outside 1 to 1200"),
        (key(&[i32::MIN, 1]), "a letter outside 1 to 1200"),
        (
            Value::Compressed(Arc::new(Compressed::leaf(Arc::from(vec![1, -2])))),
            "a compressed word without a transfer form",
        ),
    ] {
        assert_eq!(codec::value_bytes(&v), Err(Unencodable(why)), "{v:?}");
    }

    for (got, why) in [
        (
            decoded(|e| {
                e.u8(18);
                e.count(2);
                nat(e, 2);
                nat(e, 1);
            }),
            order,
        ),
        (
            decoded(|e| {
                e.u8(18);
                e.count(2);
                nat(e, 1);
                nat(e, 1);
            }),
            order,
        ),
        // An `Int` and a `Rat` of one number are one key on the way in too.
        (
            decoded(|e| {
                e.u8(18);
                e.count(2);
                nat(e, 1);
                e.bytes.extend(codec::value_bytes(&rat(1, 1)).unwrap());
            }),
            order,
        ),
        (
            decoded(|e| {
                e.u8(19);
                e.count(2);
                nat(e, 2);
                e.u8(0);
                nat(e, 1);
                e.u8(0);
            }),
            map_order,
        ),
        (
            decoded(|e| {
                e.u8(19);
                e.count(2);
                nat(e, 1);
                e.u8(0);
                nat(e, 1);
                e.u8(1);
                e.u8(1);
            }),
            map_order,
        ),
        (
            decoded(|e| {
                e.u8(17);
                e.count(2);
                e.i32(5);
                e.i32(-5);
            }),
            "a word key that is not freely reduced",
        ),
        (
            decoded(|e| {
                e.u8(17);
                e.count(1);
                e.i32(0);
            }),
            "a letter outside 1 to 1200",
        ),
        // An element takes at least its tag, and a map entry two bytes, the tags of its key and its value.
        (
            decoded(|e| {
                e.u8(16);
                e.count(usize::MAX);
            }),
            "a length beyond the payload",
        ),
        (
            decoded(|e| {
                e.u8(18);
                e.count(1);
            }),
            "a length beyond the payload",
        ),
        (
            decoded(|e| {
                e.u8(19);
                e.count(1);
                e.u8(0);
            }),
            "a length beyond the payload",
        ),
        (
            decoded(|e| {
                e.u8(19);
                e.count(2);
                e.u8(0);
                e.u8(0);
                e.u8(0);
            }),
            "a length beyond the payload",
        ),
        // The first tag after a map's: a compressed word's form will take it.
        (decoded(|e| e.u8(20)), "an unknown value tag"),
    ] {
        assert_eq!(got.err().as_deref(), Some(why));
    }

    // The canonical controls of the same layouts decode: the refusals are about the form, not the layout.
    let both = Value::set(vec![Value::int(1), Value::int(2)]);
    assert_same(
        &decoded(|e| {
            e.u8(18);
            e.count(2);
            nat(e, 1);
            nat(e, 2);
        })
        .unwrap(),
        &both,
    );
    assert_same(
        &decoded(|e| {
            e.u8(19);
            e.count(2);
            nat(e, 1);
            e.u8(0);
            nat(e, 2);
            e.u8(0);
        })
        .unwrap(),
        &Value::map(vec![(Value::int(1), Value::Unit), (Value::int(2), Value::Unit)]),
    );
    assert_same(
        &decoded(|e| {
            e.u8(17);
            e.count(3);
            e.i32(5);
            e.i32(5);
            e.i32(-6);
        })
        .unwrap(),
        &key(&[5, 5, -6]),
    );
    for (tag, empty) in [(16, Value::list(vec![])), (18, Value::set(vec![])), (19, Value::map(vec![]))] {
        let got = decoded(|e| {
            e.u8(tag);
            e.count(0);
        });
        assert_same(&got.unwrap(), &empty);
    }
}

/// Sets and maps nested far deeper than any stack allows decode, encode again and drop without recursion, as lists do
/// (`tests/protocol.rs`).
#[test]
fn deep_sets_and_maps_cross_without_recursion() {
    const DEPTH: usize = 200_000;
    for tag in [18, 19] {
        let mut e = Enc::new();
        for _ in 0..DEPTH {
            e.u8(tag);
            e.count(1);
            if tag == 19 {
                // The key; the next level is the value.
                e.u8(0);
            }
        }
        e.u8(0);
        let value = codec::value_from_bytes(&e.bytes).unwrap_or_else(|m| panic!("tag {tag}: {m}"));
        assert_eq!(codec::value_bytes(&value).unwrap(), e.bytes);
        drop(value);
    }
}

/// The collection types round-trip with one encoding each, a compressed word's among them, and their nesting counts
/// towards the transfer depth bound.
#[test]
fn collection_types_round_trip_under_the_depth_bound() {
    let q = |ty: Ty| QTy { ty, display: false };
    let types = [
        Ty::List(Box::new(q(Ty::Nat))),
        Ty::WordEqualityKey,
        Ty::Set(Box::new(q(Ty::Rat))),
        Ty::Set(Box::new(QTy { ty: Ty::Rat, display: true })),
        Ty::Map(Box::new(q(Ty::Text)), Box::new(q(Ty::Set(Box::new(q(Ty::WordEqualityKey)))))),
        Ty::Map(Box::new(q(Ty::Set(Box::new(q(Ty::WordEqualityKey))))), Box::new(q(Ty::Text))),
        Ty::CompressedWord,
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
    let nested = |depth: usize| (0..depth).fold(Ty::Nat, |t, _| Ty::Set(Box::new(q(t))));
    assert!(codec::encode_ty(&nested(codec::TYPE_DEPTH), &mut Enc::new()).is_ok());
    assert!(codec::encode_ty(&nested(codec::TYPE_DEPTH + 1), &mut Enc::new()).is_err());
    // The same nesting written by hand: the set tags, the element's, then each level's display flag.
    let written = |depth: usize| {
        let mut e = Enc::new();
        (0..depth).for_each(|_| e.u8(20));
        e.u8(2);
        (0..depth).for_each(|_| e.bool(false));
        let mut d = Dec::new(&e.bytes);
        let t = codec::decode_ty(&mut d).map_err(|m| m.to_string())?;
        d.end().map_err(|m| m.to_string())?;
        Ok::<Ty, String>(t)
    };
    assert_eq!(written(codec::TYPE_DEPTH), Ok(nested(codec::TYPE_DEPTH)));
    assert!(written(codec::TYPE_DEPTH + 1).is_err(), "a set type nested beyond the bound");
    // The first type tag after a compressed word's.
    assert!(codec::decode_ty(&mut Dec::new(&[23])).is_err());
}

const COLLECTIONS: &str = "\
    export fn made() -> record { l: List<Nat>, s: Set<Rat>, m: Map<Nat, Text> } effects [] {\n    \
    return record { l = list<Nat>(1, 2), s = set<Rat>(4/2, 2, 1/2), \
    m = map<Nat, Text>(2, \"b\", 1, \"a\", 2, \"c\") };\n}\n\n\
    export fn echo(s: Set<Nat>, m: Map<Nat, Text>) -> record { s: Set<Nat>, m: Map<Nat, Text> } effects [] {\n    \
    return record { s = insert(s, 0), m = insert(m, 0, \"z\") };\n}\n\n\
    export fn read() -> Set<Nat> effects [snapshot] {\n    return read_snapshot<Set<Nat>>(\"set\");\n}\n\n\
    export fn keyed(w: Word) -> Bool effects [snapshot] {\n    \
    return exact_eq(read_snapshot<WordEqualityKey>(\"key\"), word_key(w));\n}\n\n\
    export fn given(k: WordEqualityKey) -> Bool effects [] {\n    return exact_eq(k, k);\n}\n\n\
    export fn size() -> Nat effects [snapshot] {\n    \
    return length(read_snapshot<CompressedWord>(\"compressed\"));\n}\n\n\
    export fn sized(c: CompressedWord) -> Nat effects [] {\n    return length(c);\n}\n";

fn field<'a>(v: &'a Value, name: &str) -> &'a Value {
    let Value::Record(r) = v else { panic!("a record: {v:?}") };
    r.fields().iter().find(|(n, _)| n.as_str() == name).map(|(_, v)| v).unwrap_or_else(|| panic!("{name}: {v:?}"))
}

/// The collections cross the process in both directions: as a result, as arguments, as a snapshot read's type and
/// as its answer. Each end equals the run in process, and each value the one built here. A word key has no output
/// form (LG2.13), so neither executor takes it as an argument or a result: it crosses as a host's answer and comes
/// back as a comparison.
#[test]
fn collections_cross_the_process_unchanged() {
    let mut rt = runtime();
    let source = format!("{}{COLLECTIONS}", header(""));
    let tariff = RowTariff { kernel_cost: 5, result_bound: 4096 };
    let run = |rt: &mut Runtime, entry: &str, args: Vec<Value>| {
        compare(rt, &job(&source, entry, args, JobOptions::new(WIDE)), |s: &mut Session| {
            s.snapshots.push(("set", tariff, Value::set(vec![Value::int(3), Value::int(1)])));
            s.snapshots.push(("key", tariff, key(&[1, 3])));
        })
    };
    let refused_at_start = |end: Ended| match end {
        Ended::Refused(r) => assert_eq!((r.stage, r.codes), (Stage::Start, vec![NONCANONICAL_VALUE.to_string()])),
        other => panic!("{other:#?}"),
    };

    let (end, _) = run(&mut rt, "made", vec![]);
    let made = ran(&end).value.as_ref().expect("a value");
    assert_same(field(made, "l"), &Value::list(vec![Value::int(1), Value::int(2)]));
    assert!(matches!(field(made, "s"), Value::Set(s) if s.items().len() == 2), "{made:?}");
    assert_same(
        field(made, "m"),
        &Value::map(vec![(Value::int(1), Value::text("a")), (Value::int(2), Value::text("c"))]),
    );

    let args =
        vec![Value::set(vec![Value::int(2), Value::int(1)]), Value::map(vec![(Value::int(1), Value::text("a"))])];
    let (end, _) = run(&mut rt, "echo", args);
    let echoed = ran(&end).value.as_ref().expect("a value");
    assert_same(field(echoed, "s"), &Value::set(vec![Value::int(0), Value::int(1), Value::int(2)]));
    assert_same(
        field(echoed, "m"),
        &Value::map(vec![(Value::int(0), Value::text("z")), (Value::int(1), Value::text("a"))]),
    );

    let (end, session) = run(&mut rt, "read", vec![]);
    assert_same(ran(&end).value.as_ref().expect("a value"), &Value::set(vec![Value::int(1), Value::int(3)]));
    assert_eq!(session.log.len(), 2, "one tariff and one read");

    // The key [1, 3] crosses in the answer: the word [1, -2, 2, 3] reduces to it and [1, 2] does not.
    for (letters, same) in [(&[1, -2, 2, 3][..], true), (&[1, 2][..], false)] {
        let (end, _) = run(&mut rt, "keyed", vec![Value::word(letters)]);
        assert_same(ran(&end).value.as_ref().expect("a value"), &Value::Bool(same));
    }
    // As an argument it now reaches the child, which refuses it as the run in process does, with the same detail.
    let (end, _) = run(&mut rt, "given", vec![key(&[1, 3])]);
    refused_at_start(end);

    // Not yet: a compressed word has no transfer form. As an argument both executors refuse it at the start, the
    // parent before anything is sent (with its own detail).
    let compressed = Value::Compressed(Arc::new(Compressed::leaf(Arc::from(vec![1, -2]))));
    let sized = job(&source, "sized", vec![compressed.clone()], JobOptions::new(WIDE));
    refused_at_start(run_in_process(&sized, &mut Session::default(), None));
    refused_at_start(rt.run(&sized, &mut Session::default(), None).unwrap_or_else(|f| panic!("{f}")));
    // As a host's answer it reaches the run in process, while the child's run stops at the read: a known
    // difference (crate draft 15).
    let size = job(&source, "size", vec![], JobOptions::new(WIDE));
    let session = || {
        let mut s = Session::default();
        s.snapshots.push(("compressed", tariff, compressed.clone()));
        s
    };
    let here = run_in_process(&size, &mut session(), None);
    assert_same(ran(&here).value.as_ref().expect("a value"), &Value::int(2));
    let there = rt.run(&size, &mut session(), None).unwrap_or_else(|f| panic!("{f}"));
    let summary = &ran(&there).summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(NONCANONICAL_VALUE)));
}
