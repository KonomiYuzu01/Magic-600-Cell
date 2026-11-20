//! LB2's row "Named macro analysis and comparison" on 600-cell-Full. The entries are library code, `c600-library`'s
//! module `macros`: `macro_report` reports one named macro through KN2.4's `effect` and `replay` rows, and
//! `macro_comparison` compares two through `effect`. They run on `c600-run` through `Boundary` and the tool's host
//! bridge onto a command layer, in the solving profile, author `built_in`; the bridge serves both rows
//! (`c600_tools::kernel_rows`). The expected answers come from the kernel's own effect, at solved and at a scrambled
//! head, with no orbit or with orbits protected in the session. A pass is source and fixture evidence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row};
use c600_command::{CommandLayer, Secret, Table};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::{Derivation, Effect, Kernel, ProtectedSet, Snapshot};
use c600_library::{MACROS_EXPORTS, MACROS_MODULE, MACROS_ROWS, MACROS_SOURCE};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Loaded, Options, Run};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{EFFECT, REPLAY};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const LIMITS: Amounts = Amounts { steps: 1 << 24, kernel_cost: 1 << 40, memory: 1 << 34, output: 1 << 28 };

/// A fixed scramble from solved.
const SCRAMBLE: [i32; 20] =
    [17, -305, 911, 4, -1200, 600, 33, -88, 1021, -7, 450, 2, -999, 316, 75, -640, 1199, -12, 808, 23];

/// The caller's named macros: an inverse pair, with identity net effect; a commutator; and the scramble.
const MACROS: [(&str, &[i32]); 3] =
    [("there and back", &[1, -1]), ("commutator", &[5, 7, -5, -7]), ("scramble", &SCRAMBLE)];

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
    LOADED.get_or_init(|| {
        load(&lower_source(MACROS_SOURCE).expect("the library module lowers")).expect("the library module loads")
    })
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
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), "macros", start, protected)
}

/// The context of one run of `entry` in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer, entry: &str, inputs: usize) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(MACROS_SOURCE.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    let inputs = vec![named_word_block(); inputs];
    Context { run: "macros".into(), base, producer, reads_state: true, inputs, pins: pins() }
}

/// Runs `entry` on `args` through a boundary and a bridge onto `layer`; a read sends no invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, args: Vec<Value>) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    let context = context(layer, entry, args.len());
    let mut bridge = Bridge::new(layer, "macros");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(MACROS_MODULE, entry, args, &mut boundary, &Options::new(LIMITS));
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    let after = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    assert_eq!(after, before, "a read changes nothing");
    run.expect("the run starts")
}

