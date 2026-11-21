//! The `orbits` row for 600-cell-Full served to a library entry by the host bridge (KN5; H-08's `reads` since the
//! command table's freeze, CM.18). An entry's snapshot read with an argument, `read_snapshot<T>("orbits", orbits)`,
//! runs on `c600-run` through `Boundary` and [`Bridge`] onto a command layer and answers the pieces of the named
//! orbits: model data, which reads no state of the session (`c600_tools::kernel_rows`). The references are the
//! kernel's own: the `positions` row the bridge serves, the kernel's progress rows, one per moving orbit with its piece
//! count, and the model's seed atlas, `assets/seed_atlas.json`, with each moving orbit's piece count. Every run here
//! runs within the draft budget ceiling, `Budget::DRAFT_CEILING`. The refusal with Jumble on is not run here: a layer
//! in Jumble on needs a pinned twist menu, which `c600-jumble` builds, and this crate's tests do not depend on it. A
//! pass is source and fixture evidence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row, TYPE_MISMATCH};
use c600_command::{code, Budget, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, ORBITS, PIECES, SLOTS};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{orbits_tariff, orbits_type, MAX_ORBIT_PIECES, ORBITS_ROW};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const MODULE: &str = "catalogue";

/// A stand-in for a library entry over the row, with variants that declare other types, pass another argument or read
/// a row the bridge does not serve, and an entry over the `positions` row.
const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule catalogue requires [];\n\n\
    export fn orbits_of(orbits: List<Int>) -> List<record { orbit: Int, pieces: List<Nat> }> effects [snapshot] {\n    \
    return read_snapshot<List<record { orbit: Int, pieces: List<Nat> }>>(\"orbits\", orbits);\n}\n\n\
    export fn reordered(orbits: List<Int>) -> List<record { pieces: List<Int>, orbit: Int }> effects [snapshot] {\n    \
    return read_snapshot<List<record { pieces: List<Int>, orbit: Int }>>(\"orbits\", orbits);\n}\n\n\
    export fn short(orbits: List<Int>) -> List<record { orbit: Int }> effects [snapshot] {\n    \
    return read_snapshot<List<record { orbit: Int }>>(\"orbits\", orbits);\n}\n\n\
    export fn nat_orbit(orbits: List<Int>) -> List<record { orbit: Nat, pieces: List<Nat> }> effects [snapshot] {\n    \
    return read_snapshot<List<record { orbit: Nat, pieces: List<Nat> }>>(\"orbits\", orbits);\n}\n\n\
    export fn one_orbit(orbit: Int) -> List<record { orbit: Int, pieces: List<Nat> }> effects [snapshot] {\n    \
    return read_snapshot<List<record { orbit: Int, pieces: List<Nat> }>>(\"orbits\", orbit);\n}\n\n\
    export fn colours(orbits: List<Int>) -> List<record { orbit: Int }> effects [snapshot] {\n    \
    return read_snapshot<List<record { orbit: Int }>>(\"colours\", orbits);\n}\n\n\
    export fn colours_of(orbit: Int) -> List<record { orbit: Int }> effects [snapshot] {\n    \
    return read_snapshot<List<record { orbit: Int }>>(\"colours\", orbit);\n}\n\n\
    export fn positions_of(pieces: List<Nat>) -> List<record { home: List<Nat>, orbit: Int, piece: Nat, \
    position: Int, slots: List<Nat> }> effects [snapshot] {\n    \
    return read_snapshot<List<record { home: List<Nat>, orbit: Int, piece: Nat, position: Int, \
    slots: List<Nat> }>>(\"positions\", pieces);\n}\n";

/// The draft budget ceiling of every profile (`Budget::DRAFT_CEILING`), as the run's limits.
const CEILING: Amounts = Amounts {
    steps: Budget::DRAFT_CEILING.steps,
    kernel_cost: Budget::DRAFT_CEILING.kernel_cost,
    memory: Budget::DRAFT_CEILING.memory,
    output: Budget::DRAFT_CEILING.output,
};

/// A fixed scramble from solved.
const SCRAMBLE: [i32; 20] =
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
    LOADED.get_or_init(|| load(&lower_source(SOURCE).expect("the stand-in lowers")).expect("the stand-in loads"))
}

/// The model's seed atlas, one row per moving orbit in orbit order.
fn atlas() -> &'static [Json] {
    static ATLAS: OnceLock<Json> = OnceLock::new();
    let path = repository().join("assets/seed_atlas.json");
    let document = ATLAS.get_or_init(|| {
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        json::parse(&bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    });
    let rows = document.as_array().expect("one atlas row per orbit");
    assert_eq!(rows.len(), ORBITS, "one atlas row per moving orbit");
    rows
}

/// The state [`SCRAMBLE`] derives from solved.
fn scrambled() -> Snapshot {
    let kernel = kernel();
    let letters: Vec<i64> = SCRAMBLE.iter().map(|&m| i64::from(m)).collect();
    let recipe = kernel.word(&letters).expect("a well-formed word");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// A layer whose session starts at `start`, with no protected orbit.
fn layer_at(start: Snapshot) -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), "catalogue", start, none)
}

