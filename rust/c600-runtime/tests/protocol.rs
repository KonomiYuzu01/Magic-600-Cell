//! The protocol itself: no message can carry a user-action token (KN5.15); a lost acknowledgement causes at most an
//! identical delivery, answered from the parent's record, never a second host call (KI.5); the canonical transfer
//! encoding keeps every value variant with exactly one encoding and refuses the rest; frames are bounded.

mod support;

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_ir::{Capability, ExactRole, QTy, Ty};
use c600_run::approx::Approx;
use c600_run::interp::{NoHost, Request, RowTariff, Run, Sink};
use c600_run::meter::code::STEPS;
use c600_run::meter::{Amounts, ChildRecord, Interruption, Outcome, Position, SourceRange};
use c600_run::value::{Closure, Quad, Ratio, Value};
use c600_runtime::codec::{self, Codes};
use c600_runtime::message::{self, Answer, Call, SCHEMA};
use c600_runtime::wire::{self, Dec, Enc, FrameError, MAX_FRAME, PROTOCOL};
use c600_runtime::{run_in_process, Ended, FaultKind, JobOptions, Refused, Stage, FAULT_VARIABLE};
use num_bigint::BigInt;
use support::{assert_same_end, header, job, runtime, Session, WIDE};

/// Words that would name an authority in a field: a user-action token, a confirmation or a credential.
const AUTHORITY: [&str; 10] =
    ["token", "auth", "confirm", "commit", "approv", "secret", "credential", "user_action", "signature", "nonce"];

fn recorded(f: impl FnOnce(&mut Enc)) -> Enc {
    let mut e = Enc::recording();
    f(&mut e);
    e
}

fn completed_run() -> Run {
    let source = format!("{}export fn f() -> Nat effects [] {{\n    return 1;\n}}\n", header(""));
    match run_in_process(&job(&source, "f", vec![], JobOptions::new(WIDE)), &mut NoHost, None) {
        Ended::Ran(run) => run,
        Ended::Refused(r) => panic!("{r:#?}"),
    }
}

/// One encoding of every message variant, with recorded field names.
fn samples() -> Vec<Enc> {
    let qty = QTy { ty: Ty::Nat, display: false };
    let ty = Ty::Word;
    let requests = [
        Request::Snapshot { query: "pieces", ty: &qty },
        Request::Declassified { id: "d", ty: &ty },
        Request::FileRead { path: "p", ty: &ty },
        Request::Clock,
        Request::Propose { word: &[1, -2], target: "t", request: b"r" },
    ];
    let sinks = [
        Sink::View,
        Sink::Workspace { record: "k", ty: &qty },
        Sink::File { path: "p" },
        Sink::Exact { role: ExactRole::Goal },
    ];
    let value = Value::int(1);
    let j = support::job(
        "source",
        "f",
        vec![Value::int(1)],
        JobOptions {
            seed: Some((vec![1; 32], vec!["a".into()])),
            grant: vec![Capability::File],
            declassifiers: vec!["d".into()],
            ..JobOptions::new(WIDE)
        },
    );
    let mut out = vec![recorded(|e| message::encode_run(1, &j, e).unwrap())];
    for a in [
        Answer::Tariff(RowTariff { kernel_cost: 1, result_bound: 2 }),
        Answer::Value(Value::int(3)),
        Answer::Delivered,
        Answer::Refused("X:y.z".into()),
    ] {
        out.push(recorded(|e| message::encode_answer(1, 1, &a, e).unwrap()));
    }
    out.push(recorded(|e| message::encode_cancel(1, Interruption::ByUser, e)));
    out.push(recorded(message::encode_hello));
    for r in &requests {
        out.push(recorded(|e| message::encode_call(1, 1, 0, &Call::Tariff(r), e).unwrap()));
        out.push(recorded(|e| message::encode_call(1, 1, 0, &Call::Perform(r), e).unwrap()));
    }
    for s in &sinks {
        out.push(recorded(|e| message::encode_call(1, 1, 0, &Call::Sink(s, &value, b"v"), e).unwrap()));
    }
    let ran = Ended::Ran(completed_run());
    let refused = Ended::Refused(Refused { stage: Stage::Admit, codes: vec!["c".into()], detail: "d".into() });
    out.push(recorded(|e| message::encode_done(1, 0, &ran, e).unwrap()));
    out.push(recorded(|e| message::encode_done(1, 0, &refused, e).unwrap()));
    out
}

