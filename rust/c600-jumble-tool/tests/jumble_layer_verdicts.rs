//! The rest of Jumble on in the command layer, taken up by C8 (LP3 plan section 8; LP2's pull requests 212, 217, 218,
//! 220 and 254): `jumble.off`, the refusals with Jumble on, redo, protected pieces and free mode, each beside the
//! jumbling tool's own library entries run at the layer's head through the host bridge (`Rows::at_head`). Where the
//! layer decides on a fact the tool reads, the tool's answer at the head predicts the layer's decision.
//!
//! - `jumble.off` and the label checkpoint (`jumble_end_tests::label_checkpoint`, with its row's evidence). The
//!   checkpoint holds exactly when the exit commits, and the retained state then has the checkpoint's labels digest; it
//!   fails exactly when the exit is refused with its fact's code (a piece off the lattice
//!   `KN5:input.jumble_off_pieces_off_lattice`, a nonzero ν `KN5:input.jumble_off_nu_nonzero`, any other fact
//!   `KN5:input.jumble_off_not_shown_reachable`); and it is unestablished exactly when the exit is refused as not shown
//!   reachable. The walks reach ν = 0 shown by each of its three sources: the identity labels at the entry, a retained
//!   witness after retained items pressed as pose moves, and the ν row after a jumble item and its exact inverse. An
//!   exit's undo returns to poses where the tool reads the same, its redo reaches the same labels, and a second exit is
//!   refused with the code that every read of the tool then stops with, `KN5:input.menu_word_needs_jumble_on`.
//! - `jumble.off` and A5's crossing prerequisites (`jumble_crossing::jumble_off_facts`, LB3.1's "Mode crossing
//!   prerequisites", the Jumble-off half). At each exit the entry's label checkpoint is the row's verdict, its count of
//!   pieces off the lattice is the head's, and its ν facts are the `nu` row's; a committed exit has every piece on the
//!   lattice and ν = 0, an exit refused for pieces off the lattice has some off, and one refused for a nonzero ν has
//!   every piece on the lattice and a nonzero ν. A walk of I_a reaches the nonzero ν: the retained word [1, 3, −5] and
//!   correction 2 of LP2's catalogue pressed as pose moves.
//! - A press and the twist check (`jumble_admissibility::twist_check`). An admissible twist commits and moves exactly
//!   the count of pieces the check names; a blocked twist is refused with the code that `word_protection` stops with
//!   for the same letter.
//! - Protected pieces and `word_protection` of the pressed letter, with the pieces that carry the protected orbits and
//!   the layer's policy, Net or Strict. A verdict `moves` is a refusal with `KN2:protection.piece_would_move`, `clear`
//!   a commit, and a stopped run a refusal with the run's code.
//! - Redo: each redone head, the entry's included, has the digest and the tool's reads of the head it brings back, and
//!   with the entry undone every read stops.
//! - Free mode: a free binding is refused with `KN2:input.menu_binding_mismatch`, since the session pins a menu (free
//!   mode is research-only, JI.35).
//!
//! Every refusal changes nothing the layer keeps (the revision, the journal, the policy, the poses and the retained
//! state) and nothing the tool reads at the head. LP2's `c600-command/tests/jumble_*.rs` and `attack_protection.rs`
//! check the layer's own order of checks, its codes and its journal; this file checks that the tool's verdicts and the
//! layer's decisions agree on W-J fixture walks. A pass is source and fixture evidence. CT-LB3-07 (part: LB3.1's "Mode
//! crossing prerequisites", the Jumble-off half, in `the_label_checkpoint_the_tool_reads_decides_the_exit`; not run
//! here: the Jumble-on half, in `jumble_on_derivation.rs`)

mod common;

use std::collections::BTreeSet;
use std::sync::OnceLock;

use c600_command::layer::{JUMBLE_ON_MODE, MODE};
use c600_command::{code, Direction, TWIST_ROW};
use c600_jumble::codes::{MENU_WORD_NEEDS_JUMBLE_ON, PIECE_WOULD_MOVE};
use c600_jumble::q5::transpose;
use c600_jumble::twist::Twist;
use c600_jumble::{corrections, levels};
use c600_jumble_tool::library::{self, ADMISSIBILITY, CROSSING, END_TESTS, PROTECTION};
use c600_jumble_tool::rows::Rows;
use c600_jumble_tool::word::Alphabet;
use c600_kernel::json::{self, Value};
use c600_kernel::request::codes::MENU_BINDING_MISMATCH;
use c600_kernel::request::{self as wire, Binding, Operation};
use c600_run::encode::encode;
use c600_run::interp::{load, Loaded, Options};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::{Amounts, Outcome};
use c600_run::value::Value as RunValue;

