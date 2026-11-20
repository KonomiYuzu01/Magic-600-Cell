//! The two draft structure rows of LP3's representations tool, served to an entry by the host bridge (KN5; LP3's C6;
//! drafts until the command table freezes at LP3). An entry's snapshot read with an argument,
//! `read_snapshot<T>("structure.compare", cells)` or `read_snapshot<T>("filter.preview", predicate)`, runs on
//! `c600-run` through `Boundary` and [`Bridge`] onto a command layer, and the bridge answers it from LP2's structure
//! rows, `CommandLayer::compare_cells` and `CommandLayer::layer_counts` (`c600_tools::kernel_rows`). `c600-repr`'s
//! `tests/served_rows.rs` runs the representations tool's own entries on the same rows through the command and source
//! routes. The refusal with Jumble on is not run here: a layer in Jumble on needs a pinned twist menu, which
//! `c600-jumble` builds, and this crate's tests do not depend on it. Covers no conformance ID.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::TYPE_MISMATCH;
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::{CellComparison, Derivation, Kernel, LayerCounts, ProtectedSet, Snapshot, PIECES, SLOTS};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{
    filter_preview_tariff, filter_preview_type, structure_compare_tariff, structure_compare_type, CELLS,
    FILTER_PREVIEW, STRUCTURE_COMPARE,
};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const MODULE: &str = "structures";

/// The two cells and the layer predicate as the representations tool passes them, and with `Int`.
const CELLS_NAT: &str = "record { first: Nat, second: Nat }";
const CELLS_INT: &str = "record { first: Int, second: Int }";
const PREDICATE_NAT: &str = "record { cell: Nat, layer: Nat }";
const PREDICATE_INT: &str = "record { cell: Int, layer: Int }";

/// The two answers as the representations tool declares them.
const COMPARED: &str = "record { first: Nat, second: Nat, steps: Nat }";
const COUNTS: &str = "record { cell: Nat, layer: Nat, pieces: Nat, slots: Nat }";

/// A stand-in for an entry over the rows, with variants that declare another order and `Int` for whole numbers,
/// declare types the rows do not answer, pass another argument or read a row the bridge does not serve.
fn source() -> String {
    let entry = |name: &str, param: &str, answer: &str, query: &str| {
        format!(
            "export fn {name}(asked: {param}) -> {answer} effects [snapshot] {{\n    \
             return read_snapshot<{answer}>(\"{query}\", asked);\n}}\n\n"
        )
    };
    let reordered = "record { steps: Int, second: Int, first: Int }";
    let counted = "record { slots: Int, pieces: Int, layer: Int, cell: Int }";
    let more = "record { cell: Nat, layer: Nat, pieces: Nat, slots: Nat, steps: Nat }";
    [
        "edition \"6cs-draft-1\";\nmodule structures requires [];\n\n".to_owned(),
        entry("compare", CELLS_NAT, COMPARED, STRUCTURE_COMPARE),
        entry("compare_any", CELLS_INT, reordered, STRUCTURE_COMPARE),
        entry("compare_short", CELLS_INT, "record { first: Nat, steps: Nat }", STRUCTURE_COMPARE),
        entry("compare_text", CELLS_INT, "record { first: Nat, second: Nat, steps: Text }", STRUCTURE_COMPARE),
        entry("compare_list", "List<Int>", COMPARED, STRUCTURE_COMPARE),
        entry("compare_layer", PREDICATE_INT, COMPARED, STRUCTURE_COMPARE),
        entry("count", PREDICATE_NAT, COUNTS, FILTER_PREVIEW),
        entry("count_any", PREDICATE_INT, counted, FILTER_PREVIEW),
        entry("count_more", PREDICATE_INT, more, FILTER_PREVIEW),
        entry("count_one", "Int", COUNTS, FILTER_PREVIEW),
        entry("count_cells", CELLS_INT, COUNTS, FILTER_PREVIEW),
        entry("colours_of_cells", CELLS_INT, COMPARED, "colours"),
        entry("colours_of_layer", PREDICATE_INT, COUNTS, "colours"),
        entry("colours_of_list", "List<Int>", COMPARED, "colours"),
        entry("colours_of_one", "Int", COUNTS, "colours"),
    ]
    .concat()
    .trim_end()
    .to_owned()
        + "\n"
}