/// KN5.15: the token never reaches the runtime. [`SCHEMA`] lists every field of every message; every encoding writes
/// only listed fields, in the listed order, and the samples write every listed field, so the list is complete. No
/// message or field names a token, a confirmation or any other authority.
#[test]
fn kn5_15_no_message_has_a_field_that_could_carry_a_token() {
    for m in &SCHEMA {
        for name in std::iter::once(&m.name).chain(m.fields.iter()) {
            let lower = name.to_lowercase();
            assert!(!AUTHORITY.iter().any(|w| lower.contains(w)), "{} has an authority field `{name}`", m.name);
        }
        let distinct: BTreeSet<_> = m.fields.iter().collect();
        assert_eq!(distinct.len(), m.fields.len(), "{}: a field listed twice", m.name);
    }
    let kinds: BTreeSet<u8> = SCHEMA.iter().map(|m| m.kind).collect();
    assert_eq!(kinds.len(), SCHEMA.len());

    let mut covered: Vec<BTreeSet<&str>> = vec![BTreeSet::new(); SCHEMA.len()];
    for sample in samples() {
        assert_eq!(sample.bytes[0], PROTOCOL);
        let index = SCHEMA.iter().position(|m| m.kind == sample.bytes[1]).expect("a listed message kind");
        let listed = SCHEMA[index].fields;
        let mut at = 0;
        for field in sample.fields() {
            let found = listed[at..].iter().position(|f| f == field);
            let Some(offset) = found else {
                panic!("{}: field `{field}` is not listed, or out of order", SCHEMA[index].name)
            };
            at += offset + 1;
            covered[index].insert(field);
        }
    }
    for (m, covered) in SCHEMA.iter().zip(&covered) {
        let listed: BTreeSet<&str> = m.fields.iter().copied().collect();
        assert_eq!(&listed, covered, "{}: listed fields no sample writes", m.name);
    }
}

/// Every decoded message is one of the listed kinds; a payload of another protocol or kind is refused.
#[test]
fn unknown_protocols_and_kinds_are_refused() {
    let mut hello = Enc::new();
    message::encode_hello(&mut hello);
    let mut other = hello.bytes.clone();
    other[0] = PROTOCOL + 1;
    assert!(message::decode_to_parent(&other, &Codes::default()).is_err());
    for kind in 0..=u8::MAX {
        let mut bytes = hello.bytes.clone();
        bytes[1] = kind;
        let listed = SCHEMA.iter().any(|m| m.kind == kind);
        if !listed {
            assert!(message::decode_to_parent(&bytes, &Codes::default()).is_err(), "{kind:#x}");
            assert!(message::decode_to_child(&bytes).is_err(), "{kind:#x}");
        }
    }
    let mut trailing = hello.bytes.clone();
    trailing.push(0);
    assert!(message::decode_to_parent(&trailing, &Codes::default()).is_err());
}

/// The work job of the KI.5 rehearsals: five host calls.
const WORK: &str = "export fn work() -> Nat effects [snapshot, view] {\n    \
    let a = read_snapshot<Nat>(\"pieces\");\n    let v = publish_view(record { a = a });\n    \
    let b = read_snapshot<Nat>(\"pieces\");\n    return exact_add(a, b);\n}\n";

fn work() -> c600_runtime::Job {
    job(&format!("{}{WORK}", header("")), "work", vec![], JobOptions::new(WIDE))
}

