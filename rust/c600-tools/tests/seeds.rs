//! LB2's rows "Seeds and full seed analysis" and "Orbit catalogue and collateral stage order" on 600-cell-Full, on the
//! model's 35 seeds. The entries are library code, `c600-library`'s module `seeds`: `seed_report` reads KN2.4's
//! `effect` row at a seed the caller names and runs on `c600-run` through `Boundary` and the tool's host bridge onto a
//! command layer, in the solving profile, author `built_in`; `stage_order` is pure and runs with no host. The
//! references are pinned: the model's seed atlas, `assets/seed_atlas.json`, with each seed's word, target cycle,
//! collateral, per-orbit signature and certified stage, and the kernel's own effect of each seed. A pass is source and
//! fixture evidence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row};
use c600_command::{CommandLayer, Secret, Table};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Kernel, ProtectedSet, ORBITS};
use c600_library::{SEEDS_EXPORTS, SEEDS_MODULE, SEEDS_ROWS, SEEDS_SOURCE};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Loaded, NoHost, Options, Run};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::Value;
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::EFFECT;
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const LIMITS: Amounts = Amounts { steps: 1 << 24, kernel_cost: 1 << 40, memory: 1 << 34, output: 1 << 28 };

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
        load(&lower_source(SEEDS_SOURCE).expect("the library module lowers")).expect("the library module loads")
    })
}

/// The model's seed atlas, one row per orbit in orbit order.
fn atlas() -> &'static [Json] {
    static ATLAS: OnceLock<Json> = OnceLock::new();
    let path = repository().join("assets/seed_atlas.json");
    let document = ATLAS.get_or_init(|| {
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        json::parse(&bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
    });
    let rows = document.as_array().expect("one atlas row per orbit");
    assert_eq!(rows.len(), ORBITS, "one atlas row per orbit");
    rows
}

fn number(row: &Json, key: &str) -> i64 {
    row.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("an integer {key}"))
}

fn numbers(row: &Json, key: &str) -> Vec<i64> {
    let items = row.get(key).and_then(Json::as_array).unwrap_or_else(|| panic!("a list {key}"));
    items.iter().map(|item| item.as_i64().expect("an integer")).collect()
}

/// The seed word of an atlas row.
fn seed(row: &Json) -> Vec<i32> {
    numbers(row, "word").into_iter().map(|m| i32::try_from(m).expect("a letter")).collect()
}

/// A layer whose session starts solved, with no protected orbit; the `effect` row reads no state.
fn layer() -> CommandLayer {
    let solved = kernel().solved();
    CommandLayer::new(
        kernel(),
        Table::h08(),
        Secret::fixed([5; 32]),
        Default::default(),
        "seeds",
        solved,
        ProtectedSet::none(),
    )
}

/// The context of one run of `entry` in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer, entry: &str, inputs: usize) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(SEEDS_SOURCE.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    let inputs = vec![named_word_block(); inputs];
    Context { run: "seeds".into(), base, producer, reads_state: true, inputs, pins: pins() }
}

/// Runs `entry` on `args` through a boundary and a bridge onto `layer`; a read sends no invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, args: Vec<Value>) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    let context = context(layer, entry, args.len());
    let mut bridge = Bridge::new(layer, "seeds");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(SEEDS_MODULE, entry, args, &mut boundary, &Options::new(LIMITS));
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    let after = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    assert_eq!(after, before, "a read changes nothing");
    run.expect("the run starts")
}

