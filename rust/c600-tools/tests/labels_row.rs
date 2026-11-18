//! KN2.4's `labels` row served to a library entry by the host bridge (KN5; LP2 draft). CT-LB2-04 (part: a host row
//! serves KN2.4's `labels` to a library entry; LB2's exact-completion entry runs whole in
//! `tests/exact_completion.rs`). An entry's snapshot read with an argument, `read_snapshot<T>("labels", slots)`, runs
//! on `c600-run` through `Boundary` and [`Bridge`] onto a command layer and reads, for each named slot, the home slot
//! of the sticker it holds at the head (`c600_tools::kernel_rows`). The refusal with Jumble on is not run here: a
//! layer in Jumble on needs a pinned twist menu, which `c600-jumble` builds, and this crate's tests do not depend on
//! it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row, TYPE_MISMATCH};
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, SLOTS};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{labels_tariff, labels_type, LABELS};
use c600_tools::tool::{checker, class_rows, contract, lower_source, manifest, named_word_block, pins, Manifest};

const MODULE: &str = "reader";

/// A stand-in for a library entry over the row, with variants that declare other types, pass another argument or read
/// a row the bridge does not serve.
const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule reader requires [];\n\n\
    export fn labels_of(slots: List<Nat>) -> List<Nat> effects [snapshot] {\n    \
    return read_snapshot<List<Nat>>(\"labels\", slots);\n}\n\n\
    export fn in_set(slots: Set<Nat>) -> List<Nat> effects [snapshot] {\n    \
    return read_snapshot<List<Nat>>(\"labels\", slots);\n}\n\n\
    export fn any(slots: List<Int>) -> List<Int> effects [snapshot] {\n    \
    return read_snapshot<List<Int>>(\"labels\", slots);\n}\n\n\
    export fn as_set(slots: List<Int>) -> Set<Nat> effects [snapshot] {\n    \
    return read_snapshot<Set<Nat>>(\"labels\", slots);\n}\n\n\
    export fn as_records(slots: List<Int>) -> List<record { label: Nat }> effects [snapshot] {\n    \
    return read_snapshot<List<record { label: Nat }>>(\"labels\", slots);\n}\n\n\
    export fn as_nat(slots: List<Int>) -> Nat effects [snapshot] {\n    \
    return read_snapshot<Nat>(\"labels\", slots);\n}\n\n\
    export fn one_slot(slot: Int) -> List<Nat> effects [snapshot] {\n    \
    return read_snapshot<List<Nat>>(\"labels\", slot);\n}\n\n\
    export fn colours(slots: List<Int>) -> List<Nat> effects [snapshot] {\n    \
    return read_snapshot<List<Nat>>(\"colours\", slots);\n}\n\n\
    export fn colours_of(slot: Int) -> List<Nat> effects [snapshot] {\n    \
    return read_snapshot<List<Nat>>(\"colours\", slot);\n}\n";

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

fn loaded() -> Loaded {
    load(&lower_source(SOURCE).expect("the reader lowers")).expect("the reader loads")
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
    loaded().run(MODULE, entry, vec![arg], &mut boundary, &Options::new(LIMITS)).expect("the run starts")
}

fn slots(slots: &[i64]) -> Value {
    Value::list(slots.iter().map(|&slot| Value::int(slot)).collect())
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

/// In a scrambled session an entry reads, for the slots it names, each slot's label as the kernel's snapshot holds it
/// at the head: the home slot of the sticker the slot holds. A slot that holds its own sticker answers itself and a
/// slot that holds another sticker answers that sticker's home slot, so the answer names stickers by identity. The
/// answer follows the asked order, repeats included; labels may be declared `Int`, and a set names its slots in
/// increasing order. No slot answers no label. A read changes nothing.
#[test]
fn a_library_entry_reads_the_labels_of_named_slots() {
    let start = scrambled();
    let labels = start.labels();
    let own = (0..SLOTS).find(|&slot| labels[slot] as usize == slot).expect("a slot that holds its own sticker");
    let other = (0..SLOTS).find(|&slot| labels[slot] as usize != slot).expect("a slot that holds another sticker");
    let asked = [other, own, SLOTS - 1, 0, other].map(|slot| slot as i64);
    let expected = |asked: &[i64]| Value::list(asked.iter().map(|&slot| Value::int(labels[slot as usize])).collect());

    let mut layer = layer_at(start.clone());
    let before = (layer.revision().clone(), layer.store().log.len());
    let read = run(&mut layer, "labels_of", slots(&asked));
    let answer = completed(&read);
    assert!(equal(answer, &expected(&asked)), "{answer:?}");
    assert!(!equal(answer, &slots(&asked)), "a slot that holds another sticker does not answer itself");
    assert!(equal(completed(&run(&mut layer, "any", slots(&asked))), &expected(&asked)));

    let set = Value::set(asked.iter().map(|&slot| Value::int(slot)).collect());
    let mut increasing = asked.to_vec();
    increasing.sort_unstable();
    increasing.dedup();
    assert!(equal(completed(&run(&mut layer, "in_set", set)), &expected(&increasing)));
    assert!(equal(completed(&run(&mut layer, "labels_of", slots(&[]))), &Value::list(Vec::new())));
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's refusals (LP2 draft, recorded in `c600_tools::kernel_rows`). A declared type the row does not answer (a
/// set, a list of records or a single number) is `LG2:type.mismatch`, and an argument that is not a list of integers
/// is `KN5:input.argument_form`: each before the row runs, so the read costs what a read of a row the bridge does not
/// serve costs with the same argument (`LG3:capability.not_granted`): its step and the argument's encoding. A slot
/// outside 0 to 259,799 is `KN2:input.named_object_invalid` once the read is charged its tariff. A refused read changes
/// nothing.
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer_at(scrambled());
    let before = (layer.revision().clone(), layer.store().log.len());
    for (entry, arg, code, other) in [
        ("as_set", slots(&[0]), TYPE_MISMATCH, "colours"),
        ("as_records", slots(&[0]), TYPE_MISMATCH, "colours"),
        ("as_nat", slots(&[0]), TYPE_MISMATCH, "colours"),
        ("one_slot", Value::int(0), code::ARGUMENT_FORM, "colours_of"),
    ] {
        let cost = refused(&mut layer, entry, &arg, code);
        assert_eq!(cost, refused(&mut layer, other, &arg, NOT_GRANTED), "{entry}: refused before the row runs");
    }
    let tariff = labels_tariff(2).kernel_cost;
    for bad in [-1, SLOTS as i64, 1 << 62] {
        let arg = slots(&[0, bad]);
        let cost = refused(&mut layer, "any", &arg, code::NAMED_OBJECT_INVALID);
        assert_eq!(cost, refused(&mut layer, "colours", &arg, NOT_GRANTED) + tariff, "{bad}: the read is charged");
    }
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's draft tariff covers its answer: the answer for every slot, asked of the bridge directly, fits the result
/// bound of its tariff, and the tariff counts one label per named slot. A read sends no invocation.
#[test]
fn the_tariff_covers_the_largest_answer() {
    let mut layer = layer_at(scrambled());
    let expected = layer.snapshot().labels().to_vec();
    let mut bridge = Bridge::new(&mut layer, "direct");
    let all: Vec<i64> = (0..SLOTS as i64).collect();
    let mut arg = Vec::new();
    encode(&slots(&all), &mut arg).expect("a canonical list");
    let ty = labels_type();
    let request = Request::SnapshotWith { query: LABELS, arg: &arg, ty: &ty };
    let tariff = bridge.tariff(&request).unwrap();
    assert_eq!(tariff, labels_tariff(SLOTS));
    assert_eq!(tariff.kernel_cost, SLOTS as u128);
    let answer = bridge.perform(&request).unwrap();
    assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    let labels: Vec<i64> = expected.iter().map(|&label| i64::from(label)).collect();
    assert!(equal(&answer, &slots(&labels)));
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
}

/// The built-in column admits an entry that reads the row only when its class lists `Row::Read("labels")`, and the
/// entry reads state and prepares no preview (LG3.13). A class built from a manifest's rows ([`class_rows`]) lists
/// preview rows only, so it does not admit the entry.
#[test]
fn a_reading_entry_needs_the_read_row_in_its_class() {
    let program = lower_source(SOURCE).unwrap();
    let read: BTreeSet<Row> = [Row::Read(LABELS.into())].into();
    let admitted = admit_built_in(&program, MODULE, "labels_of", &read, &Grant::none(), &Policy::interim()).unwrap();
    assert!(admitted.is_admitted(), "{:?}", admitted.diagnostics);
    assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);

    let previews = class_rows(&Manifest { rows: &[], ..manifest() });
    let refused = admit_built_in(&program, MODULE, "labels_of", &previews, &Grant::none(), &Policy::interim()).unwrap();
    assert!(!refused.is_admitted());
}