/// The answer of a completed run of `entry`.
fn answer(layer: &mut CommandLayer, entry: &str, args: Vec<Value>) -> Value {
    let run = run(layer, entry, args);
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{entry}: {:?}", run.summary.stop);
    run.value.expect("an answer")
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

fn text(value: &Value) -> &str {
    let Value::Text(t) = value else { panic!("a text: {value:?}") };
    t.as_str()
}

fn ints(value: &Value) -> Vec<i64> {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items().iter().map(int).collect()
}

fn boolean(value: &Value) -> bool {
    let Value::Bool(b) = value else { panic!("a boolean: {value:?}") };
    *b
}

/// A recipe as the entries take it: one word step.
fn steps(word: &[i32]) -> Value {
    Value::list(vec![Value::record(vec![
        ("kind", Value::text("word")),
        ("moves", Value::word(word)),
        ("node", Value::int(0)),
        ("orbit", Value::int(0)),
        ("sign", Value::int(0)),
    ])])
}

/// The arguments of `macro_comparison`: each macro's name and its recipe, in that order.
fn compared(first_name: &str, first: &[i32], second_name: &str, second: &[i32]) -> Vec<Value> {
    vec![Value::text(first_name), steps(first), Value::text(second_name), steps(second)]
}

/// `Digest<literal word>`: the SHA-256 of the letters as little-endian 32-bit integers.
fn literal_digest(letters: &[i32]) -> String {
    sha256_hex(&letters.iter().flat_map(|m| m.to_le_bytes()).collect::<Vec<u8>>())
}

/// The kernel's effect of a word.
fn effect(letters: &[i32]) -> Effect {
    let kernel = kernel();
    kernel.effect(&kernel.word(&wide(letters)).unwrap()).unwrap()
}

/// The orbits, -1 to 34, that a word's net map moves, in increasing order.
fn support_orbits(letters: &[i32]) -> Vec<i64> {
    effect(letters).support.iter().map(|row| i64::from(row.orbit)).collect()
}

/// The support rows of a word's net map as the entries answer them.
fn support(letters: &[i32]) -> Value {
    let row = |row: &c600_kernel::SupportRow| {
        Value::record(vec![
            ("orbit", Value::int(row.orbit)),
            ("pieces", Value::int(row.pieces as u64)),
            ("stickers", Value::int(row.stickers as u64)),
        ])
    };
    Value::list(effect(letters).support.iter().map(row).collect())
}

/// The built-in column admits each entry only when its class lists the rows it reads, `effect` and `replay` for
/// `macro_report` and `effect` for `macro_comparison`, and each reads state and prepares no preview (LG3.13). Each
/// entry reads its rows at the recipes the caller names, never at an argument built from another read (LB3.5).
#[test]
fn the_entries_are_admitted_with_their_rows() {
    assert_eq!(MACROS_ROWS, [EFFECT, REPLAY]);
    let program = lower_source(MACROS_SOURCE).unwrap();
    let admit = |entry: &str, rows: &[&str]| {
        let class: BTreeSet<Row> = rows.iter().map(|row| Row::Read(row.to_string())).collect();
        admit_built_in(&program, MACROS_MODULE, entry, &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    for entry in MACROS_EXPORTS {
        let own: &[&str] = match entry {
            "macro_report" => &[EFFECT, REPLAY],
            _ => &[EFFECT],
        };
        for rows in [own, &MACROS_ROWS[..]] {
            let admitted = admit(entry, rows);
            assert!(admitted.is_admitted(), "{entry}: {:?}", admitted.diagnostics);
            assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews, "{entry}");
        }
        for missing in own {
            let fewer: Vec<&str> = own.iter().copied().filter(|row| row != missing).collect();
            assert!(!admit(entry, &fewer).is_admitted(), "{entry} without {missing}");
        }
    }
}

/// A report answers the caller's name and the macro's whole effect as the kernel gives it: the net map's digest, the
/// moved stickers and pieces and the full support; the expansion's digest and length; and the session's protected
/// orbits at the head. With none protected nothing is in conflict and no prefix is refused. With orbits protected,
/// Net's conflicts are the protected orbits the net map moves, none for the inverse pair, and Strict refuses the first
/// letter, which on the shipped model moves every orbit: the inverse pair passes Net and fails Strict (LB2.3). The
/// report has these fields only, so it ranks and selects nothing, and it is the same at solved and at a scrambled head.
#[test]
fn a_report_answers_a_named_macros_whole_effect_under_the_sessions_protection() {
    for orbits in [vec![], vec![7], vec![3, 7, 20]] {
        for start in [kernel().solved(), scrambled()] {
            let mut layer = layer_at(start, &orbits);
            for (name, word) in MACROS {
                let report = answer(&mut layer, "macro_report", vec![Value::text(name), steps(word)]);
                assert_eq!(
                    names(&report),
                    [
                        "conflicts",
                        "expansion_sha256",
                        "level",
                        "moved_pieces",
                        "moved_stickers",
                        "name",
                        "net_conflicts",
                        "net_sha256",
                        "primitive_count",
                        "protected",
                        "refused_prefix",
                        "support"
                    ]
                );
                let whole = effect(word);
                assert_eq!(text(field(&report, "name")), name);
                assert_eq!(text(field(&report, "net_sha256")), whole.net_sha256);
                assert_eq!(int(field(&report, "moved_stickers")), whole.moved_stickers() as i64);
                assert_eq!(int(field(&report, "moved_pieces")), whole.moved_pieces as i64);
                assert!(equal(field(&report, "support"), &support(word)), "{name}: the full support");
                assert_eq!(text(field(&report, "expansion_sha256")), literal_digest(word));
                assert_eq!(int(field(&report, "primitive_count")), word.len() as i64);
                assert_eq!(text(field(&report, "level")), "labels");
                assert_eq!(ints(field(&report, "protected")), orbits);

                let moved = support_orbits(word);
                let net: Vec<i64> = orbits.iter().copied().filter(|orbit| moved.contains(orbit)).collect();
                assert_eq!(ints(field(&report, "net_conflicts")), net, "{name}: Net's conflicts");
                let first = support_orbits(&word[..1]);
                assert!(orbits.iter().all(|orbit| first.contains(orbit)), "the first letter moves every orbit");
                let (refused, conflicts) = if orbits.is_empty() { (-1, vec![]) } else { (1, orbits.clone()) };
                assert_eq!(int(field(&report, "refused_prefix")), refused, "{name}: Strict's first refused prefix");
                assert_eq!(ints(field(&report, "conflicts")), conflicts, "{name}: Strict's conflicts");
                if name == "there and back" {
                    assert_eq!(int(field(&report, "moved_stickers")), 0, "identity net effect");
                    assert!(ints(field(&report, "net_conflicts")).is_empty(), "the inverse pair passes Net");
                }
            }
        }
    }
}

/// A comparison of two named macros answers the names, the support orbits both net maps move and those only one
/// moves, each in increasing order, and whether the net maps and the literal expansions are equal, from the kernel's
/// effects. A macro compared with itself has all its support in common; two inverse pairs have the same identity net
/// map and different expansions; swapping the two swaps the names and the one-sided orbits and nothing else.
#[test]
fn a_comparison_answers_the_shared_and_one_sided_support() {
    let mut layer = layer_at(scrambled(), &[7]);
    let pairs: [(&str, &[i32], &str, &[i32]); 4] = [
        ("commutator", &[5, 7, -5, -7], "commutator again", &[5, 7, -5, -7]),
        ("there and back", &[1, -1], "and back again", &[2, -2]),
        ("commutator", &[5, 7, -5, -7], "scramble", &SCRAMBLE),
        ("scramble", &SCRAMBLE, "there and back", &[1, -1]),
    ];
    for (first_name, first, second_name, second) in pairs {
        let comparison = answer(&mut layer, "macro_comparison", compared(first_name, first, second_name, second));
        let order = ["both", "first", "only_first", "only_second", "same_expansion", "same_net", "second"];
        assert_eq!(names(&comparison), order);
        let (left, right) = (support_orbits(first), support_orbits(second));
        let both: Vec<i64> = left.iter().copied().filter(|orbit| right.contains(orbit)).collect();
        let only_first: Vec<i64> = left.iter().copied().filter(|orbit| !right.contains(orbit)).collect();
        let only_second: Vec<i64> = right.iter().copied().filter(|orbit| !left.contains(orbit)).collect();
        assert_eq!((text(field(&comparison, "first")), text(field(&comparison, "second"))), (first_name, second_name));
        assert_eq!(ints(field(&comparison, "both")), both, "{first_name} and {second_name}");
        assert_eq!(ints(field(&comparison, "only_first")), only_first);
        assert_eq!(ints(field(&comparison, "only_second")), only_second);
        let same_net = effect(first).net_sha256 == effect(second).net_sha256;
        assert_eq!(boolean(field(&comparison, "same_net")), same_net);
        assert_eq!(boolean(field(&comparison, "same_expansion")), literal_digest(first) == literal_digest(second));

        let swapped = answer(&mut layer, "macro_comparison", compared(second_name, second, first_name, first));
        assert!(equal(field(&swapped, "both"), field(&comparison, "both")));
        assert!(equal(field(&swapped, "only_first"), field(&comparison, "only_second")));
        assert!(equal(field(&swapped, "only_second"), field(&comparison, "only_first")));
        assert_eq!(text(field(&swapped, "first")), second_name);
        for name in ["same_expansion", "same_net"] {
            assert!(equal(field(&swapped, name), field(&comparison, name)), "{name}");
        }
    }

    let same = answer(&mut layer, "macro_comparison", compared("a", &[5, 7, -5, -7], "b", &[5, 7, -5, -7]));
    assert_eq!(ints(field(&same, "both")), support_orbits(&[5, 7, -5, -7]));
    assert!(ints(field(&same, "only_first")).is_empty() && ints(field(&same, "only_second")).is_empty());
    assert!(boolean(field(&same, "same_net")) && boolean(field(&same, "same_expansion")));
    let inverse = answer(&mut layer, "macro_comparison", compared("a", &[1, -1], "b", &[2, -2]));
    assert!(boolean(field(&inverse, "same_net")) && !boolean(field(&inverse, "same_expansion")));
    assert!(ints(field(&inverse, "both")).is_empty(), "an identity net map moves no orbit");
}
