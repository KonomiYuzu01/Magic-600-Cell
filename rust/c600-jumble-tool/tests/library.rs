//! The jumbling library entries as language code (LP3, C8; chapter 03's LB1.8 and LB3.1 catalogues): four of the
//! modules of `c600_jumble_tool::library` in 600 Cell Script, checked by LP2's checker and run by LP2's interpreter on the IR
//! draft's menu words (LP2's pull request 210), with the run's alphabet pinned from the fixture's menu
//! (`word::Alphabet::pinned`). The kernel halves in `tests/ct_lb1_03.rs` and `tests/lb3.rs` are the reference: each
//! value the language code returns is compared with `c600_jumble`'s answer, or with the same fact computed apart.
//!
//! No host bridge serves KN2.4's jumbling rows to programs, so the test host `Rows` answers them from `c600_jumble`'s
//! public API at one state, in the library's draft answer types (`c600_jumble_tool::library`, Drafts): `heights` and
//! `height_counts` from `observe::heights`, `replay` from `observe::prefix_observations`, `word_check` from
//! `observe::word_check` and `protection` from M8's `levels::derive` (pull request 182). It reads a menu word's letters
//! as `word::LiteralWord::letters` reads them, refusing a word of another alphabet than the state's with
//! `LB1:input.incompatible_alphabet`, writes every field of an answer, null when undefined, refuses a declared type
//! that the answer does not have, or a declared field that it does not write, with `LG2:type.mismatch`, and grants
//! snapshot reads only.
//!
//! What runs: CT-LB1-03's jumbling half in the language; CT-LB3-03's library entry, against the kernel's heights; and
//! CT-LB3-06's library entry, with a whole word's Net and Strict protection at pose level. Not run here: the entries
//! through a host bridge or the command layer, which serve no jumbling row to programs yet; the agreement of heights
//! with `trees.py`, which M5's `tests/test_jumbling_rust_candidate.py` runs on the same kernel API; and LB3.8's named
//! cap with ΔN, Δn_h, Δu, the moved sets and the prefix digests, which no entry takes yet. A pass is source and fixture
//! evidence.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{check, separation, Policy, Row, Separation, StateRead, TYPE_MISMATCH};
use c600_digest::sha256_hex;
use c600_ir::{Callee, Effect, Expr, Lit, QTy, Ty};
use c600_jumble::levels::{self, Base, Derivation, Snapshot, Verdict};
use c600_jumble::model::NP;
use c600_jumble::observe::{
    cayley_record, heights, prefix_observations, tables, word_check, Batch, Fields, PrefixRequest,
};
use c600_jumble::q5::{identity4, Q5};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_jumble_tool::library::{self, Module, HEIGHTS, MENU_WORDS, PROTECTION, WORD_CHECKS};
use c600_jumble_tool::word::{self, menu_identity, Alphabet, Letter, LiteralWord, INCOMPATIBLE_ALPHABET};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectionMode;
use c600_library::{lower_source, lower_sources};
use c600_run::interp::{self, load, Host, Loaded, NoHost, Options, Request, RowTariff, Run, Sink};
use c600_run::menu::{MenuAlphabet, MenuIdentity};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, Json};
use c600_run::value::{entry_key, entry_value, Value as RunValue};
use c600_syntax::{format, parse};

/// Run limits wide enough for every run here.
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// A client of `menu_words`: it imports the module by the SHA-256 of its source (the placeholder `LIBRARY`) and builds
/// menu words with `menu_words.power` and the IR draft's own word operations.
const CLIENT: &str = r#"edition "6cs-draft-1";
module client requires [];
import menu_words "LIBRARY";

export fn inverse_of(w: MenuWord) -> MenuWord effects [] {
    return inverse(w);
}

export fn joined(a: MenuWord, b: MenuWord) -> MenuWord effects [] {
    return concat(a, b);
}

export fn conjugate_of(s: MenuWord, w: MenuWord) -> MenuWord effects [] {
    return conjugate(s, w);
}

export fn commutator_of(a: MenuWord, b: MenuWord) -> MenuWord effects [] {
    return commutator(a, b);
}

export fn commutator_power(a: MenuWord, b: MenuWord, n: Int) -> MenuWord effects [] {
    return menu_words.power(commutator(a, b), n);
}

export fn there_and_back(w: MenuWord, n: Int) -> MenuWord effects [] {
    let there = menu_words.power(w, n);
    return concat(there, inverse(there));
}
"#;

/// A client that reads `heights` with a declared type that the row's answer does not have, and with a misspelled list
/// field, which the answer does not write.
const MISREAD: &str = r#"edition "6cs-draft-1";
module misread requires [];

export fn lattice_key() -> Nat effects [snapshot] {
    let table = read_snapshot<record { lattice_key: Nat }>("heights");
    return table.lattice_key;
}

export fn misspelled() -> List<Nat> effects [snapshot] {
    let table = read_snapshot<record { max_left_heigth: List<Nat> }>("heights");
    return table.max_left_heigth;
}
"#;

/// A client that passes a 600-cell-Full `Word` where `menu_words.power` takes a `MenuWord`.
const MIXED: &str = r#"edition "6cs-draft-1";
module mixed requires [];
import menu_words "LIBRARY";

export fn mixed(w: Word) -> MenuWord effects [] {
    return menu_words.power(w, 2);
}
"#;

fn with_library(source: &str) -> String {
    source.replace("LIBRARY", &MENU_WORDS.sha256())
}

/// The library's modules with the client and the misreading client, lowered together, checked with no
/// diagnostic and loaded.
fn loaded() -> Loaded {
    let client = with_library(CLIENT);
    let program = library::program(&[client.as_str(), MISREAD]).expect("the library and its clients lower together");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&program).expect("the program loads")
}

/// Runs `module.function` on `args` against `host`, with `alphabet` pinned for the run's menu words.
fn run(
    loaded: &Loaded,
    module: &str,
    function: &str,
    args: Vec<RunValue>,
    host: &mut dyn Host,
    alphabet: Option<&MenuAlphabet>,
) -> Result<Run, interp::Refusal> {
    loaded.run(module, function, args, host, &Options { menu: alphabet, ..Options::new(WIDE) })
}

/// The value of a run that completed.
fn completed(run: Result<Run, interp::Refusal>, what: &str) -> RunValue {
    let run = run.unwrap_or_else(|refusal| panic!("{what}: refused before it ran: {refusal:?}"));
    assert_eq!(run.summary.outcome, Outcome::Completed, "{what}: {:?}", run.summary.code);
    run.value.unwrap_or_else(|| panic!("{what}: no value"))
}

/// The code of a run that stopped with an error.
fn stopped(run: Result<Run, interp::Refusal>, what: &str) -> &'static str {
    let run = run.unwrap_or_else(|refusal| panic!("{what}: refused before it ran: {refusal:?}"));
    assert_eq!(run.summary.outcome, Outcome::Error, "{what}: {:?}", run.value);
    run.summary.code.unwrap_or_else(|| panic!("{what}: no code"))
}

/// A literal word as a menu word of `pinned`.
fn menu_word(pinned: &MenuAlphabet, word: &LiteralWord) -> RunValue {
    let letters: Vec<[u32; 2]> = word.pairs().iter().map(|&[cap, item]| [cap as u32, item as u32]).collect();
    RunValue::menu_word(pinned.shared_identity(), &letters).expect("letters below the index limit")
}

/// The loaded library and clients with the alphabet the runs pin.
struct Library {
    loaded: Loaded,
    pinned: MenuAlphabet,
}

impl Library {
    fn new(alphabet: &Alphabet) -> Library {
        Library { loaded: loaded(), pinned: alphabet.pinned().expect("the run's menu alphabet") }
    }

    fn run(
        &self,
        module: &str,
        function: &str,
        args: Vec<RunValue>,
        host: &mut dyn Host,
    ) -> Result<Run, interp::Refusal> {
        run(&self.loaded, module, function, args, host, Some(&self.pinned))
    }

    /// The menu word a client function builds with no host, so with no state to read.
    fn build(&self, function: &str, args: Vec<RunValue>) -> RunValue {
        completed(self.run("client", function, args, &mut NoHost), function)
    }

    fn word(&self, word: &LiteralWord) -> RunValue {
        menu_word(&self.pinned, word)
    }

    /// The letters of a menu word of the pinned alphabet.
    fn pairs(&self, value: &RunValue) -> Vec<[i32; 2]> {
        let RunValue::MenuWord(word) = value else { panic!("no menu word: {value:?}") };
        assert_eq!(word.identity(), self.pinned.identity(), "a word of the pinned alphabet");
        word.letters().iter().map(|&[cap, item]| [cap as i32, item as i32]).collect()
    }
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

fn only(value: &RunValue) -> &RunValue {
    let [item] = items(value) else { panic!("not one item: {value:?}") };
    item
}

fn pieces(list: &[u32]) -> RunValue {
    RunValue::list(list.iter().map(|&p| RunValue::int(p)).collect())
}

/// The test host: KN2.4's jumbling rows answered by `c600_jumble` at one state, in the library's draft answer types.
/// It grants snapshot reads only, never a preview, a file, the clock or a sink, and keeps the names of the rows read.
struct Rows {
    state: State,
    /// The witnessed snapshot of the state: the base the `protection` row derives from.
    snapshot: Option<Snapshot>,
    asked: Vec<String>,
}

impl Rows {
    fn at(snapshot: &Snapshot) -> Rows {
        Rows { state: snapshot.state().clone(), snapshot: Some(snapshot.clone()), asked: Vec::new() }
    }