/// KI.5: when every answer is delivered twice, as after a lost acknowledgement, the parent answers the repeats from
/// its record with identical bytes (the child checks), the host sees each call once, and the run is the same.
#[cfg(debug_assertions)]
#[test]
fn ki_5_a_repeated_call_is_answered_from_the_record() {
    let mut rt = runtime();
    rt.config_mut().env.push((FAULT_VARIABLE.into(), "duplicate@0".into()));
    let mut reference = Session::default();
    let expected = run_in_process(&work(), &mut reference, None);
    let mut session = Session::default();
    let got = rt.run(&work(), &mut session, None).unwrap_or_else(|f| panic!("{f}\n{}", f.stderr));
    assert_same_end(&expected, &got);
    reference.assert_same(&session);
    assert_eq!(session.log.len(), 5);
    assert_eq!(rt.repeats(), 5, "every call was repeated once and answered from the record");
}

/// KI.5: a call repeated after its acknowledgement, or a call id reused for another request, is a protocol fault;
/// the host never sees the repeat.
#[cfg(debug_assertions)]
#[test]
fn ki_5_a_stale_or_changed_call_is_a_protocol_fault() {
    for fault in ["stale@1", "changed@1"] {
        let mut rt = runtime();
        rt.config_mut().env.push((FAULT_VARIABLE.into(), fault.into()));
        let mut session = Session::default();
        let f = rt.run(&work(), &mut session, None).expect_err("a fault");
        assert_eq!(f.kind, FaultKind::Protocol, "{fault}: {f}");
        assert_eq!(f.code(), c600_runtime::parent::RUNTIME_LOST);
        assert_eq!(session.log.len(), 1, "{fault}: the host saw the first call only");
    }
}

fn approx(precision: u32, negative: bool, mantissa: u128, exponent: i128) -> Value {
    Value::Approx(Approx { precision, negative, mantissa, exponent })
}

/// Every value variant round-trips with exactly one encoding.
#[test]
fn every_value_variant_round_trips_with_one_encoding() {
    let values = vec![
        Value::Unit,
        Value::Bool(true),
        Value::int(0),
        Value::int(255),
        Value::int(-(BigInt::from(1u8) << 130u32)),
        Value::Rat(Arc::new(Ratio::new(BigInt::from(-3), BigInt::from(4)))),
        Value::Q5(Arc::new(Quad::new(BigInt::from(1), BigInt::from(1), BigInt::from(2)))),
        approx(24, false, 0, 0),
        approx(24, true, 1 << 23, -3),
        approx(53, false, (1 << 53) - 1, 7),
        approx(113, true, 1 << 112, -200),
        Value::ApproxBool(false),
        Value::text("\u{3bb} \"quoted\"\n"),
        Value::target("t"),
        Value::tag("clear"),
        Value::colour("red"),
        Value::word(&[]),
        Value::word(&[1, -1200, 600]),
        Value::record(vec![]),
        Value::record(vec![("b", Value::int(1)), ("a", Value::record(vec![("c", Value::word(&[2]))]))]),
        Value::Closure(Arc::new(Closure::new(3, vec![Value::int(1), Value::text("a")]))),
        Value::Closure(Arc::new(Closure::new(0, vec![]))),
        Value::Fn(7),
        Value::receipt(vec![("id", Value::text("preview-1"))]),
    ];
    let mut seen = BTreeSet::new();
    for v in &values {
        let bytes = codec::value_bytes(v).unwrap_or_else(|e| panic!("{v:?}: {e:?}"));
        let back = codec::value_from_bytes(&bytes).unwrap_or_else(|e| panic!("{v:?}: {e}"));
        assert_eq!(format!("{v:?}"), format!("{back:?}"));
        assert_eq!(codec::value_bytes(&back).unwrap(), bytes);
        assert!(seen.insert(bytes), "{v:?} shares an encoding");
    }
    // A Nat and an equal Rat stay apart.
    assert!(!codec::same_value(&Value::int(1), &Value::Rat(Arc::new(Ratio::new(1.into(), 1.into())))));
}

