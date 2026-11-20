//! KN2.4's `positions` row served to a library entry by the host bridge (KN5; LP2 draft). An entry's snapshot read with
//! an argument, `read_snapshot<T>("positions", pieces)`, runs on `c600-run` through `Boundary` and [`Bridge`] onto a
//! command layer and reads identities, slots and orientations at the head (`c600_tools::kernel_rows`). Each test names
//! the conformance parts it covers; none of those IDs runs whole here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row, TYPE_MISMATCH};
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::{Derivation, Kernel, PiecePosition, ProtectedSet, Snapshot, PIECES};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{positions_tariff, positions_type, MAX_PIECE_STICKERS, POSITIONS};
use c600_tools::tool::{checker, class_rows, contract, lower_source, manifest, named_word_block, pins, Manifest};

const MODULE: &str = "reader";

/// The reader's record type, its fields in the order the entries declare them.
const ROW: &str = "record { piece: Nat, orbit: Int, home: List<Nat>, slots: List<Nat>, position: Int }";

/// A stand-in for a library entry over the row, with variants that declare other types, pass another argument or read
/// a row the bridge does not serve.
const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule reader requires [];\n\n\
    export fn where_are(pieces: List<Nat>) -> List<ROW> effects [snapshot] {\n    \
    return read_snapshot<List<ROW>>(\"positions\", pieces);\n}\n\n\
    export fn in_set(pieces: Set<Nat>) -> List<ROW> effects [snapshot] {\n    \
    return read_snapshot<List<ROW>>(\"positions\", pieces);\n}\n\n\
    export fn any(pieces: List<Int>) -> List<ROW> effects [snapshot] {\n    \
    return read_snapshot<List<ROW>>(\"positions\", pieces);\n}\n\n\
    export fn reordered(pieces: List<Nat>) -> List<WIDE> effects [snapshot] {\n    \
    return read_snapshot<List<WIDE>>(\"positions\", pieces);\n}\n\n\
    export fn too_few(pieces: List<Int>) -> List<record { piece: Nat }> effects [snapshot] {\n    \
    return read_snapshot<List<record { piece: Nat }>>(\"positions\", pieces);\n}\n\n\
    export fn orbit_as_nat(pieces: List<Int>) -> List<NAT_ORBIT> effects [snapshot] {\n    \
    return read_snapshot<List<NAT_ORBIT>>(\"positions\", pieces);\n}\n\n\
    export fn one_piece(piece: Int) -> List<ROW> effects [snapshot] {\n    \
    return read_snapshot<List<ROW>>(\"positions\", piece);\n}\n\n\
    export fn replay(pieces: List<Int>) -> List<ROW> effects [snapshot] {\n    \
    return read_snapshot<List<ROW>>(\"replay\", pieces);\n}\n\n\
    export fn replay_of(piece: Int) -> List<ROW> effects [snapshot] {\n    \
    return read_snapshot<List<ROW>>(\"replay\", piece);\n}\n";

/// The fields in another order, the whole numbers declared `Int`.
const WIDE: &str = "record { slots: List<Int>, position: Int, piece: Int, home: List<Int>, orbit: Int }";

/// `orbit` declared `Nat`, which cannot hold a fixed cell centre's -1.
const NAT_ORBIT: &str = "record { piece: Nat, orbit: Nat, home: List<Nat>, slots: List<Nat>, position: Int }";

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

fn source() -> String {
    SOURCE.replace("NAT_ORBIT", NAT_ORBIT).replace("WIDE", WIDE).replace("ROW", ROW)
}

fn loaded() -> Loaded {
    load(&lower_source(&source()).expect("the reader lowers")).expect("the reader loads")
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
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), "reader", start, none)
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
    Context { run: "read".into(), base, producer, reads_state: true, inputs: vec![named_word_block()], pins: pins() }
}