    /// A state with no witnessed claim, at which the `protection` row is not granted.
    fn of(state: State) -> Rows {
        Rows { state, snapshot: None, asked: Vec::new() }
    }

    /// The literal letters of a menu word in the run's canonical bytes, refused with
    /// `LB1:input.incompatible_alphabet` when its alphabet is not the state's, as `LiteralWord::letters` refuses them.
    fn letters(&self, word: &Json) -> Result<Vec<Letter>, &'static str> {
        let named = word.get("menu").ok_or(TYPE_MISMATCH)?;
        let part = |key: &str| named.get(key).and_then(Json::as_text).map(str::to_owned).ok_or(TYPE_MISMATCH);
        let identity = MenuIdentity {
            puzzle: part("puzzle")?,
            model_id: part("model_id")?,
            menu_id: part("menu_id")?,
            menu_order: part("menu_order")?,
            contract_rev: part("contract_rev")?,
        };
        if identity.differing(&menu_identity(&ctx().identity(self.state.menu()))).is_some() {
            return Err(INCOMPATIBLE_ALPHABET);
        }
        let menu = self.state.menu().ok_or(INCOMPATIBLE_ALPHABET)?;
        let Some(Json::List(pairs)) = word.get("letters") else { return Err(TYPE_MISMATCH) };
        let mut flat = Vec::new();
        for pair in pairs {
            let Json::List(pair) = pair else { return Err(TYPE_MISMATCH) };
            for x in pair {
                let Json::Int(x) = x else { return Err(TYPE_MISMATCH) };
                flat.push(i32::try_from(*x).map_err(|_| TYPE_MISMATCH)?);
            }
        }
        word::letters(ctx(), menu, &flat).map_err(|_| TYPE_MISMATCH)
    }

    fn answer(&self, query: &str, arg: Option<&Json>) -> Result<Value, &'static str> {
        let ctx = ctx();
        let state = &self.state;
        match (query, arg) {
            ("heights", None) => Ok(heights(ctx, state).expect("heights")),
            ("height_counts", None) => Ok(field(&heights(ctx, state).expect("heights"), "height_counts").clone()),
            ("replay", Some(word)) => Ok(served_replay(observed(state, &self.letters(word)?))),
            ("word_check", Some(arg)) => {
                let letters = self.letters(arg.get("word").ok_or(TYPE_MISMATCH)?)?;
                let unit = naturals(arg.get("unit").ok_or(TYPE_MISMATCH)?)?;
                let check = word_check(ctx, state, &letters, Some(&unit), None, None).expect("a word check");
                if text(&check, "status") == "ok" {
                    Ok(check)
                } else {
                    assert_eq!(text(&check, "code"), codes::NAMED_OBJECT_INVALID);
                    Err(codes::NAMED_OBJECT_INVALID)
                }
            }
            ("protection", Some(arg)) => {
                let snapshot = self.snapshot.as_ref().ok_or(NOT_GRANTED)?;
                let letters = self.letters(arg.get("word").ok_or(TYPE_MISMATCH)?)?;
                let mode = match arg.get("mode").and_then(Json::as_text) {
                    Some("net") => ProtectionMode::Net,
                    Some("strict") => ProtectionMode::Strict,
                    _ => return Err(TYPE_MISMATCH),
                };
                let protect = naturals(arg.get("pieces").ok_or(TYPE_MISMATCH)?)?;
                let protect: Vec<u32> =
                    protect.iter().map(|&p| u32::try_from(p).map_err(|_| TYPE_MISMATCH)).collect::<Result<_, _>>()?;
                let identity = ctx.identity(state.menu());
                let derivation =
                    levels::derive(ctx, Base::Snapshot(snapshot), &identity, &twists(&letters), mode, &protect);
                match derivation.expect("no engine error") {
                    Err(refusal) => Err(refusal.code),
                    Ok(Derivation::Derived(derived)) => {
                        Ok(derived.protection().expect("a protection report").to_json())
                    }
                    Ok(Derivation::Refused(refused)) => match &refused.protection {
                        Some(protection) => Ok(protection.to_json()),
                        None => Err(refused.refusal.code),
                    },
                }
            }
            _ => Err(NOT_GRANTED),
        }
    }
}

impl Host for Rows {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Snapshot { .. } | Request::SnapshotWith { .. } => {
                Ok(RowTariff { kernel_cost: 1, result_bound: 1 << 32 })
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<RunValue, &'static str> {
        let (query, arg, ty) = match *request {
            Request::Snapshot { query, ty } => (query, None, ty),
            Request::SnapshotWith { query, arg, ty } => (query, Some(provenance::parse(arg).ok_or(TYPE_MISMATCH)?), ty),
            _ => return Err(NOT_GRANTED),
        };
        self.asked.push(query.to_owned());
        convert(&self.answer(query, arg.as_ref())?, ty)
    }

    fn sink(&mut self, _: &Sink, _: &RunValue, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

/// The integers of a list in a run's canonical bytes, each written `{"int": "n"}`.
fn naturals(list: &Json) -> Result<Vec<i64>, &'static str> {
    let Json::List(items) = list else { return Err(TYPE_MISMATCH) };
    let natural = |item: &Json| item.get("int").and_then(Json::as_text).and_then(|n| n.parse().ok());
    items.iter().map(|item| natural(item).ok_or(TYPE_MISMATCH)).collect()
}

/// The after-state facts that JI.10's prefix observations write in an admitted letter's row, with no optional field
/// requested.
const AFTER_FACTS: [&str; 4] = ["phi_before", "phi_after", "delta_phi", "max_left_height"];

/// The prefix observations as the `replay` row serves them: `prefix_observations` writes no after-state facts in the
/// row of a refused letter, and the row writes them as null, undefined, so that every row has an admitted row's fields.
fn served_replay(mut replay: Value) -> Value {
    let Value::Object(fields) = &mut replay else { panic!("prefix observations are an object") };
    let Some(Value::Array(rows)) = fields.get_mut("rows") else { panic!("prefix observations have rows") };
    for row in rows {
        let admitted = row.get("status").and_then(Value::as_str) == Some(Status::Admissible.as_str());
        let Value::Object(row) = row else { panic!("a row is an object") };
        for name in AFTER_FACTS {
            assert_eq!(row.contains_key(name), admitted, "`{name}` in a row that is admitted: {admitted}");
            row.entry(name.to_string()).or_insert(Value::Null);
        }
    }
    replay
}

/// A kernel answer as a value of the declared type, by the library's draft answer types: a record takes the declared
/// fields of a JSON object, each of which the answer writes, null when undefined, so that a declared field that it
/// does not write, such as a misspelled one, is refused and never read as undefined; a list takes an array, or holds
/// nothing for null and one value otherwise; a map from text takes an object. Any other shape is `LG2:type.mismatch`.
fn convert(value: &Value, ty: &QTy) -> Result<RunValue, &'static str> {
    Ok(match (&ty.ty, value) {
        (Ty::Bool, Value::Bool(b)) => RunValue::Bool(*b),
        (Ty::Nat, Value::Int(n)) if *n >= 0 => RunValue::int(*n),
        (Ty::Int, Value::Int(n)) => RunValue::int(*n),
        (Ty::Text, Value::String(s)) => RunValue::text(s),
        (Ty::List(item), Value::Array(values)) => {
            RunValue::list(values.iter().map(|v| convert(v, item)).collect::<Result<_, _>>()?)
        }
        (Ty::List(_), Value::Null) => RunValue::list(Vec::new()),
        (Ty::List(item), other) => RunValue::list(vec![convert(other, item)?]),
        (Ty::Map(key, item), Value::Object(entries)) if key.ty == Ty::Text => RunValue::map(
            entries.iter().map(|(k, v)| Ok((RunValue::text(k), convert(v, item)?))).collect::<Result<_, &str>>()?,
        ),
        (Ty::Record(fields), Value::Object(entries)) => {
            let mut out = Vec::with_capacity(fields.len());
            for (name, field_ty) in fields {
                out.push((name.as_str(), convert(entries.get(name).ok_or(TYPE_MISMATCH)?, field_ty)?));
            }
            RunValue::record(out)
        }
        _ => return Err(TYPE_MISMATCH),
    })
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The jumbling model, loaded once for the tests of this file.
fn ctx() -> &'static Context {
    static CTX: OnceLock<Context> = OnceLock::new();
    CTX.get_or_init(|| Context::load(&repository().join("assets")).expect("the jumbling model loads"))
}