use common::{committed, completed, engine, moved, Fixture, Session};

/// Run limits wide enough for every run here.
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The library's modules, lowered together, checked with no diagnostic and loaded once.
fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        let program = library::program(&[]).expect("the library lowers");
        let report = c600_check::check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        load(&program).expect("the library loads")
    })
}

/// `fixture`'s menu as the pinned alphabet of a run's menu words.
fn pinned(fixture: &Fixture) -> MenuAlphabet {
    Alphabet::of(engine(), &fixture.menu()).expect("an inverse-closed menu").pinned().expect("pinned")
}

/// What a run of a library entry gave: its outcome, its code and its value, read back from canonical bytes as JSON.
#[derive(Debug, PartialEq)]
struct Ran {
    outcome: Outcome,
    code: Option<&'static str>,
    value: Option<Value>,
}

/// Entry `entry` of `module` with `args`, run at `session`'s head through the bridge.
fn run(session: &Session, pinned: &MenuAlphabet, module: &'static str, entry: &str, args: Vec<RunValue>) -> Ran {
    let mut host = Rows::at_head(engine(), &session.layer);
    let options = Options { menu: Some(pinned), ..Options::new(WIDE) };
    let run = loaded().run(module, entry, args, &mut host, &options).expect("the run starts");
    let value = run.value.as_ref().map(|value| {
        let mut out = Vec::new();
        encode(value, &mut out).expect("a canonical value");
        json::parse(&out).expect("canonical bytes are JSON")
    });
    Ran { outcome: run.summary.outcome, code: run.summary.code, value }
}

/// The text field `name` of `value`.
fn text<'v>(value: &'v Value, name: &str) -> &'v str {
    value.get(name).and_then(Value::as_str).unwrap_or_else(|| panic!("a text field {name} in {value:?}"))
}

/// The natural field `name` of `value`, which canonical bytes write `{"int": "<digits>"}`.
fn nat(value: &Value, name: &str) -> i64 {
    let digits = value.get(name).and_then(|n| n.get("int")).and_then(Value::as_str);
    digits.and_then(|digits| digits.parse().ok()).unwrap_or_else(|| panic!("a natural field {name} in {value:?}"))
}

/// The one-letter menu word of `twist`.
fn letter(session: &mut Session, pinned: &MenuAlphabet, twist: &Twist) -> RunValue {
    let letter = session.letter(twist);
    RunValue::menu_word(pinned.shared_identity(), &[letter]).expect("a letter below the index limit")
}

/// What the tool reads at the head and a refusal may not change: the revision's digest, the label checkpoint's row and
/// the heights row.
fn reads(session: &Session) -> (String, Result<Value, &'static str>, Result<Value, &'static str>) {
    let mut rows = Rows::at_head(engine(), &session.layer);
    let digest = session.layer.revision().digest.clone();
    (digest, rows.answer("label_checkpoint", None), rows.answer("heights", None))
}

/// The code of a refused exit for the label checkpoint's fact `fact` (the layer's draft: `c600-command`'s
/// `jumble.off`).
fn exit_code(fact: &str) -> &'static str {
    match fact {
        "off_lattice" => code::JUMBLE_OFF_PIECES_OFF_LATTICE,
        "nu_nonzero" => code::JUMBLE_OFF_NU_NONZERO,
        _ => code::JUMBLE_OFF_NOT_SHOWN_REACHABLE,
    }
}