/// Runs `entry` on `arg` through a boundary and a bridge onto `layer`.
fn run(layer: &mut CommandLayer, entry: &str, arg: Value) -> Run {
    let context = context(layer, entry);
    let mut bridge = Bridge::new(layer, "read");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(LIMITS)).expect("the run starts")
}

fn pieces(pieces: &[i64]) -> Value {
    Value::list(pieces.iter().map(|&piece| Value::int(piece)).collect())
}

/// The kernel's rows as the row answers them, fields in `where_are`'s order.
fn expected(rows: &[PiecePosition]) -> Value {
    let ints = |values: &[i32]| Value::list(values.iter().map(|&n| Value::int(n)).collect());
    let record = |row: &PiecePosition| {
        Value::record(vec![
            ("piece", Value::int(row.piece as u64)),
            ("orbit", Value::int(row.orbit)),
            ("home", ints(&row.home)),
            ("slots", ints(&row.slots)),
            ("position", Value::int(row.position.map_or(-1, |position| position as i64))),
        ])
    };
    Value::list(rows.iter().map(record).collect())
}

/// The field names of an answer's first record, in order.
fn first_fields(answer: &Value) -> Vec<String> {
    let Value::List(rows) = answer else { panic!("a list: {answer:?}") };
    let Some(Value::Record(record)) = rows.items().first() else { panic!("a record first: {answer:?}") };
    record.fields().iter().map(|(name, _)| name.as_str().to_owned()).collect()
}

