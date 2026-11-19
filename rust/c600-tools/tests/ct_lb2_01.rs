//! CT-LB2-01 (whole: retained word and star recipes from the oracle preserve chronological full-label effects, exact
//! frames and collateral, and filtering out another orbit changes no reported mechanical support) on 600-cell-Full.
//! The entries are library code, `c600-library`'s module `recipes`, admitted by the built-in column with the read row
//! `effect` and run on `c600-run` through `Boundary` and the tool's host bridge onto a command layer, in the solving
//! profile, author `built_in`; the bridge serves KN2.4's `effect` row (`c600_tools::kernel_rows`), which reads no
//! state.
//!
//! The references are pinned: the differential oracle's cases O01 to O07
//! (`work/experiments/language-lx/inputs/oracle/`, `docs/progress/1.0/oracle/README.md`), which record 0.4's effect of
//! every step, and the model's own seed atlas and execution trees in `assets/`. A pass is source and fixture evidence.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row};
use c600_command::{CommandLayer, Secret, Table};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::archive::gunzip;
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Kernel, ProtectedSet, Step};
use c600_library::{RECIPES_EXPORTS, RECIPES_MODULE, RECIPES_ROWS, RECIPES_SOURCE};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Loaded, Options, Run};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::EFFECT;
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const LIMITS: Amounts = Amounts { steps: 1 << 24, kernel_cost: 1 << 40, memory: 1 << 34, output: 1 << 28 };

/// The oracle's cases whose steps have an effect, and the case of invalid inputs.
const CASES: [&str; 6] = ["O01", "O02", "O03", "O04", "O05", "O06"];
const INVALID: &str = "O07";

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
        load(&lower_source(RECIPES_SOURCE).expect("the library module lowers")).expect("the library module loads")
    })
}

/// A JSON document, gunzipped when its name ends in `.gz`.
fn document(path: &Path) -> Json {
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let bytes = if path.extension().is_some_and(|ext| ext == "gz") { gunzip(&bytes, 1 << 30).unwrap() } else { bytes };
    json::parse(&bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// One oracle case.
fn case(id: &str) -> Json {
    document(&repository().join("work/experiments/language-lx/inputs/oracle").join(format!("{id}.json.gz")))
}

/// The model's execution trees, one per orbit: seed, relocation and per-node frames.
fn trees() -> &'static Json {
    static TREES: OnceLock<Json> = OnceLock::new();
    TREES.get_or_init(|| document(&repository().join("assets/execution_trees.json.gz")))
}

/// The model's seed atlas, one row per orbit, with the orbit's three atlas frames.
fn atlas() -> &'static Json {
    static ATLAS: OnceLock<Json> = OnceLock::new();
    ATLAS.get_or_init(|| document(&repository().join("assets/seed_atlas.json")))
}

fn ints(value: &Json) -> Vec<i32> {
    let items = value.as_array().expect("a list");
    items.iter().map(|item| i32::try_from(item.as_i64().expect("an integer")).expect("an i32")).collect()
}