/// A5's crossing prerequisites as `jumble_crossing::jumble_off_facts` answers them at the head, each checked against
/// what the layer and the bridge hold there: the label checkpoint's outcome, the count of pieces off the lattice, and
/// whether ν is defined with the count of its nonzero values.
fn crossing_facts(session: &Session, pinned: &MenuAlphabet, outcome: &str, what: &str) -> (i64, bool, i64) {
    let ran = run(session, pinned, CROSSING.name, "jumble_off_facts", vec![]);
    assert_eq!((ran.outcome, ran.code), (Outcome::Completed, None), "{what}: the crossing entry completes");
    let facts = ran.value.expect("a record");
    let checkpoint = facts.get("label_checkpoint").expect("the label checkpoint");
    assert_eq!(text(checkpoint, "outcome"), outcome, "{what}: the crossing entry's checkpoint is the row's verdict");
    let off = nat(&facts, "off_lattice");
    assert_eq!(off, session.state().off_lattice_count() as i64, "{what}: the pieces off the lattice");
    let nu = Rows::at_head(engine(), &session.layer).answer("nu", None).expect("the ν row answers");
    let values: Option<Vec<i64>> = nu
        .get("values")
        .and_then(Value::as_array)
        .map(|values| values.iter().map(|v| v.as_i64().expect("a ν value")).collect());
    let defined = facts.get("nu_defined") == Some(&Value::Bool(true));
    assert_eq!(defined, values.is_some(), "{what}: ν is defined as the row says");
    let nonzero = nat(&facts, "nu_nonzero");
    let counted = values.map_or(0, |values| values.iter().filter(|&&v| v != 0).count());
    assert_eq!(nonzero, counted as i64, "{what}: the nonzero values of the row's ν");
    (off, defined, nonzero)
}

/// `jumble.off` at the head beside the label checkpoint and the crossing prerequisites the tool reads there. Returns
/// the case met: the outcome and, for a checkpoint that holds, how ν = 0 was shown, for one that fails, its fact.
fn exit_at_head(session: &mut Session, pinned: &MenuAlphabet, what: &str) -> String {
    let at = reads(session);
    let row = at.1.clone().expect("the label checkpoint's row answers with Jumble on");
    let ran = run(session, pinned, END_TESTS.name, "label_checkpoint", vec![]);
    let outcome = match text(&row, "verdict") {
        "yes" => "holds",
        "no" => "fails",
        _ => "unestablished",
    };
    assert_eq!((ran.outcome, ran.code), (Outcome::Completed, None), "{what}: the entry completes");
    let value = ran.value.expect("a verdict");
    assert_eq!(text(&value, "outcome"), outcome, "{what}: the entry's outcome is the row's verdict");
    let (off, nu_defined, nu_nonzero) = crossing_facts(session, pinned, outcome, what);
    let before = session.fixed();
    let receipt = session.off();
    match receipt.status.code() {
        None => assert!(off == 0 && nu_defined && nu_nonzero == 0, "{what}: an exit with every prerequisite"),
        Some(code::JUMBLE_OFF_PIECES_OFF_LATTICE) => assert!(off > 0, "{what}: pieces off the lattice"),
        Some(code::JUMBLE_OFF_NU_NONZERO) => assert!(off == 0 && nu_nonzero > 0, "{what}: a nonzero ν on the lattice"),
        Some(_) => assert_eq!(off, 0, "{what}: refused with every piece on the lattice"),
    }
    match outcome {
        "holds" => {
            assert!(committed(&receipt), "{what}: the checkpoint holds, so the exit commits: {:?}", receipt.status);
            let labels = text(&row, "labels_digest");
            assert_eq!(session.layer.snapshot().digest(), labels, "{what}: the retained state has the labels");
            let r = session.layer.revision();
            assert_eq!((r.mode, r.digest.as_str()), (MODE, labels), "{what}: Jumble off at the labels");
            let off = run(session, pinned, END_TESTS.name, "label_checkpoint", vec![]);
            assert_eq!((off.outcome, off.code), (Outcome::Error, Some(MENU_WORD_NEEDS_JUMBLE_ON)), "{what}");
            let after = session.fixed();
            let again = session.off();
            assert_eq!(again.status.code(), Some(MENU_WORD_NEEDS_JUMBLE_ON), "{what}: a second exit");
            assert_eq!(session.fixed(), after, "{what}: the refused second exit changes nothing");

            assert!(committed(&session.undo()), "{what}: the exit's undo");
            assert_eq!(session.layer.revision().mode, JUMBLE_ON_MODE, "{what}: the undo is Jumble on again");
            assert_eq!(reads(session), at, "{what}: the undo returns to poses where the tool reads the same");
            assert!(committed(&session.redo()), "{what}: the exit's redo");
            assert_eq!(session.layer.snapshot().digest(), labels, "{what}: the redo reaches the same labels");
            assert!(committed(&session.undo()), "{what}: the exit's undo again");
            assert_eq!(reads(session), at, "{what}: and the poses again");
            format!("holds: {}", text(&row, "nu_source"))
        }
        "fails" => {
            let fact = text(&row, "fact");
            assert_eq!(receipt.status.code(), Some(exit_code(fact)), "{what}: the exit is refused with its fact");
            assert_eq!(session.fixed(), before, "{what}: the refused exit changes nothing");
            assert_eq!(reads(session), at, "{what}: nor what the tool reads");
            format!("fails: {fact}")
        }
        _ => {
            assert_eq!(receipt.status.code(), Some(code::JUMBLE_OFF_NOT_SHOWN_REACHABLE), "{what}");
            assert_eq!(session.fixed(), before, "{what}: the refused exit changes nothing");
            assert_eq!(reads(session), at, "{what}: nor what the tool reads");
            "unestablished".to_owned()
        }
    }
}