fn completed(run: &Run) -> &Value {
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

/// CT-LB2-04 (part: a host row serves KN2.4's `positions` to a library entry; LB2's exact-completion entry runs whole
/// in `tests/exact_completion.rs`). In a scrambled session an entry reads, for the pieces it names, each piece's
/// identity, orbit, home slots and the slots that hold its stickers, so their orientation, as the kernel reads them at
/// the head. A piece in its home position but turned answers slots other than its home slots, and a piece moved away
/// answers the position that holds it. The answer follows the asked order, repeats and a fixed cell centre (orbit -1)
/// included, and the declared field order; whole numbers may be declared `Int`, and a set names its pieces in
/// increasing order. No piece answers no row. A read changes nothing.
#[test]
fn a_library_entry_reads_identities_slots_and_orientations() {
    let start = scrambled();
    let kernel = kernel();
    let all: Vec<i64> = (0..PIECES as i64).collect();
    let rows = kernel.positions(&start, &all).unwrap();
    let turned = rows.iter().find(|row| row.position_correct() && !row.solved()).expect("a turned piece").piece;
    let moved = rows.iter().find(|row| !row.position_correct()).expect("a moved piece").piece;
    let centre = rows.iter().find(|row| row.orbit == -1).expect("a fixed cell centre").piece;
    let asked = [turned, moved, centre, 0, moved].map(|piece| piece as i64);

    let mut layer = layer_at(start.clone());
    let before = (layer.revision().clone(), layer.store().log.len());
    let read = run(&mut layer, "where_are", pieces(&asked));
    let answer = completed(&read);
    let rows = kernel.positions(&start, &asked).unwrap();
    assert!(equal(answer, &expected(&rows)), "{answer:?}");
    assert_eq!(first_fields(answer), ["piece", "orbit", "home", "slots", "position"]);
    assert_ne!(rows[0].slots, rows[0].home, "the turned piece answers its orientation");
    assert_eq!(rows[0].position, Some(turned));
    assert_ne!(rows[1].position, Some(moved), "the moved piece answers where it is");
    assert_eq!(rows[2].orbit, -1);
    assert!(equal(&expected(&rows[4..]), &expected(&rows[1..2])));

    let set = Value::set(asked.iter().map(|&piece| Value::int(piece)).collect());
    let mut increasing = asked.to_vec();
    increasing.sort_unstable();
    increasing.dedup();
    let rows = kernel.positions(&start, &increasing).unwrap();
    assert!(equal(completed(&run(&mut layer, "in_set", set)), &expected(&rows)));

    let reordered = run(&mut layer, "reordered", pieces(&asked));
    assert_eq!(first_fields(completed(&reordered)), ["slots", "position", "piece", "home", "orbit"]);
    assert!(equal(completed(&run(&mut layer, "where_are", pieces(&[]))), &Value::list(Vec::new())));
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's refusals (LP2 draft, recorded in `c600_tools::kernel_rows`). A declared type the row does not answer
/// (fields missing, or `orbit` declared `Nat`) is `LG2:type.mismatch`, and an argument that is not a list of integers
/// is `KN5:input.argument_form`: each before the kernel runs, so the read costs what a read of a row the bridge does
/// not serve costs with the same argument (`LG3:capability.not_granted`): its step and the argument's encoding. A
/// piece outside 0 to 177,119 is the kernel's `KN2:input.named_object_invalid` once the read is charged its tariff. A
/// refused read changes nothing.
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer_at(scrambled());
    let before = (layer.revision().clone(), layer.store().log.len());
    for (entry, arg, code, other) in [
        ("too_few", pieces(&[0]), TYPE_MISMATCH, "replay"),
        ("orbit_as_nat", pieces(&[0]), TYPE_MISMATCH, "replay"),
        ("one_piece", Value::int(0), code::ARGUMENT_FORM, "replay_of"),
    ] {
        let cost = refused(&mut layer, entry, &arg, code);
        assert_eq!(cost, refused(&mut layer, other, &arg, NOT_GRANTED), "{entry}: refused before the kernel runs");
    }
    let tariff = positions_tariff(2).kernel_cost;
    for bad in [-1, PIECES as i64, 1 << 62] {
        let arg = pieces(&[0, bad]);
        let cost = refused(&mut layer, "any", &arg, code::NAMED_OBJECT_INVALID);
        assert_eq!(cost, refused(&mut layer, "replay", &arg, NOT_GRANTED) + tariff, "{bad}: the read is charged");
    }
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's draft tariff covers its answer: [`MAX_PIECE_STICKERS`] is the kernel's largest piece, and the answer for
/// every piece, asked of the bridge directly, fits the result bound of its tariff. The tariff depends only on the
/// number of named pieces.
#[test]
fn the_tariff_covers_the_largest_answer() {
    let kernel = kernel();
    let all: Vec<i64> = (0..PIECES as i64).collect();
    let largest = kernel.positions(&kernel.solved(), &all).unwrap().iter().map(|row| row.home.len()).max();
    assert_eq!(largest, Some(MAX_PIECE_STICKERS));

    let mut layer = layer_at(scrambled());
    let mut bridge = Bridge::new(&mut layer, "direct");
    let mut arg = Vec::new();
    encode(&pieces(&all), &mut arg).expect("a canonical list");
    let ty = positions_type();
    let request = Request::SnapshotWith { query: POSITIONS, arg: &arg, ty: &ty };
    let tariff = bridge.tariff(&request).unwrap();
    assert_eq!(tariff, positions_tariff(PIECES));
    let answer = bridge.perform(&request).unwrap();
    assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
}

/// The built-in column admits an entry that reads the row only when its class lists `Row::Read("positions")`, and the
/// entry reads state and prepares no preview (LG3.13). A class built from a manifest's rows ([`class_rows`]) lists
/// preview rows only, so it does not admit the entry: LP3's admission of a built-in reading entry needs the read row.
#[test]
fn a_reading_entry_needs_the_read_row_in_its_class() {
    let program = lower_source(&source()).unwrap();
    let read: BTreeSet<Row> = [Row::Read(POSITIONS.into())].into();
    let admitted = admit_built_in(&program, MODULE, "where_are", &read, &Grant::none(), &Policy::interim()).unwrap();
    assert!(admitted.is_admitted(), "{:?}", admitted.diagnostics);
    assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);

    let previews = class_rows(&Manifest { rows: &[], ..manifest() });
    let refused = admit_built_in(&program, MODULE, "where_are", &previews, &Grant::none(), &Policy::interim()).unwrap();
    assert!(!refused.is_admitted());
}
