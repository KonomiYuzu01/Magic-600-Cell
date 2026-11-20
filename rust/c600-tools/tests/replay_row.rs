//! KN2.4's `replay` row for 600-cell-Full served to a library entry by the host bridge (KN5; LP2 draft). An entry's
//! snapshot read with an argument, `read_snapshot<T>("replay", recipe)`, runs on `c600-run` through `Boundary` and
//! [`Bridge`] onto a command layer and replays the recipe from the layer's head, prefix by prefix, under the session's
//! protected orbits (`c600_tools::kernel_rows`). LB2's named macro analysis reads it (`c600-library`'s `macros`, run
//! whole in `tests/macros.rs`). The refusal with Jumble on is not run here: a layer in Jumble on needs a pinned twist
//! menu, which `c600-jumble` builds, and this crate's tests do not depend on it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row, TYPE_MISMATCH};
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::{codes, Derivation, Kernel, ProtectedSet, Snapshot, ORBITS, SLOTS};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::encode::encode;
use c600_run::interp::{load, Host, Loaded, Options, Request, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{replay_tariff, replay_type, MAX_PRIMITIVES, PROTECTION_LEVEL, REPLAY};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const MODULE: &str = "replayer";

/// A stand-in for a library entry over the row, with variants that declare other types, pass another argument or read
/// a row the bridge does not serve.
const SOURCE: &str = "edition \"6cs-draft-1\";\nmodule replayer requires [];\n\n\
    export fn replay_of(w: Word) -> record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    net_sha256: Text, primitive_count: Nat, protected: List<Nat>, refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, net_sha256: Text, \
    primitive_count: Nat, protected: List<Nat>, refused_prefix: Int }>(\"replay\", w);\n}\n\n\
    export fn replay_steps(steps: List<record { kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }>) -> \
    record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, net_sha256: Text, primitive_count: Nat, \
    protected: List<Nat>, refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, net_sha256: Text, \
    primitive_count: Nat, protected: List<Nat>, refused_prefix: Int }>(\"replay\", steps);\n}\n\n\
    export fn reordered(w: List<Int>) -> record { refused_prefix: Int, protected: List<Int>, primitive_count: Int, \
    net_sha256: Text, level: Text, expansion_sha256: Text, conflicts: List<Int> } effects [snapshot] {\n    \
    return read_snapshot<record { refused_prefix: Int, protected: List<Int>, primitive_count: Int, \
    net_sha256: Text, level: Text, expansion_sha256: Text, conflicts: List<Int> }>(\"replay\", w);\n}\n\n\
    export fn short(w: Word) -> record { refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { refused_prefix: Int }>(\"replay\", w);\n}\n\n\
    export fn nat_prefix(w: Word) -> record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    net_sha256: Text, primitive_count: Nat, protected: List<Nat>, refused_prefix: Nat } effects [snapshot] {\n    \
    return read_snapshot<record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, net_sha256: Text, \
    primitive_count: Nat, protected: List<Nat>, refused_prefix: Nat }>(\"replay\", w);\n}\n\n\
    export fn one_letter(m: Int) -> record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, \
    net_sha256: Text, primitive_count: Nat, protected: List<Nat>, refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, net_sha256: Text, \
    primitive_count: Nat, protected: List<Nat>, refused_prefix: Int }>(\"replay\", m);\n}\n\n\
    export fn colours(w: Word) -> record { refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { refused_prefix: Int }>(\"colours\", w);\n}\n\n\
    export fn colours_of(m: Int) -> record { refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { refused_prefix: Int }>(\"colours\", m);\n}\n\n\
    export fn colours_of_steps(steps: List<record { kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }>) -> \
    record { refused_prefix: Int } effects [snapshot] {\n    \
    return read_snapshot<record { refused_prefix: Int }>(\"colours\", steps);\n}\n";

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

/// A layer whose session starts at `start` with `orbits` protected.
fn layer_at(start: Snapshot, orbits: &[i64]) -> CommandLayer {
    let protected = kernel().protected_set(orbits).expect("orbits 0 to 34");
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), "replayer", start, protected)
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
    Context { run: "replay".into(), base, producer, reads_state: true, inputs: vec![named_word_block()], pins: pins() }
}

