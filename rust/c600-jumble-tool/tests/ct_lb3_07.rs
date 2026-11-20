//! CT-LB3-07 (chapter 03, LB3.5), the inspector library entries in the language (LP3, C8): "Per-revision fact results
//! contain no all-items outcome table. Named-word checks answer only their named word; no recommendation, next-cap
//! skip or correction combination leaks through the inspector entries."
//!
//! The entries are the modules of `c600_jumble_tool::library` that LB6.5's candidate jumbling inspectors use: LB3.1's
//! "Rim and seam facts; N" (`jumble_rim`), "Covers at a named cap" (`jumble_covers`), "Correction catalogue"
//! (`jumble_corrections`), "Admissibility and blocked-twist check" (`jumble_admissibility`), "Straddlers and u at a
//! named cap" (`jumble_straddlers`) and "Units and clean-step contract" (`jumble_units`), and the named-word checks
//! (`jumble_word_checks`) and the drafted correction-set check (`jumble_nu`). Each runs through LP2's interpreter
//! against C8's host bridge (`c600_jumble_tool::rows`) on a witnessed snapshot of the I_a walk, and each answer is
//! compared with `c600_jumble`'s, computed apart. A recording host keeps every row the run reads with its argument, so
//! a test sees what an entry asked as well as what it answers.
//!
//! - The rim entry answers one row for every cap, in cap-ID order, blocked and closed caps included, each with the
//!   kernel's facts at this revision and nothing about a twist; it reads the rim summary and the seam scan once each
//!   and no word row. Its counts and grip N are the same facts summed apart.
//! - The covers entry reads the solver's named cap once and answers every meeting cap in cap-ID order, with no mark,
//!   rank or selection, and recomputes the face-piece definition from the kernel's counts.
//! - The catalogue entry reads the solver's id once and answers that correction only, under I_a's menu only; the
//!   drafted check reads exactly the sets the solver drafted.
//! - The admissibility entries read the solver's one letter or named grip once and answer that twist or grip only;
//!   the straddlers entry reads the named cap once and answers its straddlers with exact certificates, and u.
//! - The survey row answers one named grip's status and no table of grips: an entry written to ask it for every grip,
//!   or for a list of grips, is refused with `KN5:input.argument_form`, and the bridge classifies with no budget, so it
//!   never serves an incomplete survey.
//! - The units entries read every off unit, or the solver's named unit, once; the clean-step entry reads the solver's
//!   word and unit once and answers the clean-step contract for that unit only.
//! - The named-word checks read their row once, with the named word as its argument, and answer no word.
//! - The end tests (`jumble_end_tests`, LB3.1's "Retained checkpoint, label checkpoint, sticker test") read their row
//!   once each, the retained checkpoint with the solver's witness word, and answer their test's own verdict type, the
//!   outcome alone (LG2.9); `tests/rows.rs` runs them through the bridge.
//! - The repair entry (`jumble_repair`, LB3.1's "Repair list at a named cap") is the owner's one exception (17 October
//!   2026): it reads the repair list once, for the cap the solver named, and returns it as read. The built-in column
//!   admits it only under the repair-list exception (BND.2), which refuses a read for another cap and any use of the
//!   list's value, so its rows keep the engine's neutral order with no mark; `tests/rows.rs` checks what it serves.
//! - The crossing entry (`jumble_crossing`, LB3.1's "Mode crossing prerequisites", the Jumble-off half) reads the label
//!   checkpoint, every unit off the lattice and ν once each, and answers A5's prerequisites of Jumble off: the
//!   checkpoint's outcome alone, the count of pieces off the lattice, and whether ν is defined with the count of its
//!   nonzero values, with no correction and no word. `tests/jumble_layer_verdicts.rs` checks them against `jumble.off`.
//!   The Jumble-on half, Full labels with journal derivation, waits for a KN4 journal query served to programs (KN4.3).
//!
//! No answer of the other entries has a field whose name is a recommendation, a next cap, a ranking, a selection or a
//! correction (a check of names), and the rim, covers, straddlers and units entries hold no field of a twist's
//! outcome. Under capability separation every entry reads state and prepares no preview. `tests/lb3.rs` runs the
//! kernel half of CT-LB3-07 and `c600-tools/tests/lb3_checker.rs` the checker half. A pass is source and fixture
//! evidence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, check, separation, Grant, Policy, Row, Separation, StateRead, NOT_GRANTED};
use c600_command::code::ARGUMENT_FORM;
use c600_digest::sha256_hex;
use c600_ir::{Effect, EndTest, EndTestOutcome, Expr};
use c600_jumble::corrections;
use c600_jumble::levels::{Base, Partial, Snapshot};
use c600_jumble::model::{NP, POLES};
use c600_jumble::nu;
use c600_jumble::observe::cuts::{covers, rim_summary};
use c600_jumble::observe::{seam_facts, units, word_check, Batch, Rows as Cells, Scope};
use c600_jumble::state::{Certificate, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::library::{
    self, Module, ADMISSIBILITY, CORRECTIONS, COVERS, CROSSING, END_TESTS, NU, REPAIR, RIM, STRADDLERS, UNITS,
    WORD_CHECKS,
};
use c600_jumble_tool::rows::Rows;
use c600_jumble_tool::word::Alphabet;
use c600_kernel::json::{self, Value};
use c600_library::lower_source;
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, Json};
use c600_run::value::Value as RunValue;
use c600_syntax::{format, parse};

/// Run limits wide enough for every run here.
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// Parts of a field name that would name a recommendation, a next cap, a ranking or a correction (`tests/lb3.rs`).
const LEAKS: [&str; 9] = ["recommend", "suggest", "next", "skip", "best", "rank", "correction", "repair", "target"];

/// Parts of a field name that would name a selection by the method, an unlock list or an existence answer (LB3.5).
const SELECTIONS: [&str; 7] = ["top", "first", "largest", "max", "unlock", "exist", "prefetch"];

/// Parts of a field name that would name a twist's outcome: a row of an all-items table (DT-J1 fix-first 6).
const OUTCOMES: [&str; 7] = ["item", "letter", "twist", "after", "delta", "outcome", "word"];

/// The fields of each cap's row in the rim entry: the kernel's rim facts at this revision.
const RIM_ROW: [&str; 10] = ["A_c", "F_c", "In_c", "N_c", "Out_c", "at_floor", "blocked", "cap", "status", "undefined"];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The jumbling model, loaded once for the tests of this file.
fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

/// The library's modules, lowered together, checked with no diagnostic and loaded once.
fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        let program = library::program(&[]).expect("the library lowers");
        let report = check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        load(&program).expect("the library loads")
    })
}

/// The I_a fixture: its pinned menu and its journal records.
struct Fixture {
    menu: Arc<Menu>,
    records: Vec<Value>,
}

fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let bytes = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
        let doc = json::parse(&bytes).expect("JSON");
        let menu = Menu::from_record(ctx(), doc.get("menu").expect("a menu")).expect("the fixture's pinned menu");
        let journal = doc.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).expect("records");
        Fixture { menu: Arc::new(menu), records: journal.to_vec() }
    })
}

impl Fixture {
    fn twists(&self, range: std::ops::Range<usize>) -> Vec<Twist> {
        self.records[range].iter().map(|record| Twist::from_record(ctx(), record).expect("a journal record")).collect()
    }

    /// The state after the first `count` records from solved, witnessed by them.
    fn after(&self, count: usize) -> Snapshot {
        let journal = self.twists(0..count);
        let mut state = State::new(Some(self.menu.clone()));
        for (index, twist) in journal.iter().enumerate() {
            let status = state.apply(ctx(), twist, None, &[]).expect("no engine error").status;
            assert_eq!(status, Status::Admissible, "letter {index}");
        }
        Snapshot::witnessed(ctx(), state, &journal, None).expect("no engine error").expect("the journal witnesses it")
    }

    fn alphabet(&self) -> Alphabet {
        Alphabet::of(ctx(), &self.menu).expect("an inverse-closed menu")
    }

    fn pinned(&self) -> MenuAlphabet {
        self.alphabet().pinned().expect("the run's menu alphabet")
    }

    /// `twists` as `[cap, item]` pairs of the pinned menu.
    fn pairs(&self, twists: &[Twist]) -> Vec<[i32; 2]> {
        let pair = |twist: &Twist| {
            let items = self.menu.for_cap(ctx(), twist.grip).expect("the menu's items");
            let item = items.iter().position(|item| item.matrix == twist.matrix).expect("an item of the menu");
            [twist.grip as i32, item as i32]
        };
        twists.iter().map(pair).collect()
    }
}

/// The I_a walk after twelve records, shared by the tests: it has blocked, closed and misaligned caps.
fn walk() -> &'static Snapshot {
    static WALK: OnceLock<Snapshot> = OnceLock::new();
    WALK.get_or_init(|| fixture().after(12))
}

