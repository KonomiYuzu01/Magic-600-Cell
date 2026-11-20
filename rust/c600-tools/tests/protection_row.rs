//! KN2.4's `protection` row served to a library entry by the host bridge (KN5; LP2 draft). CT-LB2-02 (part: a host row
//! serves KN2.4's `protection` to a library entry; the captured locks and the coverage report run whole in
//! `tests/ct_lb2_02.rs`). An entry's snapshot read with an argument, `read_snapshot<T>("protection", check)`, runs on
//! `c600-run` through `Boundary` and [`Bridge`] onto a command layer and judges a captured lock against a recipe at the
//! layer's head (`c600_tools::kernel_rows`). The refusal with Jumble on is not run here: a layer in Jumble on needs a
//! pinned twist menu, which `c600-jumble` builds, and this crate's tests do not depend on it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row, TYPE_MISMATCH};
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::{codes, Kernel, ProtectedSet, SLOTS};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{
    protection_tariff, protection_type, GENERATORS, MAX_PRIMITIVES, PROTECTION, PROTECTION_LEVEL,
};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const MODULE: &str = "judge";

/// A stand-in for a library entry over the row, with variants that declare other types, pass another argument or read
/// a row the bridge does not serve, and one that reads the row at a lock it captured itself.
const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule judge requires [];\n\n\
    export fn verdict(check: record { lock: record { kind: Text, labels: List<Nat>, slots: List<Nat> }, mode: Text, \
    recipe: Word }) -> record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    primitive_count: Nat, refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    primitive_count: Nat, refused_prefix: Int }>(\"protection\", check);\n}\n\n\
    export fn within(check: record { limit: Int, lock: record { kind: Text, labels: List<Int>, slots: List<Int> }, \
    mode: Text, recipe: Word }) -> record { refused_prefix: Int, primitive_count: Int, level: Text, \
    expansion_sha256: Text, conflicts: List<Int>, checked: Int } effects [snapshot] {\n    \
    return read_snapshot<record { refused_prefix: Int, primitive_count: Int, level: Text, expansion_sha256: Text, \
    conflicts: List<Int>, checked: Int }>(\"protection\", check);\n}\n\n\
    export fn short(check: record { lock: record { kind: Text, labels: List<Nat>, slots: List<Nat> }, mode: Text, \
    recipe: Word }) -> record { checked: Nat } effects [snapshot] {\n    \
    return read_snapshot<record { checked: Nat }>(\"protection\", check);\n}\n\n\
    export fn no_level(check: record { lock: record { kind: Text, labels: List<Nat>, slots: List<Nat> }, mode: Text, \
    recipe: Word }) -> record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, primitive_count: Nat, \
    refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, primitive_count: Nat, \
    refused_prefix: Int }>(\"protection\", check);\n}\n\n\
    export fn nat_prefix(check: record { lock: record { kind: Text, labels: List<Nat>, slots: List<Nat> }, mode: Text, \
    recipe: Word }) -> record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    primitive_count: Nat, refused_prefix: Nat } effects [snapshot] {\n    \
    return read_snapshot<record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    primitive_count: Nat, refused_prefix: Nat }>(\"protection\", check);\n}\n\n\
    export fn of_word(w: Word) -> record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    primitive_count: Nat, refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    primitive_count: Nat, refused_prefix: Int }>(\"protection\", w);\n}\n\n\
    export fn colours(w: Word) -> record { checked: Nat } effects [snapshot] {\n    \
    return read_snapshot<record { checked: Nat }>(\"colours\", w);\n}\n\n\
    export fn capture_and_check(slots: List<Nat>, recipe: Word) -> record { checked: Nat, conflicts: List<Nat>, \
    expansion_sha256: Text, level: Text, primitive_count: Nat, refused_prefix: Int } effects [snapshot] {\n    \
    let lock = record { kind = \"position\", labels = read_snapshot<List<Nat>>(\"labels\", slots), slots = slots };\n    \
    return read_snapshot<record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    primitive_count: Nat, refused_prefix: Int }>(\"protection\", record { lock = lock, mode = \"strict\", \
    recipe = recipe });\n}\n";

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

fn loaded() -> Loaded {
    load(&lower_source(SOURCE).expect("the stand-in lowers")).expect("the stand-in loads")
}

fn layer() -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(
        kernel(),
        Table::h08(),
        Secret::fixed([5; 32]),
        Default::default(),
        "judge",
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
        module: sha256(SOURCE.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    Context { run: "judge".into(), base, producer, reads_state: true, inputs: vec![named_word_block()], pins: pins() }
}

/// Runs `entry` on `arg` through a boundary and a bridge onto `layer`; a read sends no invocation.
fn run(layer: &mut CommandLayer, entry: &str, arg: Value) -> Run {
    let context = context(layer, entry);
    let mut bridge = Bridge::new(layer, "judge");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(LIMITS)).expect("the run starts");
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    run
}

/// The kernel cost of a read of `entry` on `arg` that stops with `expected`.
fn refused(layer: &mut CommandLayer, entry: &str, arg: &Value, expected: &str) -> u128 {
    let read = run(layer, entry, arg.clone());
    let summary = &read.summary;
    assert_eq!((summary.outcome, summary.code), (Outcome::Error, Some(expected)), "{entry}: {:?}", summary.stop);
    summary.used.kernel_cost
}

fn nats(items: &[i64]) -> Value {
    Value::list(items.iter().map(|&item| Value::int(item)).collect())
}

fn lock(slots: &[i64], labels: &[i64]) -> Value {
    Value::record(vec![("kind", Value::text("position")), ("labels", nats(labels)), ("slots", nats(slots))])
}

/// A `protection` argument: the lock, the mode and the word, with a limit when one is given.
fn check(lock: Value, mode: &str, word: &[i32], limit: Option<i64>) -> Value {
    let mut fields = vec![("lock", lock), ("mode", Value::text(mode)), ("recipe", Value::word(word))];
    if let Some(limit) = limit {
        fields.insert(0, ("limit", Value::int(limit)));
    }
    Value::record(fields)
}

/// Whether an answer is a record whose `level` is [`PROTECTION_LEVEL`], `labels`.
fn at_labels(answer: &Value) -> bool {
    let Value::Record(fields) = answer else { return false };
    let labels = Value::text(PROTECTION_LEVEL);
    fields.fields().iter().any(|(name, value)| name.as_str() == "level" && equal(value, &labels))
}

/// The row answers the fields an entry declares, in its order, whole-number fields declared `Nat` or `Int`: Net with
/// one judged state, Strict with the prefixes it judged, the input's digest and length, the conflicts and the level,
/// `labels`. A read
/// changes nothing.
#[test]
fn a_library_entry_reads_a_verdict() {
    let mut layer = layer();
    let before = (layer.revision().clone(), layer.store().log.len());
    let slots = [0, 1, 2];
    let held = check(lock(&slots, &slots), "net", &[1, -1], None);
    let read = run(&mut layer, "verdict", held);
    let answer = read.value.as_ref().expect("an answer");
    let Value::Record(fields) = answer else { panic!("a record") };
    let names: Vec<&str> = fields.fields().iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["checked", "conflicts", "expansion_sha256", "level", "primitive_count", "refused_prefix"]);
    assert!(at_labels(answer), "the row judges at labels");

    let within = check(lock(&slots, &slots), "strict", &[1, -1], Some(1));
    let read = run(&mut layer, "within", within);
    let Some(Value::Record(fields)) = read.value.as_ref() else { panic!("a record: {:?}", read.summary.stop) };
    let names: Vec<&str> = fields.fields().iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["refused_prefix", "primitive_count", "level", "expansion_sha256", "conflicts", "checked"]);
    assert!(read.value.as_ref().is_some_and(at_labels), "the row judges at labels");
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's refusals (LP2 draft, recorded in `c600_tools::kernel_rows`). A declared type the row does not answer (a
/// field missing, `level` among them, or `refused_prefix` declared `Nat`) is `LG2:type.mismatch`, and an argument of another form (a word; a mode other than `net` and `strict`; a limit with
/// Net or a negative one; a lock whose lists differ in length or that has no kind) is `KN5:input.argument_form`: each
/// before the row runs, so the read costs what a read of a row the bridge does not serve costs with the same argument.
/// A slot or a label outside 0 to 259,799 is `KN2:input.named_object_invalid`, and a recipe the kernel refuses stops
/// with the kernel's code, each charged the row's tariff. A refused read changes nothing.
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer();
    let before = (layer.revision().clone(), layer.store().log.len());
    let held = || lock(&[0, 1], &[0, 1]);
    let no_kind = Value::record(vec![("labels", nats(&[0])), ("slots", nats(&[0]))]);
    for (entry, arg, code) in [
        ("short", check(held(), "net", &[1], None), TYPE_MISMATCH),
        ("no_level", check(held(), "net", &[1], None), TYPE_MISMATCH),
        ("nat_prefix", check(held(), "net", &[1], None), TYPE_MISMATCH),
        ("of_word", Value::word(&[1]), code::ARGUMENT_FORM),
        ("verdict", check(held(), "both", &[1], None), code::ARGUMENT_FORM),
        ("within", check(held(), "net", &[1], Some(3)), code::ARGUMENT_FORM),
        ("within", check(held(), "strict", &[1], Some(-1)), code::ARGUMENT_FORM),
        ("verdict", check(lock(&[0, 1], &[0]), "net", &[1], None), code::ARGUMENT_FORM),
        ("verdict", check(no_kind, "net", &[1], None), code::ARGUMENT_FORM),
    ] {
        let cost = refused(&mut layer, entry, &arg, code);
        assert_eq!(cost, refused(&mut layer, "colours", &arg, NOT_GRANTED), "{entry}: refused before the row runs");
    }
    let charged = |primitives: usize, strict: bool| protection_tariff(2, primitives, strict).kernel_cost;
    for (arg, expected, cost) in [
        (check(lock(&[SLOTS as i64, 1], &[0, 1]), "net", &[1], None), codes::NAMED_OBJECT_INVALID, charged(1, false)),
        (check(lock(&[0, 1], &[0, SLOTS as i64]), "strict", &[1], None), codes::NAMED_OBJECT_INVALID, charged(1, true)),
    ] {
        let used = refused(&mut layer, "verdict", &arg, expected);
        assert_eq!(used, refused(&mut layer, "colours", &arg, NOT_GRANTED) + cost, "{arg:?}: charged");
    }
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's draft tariff covers its answer: a Strict answer asked of the bridge directly fits the result bound of its
/// tariff, and the tariff counts the pairs, one unit per primitive, one pass over the slots and one more for each
/// generator a Strict check may use. The bound covers the largest answer: every pair in conflict and counts of
/// [`MAX_PRIMITIVES`]. A read sends no invocation.
#[test]
fn the_tariff_covers_the_largest_answer() {
    let mut layer = layer();
    let mut bridge = Bridge::new(&mut layer, "direct");
    let slots: Vec<i64> = (0..64).collect();
    let mut arg = Vec::new();
    encode(&check(lock(&slots, &slots), "strict", &[1, 2, 1], None), &mut arg).expect("a canonical argument");
    let ty = protection_type();
    let request = Request::SnapshotWith { query: PROTECTION, arg: &arg, ty: &ty };
    let tariff = bridge.tariff(&request).unwrap();
    assert_eq!(tariff, protection_tariff(64, 3, true));
    assert_eq!(tariff.kernel_cost, (64 + 3 + SLOTS * 4) as u128);
    let answer = bridge.perform(&request).unwrap();
    assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");

    let largest = protection_tariff(SLOTS, MAX_PRIMITIVES, true);
    assert_eq!(largest.kernel_cost, (SLOTS + MAX_PRIMITIVES + SLOTS * (GENERATORS + 1)) as u128);
    let most = Value::int(MAX_PRIMITIVES as u64 + 1);
    let answer = Value::record(vec![
        ("checked", most.clone()),
        ("conflicts", Value::list(vec![Value::int((SLOTS - 1) as u64); SLOTS])),
        ("expansion_sha256", Value::text(&"f".repeat(64))),
        ("level", Value::text(PROTECTION_LEVEL)),
        ("primitive_count", most.clone()),
        ("refused_prefix", most),
    ]);
    assert!(answer.size() <= largest.result_bound);
}