/// A layer whose session starts solved, with no protected orbit; the `effect` row reads no state.
fn layer() -> CommandLayer {
    let solved = kernel().solved();
    CommandLayer::new(
        kernel(),
        Table::h08(),
        Secret::fixed([5; 32]),
        Default::default(),
        "recipes",
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
        module: sha256(RECIPES_SOURCE.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    let inputs = vec![named_word_block(); inputs];
    Context { run: "recipes".into(), base, producer, reads_state: true, inputs, pins: pins() }
}

/// Runs `entry` on `args` through a boundary and a bridge onto `layer`; a read sends no invocation and changes nothing.
fn run(layer: &mut CommandLayer, entry: &str, args: Vec<Value>) -> Run {
    let before = (layer.revision().clone(), layer.store().log.len());
    let context = context(layer, entry, args.len());
    let mut bridge = Bridge::new(layer, "recipes");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(RECIPES_MODULE, entry, args, &mut boundary, &Options::new(LIMITS));
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "a read changes nothing");
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

fn int(value: &Value) -> i64 {
    let Value::Int(n) = value else { panic!("an integer: {value:?}") };
    n.to_string().parse().expect("an i64")
}

fn text(value: &Value) -> &str {
    let Value::Text(t) = value else { panic!("a text: {value:?}") };
    t.as_str()
}

fn letters(value: &Value) -> Vec<i32> {
    let Value::Word(w) = value else { panic!("a word: {value:?}") };
    w.to_vec()
}

fn nats(items: &[i32]) -> Value {
    Value::list(items.iter().map(|&item| Value::int(item)).collect())
}

/// `Digest<literal word>`: the SHA-256 of the letters as little-endian 32-bit integers.
fn literal_digest(letters: &[i32]) -> String {
    sha256_hex(&letters.iter().flat_map(|m| m.to_le_bytes()).collect::<Vec<u8>>())
}

/// A recipe step as the entries take it: `record { kind, moves, node, orbit, sign }`.
fn step(kind: &str, moves: &[i32], node: i64, orbit: i64, sign: i64) -> Value {
    Value::record(vec![
        ("kind", Value::text(kind)),
        ("moves", Value::word(moves)),
        ("node", Value::int(node)),
        ("orbit", Value::int(orbit)),
        ("sign", Value::int(sign)),
    ])
}

/// One oracle recipe step in the entries' form.
fn oracle_step(json: &Json) -> Value {
    let get = |name: &str| json.get(name).and_then(Json::as_i64).expect("an integer field");
    match json.get("kind").and_then(Json::as_str) {
        Some("word") => step("word", &ints(json.get("moves").expect("moves")), 0, 0, 0),
        Some("star") => step("star", &[], get("node"), get("orbit"), get("sign")),
        other => panic!("a step kind: {other:?}"),
    }
}

fn recipe(json: &Json) -> Value {
    Value::list(json.as_array().expect("a recipe").iter().map(oracle_step).collect())
}

/// A support as the entries answer it, from the oracle's rows.
fn support(rows: &Json) -> Value {
    let row = |row: &Json| {
        let get = |name: &str| Value::int(row.get(name).and_then(Json::as_i64).expect("an integer field"));
        Value::record(vec![("orbit", get("orbit")), ("pieces", get("pieces")), ("stickers", get("stickers"))])
    };
    Value::list(rows.as_array().expect("support rows").iter().map(row).collect())
}

/// The built-in column admits the four reading entries only when their class lists `Row::Read("effect")`; each reads
/// state through the snapshot read and prepares no preview (LG3.13). The three constructors are pure and are admitted
/// with no row.
#[test]
fn ct_lb2_01_the_entries_are_admitted_with_the_effect_row() {
    assert_eq!(RECIPES_ROWS, [EFFECT]);
    let program = lower_source(RECIPES_SOURCE).unwrap();
    let admit = |entry: &str, rows: &[&str]| {
        let class: BTreeSet<Row> = rows.iter().map(|row| Row::Read(row.to_string())).collect();
        admit_built_in(&program, RECIPES_MODULE, entry, &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    for entry in RECIPES_EXPORTS {
        let pure = ["word_step", "star_step", "star_word"].contains(&entry);
        let admitted = admit(entry, &RECIPES_ROWS);
        assert!(admitted.is_admitted(), "{entry}: {:?}", admitted.diagnostics);
        assert_eq!(admitted.separation.reads_state, !pure, "{entry}");
        assert!(!admitted.separation.prepares_previews, "{entry}");
        assert_eq!(admit(entry, &[]).is_admitted(), pure, "{entry}: the effect row in its class");
    }
}

/// CT-LB2-01, full-label effects: every step of the oracle's cases O01 to O06 (every generator and its inverse,
/// generator chains, stars of every orbit with both signs, mixed word and star recipes and their chronological
/// inverses, and protection's cases), given to `recipes.effect_report` as the library's step records, answers the
/// effect 0.4 recorded: the net map's digest, the moved stickers and pieces, the digest of the literal expansion, its
/// length and the support per orbit, row for row. The answer's full source-to-destination map is the kernel's own, and
/// its digest is the recorded one, so the report carries every source-to-destination pair. A step that protection
/// refused in its chain still has its effect, which the report answers: the effect is a property of the recipe alone.
#[test]
fn ct_lb2_01_oracle_recipes_report_their_full_effect() {
    let kernel = kernel();
    let mut layer = layer();
    let mut reported = 0;
    for id in CASES {
        let case = case(id);
        for oracle in case.get("steps").and_then(Json::as_array).expect("steps") {
            let expect = oracle.get("expect").expect("expect");
            let Some(net) = expect.get("net_sha256").and_then(Json::as_str) else { continue };
            let report = answer(&mut layer, "effect_report", vec![recipe(oracle.get("recipe").expect("a recipe"))]);
            let at = format!("{id} step {:?}", oracle.get("index"));
            assert_eq!(text(field(&report, "net_sha256")), net, "{at}");
            let expansion = expect.get("expansion_sha256").and_then(Json::as_str);
            assert_eq!(Some(text(field(&report, "expansion_sha256"))), expansion, "{at}");
            for (name, recorded) in [
                ("moved_stickers", "net_moved_stickers"),
                ("moved_pieces", "net_moved_pieces"),
                ("primitive_count", "primitive_count"),
            ] {
                assert_eq!(Some(int(field(&report, name))), expect.get(recorded).and_then(Json::as_i64), "{at} {name}");
            }
            assert!(equal(field(&report, "support"), &support(expect.get("support").expect("support"))), "{at}");

            let normalized = kernel.recipe(oracle.get("recipe").unwrap()).expect("the kernel normalizes the recipe");
            let effect = kernel.effect(&normalized).expect("the kernel's effect");
            assert!(equal(field(&report, "source"), &nats(effect.source())), "{at}: the full map's sources");
            assert!(equal(field(&report, "destination"), &nats(effect.destination())), "{at}: its destinations");
            let map: Vec<u8> =
                effect.source().iter().chain(effect.destination()).flat_map(|s| s.to_le_bytes()).collect();
            assert_eq!(sha256_hex(&map), net, "{at}: the reported map is the recorded one");
            reported += 1;
        }
    }
    assert_eq!(reported, 2400 + 500 + 210 + 25 + 50 + 6, "every step of O01 to O06 has an effect");
}

/// CT-LB2-01, invalid recipes: O07's recipes that the library's step records can carry (an empty recipe, a star of an
/// orbit outside 0 to 34, of sign 0, of a node outside the tree, and a valid word followed by such a star) stop the
/// run with the kernel's own refusal, and none answers an effect. O07's letters 0, 1201, -1201 and 1.5 are no Full
/// letter: a `Word` holds signed letters of magnitude 1 to 1,200 only, so those recipes cannot reach the entry.
#[test]
fn ct_lb2_01_invalid_recipes_answer_no_effect() {
    let kernel = kernel();
    let mut layer = layer();
    let mut refused = 0;
    for oracle in case(INVALID).get("steps").and_then(Json::as_array).expect("steps") {
        let json = oracle.get("recipe").expect("a recipe");
        let steps = json.as_array().expect("a list");
        let full = |step: &Json| match step.get("moves") {
            Some(moves) => {
                moves.as_array().unwrap().iter().all(|m| m.as_i64().is_some_and(|m| (1..=1200).contains(&m.abs())))
            }
            None => true,
        };
        if !steps.iter().all(full) {
            continue;
        }
        let expected = kernel.recipe(json).expect_err("the kernel refuses the recipe").code;
        let stopped = run(&mut layer, "effect_report", vec![recipe(json)]);
        assert_eq!((stopped.summary.outcome, stopped.summary.code), (Outcome::Error, Some(expected)), "{json:?}");
        assert!(stopped.value.is_none());
        refused += 1;
    }
    assert_eq!(refused, 5);
}

/// The stars of O03 (the first, middle and last node of every orbit's tree, each with sign 1 and -1): the orbit, the
/// node, the sign and the setup path from the tree's root to the node, as the kernel normalizes the star.
fn stars() -> Vec<(usize, usize, i32, Vec<i32>)> {
    let kernel = kernel();
    let case = case("O03");
    let steps = case.get("steps").and_then(Json::as_array).expect("steps");
    let star = |oracle: &Json| {
        let normalized = kernel.recipe(oracle.get("recipe").unwrap()).expect("a star the kernel normalizes");
        let [Step::Star(star)] = normalized.steps() else { panic!("one star step") };
        (star.orbit(), star.node(), star.sign(), star.path().to_vec())
    };
    let stars: Vec<_> = steps.iter().map(star).collect();
    assert_eq!(stars.len(), 210);
    stars
}

/// CT-LB2-01, star construction: for every star of O03, `recipes.star_word` builds the literal word setup⁻¹,
/// relocation⁻¹, seed, relocation, setup from the tree's seed and relocation and the star's setup path, in that order.
/// Its digest is the expansion digest the kernel reports for the star step of sign 1 (its inverse for sign -1), and as
/// a word step it has the star step's effect: the same net map, moved stickers and support. The star step the library
/// builds with `recipes.star_step` is the oracle's step.
#[test]
fn ct_lb2_01_library_star_words_are_the_kernels_stars() {
    let mut layer = layer();
    let trees = trees().as_array().expect("one tree per orbit");
    for (orbit, node, sign, path) in stars() {
        let tree = &trees[orbit];
        let (seed, relocation) = (ints(tree.get("seed").unwrap()), ints(tree.get("relocation").unwrap()));
        let args = vec![Value::word(&path), Value::word(&relocation), Value::word(&seed)];
        let built = letters(&answer(&mut layer, "star_word", args));
        let inverse = |w: &[i32]| w.iter().rev().map(|m| -m).collect::<Vec<i32>>();
        let mut expected =
            [inverse(&path), inverse(&relocation), seed.clone(), relocation.clone(), path.clone()].concat();
        if sign < 0 {
            expected = inverse(&expected);
        }
        let word = if sign < 0 { inverse(&built) } else { built.clone() };
        assert_eq!(word, expected, "orbit {orbit} node {node} sign {sign}");

        let args = vec![Value::int(orbit as i64), Value::int(node as i64), Value::int(i64::from(sign))];
        let star = answer(&mut layer, "star_step", args);
        assert!(equal(&star, &step("star", &[], node as i64, orbit as i64, i64::from(sign))));
        let by_star = answer(&mut layer, "effect_report", vec![Value::list(vec![star])]);
        let as_word = answer(&mut layer, "word_step", vec![Value::word(&word)]);
        let by_word = answer(&mut layer, "effect_report", vec![Value::list(vec![as_word])]);
        assert_eq!(text(field(&by_star, "expansion_sha256")), literal_digest(&word), "orbit {orbit} node {node}");
        for name in ["net_sha256", "expansion_sha256", "moved_stickers", "primitive_count", "support"] {
            assert!(
                equal(field(&by_star, name), field(&by_word, name)),
                "orbit {orbit} node {node} sign {sign}: {name}"
            );
        }
    }
}

/// CT-LB2-01, exact frames: the frames of a star are ordered slot tuples. For every star of O03 with sign 1, the
/// orbit's atlas frames a and b and the tree's frame x at the star's node go a to b, b to x and x to a under the
/// star's map, slot for slot: `recipes.frame_image` answers the image tuple and `recipes.frame_maps` confirms each
/// pair, for the star step and for the library's star word as a word step alike. Sign -1 runs the cycle backwards.
/// Order matters: a frame of more than one slot (17 orbits' frames have two, five or twenty slots, the others one)
/// never maps to its image reversed, and no frame maps to itself. A slot the map does not move is its own image.
#[test]
fn ct_lb2_01_star_frames_are_exact_slot_tuples() {
    let mut layer = layer();
    let trees = trees().as_array().expect("one tree per orbit");
    let atlas = atlas().as_array().expect("one atlas row per orbit");
    let mut ordered = BTreeSet::new();
    for (orbit, node, sign, path) in stars() {
        let frames = atlas[orbit].get("frames").and_then(Json::as_array).expect("three atlas frames");
        let (a, b) = (ints(&frames[0]), ints(&frames[1]));
        let x = ints(&trees[orbit].get("frames").and_then(Json::as_array).expect("the tree's frames")[node]);
        let cycle = if sign > 0 { [(&a, &b), (&b, &x), (&x, &a)] } else { [(&b, &a), (&x, &b), (&a, &x)] };
        let star = Value::list(vec![step("star", &[], node as i64, orbit as i64, i64::from(sign))]);
        let tree = &trees[orbit];
        let args = vec![
            Value::word(&path),
            Value::word(&ints(tree.get("relocation").unwrap())),
            Value::word(&ints(tree.get("seed").unwrap())),
        ];
        let mut word = letters(&answer(&mut layer, "star_word", args));
        if sign < 0 {
            word = word.iter().rev().map(|m| -m).collect();
        }
        let as_word = Value::list(vec![step("word", &word, 0, 0, 0)]);
        for recipe in [&star, &as_word] {
            for &(from, to) in &cycle {
                let image = answer(&mut layer, "frame_image", vec![recipe.clone(), nats(from)]);
                assert!(equal(&image, &nats(to)), "orbit {orbit} node {node} sign {sign}: {from:?} goes to {to:?}");
                let maps = |layer: &mut CommandLayer, to: &[i32]| match answer(
                    layer,
                    "frame_maps",
                    vec![recipe.clone(), nats(from), nats(to)],
                ) {
                    Value::Bool(maps) => maps,
                    other => panic!("a Bool: {other:?}"),
                };
                assert!(maps(&mut layer, to));
                if to.len() > 1 {
                    let mut reversed = to.clone();
                    reversed.reverse();
                    assert!(!maps(&mut layer, &reversed), "orbit {orbit} node {node}: the reversed tuple");
                    ordered.insert(orbit);
                }
                assert!(!maps(&mut layer, from), "orbit {orbit} node {node}: the frame itself");
            }
        }
    }
    assert_eq!(ordered.len(), 17, "the orbits whose frames have more than one slot");
    let star = Value::list(vec![step("star", &[], 0, 0, 1)]);
    let report = answer(&mut layer, "effect_report", vec![star.clone()]);
    let Value::List(moved) = field(&report, "source") else { panic!("a list") };
    let moved: BTreeSet<i64> = moved.items().iter().map(int).collect();
    let fixed: Vec<i32> = (0..).filter(|slot| !moved.contains(&i64::from(*slot))).take(3).collect();
    let last = (0..259_800).rev().find(|slot| !moved.contains(&i64::from(*slot))).expect("a fixed slot");
    let fixed = [fixed, vec![last]].concat();
    assert!(equal(&answer(&mut layer, "frame_image", vec![star, nats(&fixed)]), &nats(&fixed)));
}

/// CT-LB2-01, collateral: `recipes.orbit_report` splits a recipe's support into the rows of the selected orbits and
/// the others, and never shortens the full support (LB2.2). For every mixed recipe of O04 and O05, with no orbit
/// selected, the orbits of its stars, an orbit it does not move, every orbit, the first orbit of its support alone and
/// the rest of its support's orbits, the report's full support is the recorded support row for row, the net map's digest and the moved
/// stickers are the recorded ones, the selected rows are exactly the support's rows of the selected orbits, the other
/// rows are the rest, and the two together, in support order, are the full support.
#[test]
fn ct_lb2_01_an_orbit_selection_reduces_no_support() {
    let mut layer = layer();
    for id in ["O04", "O05"] {
        let case = case(id);
        for oracle in case.get("steps").and_then(Json::as_array).expect("steps") {
            let expect = oracle.get("expect").expect("expect");
            let json = oracle.get("recipe").expect("a recipe");
            let full = support(expect.get("support").expect("support"));
            let Value::List(rows) = &full else { panic!("a list") };
            let moved: Vec<i64> = rows.items().iter().map(|row| int(field(row, "orbit"))).collect();
            let stars: Vec<i64> =
                json.as_array().unwrap().iter().filter_map(|s| s.get("orbit").and_then(Json::as_i64)).collect();
            let unmoved: Vec<i64> = (-1..35).filter(|orbit| !moved.contains(orbit)).take(1).collect();
            let first: Vec<i64> = moved.iter().copied().take(1).collect();
            let rest: Vec<i64> = moved.iter().copied().skip(1).collect();
            for selected in [Vec::new(), stars, unmoved, (-1..35).collect(), first, rest] {
                let orbits = Value::list(selected.iter().map(|&orbit| Value::int(orbit)).collect());
                let report = answer(&mut layer, "orbit_report", vec![recipe(json), orbits]);
                let at = format!("{id} step {:?} selecting {selected:?}", oracle.get("index"));
                assert!(equal(field(&report, "support"), &full), "{at}: the full support");
                assert_eq!(
                    Some(text(field(&report, "net_sha256"))),
                    expect.get("net_sha256").and_then(Json::as_str),
                    "{at}"
                );
                let stickers = expect.get("net_moved_stickers").and_then(Json::as_i64);
                assert_eq!(Some(int(field(&report, "moved_stickers"))), stickers, "{at}");
                let part = |inside: bool| {
                    let keep = |row: &&Value| selected.contains(&int(field(row, "orbit"))) == inside;
                    Value::list(rows.items().iter().filter(keep).cloned().collect())
                };
                assert!(equal(field(&report, "selected"), &part(true)), "{at}: the selected rows");
                assert!(equal(field(&report, "others"), &part(false)), "{at}: the other rows");
            }
        }
    }
}