/// The items of `fixture`'s menu at `grip`, in item order.
fn items(fixture: &Fixture, grip: usize) -> Vec<Twist> {
    fixture.menu().for_cap(engine(), grip).expect("the menu's items")
}

/// The item at `twist`'s grip that is its exact inverse; the pinned menus are inverse-closed.
fn inverse(fixture: &Fixture, twist: &Twist) -> Twist {
    let matrix = transpose(&twist.matrix);
    items(fixture, twist.grip).into_iter().find(|item| *item.matrix == matrix).expect("the inverse is an item")
}

/// CT-JI-01's and A5's exit through the command layer, beside the tool's label checkpoint (C8's take-up of LP2's
/// pull requests 217 and 218). On the I_a, I_b and S4 fixtures, `jumble.off` is pressed at the entry and after each of
/// the first records of the journal, so at solved and off the lattice. On I_a it is pressed also after retained items
/// 1 and 6 at cap 0 pressed as pose moves, and after retained item 1, jumble item 12 at cap 0 and its exact inverse,
/// which return to poses that neither the identity labels nor a retained witness show. At each head the tool's
/// checkpoint decides the exit as the module documentation says.
#[test]
fn the_label_checkpoint_the_tool_reads_decides_the_exit() {
    let mut met = BTreeSet::new();
    for (name, records) in [("I_a", 6), ("I_b", 3), ("S4", 6)] {
        let fixture = Fixture::load(name);
        let pinned = pinned(&fixture);
        let mut session = Session::entered(&fixture);
        for step in 0..=records {
            met.insert(exit_at_head(&mut session, &pinned, &format!("{name} after {step} records")));
            if step < records {
                assert!(committed(&session.press(&fixture.twists[step])), "{name}: record {step}");
            }
        }
    }

    let fixture = Fixture::load("I_a");
    let pinned = pinned(&fixture);
    let at_zero = items(&fixture, 0);
    let mut session = Session::entered(&fixture);
    for index in [1, 6] {
        assert!(committed(&session.press(&at_zero[index])), "retained item {index} as a pose move");
        met.insert(exit_at_head(&mut session, &pinned, &format!("after retained item {index}")));
    }
    let mut session = Session::entered(&fixture);
    assert!(committed(&session.press(&at_zero[1])), "retained item 1 as a pose move");
    let jumble = &at_zero[12];
    assert!(committed(&session.press(jumble)), "jumble item 12");
    met.insert(exit_at_head(&mut session, &pinned, "after retained item 1 and jumble item 12"));
    assert!(committed(&session.press(&inverse(&fixture, jumble))), "its exact inverse");
    met.insert(exit_at_head(&mut session, &pinned, "after retained item 1, jumble item 12 and its inverse"));

    // The retained word [1, 3, −5] and correction 2 of LP2's catalogue, pressed as pose moves: a lattice
    // configuration whose ν, which the ν row computes, is correction 2's row (b34 = 1).
    let retained = [1, 3, -5].map(|id| levels::primitive(engine(), id).expect("a retained generator"));
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let correction = catalogue.twists(engine(), &fixture.menu(), 2).expect("no engine error").expect("the word reads");
    let mut session = Session::entered(&fixture);
    for (index, twist) in retained.iter().chain(&correction).enumerate() {
        assert!(committed(&session.press(twist)), "letter {index} of the retained word and correction 2");
    }
    met.insert(exit_at_head(&mut session, &pinned, "after the retained word and correction 2"));

    let cases = ["holds: identity_labels", "holds: retained_witness", "holds: supplied", "fails: off_lattice"];
    for case in cases.into_iter().chain(["fails: nu_nonzero"]) {
        assert!(met.contains(case), "{case} is met: {met:?}");
    }
}

