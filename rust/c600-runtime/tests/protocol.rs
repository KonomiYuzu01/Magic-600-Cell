//! The protocol itself: no message can carry a user-action token (KN5.15); a lost acknowledgement causes at most an
//! identical delivery, answered from the parent's record, never a second host call, and a new call must acknowledge
//! the last answer (KI.5); the canonical transfer encoding keeps every value variant with exactly one encoding and
//! refuses the rest; frames are bounded. LP2's drafts keep the earlier bytes: a snapshot read with an argument is
//! request tag 5, and a menu word (LB1.8) is value tag 21, type tag 23 and, as a preview, request tag 6; an end
//! test's verdict (LB3.2) is value tag 22 and type tag 24; a trace key (LB1.10) is value tag 23 and type tag 25; a
//! generator (DET.61) has no value tag and is type tag 27; the built-in column, the pinned menu and the pinned trace
//! relation are strict trailing fields of the run, in that order.

mod support;

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_check::{Profile, Row};
use c600_ir::{Capability, EndTest, EndTestOutcome, ExactRole, QTy, Ty};
use c600_run::approx::Approx;
use c600_run::interp::{NoHost, Request, RowTariff, Run, Sink};
use c600_run::menu::{MenuAlphabet, MenuError, MenuIdentity};
use c600_run::meter::code::STEPS;
use c600_run::meter::{Amounts, ChildRecord, Interruption, Outcome, Position, SourceRange};
use c600_run::rng::Cursor;
use c600_run::trace::{Relation, RelationError, TraceKey, FULL_DISJOINT_SUPPORT};
use c600_run::value::{Closure, Generator, Quad, Ratio, Value};
use c600_runtime::codec::{self, Codes};
use c600_runtime::message::{self, Answer, Call, ToChild, SCHEMA};
use c600_runtime::wire::{self, Dec, Enc, FrameError, MAX_FRAME, PROTOCOL};
use c600_runtime::{run_in_process, Ended, FaultKind, JobOptions, Refused, Stage, FAULT_VARIABLE};
use num_bigint::BigInt;
use support::{assert_same_end, header, job, runtime, Session, WIDE};

/// Words that would name an authority in a field: a user-action token, a confirmation or a credential.
const AUTHORITY: [&str; 10] =
    ["token", "auth", "confirm", "commit", "approv", "secret", "credential", "user_action", "signature", "nonce"];

/// The fields of an identity block that names an alphabet (LB1.8; LP2 draft), in KN1.4's order.
const NAMED: [&str; 5] = ["600-cell-jumbled", "model", "menu", "order", "rev"];

/// The identity block [`NAMED`] with another menu identity.
fn identity(menu_id: &str) -> MenuIdentity {
    let [puzzle, model_id, _, menu_order, contract_rev] = NAMED.map(String::from);
    MenuIdentity { puzzle, model_id, menu_id: menu_id.into(), menu_order, contract_rev }
}

/// A pinned alphabet of four caps and a menu of three items whose last two are each other's inverses.
fn alphabet() -> MenuAlphabet {
    MenuAlphabet::new(identity("menu"), 4, vec![0, 2, 1]).expect("an alphabet")
}

