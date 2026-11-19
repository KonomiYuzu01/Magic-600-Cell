//! The jumbling rows' host bridge at the command layer's head (LP3, C8; `c600_jumble_tool::rows`): the library entries
//! of CT-LB1-03, CT-LB3-03, CT-LB3-04 and CT-LB3-06 run through `Rows::at_head` on sessions of the jumbled puzzle
//! through the command layer with Jumble on, as `tests/common` drives them: `jumble.on` at solved under a W-J fixture's
//! pinned menu, then one `jumble.twist.menu` press for each of the fixture's first journal records.
//!
//! At each head, each entry run through the bridge at the head equals its run through the bridge on the same state,
//! witnessed apart from the layer by the same records (`Rows::on`), byte for byte: outcome, code, the value's canonical
//! bytes, the rows read and the drafted sets checked. `tests/library.rs` and `tests/ct_lb3_04.rs` compare the entries
//! run on such snapshots with `c600_jumble`'s answers, so the head serves the kernel's answers on the head's state. The
//! heads are I_a and S4 after 0, 1 and 12 records and I_b after 1. The words are the fixture's own next 20 records, as
//! menu words of its alphabet; the step flag's unit is the head's first exact-pose group off the lattice, and at
//! solved, where there is none, the row refuses piece 0 as a unit (`KN2:input.named_object_invalid`). The corrections
//! are words of the I_a menu: under S4 and I_b `nu` answers and the drafted check refuses the menu
//! (`KN2:input.menu_binding_mismatch`); under I_a an unshipped id stops the drafted check
//! (`KN2:input.named_object_invalid`).
//!
//! With Jumble off, after `jumble.off`, every read stops with `KN5:input.menu_word_needs_jumble_on`. The bridge grants
//! no preview, clock or other row (`LG3:capability.not_granted`), refuses a row read in another argument form with
//! `KN5:input.argument_form` at its tariff, and its draft tariff counts one pass of the model's pieces for the state
//! and one for each letter of a named word or entry of a drafted set, two for a named cap's seam facts.
//!
//! The `seam_facts` row, which CT-RTE-07's jumbling keys read (`tests/ct_rte_07_jumble.rs`), answers for the cap it is
//! named: at an admissible cap its N_c equals the engine's rim summary for that cap, a blocked cap has no totals, and a
//! cap outside the model is `KN2:input.named_object_invalid`. A pass is source and fixture evidence.

mod common;

use std::sync::OnceLock;

use c600_command::code::ARGUMENT_FORM;
use c600_ir::{QTy, Ty};
use c600_jumble::codes::{MENU_BINDING_MISMATCH, MENU_WORD_NEEDS_JUMBLE_ON, NAMED_OBJECT_INVALID};
use c600_jumble::levels::{Base, Snapshot};
use c600_jumble::model::NP;
use c600_jumble::observe::cuts::rim_summary;
use c600_jumble::observe::{seam_facts, Batch, Rows as Cells, Scope};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Twist;
use c600_jumble_tool::library::{self, HEIGHTS, NU, PROTECTION, WORD_CHECKS};
use c600_jumble_tool::rows::{Rows, RESULT_BOUND};
use c600_jumble_tool::word::{Alphabet, LiteralWord};
use c600_kernel::json::Value as JsonValue;
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::value::Value as RunValue;

use common::{committed, engine, Fixture, Session};

/// Run limits wide enough for every run here.
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The library's six modules, lowered together, checked with no diagnostic and loaded once.
fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        let program = library::program(&[]).expect("the library lowers");
        let report = c600_check::check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        load(&program).expect("the library loads")
    })
}

/// One entry with its arguments.
struct Call {
    module: &'static str,
    entry: &'static str,
    args: Vec<RunValue>,
}

/// What a run did, comparably.
#[derive(Debug, PartialEq)]
struct Seen {
    outcome: Outcome,
    code: Option<&'static str>,
    value: Option<Vec<u8>>,
    asked: Vec<String>,
    drafts: Vec<Vec<(u32, i64)>>,
}

/// `call` run against `host` with `pinned` for the run's menu words.
fn seen(call: &Call, mut host: Rows<'_>, pinned: &MenuAlphabet) -> Seen {
    let options = Options { menu: Some(pinned), ..Options::new(WIDE) };
    let run = loaded().run(call.module, call.entry, call.args.clone(), &mut host, &options).expect("the run starts");
    let value = run.value.as_ref().map(|value| {
        let mut out = Vec::new();
        encode(value, &mut out).expect("a canonical value");
        out
    });
    Seen {
        outcome: run.summary.outcome,
        code: run.summary.code,
        value,
        asked: host.asked().to_vec(),
        drafts: host.drafts().to_vec(),
    }
}

