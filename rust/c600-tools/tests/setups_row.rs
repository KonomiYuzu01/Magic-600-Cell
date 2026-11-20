//! The draft `setups` row served to a library entry by the host bridge (KN5; LP2 draft; BND.1). CT-BND-18 (part: a
//! host row serves the setup search for a target the solver names; the library entry runs whole in
//! `tests/ct_bnd_18.rs`). An entry's snapshot read with an argument, `read_snapshot<T>("setups", target)`, runs on
//! `c600-run` through `Boundary` and [`Bridge`] onto a command layer, and the bridge answers it from the certified
//! guarded setup tables the host gives it (`c600_tools::kernel_rows::SetupTable`). Two refusals are not run here: with
//! Jumble on, because a layer in Jumble on needs a pinned twist menu, which `c600-jumble` builds and this crate's tests
//! do not depend on; and for a table of another model, because no second model's assets exist to load one from.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::TYPE_MISMATCH;
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::archive::gunzip;
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Kernel, ProtectedSet, PIECES, SLOTS};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{
    setups_tariff, setups_type, SetupTable, GENERATORS, MAX_PIECE_STICKERS, SETUPS, SETUP_FAMILY, SETUP_TABLE,
};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const MODULE: &str = "searches";

/// The target type, `record { orbit: Nat, position: Nat }`.
const TARGET: &str = "record { orbit: Nat, position: Nat }";

/// A stand-in for a library entry over the row, with variants that declare another order and `Int` for whole numbers,
/// declare types the row does not answer, pass another argument or read a row the bridge does not serve.
fn source() -> String {
    let entry = |name: &str, param: &str, answer: &str, query: &str| {
        format!(
            "export fn {name}(goal: {param}) -> {answer} effects [snapshot] {{\n    \
             return read_snapshot<{answer}>(\"{query}\", goal);\n}}\n\n"
        )
    };
    let answer = "record { buffers: List<Nat>, family: Text, orbit: Nat, position: Nat, setups: List<record { frame: \
                  List<Nat>, node: Nat, setup: Word }>, table_sha256: Text }";
    let ordered = "record { table_sha256: Text, setups: List<record { setup: Word, node: Int, frame: List<Int> }>, \
                   position: Int, orbit: Int, family: Text, buffers: List<Int> }";
    let short =
        "record { buffers: List<Nat>, orbit: Nat, position: Nat, setups: List<record { frame: List<Nat>, node: \
                 Nat, setup: Word }>, table_sha256: Text }";
    let listed = "record { buffers: List<Nat>, family: Text, orbit: Nat, position: Nat, setups: List<record { frame: \
                  List<Nat>, node: Nat, setup: List<Int> }>, table_sha256: Text }";
    let triple = "record { node: Nat, orbit: Nat, position: Nat }";
    [
        "edition \"6cs-draft-1\";\nmodule searches requires [];\n\n".to_owned(),
        entry("named", TARGET, answer, SETUPS),
        entry("ordered", TARGET, ordered, SETUPS),
        entry("short", TARGET, short, SETUPS),
        entry("listed", TARGET, listed, SETUPS),
        entry("of_nat", "Nat", answer, SETUPS),
        entry("of_triple", triple, answer, SETUPS),
        entry("colours", TARGET, answer, "colours"),
    ]
    .concat()
    .trim_end()
    .to_owned()
        + "\n"
}

const LIMITS: Amounts = Amounts { steps: 1 << 20, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    let kernel = KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => Arc::new(kernel),
        Err(error) => panic!("the model in assets/ loads: {error}"),
    });
    kernel.clone()
}

fn table() -> Arc<SetupTable> {
    static TABLE: OnceLock<Arc<SetupTable>> = OnceLock::new();
    let table = TABLE.get_or_init(|| match SetupTable::load(&repository().join("assets")) {
        Ok(table) => Arc::new(table),
        Err(error) => panic!("the setup table in assets/ loads: {error}"),
    });
    table.clone()
}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| load(&lower_source(&source()).expect("the stand-in lowers")).expect("the stand-in loads"))
}

fn layer() -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(
        kernel(),
        Table::h08(),
        Secret::fixed([5; 32]),
        Default::default(),
        MODULE,
        kernel().solved(),
        none,
    )
}

/// The context of one run of `entry` in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer, entry: &str) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(source().as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    Context { run: MODULE.into(), base, producer, reads_state: true, inputs: vec![named_word_block()], pins: pins() }
}

