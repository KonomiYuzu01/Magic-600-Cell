//! LP2's draft collections cross the process boundary in C3's transfer forms (draft `c600-rt1-wire-1`): lists, word
//! keys, sets and maps round-trip with exactly one encoding each. Lists, sets and maps cross as results, arguments,
//! a host call's type and its answer, a word key as an answer, and the run in the child equals the run in process.
//! A set or map outside the canonical key order, a map entry that is not a `{ key, value }` record and a word key
//! that is not freely reduced are refused on both sides. A compressed word crosses as its parts (crate draft 15): it
//! round-trips with one encoding, and as an answer the run in the child equals the run in process. A part list that
//! `from_parts` refuses and compressed words above the bound on a payload's derived lengths are refused on both sides;
//! above that bound, an answer stops the child's run only. Source and synthetic evidence only.

mod support;

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_ir::{QTy, Ty};
use c600_run::interp::code::NONCANONICAL_VALUE;
use c600_run::interp::RowTariff;
use c600_run::meter::Outcome;
use c600_run::value::{entry, Compressed, List, Part, Ratio, Value};
use c600_runtime::codec::{self, Unencodable, COMPRESSED_LENGTH_BITS};
use c600_runtime::wire::{Dec, Enc};
use c600_runtime::{run_in_process, Ended, JobOptions, Runtime, Stage};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
use support::{compare, header, job, ran, runtime, Session, WIDE};

fn rat(num: i64, den: i64) -> Value {
    Value::Rat(Arc::new(Ratio::new(num.into(), den.into())))
}

fn key(letters: &[i32]) -> Value {
    Value::WordEqualityKey(letters.into())
}

/// The word `letters` as a compressed word of one part.
fn leaf_word(letters: &[i32]) -> Arc<Compressed> {
    Arc::new(Compressed::leaf(letters.into()))
}

/// The letter 1 doubled `n` times: a leaf, then each part joined to itself. By the codec's bound the derived lengths
/// take (n + 1)(n + 2)/2 bits together, so 16,382 doublings are the most that cross.
fn doublings(n: usize) -> Arc<Compressed> {
    let mut parts = vec![Part::Leaf(Arc::from(vec![1]))];
    parts.extend((0..n).map(|k| Part::Concat(k, k)));
    Arc::new(Compressed::from_parts(parts).expect("a straight-line program"))
}

/// A compressed word's leaf as the encoding writes it.
fn leaf(e: &mut Enc, letters: &[i32]) {
    e.u8(0);
    e.count(letters.len());
    for l in letters {
        e.i32(*l);
    }
}

/// A compressed word's concatenation of parts `i` and `j` as the encoding writes it.
fn concat(e: &mut Enc, i: u64, j: u64) {
    e.u8(1);
    e.u64(i);
    e.u64(j);
}