/// `twists`, letters of `fixture`'s pinned menu, as `[cap, item]` pairs: each item as `common::Session` presses it.
fn pairs(fixture: &Fixture, twists: &[Twist]) -> Vec<[i32; 2]> {
    let menu = fixture.menu();
    let pair = |twist: &Twist| {
        let items = menu.for_cap(engine(), twist.grip).expect("the menu's items");
        let item = items.iter().position(|item| item.matrix == twist.matrix).expect("an item of the pinned menu");
        [twist.grip as i32, item as i32]
    };
    twists.iter().map(pair).collect()
}

fn menu_word(pinned: &MenuAlphabet, word: &LiteralWord) -> RunValue {
    let letters: Vec<[u32; 2]> = word.pairs().iter().map(|&[cap, item]| [cap as u32, item as u32]).collect();
    RunValue::menu_word(pinned.shared_identity(), &letters).expect("letters below the index limit")
}

fn naturals(list: &[u32]) -> RunValue {
    RunValue::list(list.iter().map(|&n| RunValue::int(n)).collect())
}

fn draft(set: &[(u32, i64)]) -> RunValue {
    let entry = |&(id, m): &(u32, i64)| RunValue::record(vec![("id", RunValue::int(id)), ("m", RunValue::int(m))]);
    RunValue::list(set.iter().map(entry).collect())
}

/// The exact-pose group of the first piece off the lattice at `state`, a named unit for CT-LB3-06's step flag, or
/// piece 0 when every piece is on the lattice, which the row refuses as a unit (JI.45).
fn unit(state: &State) -> Vec<u32> {
    let Some(first) = (0..NP).find(|&p| !state.is_lattice_piece(p)) else { return vec![0] };
    let pose = state.pose_ids()[first];
    (0..NP).filter(|&p| !state.is_lattice_piece(p) && state.pose_ids()[p] == pose).map(|p| p as u32).collect()
}

/// The calls run at each head, each with a label: CT-LB3-03's heights entry, CT-LB1-03's application and CT-LB3-06's
/// step flag with `unit` and protection on `word`, and CT-LB3-04's ν entries with drafted sets, an unshipped id among
/// them.
fn calls(pinned: &MenuAlphabet, word: &LiteralWord, unit: &[u32]) -> Vec<(&'static str, Call)> {
    let w = menu_word(pinned, word);
    let call = |module: &'static str, entry: &'static str, args: Vec<RunValue>| Call { module, entry, args };
    vec![
        ("heights", call(HEIGHTS.name, "height_facts", vec![])),
        ("application", call(WORD_CHECKS.name, "word_observations", vec![w.clone()])),
        ("step flag", call(WORD_CHECKS.name, "step_flag", vec![w.clone(), naturals(unit)])),
        ("protection, none", call(PROTECTION.name, "word_protection", vec![w.clone(), naturals(&[])])),
        ("protection, four", call(PROTECTION.name, "word_protection", vec![w, naturals(&[0, 7, 11, 1200])])),
        ("ν", call(NU.name, "nu_facts", vec![])),
        ("ν, empty draft", call(NU.name, "drafted_check", vec![draft(&[])])),
        ("ν, two entries", call(NU.name, "drafted_check", vec![draft(&[(2, 1), (3, -1)])])),
        ("ν, unshipped id", call(NU.name, "drafted_check", vec![draft(&[(2, 1), (36, 1)])])),
    ]
}

/// The state after `fixture`'s first `count` records from solved, witnessed by them apart from any layer.
fn witnessed(fixture: &Fixture, count: usize) -> Snapshot {
    let journal = &fixture.twists[..count];
    let mut state = State::new(Some(std::sync::Arc::new(fixture.menu())));
    for (index, twist) in journal.iter().enumerate() {
        let status = state.apply(engine(), twist, None, &[]).expect("no engine error").status;
        assert_eq!(status, Status::Admissible, "{}: record {index}", fixture.name);
    }
    Snapshot::witnessed(engine(), state, journal, None).expect("no engine error").expect("the journal witnesses it")
}