/// Runs `entry` on `arg` through a boundary and a bridge onto `layer`, with the table when `table` is set; a read sends
/// no invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, arg: Value, table: bool) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len());
    let context = context(layer, entry);
    let mut bridge = Bridge::new(layer, MODULE).with_setups(table.then(self::table));
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(LIMITS)).expect("the run starts");
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "a read changes nothing");
    run
}

/// The kernel cost of a read of `entry` on `arg` that stops with `expected`.
fn refused(layer: &mut CommandLayer, entry: &str, arg: &Value, table: bool, expected: &str) -> u128 {
    let read = run(layer, entry, arg.clone(), table);
    let summary = &read.summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(expected)), "{entry}: {:?}", summary.stop);
    summary.used.kernel_cost
}

fn target(orbit: i64, position: i64) -> Value {
    Value::record(vec![("orbit", Value::int(orbit)), ("position", Value::int(position))])
}

fn names(value: &Value) -> Vec<String> {
    let Value::Record(fields) = value else { panic!("a record: {value:?}") };
    fields.fields().iter().map(|(name, _)| name.as_str().to_owned()).collect()
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.get(name).unwrap_or_else(|| panic!("a field {name}: {value:?}"))
}

/// The position of orbit 34 with the most setups, read from the table's own bytes, and how many it has.
fn widest() -> (i64, usize) {
    let bytes = std::fs::read(repository().join("assets").join(SETUP_TABLE)).unwrap();
    let trees = json::parse(&gunzip(&bytes, 1 << 30).unwrap()).unwrap();
    let positions = trees.as_array().unwrap()[34].get("positions").unwrap().as_array().unwrap().to_vec();
    let mut count: BTreeMap<i64, usize> = BTreeMap::new();
    for position in positions.iter().map(|p| Json::as_i64(p).unwrap()) {
        *count.entry(position).or_default() += 1;
    }
    count.into_iter().max_by_key(|&(_, n)| n).unwrap()
}

/// The row answers the fields an entry declares, in its order, and the fields of each setup in the declared order;
/// whole-number fields may be declared `Nat` or `Int`. A read changes nothing.
#[test]
fn a_library_entry_reads_setups_in_its_declared_order() {
    let mut layer = layer();
    let (position, _) = widest();
    let named = run(&mut layer, "named", target(34, position), true).value.expect("an answer");
    assert_eq!(names(&named), ["buffers", "family", "orbit", "position", "setups", "table_sha256"]);
    let ordered = run(&mut layer, "ordered", target(34, position), true).value.expect("an answer");
    assert_eq!(names(&ordered), ["table_sha256", "setups", "position", "orbit", "family", "buffers"]);
    for name in ["table_sha256", "position", "orbit", "family", "buffers"] {
        assert!(equal(field(&named, name), field(&ordered, name)), "{name}");
    }
    let (Value::List(first), Value::List(second)) = (field(&named, "setups"), field(&ordered, "setups")) else {
        panic!("two lists")
    };
    assert_eq!(first.items().len(), second.items().len());
    for (a, b) in first.items().iter().zip(second.items()) {
        assert_eq!(names(a), ["frame", "node", "setup"]);
        assert_eq!(names(b), ["setup", "node", "frame"]);
        for name in ["frame", "node", "setup"] {
            assert!(equal(field(a, name), field(b, name)), "{name}");
        }
    }
    let Value::Text(family) = field(&named, "family") else { panic!("a text") };
    assert_eq!(family.as_str(), SETUP_FAMILY);
}