/// The twist check of `twist` at the head: its status and the count of pieces it would move.
fn checked(session: &mut Session, pinned: &MenuAlphabet, twist: &Twist) -> (String, i64) {
    let word = letter(session, pinned, twist);
    let ran = run(session, pinned, ADMISSIBILITY.name, "twist_check", vec![word]);
    assert_eq!((ran.outcome, ran.code), (Outcome::Completed, None), "the twist check completes");
    let value = ran.value.expect("a record");
    (text(&value, "status").to_owned(), nat(&value, "moved"))
}

/// An item at the first grip, from `from` on, that the grip status names blocked at the head, and that grip's status.
fn blocked(session: &mut Session, pinned: &MenuAlphabet, fixture: &Fixture, from: usize) -> Twist {
    for grip in from..600 {
        let ran = run(session, pinned, ADMISSIBILITY.name, "grip_status", vec![RunValue::int(grip as u32)]);
        let value = ran.value.expect("a grip status");
        if text(&value, "status") == "blocked" {
            return items(fixture, grip).swap_remove(1);
        }
    }
    panic!("no blocked grip at the head")
}

/// `word_protection` of `twist`'s letter at the head with `pieces` protected, under `policy`: the verdict's record, or
/// the code the run stops with.
fn protection(
    session: &mut Session,
    pinned: &MenuAlphabet,
    twist: &Twist,
    pieces: &[u32],
    policy: &str,
) -> Result<Value, &'static str> {
    let word = letter(session, pinned, twist);
    let pieces = RunValue::list(pieces.iter().map(|&piece| RunValue::int(piece)).collect());
    let ran = run(session, pinned, PROTECTION.name, "word_protection", vec![word, pieces]);
    match ran.outcome {
        Outcome::Completed => Ok(ran.value.and_then(|value| value.get(policy).cloned()).expect("the policy's verdict")),
        _ => Err(ran.code.expect("a stopped run has a code")),
    }
}

/// A press beside the tool's twist check, and the refusals with Jumble on beside what the tool reads (C8's take-up of
/// LP2's pull requests 212 and 220). On the I_a fixture each of the first eight records is admissible by the check and
/// moves the count of pieces it names. At the head that follows, each refusal changes nothing the layer keeps or the
/// tool reads: `jumble.on` and `twist.apply` (`KN5:input.retained_tool_needs_jumble_off`), an item at a grip the tool's
/// grip status names blocked, which the check names blocked and the layer refuses with the code that
/// `word_protection` stops with, and a press prepared before the next record's commit
/// (`KN5:stale.revision_changed`).
#[test]
fn the_twist_check_predicts_each_press_and_each_refusal_changes_nothing() {
    let fixture = Fixture::load("I_a");
    let pinned = pinned(&fixture);
    let mut session = Session::entered(&fixture);
    for (index, twist) in fixture.twists[..8].iter().enumerate() {
        let (status, count) = checked(&mut session, &pinned, twist);
        assert_eq!(status, "admissible", "record {index}");
        let before = session.state().clone();
        assert!(committed(&session.press(twist)), "record {index}");
        assert_eq!(moved(&before, session.state()).len() as i64, count, "record {index}: the pieces it moves");
    }

    let (at, before) = (reads(&session), session.fixed());
    let refused = |session: &Session, what: &str| {
        assert_eq!(session.fixed(), before, "{what}: the refusal changes nothing the layer keeps");
        assert_eq!(reads(session), at, "{what}: nor what the tool reads");
    };
    let receipt = session.on();
    assert_eq!(receipt.status.code(), Some(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF), "jumble.on with Jumble on");
    refused(&session, "jumble.on");
    let Value::Object(mut fields) = session.binding(&items(&fixture, 0)[1]).to_json() else { panic!("an object") };
    let cap = fields.remove("cap").expect("a cap");
    let receipt = session.invoke(TWIST_ROW, json::object([("cap", cap), ("rotation", Value::Object(fields))]));
    assert_eq!(receipt.status.code(), Some(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF), "twist.apply with Jumble on");
    refused(&session, "twist.apply");

    let stuck = blocked(&mut session, &pinned, &fixture, 0);
    let (status, count) = checked(&mut session, &pinned, &stuck);
    assert_eq!((status.as_str(), count), ("blocked", 0), "the check names the item blocked");
    let stopped = protection(&mut session, &pinned, &stuck, &[], "net").expect_err("the word stops");
    let receipt = session.press(&stuck);
    assert_eq!(receipt.status.code(), Some(stopped), "the layer refuses the blocked item with the tool's code");
    refused(&session, "a blocked item");

    let next = fixture.twists[8].clone();
    let binding = session.binding(&next);
    let stale = session.prepare(&binding);
    assert!(committed(&session.press(&next)), "record 8");
    let (at, before) = (reads(&session), session.fixed());
    let receipt = session.send(stale);
    assert_eq!(receipt.status.code(), Some(code::REVISION_CHANGED), "a press from before record 8");
    assert_eq!((session.fixed(), reads(&session)), (before, at), "the stale press changes nothing");
}