/// The bridge with every read it answered or refused, each with its argument's canonical bytes.
struct Recording<'a> {
    rows: Rows<'a>,
    reads: Vec<(String, Option<Json>)>,
}

impl<'a> Recording<'a> {
    fn on(snapshot: &'a Snapshot) -> Recording<'a> {
        Recording { rows: Rows::on(ctx(), Base::Snapshot(snapshot)), reads: Vec::new() }
    }

    fn queries(&self) -> Vec<&str> {
        self.reads.iter().map(|(query, _)| query.as_str()).collect()
    }
}

impl Host for Recording<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.rows.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<RunValue, &'static str> {
        match *request {
            Request::Snapshot { query, .. } => self.reads.push((query.to_owned(), None)),
            Request::SnapshotWith { query, arg, .. } => self.reads.push((query.to_owned(), provenance::parse(arg))),
            _ => {}
        }
        self.rows.perform(request)
    }

    fn sink(&mut self, sink: &Sink, value: &RunValue, bytes: &[u8]) -> Result<(), &'static str> {
        self.rows.sink(sink, value, bytes)
    }
}

/// `module.function` on `args`, against `host`, with the fixture's alphabet pinned.
fn run(module: &str, function: &str, args: Vec<RunValue>, host: &mut dyn Host) -> Run {
    let pinned = fixture().pinned();
    let options = Options { menu: Some(&pinned), ..Options::new(WIDE) };
    loaded().run(module, function, args, host, &options).unwrap_or_else(|r| panic!("{function}: refused: {r:?}"))
}

fn completed(run: Run, what: &str) -> RunValue {
    assert_eq!(run.summary.outcome, Outcome::Completed, "{what}: {:?}", run.summary.code);
    run.value.unwrap_or_else(|| panic!("{what}: no value"))
}

fn get<'v>(value: &'v RunValue, name: &str) -> &'v RunValue {
    match value {
        RunValue::Record(record) => record.get(name).unwrap_or_else(|| panic!("no field `{name}`")),
        other => panic!("no record: {other:?}"),
    }
}

fn number(value: &RunValue) -> i64 {
    match value {
        RunValue::Int(n) => n.to_string().parse().expect("an integer of i64"),
        other => panic!("no integer: {other:?}"),
    }
}

fn truth(value: &RunValue) -> bool {
    match value {
        RunValue::Bool(b) => *b,
        other => panic!("no Bool: {other:?}"),
    }
}

fn string(value: &RunValue) -> &str {
    match value {
        RunValue::Text(t) => t.as_str(),
        other => panic!("no Text: {other:?}"),
    }
}

fn items(value: &RunValue) -> &[RunValue] {
    match value {
        RunValue::List(list) => list.items(),
        other => panic!("no List: {other:?}"),
    }
}

fn numbers(value: &RunValue) -> Vec<i64> {
    items(value).iter().map(number).collect()
}

/// A field that may be undefined, as the library declares it: no value, or one.
fn optional(value: &RunValue) -> Option<&RunValue> {
    match items(value) {
        [] => None,
        [one] => Some(one),
        more => panic!("{} values where at most one is declared", more.len()),
    }
}

/// The engine's value of a field that may be null, as the library's optional field reads it.
fn engine_int(value: &Value, key: &str) -> Option<i64> {
    match value.get(key).unwrap_or_else(|| panic!("no field `{key}`")) {
        Value::Null => None,
        other => Some(other.as_i64().unwrap_or_else(|| panic!("`{key}` is no integer"))),
    }
}

fn engine_flag(value: &Value, key: &str) -> Option<bool> {
    match value.get(key).unwrap_or_else(|| panic!("no field `{key}`")) {
        Value::Null => None,
        Value::Bool(b) => Some(*b),
        other => panic!("`{key}` is {}", other.canonical()),
    }
}

fn engine_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// Every field name in a run value, at any depth.
fn field_names(value: &RunValue, out: &mut BTreeSet<String>) {
    match value {
        RunValue::Record(record) => {
            for (name, child) in record.fields() {
                out.insert(name.as_str().to_owned());
                field_names(child, out);
            }
        }
        RunValue::List(list) => list.items().iter().for_each(|item| field_names(item, out)),
        _ => {}
    }
}

/// Whether a run value holds a word: a word or menu word value, or a list of lists, the shape of letters.
fn holds_a_word(value: &RunValue) -> bool {
    match value {
        RunValue::Word(_) | RunValue::MenuWord(_) | RunValue::Compressed(_) => true,
        RunValue::Record(record) => record.fields().iter().any(|(_, child)| holds_a_word(child)),
        RunValue::List(list) => list.items().iter().any(|item| matches!(item, RunValue::List(_)) || holds_a_word(item)),
        _ => false,
    }
}

/// No field name in `value` contains a part of `forbidden`.
fn assert_no_field(value: &RunValue, forbidden: &[&str], what: &str) {
    let mut names = BTreeSet::new();
    field_names(value, &mut names);
    for name in &names {
        let lower = name.to_lowercase();
        for part in forbidden {
            assert!(!lower.contains(part), "{what}: field `{name}`");
        }
    }
}

fn nodes<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
    out.push(e);
    for child in e.children() {
        nodes(child, out);
    }
}

