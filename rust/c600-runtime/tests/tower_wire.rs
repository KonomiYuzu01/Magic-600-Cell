//! The identity tower (T1 drafts until the IR freeze, PHS.49; T1c's wire half) crosses the process boundary in the
//! transfer forms of draft `c600-rt1-wire-1`, appended after the earlier tags and renumbering none: value tags 28 to 34
//! (an ID, a pose-table index, a digest, a cost, a sum value, a certificate, a handle), type tags 32 to 48 and request
//! tag 7 (a kernel row). Every form round-trips with exactly one encoding, and a form that is no tower value is
//! refused on both sides: a negative cost, a constructor its sum type does not have, an index outside its declared
//! order, a digest that is not lowercase hex bytes, a certificate whose bound or puzzle does not fit its goal and a key
//! on a handle that is no pose. A handle crosses with its id so that the run hands it back to the host that made it;
//! the run never publishes, stores, keys or compares it by that id (`c600-run/tests/tower.rs`). Source and synthetic
//! evidence only.

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_ir::{CostUnit, Goal, KernelOp, Level, Puzzle, QTy, Ty};
use c600_run::interp::Request;
use c600_run::tower::{Digest, Handle, HandleTy, Holds, IdKind, SumTy};
use c600_run::value::Value;
use c600_runtime::codec;
use c600_runtime::wire::{Dec, Enc};
use num_bigint::BigInt;

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

fn digest(puzzle: Puzzle, level: Level, hex: &str) -> Digest {
    Digest::new(puzzle, level, hex).expect("lowercase hex bytes")
}

fn handle(ty: HandleTy, id: u64) -> Value {
    Value::handle(Handle::new(ty, id, 64))
}

/// One value of every tower form: every ID kind on both puzzles, digests, costs of every unit, certificates with and
/// without a bound, every handle type, a keyed pose, and every constructor of every built-in sum type and of a
/// declared one.
fn tower_values() -> Vec<Value> {
    let mut values = Vec::new();
    for kind in IdKind::ALL {
        for puzzle in Puzzle::ALL {
            values.push(Value::id(kind, puzzle, 7));
        }
    }
    values.push(Value::id(IdKind::Sticker, Puzzle::Full, u32::MAX));
    values.extend([Value::PoseTableIndex(0), Value::PoseTableIndex(u32::MAX)]);
    values.push(Value::digest(Puzzle::Full, Level::Labels, "00ff").unwrap());
    values.push(Value::digest(Puzzle::Jumbling, Level::Poses, "ab").unwrap());
    for unit in CostUnit::ALL {
        values.push(Value::cost(Puzzle::Jumbling, unit, 0).unwrap());
        values.push(Value::cost(Puzzle::Full, unit, BigInt::from(1u8) << 200).unwrap());
    }
    values.push(Value::holds(Holds::new(Goal::StickerTest, digest(Puzzle::Full, Level::Labels, "aa"), None).unwrap()));
    let low = Holds::new(Goal::LowHeight, digest(Puzzle::Jumbling, Level::Poses, "bb"), Some(5.into())).unwrap();
    values.push(Value::holds(low));
    let zero = Holds::new(Goal::LowHeight, digest(Puzzle::Jumbling, Level::Poses, "bb"), Some(0.into())).unwrap();
    values.push(Value::holds(zero));
    for ty in [
        HandleTy::State(Puzzle::Full),
        HandleTy::State(Puzzle::Jumbling),
        HandleTy::LevelValue(Puzzle::Jumbling, Level::Coset),
        HandleTy::PoseTable,
        HandleTy::Evidence,
        HandleTy::Pose,
    ] {
        values.push(handle(ty, 3));
    }
    values.push(Value::handle(Handle::new(HandleTy::Pose, 4, 16).with_key("pose/12").unwrap()));
    let sums = [
        SumTy::Verdict(Puzzle::Full, Level::Labels, None),
        SumTy::Verdict(Puzzle::Jumbling, Level::Poses, Some(Goal::LowHeight)),
        SumTy::Partial,
        SumTy::Reachability(Puzzle::Jumbling),
        SumTy::Derivation(Puzzle::Full),
    ];
    for ty in sums {
        for (ctor, n) in ty.builtin().expect("a built-in sum type") {
            values.push(Value::sum(ty.clone(), ctor, (0..n).map(|k| Value::int(k as i64)).collect()));
        }
    }
    let shape = SumTy::Named { module: "m".into(), name: "Shape".into() };
    values.push(Value::sum(shape.clone(), "circle", vec![Value::int(1)]));
    values.push(Value::sum(shape, "point", vec![]));
    values
}