/// The pieces of the jumbling model that carry the 600-cell-Full orbits `orbits`, ascending, as the layer protects
/// them inside Jumble (LP2's pull request 254).
fn pieces_of(orbits: &[i64]) -> Vec<u32> {
    let orbit_id = &engine().model.orbit_id;
    let carries = |piece: &usize| orbits.contains(&i64::from(orbit_id[*piece]));
    (0..orbit_id.len()).filter(carries).map(|piece| piece as u32).collect()
}

/// Protected pieces inside Jumble beside the tool's `word_protection` (C8's take-up of LP2's pull request 254). Under
/// Net and under Strict, a session protecting orbit 0 enters Jumble on, and each of the first four records of the I_a
/// journal is pressed beside the tool's verdict on its letter with orbit 0's pieces and the layer's policy. Then
/// orbit 0 is released with its confirmation and the walk goes on unprotected for four records; orbit 0 is protected
/// again and the next four records and an item at a blocked grip are pressed beside the verdict. A verdict `moves` is
/// refused with `KN2:protection.piece_would_move` and changes nothing; `clear` commits; a stopped run is refused with
/// its code. Each case is met under each policy.
#[test]
fn protected_pieces_refuse_the_pose_moves_the_tool_finds_moving() {
    let fixture = Fixture::load("I_a");
    let pinned = pinned(&fixture);
    let orbit = pieces_of(&[0]);
    assert!(!orbit.is_empty(), "orbit 0 has pieces in the jumbling model");
    for policy in ["net", "strict"] {
        let mut session = Session::entered_protecting(&fixture, &[0]);
        if policy == "strict" {
            assert!(completed(&session.set_policy("strict")), "the Strict policy");
        }
        let mut met = BTreeSet::new();
        let mut press = |session: &mut Session, twist: &Twist, pieces: &[u32], what: &str| {
            let verdict = protection(session, &pinned, twist, pieces, policy);
            let (at, before) = (reads(session), session.fixed());
            let receipt = session.press(twist);
            let case = match &verdict {
                Ok(verdict) if text(verdict, "verdict") == "moves" => {
                    assert_eq!(receipt.status.code(), Some(PIECE_WOULD_MOVE), "{policy}, {what}");
                    "moves"
                }
                Ok(verdict) => {
                    assert_eq!(text(verdict, "verdict"), "clear", "{policy}, {what}");
                    assert!(committed(&receipt), "{policy}, {what}: a clear verdict commits: {:?}", receipt.status);
                    met.insert("clear");
                    return;
                }
                Err(code) => {
                    assert_eq!(receipt.status.code(), Some(*code), "{policy}, {what}");
                    "stopped"
                }
            };
            assert_eq!(
                (session.fixed(), reads(session)),
                (before, at),
                "{policy}, {what}: the refusal changes nothing"
            );
            met.insert(case);
        };
        for (index, twist) in fixture.twists[..4].iter().enumerate() {
            press(&mut session, twist, &orbit, &format!("record {index}, orbit 0 protected"));
        }
        assert!(completed(&session.protect(&[], true)), "{policy}: orbit 0's confirmed release");
        for (index, twist) in fixture.twists[..4].iter().enumerate() {
            press(&mut session, twist, &[], &format!("record {index}, nothing protected"));
        }
        assert!(completed(&session.protect(&[0], false)), "{policy}: orbit 0 protected again");
        for (index, twist) in fixture.twists[4..8].iter().enumerate() {
            press(&mut session, twist, &orbit, &format!("record {}, orbit 0 protected", index + 4));
        }
        let stuck = blocked(&mut session, &pinned, &fixture, 0);
        press(&mut session, &stuck, &orbit, "an item at a blocked grip");
        for case in ["moves", "clear", "stopped"] {
            assert!(met.contains(case), "{policy}: {case} is met: {met:?}");
        }
    }
}