/// The module's source parses with no diagnostic, is in the formatter's canonical layout and has LF line endings.
fn canonical(module: &Module) {
    let file = parse(module.source.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{}: {:#?}", module.name, file.diagnostics);
    let formatted = format(&file);
    assert_eq!(
        formatted.as_deref(),
        Some(module.source),
        "{}: the canonical layout is\n{}",
        module.name,
        formatted.as_deref().unwrap_or_default()
    );
    assert!(!module.source.contains('\r') && module.source.ends_with('\n'), "{}: LF line endings", module.name);
}

/// Letters of the pinned menu, `[cap, item]` each, as a run's menu word.
fn menu_word(pairs: &[[i32; 2]]) -> RunValue {
    let pinned = fixture().pinned();
    let letters: Vec<[u32; 2]> = pairs.iter().map(|&[cap, item]| [cap as u32, item as u32]).collect();
    RunValue::menu_word(pinned.shared_identity(), &letters).expect("a menu word")
}

/// Piece IDs as a run's list.
fn piece_list(pieces: &[i64]) -> RunValue {
    RunValue::list(pieces.iter().map(|&p| RunValue::int(p)).collect())
}

/// The letters of a menu word argument in the run's canonical bytes.
fn read_letters(arg: &Json) -> Vec<[i64; 2]> {
    let Some(Json::List(list)) = arg.get("letters") else { panic!("a menu word's letters") };
    let int = |x: &Json| match x {
        Json::Int(n) => *n,
        other => panic!("no integer: {other:?}"),
    };
    let letter = |pair: &Json| {
        let Json::List(pair) = pair else { panic!("a letter") };
        [int(&pair[0]), int(&pair[1])]
    };
    list.iter().map(letter).collect()
}

/// The integers of a list argument in the run's canonical bytes.
fn read_ints(arg: &Json) -> Vec<i64> {
    let Json::List(list) = arg else { panic!("a list") };
    let int = |x: &Json| x.get("int").and_then(Json::as_text).and_then(|n| n.parse().ok()).expect("an integer");
    list.iter().map(int).collect()
}

/// The single integer argument of a read, as the run's canonical bytes hold it.
fn read_int(arg: &Option<Json>) -> Option<i64> {
    arg.as_ref().and_then(|arg| arg.get("int")).and_then(Json::as_text).and_then(|n| n.parse().ok())
}

fn engine_ints(value: &Value, key: &str) -> Vec<i64> {
    let list = value.get(key).and_then(Value::as_array).unwrap_or_else(|| panic!("no list `{key}`"));
    list.iter().map(|v| v.as_i64().expect("an integer")).collect()
}

/// An exact number of Q(√5) as `[a, b, d]` text: a run's `Q5`. The run and the engine both keep it in lowest terms with
/// d > 0, so equal numbers have equal text.
fn q5_text(value: &RunValue) -> [String; 3] {
    match value {
        RunValue::Q5(q) => [q.a.to_string(), q.b.to_string(), q.d.to_string()],
        other => panic!("no Q5: {other:?}"),
    }
}

/// The engine's JSON triple of an exact number as `[a, b, d]` text.
fn engine_q5(value: &Value) -> [String; 3] {
    let text = |v: &Value| match v {
        Value::Int(n) => n.to_string(),
        Value::Number(lexeme) => lexeme.to_string(),
        other => panic!("no integer: {}", other.canonical()),
    };
    match value.as_array() {
        Some([a, b, d]) => [text(a), text(b), text(d)],
        _ => panic!("no triple: {}", value.canonical()),
    }
}

/// A certificate of the run is the engine's, field for field: the piece, the cut, both vertices and both exact points.
fn same_certificate(ours: &RunValue, theirs: &Certificate, what: &str) {
    let theirs = theirs.to_json();
    let RunValue::Record(record) = ours else { panic!("{what}: a certificate is a record") };
    assert_eq!(record.fields().len(), 11, "{what}: the certificate's fields");
    for name in ["grip", "orbit", "piece", "transport", "vertex_above", "vertex_below"] {
        assert_eq!(Some(number(get(ours, name))), engine_int(&theirs, name), "{what}: {name}");
    }
    let checked = "points_checked_against_all_constraints";
    assert_eq!(Some(truth(get(ours, checked))), engine_flag(&theirs, checked), "{what}");
    for name in ["h_above", "h_below"] {
        assert_eq!(q5_text(get(ours, name)), engine_q5(theirs.get(name).expect("a height")), "{what}: {name}");
    }
    for name in ["point_above", "point_below"] {
        let point: Vec<[String; 3]> = items(get(ours, name)).iter().map(q5_text).collect();
        let engine = theirs.get(name).and_then(Value::as_array).expect("a point");
        assert_eq!(point.len(), 4, "{what}: {name} has four coordinates");
        assert_eq!(point, engine.iter().map(engine_q5).collect::<Vec<_>>(), "{what}: {name}");
    }
}

/// A unit of the run is the engine's: its members, pose, domain, heights and branches.
fn same_unit(ours: &RunValue, theirs: &Value, what: &str) {
    for name in ["branches", "heights", "members"] {
        assert_eq!(numbers(get(ours, name)), engine_ints(theirs, name), "{what}: {name}");
    }
    for name in ["domain", "pose"] {
        assert_eq!(Some(string(get(ours, name))), engine_text(theirs, name), "{what}: {name}");
    }
}

/// CT-LB3-07 (part: the inspector entries' sources). `jumble_rim`, `jumble_covers`, `jumble_corrections`,
/// `jumble_admissibility`, `jumble_straddlers`, `jumble_units`, `jumble_repair`, `jumble_end_tests` and
/// `jumble_crossing` parse with no diagnostic, keep the canonical layout and
/// LF line endings, request no capability and check with no diagnostic. Each exported entry has the one effect
/// `snapshot` and reads only its rows: under capability separation it reads state and prepares no preview. No function
/// names a proposal target, the helpers are pure, and each entry takes the solver's named objects only: a cap, an id, a
/// letter, a grip, a unit, or a word with a unit, or nothing. The catalogue rows name the entries, the effect and the
/// digest of each module's source.
#[test]
fn ct_lb3_07_the_inspector_entries_read_state_and_prepare_no_preview() {
    // Each entry, the rows it reads and its parameters: a named cap, id, twist, grip, unit or word, or nothing.
    let reads: [(&str, &[&str], usize); 14] = [
        ("rim_facts", &["rim_summary", "seam_facts"], 0),
        ("covers_at", &["covers"], 1),
        ("correction", &["correction_lookup"], 1),
        ("twist_check", &["classify"], 1),
        ("grip_status", &["survey"], 1),
        ("straddlers_at", &["straddlers"], 1),
        ("off_units", &["units"], 0),
        ("named_unit", &["units"], 1),
        ("clean_step", &["word_check"], 2),
        ("repair_list_at", &["repair_list"], 1),
        ("sticker_test", &["sticker_test"], 0),
        ("retained_checkpoint", &["retained_checkpoint"], 1),
        ("label_checkpoint", &["label_checkpoint"], 0),
        ("jumble_off_facts", &["label_checkpoint", "nu", "units"], 0),
    ];
    for module in [RIM, COVERS, CORRECTIONS, ADMISSIBILITY, STRADDLERS, UNITS, REPAIR, END_TESTS, CROSSING] {
        canonical(&module);
        let program = lower_source(module.source).expect("the module lowers");
        let [lowered] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
        assert_eq!(lowered.name, module.name);
        assert!(lowered.capabilities.is_empty(), "{}: no capability", module.name);
        let exports: Vec<&str> = lowered.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
        assert_eq!(exports, module.exports);
        let report = check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{}: {:#?}", module.name, report.diagnostics);
        let mut module_rows = BTreeSet::new();
        for function in &lowered.functions {
            let name = format!("{}.{}", module.name, function.name);
            let summary = report.summary(module.name, &function.name).expect("a summary");
            assert!(summary.capabilities.is_empty() && summary.vars.is_empty(), "{name}: {summary:?}");
            assert!(summary.unresolved.is_empty(), "{name}: {summary:?}");
            let mut all = Vec::new();
            nodes(&function.body, &mut all);
            assert!(!all.iter().any(|e| matches!(e, Expr::Propose { .. })), "{name} names no proposal target");
            if !function.exported {
                assert!(summary.is_pure(), "{name}: a helper is pure");
                continue;
            }
            assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{name}");
            assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{name}");
            let separated = separation(summary, &Policy::interim());
            assert_eq!(separated, Separation { reads_state: true, prepares_previews: false }, "{name}");
            let (_, expected, params) =
                reads.iter().find(|(f, ..)| *f == function.name).expect("an entry of the table");
            let rows: BTreeSet<Row> = expected.iter().map(|q| Row::Read(q.to_string())).collect();
            assert_eq!(summary.rows, rows, "{name}: the rows it reads");
            module_rows.extend(expected.iter().copied());
            assert_eq!(function.params.len(), *params, "{name}: the solver's named objects, or nothing");
        }
        assert_eq!(module_rows.into_iter().collect::<Vec<_>>(), module.rows, "{}: the module's rows", module.name);
    }

    let catalogue = [
        (library::rim_and_seam_facts(), RIM, "Rim and seam facts; N", "Jumbling poses"),
        (library::covers_at_a_named_cap(), COVERS, "Covers at a named cap", "Jumbling poses"),
        (library::correction_catalogue(), CORRECTIONS, "Correction catalogue", "Jumbling labelled action"),
        (
            library::admissibility_and_blocked_twist_check(),
            ADMISSIBILITY,
            "Admissibility and blocked-twist check",
            "Jumbling poses",
        ),
        (library::straddlers_and_u_at_a_named_cap(), STRADDLERS, "Straddlers and u at a named cap", "Jumbling poses"),
        (library::units_and_clean_step_contract(), UNITS, "Units and clean-step contract", "Jumbling poses"),
        (library::repair_list_at_a_named_cap(), REPAIR, "Repair list at a named cap", "Jumbling poses"),
        (
            library::retained_checkpoint_label_checkpoint_sticker_test(),
            END_TESTS,
            "Retained checkpoint, label checkpoint, sticker test",
            "Jumbling poses and explicit label projection",
        ),
        (library::mode_crossing_prerequisites(), CROSSING, "Mode crossing prerequisites", "Jumbling poses"),
    ];
    for (row, module, entry, level) in catalogue {
        assert_eq!(row.entry, entry);
        assert!(row.level.starts_with(level), "{entry}");
        assert_eq!(row.effects, &[Effect::ReadSnapshot][..], "{entry}: Effect");
        assert_eq!((row.module, row.functions), (module.name, module.exports), "{entry}");
        assert_eq!(row.source_sha256, sha256_hex(module.source.as_bytes()), "{entry}");
        for function in module.exports {
            assert!(row.backing.contains(&format!("`{}.{function}`", module.name)), "{entry}: Backing");
        }
    }
    assert!(library::rim_and_seam_facts().notes.contains("not the sum of the displayed N_c"));
    assert!(library::correction_catalogue().notes.contains("chooses no combination"));
    assert!(library::mode_crossing_prerequisites().notes.contains("KN4 journal query"), "the Jumble-on half waits");
}

/// CT-LB3-07 (part: the repair entry under the column's repair-list exception, BND.2 and the owner's decision of 17
/// October 2026). `jumble_repair.repair_list_at` is admitted as a built-in entry under a class that lists
/// `repair_list` as the repair list (`c600_check::Row::RepairList`): it reads the list in its own body, for its cap
/// parameter as given, and returns the value as read, so it reads state and prepares no preview. The same entry
/// changed is refused with `LG3:capability.not_granted`: reading the list for a cap it did not take (a literal), in a
/// helper it calls, or using the list's value, by a field or by a condition on it.
#[test]
fn ct_lb3_07_the_repair_entry_is_admitted_only_under_the_exception_as_written() {
    const LIST: &str = "record { input_digest: Text, message: List<Text>, parameters: record { cap: Nat, \
                        contract_revision: Text, menu_identity: List<Text>, order_digest: List<Text>, tie_rule: Text \
                        }, rows: List<record { d: Nat, delta: Int, x: Nat, y: Nat, z: Nat }>, status: Text }";
    let module = |helpers: &str, result: &str, body: &str| {
        format!(
            "edition \"6cs-draft-1\";\nmodule jumble_repair requires [];\n\n{helpers}export fn \
             repair_list_at(cap: Nat) -> {result} effects [snapshot] {{\n{body}}}\n"
        )
    };
    let read = |cap: &str| format!("read_snapshot<{LIST}>(\"repair_list\", {cap})");
    let as_written = module("", LIST, &format!("    return {};\n", read("cap")));
    assert_eq!(as_written, REPAIR.source, "the entry as the library writes it");
    let class = BTreeSet::from([Row::RepairList("repair_list".to_owned())]);
    let admit = |source: &str| {
        let program = lower_source(source).expect("the module lowers");
        admit_built_in(&program, REPAIR.name, "repair_list_at", &class, &Grant::none(), &Policy::interim())
            .expect("well-formed IR")
    };
    let admitted = admit(&as_written);
    assert!(admitted.is_admitted(), "{:#?}", admitted.diagnostics);
    assert_eq!(admitted.separation, Separation { reads_state: true, prepares_previews: false });

    let helper = format!("fn list_at(cap: Nat) -> {LIST} effects [snapshot] {{\n    return {};\n}}\n\n", read("cap"));
    let changed = [
        ("a cap it did not take", module("", LIST, &format!("    return {};\n", read("57")))),
        ("a helper's read", module(&helper, LIST, "    return list_at(cap);\n")),
        ("a field", module("", "Text", &format!("    return {}.status;\n", read("cap")))),
        (
            "a condition",
            module(
                "",
                LIST,
                &format!(
                    "    let answer = {};\n    return match exact_eq(answer.status, \"completed\") {{\n        true => \
                     answer;\n        _ => answer;\n    }};\n",
                    read("cap")
                ),
            ),
        ),
    ];
    for (what, source) in changed {
        let refused = admit(&source);
        assert!(refused.diagnostics.iter().any(|d| d.code == NOT_GRANTED), "{what}: {:#?}", refused.diagnostics);
    }
}

/// CT-LB3-07 (part: "per-revision fact results contain no all-items outcome table", on the rim entry). At the I_a walk
/// after twelve records, `rim_facts` reads `rim_summary` and `seam_facts` with scope `all` once each and no word row.
/// It answers one row for each of the 600 caps, in cap-ID order, with the kernel's facts at this revision: blocked caps
/// keep their row with an undefined rim, and closed and misaligned caps keep theirs, so none is skipped, reordered or
/// selected. Each row's fields are the rim facts and nothing of a twist. The counts of admissible, blocked, closed,
/// misaligned and at-floor caps are the same counts taken apart from the engine's rows, grip N is the kernel's and
/// equals the sum of N_c over the admissible caps, and total N is the seam scan's, the sum of its on-grip and off-grip
/// mismatches. At solved every cap is closed and both N are zero.
#[test]
fn ct_lb3_07_the_rim_entry_holds_no_all_items_outcome_table() {
    for (label, snapshot) in [("walk", walk()), ("solved", &Snapshot::solved(ctx(), Some(fixture().menu.clone())))] {
        let mut host = Recording::on(snapshot);
        let facts = completed(run(RIM.name, "rim_facts", vec![], &mut host), label);
        assert_eq!(host.queries(), ["rim_summary", "seam_facts"], "{label}: the rows read, once each");
        assert_eq!(host.reads[1].1, Some(Json::text("all")), "{label}: the seam scan of every domain");

        let mut state = snapshot.state().clone();
        let summary = rim_summary(ctx(), &mut state, None, None).expect("no engine error");
        let engine_rows = summary.get("rows").and_then(Value::as_array).expect("a row for each cap");
        let caps = items(get(&facts, "caps"));
        assert_eq!(caps.len(), POLES, "{label}: one row for each cap, not one for each twist");
        let (mut admissible, mut blocked, mut closed, mut misaligned, mut at_floor, mut grip) = (0, 0, 0, 0, 0, 0);
        for (index, (row, engine)) in caps.iter().zip(engine_rows).enumerate() {
            assert_eq!(number(get(row, "cap")), index as i64, "{label}: cap-ID order, none skipped");
            let RunValue::Record(record) = row else { panic!("a row is a record") };
            let names: Vec<&str> = record.fields().iter().map(|(name, _)| name.as_str()).collect();
            assert_eq!(names, RIM_ROW, "{label}: cap {index}'s fields");
            for name in ["A_c", "F_c", "In_c", "N_c", "Out_c"] {
                assert_eq!(
                    optional(get(row, name)).map(number),
                    engine_int(engine, name),
                    "{label}: cap {index} {name}"
                );
            }
            for name in ["at_floor", "blocked"] {
                assert_eq!(
                    optional(get(row, name)).map(truth),
                    engine_flag(engine, name),
                    "{label}: cap {index} {name}"
                );
            }
            assert_eq!(Some(string(get(row, "status"))), engine_text(engine, "status"), "{label}: cap {index}");
            assert_eq!(optional(get(row, "undefined")).map(string), engine_text(engine, "undefined"), "{label}");
            match engine_text(engine, "status") {
                Some("admissible") => {
                    let n = engine_int(engine, "N_c").expect("an admissible cap's N_c");
                    admissible += 1;
                    grip += n;
                    if n == 0 {
                        closed += 1;
                    } else {
                        misaligned += 1;
                    }
                    at_floor += i64::from(engine_flag(engine, "at_floor") == Some(true));
                }
                Some("blocked") => blocked += 1,
                other => panic!("{label}: cap {index} has status {other:?} with no budget"),
            }
        }
        let counts =
            ["admissible", "blocked", "closed", "misaligned", "at_floor"].map(|name| number(get(&facts, name)));
        assert_eq!(counts, [admissible, blocked, closed, misaligned, at_floor], "{label}: the counts, taken apart");
        assert_eq!(admissible + blocked, POLES as i64);
        assert_eq!(optional(get(&facts, "grip_n")).map(number), engine_int(&summary, "grip_n"), "{label}: grip N");
        assert_eq!((number(get(&facts, "summed")), engine_int(&summary, "grip_n")), (grip, Some(grip)), "{label}");
        assert!(truth(get(&facts, "complete")) && optional(get(&facts, "grip_n_undefined")).is_none(), "{label}");
        assert_eq!(Some(string(get(&facts, "digest"))), engine_text(&summary, "digest"), "{label}");

        let whole = Batch { chunk: 1 << 12, budget: None, cancel_after: None };
        let seams = seam_facts(ctx(), &mut state, Scope::All, &whole, Cells::None, None).expect("no engine error");
        let totals = seams.get("totals").expect("the totals of a complete scan");
        let total = optional(get(&facts, "totals")).expect("total N");
        for name in ["aligned", "off_grip_mismatches", "on_grip_mismatches", "total_N"] {
            assert_eq!(Some(number(get(total, name))), engine_int(totals, name), "{label}: {name}");
        }
        let (on, off) = (number(get(total, "on_grip_mismatches")), number(get(total, "off_grip_mismatches")));
        assert_eq!(number(get(total, "total_N")), on + off, "{label}: total N counts the off-grip seams too");

        if label == "walk" {
            assert!(blocked > 0 && closed > 0 && misaligned > 0, "{blocked} blocked, {closed} closed, {misaligned}");
        } else {
            assert_eq!((blocked, misaligned, grip, number(get(total, "total_N"))), (0, 0, 0, 0), "solved");
        }
        assert!(!holds_a_word(&facts), "{label}: no word and no letter table");
        assert_no_field(&facts, &LEAKS, label);
        assert_no_field(&facts, &SELECTIONS, label);
        assert_no_field(&facts, &OUTCOMES, label);
    }
}

/// CT-LB3-07 (part: "per-revision fact results contain no all-items outcome table", on the survey row, LB3.5). The
/// survey reaches the language one named grip at a time, never as a table of grips. `grip_status` takes one grip and
/// reads `survey` once with it; at the walk's first closed, first misaligned and first blocked cap, and at solved, it
/// answers exactly `cap` and `status`, the kernel's status of that grip, and nothing of another grip. A blocked grip's
/// certificates come back only from the named cap's own read (`straddlers_at`), each certificate of that grip. An entry
/// written to ask the row for every grip, with the text `"all"`, or for a list of grips, as a table of statuses, checks
/// with no diagnostic and stops with `KN5:input.argument_form` at the row's tariff, so the row never answers and the
/// run has no value; the bridge's answer refuses the same arguments. The bridge classifies the named grip with no
/// budget, so it never serves an incomplete survey: a budget of no exact evaluation would leave each of these grips
/// uncertain, whose survey letter is `u`, never `a` or `b`, while the bridge answers their exact status.
#[test]
fn ct_lb3_07_the_survey_answers_a_named_grip_and_no_table() {
    let (misaligned, blocked) = walk_caps();
    let mut state = walk().state().clone();
    let summary = rim_summary(ctx(), &mut state, None, None).expect("no engine error");
    let rows = summary.get("rows").and_then(Value::as_array).expect("a row for each cap");
    let closed = rows
        .iter()
        .find(|row| engine_flag(row, "blocked") == Some(false) && engine_int(row, "N_c") == Some(0))
        .map(|row| engine_int(row, "cap").expect("a cap ID"))
        .expect("a closed cap");
    let solved = Snapshot::solved(ctx(), Some(fixture().menu.clone()));
    let named = [
        ("closed", walk(), closed, Status::Admissible),
        ("misaligned", walk(), misaligned, Status::Admissible),
        ("blocked", walk(), blocked, Status::Blocked),
        ("solved", &solved, blocked, Status::Admissible),
    ];
    for (label, snapshot, grip, expected) in named {
        let mut host = Recording::on(snapshot);
        let answer = completed(run(ADMISSIBILITY.name, "grip_status", vec![RunValue::int(grip)], &mut host), label);
        assert_eq!(host.queries(), ["survey"], "{label}: one read");
        assert_eq!(read_int(&host.reads[0].1), Some(grip), "{label}: the named grip, as given");
        let RunValue::Record(record) = &answer else { panic!("{label}: a record") };
        let names: Vec<&str> = record.fields().iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["cap", "status"], "{label}: the named grip's status and nothing else");
        let engine = snapshot.state().clone().survey(ctx(), &[grip as usize], None).expect("no engine error");
        assert_eq!(engine, [expected], "{label}: grip {grip}");
        assert_eq!((number(get(&answer, "cap")), string(get(&answer, "status"))), (grip, expected.as_str()), "{label}");
    }

    let mut host = Recording::on(walk());
    let set = completed(run(STRADDLERS.name, "straddlers_at", vec![RunValue::int(blocked)], &mut host), "straddlers");
    assert_eq!(host.queries(), ["straddlers"], "the blocked grip's certificates: one read");
    assert_eq!(read_int(&host.reads[0].1), Some(blocked), "the named grip, as given");
    let certificates = items(get(&set, "certificates"));
    assert!(!certificates.is_empty(), "a blocked grip has a certified straddler");
    for certificate in certificates {
        assert_eq!(number(get(certificate, "grip")), blocked, "a certificate of the named grip only");
    }

    const TABLE: &str = "List<record { cap: Nat, status: Text }>";
    let source = format!(
        "edition \"6cs-draft-1\";\nmodule survey_table requires [];\n\nexport fn every_grip(what: Text) -> {TABLE} \
         effects [snapshot] {{\n    return read_snapshot<{TABLE}>(\"survey\", what);\n}}\n\nexport fn \
         these_grips(grips: List<Nat>) -> {TABLE} effects [snapshot] {{\n    return \
         read_snapshot<{TABLE}>(\"survey\", grips);\n}}\n"
    );
    let program = lower_source(&source).expect("the module lowers");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let asking = load(&program).expect("the module loads");
    let pinned = fixture().pinned();
    let options = Options { menu: Some(&pinned), ..Options::new(WIDE) };
    let grips = RunValue::list(vec![RunValue::int(misaligned), RunValue::int(blocked)]);
    for (function, arg) in [("every_grip", RunValue::text("all")), ("these_grips", grips)] {
        let mut host = Recording::on(walk());
        let refused = asking
            .run("survey_table", function, vec![arg], &mut host, &options)
            .unwrap_or_else(|r| panic!("{function}: refused: {r:?}"));
        let stopped = (refused.summary.outcome, refused.summary.code);
        assert_eq!(stopped, (Outcome::Error, Some(ARGUMENT_FORM)), "{function}");
        assert!(refused.value.is_none(), "{function}: no table");
        assert!(host.queries().is_empty(), "{function}: refused at the tariff, so the row never answers");
    }
    let mut rows = Rows::on(ctx(), Base::Snapshot(walk()));
    let list = format!(r#"[{{"int":"{misaligned}"}},{{"int":"{blocked}"}}]"#);
    for arg in [Json::text("all"), provenance::parse(list.as_bytes()).expect("a list of grips")] {
        assert_eq!(rows.answer("survey", Some(&arg)).err(), Some(ARGUMENT_FORM), "the bridge's answer: {arg:?}");
    }

    let grips = [closed, misaligned, blocked].map(|grip| grip as usize);
    let bounded = walk().state().clone().survey(ctx(), &grips, Some(0)).expect("no engine error");
    assert_eq!(bounded, [Status::Uncertain; 3], "a budget of no exact evaluation leaves each grip uncertain");
    assert!(bounded.iter().all(|status| status.letter() == "u"), "an uncertain grip is never a or b");
}

/// The first misaligned admissible cap and the first blocked cap of the walk, in cap-ID order.
fn walk_caps() -> (i64, i64) {
    let mut state = walk().state().clone();
    let summary = rim_summary(ctx(), &mut state, None, None).expect("no engine error");
    let rows = summary.get("rows").and_then(Value::as_array).expect("a row for each cap");
    let cap = |row: &Value| engine_int(row, "cap").expect("a cap ID");
    let misaligned =
        rows.iter().find(|row| engine_int(row, "N_c").is_some_and(|n| n > 0)).map(cap).expect("misaligned");
    let blocked = rows.iter().find(|row| engine_flag(row, "blocked") == Some(true)).map(cap).expect("a blocked cap");
    (misaligned, blocked)
}

/// CT-LB3-07 (part: "no recommendation or next-cap skip", on the covers entry). At the walk's first misaligned cap c,
/// `covers_at(c)` reads `covers` once, with c as its argument. It answers every cap meeting c, in the kernel's domain
/// and cap-ID order, blocked and closed meeting caps included: none is skipped, marked or reordered. Each direction's
/// counts and straddlers are the kernel's, "c covers d" is not applicable at a blocked or closed d with the kernel's
/// reason, and the face-piece definition the entry recomputes from the counts and the kernel's threshold is the
/// kernel's flag. A blocked named cap answers its status and message and no answers; a cap outside the model stops the
/// run with `KN2:input.named_object_invalid`. No field names a recommendation, selection or outcome.
#[test]
fn ct_lb3_07_covers_answer_the_named_cap_in_cap_order() {
    let (c, blocked) = walk_caps();
    let mut host = Recording::on(walk());
    let table = completed(run(COVERS.name, "covers_at", vec![RunValue::int(c)], &mut host), "covers_at");
    assert_eq!(host.queries(), ["covers"]);
    assert_eq!(host.reads[0].1.as_ref().and_then(|arg| arg.get("int")).and_then(Json::as_text), Some(&*c.to_string()));

    let mut state = walk().state().clone();
    let engine = covers(ctx(), &mut state, c, None).expect("no engine error");
    assert_eq!((string(get(&table, "status")), number(get(&table, "cap"))), ("completed", c));
    assert_eq!(Some(string(get(&table, "input_digest"))), engine_text(&engine, "input_digest"));
    assert!(optional(get(&table, "message")).is_none());
    let domain: Vec<i64> = engine
        .get("domain")
        .and_then(Value::as_array)
        .expect("a domain")
        .iter()
        .map(|d| d.as_i64().expect("a cap ID"))
        .collect();
    assert_eq!(numbers(get(&table, "domain")), domain);
    assert!(domain.len() == 80 && domain.windows(2).all(|w| w[0] < w[1]) && !domain.contains(&c));
    let answers = items(get(&table, "answers"));
    let engine_answers = engine.get("answers").and_then(Value::as_array).expect("answers");
    let caps: Vec<i64> = answers.iter().map(|row| number(get(row, "cap"))).collect();
    assert_eq!(caps, domain, "every meeting cap, in cap-ID order");
    let mut not_applicable = 0;
    for (row, engine_row) in answers.iter().zip(engine_answers) {
        let d = number(get(row, "cap"));
        for direction in ["d_covers_c", "c_covers_d"] {
            let (ours, theirs) = (get(row, direction), engine_row.get(direction).expect("both directions"));
            let applicable = truth(get(ours, "applicable"));
            assert_eq!(applicable, theirs.get("counts").is_some(), "cap {d} {direction}");
            assert_eq!(optional(get(ours, "reason")).map(string), engine_text(theirs, "reason"), "cap {d} {direction}");
            if !applicable {
                not_applicable += 1;
                assert!(!truth(get(ours, "covers")) && items(get(ours, "straddlers")).is_empty(), "cap {d}");
                continue;
            }
            let counts = theirs.get("counts").expect("counts");
            for name in ["inside", "outside", "aligned_inside", "aligned_outside"] {
                assert_eq!(Some(number(get(ours, name))), engine_int(counts, name), "cap {d} {direction} {name}");
            }
            assert_eq!(Some(truth(get(ours, "covers"))), engine_flag(theirs, "covers"), "cap {d} {direction}");
            let straddlers: Vec<i64> = theirs
                .get("straddlers")
                .and_then(Value::as_array)
                .expect("straddlers")
                .iter()
                .map(|p| p.as_i64().expect("a piece"))
                .collect();
            assert_eq!(numbers(get(ours, "straddlers")), straddlers, "cap {d} {direction}");
        }
    }
    assert!(not_applicable > 0, "a blocked or closed meeting cap keeps its row");
    assert!(!holds_a_word(&table));
    assert_no_field(&table, &LEAKS, "covers_at");
    assert_no_field(&table, &SELECTIONS, "covers_at");
    assert_no_field(&table, &OUTCOMES, "covers_at");

    let mut host = Recording::on(walk());
    let at_blocked = completed(run(COVERS.name, "covers_at", vec![RunValue::int(blocked)], &mut host), "blocked");
    assert_eq!(string(get(&at_blocked, "status")), "error");
    assert_eq!(optional(get(&at_blocked, "message")).map(string), Some(&*format!("cap {blocked}: blocked")));
    assert!(items(get(&at_blocked, "answers")).is_empty(), "no answer about any meeting cap");
    let mut host = Recording::on(walk());
    let outside = run(COVERS.name, "covers_at", vec![RunValue::int(600)], &mut host);
    assert_eq!((outside.summary.outcome, outside.summary.code), (Outcome::Error, Some(codes::NAMED_OBJECT_INVALID)));
    assert_eq!(host.queries(), ["covers"], "the named cap only");
}

/// CT-LB3-07 (part: "no correction combination leaks", on the catalogue entry and the drafted check). On the solved
/// I_a snapshot, `correction(id)` reads `correction_lookup` once with the solver's id and answers that correction only:
/// its id, family, literal letters and fixed ν row are the catalogue's, its length is its letter count and its support
/// the row's nonzero count. An id the catalogue does not ship stops the run with `KN2:input.named_object_invalid`. The
/// letters number I_a's items in its recorded order, so under the S4 menu or in free mode the lookup stops with
/// `KN2:input.menu_binding_mismatch` and answers no letters. No lookup checks a set. `drafted_check` checks exactly the
/// sets the solver drafts, in order, and the empty draft answers ν itself, not a set that would cancel it. No answer
/// names a recommendation or a selection.
#[test]
fn ct_lb3_07_a_correction_is_looked_up_by_its_named_id_only() {
    let solved = Snapshot::solved(ctx(), Some(fixture().menu.clone()));
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    for id in [0u32, 2, 27, 35] {
        let mut host = Recording::on(&solved);
        let found = completed(run(CORRECTIONS.name, "correction", vec![RunValue::int(id)], &mut host), "correction");
        assert_eq!(host.queries(), ["correction_lookup"], "id {id}: one read");
        let arg = host.reads[0].1.as_ref().and_then(|arg| arg.get("int")).and_then(Json::as_text);
        assert_eq!(arg, Some(&*id.to_string()), "id {id}: the solver's id, as given");
        assert!(host.rows.drafts().is_empty(), "id {id}: a lookup checks no set");
        let shipped = catalogue.lookup(id).expect("a shipped id");
        assert_eq!((number(get(&found, "id")), string(get(&found, "family"))), (i64::from(id), shipped.family()));
        let letters: Vec<Vec<i64>> = items(get(&found, "letters")).iter().map(numbers).collect();
        let expected: Vec<Vec<i64>> =
            shipped.letters().iter().map(|&(grip, item)| vec![i64::from(grip), i64::from(item)]).collect();
        assert_eq!(letters, expected, "id {id}: the shipped literal word");
        assert_eq!(number(get(&found, "length")), expected.len() as i64);
        let row = shipped.row().values().to_vec();
        assert_eq!(numbers(get(&found, "row")), row, "id {id}: the fixed ν row");
        assert_eq!(number(get(&found, "support")), row.iter().filter(|&&v| v != 0).count() as i64);
        assert_no_field(&found, &LEAKS, "correction");
        assert_no_field(&found, &SELECTIONS, "correction");
    }
    let mut host = Recording::on(&solved);
    let unshipped = run(CORRECTIONS.name, "correction", vec![RunValue::int(36)], &mut host);
    assert_eq!(
        (unshipped.summary.outcome, unshipped.summary.code),
        (Outcome::Error, Some(codes::NAMED_OBJECT_INVALID))
    );
    // The letters number I_a's items in its recorded order: under the S4 menu, or in free mode, the lookup refuses.
    let bytes = std::fs::read(repository().join("research/jumbling/fixtures/wj-S4.json")).expect("the S4 fixture");
    let doc = json::parse(&bytes).expect("JSON");
    let s4 = Menu::from_record(ctx(), doc.get("menu").expect("a menu")).expect("the S4 fixture's pinned menu");
    for (label, other) in [("S4", Snapshot::solved(ctx(), Some(Arc::new(s4)))), ("free", Snapshot::solved(ctx(), None))]
    {
        let mut host = Recording::on(&other);
        let refused = run(CORRECTIONS.name, "correction", vec![RunValue::int(2u32)], &mut host);
        let stopped = (refused.summary.outcome, refused.summary.code);
        assert_eq!(stopped, (Outcome::Error, Some(codes::MENU_BINDING_MISMATCH)), "{label}");
        assert!(refused.value.is_none(), "{label}: no letters under another menu");
    }

    // The drafted check reads the solver's sets only, as drafted.
    let draft = |set: &[(u32, i64)]| {
        let entry = |&(id, m): &(u32, i64)| RunValue::record(vec![("id", RunValue::int(id)), ("m", RunValue::int(m))]);
        RunValue::list(set.iter().map(entry).collect())
    };
    let sets: [&[(u32, i64)]; 3] = [&[], &[(2, 1)], &[(27, 2), (0, -1)]];
    for set in sets {
        let mut host = Recording::on(&solved);
        let checked = completed(run(NU.name, "drafted_check", vec![draft(set)], &mut host), "drafted_check");
        assert_eq!(host.queries(), ["nu", "correction_set_check"]);
        assert_eq!(host.rows.drafts(), [set.to_vec()], "the solver's set, once, as drafted");
        let predicted = catalogue.predict(&c600_jumble::nu::NuVector::ZERO, set).expect("shipped ids");
        let values: Vec<i64> = predicted.values().to_vec();
        assert_eq!(numbers(get(&checked, "predicted")), values, "ν at solved is zero, so the set's own sum");
        if set.is_empty() {
            assert!(values.iter().all(|&v| v == 0), "the empty draft is ν itself");
        }
        assert_no_field(&checked, &LEAKS, "drafted_check");
        assert_no_field(&checked, &SELECTIONS, "drafted_check");
    }
}

/// CT-LB3-07 (part: "named-word checks answer only their named word", on the library's word entries). At the walk,
/// the named word is the fixture's next five records, as a menu word of the pinned alphabet. `word_observations` and
/// `step_flag` each read their row once, with the named word as its argument, letter for letter; `word_observations`
/// plays exactly its letters, and `step_flag` judges the named unit only. Neither answer holds a word or a letter
/// table, and no field names a recommendation, a next cap, a ranking or a correction.
#[test]
fn ct_lb3_07_named_word_checks_answer_only_their_named_word() {
    let fixture = fixture();
    let pairs = fixture.pairs(&fixture.twists(12..17));
    let word = fixture.alphabet().word(&pairs).expect("letters of the menu");
    let pinned = fixture.pinned();
    let letters: Vec<[u32; 2]> = word.pairs().iter().map(|&[cap, item]| [cap as u32, item as u32]).collect();
    let named = RunValue::menu_word(pinned.shared_identity(), &letters).expect("a menu word");
    let state = walk().state();
    let off = (0..NP).find(|&p| !state.is_lattice_piece(p)).expect("a piece off the lattice");
    let pose = state.pose_ids()[off];
    let unit: Vec<u32> =
        (0..NP).filter(|&p| !state.is_lattice_piece(p) && state.pose_ids()[p] == pose).map(|p| p as u32).collect();
    let unit_value = RunValue::list(unit.iter().map(|&p| RunValue::int(p)).collect());
    let expected: Vec<[i64; 2]> = pairs.iter().map(|&[cap, item]| [i64::from(cap), i64::from(item)]).collect();

    let mut host = Recording::on(walk());
    let observed = completed(run(WORD_CHECKS.name, "word_observations", vec![named.clone()], &mut host), "observe");
    assert_eq!(host.queries(), ["replay"], "one read");
    assert_eq!(read_letters(host.reads[0].1.as_ref().expect("an argument")), expected, "the named word, as given");
    assert!(truth(get(&observed, "complete")));
    assert_eq!(number(get(&observed, "played")), pairs.len() as i64, "the named word's letters, no more");

    let mut host = Recording::on(walk());
    let flagged = completed(run(WORD_CHECKS.name, "step_flag", vec![named, unit_value], &mut host), "step_flag");
    assert_eq!(host.queries(), ["word_check"], "one read");
    let arg = host.reads[0].1.as_ref().expect("an argument");
    assert_eq!(read_letters(arg.get("word").expect("the word")), expected, "the named word, as given");
    let Some(Json::List(read_unit)) = arg.get("unit") else { panic!("the named unit") };
    let read_unit: Vec<String> =
        read_unit.iter().filter_map(|p| p.get("int").and_then(Json::as_text)).map(String::from).collect();
    assert_eq!(read_unit, unit.iter().map(u32::to_string).collect::<Vec<_>>(), "the named unit, as given");

    for (name, answer) in [("word_observations", &observed), ("step_flag", &flagged)] {
        assert!(!holds_a_word(answer), "{name}: no word and no letter table");
        assert_no_field(answer, &LEAKS, name);
    }
}

/// CT-LB3-07 (part: "named-word checks answer only their named word", on the admissibility entries). At the walk,
/// `twist_check` reads `classify` once, with the solver's one letter as its argument, and answers that twist only, as
/// `State::apply` answers it on a copy of the state: a letter at the walk's first blocked cap is blocked, moves nothing
/// and carries the engine's certificate (the piece, the cut, both vertices and both exact points, each coordinate an
/// exact number of Q(√5)); the fixture's next letter is admissible, moves the engine's inside set and carries none.
/// `grip_status` reads `survey` once with the solver's named grip and answers that grip's status, the engine's; a grip
/// outside the model stops the run with `KN2:input.named_object_invalid` after the one read. No answer holds a word or
/// names a recommendation or a selection.
#[test]
fn ct_lb3_07_a_named_twist_and_grip_are_checked_alone() {
    let fixture = fixture();
    let (misaligned, blocked) = walk_caps();
    let next = fixture.pairs(&fixture.twists(12..13))[0];
    for (label, pair) in [("blocked", [blocked as i32, 0]), ("next", next)] {
        let mut host = Recording::on(walk());
        let checked = completed(run(ADMISSIBILITY.name, "twist_check", vec![menu_word(&[pair])], &mut host), label);
        assert_eq!(host.queries(), ["classify"], "{label}: one read");
        let letters = read_letters(host.reads[0].1.as_ref().expect("an argument"));
        assert_eq!(letters, [[i64::from(pair[0]), i64::from(pair[1])]], "{label}: the named letter, as given");
        let twist = fixture.menu.for_cap(ctx(), pair[0] as usize).expect("the menu's items").remove(pair[1] as usize);
        let outcome = walk().state().clone().apply(ctx(), &twist, None, &[]).expect("no engine error");
        assert_eq!(string(get(&checked, "status")), outcome.status.as_str(), "{label}");
        let counts = ["grip", "moved", "uncertain"].map(|name| number(get(&checked, name)));
        assert_eq!(counts, [outcome.grip as i64, outcome.moved() as i64, outcome.uncertain as i64], "{label}");
        let code = outcome.refusal.as_ref().map(|(code, _)| *code);
        assert_eq!(optional(get(&checked, "code")).map(string), code, "{label}");
        match (optional(get(&checked, "certificate")), &outcome.straddle) {
            (Some(ours), Some(theirs)) => same_certificate(ours, theirs, label),
            (None, None) => {}
            (ours, theirs) => panic!("{label}: certificate {ours:?}, the engine's {theirs:?}"),
        }
        if label == "blocked" {
            assert_eq!((outcome.status, outcome.moved()), (Status::Blocked, 0), "a blocked twist moves nothing");
            assert!(outcome.straddle.is_some(), "a blocked twist has a certified straddler");
        } else {
            assert_eq!(outcome.status, Status::Admissible);
            assert!(outcome.moved() > 0 && outcome.straddle.is_none());
        }
        assert!(!holds_a_word(&checked), "{label}: no word and no letter table");
        assert_no_field(&checked, &LEAKS, label);
        assert_no_field(&checked, &SELECTIONS, label);
    }

    for (cap, expected) in [(misaligned, Status::Admissible), (blocked, Status::Blocked)] {
        let mut host = Recording::on(walk());
        let status = completed(run(ADMISSIBILITY.name, "grip_status", vec![RunValue::int(cap)], &mut host), "grip");
        assert_eq!(host.queries(), ["survey"], "grip {cap}: one read");
        assert_eq!(read_int(&host.reads[0].1), Some(cap), "grip {cap}: the named grip, as given");
        let engine = walk().state().clone().survey(ctx(), &[cap as usize], None).expect("no engine error");
        assert_eq!(engine, [expected], "grip {cap}");
        assert_eq!((number(get(&status, "cap")), string(get(&status, "status"))), (cap, expected.as_str()));
        assert_no_field(&status, &LEAKS, "grip_status");
        assert_no_field(&status, &SELECTIONS, "grip_status");
        assert_no_field(&status, &OUTCOMES, "grip_status");
    }
    let mut host = Recording::on(walk());
    let outside = run(ADMISSIBILITY.name, "grip_status", vec![RunValue::int(POLES as i64)], &mut host);
    assert_eq!((outside.summary.outcome, outside.summary.code), (Outcome::Error, Some(codes::NAMED_OBJECT_INVALID)));
    assert_eq!(host.queries(), ["survey"], "the named grip only");
}

/// CT-LB3-07 (part: "named-word checks answer only their named word", on the straddlers entry). `straddlers_at(c)`
/// reads `straddlers` once with the solver's named cap c and answers the engine's straddler set at c: its pieces in
/// ascending order, one certificate each, field for field, its status, u the number of its pieces, and complete while
/// no piece is uncertain. At the walk's first blocked cap u is positive; at its first misaligned admissible cap and at
/// solved there is no straddler and u is zero. A cap outside the model stops the run with
/// `KN2:input.named_object_invalid` after the one read. No answer holds a word or names a recommendation, a selection
/// or a twist's outcome.
#[test]
fn ct_lb3_07_straddlers_and_u_answer_the_named_cap() {
    let (misaligned, blocked) = walk_caps();
    let solved = Snapshot::solved(ctx(), Some(fixture().menu.clone()));
    for (label, snapshot, cap) in
        [("blocked", walk(), blocked), ("misaligned", walk(), misaligned), ("solved", &solved, 0)]
    {
        let mut host = Recording::on(snapshot);
        let set = completed(run(STRADDLERS.name, "straddlers_at", vec![RunValue::int(cap)], &mut host), label);
        assert_eq!(host.queries(), ["straddlers"], "{label}: one read");
        assert_eq!(read_int(&host.reads[0].1), Some(cap), "{label}: the named cap, as given");
        let engine = snapshot.state().clone().straddler_set(ctx(), cap as usize, None).expect("no engine error");
        let pieces: Vec<i64> = engine.pieces.iter().map(|&p| i64::from(p)).collect();
        assert_eq!(numbers(get(&set, "pieces")), pieces, "{label}: the engine's straddlers");
        assert!(pieces.windows(2).all(|w| w[0] < w[1]), "{label}: ascending");
        assert_eq!(number(get(&set, "u")), pieces.len() as i64, "{label}: u counts the straddlers");
        assert_eq!((number(get(&set, "cap")), string(get(&set, "status"))), (cap, engine.status()), "{label}");
        assert_eq!(truth(get(&set, "complete")), engine.uncertain == 0, "{label}");
        let certificates = items(get(&set, "certificates"));
        assert_eq!(certificates.len(), engine.certificates.len(), "{label}: one certificate each");
        for (index, (ours, theirs)) in certificates.iter().zip(&engine.certificates).enumerate() {
            same_certificate(ours, theirs, &format!("{label}: certificate {index}"));
        }
        assert_eq!(pieces.is_empty(), label != "blocked", "{label}: {} straddlers", pieces.len());
        assert!(!holds_a_word(&set), "{label}: no word and no letter table");
        assert_no_field(&set, &LEAKS, label);
        assert_no_field(&set, &SELECTIONS, label);
        assert_no_field(&set, &OUTCOMES, label);
    }
    let mut host = Recording::on(walk());
    let outside = run(STRADDLERS.name, "straddlers_at", vec![RunValue::int(POLES as i64)], &mut host);
    assert_eq!((outside.summary.outcome, outside.summary.code), (Outcome::Error, Some(codes::NAMED_OBJECT_INVALID)));
    assert_eq!(host.queries(), ["straddlers"], "the named cap only");
}

/// CT-LB3-07 (part: units and the clean-step contract). At the walk, `off_units` reads `units` once with "all" and
/// answers the engine's units, the exact-pose groups of the off pieces ordered by smallest member, each with its
/// members, pose, domain, heights and branches, and their count; `named_unit` reads `units` once with the solver's
/// pieces and answers that unit only. Pieces on the lattice, or of two poses, stop the run with
/// `KN2:input.named_object_invalid`. `clean_step(w, U)` reads `word_check` once with the solver's word and unit, as
/// given, and answers the engine's whole-word check of them: whether every letter was admissible, the letters applied,
/// the lattice pieces lifted off, the other off pieces kept and landed, those changed, and whether U landed. It holds
/// when the word is admissible, U lands, no lattice piece is lifted off and every other off piece keeps its pose or
/// lands, which is LB3.8's step flag: the inverse of the walk's journal is clean for U and the fixture's next five
/// letters are not, and a word whose first letter is blocked answers nothing past its refused prefix.
#[test]
fn ct_lb3_07_units_and_clean_steps_answer_the_named_unit_and_word() {
    let fixture = fixture();
    let state = walk().state();
    let mut host = Recording::on(walk());
    let all = completed(run(UNITS.name, "off_units", vec![], &mut host), "off_units");
    assert_eq!(host.queries(), ["units"], "one read");
    assert_eq!(host.reads[0].1, Some(Json::text("all")), "every off unit");
    let engine = units(ctx(), state, None).expect("no engine error");
    let engine_units = engine.get("units").and_then(Value::as_array).expect("the engine's units");
    let ours = items(get(&all, "units"));
    assert_eq!(ours.len(), engine_units.len());
    assert!(ours.len() > 1 && truth(get(&all, "complete")), "{} units", ours.len());
    assert_eq!(number(get(&all, "count")), ours.len() as i64);
    for (index, (unit, theirs)) in ours.iter().zip(engine_units).enumerate() {
        same_unit(unit, theirs, &format!("unit {index}"));
    }
    let members: Vec<Vec<i64>> = ours.iter().map(|unit| numbers(get(unit, "members"))).collect();
    assert!(members.windows(2).all(|w| w[0][0] < w[1][0]), "ordered by smallest member");
    let off = (0..NP).filter(|&p| !state.is_lattice_piece(p)).count();
    assert_eq!(members.iter().map(Vec::len).sum::<usize>(), off, "every off piece in one unit");
    assert_no_field(&all, &LEAKS, "off_units");
    assert_no_field(&all, &SELECTIONS, "off_units");
    assert_no_field(&all, &OUTCOMES, "off_units");

    let mut host = Recording::on(walk());
    let named = completed(run(UNITS.name, "named_unit", vec![piece_list(&members[1])], &mut host), "named_unit");
    assert_eq!(host.queries(), ["units"], "one read");
    assert_eq!(read_ints(host.reads[0].1.as_ref().expect("an argument")), members[1], "the named unit, as given");
    same_unit(&named, &engine_units[1], "the named unit");
    let lattice = (0..NP).find(|&p| state.is_lattice_piece(p)).expect("a piece on the lattice") as i64;
    for (label, pieces) in [("on the lattice", vec![lattice]), ("two poses", vec![members[0][0], members[1][0]])] {
        let mut host = Recording::on(walk());
        let refused = run(UNITS.name, "named_unit", vec![piece_list(&pieces)], &mut host);
        let stopped = (refused.summary.outcome, refused.summary.code);
        assert_eq!(stopped, (Outcome::Error, Some(codes::NAMED_OBJECT_INVALID)), "{label}");
        assert_eq!(host.queries(), ["units"], "{label}: the named pieces only");
    }

    let unit = &members[0];
    let (_, blocked) = walk_caps();
    let alphabet = fixture.alphabet();
    let journal = alphabet.word(&fixture.pairs(&fixture.twists(0..12))).expect("letters of the menu");
    let next = alphabet.word(&fixture.pairs(&fixture.twists(12..17))).expect("letters of the menu");
    let refused = alphabet.word(&[[blocked as i32, 0]]).expect("a letter of the menu");
    for (label, word, clean) in
        [("inverse", journal.inverse(), true), ("next", next, false), ("blocked", refused, false)]
    {
        let mut host = Recording::on(walk());
        let step =
            completed(run(UNITS.name, "clean_step", vec![menu_word(word.pairs()), piece_list(unit)], &mut host), label);
        assert_eq!(host.queries(), ["word_check"], "{label}: one read");
        let arg = host.reads[0].1.as_ref().expect("an argument");
        let pairs: Vec<[i64; 2]> = word.pairs().iter().map(|&[cap, item]| [i64::from(cap), i64::from(item)]).collect();
        assert_eq!(read_letters(arg.get("word").expect("the word")), pairs, "{label}: the named word, as given");
        assert_eq!(&read_ints(arg.get("unit").expect("the unit")), unit, "{label}: the named unit, as given");

        let letters = word.letters(ctx(), state).expect("letters of the state's menu");
        let check = word_check(ctx(), state, &letters, Some(unit.as_slice()), None, None).expect("no engine error");
        assert_eq!(Some(truth(get(&step, "admissible"))), engine_flag(&check, "admissible"), "{label}");
        assert_eq!(Some(number(get(&step, "applied"))), engine_int(&check, "applied_letters"), "{label}");
        assert_eq!(Some(number(get(&step, "others"))), engine_int(&check, "others"), "{label}");
        assert_eq!(&numbers(get(&step, "unit")), unit, "{label}: the named unit");
        let ends = [("lattice_lifted", "lattice_pieces_lifted_off"), ("others_kept", "others_exact")];
        for (ours, theirs) in ends.into_iter().chain([("others_landed", "others_landed")]) {
            assert_eq!(optional(get(&step, ours)).map(number), engine_int(&check, theirs), "{label}: {ours}");
        }
        assert_eq!(optional(get(&step, "unit_landed")).map(truth), engine_flag(&check, "unit_landed"), "{label}");
        let changed = match check.get("others_changed_ids") {
            Some(Value::Null) => Vec::new(),
            Some(_) => engine_ints(&check, "others_changed_ids"),
            None => panic!("{label}: no field `others_changed_ids`"),
        };
        assert_eq!(numbers(get(&step, "others_changed")), changed, "{label}");
        let holds = truth(get(&step, "holds"));
        assert_eq!(Some(holds), engine_flag(&check, "step_flag"), "{label}: LB3.8's step flag, recomputed");
        assert_eq!(holds, clean, "{label}");
        if label == "blocked" {
            assert!(!truth(get(&step, "admissible")) && number(get(&step, "applied")) == 0, "a refused first letter");
            assert!(optional(get(&step, "unit_landed")).is_none(), "nothing past the refused prefix");
        }
        assert!(!holds_a_word(&step), "{label}: no word and no letter table");
        assert_no_field(&step, &LEAKS, label);
        assert_no_field(&step, &SELECTIONS, label);
    }
}

/// CT-LB3-07 (part: LB3.1's "Mode crossing prerequisites", the Jumble-off half). `jumble_crossing.jumble_off_facts`
/// reads the label checkpoint, every unit off the lattice and ν once each, in that order, and answers A5's
/// prerequisites of Jumble off, each the kernel's fact computed apart: the label checkpoint's outcome as its own
/// verdict type, the count of pieces off the lattice, and whether ν is defined with the count of its nonzero values. On
/// the I_a walk the pieces off the lattice fail the checkpoint and leave ν undefined; at solved, witnessed by the empty
/// journal, the checkpoint holds with every piece on the lattice and ν = 0. The answer holds no word, no correction and
/// no field of a recommendation, a selection or a twist's outcome.
#[test]
fn ct_lb3_07_the_crossing_entry_reports_the_jumble_off_prerequisites() {
    let solved = fixture().after(0);
    for (label, snapshot, outcome) in
        [("the walk", walk(), EndTestOutcome::Fails), ("solved", &solved, EndTestOutcome::Holds)]
    {
        let state = snapshot.state();
        let mut host = Recording::on(snapshot);
        let facts = completed(run(CROSSING.name, "jumble_off_facts", vec![], &mut host), label);
        assert_eq!(host.queries(), ["label_checkpoint", "units", "nu"], "{label}: one read each");
        assert_eq!(host.reads[1].1, Some(Json::text("all")), "{label}: every off unit");
        let checkpoint = get(&facts, "label_checkpoint");
        let RunValue::EndTest(test, given) = checkpoint else { panic!("{label}: a verdict, not {checkpoint:?}") };
        assert_eq!((*test, *given), (EndTest::LabelCheckpoint, outcome), "{label}: the outcome alone");
        let off = number(get(&facts, "off_lattice"));
        assert_eq!(off, state.off_lattice_count() as i64, "{label}: the pieces off the lattice");
        let values = match nu::nu(ctx(), Base::Snapshot(snapshot)).expect("no engine error").expect("no refusal") {
            Partial::Total(value) => Some(value.values()),
            Partial::Undefined(_) => None,
            other => panic!("{label}: {other:?}"),
        };
        assert_eq!(truth(get(&facts, "nu_defined")), values.is_some(), "{label}: ν is defined as the row says");
        let nonzero = values.map_or(0, |values| values.iter().filter(|&&v| v != 0).count());
        assert_eq!(number(get(&facts, "nu_nonzero")), nonzero as i64, "{label}: the nonzero values of ν");
        match outcome {
            EndTestOutcome::Holds => {
                assert!(off == 0 && values.is_some() && nonzero == 0, "{label}: every prerequisite")
            }
            _ => assert!(off > 0 && values.is_none(), "{label}: pieces off the lattice and no ν"),
        }
        assert!(!holds_a_word(&facts), "{label}: no word and no letter table");
        assert_no_field(&facts, &LEAKS, label);
        assert_no_field(&facts, &SELECTIONS, label);
        assert_no_field(&facts, &OUTCOMES, label);
    }
}