const LIMITS: Amounts = Amounts { steps: 1 << 20, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };

/// A fixed scramble from solved.
const SCRAMBLE: [i64; 20] =
    [17, -305, 911, 4, -1200, 600, 33, -88, 1021, -7, 450, 2, -999, 316, 75, -640, 1199, -12, 808, 23];

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

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| load(&lower_source(&source()).expect("the stand-in lowers")).expect("the stand-in loads"))
}

/// The state [`SCRAMBLE`] derives from solved.
fn scrambled() -> Snapshot {
    let kernel = kernel();
    let recipe = kernel.word(&SCRAMBLE).expect("a well-formed word");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// A layer whose session starts at `start`, with no protected orbit.
fn layer_at(start: Snapshot) -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), MODULE, start, none)
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

/// Runs `entry` on `arg` through a boundary and a bridge onto `layer`; a read sends no invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, arg: Value) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len());
    let context = context(layer, entry);
    let mut bridge = Bridge::new(layer, MODULE);
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(LIMITS)).expect("the run starts");
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "a read changes nothing");
    run
}

fn answer(run: &Run) -> &Value {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
    run.value.as_ref().expect("an answer")
}

/// The kernel cost of a read of `entry` on `arg` that stops with `expected`.
fn refused(layer: &mut CommandLayer, entry: &str, arg: &Value, expected: &str) -> u128 {
    let read = run(layer, entry, arg.clone());
    let summary = &read.summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(expected)), "{entry}: {:?}", summary.stop);
    summary.used.kernel_cost
}

fn nat(n: usize) -> Value {
    Value::int(n as u64)
}

fn cells(first: i64, second: i64) -> Value {
    Value::record(vec![("first", Value::int(first)), ("second", Value::int(second))])
}

fn predicate(cell: i64, layer: i64) -> Value {
    Value::record(vec![("cell", Value::int(cell)), ("layer", Value::int(layer))])
}

/// The answer a `structure.compare` read declared as [`COMPARED`] gives for a comparison.
fn compared(comparison: CellComparison) -> Value {
    let CellComparison { first, second, steps } = comparison;
    Value::record(vec![("first", nat(first)), ("second", nat(second)), ("steps", nat(steps))])
}

/// The answer a `filter.preview` read declared as [`COUNTS`] gives for a layer's counts.
fn counts(counts: LayerCounts) -> Value {
    let LayerCounts { cell, layer, pieces, slots } = counts;
    Value::record(vec![("cell", nat(cell)), ("layer", nat(layer)), ("pieces", nat(pieces)), ("slots", nat(slots))])
}

fn names(value: &Value) -> Vec<String> {
    let Value::Record(fields) = value else { panic!("a record: {value:?}") };
    fields.fields().iter().map(|(name, _)| name.as_str().to_owned()).collect()
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.get(name).unwrap_or_else(|| panic!("a field {name}: {value:?}"))
}

/// A cell the most face steps from cell 0, with its comparison.
fn farthest() -> CellComparison {
    let kernel = kernel();
    let all = (0..CELLS as i64).map(|cell| kernel.compare_cells(0, cell).expect("a cell"));
    all.max_by_key(|comparison| comparison.steps).expect("600 cells")
}