/// The built-in column admits an entry that reads the row only when its class lists `Row::Read("protection")`, and
/// the entry reads state and prepares no preview (LG3.13). An entry that captures a lock and judges it in one run reads
/// the row at a lock it built from a read, not at one the solver named, and the column refuses it (LB3.5): a lock is
/// captured in one run and named to the check in the next.
#[test]
fn a_reading_entry_needs_the_read_row_and_a_named_lock() {
    let program = lower_source(SOURCE).unwrap();
    let read: BTreeSet<Row> = [Row::Read(PROTECTION.into())].into();
    let admitted = admit_built_in(&program, MODULE, "verdict", &read, &Grant::none(), &Policy::interim()).unwrap();
    assert!(admitted.is_admitted(), "{:?}", admitted.diagnostics);
    assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);
    let none = BTreeSet::new();
    assert!(!admit_built_in(&program, MODULE, "verdict", &none, &Grant::none(), &Policy::interim())
        .unwrap()
        .is_admitted());

    let both: BTreeSet<Row> = [Row::Read(PROTECTION.into()), Row::Read("labels".into())].into();
    let refused = admit_built_in(&program, MODULE, "capture_and_check", &both, &Grant::none(), &Policy::interim());
    let refused = refused.unwrap();
    assert!(!refused.is_admitted());
    assert!(refused.diagnostics.iter().any(|d| d.code == NOT_GRANTED), "{:?}", refused.diagnostics);
}