/// A relation of 1,200 generators (LB1.10; LP2 draft) in which only generators 1 and 3 share a slot.
fn relation(model_id: &str) -> Relation {
    let supports: Vec<Vec<i32>> = (1..=1200).map(|g| if g == 3 { vec![1, 3] } else { vec![g] }).collect();
    Relation::full_disjoint_support("600-cell-Full", model_id, &supports).expect("a relation")
}

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
    let menu = identity("menu");
    let requests = [
        Request::Snapshot { query: "pieces", ty: &qty },
        Request::Declassified { id: "d", ty: &ty },
        Request::FileRead { path: "p", ty: &ty },
        Request::Clock,
        Request::Propose { word: &[1, -2], target: "t", request: b"r" },
        Request::SnapshotWith { query: "protection", arg: b"[1,-2]\n", ty: &qty },
        Request::ProposeMenu { letters: &[[0, 1]], menu: &menu, target: "t", request: b"r" },
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
            menu: Some(alphabet()),
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

/// Every host request decodes to the request it was encoded from. Tags 0 to 4 keep their bytes; LP2's snapshot read
/// with an argument is tag 5 and a menu-word preview tag 6 (LP2 drafts). A cut request and an unknown tag are
/// refused, and so is, on both sides, a menu-word preview with a letter index of 600 or more or an identity block
/// that names no alphabet.
#[test]
fn every_request_round_trips_and_a_cut_or_unknown_one_is_refused() {
    let qty = QTy { ty: Ty::Nat, display: false };
    let ty = Ty::Word;
    let menu = identity("menu");
    let requests = [
        Request::Snapshot { query: "pieces", ty: &qty },
        Request::Declassified { id: "d", ty: &ty },
        Request::FileRead { path: "p", ty: &ty },
        Request::Clock,
        Request::Propose { word: &[1, -2], target: "t", request: b"r" },
        Request::SnapshotWith { query: "protection", arg: b"[1,-2]\n", ty: &qty },
        Request::ProposeMenu { letters: &[[0, 1], [3, 2]], menu: &menu, target: "t", request: b"r" },
    ];
    for (tag, r) in requests.iter().enumerate() {
        let mut e = Enc::new();
        codec::encode_request(r, &mut e).unwrap();
        assert_eq!(usize::from(e.bytes[0]), tag, "{r:?}");
        let mut d = Dec::new(&e.bytes);
        let owned = codec::decode_request(&mut d).unwrap();
        d.end().unwrap();
        let mut again = Enc::new();
        codec::encode_request(&owned.as_request(), &mut again).unwrap();
        assert_eq!(again.bytes, e.bytes, "{r:?}");
        for cut in 0..e.bytes.len() {
            let mut d = Dec::new(&e.bytes[..cut]);
            assert!(codec::decode_request(&mut d).and_then(|_| d.end()).is_err(), "{r:?} cut at {cut}");
        }
    }
    let mut with = Enc::new();
    codec::encode_request(&requests[5], &mut with).unwrap();
    assert_eq!(with.bytes[0], 5);
    with.bytes[0] = 7;
    assert!(codec::decode_request(&mut Dec::new(&with.bytes)).is_err(), "an unknown tag");

    let nameless = MenuIdentity::default();
    for r in [
        Request::ProposeMenu { letters: &[[600, 0]], menu: &menu, target: "t", request: b"r" },
        Request::ProposeMenu { letters: &[[0, 600]], menu: &menu, target: "t", request: b"r" },
        Request::ProposeMenu { letters: &[[0, 1]], menu: &nameless, target: "t", request: b"r" },
    ] {
        assert!(codec::encode_request(&r, &mut Enc::new()).is_err(), "{r:?}");
    }
    let written = |letters: &[[u32; 2]], fields: [&str; 5]| {
        let mut e = Enc::new();
        e.u8(6);
        e.count(letters.len());
        for [cap, item] in letters {
            e.u32(*cap);
            e.u32(*item);
        }
        for text in fields {
            e.text(text);
        }
        e.text("t");
        e.bytes(b"r");
        let mut d = Dec::new(&e.bytes);
        codec::decode_request(&mut d).and_then(|_| d.end()).map_err(|m| m.to_string())
    };
    assert_eq!(written(&[[599, 599]], NAMED), Ok(()), "a canonical control");
    assert_eq!(written(&[[600, 0]], NAMED).unwrap_err(), "a menu index of 600 or more");
    assert_eq!(written(&[[0, 600]], NAMED).unwrap_err(), "a menu index of 600 or more");
    let unnamed = ["600-cell-jumbled", "model", "menu", "order", ""];
    assert_eq!(written(&[[0, 1]], unnamed).unwrap_err(), "a menu word that names no alphabet");
}

/// A built-in tool's class rows (LP2 draft) cross as a trailing field of the run, before the pinned menu and the pinned
/// relation: a flag, then the rows strictly increasing in their own order, so a class has one encoding and an empty
/// class differs from none. A flag other than 0 or 1, an unknown row, rows out of order or repeated, a count beyond the
/// payload, a cut message and a trailing byte are refused. The wire does not read the profile: both executors refuse
/// class rows outside the solving profile at the admission stage (`equivalence.rs`).
#[test]
fn the_built_in_column_is_a_strict_trailing_field() {
    let all: BTreeSet<Row> = [
        Row::Unknown,
        Row::Preview("t".into()),
        Row::Workspace("k".into()),
        Row::View,
        Row::Declassified("d".into()),
        Row::Outcome,
        Row::Read("pieces".into()),
        Row::Read("best_word".into()),
    ]
    .into_iter()
    .collect();
    let run = |profile, built_in| {
        let mut e = Enc::new();
        let options = JobOptions { profile, built_in, ..JobOptions::new(WIDE) };
        message::encode_run(1, &job("source", "f", vec![], options), &mut e).unwrap();
        e.bytes
    };
    let decoded = |bytes: &[u8]| match message::decode_to_child(bytes) {
        Ok(ToChild::Run { job, .. }) => Ok(job),
        Ok(other) => panic!("{other:?}"),
        Err(m) => Err(m.to_string()),
    };
    for profile in [Profile::Solving, Profile::ToolAuthoring] {
        // The run ends with the column's flag, then the pinned menu's and the pinned relation's: none here.
        let none = run(profile, None);
        let (head, flags) = none.split_at(none.len() - 3);
        assert_eq!((flags, decoded(&none).unwrap().options.built_in), (&[0, 0, 0][..], None));
        for rows in [BTreeSet::new(), all.clone()] {
            let bytes = run(profile, Some(rows.clone()));
            let last = &bytes[bytes.len() - 2..];
            assert_eq!((&bytes[..head.len()], bytes[head.len()], last), (head, 1, &[0, 0][..]));
            let job = decoded(&bytes).unwrap();
            assert_eq!(job.options.built_in.as_ref(), Some(&rows));
            let mut again = Enc::new();
            message::encode_run(1, &job, &mut again).unwrap();
            assert_eq!(again.bytes, bytes);
            for cut in 0..bytes.len() {
                assert!(decoded(&bytes[..cut]).is_err(), "{profile:?}, {rows:?} cut at {cut}");
            }
            assert!(decoded(&[&bytes[..], &[0]].concat()).is_err(), "a trailing byte");
        }
    }

    let none = run(Profile::Solving, None);
    let head = &none[..none.len() - 3];
    let tail = |f: &dyn Fn(&mut Enc)| {
        let mut e = Enc::new();
        f(&mut e);
        [head, &e.bytes[..]].concat()
    };
    let listing = |rows: &[Row]| {
        tail(&|e: &mut Enc| {
            e.u8(1);
            e.count(rows.len());
            for row in rows {
                codec::encode_row(row, e);
            }
            // No pinned menu and no pinned relation.
            e.u8(0);
            e.u8(0);
        })
    };
    let control = decoded(&listing(&[Row::Read("a".into()), Row::Outcome, Row::View])).unwrap();
    assert_eq!(control.options.built_in.map(|rows| rows.len()), Some(3));
    let refusals = [
        (tail(&|e: &mut Enc| e.u8(2)), "an option flag other than 0 or 1"),
        (listing(&[Row::View, Row::Outcome]), "class rows out of order or repeated"),
        (listing(&[Row::View, Row::View]), "class rows out of order or repeated"),
        (listing(&[Row::Read("b".into()), Row::Read("a".into())]), "class rows out of order or repeated"),
        (
            tail(&|e: &mut Enc| {
                e.u8(1);
                e.count(1);
                e.u8(7);
            }),
            "an unknown class row",
        ),
        (
            tail(&|e: &mut Enc| {
                e.u8(1);
                e.count(2);
                codec::encode_row(&Row::View, e);
            }),
            "a length beyond the payload",
        ),
    ];
    for (bytes, why) in refusals {
        assert_eq!(decoded(&bytes).unwrap_err(), why);
    }
}

/// A pinned menu alphabet (LB1.8; LP2 draft) crosses as a trailing field of the run, before the pinned relation: a
/// flag, then the identity block in KN1.4's order, the cap count, the item count and the item of each item's inverse
/// rotation, so an alphabet has one encoding and no alphabet differs from every one. A flag other than 0 or 1, an
/// empty identity field, more than 600 caps or items, inverse items that are no involution, a count beyond the payload,
/// a cut message and a trailing byte are refused, each with `MenuAlphabet::new`'s reason where it has one.
#[test]
fn the_pinned_menu_is_a_strict_trailing_field() {
    let run = |menu: Option<MenuAlphabet>| {
        let mut e = Enc::new();
        let options = JobOptions { menu, ..JobOptions::new(WIDE) };
        message::encode_run(1, &job("source", "f", vec![], options), &mut e).unwrap();
        e.bytes
    };
    let decoded = |bytes: &[u8]| match message::decode_to_child(bytes) {
        Ok(ToChild::Run { job, .. }) => Ok(job),
        Ok(other) => panic!("{other:?}"),
        Err(m) => Err(m.to_string()),
    };
    // The run ends with the pinned menu's flag, then the pinned relation's: none here.
    let none = run(None);
    let (head, flags) = none.split_at(none.len() - 2);
    assert_eq!((flags, decoded(&none).unwrap().options.menu), (&[0, 0][..], None));
    let lettered = MenuAlphabet::new(identity("empty"), 0, vec![]).expect("an alphabet without letters");
    for menu in [alphabet(), lettered] {
        let bytes = run(Some(menu.clone()));
        assert_eq!((&bytes[..head.len()], bytes[head.len()], bytes.last()), (head, 1, Some(&0)));
        let job = decoded(&bytes).unwrap();
        assert_eq!(job.options.menu.as_ref(), Some(&menu));
        let mut again = Enc::new();
        message::encode_run(1, &job, &mut again).unwrap();
        assert_eq!(again.bytes, bytes);
        for cut in 0..bytes.len() {
            assert!(decoded(&bytes[..cut]).is_err(), "{menu:?} cut at {cut}");
        }
        assert!(decoded(&[&bytes[..], &[0]].concat()).is_err(), "a trailing byte");
    }

    let tail = |f: &dyn Fn(&mut Enc)| {
        let mut e = Enc::new();
        f(&mut e);
        [head, &e.bytes[..]].concat()
    };
    let pinned = |fields: [&str; 5], caps: u32, count: usize, inverse: &[u32]| {
        tail(&|e: &mut Enc| {
            e.u8(1);
            for text in fields {
                e.text(text);
            }
            e.u32(caps);
            e.count(count);
            for item in inverse {
                e.u32(*item);
            }
            // No pinned relation.
            e.u8(0);
        })
    };
    let control = decoded(&pinned(NAMED, 600, 3, &[1, 0, 2])).unwrap();
    assert_eq!(control.options.menu.map(|m| (m.caps(), m.items())), Some((600, 3)));
    let wide: Vec<u32> = (0..601).collect();
    let refusals = [
        (tail(&|e: &mut Enc| e.u8(2)), "an option flag other than 0 or 1".to_string()),
        (
            pinned(["600-cell-jumbled", "model", "", "order", "rev"], 4, 1, &[0]),
            MenuError::Identity("menu_id").to_string(),
        ),
        (pinned(NAMED, 601, 1, &[0]), MenuError::Caps(601).to_string()),
        (pinned(NAMED, 4, 601, &wide), MenuError::Items(601).to_string()),
        (pinned(NAMED, 4, 2, &[1, 1]), MenuError::Inverse { item: 0 }.to_string()),
        (pinned(NAMED, 4, 2, &[0, 5]), MenuError::Inverse { item: 1 }.to_string()),
        (pinned(NAMED, 4, 2, &[0]), "a length beyond the payload".to_string()),
    ];
    for (bytes, why) in refusals {
        assert_eq!(decoded(&bytes).unwrap_err(), why);
    }
}

/// A pinned trace relation (LB1.10; LP2 draft) crosses as the run's last field: a flag, then its contract, its
/// identity block's puzzle and model ID, its generator count and its independence bits; the child recomputes the
/// digest, so a relation has one encoding. A flag other than 0 or 1, another contract, an empty identity field,
/// another generator count than 1,200, bits of another length, a count beyond the payload, a cut message and a
/// trailing byte are refused, each with `Relation::from_parts`'s reason where it has one.
#[test]
fn the_pinned_relation_is_a_strict_trailing_field() {
    let run = |relation: Option<Relation>| {
        let mut e = Enc::new();
        let options = JobOptions { relation, ..JobOptions::new(WIDE) };
        message::encode_run(1, &job("source", "f", vec![], options), &mut e).unwrap();
        e.bytes
    };
    let decoded = |bytes: &[u8]| match message::decode_to_child(bytes) {
        Ok(ToChild::Run { job, .. }) => Ok(job),
        Ok(other) => panic!("{other:?}"),
        Err(m) => Err(m.to_string()),
    };
    let none = run(None);
    let (flag, head) = none.split_last().unwrap();
    assert_eq!((*flag, decoded(&none).unwrap().options.relation), (0, None));
    let pinned = relation("model");
    let bytes = run(Some(pinned.clone()));
    assert_eq!((&bytes[..head.len()], bytes[head.len()]), (head, 1));
    let job = decoded(&bytes).unwrap();
    let back = job.options.relation.clone().expect("the pinned relation");
    assert_eq!((&back, back.digest()), (&pinned, pinned.digest()));
    assert!(back.independent(2, 3) && !back.independent(1, 3) && !back.independent(-3, 1));
    let mut again = Enc::new();
    message::encode_run(1, &job, &mut again).unwrap();
    assert_eq!(again.bytes, bytes);
    // Every cut through the head and the relation's fields, and a sample through its 89,925 bytes of bits.
    let fields = head.len() + 80;
    for cut in (0..fields).chain((fields..bytes.len()).step_by(4093)).chain([bytes.len() - 1]) {
        assert!(decoded(&bytes[..cut]).is_err(), "cut at {cut}");
    }
    assert!(decoded(&[&bytes[..], &[0]].concat()).is_err(), "a trailing byte");

    let tail = |f: &dyn Fn(&mut Enc)| {
        let mut e = Enc::new();
        f(&mut e);
        [head, &e.bytes[..]].concat()
    };
    let parts = |contract: &str, puzzle: &str, model_id: &str, generators: u32, bits: &[u8]| {
        tail(&|e: &mut Enc| {
            e.u8(1);
            e.text(contract);
            e.text(puzzle);
            e.text(model_id);
            e.u32(generators);
            e.bytes(bits);
        })
    };
    let bits = pinned.bits();
    let control = decoded(&parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model", 1200, bits)).unwrap();
    assert_eq!(control.options.relation.as_ref(), Some(&pinned));
    let longer = [bits, &[0]].concat();
    let refusals = [
        (tail(&|e: &mut Enc| e.u8(2)), "an option flag other than 0 or 1".to_string()),
        (parts("other/1", "600-cell-Full", "model", 1200, bits), RelationError::Contract("other/1".into()).to_string()),
        (parts(FULL_DISJOINT_SUPPORT, "", "model", 1200, bits), RelationError::Identity("puzzle").to_string()),
        (
            parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "", 1200, bits),
            RelationError::Identity("model_id").to_string(),
        ),
        (
            parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model", 1199, bits),
            RelationError::Generators(1199).to_string(),
        ),
        (parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model", 1200, &bits[1..]), RelationError::Bits.to_string()),
        (parts(FULL_DISJOINT_SUPPORT, "600-cell-Full", "model", 1200, &longer), RelationError::Bits.to_string()),
        (
            tail(&|e: &mut Enc| {
                e.u8(1);
                e.text(FULL_DISJOINT_SUPPORT);
                e.text("600-cell-Full");
                e.text("model");
                e.u32(1200);
                e.count(8);
            }),
            "a length beyond the payload".to_string(),
        ),
    ];
    for (bytes, why) in refusals {
        assert_eq!(decoded(&bytes).unwrap_err(), why);
    }
}

/// A trace key (LB1.10, LG2.23; LP2 draft) crosses as value tag 23 with its header in LG2.23's order (puzzle, model ID,
/// generator count, key, contract, digest), then its letters, so keys under two relations have two encodings. A header
/// with an empty field and a letter outside 1 to 1200 are refused on both sides, and a digest of other than 32 bytes
/// when decoded. The codec cannot order letters without the relation: a run checks a decoded key against its pinned
/// relation (`c600-run`'s `trace_keys` tests). The type `TraceEqualityKey` is type tag 25.
#[test]
fn a_trace_key_crosses_with_its_header() {
    let (one, other) = (relation("model"), relation("other"));
    let keys = [
        Value::TraceEqualityKey(Arc::new(one.key(&[]).expect("a key"))),
        Value::TraceEqualityKey(Arc::new(one.key(&[3, 2, -1200]).expect("a key"))),
        Value::TraceEqualityKey(Arc::new(other.key(&[3, 2, -1200]).expect("a key"))),
    ];
    let mut seen = BTreeSet::new();
    for v in &keys {
        let bytes = codec::value_bytes(v).unwrap_or_else(|e| panic!("{v:?}: {e:?}"));
        assert_eq!(bytes[0], 23);
        let back = codec::value_from_bytes(&bytes).unwrap_or_else(|e| panic!("{v:?}: {e}"));
        assert_eq!(format!("{v:?}"), format!("{back:?}"));
        assert_eq!(codec::value_bytes(&back).unwrap(), bytes);
        assert!(seen.insert(bytes), "{v:?} shares an encoding");
    }
    let header = Arc::new(one.header().clone());
    for letters in [&[0][..], &[1201], &[-1201], &[i32::MIN]] {
        let v = Value::TraceEqualityKey(Arc::new(TraceKey::from_parts(header.clone(), letters)));
        assert!(codec::value_bytes(&v).is_err(), "{letters:?}");
    }
    let mut blank = one.header().clone();
    blank.key = String::new();
    let v = Value::TraceEqualityKey(Arc::new(TraceKey::from_parts(Arc::new(blank), &[1])));
    assert!(codec::value_bytes(&v).is_err(), "a header with an empty field");

    let written = |fields: [&str; 4], digest: &[u8], letters: &[i32]| {
        let mut e = Enc::new();
        e.u8(23);
        e.text(fields[0]);
        e.text(fields[1]);
        e.u32(1200);
        e.text(fields[2]);
        e.text(fields[3]);
        e.bytes(digest);
        e.count(letters.len());
        for l in letters {
            e.i32(*l);
        }
        codec::value_from_bytes(&e.bytes).map(|_| ()).map_err(|m| m.to_string())
    };
    let h = one.header();
    let fields = [h.puzzle.as_str(), h.model_id.as_str(), h.key.as_str(), h.contract.as_str()];
    assert_eq!(written(fields, &h.digest, &[3, 2]), Ok(()), "a canonical control");
    // The letters are taken as written; their order is the run's to check against its relation.
    assert_eq!(written(fields, &h.digest, &[2, 3]), Ok(()));
    for i in 0..4 {
        let mut empty = fields;
        empty[i] = "";
        assert_eq!(written(empty, &h.digest, &[1]).unwrap_err(), "a trace key header with an empty field");
    }
    let longer = [&h.digest[..], &[0]].concat();
    for digest in [&h.digest[..31], &longer[..]] {
        assert_eq!(written(fields, digest, &[1]).unwrap_err(), "a relation digest of other than 32 bytes");
    }
    for letter in [0, 1201, -1201] {
        assert_eq!(written(fields, &h.digest, &[1, letter]).unwrap_err(), "a letter outside 1 to 1200");
    }

    let mut e = Enc::new();
    codec::encode_ty(&Ty::TraceEqualityKey, &mut e).unwrap();
    assert_eq!(e.bytes, [25]);
    let mut d = Dec::new(&e.bytes);
    assert_eq!(codec::decode_ty(&mut d).unwrap(), Ty::TraceEqualityKey);
    d.end().unwrap();
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

/// KI.5: the child is synchronous, so a new call must acknowledge the last answer and the parent keeps at most one
/// answer open. A child that stops acknowledging is a protocol fault at its first unacknowledged call, which the host
/// never sees.
#[cfg(debug_assertions)]
#[test]
fn ki_5_a_new_call_must_acknowledge_the_last_answer() {
    for after in [1usize, 3] {
        let mut rt = runtime();
        rt.config_mut().env.push((FAULT_VARIABLE.into(), format!("unacked@{after}").into()));
        let mut session = Session::default();
        let f = rt.run(&work(), &mut session, None).expect_err("a fault");
        assert_eq!(f.kind, FaultKind::Protocol, "unacked@{after}: {f}");
        assert!(f.detail.contains("not the last answer"), "unacked@{after}: {f}");
        assert_eq!(f.code(), c600_runtime::parent::RUNTIME_LOST);
        assert_eq!(session.log.len(), after, "unacked@{after}: the host saw the acknowledged calls only");
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

/// A menu word (LB1.8; LP2 draft) crosses with the identity block of its alphabet: words of two alphabets have two
/// encodings, a word whose block names no alphabet has none, and a decoded block with an empty field or a letter index
/// of 600 or more is refused. The type `MenuWord` is type tag 23.
#[test]
fn a_menu_word_crosses_with_its_alphabet() {
    let named = Arc::new(identity("menu"));
    let words = [
        Value::menu_word(&named, &[]),
        Value::menu_word(&named, &[[0, 1], [599, 2]]),
        Value::menu_word(&Arc::new(identity("other")), &[[0, 1], [599, 2]]),
    ];
    let mut seen = BTreeSet::new();
    for v in words.iter().map(|w| w.as_ref().expect("letters below 600")) {
        let bytes = codec::value_bytes(v).unwrap_or_else(|e| panic!("{v:?}: {e:?}"));
        assert_eq!(bytes[0], 21);
        let back = codec::value_from_bytes(&bytes).unwrap_or_else(|e| panic!("{v:?}: {e}"));
        assert_eq!(format!("{v:?}"), format!("{back:?}"));
        assert_eq!(codec::value_bytes(&back).unwrap(), bytes);
        assert!(seen.insert(bytes), "{v:?} shares an encoding");
    }
    let nameless = Value::menu_word(&Arc::new(MenuIdentity::default()), &[[0, 1]]).expect("letters below 600");
    assert!(codec::value_bytes(&nameless).is_err());

    let decoded = |fields: [&str; 5], letters: &[[u32; 2]]| {
        let mut e = Enc::new();
        e.u8(21);
        for text in fields {
            e.text(text);
        }
        e.count(letters.len());
        for [cap, item] in letters {
            e.u32(*cap);
            e.u32(*item);
        }
        codec::value_from_bytes(&e.bytes).map(|_| ()).map_err(|m| m.to_string())
    };
    assert_eq!(decoded(NAMED, &[[599, 0]]), Ok(()), "a canonical control");
    let unnamed = ["600-cell-jumbled", "model", "menu", "", "rev"];
    assert_eq!(decoded(unnamed, &[[0, 0]]).unwrap_err(), "a menu word that names no alphabet");
    assert_eq!(decoded(NAMED, &[[600, 0]]).unwrap_err(), "a menu index of 600 or more");
    assert_eq!(decoded(NAMED, &[[0, 600]]).unwrap_err(), "a menu index of 600 or more");

    let mut e = Enc::new();
    codec::encode_ty(&Ty::MenuWord, &mut e).unwrap();
    assert_eq!(e.bytes, [23]);
    let mut d = Dec::new(&e.bytes);
    assert_eq!(codec::decode_ty(&mut d).unwrap(), Ty::MenuWord);
    d.end().unwrap();
}

/// A generator (DET.61; LP2 draft) stays in its run: a copy in another process would be a second cursor on its stream,
/// which only DET.61's alias makes, so it has no value tag and a value that holds one is refused whole. The type
/// `Generator` is type tag 27; tag 26 stays free for KN3.2's draft type.
#[test]
fn a_generator_has_no_transfer_form_and_its_type_is_tag_27() {
    let g = Value::Generator(Arc::new(Generator::new(Cursor::new([1; 32]))));
    for v in [g.clone(), Value::list(vec![g.clone()]), Value::record(vec![("g", g)])] {
        let refused = codec::value_bytes(&v).expect_err("no transfer form");
        assert_eq!(refused.0, "a generator, which stays in its run", "{v:?}");
    }
    let mut e = Enc::new();
    codec::encode_ty(&Ty::Generator, &mut e).unwrap();
    assert_eq!(e.bytes, [27]);
    let mut d = Dec::new(&e.bytes);
    assert_eq!(codec::decode_ty(&mut d).unwrap(), Ty::Generator);
    d.end().unwrap();
}

/// An end test's verdict (LB3.2; LP2 draft) crosses as value tag 22, its test's index and its outcome's index, so
/// each of the nine verdicts has one encoding of its own; an index of 3 or more and a missing byte are refused. The
/// verdict's type is type tag 24 and the test's index, so one test's verdict type never decodes as another's.
#[test]
fn an_end_test_verdict_crosses_with_its_test_and_outcome() {
    let mut seen = BTreeSet::new();
    for (i, test) in EndTest::ALL.into_iter().enumerate() {
        for (j, outcome) in EndTestOutcome::ALL.into_iter().enumerate() {
            let v = Value::EndTest(test, outcome);
            let bytes = codec::value_bytes(&v).unwrap_or_else(|e| panic!("{v:?}: {e:?}"));
            assert_eq!(bytes, [22, i as u8, j as u8]);
            let back = codec::value_from_bytes(&bytes).unwrap_or_else(|e| panic!("{v:?}: {e}"));
            assert_eq!(format!("{v:?}"), format!("{back:?}"));
            assert!(codec::same_value(&v, &back));
            assert!(seen.insert(bytes), "{v:?} shares an encoding");
        }
        let mut e = Enc::new();
        codec::encode_ty(&Ty::EndTest(test), &mut e).unwrap();
        assert_eq!(e.bytes, [24, i as u8]);
        let mut d = Dec::new(&e.bytes);
        assert_eq!(codec::decode_ty(&mut d).unwrap(), Ty::EndTest(test));
        d.end().unwrap();
    }
    let decoded = |bytes: &[u8]| codec::value_from_bytes(bytes).map(|_| ()).map_err(|m| m.to_string());
    assert_eq!(decoded(&[22, 3, 0]).unwrap_err(), "an unknown end test");
    assert_eq!(decoded(&[22, 0, 3]).unwrap_err(), "an unknown end-test outcome");
    assert!(decoded(&[22, 0]).is_err(), "a verdict without its outcome");
    assert!(decoded(&[22, 0, 0, 0]).is_err(), "a byte after the verdict");
    assert!(codec::decode_ty(&mut Dec::new(&[24, 3])).is_err(), "an unknown end test's verdict type");
    assert!(codec::decode_ty(&mut Dec::new(&[24])).is_err(), "a verdict type without its test");
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
    assert!(decoded(&|e| e.u8(u8::MAX)).is_err());
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
