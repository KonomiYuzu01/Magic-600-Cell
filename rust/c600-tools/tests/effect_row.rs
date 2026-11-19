//! KN2.4's `effect` row served to a library entry by the host bridge (KN5; LP2 draft). CT-LB2-01 (part: a host row
//! serves KN2.4's `effect` to a library entry; LB2's recipes and reports run whole in `tests/ct_lb2_01.rs`). An entry's
//! snapshot read with an argument, `read_snapshot<T>("effect", recipe)`, runs on `c600-run` through `Boundary` and
//! [`Bridge`] onto a command layer and answers the recipe's net map and what it touches (`c600_tools::kernel_rows`).
//! The row reads no state. The refusal with Jumble on is not run here: a layer in Jumble on needs a pinned twist menu,
//! which `c600-jumble` builds, and this crate's tests do not depend on it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row, TYPE_MISMATCH};
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::{codes, Derivation, Kernel, ProtectedSet, Snapshot, SLOTS};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{effect_tariff, effect_type, EFFECT, MAX_PRIMITIVES};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const MODULE: &str = "reader";

/// A stand-in for a library entry over the row, with variants that declare other types, pass another argument or read
/// a row the bridge does not serve.
const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule reader requires [];\n\n\
    export fn net_of(w: Word) -> record { net_sha256: Text, moved_stickers: Nat } effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text, moved_stickers: Nat }>(\"effect\", w);\n}\n\n\
    export fn net_of_ints(w: List<Int>) -> record { moved_stickers: Int, net_sha256: Text } effects [snapshot] {\n    \
    return read_snapshot<record { moved_stickers: Int, net_sha256: Text }>(\"effect\", w);\n}\n\n\
    export fn support_of(steps: List<record { kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }>) -> \
    record { support: List<record { stickers: Nat, orbit: Int, pieces: Nat }> } effects [snapshot] {\n    \
    return read_snapshot<record { support: List<record { stickers: Nat, orbit: Int, pieces: Nat }> }>(\"effect\", \
    steps);\n}\n\n\
    export fn as_list(w: Word) -> List<Nat> effects [snapshot] {\n    \
    return read_snapshot<List<Nat>>(\"effect\", w);\n}\n\n\
    export fn other_field(w: Word) -> record { net: Text } effects [snapshot] {\n    \
    return read_snapshot<record { net: Text }>(\"effect\", w);\n}\n\n\
    export fn nat_orbit(w: Word) -> record { support: List<record { orbit: Nat, pieces: Nat, stickers: Nat }> } \
    effects [snapshot] {\n    \
    return read_snapshot<record { support: List<record { orbit: Nat, pieces: Nat, stickers: Nat }> }>(\"effect\", \
    w);\n}\n\n\
    export fn one_letter(m: Int) -> record { net_sha256: Text } effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text }>(\"effect\", m);\n}\n\n\
    export fn short_steps(steps: List<record { kind: Text, moves: Word }>) -> record { net_sha256: Text } \
    effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text }>(\"effect\", steps);\n}\n\n\
    export fn steps_of(steps: List<record { kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }>) -> \
    record { net_sha256: Text } effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text }>(\"effect\", steps);\n}\n\n\
    export fn colours(w: Word) -> record { net_sha256: Text } effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text }>(\"colours\", w);\n}\n\n\
    export fn colours_of(m: Int) -> record { net_sha256: Text } effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text }>(\"colours\", m);\n}\n\n\
    export fn colours_of_steps(steps: List<record { kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }>) -> \
    record { net_sha256: Text } effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text }>(\"colours\", steps);\n}\n\n\
    export fn colours_of_short(steps: List<record { kind: Text, moves: Word }>) -> record { net_sha256: Text } \
    effects [snapshot] {\n    \
    return read_snapshot<record { net_sha256: Text }>(\"colours\", steps);\n}\n";

const LIMITS: Amounts = Amounts { steps: 1 << 20, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };

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

fn loaded() -> Loaded {
    load(&lower_source(SOURCE).expect("the stand-in lowers")).expect("the stand-in loads")
}

fn wide(letters: &[i32]) -> Vec<i64> {
    letters.iter().map(|&m| i64::from(m)).collect()
}

/// The state [`SCRAMBLE`] derives from solved.
fn scrambled() -> Snapshot {
    let kernel = kernel();
    let recipe = kernel.word(&wide(&SCRAMBLE)).expect("a well-formed word");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

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
        module: sha256(SOURCE.as_bytes()),
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
    let run = loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(LIMITS)).expect("the run starts");
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    run
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

fn step(kind: &str, moves: &[i32], node: i64, orbit: i64, sign: i64) -> Value {
    Value::record(vec![
        ("kind", Value::text(kind)),
        ("moves", Value::word(moves)),
        ("node", Value::int(node)),
        ("orbit", Value::int(orbit)),
        ("sign", Value::int(sign)),
    ])
}

fn ints(letters: &[i32]) -> Value {
    Value::list(letters.iter().map(|&m| Value::int(m)).collect())
}

/// The row answers the fields an entry declares, in its order, for a word, a list of integers or a list of step
/// records, and the same recipe in any of the three forms has the same effect: the kernel's, at the head of a scrambled
/// session as at solved, since the row reads no state. A read changes nothing.
#[test]
fn a_library_entry_reads_a_recipes_effect() {
    let kernel = kernel();
    let effect = kernel.effect(&kernel.word(&wide(&SCRAMBLE)).unwrap()).unwrap();
    let net = Value::record(vec![
        ("net_sha256", Value::text(&effect.net_sha256)),
        ("moved_stickers", Value::int(effect.moved_stickers() as u64)),
    ]);
    let row = |row: &c600_kernel::SupportRow| {
        let (stickers, pieces) = (Value::int(row.stickers as u64), Value::int(row.pieces as u64));
        Value::record(vec![("stickers", stickers), ("orbit", Value::int(row.orbit)), ("pieces", pieces)])
    };
    let support = Value::record(vec![("support", Value::list(effect.support.iter().map(row).collect()))]);
    let (first, second) = SCRAMBLE.split_at(7);
    let steps = Value::list(vec![step("word", first, 0, 0, 0), step("word", second, 0, 0, 0)]);

    for start in [kernel.solved(), scrambled()] {
        let mut layer = layer_at(start);
        let before = (layer.revision().clone(), layer.store().log.len());
        assert!(equal(completed(&run(&mut layer, "net_of", Value::word(&SCRAMBLE))), &net));
        let reordered = completed(&run(&mut layer, "net_of_ints", ints(&SCRAMBLE))).clone();
        let Value::Record(fields) = &reordered else { panic!("a record") };
        let names: Vec<&str> = fields.fields().iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["moved_stickers", "net_sha256"], "the declared order");
        assert!(equal(fields.get("net_sha256").unwrap(), &Value::text(&effect.net_sha256)));
        assert!(equal(completed(&run(&mut layer, "support_of", steps.clone())), &support));
        assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
    }
}