/// The row's refusals (LP2 draft, recorded in `c600_tools::kernel_rows`). A declared type the row does not answer (a
/// field missing, or a setup declared `List<Int>`) is `LG2:type.mismatch`, and an argument of another form (a number, a
/// record with a third field) is `KN5:input.argument_form`: each before the row runs, so the read costs what a read of
/// a row the bridge does not serve costs with the same argument. A bridge the host gave no table grants no `setups`
/// read, `LG3:capability.not_granted`, at the same cost. An orbit outside 0 to 34, a position outside 0 to 177,119 and a
/// position outside the named orbit are `KN2:input.named_object_invalid`, charged the row's tariff for a target with no
/// setup. A refused read changes nothing.
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer();
    let named = target(0, 6290);
    let triple = Value::record(vec![("node", Value::int(0)), ("orbit", Value::int(0)), ("position", Value::int(6290))]);
    for (entry, arg, table, code) in [
        ("short", named.clone(), true, TYPE_MISMATCH),
        ("listed", named.clone(), true, TYPE_MISMATCH),
        ("of_nat", Value::int(6290), true, code::ARGUMENT_FORM),
        ("of_triple", triple, true, code::ARGUMENT_FORM),
        ("named", named.clone(), false, NOT_GRANTED),
    ] {
        let cost = refused(&mut layer, entry, &arg, table, code);
        let unserved = refused(&mut layer, "colours", &arg, true, NOT_GRANTED);
        assert_eq!(cost, unserved, "{entry}: refused before the row runs");
    }
    let charged = setups_tariff(0, table().deepest()).kernel_cost;
    assert_eq!(charged, SLOTS as u128);
    for arg in [target(35, 6290), target(0, PIECES as i64), target(1, 6290)] {
        let used = refused(&mut layer, "named", &arg, true, code::NAMED_OBJECT_INVALID);
        assert_eq!(used, refused(&mut layer, "colours", &arg, true, NOT_GRANTED) + charged, "{arg:?}: charged");
    }
}

/// The row's draft tariff covers its answer: the answer for the target with the most setups, asked of the bridge
/// directly, fits the result bound of its tariff, and the tariff counts one pass over the slots and three for each
/// setup the kernel certifies. The bound covers the largest answer: as many setups as any target has, each with a frame
/// of [`MAX_PIECE_STICKERS`] slots and a setup as long as the deepest. A read sends no invocation.
#[test]
fn the_tariff_covers_the_largest_answer() {
    let table = table();
    let (position, count) = widest();
    assert_eq!((count, table.widest()), (60, 60));
    let mut layer = layer();
    let mut bridge = Bridge::new(&mut layer, "direct").with_setups(Some(table.clone()));
    let mut arg = Vec::new();
    encode(&target(34, position), &mut arg).expect("a canonical argument");
    let ty = setups_type();
    let request = Request::SnapshotWith { query: SETUPS, arg: &arg, ty: &ty };
    let tariff = bridge.tariff(&request).unwrap();
    assert_eq!(tariff, setups_tariff(60, table.deepest()));
    assert_eq!(tariff.kernel_cost, (SLOTS + 60 * 3 * SLOTS) as u128);
    let answer = bridge.perform(&request).unwrap();
    assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");

    let largest = Value::int((SLOTS - 1) as u64);
    let setup = Value::record(vec![
        ("frame", Value::list(vec![largest.clone(); MAX_PIECE_STICKERS])),
        ("node", largest.clone()),
        ("setup", Value::word(&vec![-(GENERATORS as i32); table.deepest()])),
    ]);
    let answer = Value::record(vec![
        ("buffers", Value::list(vec![largest.clone(); 2])),
        ("family", Value::text(SETUP_FAMILY)),
        ("orbit", largest.clone()),
        ("position", largest),
        ("setups", Value::list(vec![setup; table.widest()])),
        ("table_sha256", Value::text(&"f".repeat(64))),
    ]);
    assert!(answer.size() <= setups_tariff(table.widest(), table.deepest()).result_bound);
}

/// The table loads through the kernel's asset check: its digest is the file's, its model ID the kernel's, it has the
/// 35 orbits' buffers, its deepest setup has 20 letters and its widest target 60 setups. A folder without the assets
/// does not load.
#[test]
fn the_table_loads_through_the_asset_check() {
    let table = table();
    let bytes = std::fs::read(repository().join("assets").join(SETUP_TABLE)).unwrap();
    assert_eq!(table.sha256(), sha256_hex(&bytes));
    assert_eq!(table.model_id(), kernel().identity().model_id);
    assert_eq!(table.buffers(0), Some([117, 175_946]));
    assert!((0..35).all(|orbit| table.buffers(orbit).is_some()) && table.buffers(35).is_none());
    assert_eq!((table.deepest(), table.widest()), (20, 60));
    assert_eq!(table.nodes(0, 6290), [0, 113]);
    assert!(table.nodes(0, 117).is_empty() && table.nodes(35, 6290).is_empty());
    assert_eq!(table.frame(0, 0), Some(&[7911, 9960][..]));
    assert!(SetupTable::load(&repository().join("no-such-assets")).is_err());
}