/// Values outside their canonical form are refused when encoded and when decoded.
#[test]
fn noncanonical_values_are_refused_on_both_sides() {
    let rat = |num: i64, den: i64| Value::Rat(Arc::new(Ratio { num: num.into(), den: den.into() }));
    let quad = |a: i64, b: i64, d: i64| Value::Q5(Arc::new(Quad { a: a.into(), b: b.into(), d: d.into() }));
    for v in [
        rat(2, 4),
        rat(1, -2),
        rat(1, 0),
        quad(2, 2, 2),
        quad(1, 1, -2),
        approx(32, false, 1 << 31, 0),
        approx(53, false, 1 << 53, 0),
        approx(53, false, 1 << 51, 0),
        approx(24, true, 0, 0),
        approx(24, false, 0, 1),
        Value::Word(Arc::from(vec![0])),
        Value::Word(Arc::from(vec![1201])),
        Value::record(vec![("a", Value::Unit), ("a", Value::Unit)]),
    ] {
        assert!(codec::value_bytes(&v).is_err(), "{v:?}");
    }

    let decoded = |f: &dyn Fn(&mut Enc)| {
        let mut e = Enc::new();
        f(&mut e);
        codec::value_from_bytes(&e.bytes)
    };
    // 2/4 written by hand.
    assert!(decoded(&|e| {
        e.u8(3);
        e.u8(1);
        e.bytes(&[2]);
        e.u8(1);
        e.bytes(&[4]);
    })
    .is_err());
    // An integer with a trailing zero byte, and a negative zero.
    assert!(decoded(&|e| {
        e.u8(2);
        e.u8(1);
        e.bytes(&[5, 0]);
    })
    .is_err());
    assert!(decoded(&|e| {
        e.u8(2);
        e.u8(2);
        e.bytes(&[]);
    })
    .is_err());
    // A flag other than 0 or 1, an unknown tag, a record with a repeated field, a length beyond the payload.
    assert!(decoded(&|e| {
        e.u8(1);
        e.u8(2);
    })
    .is_err());
    assert!(decoded(&|e| e.u8(16)).is_err());
    assert!(decoded(&|e| {
        e.u8(12);
        e.count(2);
        e.text("a");
        e.u8(0);
        e.text("a");
        e.u8(0);
    })
    .is_err());
    assert!(decoded(&|e| {
        e.u8(9);
        e.count(1 << 40);
    })
    .is_err());
    // The same layouts in canonical form decode: the refusals above are about the form, not the layout.
    let controls: [&dyn Fn(&mut Enc); 6] = [
        &|e| {
            e.u8(3);
            e.u8(1);
            e.bytes(&[1]);
            e.u8(1);
            e.bytes(&[4]);
        },
        &|e| {
            e.u8(2);
            e.u8(1);
            e.bytes(&[5]);
        },
        &|e| {
            e.u8(2);
            e.u8(0);
            e.bytes(&[]);
        },
        &|e| {
            e.u8(1);
            e.u8(1);
        },
        &|e| {
            e.u8(12);
            e.count(2);
            e.text("a");
            e.u8(0);
            e.text("b");
            e.u8(0);
        },
        &|e| {
            e.u8(9);
            e.count(1);
            e.i32(-1200);
        },
    ];
    for control in controls {
        if let Err(e) = decoded(control) {
            panic!("a canonical control was refused: {e}");
        }
    }
    // Trailing bytes after a whole value.
    let mut bytes = codec::value_bytes(&Value::Unit).unwrap();
    bytes.push(0);
    assert!(codec::value_from_bytes(&bytes).is_err());
}