/// CT-LB1-03, CT-LB3-03, CT-LB3-04 and CT-LB3-06 (part: the library entries through the host bridge at the command
/// layer's head with Jumble on).
#[test]
fn the_library_entries_read_the_layers_head_through_the_bridge() {
    let heads: [(&str, &[usize]); 3] = [("I_a", &[0, 1, 12]), ("S4", &[0, 1, 12]), ("I_b", &[1])];
    for (name, counts) in heads {
        let fixture = Fixture::load(name);
        let alphabet = Alphabet::of(engine(), &fixture.menu()).expect("an inverse-closed menu");
        let pinned = alphabet.pinned().expect("the run's menu alphabet");
        let mut session = Session::entered(&fixture);
        let mut pressed = 0;
        for &count in counts {
            while pressed < count {
                assert!(committed(&session.press(&fixture.twists[pressed])), "{name}: record {pressed}");
                pressed += 1;
            }
            let snapshot = witnessed(&fixture, count);
            assert_eq!(session.state().digest(engine()), snapshot.state().digest(engine()), "{name} after {count}");
            let word = alphabet.word(&pairs(&fixture, &fixture.twists[count..count + 20])).expect("a word of the menu");
            let unit = unit(snapshot.state());
            let on_lattice = (0..NP).all(|p| snapshot.state().is_lattice_piece(p));
            for (label, call) in calls(&pinned, &word, &unit) {
                let what = format!("{name} after {count}: {label}");
                let at_head = seen(&call, Rows::at_head(engine(), &session.layer), &pinned);
                let apart = seen(&call, Rows::on(engine(), Base::Snapshot(&snapshot)), &pinned);
                assert_eq!(at_head, apart, "{what}");
                assert!(!at_head.asked.is_empty(), "{what}: the entry reads its rows");
                // A unit holds pieces off the lattice. The corrections are words of the I_a menu: under another menu
                // `nu` answers and the drafted check refuses the menu; under I_a only the unshipped id stops a run.
                let expected = match (name, label.starts_with('ν'), label) {
                    (_, false, "step flag") if on_lattice => Some(NAMED_OBJECT_INVALID),
                    (_, false, _) => None,
                    (_, true, "ν") => None,
                    ("I_a", true, "ν, unshipped id") => Some(NAMED_OBJECT_INVALID),
                    ("I_a", true, _) => None,
                    (_, true, _) => Some(MENU_BINDING_MISMATCH),
                };
                let outcome = if expected.is_some() { Outcome::Error } else { Outcome::Completed };
                assert_eq!((at_head.outcome, at_head.code), (outcome, expected), "{what}");
            }
        }
    }
}

/// The bridge with Jumble off: after `jumble.off`, every read stops with `KN5:input.menu_word_needs_jumble_on`.
#[test]
fn with_jumble_off_every_read_stops() {
    let fixture = Fixture::load("I_a");
    let pinned = Alphabet::of(engine(), &fixture.menu()).expect("an inverse-closed menu").pinned().expect("pinned");
    let mut session = Session::entered(&fixture);
    assert!(committed(&session.off()), "`jumble.off` at solved");
    assert!(session.layer.jumbled().is_none());
    for call in [
        Call { module: HEIGHTS.name, entry: "height_facts", args: vec![] },
        Call { module: NU.name, entry: "nu_facts", args: vec![] },
    ] {
        let at_head = seen(&call, Rows::at_head(engine(), &session.layer), &pinned);
        assert_eq!(
            (at_head.outcome, at_head.code),
            (Outcome::Error, Some(MENU_WORD_NEEDS_JUMBLE_ON)),
            "{}",
            call.entry
        );
        assert_eq!(at_head.value, None);
    }
}

