//! CT-LB3-07 (chapter 03, LB3.5), the inspector library entries in the language (LP3, C8): "Per-revision fact results
//! contain no all-items outcome table. Named-word checks answer only their named word; no recommendation, next-cap
//! skip or correction combination leaks through the inspector entries."
//!
//! The entries are the modules of `c600_jumble_tool::library` that LB6.5's candidate jumbling inspectors use: LB3.1's
//! "Rim and seam facts; N" (`jumble_rim`), "Covers at a named cap" (`jumble_covers`) and "Correction catalogue"
//! (`jumble_corrections`), new here, and the named-word checks (`jumble_word_checks`) and the drafted correction-set
//! check (`jumble_nu`). Each runs through LP2's interpreter against C8's host bridge (`c600_jumble_tool::rows`) on a
//! witnessed snapshot of the I_a walk, and each answer is compared with `c600_jumble`'s, computed apart. A recording
//! host keeps every row the run reads with its argument, so a test sees what an entry asked as well as what it answers.
//!
//! - The rim entry answers one row for every cap, in cap-ID order, blocked and closed caps included, each with the
//!   kernel's facts at this revision and nothing about a twist; it reads the rim summary and the seam scan once each
//!   and no word row. Its counts and grip N are the same facts summed apart.
//! - The covers entry reads the solver's named cap once and answers every meeting cap in cap-ID order, with no mark,
//!   rank or selection, and recomputes the face-piece definition from the kernel's counts.
//! - The catalogue entry reads the solver's id once and answers that correction only, under I_a's menu only; the
//!   drafted check reads exactly the sets the solver drafted.
//! - The named-word checks read their row once, with the named word as its argument, and answer no word.
//!
//! No answer has a field whose name is a recommendation, a next cap, a ranking, a selection or a correction (a check of
//! names), and the new entries hold no field of a twist's outcome. Under capability separation every entry reads state
//! and prepares no preview. `tests/lb3.rs` runs the kernel half of CT-LB3-07 and `c600-tools/tests/lb3_checker.rs`
//! the checker half. A pass is source and fixture evidence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::{Effect, Expr};
use c600_jumble::corrections;
use c600_jumble::levels::{Base, Snapshot};
use c600_jumble::model::{NP, POLES};
use c600_jumble::observe::cuts::{covers, rim_summary};
use c600_jumble::observe::{seam_facts, Batch, Rows as Cells, Scope};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::library::{self, Module, CORRECTIONS, COVERS, NU, RIM, WORD_CHECKS};
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

/// CT-LB3-07 (part: the inspector entries' sources). `jumble_rim`, `jumble_covers` and `jumble_corrections` parse with
/// no diagnostic, keep the canonical layout and LF line endings, request no capability and check with no diagnostic.
/// Each exported entry has the one effect `snapshot` and reads only its rows: under capability separation it reads
/// state and prepares no preview. No function names a proposal target, the helpers are pure, and the covers and
/// catalogue entries take one named cap or one id, nothing more. The catalogue rows name the entries, the effect and
/// the digest of each module's source.
#[test]
fn ct_lb3_07_the_inspector_entries_read_state_and_prepare_no_preview() {
    let reads: [(&str, &[&str]); 3] = [
        ("rim_facts", &["rim_summary", "seam_facts"]),
        ("covers_at", &["covers"]),
        ("correction", &["correction_lookup"]),
    ];
    for module in [RIM, COVERS, CORRECTIONS] {
        canonical(&module);
        let program = lower_source(module.source).expect("the module lowers");
        let [lowered] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
        assert_eq!(lowered.name, module.name);
        assert!(lowered.capabilities.is_empty(), "{}: no capability", module.name);
        let exports: Vec<&str> = lowered.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
        assert_eq!(exports, module.exports);
        let report = check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{}: {:#?}", module.name, report.diagnostics);
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
            let (_, expected) = reads.iter().find(|(f, _)| *f == function.name).expect("an entry of the table");
            let rows: BTreeSet<Row> = expected.iter().map(|q| Row::Read(q.to_string())).collect();
            assert_eq!(summary.rows, rows, "{name}: the rows it reads");
            assert_eq!(*expected, module.rows, "{name}: the module's rows");
            let params = function.params.len();
            assert_eq!(params, usize::from(function.name != "rim_facts"), "{name}: a named cap or id, or nothing");
        }
    }

    let catalogue = [
        (library::rim_and_seam_facts(), RIM, "Rim and seam facts; N", "Jumbling poses"),
        (library::covers_at_a_named_cap(), COVERS, "Covers at a named cap", "Jumbling poses"),
        (library::correction_catalogue(), CORRECTIONS, "Correction catalogue", "Jumbling labelled action"),
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
    let read_letters = |arg: &Json| -> Vec<[i64; 2]> {
        let Some(Json::List(list)) = arg.get("letters") else { panic!("a menu word's letters") };
        list.iter()
            .map(|pair| {
                let Json::List(pair) = pair else { panic!("a letter") };
                let int = |x: &Json| match x {
                    Json::Int(n) => *n,
                    other => panic!("no integer: {other:?}"),
                };
                [int(&pair[0]), int(&pair[1])]
            })
            .collect()
    };
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
