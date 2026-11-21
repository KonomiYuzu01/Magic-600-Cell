//! The jumbling rows' host bridge at the command layer's head (LP3, C8; `c600_jumble_tool::rows`): the library entries
//! of CT-LB1-03, CT-LB3-03, CT-LB3-04, CT-LB3-06 and CT-LB3-07 run through `Rows::at_head` on sessions of the jumbled
//! puzzle through the command layer with Jumble on, as `tests/common` drives them: `jumble.on` at solved under a W-J
//! fixture's pinned menu, then one `jumble.twist.menu` press for each of the fixture's first journal records.
//!
//! At each head, each entry run through the bridge at the head equals its run through the bridge on the same state,
//! witnessed apart from the layer by the same records (`Rows::on`), byte for byte: outcome, code, the value's canonical
//! bytes, the rows read and the drafted sets checked. `tests/library.rs`, `tests/ct_lb3_04.rs` and `tests/ct_lb3_07.rs`
//! compare the entries run on such snapshots with `c600_jumble`'s answers, so the head serves the kernel's answers on
//! the head's state. The heads are I_a and S4 after 0, 1 and 12 records and I_b after 1. The words are the fixture's
//! own next 20 records, as menu words of its alphabet; the step flag's unit is the head's first exact-pose group off
//! the lattice, and at solved, where there is none, the row refuses piece 0 as a unit
//! (`KN2:input.named_object_invalid`). The corrections are words of the I_a menu: under S4 and I_b `nu` answers and the
//! drafted check refuses the menu (`KN2:input.menu_binding_mismatch`); under I_a an unshipped id stops the drafted
//! check (`KN2:input.named_object_invalid`).
//!
//! With Jumble off, after `jumble.off`, every read stops with `KN5:input.menu_word_needs_jumble_on`. The bridge grants
//! no preview, clock or other row (`LG3:capability.not_granted`), refuses a row read in another argument form with
//! `KN5:input.argument_form` at its tariff, and its tariff counts one pass of the model's pieces for the state and
//! one for each letter of a named word or entry of a drafted set, two for a named cap's seam facts or the whole scan,
//! one more for each cap for the rim summary, one more for a named cap and each of its 80 meeting caps for covers, one
//! for a named correction id, and one more for a named twist, grip, cap or unit, or every unit off the lattice; one for
//! the sticker test, two for the label checkpoint, one more for each letter of a retained witness, and the repair
//! list's worst case under the state's menu. Covers, the survey, the straddlers and the repair list take one cap, the
//! catalogue lookup one id and the twist check one letter, never `all` or a list: the research survey's all-items loop
//! is not served.
//!
//! The `seam_facts` row, which CT-RTE-07's jumbling keys read (`tests/ct_rte_07_jumble.rs`), answers for the cap it is
//! named: at an admissible cap its N_c equals the engine's rim summary for that cap, a blocked cap has no totals, and a
//! cap outside the model is `KN2:input.named_object_invalid`. The `repair_list` row serves the engine's completed list
//! at the cap it is named, each row a record of its five integers, without the outcome of every pair tried (CT-LB3-07),
//! and the end tests' rows serve M8's verdicts with every evidence key they write. A pass is source and fixture
//! evidence.

mod common;

use std::sync::OnceLock;