/// What the bridge grants, and its draft tariff.
#[test]
fn the_bridge_grants_its_rows_only_and_prices_each_pass() {
    let fixture = Fixture::load("I_a");
    let solved = witnessed(&fixture, 0);
    let mut rows = Rows::on(engine(), Base::Snapshot(&solved));
    let nat = QTy { ty: Ty::Nat, display: false };
    let no_word = c600_run::provenance::canonical(&c600_run::provenance::Json::List(Vec::new()));
    let pass = NP as u128;

    assert_eq!(rows.tariff(&Request::Clock).map(|t| t.kernel_cost), Err(NOT_GRANTED));
    let propose = Request::Propose { word: &[1, 2], target: "operation.preview", request: b"{}" };
    assert_eq!(rows.tariff(&propose).map(|t| t.kernel_cost), Err(NOT_GRANTED));
    for query in ["positions", "labels", "state", "nu_check"] {
        assert_eq!(rows.tariff(&Request::Snapshot { query, ty: &nat }).map(|t| t.kernel_cost), Err(NOT_GRANTED));
    }
    for query in ["heights", "height_counts", "nu"] {
        let tariff = rows.tariff(&Request::Snapshot { query, ty: &nat }).expect("granted");
        assert_eq!((tariff.kernel_cost, tariff.result_bound), (pass, RESULT_BOUND), "{query}");
        let with = Request::SnapshotWith { query, arg: &no_word, ty: &nat };
        assert_eq!(rows.tariff(&with).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "{query} takes no argument");
    }
    for query in ["replay", "word_check", "protection", "correction_set_check", "seam_facts"] {
        let without = Request::Snapshot { query, ty: &nat };
        assert_eq!(rows.tariff(&without).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "{query} takes an argument");
    }
    let cap = Request::SnapshotWith { query: "seam_facts", arg: br#"{"int":"7"}"#, ty: &nat };
    assert_eq!(rows.tariff(&cap).map(|t| t.kernel_cost), Ok(2 * pass), "the cap's classification and its cut");
    let not_a_cap = Request::SnapshotWith { query: "seam_facts", arg: br#"{"letters":[],"menu":{}}"#, ty: &nat };
    assert_eq!(rows.tariff(&not_a_cap).map(|t| t.kernel_cost), Err(ARGUMENT_FORM));
    let word = br#"{"letters":[[0,1],[2,3],[4,5]],"menu":{}}"#;
    let replay = Request::SnapshotWith { query: "replay", arg: word, ty: &nat };
    assert_eq!(rows.tariff(&replay).map(|t| t.kernel_cost), Ok(4 * pass), "one pass and one for each letter");
    let set = br#"[{"id":{"int":"2"},"m":{"int":"1"}},{"id":{"int":"3"},"m":{"int":"-1"}}]"#;
    let check = Request::SnapshotWith { query: "correction_set_check", arg: set, ty: &nat };
    assert_eq!(rows.tariff(&check).map(|t| t.kernel_cost), Ok(3 * pass), "one pass and one for each entry");
    let malformed = Request::SnapshotWith { query: "replay", arg: br#"{"letters":[[0]]"#, ty: &nat };
    assert_eq!(rows.tariff(&malformed).map(|t| t.kernel_cost), Err(ARGUMENT_FORM));
    assert!(rows.asked().is_empty(), "a tariff reads no row");
}

/// The `seam_facts` row at the I_a walk's state after three records: a misaligned admissible cap and a blocked cap, the
/// first of each in the rim summary's cap order. The row serves the engine's seam facts of the named cap, field by
/// field, with N_c equal to the rim summary's; a blocked cap has no totals, cells or domains, and says why; a cap
/// outside the model is refused (JI.45), and an argument that is not an integer is `KN5:input.argument_form`.
#[test]
fn the_seam_facts_row_answers_for_the_named_cap() {
    let fixture = Fixture::load("I_a");
    let walk = witnessed(&fixture, 3);
    let summary = rim_summary(engine(), &mut walk.state().clone(), None, None).expect("no engine error");
    let caps = summary.get("rows").and_then(JsonValue::as_array).expect("a row for each cap");
    let n_c = |row: &JsonValue| row.get("N_c").and_then(JsonValue::as_i64);
    let blocked = |row: &JsonValue| row.get("blocked") == Some(&JsonValue::Bool(true));
    let misaligned = caps.iter().find(|row| n_c(row).is_some_and(|n| n > 0)).expect("a misaligned admissible cap");
    let blocked = caps.iter().find(|row| blocked(row)).expect("a blocked cap");
    let cap_of = |row: &JsonValue| row.get("cap").and_then(JsonValue::as_i64).expect("a cap ID");
    let arg = |text: &str| c600_run::provenance::parse(text.as_bytes()).expect("canonical bytes");
    let whole = Batch { chunk: 1 << 12, budget: None, cancel_after: None };

    let mut rows = Rows::on(engine(), Base::Snapshot(&walk));
    for row in [misaligned, blocked] {
        let cap = cap_of(row);
        let served = rows.answer("seam_facts", Some(&arg(&format!(r#"{{"int":"{cap}"}}"#)))).expect("served");
        let mut state = walk.state().clone();
        let facts = seam_facts(engine(), &mut state, Scope::Cap(cap), &whole, Cells::None, None).expect("no error");
        let fields = served.as_object().expect("a record");
        assert_eq!(fields.len(), 7, "cap {cap}: every field written");
        for (name, value) in fields {
            assert_eq!(value, facts.get(name).unwrap_or(&JsonValue::Null), "cap {cap}: {name}");
        }
        let totals = served.get("totals").expect("written");
        assert_eq!(totals.get("N_c").and_then(JsonValue::as_i64), n_c(row), "cap {cap}: the rim summary's N_c");
    }
    let at_blocked = rows.answer("seam_facts", Some(&arg(&format!(r#"{{"int":"{}"}}"#, cap_of(blocked)))));
    let at_blocked = at_blocked.expect("served");
    assert_eq!(at_blocked.get("totals"), Some(&JsonValue::Null));
    assert_eq!(at_blocked.get("cells"), Some(&JsonValue::Null));
    assert_eq!(at_blocked.get("undefined").and_then(JsonValue::as_str), Some("cap_not_admissible"));
    for outside in ["600", "-1"] {
        let refused = rows.answer("seam_facts", Some(&arg(&format!(r#"{{"int":"{outside}"}}"#))));
        assert_eq!(refused, Err(NAMED_OBJECT_INVALID), "cap {outside}");
    }
    assert_eq!(rows.answer("seam_facts", Some(&arg(r#"{"nat":"7"}"#))), Err(ARGUMENT_FORM));
    assert_eq!(rows.answer("seam_facts", None), Err(ARGUMENT_FORM));
}