/// The answer of a completed run.
fn completed(run: Run) -> Value {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
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

fn items(value: &Value) -> &[Value] {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items()
}

fn ints(value: &Value) -> Vec<i64> {
    items(value).iter().map(int).collect()
}

fn list(orbits: &[i64]) -> Value {
    Value::list(orbits.iter().map(|&orbit| Value::int(orbit)).collect())
}

/// A recipe as the entry takes it: one word step.
fn steps(word: &[i32]) -> Value {
    Value::list(vec![Value::record(vec![
        ("kind", Value::text("word")),
        ("moves", Value::word(word)),
        ("node", Value::int(0)),
        ("orbit", Value::int(0)),
        ("sign", Value::int(0)),
    ])])
}

/// `Digest<literal word>`: the SHA-256 of the letters as little-endian 32-bit integers.
fn literal_digest(letters: &[i32]) -> String {
    sha256_hex(&letters.iter().flat_map(|m| m.to_le_bytes()).collect::<Vec<u8>>())
}

/// `stage_order` on rows (orbit, collateral), run with no host: the answer's `order` and `cycle`.
fn stage_order(rows: &[(i64, Vec<i64>)]) -> (Vec<i64>, Vec<i64>) {
    let row = |(orbit, collateral): &(i64, Vec<i64>)| {
        Value::record(vec![("collateral", list(collateral)), ("orbit", Value::int(*orbit))])
    };
    let seeds = Value::list(rows.iter().map(row).collect());
    let answer =
        completed(loaded().run(SEEDS_MODULE, "stage_order", vec![seeds], &mut NoHost, &Options::new(LIMITS)).unwrap());
    assert_eq!(names(&answer), ["cycle", "order"]);
    (ints(field(&answer, "order")), ints(field(&answer, "cycle")))
}

/// The built-in column admits `seed_report` only when its class lists `effect`, and the entry reads state and prepares
/// no preview (LG3.13); it reads the row at the seed the caller names, never at an argument built from another read
/// (LB3.5). `stage_order` reads nothing and is admitted with no row.
#[test]
fn the_entries_are_admitted_with_their_rows() {
    assert_eq!(SEEDS_ROWS, [EFFECT]);
    let program = lower_source(SEEDS_SOURCE).unwrap();
    let admit = |entry: &str, rows: &[&str]| {
        let class: BTreeSet<Row> = rows.iter().map(|row| Row::Read(row.to_string())).collect();
        admit_built_in(&program, SEEDS_MODULE, entry, &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    let report = admit("seed_report", &SEEDS_ROWS);
    assert!(report.is_admitted(), "{:?}", report.diagnostics);
    assert!(report.separation.reads_state && !report.separation.prepares_previews);
    assert!(!admit("seed_report", &[]).is_admitted(), "seed_report without effect");
    for rows in [&[][..], &SEEDS_ROWS[..]] {
        let order = admit("stage_order", rows);
        assert!(order.is_admitted(), "{:?}", order.diagnostics);
        assert!(!order.separation.reads_state && !order.separation.prepares_previews);
    }
    assert_eq!(SEEDS_EXPORTS, ["seed_report", "stage_order"]);
}

/// A report answers each of the model's 35 seeds whole, as the kernel's effect gives it: the target orbit as named;
/// the net map's digest, the moved stickers and pieces and the full support; the expansion's digest and length; the
/// three pieces and the stickers the seed moves in its target orbit; and its collateral, every other orbit the net map
/// moves, in increasing order. Against the atlas: the target's pieces are its three-piece cycle, the support is the
/// atlas signature orbit by orbit, and the collateral is the atlas collateral, empty for the 12 seeds that have none.
#[test]
fn a_report_answers_each_seeds_whole_effect_and_collateral() {
    let kernel = kernel();
    let mut layer = layer();
    let mut free = 0;
    for row in atlas() {
        let orbit = number(row, "orbit");
        let word = seed(row);
        let report = completed(run(&mut layer, "seed_report", vec![Value::int(orbit), steps(&word)]));
        assert_eq!(
            names(&report),
            [
                "collateral",
                "expansion_sha256",
                "moved_pieces",
                "moved_stickers",
                "net_sha256",
                "orbit",
                "orbit_pieces",
                "orbit_stickers",
                "primitive_count",
                "support"
            ]
        );
        let wide: Vec<i64> = word.iter().map(|&m| i64::from(m)).collect();
        let effect = kernel.effect(&kernel.word(&wide).unwrap()).unwrap();
        assert_eq!(int(field(&report, "orbit")), orbit);
        assert_eq!(text(field(&report, "net_sha256")), effect.net_sha256, "orbit {orbit}");
        assert_eq!(int(field(&report, "moved_stickers")), effect.moved_stickers() as i64);
        assert_eq!(int(field(&report, "moved_pieces")), effect.moved_pieces as i64);
        assert_eq!(text(field(&report, "expansion_sha256")), literal_digest(&word));
        assert_eq!(int(field(&report, "primitive_count")), word.len() as i64);

        let signature = row.get("signature").expect("a signature");
        let support = items(field(&report, "support"));
        assert_eq!(support.len(), effect.support.len(), "orbit {orbit}: the full support");
        for (answered, kernel_row) in support.iter().zip(&effect.support) {
            let moved = int(field(answered, "orbit"));
            assert_eq!(moved, i64::from(kernel_row.orbit));
            let (pieces, stickers) = (int(field(answered, "pieces")), int(field(answered, "stickers")));
            assert_eq!((pieces, stickers), (kernel_row.pieces as i64, kernel_row.stickers as i64));
            let atlas_row = signature.get(&moved.to_string()).unwrap_or_else(|| panic!("orbit {orbit} moves {moved}"));
            assert_eq!((pieces, stickers), (number(atlas_row, "pieces"), number(atlas_row, "stickers")), "{orbit}");
        }
        let target = signature.get(&orbit.to_string()).expect("the seed moves its target orbit");
        assert_eq!(int(field(&report, "orbit_pieces")), 3);
        assert_eq!(int(field(&report, "orbit_pieces")), numbers(row, "raw_target_piece_cycle").len() as i64);
        assert_eq!(int(field(&report, "orbit_stickers")), number(target, "stickers"));

        let collateral = ints(field(&report, "collateral"));
        assert_eq!(collateral, numbers(row, "collateral"), "orbit {orbit}: the atlas collateral");
        let moved: Vec<i64> = effect.support.iter().map(|row| i64::from(row.orbit)).filter(|&o| o != orbit).collect();
        assert_eq!(collateral, moved, "orbit {orbit}: every other orbit the net map moves");
        free += usize::from(collateral.is_empty());
    }
    assert_eq!(free, 12, "twelve of the 35 seeds have no collateral");

    let none = completed(run(&mut layer, "seed_report", vec![Value::int(0), steps(&[1, -1])]));
    assert_eq!((int(field(&none, "orbit_pieces")), int(field(&none, "orbit_stickers"))), (0, 0));
    assert!(ints(field(&none, "collateral")).is_empty() && items(field(&none, "support")).is_empty());
}

/// The stage order over the collateral the reports answer: with the rows in orbit ID order, every orbit comes before
/// each orbit its seed moves, no row is held in a cycle, and the order is not ID order, which 11 of the seeds' edges
/// contradict. With the rows in the atlas's certified stage order, the order comes back unchanged.
#[test]
fn the_stage_order_follows_the_seeds_collateral() {
    let mut layer = layer();
    let rows: Vec<(i64, Vec<i64>)> = atlas()
        .iter()
        .map(|row| {
            let orbit = number(row, "orbit");
            let report = completed(run(&mut layer, "seed_report", vec![Value::int(orbit), steps(&seed(row))]));
            (orbit, ints(field(&report, "collateral")))
        })
        .collect();
    let edges: Vec<(i64, i64)> =
        rows.iter().flat_map(|(orbit, collateral)| collateral.iter().map(move |&moved| (*orbit, moved))).collect();
    assert_eq!(edges.iter().filter(|(from, to)| to < from).count(), 11, "edges against orbit ID order");

    let (order, cycle) = stage_order(&rows);
    assert!(cycle.is_empty(), "no seed's collateral closes a cycle: {cycle:?}");
    let ids: Vec<i64> = (0..ORBITS as i64).collect();
    assert_eq!(order.iter().copied().collect::<BTreeSet<_>>(), ids.iter().copied().collect(), "every orbit once");
    assert_eq!(order.len(), ORBITS);
    let at = |orbit: i64| order.iter().position(|&o| o == orbit).expect("placed");
    assert!(edges.iter().all(|&(from, to)| at(from) < at(to)), "every orbit before each orbit its seed moves");
    assert_ne!(order, ids, "not orbit ID order");

    let mut certified: Vec<&Json> = atlas().iter().collect();
    certified.sort_by_key(|row| number(row, "stage"));
    let row_of = |orbit: i64| rows.iter().find(|(listed, _)| *listed == orbit).expect("a row per orbit").clone();
    let staged: Vec<(i64, Vec<i64>)> = certified.iter().map(|row| row_of(number(row, "orbit"))).collect();
    let certified_order: Vec<i64> = staged.iter().map(|(orbit, _)| *orbit).collect();
    assert_eq!(stage_order(&staged), (certified_order, vec![]), "the certified order comes back unchanged");
}