use c600_check::TYPE_MISMATCH;
use c600_command::code::ARGUMENT_FORM;
use c600_ir::{EndTest, QTy, Ty};
use c600_jumble::codes::{MENU_BINDING_MISMATCH, MENU_WORD_NEEDS_JUMBLE_ON, NAMED_OBJECT_INVALID};
use c600_jumble::levels::{self, Base, Partial, Snapshot, Verdict, Witness};
use c600_jumble::model::{NP, POLES};
use c600_jumble::nu;
use c600_jumble::observe::cuts::{repair_list, rim_summary};
use c600_jumble::observe::{seam_facts, Batch, Rows as Cells, Scope};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Twist;
use c600_jumble_tool::library::{
    self, ADMISSIBILITY, CORRECTIONS, COVERS, END_TESTS, HEIGHTS, NU, PROTECTION, REPAIR, RIM, STRADDLERS, UNITS,
    WORD_CHECKS,
};
use c600_jumble_tool::rows::{
    repair_passes, Rows, LABEL_FIELDS, MEETING_CAPS, RESULT_BOUND, RETAINED_FIELDS, STICKER_FIELDS,
};
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
/// step flag with `unit` and protection on `word`, CT-LB3-04's ν entries with drafted sets, an unshipped id among
/// them, and CT-LB3-07's inspector entries: the rim facts, the covers at cap 57, the catalogue's correction 27, which
/// only a head under I_a answers, the twist check of `word`'s first letter, the status and the straddlers of grip 57,
/// the units off the lattice, `unit` named, and the clean-step contract of `word` with `unit`; and the end tests: the
/// sticker test, the label checkpoint and the retained checkpoint of the witness 1, -1.
fn calls(pinned: &MenuAlphabet, word: &LiteralWord, unit: &[u32]) -> Vec<(&'static str, Call)> {
    let w = menu_word(pinned, word);
    let first = word.alphabet().word(&word.pairs()[..1]).expect("the word's first letter");
    let call = |module: &'static str, entry: &'static str, args: Vec<RunValue>| Call { module, entry, args };
    vec![
        ("twist check", call(ADMISSIBILITY.name, "twist_check", vec![menu_word(pinned, &first)])),
        ("grip status", call(ADMISSIBILITY.name, "grip_status", vec![RunValue::int(57u32)])),
        ("straddlers", call(STRADDLERS.name, "straddlers_at", vec![RunValue::int(57u32)])),
        ("units", call(UNITS.name, "off_units", vec![])),
        ("named unit", call(UNITS.name, "named_unit", vec![naturals(unit)])),
        ("clean step", call(UNITS.name, "clean_step", vec![w.clone(), naturals(unit)])),
        ("heights", call(HEIGHTS.name, "height_facts", vec![])),
        ("application", call(WORD_CHECKS.name, "word_observations", vec![w.clone()])),
        ("step flag", call(WORD_CHECKS.name, "step_flag", vec![w.clone(), naturals(unit)])),
        ("protection, none", call(PROTECTION.name, "word_protection", vec![w.clone(), naturals(&[])])),
        ("protection, four", call(PROTECTION.name, "word_protection", vec![w, naturals(&[0, 7, 11, 1200])])),
        ("ν", call(NU.name, "nu_facts", vec![])),
        ("ν, empty draft", call(NU.name, "drafted_check", vec![draft(&[])])),
        ("ν, two entries", call(NU.name, "drafted_check", vec![draft(&[(2, 1), (3, -1)])])),
        ("ν, unshipped id", call(NU.name, "drafted_check", vec![draft(&[(2, 1), (36, 1)])])),
        ("rim", call(RIM.name, "rim_facts", vec![])),
        ("covers", call(COVERS.name, "covers_at", vec![RunValue::int(57u32)])),
        ("correction", call(CORRECTIONS.name, "correction", vec![RunValue::int(27u32)])),
        ("sticker test", call(END_TESTS.name, "sticker_test", vec![])),
        ("label checkpoint", call(END_TESTS.name, "label_checkpoint", vec![])),
        ("retained checkpoint", call(END_TESTS.name, "retained_checkpoint", vec![RunValue::word(&[1, -1])])),
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

/// CT-LB1-03, CT-LB3-03, CT-LB3-04, CT-LB3-06 and CT-LB3-07 (part: the library entries through the host bridge at the
/// command layer's head with Jumble on).
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
                // `nu` answers and the drafted check and the catalogue lookup refuse the menu; under I_a only the
                // unshipped id stops a run.
                let expected = match (name, label.starts_with('ν'), label) {
                    (_, false, "step flag" | "named unit" | "clean step") if on_lattice => Some(NAMED_OBJECT_INVALID),
                    ("I_a", false, "correction") => None,
                    (_, false, "correction") => Some(MENU_BINDING_MISMATCH),
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

/// What the bridge grants, and its tariff (CM.18).
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
    for query in ["positions", "labels", "state", "nu_check", "classify_all", "setup_search"] {
        assert_eq!(rows.tariff(&Request::Snapshot { query, ty: &nat }).map(|t| t.kernel_cost), Err(NOT_GRANTED));
    }
    for query in ["heights", "height_counts", "nu", "sticker_test"] {
        let tariff = rows.tariff(&Request::Snapshot { query, ty: &nat }).expect("granted");
        assert_eq!((tariff.kernel_cost, tariff.result_bound), (pass, RESULT_BOUND), "{query}");
        let with = Request::SnapshotWith { query, arg: &no_word, ty: &nat };
        assert_eq!(rows.tariff(&with).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "{query} takes no argument");
    }
    let rim = rows.tariff(&Request::Snapshot { query: "rim_summary", ty: &nat }).map(|t| t.kernel_cost);
    assert_eq!(rim, Ok((1 + POLES as u128) * pass), "one pass and one for each cap's classification");
    let rim_with = Request::SnapshotWith { query: "rim_summary", arg: &no_word, ty: &nat };
    assert_eq!(rows.tariff(&rim_with).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "rim_summary takes no argument");
    let label = rows.tariff(&Request::Snapshot { query: "label_checkpoint", ty: &nat }).map(|t| t.kernel_cost);
    assert_eq!(label, Ok(2 * pass), "the labels' pass and ν's");
    let label_with = Request::SnapshotWith { query: "label_checkpoint", arg: &no_word, ty: &nat };
    assert_eq!(rows.tariff(&label_with).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "label_checkpoint takes none");
    let taking = [
        "replay",
        "word_check",
        "protection",
        "correction_set_check",
        "seam_facts",
        "covers",
        "correction_lookup",
        "classify",
        "survey",
        "straddlers",
        "units",
        "repair_list",
        "retained_checkpoint",
    ];
    for query in taking {
        let without = Request::Snapshot { query, ty: &nat };
        assert_eq!(rows.tariff(&without).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "{query} takes an argument");
    }
    let cap = Request::SnapshotWith { query: "seam_facts", arg: br#"{"int":"7"}"#, ty: &nat };
    assert_eq!(rows.tariff(&cap).map(|t| t.kernel_cost), Ok(2 * pass), "the cap's classification and its cut");
    let not_a_cap = Request::SnapshotWith { query: "seam_facts", arg: br#"{"letters":[],"menu":{}}"#, ty: &nat };
    assert_eq!(rows.tariff(&not_a_cap).map(|t| t.kernel_cost), Err(ARGUMENT_FORM));
    let all = Request::SnapshotWith { query: "seam_facts", arg: br#""all""#, ty: &nat };
    assert_eq!(rows.tariff(&all).map(|t| t.kernel_cost), Ok(2 * pass), "the whole scan");
    let other = Request::SnapshotWith { query: "seam_facts", arg: br#""some""#, ty: &nat };
    assert_eq!(rows.tariff(&other).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "a cap or all, nothing else");
    let covers = Request::SnapshotWith { query: "covers", arg: br#"{"int":"7"}"#, ty: &nat };
    let meeting = 1 + MEETING_CAPS as u128;
    assert_eq!(rows.tariff(&covers).map(|t| t.kernel_cost), Ok((1 + meeting) * pass), "the cap and its meeting caps");
    let covers_all = Request::SnapshotWith { query: "covers", arg: br#""all""#, ty: &nat };
    assert_eq!(rows.tariff(&covers_all).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "covers takes one named cap");
    let lookup = Request::SnapshotWith { query: "correction_lookup", arg: br#"{"int":"27"}"#, ty: &nat };
    assert_eq!(rows.tariff(&lookup).map(|t| t.kernel_cost), Ok(pass), "one pass for the named id");
    let ids = Request::SnapshotWith { query: "correction_lookup", arg: br#"[{"int":"2"},{"int":"3"}]"#, ty: &nat };
    assert_eq!(rows.tariff(&ids).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "one id, never a list of ids");
    let word = br#"{"letters":[[0,1],[2,3],[4,5]],"menu":{}}"#;
    let replay = Request::SnapshotWith { query: "replay", arg: word, ty: &nat };
    assert_eq!(rows.tariff(&replay).map(|t| t.kernel_cost), Ok(4 * pass), "one pass and one for each letter");
    let set = br#"[{"id":{"int":"2"},"m":{"int":"1"}},{"id":{"int":"3"},"m":{"int":"-1"}}]"#;
    let check = Request::SnapshotWith { query: "correction_set_check", arg: set, ty: &nat };
    assert_eq!(rows.tariff(&check).map(|t| t.kernel_cost), Ok(3 * pass), "one pass and one for each entry");
    let malformed = Request::SnapshotWith { query: "replay", arg: br#"{"letters":[[0]]"#, ty: &nat };
    assert_eq!(rows.tariff(&malformed).map(|t| t.kernel_cost), Err(ARGUMENT_FORM));
    let twist = Request::SnapshotWith { query: "classify", arg: br#"{"letters":[[0,1]],"menu":{}}"#, ty: &nat };
    assert_eq!(rows.tariff(&twist).map(|t| t.kernel_cost), Ok(2 * pass), "one pass for the named twist");
    let two = Request::SnapshotWith { query: "classify", arg: word, ty: &nat };
    assert_eq!(rows.tariff(&two).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "one named twist, not a word of three");
    for query in ["survey", "straddlers"] {
        let named = Request::SnapshotWith { query, arg: br#"{"int":"7"}"#, ty: &nat };
        assert_eq!(rows.tariff(&named).map(|t| t.kernel_cost), Ok(2 * pass), "{query}: the named cap's pass");
        for arg in [&br#""all""#[..], br#"[{"int":"7"},{"int":"8"}]"#] {
            let unnamed = Request::SnapshotWith { query, arg, ty: &nat };
            assert_eq!(rows.tariff(&unnamed).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "{query}: one named cap");
        }
    }
    let unit = Request::SnapshotWith { query: "units", arg: br#"[{"int":"3"},{"int":"9"}]"#, ty: &nat };
    assert_eq!(rows.tariff(&unit).map(|t| t.kernel_cost), Ok(2 * pass), "the named unit");
    let every = Request::SnapshotWith { query: "units", arg: br#""all""#, ty: &nat };
    assert_eq!(rows.tariff(&every).map(|t| t.kernel_cost), Ok(2 * pass), "every unit off the lattice");
    let one = Request::SnapshotWith { query: "units", arg: br#"{"int":"3"}"#, ty: &nat };
    assert_eq!(rows.tariff(&one).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "a unit is a list of pieces");
    // The repair list at its worst case under the state's menu: the named cap's score and its meeting caps', then for
    // each item other than the identity its play and, at each meeting cap, that cap's score, y's play and the named
    // cap's score again.
    let items = solved.state().menu().map_or(0, |menu| menu.items.len()) as u128;
    assert!(items > 1, "the I_a menu has items other than the identity");
    let meeting = MEETING_CAPS as u128;
    let worst = 1 + (1 + meeting) + (items - 1) * (1 + 3 * meeting);
    assert_eq!(1 + repair_passes(items as usize) as u128, worst);
    let repair = Request::SnapshotWith { query: "repair_list", arg: br#"{"int":"57"}"#, ty: &nat };
    assert_eq!(rows.tariff(&repair).map(|t| t.kernel_cost), Ok(worst * pass), "the named cap's list, at its worst");
    for arg in [&br#""all""#[..], br#"[{"int":"57"},{"int":"58"}]"#] {
        let unnamed = Request::SnapshotWith { query: "repair_list", arg, ty: &nat };
        assert_eq!(rows.tariff(&unnamed).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "repair_list: one named cap");
    }
    let witness = Request::SnapshotWith { query: "retained_checkpoint", arg: b"[1,-2,1200]", ty: &nat };
    assert_eq!(rows.tariff(&witness).map(|t| t.kernel_cost), Ok(4 * pass), "one pass and one for each letter");
    for arg in [&b"[0]"[..], b"[1201]", br#"[{"int":"1"}]"#, br#"{"letters":[[0,1]],"menu":{}}"#] {
        let other = Request::SnapshotWith { query: "retained_checkpoint", arg, ty: &nat };
        assert_eq!(rows.tariff(&other).map(|t| t.kernel_cost), Err(ARGUMENT_FORM), "a word of primitive ids only");
    }
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

/// The `repair_list` row at the head of the I_a session after two records, where caps 57 and 86 are the admissible
/// misaligned caps (as `tests/lb3.rs` CT-LB3-05 reads them): at the named cap 57 the row serves the engine's completed
/// list on the head's state, its status, input digest, message and parameters as the engine wrote them and each row as
/// a record of its five integers x, d, y and z and delta, in the engine's x-then-d order. It serves neither the outcome
/// of every pair tried, nor the coverage, nor what stopped a list, and no field's name ranks, marks or recommends.
/// `jumble_repair.repair_list_at` runs through the bridge at the head and reads the row once, for the cap it is given.
/// At solved the closed cap 57 is an error with no rows, and so is free mode; a cap outside the model is
/// `KN2:input.named_object_invalid`, and an argument that is not one integer is `KN5:input.argument_form`.
#[test]
fn the_repair_list_row_serves_the_named_caps_list_only() {
    let fixture = Fixture::load("I_a");
    let pinned = Alphabet::of(engine(), &fixture.menu()).expect("an inverse-closed menu").pinned().expect("pinned");
    let mut session = Session::entered(&fixture);
    for twist in &fixture.twists[..2] {
        assert!(committed(&session.press(twist)), "a record of the walk");
    }
    let state = session.state().clone();
    let arg = |text: &str| c600_run::provenance::parse(text.as_bytes()).expect("canonical bytes");
    let at_57 = arg(r#"{"int":"57"}"#);

    let served = Rows::at_head(engine(), &session.layer).answer("repair_list", Some(&at_57)).expect("served");
    let list = repair_list(engine(), &state, 57, None, None, None).expect("no engine error");
    let fields = served.as_object().expect("a record");
    let names: Vec<&str> = fields.keys().map(String::as_str).collect();
    assert_eq!(names, ["input_digest", "message", "parameters", "rows", "status"], "no outcomes, coverage or stop");
    for name in ["input_digest", "message", "parameters", "status"] {
        assert_eq!(served.get(name), list.get(name), "{name}, as the engine wrote it");
    }
    assert_eq!(served.get("status").and_then(JsonValue::as_str), Some("completed"));
    let engine_rows = list.get("rows").and_then(JsonValue::as_array).expect("a completed list has rows");
    let rows = served.get("rows").and_then(JsonValue::as_array).expect("the rows");
    assert!(!rows.is_empty() && rows.len() == engine_rows.len(), "{} rows", rows.len());
    let mut pairs = Vec::new();
    for (row, engine_row) in rows.iter().zip(engine_rows) {
        let named = ["x", "d", "y", "z", "delta"].map(|name| row.get(name).cloned().expect("written"));
        assert_eq!(&named[..], engine_row.as_array().expect("five integers"), "the engine's row");
        assert_eq!(row.as_object().map(|row| row.len()), Some(5), "five integers and no mark");
        pairs.push([named[0].as_i64().expect("x"), named[1].as_i64().expect("d")]);
    }
    assert!(pairs.windows(2).all(|w| w[0] < w[1]), "x in pinned order, then d: {pairs:?}");
    let parameters = served.get("parameters").and_then(JsonValue::as_object).expect("the parameters");
    for name in fields.keys().chain(parameters.keys()) {
        for leak in ["recommend", "suggest", "next", "skip", "best", "rank", "score", "target", "outcome", "coverage"] {
            assert!(!name.contains(leak), "field `{name}`");
        }
    }

    // The entry through the bridge at the head: one read of the row, for the named cap.
    let call = Call { module: REPAIR.name, entry: "repair_list_at", args: vec![RunValue::int(57u32)] };
    let run = seen(&call, Rows::at_head(engine(), &session.layer), &pinned);
    assert_eq!((run.outcome, run.code), (Outcome::Completed, None));
    assert_eq!(run.asked, ["repair_list"]);
    assert!(run.value.is_some(), "the list as read");

    // A closed cap and free mode are errors with no rows; a cap outside the model and another argument are refused.
    let solved = witnessed(&fixture, 0);
    let free = State::new(None);
    let errors = [("closed", Rows::on(engine(), Base::Snapshot(&solved))), ("free", Rows::at_state(engine(), &free))];
    for (label, mut rows) in errors {
        let answer = rows.answer("repair_list", Some(&at_57)).expect("served");
        assert_eq!(answer.get("status").and_then(JsonValue::as_str), Some("error"), "{label}");
        assert_eq!(answer.get("rows"), Some(&JsonValue::Null), "{label}: no rows");
        assert!(answer.get("message").and_then(JsonValue::as_str).is_some(), "{label}: says why");
    }
    let mut rows = Rows::on(engine(), Base::Snapshot(&solved));
    for outside in ["600", "-1"] {
        let refused = rows.answer("repair_list", Some(&arg(&format!(r#"{{"int":"{outside}"}}"#))));
        assert_eq!(refused, Err(NAMED_OBJECT_INVALID), "cap {outside}");
    }
    assert_eq!(rows.answer("repair_list", Some(&arg(r#""all""#))), Err(ARGUMENT_FORM));
    assert_eq!(rows.answer("repair_list", None), Err(ARGUMENT_FORM));
}

/// The end tests' rows at the I_a walk after 0 and 12 records, on the witnessed snapshot: each serves M8's verdict on
/// the state with the level it was reached at, an unknown's reason, and every evidence key the verdict writes under its
/// name in the row's field list, the reachability claim as its kind; a field the verdict does not write is null. The
/// label checkpoint takes ν from the ν row when the identity labels and a retained witness leave it unknown, as the
/// command layer's exit reads it. At solved the sticker test says yes; after twelve records it says no. Each entry of
/// `jumble_end_tests` reads its own row once through the bridge and answers its test's verdict type with the outcome
/// alone: yes `holds`, no `fails`, unknown `unestablished`.
#[test]
fn the_end_test_rows_serve_each_verdict_with_its_evidence() {
    let fixture = Fixture::load("I_a");
    let pinned = Alphabet::of(engine(), &fixture.menu()).expect("an inverse-closed menu").pinned().expect("pinned");
    let witness = c600_run::provenance::parse(b"[1,-1]").expect("canonical bytes");
    let tag = |verdict: &str| match verdict {
        "yes" => "holds",
        "no" => "fails",
        _ => "unestablished",
    };
    for (count, sticker) in [(0, "yes"), (12, "no")] {
        let snapshot = witnessed(&fixture, count);
        let (state, reach) = (snapshot.state(), snapshot.reachability());
        let mut label = levels::label_checkpoint(engine(), state, reach, None).expect("no engine error");
        if matches!(label, Verdict::Unknown(_)) {
            if let Ok(Partial::Total(nu)) = nu::nu(engine(), Base::Snapshot(&snapshot)).expect("no engine error") {
                label = levels::label_checkpoint(engine(), state, reach, Some(&nu.values()[..])).expect("no error");
            }
        }
        let retained = levels::retained_checkpoint(engine(), state, Witness::Word(&[1, -1])).expect("no engine error");
        let tests = [
            (
                "sticker_test",
                None,
                levels::sticker_test(engine(), state).expect("no engine error"),
                levels::STICKER_TEST_LEVEL,
                &STICKER_FIELDS[..],
            ),
            ("retained_checkpoint", Some(&witness), retained, levels::RETAINED_CHECKPOINT_LEVEL, &RETAINED_FIELDS[..]),
            ("label_checkpoint", None, label, levels::LABEL_CHECKPOINT_LEVEL, &LABEL_FIELDS[..]),
        ];
        let mut rows = Rows::on(engine(), Base::Snapshot(&snapshot));
        let mut outcomes = Vec::new();
        for (query, arg, verdict, level, fields) in tests {
            let what = format!("after {count}: {query}");
            outcomes.push((query, verdict.name()));
            let served = rows.answer(query, arg).expect("served");
            let read = verdict.to_json_at(level);
            for name in ["level", "verdict", "reason"] {
                assert_eq!(served.get(name), Some(read.get(name).unwrap_or(&JsonValue::Null)), "{what}: {name}");
            }
            let evidence = read.get("evidence").and_then(JsonValue::as_object);
            for (name, value) in evidence.into_iter().flatten() {
                assert!(fields.contains(&name.as_str()), "{what}: the evidence key {name} is served");
                let value = if name == "reachability" { value.get("kind").expect("a kind") } else { value };
                assert_eq!(served.get(name), Some(value), "{what}: {name}");
            }
            assert_eq!(served.as_object().map(|row| row.len()), Some(3 + fields.len()), "{what}: every field written");
            for name in fields {
                if !evidence.is_some_and(|evidence| evidence.contains_key(*name)) {
                    assert_eq!(served.get(name), Some(&JsonValue::Null), "{what}: {name}, not written");
                }
            }
        }
        let served = rows.answer("sticker_test", None).expect("served");
        assert_eq!(served.get("verdict").and_then(JsonValue::as_str), Some(sticker), "after {count}");

        // Each entry through the bridge: one read of its own row, and the verdict's outcome in its test's type.
        for (query, verdict) in outcomes {
            let args = if query == "retained_checkpoint" { vec![RunValue::word(&[1, -1])] } else { vec![] };
            let call = Call { module: END_TESTS.name, entry: query, args };
            let run = seen(&call, Rows::on(engine(), Base::Snapshot(&snapshot)), &pinned);
            assert_eq!((run.outcome, run.code), (Outcome::Completed, None), "after {count}: {query}");
            assert_eq!(run.asked, [query], "after {count}: {query} reads its own row once");
            let value = format!("{{\"outcome\":\"{}\",\"test\":\"{query}\"}}\n", tag(verdict));
            assert_eq!(run.value.as_deref(), Some(value.as_bytes()), "after {count}: {query}, the outcome alone");
        }
    }

    // Another test's verdict type: only a test's own row answers its verdict, at the bridge as in the checker.
    let snapshot = witnessed(&fixture, 0);
    let mut rows = Rows::on(engine(), Base::Snapshot(&snapshot));
    let label = QTy { ty: Ty::EndTest(EndTest::LabelCheckpoint), display: false };
    let other = Request::Snapshot { query: "sticker_test", ty: &label };
    assert_eq!(rows.perform(&other).map(|_| ()), Err(TYPE_MISMATCH), "a sticker test is no label checkpoint");
    let own = Request::Snapshot { query: "label_checkpoint", ty: &label };
    assert!(matches!(rows.perform(&own), Ok(RunValue::EndTest(EndTest::LabelCheckpoint, _))), "its own row");
}