/// An entry reads two cells compared and the counts of a layer predicate as LP2's structure rows give them: the fewest
/// face steps between the cells, 0 for a cell with itself, and the positions and home slots of the layer around the
/// cell. The answers' fields follow the declared order, and whole numbers may be declared `Int`. Both rows read no
/// state: a scrambled session gives the solved session's answers. A read changes nothing and sends no invocation.
#[test]
fn an_entry_reads_two_cells_compared_and_the_counts_of_a_layer() {
    let far = farthest();
    assert!(far.steps > 1 && far.steps < CELLS, "{far:?}");
    let last = kernel().cell_layers(7).expect("a cell").len() - 1;
    let pairs = [(0, far.second as i64), (5, 5), (1, 2)];
    let layers = [(7, 0), (7, 2), (7, last as i64), (599, 1)];
    let mut solved = layer_at(kernel().solved());
    let mut scrambled = layer_at(scrambled());
    assert_ne!(solved.snapshot().digest(), scrambled.snapshot().digest());
    for layer in [&mut solved, &mut scrambled] {
        for (first, second) in pairs {
            let expected = compared(layer.compare_cells(first, second).expect("two cells"));
            let read = run(layer, "compare", cells(first, second));
            assert!(equal(answer(&read), &expected), "{first}, {second}: {:?}", read.value);
            let any = run(layer, "compare_any", cells(first, second));
            assert_eq!(names(answer(&any)), ["steps", "second", "first"]);
            for name in ["first", "second", "steps"] {
                assert!(equal(field(answer(&any), name), field(&expected, name)), "{name}");
            }
        }
        let same = run(layer, "compare", cells(5, 5));
        assert!(equal(field(answer(&same), "steps"), &nat(0)), "a cell with itself");
        for (cell, named) in layers {
            let expected = counts(layer.layer_counts(cell, named).expect("a layer"));
            assert!(equal(&expected, &counts(kernel().cell_layers(cell).expect("a cell")[named as usize])));
            let read = run(layer, "count", predicate(cell, named));
            assert!(equal(answer(&read), &expected), "{cell}, {named}: {:?}", read.value);
            let any = run(layer, "count_any", predicate(cell, named));
            assert_eq!(names(answer(&any)), ["slots", "pieces", "layer", "cell"]);
            for name in ["cell", "layer", "pieces", "slots"] {
                assert!(equal(field(answer(&any), name), field(&expected, name)), "{name}");
            }
        }
    }
}

/// The rows' refusals (LP3 draft, recorded in `c600_tools::kernel_rows`). A declared type a row does not answer (a
/// field missing, a field of text, a field more) is `LG2:type.mismatch`, and an argument of another form (a list, a
/// number, the other row's record) is `KN5:input.argument_form`: each before the row runs, so the read costs what a
/// read of a row the bridge does not serve costs with the same argument (`LG3:capability.not_granted`). A cell outside
/// 0 to 599, of any size, and a layer below 0 or past the cell's last are `KN2:input.named_object_invalid` once the
/// read is charged its tariff. A refused read changes nothing.
#[test]
fn the_rows_refuse_what_they_do_not_answer() {
    let mut layer = layer_at(kernel().solved());
    let list = Value::list(vec![Value::int(1), Value::int(2)]);
    for (entry, arg, code, unserved) in [
        ("compare_short", cells(1, 2), TYPE_MISMATCH, "colours_of_cells"),
        ("compare_text", cells(1, 2), TYPE_MISMATCH, "colours_of_cells"),
        ("compare_list", list, code::ARGUMENT_FORM, "colours_of_list"),
        ("compare_layer", predicate(1, 2), code::ARGUMENT_FORM, "colours_of_layer"),
        ("count_more", predicate(7, 2), TYPE_MISMATCH, "colours_of_layer"),
        ("count_one", Value::int(7), code::ARGUMENT_FORM, "colours_of_one"),
        ("count_cells", cells(7, 2), code::ARGUMENT_FORM, "colours_of_cells"),
    ] {
        let cost = refused(&mut layer, entry, &arg, code);
        assert_eq!(cost, refused(&mut layer, unserved, &arg, NOT_GRANTED), "{entry}: refused before the row runs");
    }

    let huge = Value::int(u64::MAX);
    let charged = structure_compare_tariff().kernel_cost;
    assert_eq!(charged, 1);
    let beyond = Value::record(vec![("first", Value::int(0)), ("second", huge.clone())]);
    for arg in [cells(0, CELLS as i64), cells(-1, 0), beyond] {
        let cost = refused(&mut layer, "compare_any", &arg, code::NAMED_OBJECT_INVALID);
        assert_eq!(cost, refused(&mut layer, "colours_of_cells", &arg, NOT_GRANTED) + charged, "{arg:?}: charged");
    }
    let last = kernel().cell_layers(7).expect("a cell").len() as i64 - 1;
    let charged = filter_preview_tariff().kernel_cost;
    assert_eq!(charged, SLOTS as u128);
    let beyond = Value::record(vec![("cell", huge), ("layer", Value::int(0))]);
    for arg in [predicate(CELLS as i64, 0), predicate(-1, 0), predicate(7, last + 1), predicate(7, -1), beyond] {
        let cost = refused(&mut layer, "count_any", &arg, code::NAMED_OBJECT_INVALID);
        assert_eq!(cost, refused(&mut layer, "colours_of_layer", &arg, NOT_GRANTED) + charged, "{arg:?}: charged");
    }
}