/// The context of one run of `entry` in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer, entry: &str) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(SOURCE.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    Context { run: "orbits".into(), base, producer, reads_state: true, inputs: vec![named_word_block()], pins: pins() }
}

/// Runs `entry` on `arg` within the draft ceiling through a boundary and a bridge onto `layer`; a read sends no
/// invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, arg: Value) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    let context = context(layer, entry);
    let mut bridge = Bridge::new(layer, "catalogue");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(CEILING)).expect("the run starts");
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    let after = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    assert_eq!(after, before, "a read changes nothing");
    run
}

fn completed(run: Run) -> Value {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
    run.value.expect("an answer")
}

/// The kernel cost of a read of `entry` on `arg` that stops with `expected`.
fn refused(layer: &mut CommandLayer, entry: &str, arg: &Value, expected: &str) -> u128 {
    let read = run(layer, entry, arg.clone());
    let summary = &read.summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(expected)), "{entry}: {:?}", summary.stop);
    summary.used.kernel_cost
}

fn list(items: &[i64]) -> Value {
    Value::list(items.iter().map(|&item| Value::int(item)).collect())
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.get(name).unwrap_or_else(|| panic!("a field {name}: {value:?}"))
}

fn names(value: &Value) -> Vec<&str> {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.fields().iter().map(|(name, _)| name.as_str()).collect()
}

fn int(value: &Value) -> i64 {
    let Value::Int(n) = value else { panic!("an integer: {value:?}") };
    n.to_string().parse().expect("an i64")
}

fn items(value: &Value) -> &[Value] {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items()
}

fn ints(value: &Value) -> Vec<i64> {
    items(value).iter().map(int).collect()
}

/// Every orbit, -1 to 34.
fn every_orbit() -> Vec<i64> {
    (-1..ORBITS as i64).collect()
}

/// The row answers model data for every orbit, -1 to 34: its pieces in increasing order, the same at solved and at a
/// scrambled head. The 36 answers split the 177,120 pieces between them, and the largest has [`MAX_ORBIT_PIECES`].
/// Each answer agrees with the kernel's orbit table: the `positions` row, read through the bridge at the scrambled
/// head, gives each of its pieces that orbit, and for the 35 moving orbits its count is the kernel's progress row's
/// and the seed atlas's. A read changes nothing.
#[test]
fn the_row_answers_each_orbits_pieces_as_the_kernel_orbit_table_gives_them() {
    let kernel = kernel();
    let every = every_orbit();
    let at_solved = completed(run(&mut layer_at(kernel.solved()), "orbits_of", list(&every)));
    let mut layer = layer_at(scrambled());
    let answer = completed(run(&mut layer, "orbits_of", list(&every)));
    assert!(equal(&answer, &at_solved), "the row reads no state");

    let progress = kernel.progress(&kernel.solved()).expect("the solved state's progress");
    assert_eq!(progress.rows.len(), ORBITS, "one progress row per moving orbit");
    let records = items(&answer);
    assert_eq!(records.len(), every.len(), "one record per named orbit");
    let mut seen = vec![false; PIECES];
    let mut largest = 0;
    for (record, &orbit) in records.iter().zip(&every) {
        assert_eq!(names(record), ["orbit", "pieces"]);
        assert_eq!(int(field(record, "orbit")), orbit);
        let pieces = ints(field(record, "pieces"));
        assert!(!pieces.is_empty() && pieces.windows(2).all(|pair| pair[0] < pair[1]), "orbit {orbit}: increasing");
        for &piece in &pieces {
            let piece = usize::try_from(piece).expect("a piece 0 to 177,119");
            assert!(!seen[piece], "orbit {orbit}: piece {piece} is in one orbit only");
            seen[piece] = true;
        }
        largest = largest.max(pieces.len());

        let positions = completed(run(&mut layer, "positions_of", list(&pieces)));
        let rows = items(&positions);
        assert_eq!(rows.len(), pieces.len(), "orbit {orbit}: one position per piece");
        for (row, &piece) in rows.iter().zip(&pieces) {
            assert_eq!((int(field(row, "piece")), int(field(row, "orbit"))), (piece, orbit), "the positions row");
        }
        if let Ok(moving) = usize::try_from(orbit) {
            let row = &progress.rows[moving];
            assert_eq!((row.orbit, row.pieces), (moving, pieces.len()), "orbit {orbit}: the kernel's progress row");
            let pinned = atlas()[moving].get("pieces").and_then(Json::as_i64);
            assert_eq!(pinned, Some(pieces.len() as i64), "orbit {orbit}: the seed atlas");
        }
    }
    assert!(seen.iter().all(|&piece| piece), "the 36 orbits hold all 177,120 pieces");
    assert_eq!(largest, MAX_ORBIT_PIECES, "the largest orbit");
}