/// Every tower value round-trips with exactly one encoding under its own appended tag.
#[test]
fn tower_values_round_trip_with_one_encoding() {
    let mut seen = BTreeSet::new();
    let mut tags = BTreeSet::new();
    for v in &tower_values() {
        let bytes = round_trip(v);
        tags.insert(bytes[0]);
        assert!(seen.insert(bytes), "{v:?} shares an encoding");
    }
    assert_eq!(tags.into_iter().collect::<Vec<_>>(), (28..=34).collect::<Vec<u8>>());
    // A sum's payloads nest like a record's: a partial result of a partial result of a list crosses whole.
    let inner = Value::sum(SumTy::Partial, "total", vec![Value::list(vec![Value::int(1), Value::text("a")])]);
    round_trip(&Value::sum(SumTy::Partial, "total", vec![inner]));
    // A certificate's bound is part of its value: two bounds, two encodings.
    let at = |b: i64| {
        let h = Holds::new(Goal::LowHeight, digest(Puzzle::Jumbling, Level::Poses, "bb"), Some(b.into())).unwrap();
        round_trip(&Value::holds(h))
    };
    assert_ne!(at(1), at(2));
}

/// A value outside the tower's forms is refused when encoded and when decoded.
#[test]
fn malformed_tower_values_are_refused_on_both_sides() {
    // Encoding: a negative cost, a constructor its sum type does not have, a payload count its constructor does not
    // take, and a declared sum value with an empty constructor.
    let unencodable = [
        Value::Cost(Puzzle::Full, CostUnit::Generator, Arc::new(BigInt::from(-1))),
        Value::sum(SumTy::Partial, "maybe", vec![]),
        Value::sum(SumTy::Verdict(Puzzle::Full, Level::Labels, None), "yes", vec![]),
        Value::sum(SumTy::Named { module: "m".into(), name: "Shape".into() }, "", vec![]),
    ];
    for v in &unencodable {
        assert!(codec::value_bytes(v).is_err(), "{v:?}");
    }

    let natural = |e: &mut Enc, n: u8| {
        e.u8(1);
        e.bytes(&[n]);
    };
    let holds = |goal: u8, puzzle: u8, flag: u8| {
        decoded(move |e| {
            e.u8(33);
            e.u8(goal);
            e.u8(puzzle);
            e.u8(0);
            e.text("aa");
            e.u8(flag);
            if flag == 1 {
                natural(e, 5);
            }
        })
    };
    let low = Goal::ALL.iter().position(|g| *g == Goal::LowHeight).unwrap() as u8;
    let exact = Goal::ALL.iter().position(|g| *g == Goal::ExactPose).unwrap() as u8;
    let sticker = Goal::ALL.iter().position(|g| *g == Goal::StickerTest).unwrap() as u8;
    for (got, why) in [
        (
            decoded(|e| {
                e.u8(28);
                e.u8(3);
                e.u8(0);
                e.u32(1);
            }),
            "an unknown ID kind",
        ),
        (
            decoded(|e| {
                e.u8(28);
                e.u8(0);
                e.u8(2);
                e.u32(1);
            }),
            "an unknown puzzle",
        ),
        (
            decoded(|e| {
                e.u8(30);
                e.u8(0);
                e.u8(8);
                e.text("aa");
            }),
            "an unknown level",
        ),
        (
            decoded(|e| {
                e.u8(30);
                e.u8(0);
                e.u8(0);
                e.text("AA");
            }),
            "a digest that is not lowercase hex bytes",
        ),
        (
            decoded(|e| {
                e.u8(30);
                e.u8(0);
                e.u8(0);
                e.text("abc");
            }),
            "a digest that is not lowercase hex bytes",
        ),
        (
            decoded(|e| {
                e.u8(30);
                e.u8(0);
                e.u8(0);
                e.text("");
            }),
            "a digest that is not lowercase hex bytes",
        ),
        (
            decoded(|e| {
                e.u8(31);
                e.u8(0);
                e.u8(3);
                natural(e, 1);
            }),
            "an unknown cost unit",
        ),
        (
            decoded(|e| {
                e.u8(31);
                e.u8(0);
                e.u8(0);
                e.u8(2);
                e.bytes(&[1]);
            }),
            "a negative count or bound",
        ),
        (holds(6, 1, 0), "an unknown goal"),
        (holds(sticker, 1, 1), "a certificate whose bound or puzzle does not fit its goal"),
        (holds(low, 1, 0), "a certificate whose bound or puzzle does not fit its goal"),
        (holds(exact, 0, 0), "a certificate whose bound or puzzle does not fit its goal"),
        (holds(sticker, 0, 2), "a bound's flag other than 0 or 1"),
        (
            decoded(|e| {
                e.u8(34);
                e.u8(5);
            }),
            "an unknown handle type",
        ),
        (
            decoded(|e| {
                e.u8(34);
                e.u8(0);
                e.u8(0);
                e.u64(1);
                e.u128(64);
                e.u8(1);
                e.text("pose/1");
            }),
            "a key on a handle that is no pose",
        ),
        (
            decoded(|e| {
                e.u8(34);
                e.u8(4);
                e.u64(1);
                e.u128(64);
                e.u8(1);
                e.text("");
            }),
            "a key on a handle that is no pose",
        ),
        (
            decoded(|e| {
                e.u8(34);
                e.u8(4);
                e.u64(1);
                e.u128(64);
                e.u8(2);
            }),
            "a key's flag other than 0 or 1",
        ),
        (
            decoded(|e| {
                e.u8(32);
                e.u8(5);
            }),
            "an unknown sum type",
        ),
        (
            decoded(|e| {
                e.u8(32);
                e.u8(1);
                e.text("maybe");
                e.count(0);
            }),
            "a constructor that its sum type does not have",
        ),
        (
            decoded(|e| {
                e.u8(32);
                e.u8(1);
                e.text("total");
                e.count(0);
            }),
            "a constructor that its sum type does not have",
        ),
    ] {
        assert_eq!(got.err().as_deref(), Some(why));
    }

    // The canonical controls of the same layouts decode.
    assert!(holds(sticker, 0, 0).is_ok());
    assert!(holds(low, 1, 1).is_ok());
    let total = decoded(|e| {
        e.u8(32);
        e.u8(1);
        e.text("total");
        e.count(1);
        e.u8(0);
    });
    assert!(codec::same_value(&total.unwrap(), &Value::sum(SumTy::Partial, "total", vec![Value::Unit])));
}