/// Runs `entry` on `arg` through a boundary and a bridge onto `layer`; a read sends no invocation.
fn run(layer: &mut CommandLayer, entry: &str, arg: Value) -> Run {
    let context = context(layer, entry);
    let mut bridge = Bridge::new(layer, "replay");
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

fn ints(items: &[i64]) -> Value {
    Value::list(items.iter().map(|&item| Value::int(item)).collect())
}

/// `Digest<literal word>`: the SHA-256 of the letters as little-endian 32-bit integers.
fn literal_digest(letters: &[i32]) -> String {
    sha256_hex(&letters.iter().flat_map(|m| m.to_le_bytes()).collect::<Vec<u8>>())
}

/// The moving orbits, 0 to 34, that a word's net map moves.
fn moved_orbits(letters: &[i32]) -> Vec<i64> {
    let kernel = kernel();
    let effect = kernel.effect(&kernel.word(&wide(letters)).unwrap()).unwrap();
    effect.support.iter().map(|row| i64::from(row.orbit)).filter(|&orbit| orbit >= 0).collect()
}

/// The answer the row owes for `letters` under `orbits`, in name order, from the kernel's effect of the whole word and
/// of its first letter: on the shipped model every generator moves a sticker of every orbit, so with an orbit protected
/// the first letter is the first refused prefix and every protected orbit is in conflict.
fn owed(letters: &[i32], orbits: &[i64]) -> Value {
    let kernel = kernel();
    let effect = kernel.effect(&kernel.word(&wide(letters)).unwrap()).unwrap();
    let first = moved_orbits(&letters[..1]);
    assert!(orbits.iter().all(|orbit| first.contains(orbit)), "{letters:?}: the first letter moves every orbit");
    let refused_prefix = if orbits.is_empty() { -1 } else { 1 };
    Value::record(vec![
        ("conflicts", ints(orbits)),
        ("expansion_sha256", Value::text(&literal_digest(letters))),
        ("level", Value::text(PROTECTION_LEVEL)),
        ("net_sha256", Value::text(&effect.net_sha256)),
        ("primitive_count", Value::int(letters.len() as u64)),
        ("protected", ints(orbits)),
        ("refused_prefix", Value::int(refused_prefix)),
    ])
}

/// The row replays a word, a list of integers or a list of step records from the head under the session's protected
/// orbits and answers the fields an entry declares, in its order: the protected orbits; the first refused prefix, -1
/// with none protected and the first letter with any, and the protected orbits in conflict there; the whole word's net
/// map digest, as the `effect` row gives it, identity words included; and the input's digest and length. It agrees with
/// the kernel's Strict derivation at the head, at solved as at a scrambled head. A read changes nothing.
#[test]
fn a_library_entry_replays_a_recipe_under_the_sessions_protected_orbits() {
    let kernel = kernel();
    let words: [&[i32]; 3] = [&[1, -1], &[5, 7, -5, -7], &SCRAMBLE];
    for orbits in [vec![], vec![7], vec![3, 7, 20]] {
        for start in [kernel.solved(), scrambled()] {
            let mut layer = layer_at(start, &orbits);
            let before = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
            for letters in words {
                let expected = owed(letters, &orbits);
                let read = completed(&run(&mut layer, "replay_of", Value::word(letters))).clone();
                assert!(equal(&read, &expected), "{letters:?} under {orbits:?}: {read:?}");

                let (first, rest) = letters.split_at(1);
                let steps = Value::list(vec![step("word", first, 0, 0, 0), step("word", rest, 0, 0, 0)]);
                assert!(equal(completed(&run(&mut layer, "replay_steps", steps)), &expected), "{letters:?} as steps");

                let recipe = kernel.word(&wide(letters)).unwrap();
                let derivation = kernel.derive_strict(layer.snapshot(), &recipe, layer.protected()).unwrap();
                let prefix = match &derivation {
                    Derivation::Derived(derived) => derived.protection.first_refused_prefix,
                    Derivation::Refused(refused) => refused.first_refused_prefix,
                };
                assert_eq!(prefix.map_or(-1, |prefix| prefix as i64), if orbits.is_empty() { -1 } else { 1 });
            }
            let after = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
            assert_eq!(after, before, "a read changes nothing");
        }
    }
}

/// The answer's fields are in the declared order, and whole-number fields may be declared `Int`.
#[test]
fn the_row_answers_in_the_declared_order() {
    let mut layer = layer_at(scrambled(), &[7]);
    let read = completed(&run(&mut layer, "reordered", ints(&wide(&SCRAMBLE)))).clone();
    let Value::Record(fields) = &read else { panic!("a record") };
    let names: Vec<&str> = fields.fields().iter().map(|(name, _)| name.as_str()).collect();
    let order =
        ["refused_prefix", "protected", "primitive_count", "net_sha256", "level", "expansion_sha256", "conflicts"];
    assert_eq!(names, order);
    let Value::Record(expected) = owed(&SCRAMBLE, &[7]) else { panic!("a record") };
    for name in order {
        assert!(equal(fields.get(name).unwrap(), expected.get(name).unwrap()), "{name}");
    }
}

/// The row's refusals (LP2 draft, recorded in `c600_tools::kernel_rows`). A declared type the row does not answer (a
/// field missing, or `refused_prefix` declared `Nat`) is `LG2:type.mismatch`, and an argument in neither recipe form (a
/// single integer, a word step with a node) is `KN5:input.argument_form`: each before the row runs, so the read costs
/// what a read of a row the bridge does not serve costs with the same argument (`LG3:capability.not_granted`). A recipe
/// the kernel refuses (no step, a star of orbit 35) stops with the kernel's code, and the read is charged the row's
/// tariff for no primitive. A refused read changes nothing.
#[test]
fn the_row_refuses_what_it_does_not_answer() {
    let mut layer = layer_at(scrambled(), &[7]);
    let before = (layer.revision().clone(), layer.store().log.len());
    for (entry, arg, code, other) in [
        ("short", Value::word(&[1]), TYPE_MISMATCH, "colours"),
        ("nat_prefix", Value::word(&[1]), TYPE_MISMATCH, "colours"),
        ("one_letter", Value::int(1), code::ARGUMENT_FORM, "colours_of"),
        ("replay_steps", Value::list(vec![step("word", &[1], 1, 0, 0)]), code::ARGUMENT_FORM, "colours_of_steps"),
    ] {
        let cost = refused(&mut layer, entry, &arg, code);
        assert_eq!(cost, refused(&mut layer, other, &arg, NOT_GRANTED), "{entry}: refused before the row runs");
    }
    let tariff = replay_tariff(0).kernel_cost;
    assert_eq!(tariff, 2 * SLOTS as u128, "a refused recipe is charged for no primitive");
    for arg in [Value::list(Vec::new()), Value::list(vec![step("star", &[], 0, 35, 1)])] {
        let cost = refused(&mut layer, "replay_steps", &arg, codes::RECIPE_SHAPE);
        assert_eq!(cost, refused(&mut layer, "colours_of_steps", &arg, NOT_GRANTED) + tariff, "{arg:?}: charged");
    }
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
}

/// The row's draft tariff covers its answer: an answer asked of the bridge directly fits the result bound of its
/// tariff, and the tariff counts two passes over the slots and two units per primitive. The bound covers the largest
/// answer: every orbit protected and in conflict, and [`MAX_PRIMITIVES`]. A read sends no invocation.
#[test]
fn the_tariff_covers_the_largest_answer() {
    let kernel = kernel();
    let mut layer = layer_at(kernel.solved(), &[7]);
    let mut bridge = Bridge::new(&mut layer, "direct");
    let mut arg = Vec::new();
    encode(&Value::word(&[1, 2, 1]), &mut arg).expect("a canonical word");
    let ty = replay_type();
    let request = Request::SnapshotWith { query: REPLAY, arg: &arg, ty: &ty };
    let tariff = bridge.tariff(&request).unwrap();
    assert_eq!(tariff, replay_tariff(3));
    assert_eq!(tariff.kernel_cost, (2 * SLOTS + 6) as u128);
    let answer = bridge.perform(&request).unwrap();
    assert!(answer.size() <= tariff.result_bound, "{} > {}", answer.size(), tariff.result_bound);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");

    let every: Vec<i64> = (0..ORBITS as i64).collect();
    let most = Value::int(MAX_PRIMITIVES as u64);
    let largest = Value::record(vec![
        ("conflicts", ints(&every)),
        ("expansion_sha256", Value::text(&"f".repeat(64))),
        ("level", Value::text(PROTECTION_LEVEL)),
        ("net_sha256", Value::text(&"f".repeat(64))),
        ("primitive_count", most.clone()),
        ("protected", ints(&every)),
        ("refused_prefix", most),
    ]);
    assert!(largest.size() <= replay_tariff(MAX_PRIMITIVES).result_bound);
}

/// The built-in column admits an entry that reads the row only when its class lists `Row::Read("replay")`, and the
/// entry reads state and prepares no preview (LG3.13).
#[test]
fn a_reading_entry_needs_the_read_row_in_its_class() {
    let program = lower_source(SOURCE).unwrap();
    let read: BTreeSet<Row> = [Row::Read(REPLAY.into())].into();
    let admitted = admit_built_in(&program, MODULE, "replay_of", &read, &Grant::none(), &Policy::interim()).unwrap();
    assert!(admitted.is_admitted(), "{:?}", admitted.diagnostics);
    assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);
    let none = BTreeSet::new();
    let refused = admit_built_in(&program, MODULE, "replay_of", &none, &Grant::none(), &Policy::interim()).unwrap();
    assert!(!refused.is_admitted());
}