/// The rows' draft tariffs cover their answers: the answers for the pair of cells farthest apart and for every layer
/// around cell 0, asked of the bridge directly, fit the result bound, and each bound is the size of the largest answer
/// the row can give: three integers of 599 for `structure.compare`; a cell and a layer of 599, 177,120 positions and
/// 259,800 slots for `filter.preview`. `structure.compare` costs one lookup, `filter.preview` one pass over the home
/// slots. A read sends no invocation.
#[test]
fn the_tariffs_cover_the_largest_answers() {
    let far = farthest();
    let around = kernel().cell_layers(0).expect("a cell");
    assert!(around.len() < CELLS && around.iter().all(|counted| counted.pieces <= PIECES && counted.slots <= SLOTS));
    let mut layer = layer_at(kernel().solved());
    let mut bridge = Bridge::new(&mut layer, "direct");
    let last = nat(CELLS - 1);

    let ty = structure_compare_type();
    let mut arg = Vec::new();
    encode(&cells(0, far.second as i64), &mut arg).expect("a canonical argument");
    let request = Request::SnapshotWith { query: STRUCTURE_COMPARE, arg: &arg, ty: &ty };
    let tariff = bridge.tariff(&request).unwrap();
    assert_eq!(tariff, structure_compare_tariff());
    assert_eq!(tariff.kernel_cost, 1);
    let answer = bridge.perform(&request).unwrap();
    assert!(equal(&answer, &compared(far)), "{answer:?}");
    assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    let largest = Value::record(vec![("first", last.clone()), ("second", last.clone()), ("steps", last.clone())]);
    assert_eq!(largest.size(), tariff.result_bound);

    let ty = filter_preview_type();
    for counted in &around {
        let mut arg = Vec::new();
        encode(&predicate(0, counted.layer as i64), &mut arg).expect("a canonical argument");
        let request = Request::SnapshotWith { query: FILTER_PREVIEW, arg: &arg, ty: &ty };
        let tariff = bridge.tariff(&request).unwrap();
        assert_eq!(tariff, filter_preview_tariff());
        assert_eq!(tariff.kernel_cost, SLOTS as u128);
        let answer = bridge.perform(&request).unwrap();
        assert!(equal(&answer, &counts(*counted)), "{answer:?}");
        assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    }
    let largest =
        Value::record(vec![("cell", last.clone()), ("layer", last), ("pieces", nat(PIECES)), ("slots", nat(SLOTS))]);
    assert_eq!(largest.size(), filter_preview_tariff().result_bound);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
}