/// Every tower type round-trips with one encoding under its appended tag, and a partial result's payload type nests
/// under the transfer depth bound.
#[test]
fn tower_types_round_trip_with_one_encoding() {
    let q = |ty: Ty| QTy { ty, display: false };
    let mut types = Vec::new();
    for p in Puzzle::ALL {
        types.extend([
            Ty::Piece(p),
            Ty::Sticker(p),
            Ty::Slot(p),
            Ty::State(p),
            Ty::Reachability(p),
            Ty::Derivation(p),
            Ty::Digest(p, Level::Shape),
            Ty::LevelValue(p, Level::Position),
            Ty::Verdict(p, Level::Labels, None),
        ]);
        types.extend(Goal::ALL.iter().map(|g| Ty::Holds(p, *g)));
        types.extend(Goal::ALL.iter().map(|g| Ty::Verdict(p, Level::Poses, Some(*g))));
        types.extend(CostUnit::ALL.iter().map(|u| Ty::Cost(p, *u)));
    }
    types.extend([
        Ty::Pose,
        Ty::PoseTableIndex,
        Ty::PoseTable,
        Ty::Evidence,
        Ty::Partial(Box::new(q(Ty::Nat))),
        Ty::Partial(Box::new(QTy { ty: Ty::Partial(Box::new(q(Ty::Piece(Puzzle::Full)))), display: true })),
        Ty::Named { module: "m".into(), name: "Shape".into() },
    ]);
    let mut seen = BTreeSet::new();
    let mut tags = BTreeSet::new();
    for t in &types {
        let mut e = Enc::new();
        codec::encode_ty(t, &mut e).unwrap_or_else(|x| panic!("{t:?}: {x:?}"));
        let mut d = Dec::new(&e.bytes);
        let back = codec::decode_ty(&mut d).unwrap_or_else(|x| panic!("{t:?}: {x}"));
        d.end().unwrap();
        assert_eq!(&back, t);
        tags.insert(e.bytes[0]);
        assert!(seen.insert(e.bytes), "{t:?} shares an encoding");
    }
    assert_eq!(tags.into_iter().collect::<Vec<_>>(), (32..=48).collect::<Vec<u8>>());
    let nested = |depth: usize| (0..depth).fold(Ty::Nat, |t, _| Ty::Partial(Box::new(q(t))));
    assert!(codec::encode_ty(&nested(codec::TYPE_DEPTH), &mut Enc::new()).is_ok());
    assert!(codec::encode_ty(&nested(codec::TYPE_DEPTH + 1), &mut Enc::new()).is_err());
    // An index outside its declared order is refused.
    for bytes in [&[32, 2][..], &[37, 0, 8], &[45, 0, 6], &[46, 0, 3], &[43, 0, 0, 2]] {
        assert!(codec::decode_ty(&mut Dec::new(bytes)).is_err(), "{bytes:?}");
    }
}