fn read(path: &str) -> Value {
    let bytes = std::fs::read(repository().join(path)).unwrap_or_else(|error| panic!("{path}: {error}"));
    json::parse(&bytes).unwrap_or_else(|_| panic!("{path} is JSON"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or_else(|| panic!("no field `{key}`"))
}

fn int(value: &Value, key: &str) -> i64 {
    field(value, key).as_i64().unwrap_or_else(|| panic!("`{key}` is no integer"))
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    field(value, key).as_str().unwrap_or_else(|| panic!("`{key}` is no string"))
}

/// A Boolean field, or `None` for null.
fn flag(value: &Value, key: &str) -> Option<bool> {
    match field(value, key) {
        Value::Bool(b) => Some(*b),
        Value::Null => None,
        other => panic!("`{key}` is {}", other.canonical()),
    }
}

fn ints(value: &Value) -> Vec<i64> {
    value.as_array().expect("an array").iter().map(|v| v.as_i64().expect("an integer")).collect()
}

/// A field that may be undefined: no value for null, one otherwise.
fn optional(value: &Value) -> Vec<i64> {
    match value {
        Value::Null => Vec::new(),
        other => vec![other.as_i64().expect("an integer")],
    }
}

/// A W-J fixture document (`research/jumbling/fixtures/wj-{name}.json`).
fn document(name: &str) -> Value {
    read(&format!("research/jumbling/fixtures/wj-{name}.json"))
}

/// A W-J fixture: its pinned menu and its journal records.
struct Fixture {
    menu: Arc<Menu>,
    records: Vec<Value>,
}

fn fixture(name: &str) -> Fixture {
    let doc = document(name);
    let menu = Menu::from_record(ctx(), field(&doc, "menu")).expect("the fixture's pinned menu");
    let records = field(field(&doc, "journal"), "records").as_array().expect("records").to_vec();
    Fixture { menu: Arc::new(menu), records }
}

impl Fixture {
    fn alphabet(&self) -> Alphabet {
        Alphabet::of(ctx(), &self.menu).expect("an inverse-closed menu")
    }

    fn solved(&self) -> Snapshot {
        Snapshot::solved(ctx(), Some(self.menu.clone()))
    }

    /// The state after the first `count` journal records from solved, witnessed by them.
    fn after(&self, count: usize) -> Snapshot {
        let journal: Vec<Twist> = self.records[..count]
            .iter()
            .map(|record| Twist::from_record(ctx(), record).expect("a journal record"))
            .collect();
        witnessed(&self.menu, &journal)
    }

    /// The literal letters of `pairs` in the fixture's menu.
    fn letters(&self, pairs: &[[i32; 2]]) -> Vec<Letter> {
        word::letters(ctx(), &self.menu, &word::word_of(pairs)).expect("letters of the menu")
    }
}

/// `journal` played from solved under `menu`, every letter admissible, with the claim its replay witnesses.
fn witnessed(menu: &Arc<Menu>, journal: &[Twist]) -> Snapshot {
    let mut state = State::new(Some(menu.clone()));
    for (index, twist) in journal.iter().enumerate() {
        let status = state.apply(ctx(), twist, None, &[]).expect("no engine error").status;
        assert_eq!(status, Status::Admissible, "letter {index}");
    }
    Snapshot::witnessed(ctx(), state, journal, None).expect("no engine error").expect("the journal witnesses the state")
}

fn twists(letters: &[Letter]) -> Vec<Twist> {
    letters.iter().map(|letter| letter.1.clone()).collect()
}

/// The pinned I_a menu with its items in reverse order: the same rotations, another pinned order.
fn reversed_i_a() -> Arc<Menu> {
    let mut record = field(&document("I_a"), "menu").clone();
    let Value::Object(fields) = &mut record else { panic!("a menu record is a mapping") };
    let Some(Value::Array(base)) = fields.get_mut("base") else { panic!("a menu record has a base") };
    base.reverse();
    Arc::new(Menu::from_record(ctx(), &record).expect("a pinned menu"))
}

/// The first `count` letters of H6030, a word in the pinned I_a menu from solved.
fn h6030(count: usize) -> Vec<[i32; 2]> {
    let doc = read("research/jumbling/restoration/10-x1-unbounded/witnesses/H6030.json");
    let word = field(&doc, "word").as_array().expect("a word");
    word[..count]
        .iter()
        .map(|pair| {
            let pair = pair.as_array().expect("a letter");
            [pair[0].as_i64().unwrap() as i32, pair[1].as_i64().unwrap() as i32]
        })
        .collect()
}

/// The first item of the menu that is not the identity rotation.
fn rotation(menu: &Menu) -> i32 {
    menu.items.iter().position(|(_, m)| *m != identity4()).expect("a rotation") as i32
}

/// JI.10's prefix observations of a literal word at `start`, which is not changed.
fn observed(start: &State, letters: &[Letter]) -> Value {
    let request = PrefixRequest {
        letters,
        fields: Fields { n: false, heights: false, fd: false, sets: false },
        cap: None,
        batch: Batch { chunk: 16, budget: None, cancel_after: None },
        keep: false,
        eval_budget: None,
    };
    prefix_observations(ctx(), start, &request).expect("prefix observations").0
}

/// The kernel's whole-word check (`check_named_word`) of a literal word at `start`.
fn checked(start: &State, letters: &[Letter], unit: Option<&[u32]>) -> Value {
    let unit: Option<Vec<i64>> = unit.map(|pieces| pieces.iter().map(|&p| i64::from(p)).collect());
    word_check(ctx(), start, letters, unit.as_deref(), None, None).expect("a word check")
}

/// M8's derivation of `letters` from `base` in `mode` with `protect` protected, under the base's own identity.
fn derive(base: &Snapshot, letters: &[Letter], mode: ProtectionMode, protect: &[u32]) -> Derivation {
    let ctx = ctx();
    let identity = ctx.identity(base.state().menu());
    levels::derive(ctx, Base::Snapshot(base), &identity, &twists(letters), mode, protect)
        .expect("no engine error")
        .expect("the base's own identity")
}

/// Applies one literal letter with no protection and returns its status.
fn play(state: &mut State, letter: &Letter) -> Status {
    state.apply(ctx(), &letter.1, None, &[]).expect("no engine error").status
}

fn heights_of(row: &Value) -> [i64; 2] {
    let pair = field(row, "heights").as_array().expect("heights");
    [pair[0].as_i64().unwrap(), pair[1].as_i64().unwrap()]
}

/// Φ = Σ max(0, h − 2) over the left heights and the largest left height, summed from JI.6's per-pose table.
fn phi_and_peak(state: &State) -> (i64, i64) {
    let facts = heights(ctx(), state).expect("heights");
    let (mut phi, mut peak) = (0, 0);
    for row in field(&facts, "poses").as_object().expect("poses").values() {
        let left = heights_of(row)[0];
        phi += int(row, "count") * (left - 2).max(0);
        peak = peak.max(left);
    }
    (phi, peak)
}

/// LB3.7's chronological observations of an admissible literal word, recomputed apart from the kernel's batch and from
/// the language code: each letter is applied by itself and Φ and the peak are summed from the per-pose height table of
/// every prefix state. Returns the peak (start included), the excursion, tameness and the end state.
fn recomputed(start: &State, letters: &[Letter]) -> (i64, i64, bool, State) {
    let mut work = start.clone();
    let (start_phi, mut peak) = phi_and_peak(&work);
    let (mut top, mut previous, mut tame) = (start_phi, start_phi, true);
    for (index, letter) in letters.iter().enumerate() {
        assert_eq!(play(&mut work, letter), Status::Admissible, "letter {index}");
        let (phi, max) = phi_and_peak(&work);
        tame &= phi <= previous;
        previous = phi;
        top = top.max(phi);
        peak = peak.max(max);
    }
    (peak, top - start_phi, tame, work)
}

/// The three terms of LB3.8's step flag besides admissibility, from the start and end states: every piece of the unit
/// is on the lattice, a lattice piece is lifted, and every other off piece lands or keeps its exact pose.
fn step_terms(start: &State, end: &State, unit: &[u32]) -> (bool, bool, bool) {
    let unit: HashSet<usize> = unit.iter().map(|&p| p as usize).collect();
    let lands = unit.iter().all(|&p| end.is_lattice_piece(p));
    let lifted = (0..NP).any(|p| start.is_lattice_piece(p) && !end.is_lattice_piece(p));
    let others_keep = (0..NP)
        .filter(|p| !start.is_lattice_piece(*p) && !unit.contains(p))
        .all(|p| end.is_lattice_piece(p) || end.pose_of(p) == start.pose_of(p));
    (lands, lifted, others_keep)
}

/// LB3.8's step flag from the start and end states: the word is admissible, the unit lands, no lattice piece is
/// lifted, and every other off piece lands or keeps its exact pose.
fn step_flag(start: &State, end: &State, unit: &[u32], admissible: bool) -> bool {
    let (lands, lifted, others_keep) = step_terms(start, end, unit);
    admissible && lands && !lifted && others_keep
}

/// The same three terms as the kernel's whole-word check reports them, the facts `jumble_word_checks.step_flag` reads.
fn kernel_terms(check: &Value) -> (bool, bool, bool) {
    let others = int(check, "others_exact") + int(check, "others_landed") == int(check, "others");
    (int(check, "unit_on_lattice") == int(check, "unit_size"), int(check, "lattice_pieces_lifted_off") > 0, others)
}

/// Every node of an IR expression, in path order.
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

/// CT-LB1-03 (part: the jumbling half in the language, "inverse, conjugate, commutator and powers construct words
/// without state access": the library's source). `menu_words` parses with no diagnostic, is in the formatter's
/// canonical layout with LF line endings, and lowers to one module that requests no capability and exports `power`
/// only. The checker reports no diagnostic, and `power`'s summary has no effect, capability, effect variable, state
/// read, grant-table row or unresolved call: under capability separation it neither reads state nor prepares previews,
/// and its body names no proposal target. It uses the IR draft's `concat` and `inverse`, the empty menu word and exact
/// integer operations, and calls only itself, as the catalogue row's Backing says.
#[test]
fn ct_lb1_03_menu_words_source_checks_and_reads_no_state() {
    canonical(&MENU_WORDS);
    let program = lower_source(MENU_WORDS.source).expect("the module lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, MENU_WORDS.name);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, MENU_WORDS.exports);
    assert!(MENU_WORDS.rows.is_empty());

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let summary = report.summary(MENU_WORDS.name, "power").expect("a summary of power");
    assert!(summary.is_pure(), "{summary:?}");
    assert!(summary.effects.is_empty() && summary.capabilities.is_empty() && summary.vars.is_empty());
    assert!(summary.state_reads.is_empty() && summary.rows.is_empty() && summary.unresolved.is_empty());
    assert_eq!(separation(summary, &Policy::interim()), Separation { reads_state: false, prepares_previews: false });

    let power = module.functions.iter().find(|f| f.name == "power").expect("the module defines power");
    let mut all = Vec::new();
    nodes(&power.body, &mut all);
    assert!(!all.iter().any(|e| matches!(e, Expr::Propose { .. })), "power names no proposal target");
    assert!(
        all.iter().any(|e| matches!(e, Expr::Lit(Lit::MenuWord(letters)) if letters.is_empty())),
        "power 0 is the empty menu word"
    );
    let ops: BTreeSet<String> = all
        .iter()
        .filter_map(|e| match e {
            Expr::Prim { op, .. } => Some(format!("{op:?}")),
            _ => None,
        })
        .collect();
    assert_eq!(ops, BTreeSet::from(["Concat", "ExactAdd", "ExactEq", "ExactLess", "Inverse"].map(String::from)));
    let calls: BTreeSet<(String, String)> = all
        .iter()
        .filter_map(|e| match e {
            Expr::Call { callee: Callee::Fn { module, name }, .. } => {
                Some((module.clone().unwrap_or_else(|| MENU_WORDS.name.into()), name.clone()))
            }
            Expr::Call { .. } => panic!("a call of a function value"),
            _ => None,
        })
        .collect();
    assert_eq!(calls, BTreeSet::from([(MENU_WORDS.name.to_string(), "power".to_string())]));

    // The catalogue row: LB1.2's five fields, with the checker's effects and the source the module was built from.
    let row = library::literal_words();
    assert_eq!(row.entry, "Literal words");
    assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), summary.effects, "Effect: pure");
    assert!(row.level.starts_with("Jumbling menu letters"));
    assert!(row.backing.contains("`menu_words.power`") && row.notes.contains("checks no state"));
    assert!(row.notes.contains(INCOMPATIBLE_ALPHABET));
    assert_eq!((row.module, row.functions), (MENU_WORDS.name, MENU_WORDS.exports));
    assert_eq!(row.source_sha256, sha256_hex(MENU_WORDS.source.as_bytes()));
}

/// CT-LB3-03 (part: the library entry's source, `jumble_heights`) and CT-LB3-06 (part: the library entries' sources,
/// `jumble_word_checks` and `jumble_protection`). The three modules parse with no diagnostic, keep the canonical
/// layout and LF line endings, request no capability and check with no diagnostic. Each exported entry has the one
/// effect `snapshot` and reads only its KN2.4 rows: under capability separation it reads state and prepares no
/// preview. No function names a proposal target, so none plays or outputs a word, and the helpers are pure. The
/// catalogue rows name the entries, the checker's effect and the digest of each module's source.
#[test]
fn ct_lb3_03_and_ct_lb3_06_entries_read_only_their_rows() {
    let reads: [(&str, &[&str]); 4] = [
        ("height_facts", &["height_counts", "heights"]),
        ("word_observations", &["replay"]),
        ("step_flag", &["word_check"]),
        ("word_protection", &["protection"]),
    ];
    for module in [HEIGHTS, WORD_CHECKS, PROTECTION] {
        canonical(&module);
        let program = lower_source(module.source).expect("the module lowers");
        let [lowered] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
        assert_eq!(lowered.name, module.name);
        assert!(lowered.capabilities.is_empty(), "{}: no capability", module.name);
        let exports: Vec<&str> = lowered.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
        assert_eq!(exports, module.exports);
        let report = check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{}: {:#?}", module.name, report.diagnostics);
        let mut rows = BTreeSet::new();
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
            let expected_rows: BTreeSet<Row> = expected.iter().map(|q| Row::Read(q.to_string())).collect();
            assert_eq!(summary.rows, expected_rows, "{name}: the rows it reads");
            rows.extend(expected.iter().copied());
        }
        assert_eq!(rows.into_iter().collect::<Vec<_>>(), module.rows, "{}: the module's rows", module.name);
    }

    let catalogue = [
        (library::heights_branches_domains_and_phi(), HEIGHTS, "Heights, branches, domains and Φ"),
        (library::release_and_whole_word_checks(), WORD_CHECKS, "Release and whole-word checks"),
        (library::protection(), PROTECTION, "Protection"),
    ];
    for (row, module, entry) in catalogue {
        assert_eq!(row.entry, entry);
        assert!(row.level.starts_with("Jumbling poses"), "{entry}");
        assert_eq!(row.effects, &[Effect::ReadSnapshot][..], "{entry}: Effect");
        assert_eq!((row.module, row.functions), (module.name, module.exports), "{entry}");
        assert_eq!(row.source_sha256, sha256_hex(module.source.as_bytes()), "{entry}");
        for function in module.exports {
            assert!(row.backing.contains(&format!("`{}.{function}`", module.name)), "{entry}: Backing");
        }
    }
    assert!(library::protection().backing.contains("levels::derive"));
}

/// CT-LB1-03 (part: the jumbling half in the language, "inverse, conjugate, commutator and powers construct words
/// without state access": construction). Every word here is built by a run with no host, so no state exists for the
/// run to read, over the pinned I_a alphabet. `menu_words.power` and a client that imports it by digest build exactly
/// the words that `word::LiteralWord` builds from the same letters: the inverse reverses the letters and gives each
/// its inverse item, the conjugate by s is s w s⁻¹, the commutator of a and b is a b a⁻¹ b⁻¹, the power n repeats the
/// word, 0 gives the empty word and a negative power repeats the inverse. Nothing cancels, and every word built is a
/// word of the pinned alphabet. The client's functions have no effect and read no state.
#[test]
fn ct_lb1_03_menu_words_are_built_letter_by_letter_with_no_host() {
    let i_a = fixture("I_a");
    let alphabet = i_a.alphabet();
    let library = Library::new(&alphabet);
    let a = rotation(&i_a.menu);
    let w = alphabet.word(&[[0, a], [57, a + 1]]).expect("two letters");
    let u = alphabet.word(&[[3, a]]).expect("one letter");

    for n in [-2, -1, 0, 1, 3] {
        let args = vec![library.word(&w), RunValue::int(n)];
        let built = completed(library.run(MENU_WORDS.name, "power", args, &mut NoHost), "power");
        assert_eq!(library.pairs(&built), w.power(n as i32).pairs(), "power {n}");
    }
    let empty =
        completed(library.run(MENU_WORDS.name, "power", vec![library.word(&u), RunValue::int(0)], &mut NoHost), "");
    assert!(library.pairs(&empty).is_empty(), "the empty menu word, of the pinned alphabet");

    assert_eq!(library.pairs(&library.build("inverse_of", vec![library.word(&w)])), w.inverse().pairs());
    let joined = library.build("joined", vec![library.word(&w), library.word(&w.inverse())]);
    assert_eq!(library.pairs(&joined), w.concat(&w.inverse()).expect("one alphabet").pairs());
    assert_eq!(library.pairs(&joined).len(), 4, "construction cancels nothing");
    let conjugate = library.build("conjugate_of", vec![library.word(&u), library.word(&w)]);
    assert_eq!(library.pairs(&conjugate), w.conjugate(&u).expect("one alphabet").pairs());
    let commutator = u.commutator(&w).expect("one alphabet");
    assert_eq!(
        library.pairs(&library.build("commutator_of", vec![library.word(&u), library.word(&w)])),
        commutator.pairs()
    );
    for n in [2, -1] {
        let built = library.build("commutator_power", vec![library.word(&u), library.word(&w), RunValue::int(n)]);
        assert_eq!(library.pairs(&built), commutator.power(n as i32).pairs(), "commutator power {n}");
    }
    let there = w.power(2);
    let built = library.build("there_and_back", vec![library.word(&w), RunValue::int(2)]);
    assert_eq!(library.pairs(&built), there.concat(&there.inverse()).expect("one alphabet").pairs());

    let client = with_library(CLIENT);
    let program = lower_sources(&[MENU_WORDS.source, client.as_str()]).expect("the module and the client lower");
    let report = check(&program).expect("well-formed IR");
    for function in ["inverse_of", "joined", "conjugate_of", "commutator_of", "commutator_power", "there_and_back"] {
        let summary = report.summary("client", function).expect("a summary");
        assert!(summary.is_pure() && summary.state_reads.is_empty(), "client.{function}: {summary:?}");
    }
}

/// CT-LB1-03 (part: the jumbling half in the language, alphabets: LB1.8's `LB1:input.incompatible_alphabet`). A run
/// pins one alphabet, and a menu word of another is refused before the run starts: a word of the S4 menu, a word of
/// I_a's rotations in another pinned order (another menu-order digest), and any menu word in a run with no alphabet
/// pinned. The same menu loaded twice is one alphabet. The checker refuses a 600-cell-Full `Word` where `power` takes
/// a `MenuWord` with the same code. Playing a word reads the state's alphabet: the entries' rows refuse an I_a word
/// with that code at a state of the S4 menu, of I_a in another order and in free mode, as `LiteralWord::letters`
/// refuses it in the kernel half.
#[test]
fn ct_lb1_03_menu_words_of_two_alphabets_do_not_mix() {
    let ctx = ctx();
    let i_a = fixture("I_a");
    let alphabet = i_a.alphabet();
    let library = Library::new(&alphabet);
    let a = rotation(&i_a.menu);
    let w = alphabet.word(&[[0, a], [57, a + 1]]).expect("two letters");

    let s4 = fixture("S4");
    let reversed = reversed_i_a();
    let turned = Alphabet::of(ctx, &reversed).expect("an inverse-closed menu");
    for (what, other) in [("the S4 menu", s4.alphabet()), ("I_a in another order", turned)] {
        let pinned = other.pinned().expect("its menu alphabet");
        let foreign = menu_word(&pinned, &other.word(&[[0, 0]]).expect("one letter"));
        let args = vec![foreign, RunValue::int(2)];
        let refusal = library.run(MENU_WORDS.name, "power", args, &mut NoHost).expect_err(what);
        assert_eq!(refusal.code, INCOMPATIBLE_ALPHABET, "{what}");
    }
    let args = vec![library.word(&w), RunValue::int(1)];
    let refusal = run(&library.loaded, MENU_WORDS.name, "power", args, &mut NoHost, None).expect_err("none pinned");
    assert_eq!(refusal.code, INCOMPATIBLE_ALPHABET, "a menu word in a run with no alphabet pinned");
    let again = fixture("I_a").alphabet().pinned().expect("the same menu, loaded again");
    let args = vec![library.word(&w), RunValue::int(1)];
    let built = completed(run(&library.loaded, MENU_WORDS.name, "power", args, &mut NoHost, Some(&again)), "again");
    assert_eq!(library.pairs(&built), w.pairs(), "one menu, loaded twice, is one alphabet");

    let mixed = with_library(MIXED);
    let program = lower_sources(&[MENU_WORDS.source, mixed.as_str()]).expect("the module and the client lower");
    let found = check(&program).expect("well-formed IR").codes();
    assert!(!found.is_empty() && found.iter().all(|&code| code == INCOMPATIBLE_ALPHABET), "{found:?}");

    let states = [
        ("an S4 state", State::new(Some(s4.menu.clone()))),
        ("I_a in another order", State::new(Some(reversed.clone()))),
        ("free mode", State::new(None)),
    ];
    for (what, state) in states {
        let error = w.letters(ctx, &state).expect_err(what);
        assert_eq!(error.code(), Some(INCOMPATIBLE_ALPHABET), "{what}: the kernel half");
        let mut host = Rows::of(state);
        let observations = library.run(WORD_CHECKS.name, "word_observations", vec![library.word(&w)], &mut host);
        assert_eq!(stopped(observations, what), INCOMPATIBLE_ALPHABET, "{what}");
        let flagged = library.run(WORD_CHECKS.name, "step_flag", vec![library.word(&w), pieces(&[0])], &mut host);
        assert_eq!(stopped(flagged, what), INCOMPATIBLE_ALPHABET, "{what}");
        assert_eq!(host.asked, ["replay", "word_check"], "{what}");
    }
}

/// CT-LB1-03 (part: the jumbling half in the language, "application checks each prefix" and "an inverse returns an
/// admitted path to its start"). Words built by the language are played by the entries' rows. From solved, the word w
/// of H6030's first 20 letters followed by its inverse, built by `there_and_back`, is admitted prefix by prefix:
/// `word_observations` reports all 40 prefixes played and the start digest as the end, as the kernel's prefix
/// observations do. At the state after 12 records of the I_a journal, the conjugate of an admitted letter by a blocked
/// one is refused at its first prefix: no prefix is played and no claim is made, as in the kernel, and
/// `word_protection` stops with `KN2:input.word_blocked_prefix`, as `levels::derive` refuses the word with no
/// protection report. The admitted letter, its inverse and then the blocked letter are refused after two prefixes
/// played, as in the kernel; the zeroth power, the empty word, is admitted and ends at the start.
#[test]
fn ct_lb1_03_application_checks_each_prefix_of_a_built_word() {
    let ctx = ctx();
    let i_a = fixture("I_a");
    let alphabet = i_a.alphabet();
    let library = Library::new(&alphabet);

    let w = alphabet.word(&h6030(20)).expect("letters of the I_a menu");
    let built = library.build("there_and_back", vec![library.word(&w), RunValue::int(1)]);
    assert_eq!(library.pairs(&built), w.concat(&w.inverse()).expect("one alphabet").pairs());
    let solved = i_a.solved();
    let start = solved.state().digest(ctx);
    let mut host = Rows::at(&solved);
    let seen = completed(library.run(WORD_CHECKS.name, "word_observations", vec![built.clone()], &mut host), "w w⁻¹");
    assert_eq!(host.asked, ["replay"]);
    assert_eq!((truth(get(&seen, "complete")), number(get(&seen, "played"))), (true, 40));
    assert_eq!(string(get(only(get(&seen, "claims")), "end_digest")), start, "the inverse returns to the start");
    let kernel = observed(solved.state(), &i_a.letters(&library.pairs(&built)));
    assert_eq!(field(&kernel, "rows").as_array().expect("rows").len(), 40);
    assert_eq!(text(field(&kernel, "final"), "end_digest"), start);

    let snapshot = i_a.after(12);
    let state = snapshot.state();
    let grips: Vec<usize> = (0..12).collect();
    let statuses = state.clone().survey(ctx, &grips, None).expect("a survey");
    let blocked = grips.iter().copied().find(|&g| statuses[g] == Status::Blocked).expect("a blocked cap") as i32;
    let open = grips.iter().copied().find(|&g| statuses[g] == Status::Admissible).expect("an open cap") as i32;
    let a = rotation(&i_a.menu);
    let admitted = alphabet.word(&[[open, a]]).expect("one letter");
    let setup = alphabet.word(&[[blocked, a]]).expect("one letter");
    let conjugate = library.build("conjugate_of", vec![library.word(&setup), library.word(&admitted)]);
    let literal = admitted.conjugate(&setup).expect("one alphabet");
    assert_eq!(library.pairs(&conjugate), literal.pairs());
    let mut host = Rows::at(&snapshot);
    let observations = library.run(WORD_CHECKS.name, "word_observations", vec![conjugate.clone()], &mut host);
    let seen = completed(observations, "the conjugate");
    assert_eq!((truth(get(&seen, "complete")), number(get(&seen, "played"))), (false, 0));
    assert!(items(get(&seen, "claims")).is_empty(), "no claim for a refused prefix");
    let kernel = observed(state, &i_a.letters(literal.pairs()));
    assert_eq!((flag(&kernel, "complete"), int(&kernel, "first_refused")), (Some(false), 0));
    let protected = library.run(PROTECTION.name, "word_protection", vec![conjugate, pieces(&[])], &mut host);
    assert_eq!(stopped(protected, "protection"), codes::WORD_BLOCKED_PREFIX);
    let Derivation::Refused(refused) = derive(&snapshot, &i_a.letters(literal.pairs()), ProtectionMode::Net, &[])
    else {
        panic!("a word blocked at its first prefix is refused")
    };
    assert_eq!((refused.refusal.code, refused.first_refused_prefix), (codes::WORD_BLOCKED_PREFIX, Some(1)));
    assert!(refused.protection.is_none(), "a blocked prefix carries no protection report");

    let back = library.build("there_and_back", vec![library.word(&admitted), RunValue::int(1)]);
    let late = library.build("joined", vec![back, library.word(&setup)]);
    let back_and_setup = admitted.concat(&admitted.inverse()).and_then(|w| w.concat(&setup)).expect("one alphabet");
    assert_eq!(library.pairs(&late), back_and_setup.pairs());
    let seen = completed(library.run(WORD_CHECKS.name, "word_observations", vec![late], &mut host), "late");
    assert_eq!((truth(get(&seen, "complete")), number(get(&seen, "played"))), (false, 2));
    assert!(items(get(&seen, "claims")).is_empty());
    let kernel = observed(state, &i_a.letters(back_and_setup.pairs()));
    assert_eq!((flag(&kernel, "complete"), int(&kernel, "first_refused")), (Some(false), 2), "the kernel's");

    let args = vec![library.word(&setup), RunValue::int(0)];
    let empty = completed(library.run(MENU_WORDS.name, "power", args, &mut NoHost), "the zeroth power");
    assert!(library.pairs(&empty).is_empty());
    let seen = completed(library.run(WORD_CHECKS.name, "word_observations", vec![empty], &mut host), "empty");
    assert_eq!((truth(get(&seen, "complete")), number(get(&seen, "played"))), (true, 0));
    let claim = only(get(&seen, "claims"));
    assert_eq!(string(get(claim, "end_digest")), state.digest(ctx), "the empty word ends at the start");
    assert_eq!((number(get(claim, "excursion")), truth(get(claim, "tame"))), (0, true));
    assert_eq!(number(get(claim, "max_left_height")), phi_and_peak(state).1);
    assert_eq!(host.asked, ["replay", "protection", "replay", "replay"]);
}

/// `jumble_heights.height_facts` at `state`, with the rows it read. The run pins no alphabet.
fn height_facts(loaded: &Loaded, state: &State) -> (RunValue, Vec<String>) {
    let mut host = Rows::of(state.clone());
    let facts = completed(run(loaded, HEIGHTS.name, "height_facts", Vec::new(), &mut host, None), "height_facts");
    (facts, host.asked)
}

/// The entry's per-pose rows, with their pose keys.
fn pose_rows(facts: &RunValue) -> Vec<(&str, &RunValue)> {
    let RunValue::Map(entries) = get(facts, "poses") else { panic!("the poses are no map") };
    entries.items().iter().map(|entry| (string(entry_key(entry)), entry_value(entry))).collect()
}

/// CT-LB3-03 (part: the library entry, "heights and domain IDs agree" with the kernel's; not run: the agreement with
/// LB1 and `trees.py` on ENG-0J item 8's states and H6030, which M5's `tests/test_jumbling_rust_candidate.py` runs on
/// the same kernel API). `jumble_heights.height_facts` reads `heights` and `height_counts` once each. After the first
/// record and after 12 records of the I_a and S4 journals, at solved and after H6030's first 35 letters, its per-pose
/// rows are the kernel's (domain ID, heights, branches and count of each occupied pose), and so are its counts and
/// lattice key. Its Φ and largest left height equal the kernel's and the sums computed apart from both. The pieces it
/// counts on the lattice domain equal the model's pieces less the kernel's off-lattice count, `NP − off`, a field apart
/// from the per-pose rows: that is the independent cross-check. The pieces it counts at heights (0,0) are compared only
/// with a sum over the kernel's per-pose rows that follows the entry's own formula, as the lattice count also is. On
/// these menu-reached states the two counts are equal (LB3.1's lattice test by heights on Γ(Λ)). A declared type that
/// the row's answer does not have stops a run with `LG2:type.mismatch`, and so does a misspelled list field, which the
/// answer does not write: it is never read as an undefined, empty list.
#[test]
fn ct_lb3_03_heights_and_domain_ids_agree_with_the_kernel() {
    let ctx = ctx();
    let loaded = loaded();
    let i_a = fixture("I_a");
    let raised = witnessed(&i_a.menu, &twists(&i_a.letters(&h6030(35))));
    let mut states = vec![("I_a solved".to_string(), i_a.solved()), ("H6030[..35]".to_string(), raised)];
    for (name, count) in [("I_a", 1), ("I_a", 12), ("S4", 1), ("S4", 12)] {
        states.push((format!("{name} after {count}"), fixture(name).after(count)));
    }
    for (name, snapshot) in &states {
        let state = snapshot.state();
        let (facts, asked) = height_facts(&loaded, state);
        assert_eq!(asked, ["heights", "height_counts"], "{name}");
        let kernel = heights(ctx, state).expect("heights");
        let lattice = text(&kernel, "lattice_key");
        assert_eq!(string(get(&facts, "lattice_key")), lattice, "{name}");
        let poses = field(&kernel, "poses").as_object().expect("poses");
        let rows = pose_rows(&facts);
        assert_eq!(rows.len(), poses.len(), "{name}: one row per occupied pose");
        let (mut at_zero, mut on_lattice) = (0, 0);
        for (key, row) in rows {
            let expected = poses.get(key).unwrap_or_else(|| panic!("{name}: pose {key} is the kernel's"));
            assert_eq!(string(get(row, "domain")), text(expected, "domain"), "{name}: pose {key}");
            assert_eq!(numbers(get(row, "heights")), ints(field(expected, "heights")), "{name}: pose {key}");
            assert_eq!(numbers(get(row, "branches")), ints(field(expected, "branches")), "{name}: pose {key}");
            assert_eq!(number(get(row, "count")), int(expected, "count"), "{name}: pose {key}");
            at_zero += if heights_of(expected) == [0, 0] { int(expected, "count") } else { 0 };
            on_lattice += if text(expected, "domain") == lattice { int(expected, "count") } else { 0 };
        }
        let phi = (number(get(&facts, "phi")), number(get(&facts, "max_left_height")));
        assert_eq!(phi, (int(&kernel, "phi"), int(&kernel, "max_left_height")), "{name}: the kernel's Φ and peak");
        assert_eq!(phi, phi_and_peak(state), "{name}: Φ and the peak summed apart");
        assert_eq!(number(get(&facts, "at_zero")), at_zero, "{name}");
        assert_eq!(number(get(&facts, "on_lattice")), on_lattice, "{name}");
        assert_eq!(on_lattice, NP as i64 - int(&kernel, "off"), "{name}");
        assert_eq!(at_zero, on_lattice, "{name}: heights (0,0) exactly on the lattice, on Γ(Λ)");
        let (counts, expected) = (get(&facts, "counts"), field(&kernel, "height_counts"));
        for bin in ["n_0", "n_2", "n_4", "n_6", "n_above_6"] {
            assert_eq!(number(get(counts, bin)), int(expected, bin), "{name}: {bin}");
        }
        assert_eq!(numbers(get(counts, "max_above_6")), optional(field(expected, "max_above_6")), "{name}");
    }

    let mut host = Rows::of(i_a.solved().state().clone());
    let misread = run(&loaded, "misread", "lattice_key", Vec::new(), &mut host, None);
    assert_eq!(stopped(misread, "a misread type"), TYPE_MISMATCH);
    let misspelled = run(&loaded, "misread", "misspelled", Vec::new(), &mut host, None);
    assert_eq!(stopped(misspelled, "a misspelled list field"), TYPE_MISMATCH);
    assert_eq!(host.asked, ["heights", "heights"]);
}

/// CT-LB3-03 (part: the library entry, "Φ = 0 with height-2 pieces does not pass the sticker test", with M8's
/// `levels::sticker_test`). After the first record of the I_a journal and after 12 records of the I_a and S4 journals,
/// `height_facts` gives Φ = 0 and a largest left height of 2, with pieces counted at left height 2 and fewer pieces on
/// the lattice than the model has, and the sticker test answers no. At solved it gives Φ = 0 with every piece at
/// heights (0,0) and on the lattice, and the sticker test answers yes.
#[test]
fn ct_lb3_03_phi_zero_with_height_two_pieces_does_not_pass_the_sticker_test() {
    let ctx = ctx();
    let loaded = loaded();
    for (name, count) in [("I_a", 1), ("I_a", 12), ("S4", 12)] {
        let snapshot = fixture(name).after(count);
        let state = snapshot.state();
        let (facts, _) = height_facts(&loaded, state);
        let phi = (number(get(&facts, "phi")), number(get(&facts, "max_left_height")));
        assert_eq!(phi, (0, 2), "{name} after {count}: Φ and the largest left height");
        assert!(number(get(get(&facts, "counts"), "n_2")) > 0, "{name} after {count}: pieces at height 2");
        assert!(number(get(&facts, "on_lattice")) < NP as i64, "{name} after {count}: pieces off the lattice");
        let verdict = levels::sticker_test(ctx, state).expect("no engine error");
        assert!(matches!(verdict, Verdict::No(_)), "{name} after {count}: {verdict:?}");
    }
    let snapshot = fixture("I_a").solved();
    let (facts, _) = height_facts(&loaded, snapshot.state());
    assert_eq!((number(get(&facts, "phi")), number(get(&facts, "max_left_height"))), (0, 0));
    let counted = [number(get(&facts, "at_zero")), number(get(&facts, "on_lattice"))];
    assert_eq!(counted, [NP as i64; 2]);
    assert!(matches!(levels::sticker_test(ctx, snapshot.state()).expect("no engine error"), Verdict::Yes(_)));
}

/// CT-LB3-03 (part: the library entry, "in free mode, heights (0,0) do not imply the lattice, and domain IDs come
/// from exact coset tests"). In free mode, after the exact Cayley rotation ω = (1/2, 0, 0) at cap 0, which lies outside
/// K⁺, `height_facts` counts every piece at heights (0,0) but only the unmoved pieces on the lattice: the moved pieces
/// form the one pose class off the lattice domain, and its domain ID is the kernel's exact coset ID of the rotation.
/// The run pins no alphabet, and the entry reads no word.
#[test]
fn ct_lb3_03_heights_zero_do_not_imply_the_lattice_in_free_mode() {
    let ctx = ctx();
    let t = tables(ctx).expect("the observation tables");
    let omega = [Q5::of(1, 0, 2), Q5::zero(), Q5::zero()];
    let twist = Twist::from_record(ctx, &cayley_record(ctx, 0, &omega)).expect("an exact rotation fixing the pole");
    assert_eq!(twist.kidx, None, "the rotation lies outside K⁺");
    let mut free = State::new(None);
    let outcome = free.apply(ctx, &twist, None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    let moved = outcome.moved() as i64;
    assert!(moved > 0);

    let (facts, _) = height_facts(&loaded(), &free);
    assert_eq!(number(get(&facts, "at_zero")), NP as i64, "every piece reads heights (0,0)");
    assert_eq!(number(get(&facts, "on_lattice")), NP as i64 - moved, "the moved pieces are off the lattice");
    assert_eq!(string(get(&facts, "lattice_key")), t.lattice_key());
    let rows = pose_rows(&facts);
    assert!(rows.iter().all(|(_, row)| numbers(get(row, "heights")) == [0, 0]));
    let off: Vec<&RunValue> =
        rows.iter().map(|(_, row)| *row).filter(|row| string(get(row, "domain")) != t.lattice_key()).collect();
    assert_eq!(off.len(), 1, "one pose class off the lattice");
    assert_eq!(number(get(off[0], "count")), moved);
    assert_eq!(string(get(off[0], "domain")), t.domain_id(&twist.matrix), "the rotation's exact coset");
}

/// CT-LB3-06 (part: the library entry, "on exact word fixtures, prefix peak, excursion and tameness are recomputed
/// separately"). `jumble_word_checks.word_observations` reads `replay` and recomputes its claim from each admitted
/// prefix's Φ and largest left height, without the kernel's final claims. On three literal words in the pinned I_a
/// menu (H6030's first 35 letters from solved, which raise Φ; from their end, the last letter's inverse and the letter
/// again, built by `there_and_back`, with zero excursion and not tame, which ends at its start; and the inverse alone,
/// built by `inverse_of`, which is tame), its claim of peak, excursion, tameness and end digest equals the one computed
/// apart by playing each letter and summing the kernel's per-pose heights, the kernel's prefix observations' final
/// claims and its whole-word check's.
#[test]
fn ct_lb3_06_peak_excursion_and_tameness_are_recomputed_by_the_entry() {
    let ctx = ctx();
    let i_a = fixture("I_a");
    let alphabet = i_a.alphabet();
    let library = Library::new(&alphabet);
    let pairs = h6030(35);
    let rising = alphabet.word(&pairs).expect("letters of the I_a menu");
    let a = alphabet.word(&pairs[34..]).expect("the last letter");
    let back = library.build("inverse_of", vec![library.word(&a)]);
    let back_and_again = library.build("there_and_back", vec![back.clone(), RunValue::int(1)]);
    assert_eq!(library.pairs(&back), a.inverse().pairs());
    assert_eq!(library.pairs(&back_and_again), a.inverse().concat(&a).expect("one alphabet").pairs());
    let solved = i_a.solved().state().clone();
    let raised = witnessed(&i_a.menu, &twists(&i_a.letters(&pairs))).state().clone();
    let words = [
        ("H6030[..35] from solved", &solved, library.word(&rising), (true, false, false)),
        ("a⁻¹,a at the raised state", &raised, back_and_again, (false, false, true)),
        ("a⁻¹ at the raised state", &raised, back, (false, true, false)),
    ];
    for (name, start, word, (rises, tame_expected, returns)) in words {
        let letters = i_a.letters(&library.pairs(&word));
        let (peak, excursion, tame, end) = recomputed(start, &letters);
        assert_eq!((excursion > 0, tame), (rises, tame_expected), "{name}");
        let end = end.digest(ctx);
        assert_eq!(end == start.digest(ctx), returns, "{name}: the end is the start");
        let mut host = Rows::of(start.clone());
        let seen = completed(library.run(WORD_CHECKS.name, "word_observations", vec![word], &mut host), name);
        assert_eq!(host.asked, ["replay"], "{name}");
        assert_eq!((truth(get(&seen, "complete")), number(get(&seen, "played"))), (true, letters.len() as i64));
        let claim = only(get(&seen, "claims"));
        let claimed = (
            number(get(claim, "max_left_height")),
            number(get(claim, "excursion")),
            truth(get(claim, "tame")),
            string(get(claim, "end_digest")),
        );
        assert_eq!(claimed, (peak, excursion, tame, end.as_str()), "{name}: computed apart");
        let last = field(&observed(start, &letters), "final").clone();
        let kernel =
            (int(&last, "peak_left_height"), int(&last, "excursion"), flag(&last, "tame"), text(&last, "end_digest"));
        assert_eq!(kernel, (claimed.0, claimed.1, Some(claimed.2), claimed.3), "{name}: the final claims");
        let check = checked(start, &letters, None);
        let kernel = (
            int(&check, "peak_left_height"),
            int(&check, "excursion"),
            flag(&check, "tame"),
            text(&check, "end_digest"),
        );
        assert_eq!(kernel, (claimed.0, claimed.1, Some(claimed.2), claimed.3), "{name}: the whole-word check");
    }
}

/// A verdict that `word_protection` returned, written as M8's report (`levels::WordProtection::to_json`).
fn report(verdict: &RunValue) -> Value {
    let ids = |name: &str| Value::Array(numbers(get(verdict, name)).into_iter().map(Value::Int).collect());
    let prefix = numbers(get(verdict, "first_refused_prefix"));
    let mut fields = BTreeMap::new();
    for name in ["level", "mode", "verdict"] {
        fields.insert(name.to_string(), Value::String(string(get(verdict, name)).to_string()));
    }
    fields.insert("protected".to_string(), ids("protected"));
    fields.insert("pieces".to_string(), ids("pieces"));
    fields.insert("first_refused_prefix".to_string(), prefix.first().map_or(Value::Null, |&n| Value::Int(n)));
    Value::Object(fields)
}

/// M8's protection report of `letters` from `base` in `mode`: the derived value's, or a refused derivation's.
fn kernel_report(base: &Snapshot, letters: &[Letter], mode: ProtectionMode, protect: &[u32]) -> Value {
    match derive(base, letters, mode, protect) {
        Derivation::Derived(derived) => derived.protection().expect("a protection report").to_json(),
        Derivation::Refused(refused) => refused.protection.expect("a protection refusal").to_json(),
    }
}

/// CT-LB3-06 (part: the library entry, "the step flag is recomputed separately" and "a false flag with no protection
/// conflict does not itself refuse an admissible word"). `jumble_word_checks.step_flag` reads `word_check` for a named
/// unit and recomputes the flag from its end facts, without the kernel's flag. From solved, after a letter that lifts
/// the unit and a second letter at a cap disjoint from it, the letter that lands the unit gets a true flag and a letter
/// that lifts lattice pieces a false one, equal to the kernel's flag and to the flag computed apart from the start and
/// end states. Each of the flag's three terms is also the only one that fails on an admissible word, by the terms
/// computed apart and by the kernel's check, and the entry's flag is false on each, as the kernel's is: from that
/// state, a letter that leaves a piece of the unit off the lattice while the other two terms hold; the letter that
/// lands the unit, then a letter that lifts lattice pieces; the letter that lands the unit, then a letter at the second
/// cap that moves another off piece to a new pose off the lattice. The false flag of the letter that lifts lattice
/// pieces refuses nothing: the word is admissible, `word_observations` plays it, and with the pieces of the unit that
/// it leaves in place protected, `word_protection` is clear under Net and Strict, as `levels::derive` derives the
/// word. A word refused at a blocked cap gets a false flag, as in the kernel, and an empty unit is refused with
/// `KN2:input.named_object_invalid`, the kernel's refusal of the named object.
#[test]
fn ct_lb3_06_the_step_flag_is_recomputed_and_a_false_flag_refuses_nothing() {
    let ctx = ctx();
    let i_a = fixture("I_a");
    let alphabet = i_a.alphabet();
    let library = Library::new(&alphabet);
    let a = i_a.menu.for_cap(ctx, 0).expect("the menu at cap 0").iter().position(|t| t.kidx.is_none());
    let a = a.expect("a letter outside K⁺") as i32;
    let inverse = alphabet.inverse_item(a).expect("an item of the menu");
    let mut start = State::new(Some(i_a.menu.clone()));
    let first = start.apply(ctx, &i_a.letters(&[[0, a]])[0].1, None, &[]).expect("no engine error");
    assert_eq!(first.status, Status::Admissible);
    let unit = first.inside.clone();
    let lands = alphabet.word(&[[0, inverse]]).expect("one letter");
    let mut second = None;
    for cap in 1..600 {
        let mut work = start.clone();
        let outcome = work.apply(ctx, &i_a.letters(&[[cap, a]])[0].1, None, &[]).expect("no engine error");
        let disjoint = outcome.inside.iter().all(|piece| unit.binary_search(piece).is_err());
        if outcome.status == Status::Admissible
            && disjoint
            && play(&mut work.clone(), &i_a.letters(lands.pairs())[0]) == Status::Admissible
        {
            second = Some((cap, work));
            break;
        }
    }
    let (second_cap, start) = second.expect("a second cap disjoint from the unit");
    let lifts = ((second_cap + 1)..600)
        .map(|cap| alphabet.word(&[[cap, a]]).expect("one letter"))
        .find(|word| {
            let mut work = start.clone();
            play(&mut work, &i_a.letters(word.pairs())[0]) == Status::Admissible
                && work.off_lattice_count() > start.off_lattice_count()
        })
        .expect("an admissible letter that lifts lattice pieces");

    for (name, word, expected) in [("the unit lands", &lands, true), ("lattice pieces are lifted", &lifts, false)] {
        let letters = i_a.letters(word.pairs());
        let (_, _, _, end) = recomputed(&start, &letters);
        let mut host = Rows::of(start.clone());
        let args = vec![library.word(word), pieces(&unit)];
        let flagged = completed(library.run(WORD_CHECKS.name, "step_flag", args, &mut host), name);
        assert_eq!(host.asked, ["word_check"], "{name}");
        let answer = (truth(get(&flagged, "admissible")), number(get(&flagged, "applied")));
        assert_eq!(answer, (true, 1), "{name}");
        assert_eq!(truth(get(&flagged, "step_flag")), expected, "{name}");
        assert_eq!(step_flag(&start, &end, &unit, true), expected, "{name}: computed apart");
        assert_eq!(flag(&checked(&start, &letters, Some(&unit)), "step_flag"), Some(expected), "{name}: the kernel's");
        let seen =
            completed(library.run(WORD_CHECKS.name, "word_observations", vec![library.word(word)], &mut host), name);
        assert_eq!((truth(get(&seen, "complete")), number(get(&seen, "played"))), (true, 1), "{name}: no refusal");
    }

    // Each term alone: a letter leaves a piece of the unit off while the other two terms hold; the unit lands and then
    // lattice pieces are lifted; the unit lands and then another off piece moves to a new pose off the lattice. Each
    // word is found by playing one letter from `start`, or after `lands`, until the terms computed apart are the
    // wanted ones, and it moves a piece.
    let items = i_a.menu.items.len() as i32;
    let all = |cap: i32| (0..items).map(move |item| [cap, item]);
    let mut landed = start.clone();
    assert_eq!(play(&mut landed, &i_a.letters(lands.pairs())[0]), Status::Admissible);
    let found = |from: &State, prefix: &[[i32; 2]], pair: [i32; 2], terms: (bool, bool, bool)| {
        let letter = alphabet.word(&[pair]).ok()?;
        let mut end = from.clone();
        let admitted = play(&mut end, &i_a.letters(letter.pairs())[0]) == Status::Admissible;
        let moves = (0..NP).any(|p| end.pose_of(p) != from.pose_of(p));
        let pairs: Vec<[i32; 2]> = prefix.iter().copied().chain([pair]).collect();
        (admitted && moves && step_terms(&start, &end, &unit) == terms).then(|| alphabet.word(&pairs).expect("letters"))
    };
    let stays_off = (1..600).flat_map(all).find_map(|pair| found(&start, &[], pair, (false, false, true)));
    let lifting = (1..600).find_map(|cap| found(&landed, lands.pairs(), [cap, a], (true, true, true)));
    let changing = all(second_cap).find_map(|pair| found(&landed, lands.pairs(), pair, (true, false, false)));
    let isolating = [
        ("a unit piece stays off", stays_off.expect("a letter failing only the unit's term"), 1, (false, false, true)),
        ("lattice pieces are lifted", lifting.expect("a letter failing only the lattice term"), 2, (true, true, true)),
        ("another off piece moves", changing.expect("a letter failing only the others' term"), 2, (true, false, false)),
    ];
    for (name, word, length, terms) in isolating {
        let letters = i_a.letters(word.pairs());
        let mut host = Rows::of(start.clone());
        let args = vec![library.word(&word), pieces(&unit)];
        let flagged = completed(library.run(WORD_CHECKS.name, "step_flag", args, &mut host), name);
        let answer =
            (truth(get(&flagged, "admissible")), number(get(&flagged, "applied")), truth(get(&flagged, "step_flag")));
        assert_eq!(answer, (true, length, false), "{name}: that term alone fails");
        let check = checked(&start, &letters, Some(&unit));
        assert_eq!(kernel_terms(&check), terms, "{name}: the kernel's terms");
        assert_eq!(flag(&check, "step_flag"), Some(false), "{name}: the kernel's flag");
    }

    // No protection conflict: the false flag's word with pieces of the unit that it leaves in place protected.
    let snapshot = witnessed(&i_a.menu, &twists(&i_a.letters(&[[0, a], [second_cap, a]])));
    assert_eq!(snapshot.state().digest(ctx), start.digest(ctx));
    let letters = i_a.letters(lifts.pairs());
    let (_, _, _, lifted) = recomputed(&start, &letters);
    let kept: Vec<u32> =
        unit.iter().copied().filter(|&p| lifted.pose_of(p as usize) == start.pose_of(p as usize)).collect();
    assert!(!kept.is_empty(), "the lifting letter leaves pieces of the unit in place");
    let mut host = Rows::at(&snapshot);
    let args = vec![library.word(&lifts), pieces(&kept)];
    let verdicts = completed(library.run(PROTECTION.name, "word_protection", args, &mut host), "protection");
    for (mode, name) in [(ProtectionMode::Net, "net"), (ProtectionMode::Strict, "strict")] {
        let verdict = get(&verdicts, name);
        assert_eq!(string(get(verdict, "verdict")), "clear", "{name}");
        assert_eq!(report(verdict), kernel_report(&snapshot, &letters, mode, &kept), "{name}");
        assert!(matches!(derive(&snapshot, &letters, mode, &kept), Derivation::Derived(_)), "{name}: derived");
    }

    // A refused word, and an empty unit.
    let refused_at = i_a.after(12);
    let state = refused_at.state();
    let grips: Vec<usize> = (0..12).collect();
    let statuses = state.clone().survey(ctx, &grips, None).expect("a survey");
    let blocked = grips.iter().copied().find(|&g| statuses[g] == Status::Blocked).expect("a blocked cap") as i32;
    let refused = alphabet.word(&[[blocked, rotation(&i_a.menu)]]).expect("one letter");
    let first_off = (0..NP).find(|&p| !state.is_lattice_piece(p)).expect("a piece off the lattice");
    let pose = state.pose_ids()[first_off];
    let group: Vec<u32> =
        (0..NP).filter(|&p| !state.is_lattice_piece(p) && state.pose_ids()[p] == pose).map(|p| p as u32).collect();
    let mut host = Rows::of(state.clone());
    let flagged = completed(
        library.run(WORD_CHECKS.name, "step_flag", vec![library.word(&refused), pieces(&group)], &mut host),
        "a refused word",
    );
    let answer =
        (truth(get(&flagged, "admissible")), number(get(&flagged, "applied")), truth(get(&flagged, "step_flag")));
    assert_eq!(answer, (false, 0, false));
    let check = checked(state, &i_a.letters(refused.pairs()), Some(&group));
    assert_eq!((flag(&check, "admissible"), flag(&check, "step_flag")), (Some(false), Some(false)));
    let empty = library.run(WORD_CHECKS.name, "step_flag", vec![library.word(&lands), pieces(&[])], &mut host);
    assert_eq!(stopped(empty, "an empty unit"), codes::NAMED_OBJECT_INVALID);
    let check = checked(&start, &i_a.letters(lands.pairs()), Some(&[]));
    assert_eq!((text(&check, "status"), text(&check, "code")), ("refused", codes::NAMED_OBJECT_INVALID));
}

/// CT-LB3-06 (part: the library entry, a whole word's Net and Strict protection at pose level, with M8's
/// `levels::derive`, pull request 182). At the state after H6030's first 35 letters from solved, witnessed by them,
/// three pieces that a⁻¹ (the last letter's inverse) moves and one that it leaves in place are protected. The word
/// a⁻¹,a, built by the language, ends where it starts, so Net is clear, while its first prefix moves the pieces, so
/// Strict reports them with first refused prefix 1; a⁻¹ alone moves them under both modes, with no prefix under Net.
/// Every verdict carries its level `poses`, its mode and the protected pieces, and equals `levels::derive`'s report in
/// that mode. A protected piece outside the model is refused with `KN2:input.named_object_invalid`.
#[test]
fn ct_lb3_06_whole_word_protection_at_pose_level() {
    let i_a = fixture("I_a");
    let alphabet = i_a.alphabet();
    let library = Library::new(&alphabet);
    let pairs = h6030(35);
    let raised = witnessed(&i_a.menu, &twists(&i_a.letters(&pairs)));
    let a = alphabet.word(&pairs[34..]).expect("the last letter");
    let back = library.build("inverse_of", vec![library.word(&a)]);
    let back_and_again = library.build("there_and_back", vec![back.clone(), RunValue::int(1)]);

    let start = raised.state();
    let mut work = start.clone();
    assert_eq!(play(&mut work, &i_a.letters(&library.pairs(&back))[0]), Status::Admissible);
    let moved: Vec<u32> = (0..NP as u32).filter(|&p| work.pose_of(p as usize) != start.pose_of(p as usize)).collect();
    let still =
        (0..NP as u32).find(|&p| work.pose_of(p as usize) == start.pose_of(p as usize)).expect("a piece in place");
    assert!(moved.len() >= 3, "a⁻¹ moves pieces");
    let moving = moved[..3].to_vec();
    let mut protected = moving.clone();
    protected.push(still);
    protected.sort_unstable();

    let none: &[i64] = &[];
    let cases = [
        ("a⁻¹,a", back_and_again, [("net", "clear", none, none), ("strict", "moves", &[][..], &[1][..])]),
        ("a⁻¹", back.clone(), [("net", "moves", &[][..], none), ("strict", "moves", &[][..], &[1][..])]),
    ];
    let moving_ids: Vec<i64> = moving.iter().map(|&p| i64::from(p)).collect();
    let protected_ids: Vec<i64> = protected.iter().map(|&p| i64::from(p)).collect();
    for (name, word, expected) in cases {
        let letters = i_a.letters(&library.pairs(&word));
        let mut host = Rows::at(&raised);
        let args = vec![word, pieces(&protected)];
        let verdicts = completed(library.run(PROTECTION.name, "word_protection", args, &mut host), name);
        assert_eq!(host.asked, ["protection", "protection"], "{name}");
        for ((mode, finding, _, prefix), kernel_mode) in
            expected.iter().zip([ProtectionMode::Net, ProtectionMode::Strict])
        {
            let verdict = get(&verdicts, mode);
            assert_eq!((string(get(verdict, "level")), string(get(verdict, "mode"))), ("poses", *mode), "{name}");
            assert_eq!(string(get(verdict, "verdict")), *finding, "{name}: {mode}");
            assert_eq!(numbers(get(verdict, "protected")), protected_ids, "{name}: {mode}");
            let expected_pieces: &[i64] = if *finding == "moves" { &moving_ids } else { none };
            assert_eq!(numbers(get(verdict, "pieces")), expected_pieces, "{name}: {mode}");
            assert_eq!(numbers(get(verdict, "first_refused_prefix")), *prefix, "{name}: {mode}");
            assert_eq!(report(verdict), kernel_report(&raised, &letters, kernel_mode, &protected), "{name}: {mode}");
        }
    }

    let mut host = Rows::at(&raised);
    let outside = library.run(PROTECTION.name, "word_protection", vec![back, pieces(&[NP as u32])], &mut host);
    assert_eq!(stopped(outside, "a piece outside the model"), codes::NAMED_OBJECT_INVALID);
}