/// The compressed word of `doublings(n)` as the encoding writes it.
fn chain(e: &mut Enc, n: u64) {
    e.u8(20);
    e.count(n as usize + 1);
    leaf(e, &[1]);
    for k in 0..n {
        concat(e, k, k);
    }
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

/// A compressed word crosses as its parts (crate draft 15). Each shape that `concat` builds, and one that it never
/// builds, round-trips with one encoding and comes back with the same parts, length, height, logical size and
/// letters. Compressed words compare by their parts, so two words of one expansion keep apart and two words built
/// alike share an encoding. The letter 1 doubled 16,382 times crosses; doubled once more, it is refused when encoded
/// and when decoded. The bound is shared by a payload's words: two words of 11,583 doublings cross together, and two
/// of 11,584 are refused.
#[test]
fn compressed_words_cross_as_their_parts() {
    let bytes = |c: &Arc<Compressed>| codec::value_bytes(&Value::Compressed(c.clone()));
    let back = |c: &Arc<Compressed>| {
        let written = bytes(c).unwrap_or_else(|e| panic!("{e:?}"));
        let Ok(Value::Compressed(got)) = codec::value_from_bytes(&written) else { panic!("a compressed word") };
        assert_eq!(bytes(&got).unwrap(), written);
        assert_eq!(got.part_list(), c.part_list());
        assert_eq!((got.length(), got.height(), got.size()), (c.length(), c.height(), c.size()));
        got
    };

    let a = leaf_word(&[1, -2]);
    let b = leaf_word(&[3]);
    let ab = Compressed::concat(&a, &b);
    let words = vec![
        leaf_word(&[]),
        a.clone(),
        b.clone(),
        ab.clone(),
        // A word joined to itself adds one part.
        Compressed::concat(&a, &a),
        Compressed::concat(&ab, &ab),
        Compressed::concat(&ab, &a),
        Compressed::concat(&Compressed::concat(&ab, &b), &ab),
        // A shape `concat` never builds: the later part first.
        Arc::new(
            Compressed::from_parts(vec![
                Part::Leaf(Arc::from(vec![1, -2])),
                Part::Leaf(Arc::from(vec![3])),
                Part::Concat(1, 0),
            ])
            .unwrap(),
        ),
    ];
    let mut seen = BTreeSet::new();
    for w in &words {
        let got = back(w);
        let length = w.length().to_usize().expect("a short word");
        assert_eq!(got.expand(&BigInt::zero(), length), w.expand(&BigInt::zero(), length));
        assert!(seen.insert(bytes(w).unwrap()), "{w:?} shares an encoding");
    }
    // `a` joined to another word with its parts is `a` joined to itself.
    assert_eq!(bytes(&Compressed::concat(&a, &leaf_word(&[1, -2]))), bytes(&Compressed::concat(&a, &a)));
    // One expansion with other parts is another value with another encoding.
    let flat = Value::Compressed(leaf_word(&[1, -2, 3]));
    let built = Value::Compressed(ab.clone());
    assert!(!c600_run::value::equal(&flat, &built));
    assert_ne!(codec::value_bytes(&flat), codec::value_bytes(&built));
    let written = decoded(|e| {
        e.u8(20);
        e.count(3);
        leaf(e, &[1, -2]);
        leaf(e, &[3]);
        concat(e, 0, 1);
    });
    assert_same(&written.unwrap(), &built);

    // The bound: n doublings of one letter derive (n + 1)(n + 2)/2 bits by the codec's count.
    let derived = |n: u64| (n + 1) * (n + 2) / 2;
    assert!(derived(16_382) <= COMPRESSED_LENGTH_BITS && derived(16_383) > COMPRESSED_LENGTH_BITS);
    let most = doublings(16_382);
    let got = back(&most);
    assert_eq!((got.length(), got.height()), (&(BigInt::one() << 16_382u32), 16_382));
    assert_same(&decoded(|e| chain(e, 16_382)).unwrap(), &Value::Compressed(most));
    let why = "compressed words above the transfer bound";
    assert_eq!(bytes(&doublings(16_383)), Err(Unencodable(why)));
    assert_eq!(decoded(|e| chain(e, 16_383)).err().as_deref(), Some(why));

    // The bound holds for a payload, not for each word: a list of two words of 11,583 doublings crosses, and one of
    // two words of 11,584, each within the bound alone, is refused when encoded and when decoded.
    assert!(2 * derived(11_583) <= COMPRESSED_LENGTH_BITS && 2 * derived(11_584) > COMPRESSED_LENGTH_BITS);
    let twice = |n: usize| Value::List(Arc::new(List::new(vec![Value::Compressed(doublings(n)); 2])));
    let two = twice(11_583);
    let written = codec::value_bytes(&two).unwrap_or_else(|e| panic!("{e:?}"));
    assert_same(&codec::value_from_bytes(&written).unwrap(), &two);
    assert_eq!(codec::value_bytes(&twice(11_584)), Err(Unencodable(why)));
    let pair = |n: u64| {
        decoded(move |e| {
            e.u8(16);
            e.count(2);
            chain(e, n);
            chain(e, n);
        })
    };
    assert_same(&pair(11_583).unwrap(), &two);
    assert_eq!(pair(11_584).err().as_deref(), Some(why));
}

/// A set or a map outside the canonical key order, a map entry that is not a `{ key, value }` record, a word key that
/// is not freely reduced and a compressed word with a letter out of range are refused when encoded. The same faults
/// written by hand are refused when decoded, and so is a compressed word's part list that `from_parts` refuses. Their
/// canonical controls decode.
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
        // `leaf` checks no letters.
        (Value::Compressed(leaf_word(&[1, 1201])), "a letter outside 1 to 1200"),
        (Value::Compressed(Compressed::concat(&leaf_word(&[2]), &leaf_word(&[0]))), "a letter outside 1 to 1200"),
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
        // A compressed word: a part list that `from_parts` refuses, an unknown part tag, and a part count or a
        // letter count that the payload cannot hold. A part takes at least nine bytes.
        (
            decoded(|e| {
                e.u8(20);
                e.count(0);
            }),
            "a compressed word without parts",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(1);
                e.u64(1);
            }),
            "a length beyond the payload",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(1);
                e.u8(2);
                e.u64(0);
            }),
            "an unknown part tag",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(1);
                e.u8(0);
                e.count(3);
                e.i32(1);
            }),
            "a length beyond the payload",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(1);
                leaf(e, &[1, -1201]);
            }),
            "a letter outside 1 to 1200",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(2);
                leaf(e, &[1]);
                concat(e, 0, 1);
            }),
            "a concatenation of a part not before it",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(2);
                leaf(e, &[1]);
                concat(e, u64::MAX, 0);
            }),
            "a concatenation of a part not before it",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(3);
                leaf(e, &[]);
                leaf(e, &[1]);
                concat(e, 0, 1);
            }),
            "a concatenation of an empty part",
        ),
        (
            decoded(|e| {
                e.u8(20);
                e.count(2);
                leaf(e, &[1]);
                leaf(e, &[2]);
            }),
            "a part the word does not reach",
        ),
        // The first tag after an F2 or F5 element's (LP2 draft, KN3.2), which follows a trace key's (LB1.10).
        (decoded(|e| e.u8(25)), "an unknown value tag"),
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
    // The first type tag after a generator's (LP2 draft, DET.61), which follows an F2 or F5 element's (KN3.2).
    assert!(codec::decode_ty(&mut Dec::new(&[28])).is_err());
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
/// back as a comparison. A compressed word has none either: it crosses as an answer and comes back as its length,
/// except above the bound on its derived lengths, where the child's run stops at the read (crate draft 15).
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

    // A compressed word crosses as its parts (crate draft 15). As an argument it now reaches the child, which refuses
    // it as the run in process does, with the same detail: it has no output form.
    let compressed = Value::Compressed(leaf_word(&[1, -2]));
    let (end, _) = run(&mut rt, "sized", vec![compressed.clone()]);
    refused_at_start(end);
    // As a host's answer it reaches both runs, which end alike.
    let size = job(&source, "size", vec![], JobOptions::new(WIDE));
    let (end, _) =
        compare(&mut rt, &size, |s: &mut Session| s.snapshots.push(("compressed", tariff, compressed.clone())));
    assert_same(ran(&end).value.as_ref().expect("a value"), &Value::int(2));
    // Above the bound on its derived lengths, the answer reaches the run in process, while the parent answers the
    // child's call with `PS1:input.noncanonical_value`: a known difference (crate draft 15).
    let over = Value::Compressed(doublings(16_383));
    // The result bound must cover the word's logical size, which counts every part's stored length (LP2's pull request
    // 186), about 17 MB here. One as large as `WIDE`'s memory budget would cut the read off before its answer.
    let wide = RowTariff { kernel_cost: 5, result_bound: 1 << 30 };
    let session = || {
        let mut s = Session::default();
        s.snapshots.push(("compressed", wide, over.clone()));
        s
    };
    let here = run_in_process(&size, &mut session(), None);
    assert_same(ran(&here).value.as_ref().expect("a value"), &Value::Int(Arc::new(BigInt::one() << 16_383u32)));
    let there = rt.run(&size, &mut session(), None).unwrap_or_else(|f| panic!("{f}"));
    let summary = &ran(&there).summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(NONCANONICAL_VALUE)));
}