/// The row's refusals (LP2 draft, recorded in `c600_tools::kernel_rows`). A declared type the row does not answer (a
/// list, a record with a field the row does not have, a support whose orbit is a `Nat`) is `LG2:type.mismatch`, and an
/// argument in neither recipe form (a single integer, step records without `node`, `orbit` and `sign`, a word step with
/// a node, a star step with moves) is `KN5:input.argument_form`: each before the row runs, so the read costs what a
/// read of a row the bridge does not serve costs with the same argument (`LG3:capability.not_granted`). A recipe the
/// kernel refuses (no step, a star of orbit 35, of sign 0 or past its tree, 257 steps) stops with the kernel's code,
/// and the read is charged the row's tariff for no primitive. A refused read changes nothing.
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer_at(scrambled());
    let before = (layer.revision().clone(), layer.store().log.len());
    for (entry, arg, code, other) in [
        ("as_list", Value::word(&[1]), TYPE_MISMATCH, "colours"),
        ("other_field", Value::word(&[1]), TYPE_MISMATCH, "colours"),
        ("nat_orbit", Value::word(&[1]), TYPE_MISMATCH, "colours"),
        ("one_letter", Value::int(1), code::ARGUMENT_FORM, "colours_of"),
        ("steps_of", Value::list(vec![step("word", &[1], 1, 0, 0)]), code::ARGUMENT_FORM, "colours_of_steps"),
        ("steps_of", Value::list(vec![step("star", &[1], 0, 0, 1)]), code::ARGUMENT_FORM, "colours_of_steps"),
        ("steps_of", Value::list(vec![step("turn", &[], 0, 0, 1)]), code::ARGUMENT_FORM, "colours_of_steps"),
    ] {
        let cost = refused(&mut layer, entry, &arg, code);
        assert_eq!(cost, refused(&mut layer, other, &arg, NOT_GRANTED), "{entry}: refused before the row runs");
    }
    let short = Value::list(vec![Value::record(vec![("kind", Value::text("word")), ("moves", Value::word(&[1]))])]);
    let cost = refused(&mut layer, "short_steps", &short, code::ARGUMENT_FORM);
    assert_eq!(cost, refused(&mut layer, "colours_of_short", &short, NOT_GRANTED));

    let tariff = effect_tariff(0, &effect_type()).expect("a type the row answers").kernel_cost;
    assert_eq!(tariff, SLOTS as u128, "a refused recipe is charged for no primitive");
    let many = Value::list((0..257).map(|_| step("word", &[1], 0, 0, 0)).collect());
    for (arg, expected) in [
        (Value::list(Vec::new()), codes::RECIPE_SHAPE),
        (Value::list(vec![step("star", &[], 0, 35, 1)]), codes::RECIPE_SHAPE),
        (Value::list(vec![step("star", &[], 0, 0, 0)]), codes::RECIPE_SHAPE),
        (Value::list(vec![step("star", &[], 2396, 0, 1)]), codes::RECIPE_SHAPE),
        (Value::list(vec![step("word", &[1], 0, 0, 0), step("star", &[], 2396, 0, 1)]), codes::RECIPE_SHAPE),
        (many, codes::RECIPE_SHAPE),
    ] {
        let cost = refused(&mut layer, "steps_of", &arg, expected);
        assert_eq!(cost, refused(&mut layer, "colours_of_steps", &arg, NOT_GRANTED) + tariff, "{arg:?}: charged");
    }
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's draft tariff covers its answer: the whole answer for a scramble, asked of the bridge directly, fits the
/// result bound of its tariff, and the tariff counts one pass over the slots and one unit per primitive. The bound for
/// all fields covers the largest answer: every slot moved, the most support rows and [`MAX_PRIMITIVES`]. A read sends
/// no invocation.
#[test]
fn the_tariff_covers_the_largest_answer() {
    let kernel = kernel();
    let mut layer = layer_at(kernel.solved());
    let mut bridge = Bridge::new(&mut layer, "direct");
    let mut arg = Vec::new();
    encode(&Value::word(&SCRAMBLE), &mut arg).expect("a canonical word");
    let ty = effect_type();
    let request = Request::SnapshotWith { query: EFFECT, arg: &arg, ty: &ty };
    let tariff = bridge.tariff(&request).unwrap();
    assert_eq!(tariff, effect_tariff(SCRAMBLE.len(), &ty).unwrap());
    assert_eq!(tariff.kernel_cost, (SLOTS + SCRAMBLE.len()) as u128);
    let answer = bridge.perform(&request).unwrap();
    assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");

    let all: Vec<i64> = (0..SLOTS as i64).collect();
    let slots = Value::list(all.iter().map(|&slot| Value::int(slot)).collect());
    let largest = Value::record(vec![
        ("destination", slots.clone()),
        ("expansion_sha256", Value::text(&"f".repeat(64))),
        ("moved_pieces", Value::int(c600_kernel::PIECES as u64)),
        ("moved_stickers", Value::int(SLOTS as u64)),
        ("net_sha256", Value::text(&"f".repeat(64))),
        ("primitive_count", Value::int(MAX_PRIMITIVES as u64)),
        ("source", slots),
        (
            "support",
            Value::list(
                (-1..35)
                    .map(|orbit| {
                        let most = Value::int((SLOTS - 1) as u64);
                        Value::record(vec![("orbit", Value::int(orbit)), ("pieces", most.clone()), ("stickers", most)])
                    })
                    .collect(),
            ),
        ),
    ]);
    assert!(largest.size() <= effect_tariff(MAX_PRIMITIVES, &ty).unwrap().result_bound);
}

/// The built-in column admits an entry that reads the row only when its class lists `Row::Read("effect")`, and the
/// entry reads state and prepares no preview (LG3.13).
#[test]
fn a_reading_entry_needs_the_read_row_in_its_class() {
    let program = lower_source(SOURCE).unwrap();
    let read: BTreeSet<Row> = [Row::Read(EFFECT.into())].into();
    let admitted = admit_built_in(&program, MODULE, "net_of", &read, &Grant::none(), &Policy::interim()).unwrap();
    assert!(admitted.is_admitted(), "{:?}", admitted.diagnostics);
    assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);
    let none = BTreeSet::new();
    let refused = admit_built_in(&program, MODULE, "net_of", &none, &Grant::none(), &Policy::interim()).unwrap();
    assert!(!refused.is_admitted());
}