/// A kernel row's request round-trips for every row with its arguments and its declared result type; a cut request
/// and an unknown row are refused. The request carries the row, the argument values and the type, nothing else.
#[test]
fn kernel_requests_round_trip_and_an_unknown_row_is_refused() {
    let qty = QTy { ty: Ty::Digest(Puzzle::Jumbling, Level::Poses), display: false };
    let state = handle(HandleTy::State(Puzzle::Jumbling), 9);
    let mut ops = vec![KernelOp::Snapshot(Puzzle::Full), KernelOp::Snapshot(Puzzle::Jumbling)];
    ops.extend(Level::ALL.iter().map(|l| KernelOp::Project(*l)));
    ops.extend(Level::ALL.iter().map(|l| KernelOp::Compare(*l)));
    ops.extend([KernelOp::Digest, KernelOp::Derive, KernelOp::Import, KernelOp::Reachability]);
    ops.extend(Goal::ALL.iter().map(|g| KernelOp::Check(*g)));
    let args = [state.clone(), Value::word(&[1, -2]), Value::int(4)];
    let mut seen = BTreeSet::new();
    for op in ops {
        let r = Request::Kernel { op, args: &args, ty: &qty };
        let mut e = Enc::new();
        codec::encode_request(&r, &mut e).unwrap();
        assert_eq!(e.bytes[0], 7, "{op:?}");
        let mut d = Dec::new(&e.bytes);
        let owned = codec::decode_request(&mut d).unwrap();
        d.end().unwrap();
        let mut again = Enc::new();
        codec::encode_request(&owned.as_request(), &mut again).unwrap();
        assert_eq!(again.bytes, e.bytes, "{op:?}");
        for cut in 0..e.bytes.len() {
            let mut d = Dec::new(&e.bytes[..cut]);
            assert!(codec::decode_request(&mut d).and_then(|_| d.end()).is_err(), "{op:?} cut at {cut}");
        }
        assert!(seen.insert(e.bytes), "{op:?} shares an encoding");
    }
    // The row byte after the request tag: 8 is no row.
    let mut unknown = Enc::new();
    unknown.u8(7);
    unknown.u8(8);
    unknown.count(0);
    codec::encode_qty(&qty, &mut unknown).unwrap();
    assert!(codec::decode_request(&mut Dec::new(&unknown.bytes)).is_err());
    // An argument with no transfer form is refused before the request is sent.
    let bad = [Value::Cost(Puzzle::Full, CostUnit::Generator, Arc::new(BigInt::from(-1)))];
    let r = Request::Kernel { op: KernelOp::Digest, args: &bad, ty: &qty };
    assert!(codec::encode_request(&r, &mut Enc::new()).is_err());
}