/// A value nested far deeper than any stack allows decodes and encodes again without recursion, and drops without it
/// (`c600-run`'s records and closures drop iteratively).
#[test]
fn a_deep_value_crosses_without_recursion() {
    const DEPTH: usize = 200_000;
    let mut e = Enc::new();
    for _ in 0..DEPTH {
        e.u8(12);
        e.count(1);
        e.text("a");
    }
    e.u8(0);
    let value = codec::value_from_bytes(&e.bytes).expect("a deep record decodes");
    assert_eq!(codec::value_bytes(&value).unwrap(), e.bytes);
    drop(value);
}

/// A summary with rows, a stop with its source range, and a child record round-trips; an unknown stop code or
/// phase is refused.
#[test]
fn summaries_round_trip_and_unknown_codes_are_refused() {
    let mut summary = completed_run().summary;
    assert!(!summary.rows.is_empty());
    summary.outcome = Outcome::CutOff;
    summary.code = Some(STEPS);
    let stop = Position {
        at: "m.f:root#value".into(),
        phase: "steps",
        scope: "root".into(),
        source: Some(SourceRange { digest: "ab".repeat(32), start: 1, end: 5 }),
    };
    summary.stop = Some(stop.clone());
    summary.children.push(ChildRecord {
        at: "m.f:0#child".into(),
        requested: Amounts { steps: 3, ..Amounts::default() },
        effective: Amounts { steps: 2, ..Amounts::default() },
        outcome: Outcome::Error,
        code: Some("TEST:host.refused"),
        stop: Some(stop),
        used: Amounts { steps: 2, kernel_cost: 1, memory: 0, output: 0 },
        returned_memory: 4,
        live_after_return: 0,
    });
    let mut e = Enc::new();
    codec::encode_summary(&summary, &mut e);

    let mut codes = Codes::default();
    assert!(codec::decode_summary(&mut Dec::new(&e.bytes), &codes).is_err(), "a host code nobody returned");
    codes.admit("TEST:host.refused");
    let mut d = Dec::new(&e.bytes);
    let back = codec::decode_summary(&mut d, &codes).unwrap();
    d.end().unwrap();
    assert_eq!(back, summary);

    // The phase is written as a length-prefixed text; the stop code `KN7:budget.steps` has another prefix.
    let written = [&5u64.to_le_bytes()[..], b"steps"].concat();
    let phase = e.bytes.windows(written.len()).position(|w| w == written).unwrap() + 8;
    let mut bad = e.bytes.clone();
    bad[phase..phase + 5].copy_from_slice(b"stepz");
    assert!(codec::decode_summary(&mut Dec::new(&bad), &codes).is_err());
}

/// Frames: a length above the bound is refused before anything is read or allocated; a stream that ends inside a
/// frame is truncated; one that ends between frames is closed; a payload above the bound is never written.
#[test]
fn frames_are_bounded() {
    let mut stream = Vec::new();
    wire::write_frame(&mut stream, b"one").unwrap();
    wire::write_frame(&mut stream, b"").unwrap();
    let mut input = &stream[..];
    assert_eq!(wire::read_frame(&mut input).unwrap(), b"one");
    assert_eq!(wire::read_frame(&mut input).unwrap(), b"");
    assert_eq!(wire::read_frame(&mut input), Err(FrameError::Closed));

    let over = ((MAX_FRAME + 1) as u32).to_le_bytes();
    assert_eq!(wire::read_frame(&mut &over[..]), Err(FrameError::Oversized(MAX_FRAME as u64 + 1)));
    let huge = u32::MAX.to_le_bytes();
    assert_eq!(wire::read_frame(&mut &huge[..]), Err(FrameError::Oversized(u64::from(u32::MAX))));
    assert_eq!(wire::read_frame(&mut &[3u8, 0][..]), Err(FrameError::Truncated));
    assert_eq!(wire::read_frame(&mut &[3u8, 0, 0, 0, 1][..]), Err(FrameError::Truncated));

    let mut sink = Vec::new();
    assert!(wire::write_frame(&mut sink, &vec![0u8; MAX_FRAME + 1]).is_err());
    assert!(sink.is_empty());
}