/// Redo through the command layer beside what the tool reads (C8's take-up of LP2's pull requests 212 and 220). On the
/// I_a fixture the first twelve records are pressed, then undone one by one back to the entry, each undo reaching the
/// head the tool read before the press; the entry is undone too, and every read of the tool stops with
/// `KN5:input.menu_word_needs_jumble_on`. Then the entry and the twelve records are redone: each redo commits, advances
/// the state revision by one and brings back the digest and the tool's reads of its head, and the redo stack ends
/// empty.
#[test]
fn redo_brings_back_each_head_and_what_the_tool_reads_there() {
    let fixture = Fixture::load("I_a");
    let mut session = Session::entered(&fixture);
    let mut heads = vec![reads(&session)];
    for (index, twist) in fixture.twists[..12].iter().enumerate() {
        assert!(committed(&session.press(twist)), "record {index}");
        heads.push(reads(&session));
    }
    for index in (0..12).rev() {
        assert!(committed(&session.undo()), "the undo of record {index}");
        assert_eq!(reads(&session), heads[index], "the undo of record {index}");
    }
    assert!(committed(&session.undo()), "the entry's undo");
    assert!(session.layer.jumbled().is_none(), "Jumble off before the entry");
    let (_, checkpoint, heights) = reads(&session);
    assert_eq!((checkpoint, heights), (Err(MENU_WORD_NEEDS_JUMBLE_ON), Err(MENU_WORD_NEEDS_JUMBLE_ON)));
    for (index, head) in heads.iter().enumerate() {
        let state = session.layer.revision().state;
        assert!(committed(&session.redo()), "redo {index}");
        assert_eq!(session.layer.revision().state, state + 1, "redo {index}: one state revision");
        assert_eq!(&reads(&session), head, "redo {index}");
    }
    assert!(session.layer.undo_hint(Direction::Redo).is_none(), "the redo stack is empty");
}

/// Free mode through the command layer (C8's take-up of LP2's pull request 220; JI.35): at the entry and after each of
/// the first three records of the I_a journal, a free binding at the next record's grip is refused with
/// `KN2:input.menu_binding_mismatch` and changes nothing the layer keeps or the tool reads, and the record's own press
/// then commits.
#[test]
fn a_free_binding_is_refused_and_the_walk_goes_on() {
    let fixture = Fixture::load("I_a");
    let mut session = Session::entered(&fixture);
    for (index, twist) in fixture.twists[..3].iter().enumerate() {
        let free = Binding {
            request: wire::cayley_request(["0:0", "0:0", "0:0"]),
            cap: twist.grip as u32,
            operation: Operation::Free { n: 1, d: 1 },
            menu: None,
        };
        let (at, before) = (reads(&session), session.fixed());
        let press = session.prepare(&free);
        let receipt = session.send(press);
        assert_eq!(receipt.status.code(), Some(MENU_BINDING_MISMATCH), "a free binding before record {index}");
        assert_eq!((session.fixed(), reads(&session)), (before, at), "before record {index}: nothing changes");
        assert!(committed(&session.press(twist)), "record {index}");
    }
}