/// The answer holds one record per named orbit in the argument's order, repeats included, with its fields in the
/// declared order, and `pieces` may be declared `List<Int>`.
#[test]
fn the_row_answers_in_the_argument_and_declared_order() {
    let mut layer = layer_at(kernel().solved());
    let every = completed(run(&mut layer, "orbits_of", list(&every_orbit())));
    let named = [34, -1, 34, 7];
    let answer = completed(run(&mut layer, "reordered", list(&named)));
    let records = items(&answer);
    assert_eq!(records.len(), named.len());
    for (record, &orbit) in records.iter().zip(&named) {
        assert_eq!(names(record), ["pieces", "orbit"]);
        assert_eq!(int(field(record, "orbit")), orbit);
        let index = usize::try_from(orbit + 1).expect("an orbit -1 to 34");
        assert_eq!(ints(field(record, "pieces")), ints(field(&items(&every)[index], "pieces")), "orbit {orbit}");
    }
}

/// The row's refusals (draft, recorded in `c600_tools::kernel_rows`). A declared type the row does not answer (a field
/// missing, or `orbit` declared `Nat`) is `LG2:type.mismatch`, and an argument that is not a list of integers is
/// `KN5:input.argument_form`: each before the row runs, so the read costs what a read of a row the bridge does not
/// serve costs with the same argument (`LG3:capability.not_granted`). An orbit outside -1 to 34 is
/// `KN2:input.named_object_invalid`, and the read is charged the row's tariff for the orbits named. A refused read
/// changes nothing.
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer_at(scrambled());
    for (entry, arg, code, other) in [
        ("short", list(&[0]), TYPE_MISMATCH, "colours"),
        ("nat_orbit", list(&[0]), TYPE_MISMATCH, "colours"),
        ("one_orbit", Value::int(0), code::ARGUMENT_FORM, "colours_of"),
    ] {
        let cost = refused(&mut layer, entry, &arg, code);
        assert_eq!(cost, refused(&mut layer, other, &arg, NOT_GRANTED), "{entry}: refused before the row runs");
    }
    for orbit in [-2, ORBITS as i64, i64::MAX, i64::MIN] {
        let arg = list(&[0, orbit]);
        let cost = refused(&mut layer, "orbits_of", &arg, code::NAMED_OBJECT_INVALID);
        let tariff = orbits_tariff(2).kernel_cost;
        assert_eq!(cost, refused(&mut layer, "colours", &arg, NOT_GRANTED) + tariff, "orbit {orbit}: charged");
    }
}

/// The row's tariff covers its answer: an answer asked of the bridge directly fits the result bound of its
/// tariff, for every orbit at once and for each orbit alone, and the tariff counts one pass over the slots and
/// [`MAX_ORBIT_PIECES`] for each named orbit. A read sends no invocation.
#[test]
fn the_tariff_covers_the_largest_answer() {
    let mut layer = layer_at(kernel().solved());
    let mut bridge = Bridge::new(&mut layer, "direct");
    let ty = orbits_type();
    let every = every_orbit();
    let mut named: Vec<Vec<i64>> = every.iter().map(|&orbit| vec![orbit]).collect();
    named.push(every.clone());
    for orbits in named {
        let mut arg = Vec::new();
        encode(&list(&orbits), &mut arg).expect("a canonical list");
        let request = Request::SnapshotWith { query: ORBITS_ROW, arg: &arg, ty: &ty };
        let tariff = bridge.tariff(&request).unwrap();
        assert_eq!(tariff, orbits_tariff(orbits.len()));
        assert_eq!(tariff.kernel_cost, (SLOTS + orbits.len() * MAX_ORBIT_PIECES) as u128);
        let answer = bridge.perform(&request).unwrap();
        assert!(answer.size() <= tariff.result_bound, "{orbits:?}: {} > {}", answer.size(), tariff.result_bound);
    }
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
}

/// The built-in column admits an entry that reads the row only when its class lists `Row::Read("orbits")`, and the
/// entry reads and prepares no preview (LG3.13).
#[test]
fn a_reading_entry_needs_the_read_row_in_its_class() {
    let program = lower_source(SOURCE).unwrap();
    let read: BTreeSet<Row> = [Row::Read(ORBITS_ROW.into())].into();
    let admitted = admit_built_in(&program, MODULE, "orbits_of", &read, &Grant::none(), &Policy::interim()).unwrap();
    assert!(admitted.is_admitted(), "{:?}", admitted.diagnostics);
    assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);
    let none = BTreeSet::new();
    let refused = admit_built_in(&program, MODULE, "orbits_of", &none, &Grant::none(), &Policy::interim()).unwrap();
    assert!(!refused.is_admitted());
}
