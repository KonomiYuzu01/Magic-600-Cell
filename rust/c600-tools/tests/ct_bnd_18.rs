//! CT-BND-18 (whole: setup search runs only for a named target, its setups keep the buffers fixed, the solver can edit
//! the setup, and nothing previews without the solver's action) on 600-cell-Full. The entry is library code,
//! `c600-library`'s module `guarded_setups` (chapter 03's row "Guarded setups and buffers"). The built-in column
//! admits it under the setup-search exception, `Row::SetupSearch("setups")` (BND.1), and it runs on `c600-run`
//! through `Boundary` and the tool's host bridge onto a command layer, in the solving profile, author `built_in`. The
//! bridge serves the `setups` row from the certified guarded setup tables, `assets/execution_trees.json.gz`,
//! which the host gives it (`c600_tools::kernel_rows::SetupTable`). The solver's edit is a word the solver names to the
//! shipped tool's `preview_word` command; the editor that offers the edit is LP4's. The checker half of the exception
//! is LP2's (`c600-check/tests/exceptions.rs`). A pass is source and fixture evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row};
use c600_command::{code, CommandLayer, Secret, Shell, Status, Table};
use c600_digest::{sha256, sha256_hex};
use c600_kernel::archive::gunzip;
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Kernel, ProtectedSet, Step, PIECES};
use c600_library::{SETUPS_EXPORTS, SETUPS_MODULE, SETUPS_ROWS, SETUPS_SOURCE};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Loaded, Options, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::Value;
use c600_tools::bridge::{base, word_args, Bridge};
use c600_tools::kernel_rows::{SetupTable, SETUPS, SETUP_FAMILY, SETUP_TABLE};
use c600_tools::process::invoke_tool;
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins, Tool, ROW};

const LIMITS: Amounts = Amounts { steps: 1 << 24, kernel_cost: 1 << 40, memory: 1 << 34, output: 1 << 28 };

/// The declared answer of a `setups` read, as the library entry declares it.
const ANSWER: &str = "record { buffers: List<Nat>, family: Text, orbit: Nat, position: Nat, setups: List<record { \
                      frame: List<Nat>, node: Nat, setup: Word }>, table_sha256: Text }";

/// The target type, `record { orbit: Nat, position: Nat }`.
const TARGET: &str = "record { orbit: Nat, position: Nat }";

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

/// The table the host gives the bridge.
fn table() -> Arc<SetupTable> {
    static TABLE: OnceLock<Arc<SetupTable>> = OnceLock::new();
    let table = TABLE.get_or_init(|| match SetupTable::load(&repository().join("assets")) {
        Ok(table) => Arc::new(table),
        Err(error) => panic!("the setup table in assets/ loads: {error}"),
    });
    table.clone()
}

/// The table's bytes, read by the test itself.
fn table_bytes() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| std::fs::read(repository().join("assets").join(SETUP_TABLE)).expect("the table reads"))
}

/// The model's execution trees, gunzipped and parsed by the test itself, one per orbit.
fn trees() -> &'static [Json] {
    static TREES: OnceLock<Json> = OnceLock::new();
    let trees = TREES.get_or_init(|| json::parse(&gunzip(table_bytes(), 1 << 30).unwrap()).unwrap());
    trees.as_array().expect("one tree per orbit")
}

fn ints(value: &Json) -> Vec<i64> {
    value.as_array().expect("a list").iter().map(|item| item.as_i64().expect("an integer")).collect()
}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        load(&lower_source(SETUPS_SOURCE).expect("the library module lowers")).expect("the library module loads")
    })
}

/// A layer at the solved state with no protected orbit, and its shell, sharing one fixed secret.
fn session() -> (CommandLayer, Shell) {
    let secret = Secret::fixed([5; 32]);
    let none = ProtectedSet::none();
    let layer = CommandLayer::new(
        kernel(),
        Table::h08(),
        secret.clone(),
        Default::default(),
        "setups",
        kernel().solved(),
        none,
    );
    (layer, Shell::new(secret, "shell"))
}

/// The head digest, the head event, the state revision, the durable transactions and the jobs running.
fn fingerprint(layer: &mut CommandLayer) -> (String, String, u64, usize, usize) {
    let revision = layer.revision();
    let state = (layer.snapshot().digest().to_owned(), revision.head.clone(), revision.state);
    (state.0, state.1, state.2, layer.store().log.len(), layer.run_jobs())
}

/// The context of one run of `entry` in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer, entry: &str) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(SETUPS_SOURCE.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    Context { run: "setups".into(), base, producer, reads_state: true, inputs: vec![named_word_block()], pins: pins() }
}

/// Runs `setups_for` on the target through a boundary and a bridge that holds the table onto `layer`. The search
/// sends no invocation, starts no job and changes nothing.
fn search(layer: &mut CommandLayer, orbit: i64, position: i64) -> Run {
    let before = fingerprint(layer);
    let context = context(layer, "setups_for");
    let mut bridge = Bridge::new(layer, "setups").with_setups(Some(table()));
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let target = Value::record(vec![("orbit", Value::int(orbit)), ("position", Value::int(position))]);
    let run = loaded().run(SETUPS_MODULE, "setups_for", vec![target], &mut boundary, &Options::new(LIMITS));
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a setup search sends no invocation");
    assert_eq!(fingerprint(layer), before, "a setup search previews nothing and changes nothing");
    run.expect("the run starts")
}

/// The answer of a completed search.
fn setups(layer: &mut CommandLayer, orbit: i64, position: i64) -> Value {
    let run = search(layer, orbit, position);
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
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

fn items(value: &Value) -> &[Value] {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items()
}

fn letters(value: &Value) -> Vec<i32> {
    let Value::Word(word) = value else { panic!("a word: {value:?}") };
    word.to_vec()
}

fn wide(letters: &[i32]) -> Vec<i64> {
    letters.iter().map(|&m| i64::from(m)).collect()
}

fn inverse(word: &[i32]) -> Vec<i32> {
    word.iter().rev().map(|m| -m).collect()
}

/// The home slots of a position, in increasing order.
fn home(position: usize) -> Vec<i64> {
    let kernel = kernel();
    let rows = kernel.positions(&kernel.solved(), &[position as i64]).unwrap();
    rows[0].home.iter().map(|&slot| i64::from(slot)).collect()
}

/// The slots generator `m` moves, in increasing order, once per generator.
fn moved(cache: &mut BTreeMap<i32, BTreeSet<i64>>, m: i32) -> &BTreeSet<i64> {
    cache.entry(m.abs()).or_insert_with(|| {
        let kernel = kernel();
        let effect = kernel.effect(&kernel.word(&[i64::from(m)]).unwrap()).unwrap();
        effect.source().iter().map(|&slot| i64::from(slot)).collect()
    })
}

/// The letters from the root of `orbit`'s tree to `node`, read from the tree's `parent` and `move` lists.
fn tree_path(orbit: usize, node: usize) -> Vec<i32> {
    let tree = &trees()[orbit];
    let (parent, moves) = (ints(tree.get("parent").unwrap()), ints(tree.get("move").unwrap()));
    let mut path = Vec::new();
    let mut at = node;
    while parent[at] >= 0 {
        path.push(moves[at] as i32);
        at = parent[at] as usize;
    }
    path.reverse();
    path
}

/// The kernel's star of sign 1 at `orbit`'s `node`.
fn star(orbit: usize, node: usize) -> c600_kernel::Star {
    let step = json::object([
        ("kind", Json::String("star".into())),
        ("node", Json::Int(node as i64)),
        ("orbit", Json::Int(orbit as i64)),
        ("sign", Json::Int(1)),
    ]);
    let recipe = kernel().recipe(&Json::Array(vec![step])).unwrap();
    let [Step::Star(star)] = recipe.steps() else { panic!("one star step") };
    star.clone()
}

/// The built-in column admits the library entry under the setup-search exception: it reads state and prepares no
/// preview (LG3.13). A class that lists the read alone, or nothing, does not admit it: the setups are words a state read
/// derived, and only the exception lets an entry deliver them (BND.1).
#[test]
fn ct_bnd_18_the_entry_is_admitted_under_the_setup_search() {
    assert_eq!(SETUPS_ROWS, [SETUPS]);
    let program = lower_source(SETUPS_SOURCE).unwrap();
    let admit = |class: &[Row]| {
        let class: BTreeSet<Row> = class.iter().cloned().collect();
        admit_built_in(&program, SETUPS_MODULE, "setups_for", &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    for entry in SETUPS_EXPORTS {
        assert_eq!(entry, "setups_for");
        for class in
            [vec![Row::SetupSearch(SETUPS.into())], vec![Row::SetupSearch(SETUPS.into()), Row::Read(SETUPS.into())]]
        {
            let admitted = admit(&class);
            assert!(admitted.is_admitted(), "{class:?}: {:?}", admitted.diagnostics);
            assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);
        }
        for class in [vec![Row::Read(SETUPS.into())], vec![]] {
            let refused = admit(&class);
            assert!(!refused.is_admitted(), "{class:?}");
            assert!(refused.diagnostics.iter().any(|d| d.code == NOT_GRANTED), "{class:?}: {:?}", refused.diagnostics);
        }
    }
}

/// A stand-in module beside the library entry: variants that read the setups for a target the solver did not name, use
/// them other than delivered as read, or read them and propose.
fn stand_in() -> String {
    let read = |arg: &str| format!("read_snapshot<{ANSWER}>(\"setups\", {arg})");
    format!(
        "edition \"6cs-draft-1\";\nmodule searches requires [];\n\n\
         export fn named(goal: {TARGET}) -> {ANSWER} effects [snapshot] {{\n    let found = {named};\n    return found;\n}}\n\n\
         export fn literal() -> {ANSWER} effects [snapshot] {{\n    return {literal};\n}}\n\n\
         export fn computed(goal: {TARGET}) -> {ANSWER} effects [snapshot] {{\n    return {computed};\n}}\n\n\
         export fn first(goal: {TARGET}) -> Word effects [snapshot] {{\n    return index({named}.setups, 0).setup;\n}}\n\n\
         export fn skip_empty(goal: {TARGET}, other: {TARGET}) -> {ANSWER} effects [snapshot] {{\n    \
         let found = {named};\n    return match exact_eq(length(found.setups), 0) {{\n        true => {other};\n        \
         _ => found;\n    }};\n}}\n\n\
         export command search_and_preview(goal: {TARGET}, w: Word) -> {ANSWER} effects [snapshot, preview] {{\n    \
         let r = propose(w, target(\"operation.preview\"));\n    return {named};\n}}\n",
        named = read("goal"),
        literal = read("record { orbit = 3, position = 4327 }"),
        computed = read("record { orbit = goal.orbit, position = exact_add(goal.position, 1) }"),
        other = read("other"),
    )
}

/// Setup search runs only for a target the solver names: the built-in column refuses a read for a literal target or a
/// target the program computed, a setup the program picks from the answer, a test of the answer for emptiness that
/// skips to another target, and an entry that reads the setups and proposes. Each is refused with
/// `LG3:capability.not_granted` (BND.1, BND.64).
#[test]
fn ct_bnd_18_setup_search_runs_only_for_a_named_target() {
    let source = stand_in();
    let program = lower_source(&source).unwrap_or_else(|error| panic!("{error}\n{source}"));
    let admit = |entry: &str, class: &[Row]| {
        let class: BTreeSet<Row> = class.iter().cloned().collect();
        admit_built_in(&program, "searches", entry, &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    let search = Row::SetupSearch(SETUPS.into());
    let named = admit("named", std::slice::from_ref(&search));
    assert!(named.is_admitted(), "{:?}", named.diagnostics);
    for entry in ["literal", "computed", "first", "skip_empty"] {
        let refused = admit(entry, std::slice::from_ref(&search));
        assert!(!refused.is_admitted(), "{entry}");
        assert!(refused.diagnostics.iter().any(|d| d.code == NOT_GRANTED), "{entry}: {:?}", refused.diagnostics);
    }
    let previews = admit("search_and_preview", &[search, Row::Preview(ROW.into())]);
    assert!(!previews.is_admitted(), "an entry that reads the setups prepares no preview");
}

/// The setups for each target are the frame nodes of the orbit's execution tree whose frame lies at the named position,
/// in increasing node order, one per orientation, with no rank and no best mark: the test reads the tree from
/// `assets/execution_trees.json.gz` itself. Each setup is the tree's path from the root to its node and the kernel's
/// star path, and its frame is the node's frame, the named position's home slots in the orientation's order. Every
/// setup keeps the buffers fixed: no letter of it moves a buffer's home slot, so no prefix does. The answer names the
/// family, the orbit's buffers and the table's digest, which the manifest lists. Each orbit is searched at its root's
/// position (the third piece's own), at its last node's position and at the position with the most setups.
#[test]
fn ct_bnd_18_setups_keep_the_buffers_fixed() {
    let (mut layer, _) = session();
    let manifest = json::parse(&std::fs::read(repository().join("assets/manifest.json")).unwrap()).unwrap();
    let listed = manifest.get("files").unwrap().get(&format!("assets/{SETUP_TABLE}")).unwrap().as_str().unwrap();
    let digest = sha256_hex(table_bytes());
    assert_eq!(digest, listed, "the table is the one the manifest lists");
    assert_eq!(table().sha256(), digest);
    let mut cache = BTreeMap::new();
    let mut searched = 0;
    for (orbit, tree) in trees().iter().enumerate() {
        let positions = ints(tree.get("positions").unwrap());
        let frames = tree.get("frames").unwrap().as_array().unwrap();
        let buffers = ints(tree.get("buffers").unwrap());
        let mut by_position: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
        for (node, &position) in positions.iter().enumerate() {
            by_position.entry(position).or_default().push(node);
        }
        let widest = by_position.iter().max_by_key(|(_, nodes)| nodes.len()).map(|(&p, _)| p).unwrap();
        let buffer_slots: BTreeSet<i64> = buffers.iter().flat_map(|&b| home(b as usize)).collect();
        for position in BTreeSet::from([positions[0], positions[positions.len() - 1], widest]) {
            let answer = setups(&mut layer, orbit as i64, position);
            assert_eq!(text(field(&answer, "family")), SETUP_FAMILY);
            assert_eq!((int(field(&answer, "orbit")), int(field(&answer, "position"))), (orbit as i64, position));
            assert_eq!(items(field(&answer, "buffers")).iter().map(int).collect::<Vec<_>>(), buffers);
            assert_eq!(text(field(&answer, "table_sha256")), digest);
            let found = items(field(&answer, "setups"));
            let nodes: Vec<usize> = found.iter().map(|setup| int(field(setup, "node")) as usize).collect();
            assert_eq!(nodes, by_position[&position], "orbit {orbit} position {position}: the tree's nodes in order");
            let mut orientations = BTreeSet::new();
            for (setup, &node) in found.iter().zip(&nodes) {
                let path = letters(field(setup, "setup"));
                assert_eq!(path, tree_path(orbit, node), "orbit {orbit} node {node}: the tree's path");
                assert_eq!(path, star(orbit, node).path(), "orbit {orbit} node {node}: the kernel's star path");
                let frame: Vec<i64> = items(field(setup, "frame")).iter().map(int).collect();
                assert_eq!(frame, ints(&frames[node]), "orbit {orbit} node {node}: the node's frame");
                let mut sorted = frame.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, home(position as usize), "orbit {orbit} node {node}: the position's slots");
                assert!(orientations.insert(frame), "orbit {orbit} node {node}: one setup per orientation");
                for &m in &path {
                    let touched: Vec<&i64> = moved(&mut cache, m).intersection(&buffer_slots).collect();
                    assert!(touched.is_empty(), "orbit {orbit} node {node}: letter {m} moves buffer slots {touched:?}");
                }
            }
            searched += 1;
        }
    }
    assert!(searched >= 70, "{searched} targets");
}

/// A buffer position has no setup, and the answer still names the family examined: the orbit's tree, its buffers and
/// the table's digest. A target outside the model, or a position outside the named orbit, stops the run with
/// `KN2:input.named_object_invalid` and changes nothing.
#[test]
fn ct_bnd_18_a_buffer_has_no_setup_and_a_target_outside_the_orbit_is_refused() {
    let (mut layer, _) = session();
    for (orbit, tree) in trees().iter().enumerate().step_by(5) {
        for buffer in ints(tree.get("buffers").unwrap()) {
            let answer = setups(&mut layer, orbit as i64, buffer);
            assert!(items(field(&answer, "setups")).is_empty(), "orbit {orbit} buffer {buffer}");
            assert_eq!(text(field(&answer, "family")), SETUP_FAMILY);
            assert_eq!(
                items(field(&answer, "buffers")).iter().map(int).collect::<Vec<_>>(),
                ints(tree.get("buffers").unwrap())
            );
            assert_eq!(text(field(&answer, "table_sha256")), table().sha256());
        }
    }
    let elsewhere = ints(trees()[1].get("positions").unwrap())[0];
    for (orbit, position) in [(35, 0), (0, PIECES as i64), (0, elsewhere)] {
        let run = search(&mut layer, orbit, position);
        let summary = &run.summary;
        let expected = (Outcome::Error, Some(code::NAMED_OBJECT_INVALID));
        assert_eq!((summary.outcome, summary.code), expected, "({orbit}, {position}): {:?}", summary.stop);
    }
}

/// The solver can edit a setup, and nothing previews without the solver's action. The search previews nothing. The
/// solver picks one of the setups, edits it by appending a letter that moves no buffer slot but moves the third
/// piece's position, and builds the star word setup⁻¹, relocation⁻¹, seed, relocation, setup from the tree's seed and
/// relocation. The unedited setup's star word has the kernel star's effect; the edited one keeps the buffers fixed in
/// every prefix of its setup and cycles the two buffers with another third piece. It previews only when the solver
/// names it to the shipped tool's `preview_word` command: one `operation.preview` invocation of exactly that word.
#[test]
fn ct_bnd_18_the_solver_edits_a_setup_and_previews_it() {
    let (mut layer, mut shell) = session();
    let kernel = kernel();
    let orbit = 3;
    let tree = &trees()[orbit];
    let (seed, relocation) = (ints(tree.get("seed").unwrap()), ints(tree.get("relocation").unwrap()));
    let (seed, relocation): (Vec<i32>, Vec<i32>) =
        (seed.iter().map(|&m| m as i32).collect(), relocation.iter().map(|&m| m as i32).collect());
    let position = ints(tree.get("positions").unwrap())[1];
    let answer = setups(&mut layer, orbit as i64, position);
    let found = items(field(&answer, "setups"));
    assert!(found.len() >= 2, "a choice of setups");
    // The solver's choice: the last one listed.
    let chosen = found.last().unwrap();
    let (node, path) = (int(field(chosen, "node")) as usize, letters(field(chosen, "setup")));
    let star_word =
        |path: &[i32]| [inverse(path), inverse(&relocation), seed.clone(), relocation.clone(), path.to_vec()].concat();
    let effect = |word: &[i32]| kernel.effect(&kernel.word(&wide(word)).unwrap()).unwrap();
    let step = json::object([
        ("kind", Json::String("star".into())),
        ("node", Json::Int(node as i64)),
        ("orbit", Json::Int(orbit as i64)),
        ("sign", Json::Int(1)),
    ]);
    let by_star = kernel.effect(&kernel.recipe(&Json::Array(vec![step])).unwrap()).unwrap();
    let unedited = effect(&star_word(&path));
    assert_eq!(unedited.net_sha256, by_star.net_sha256, "the setup's star word is the kernel's star");

    let buffers = ints(tree.get("buffers").unwrap());
    let buffer_slots: BTreeSet<i64> = buffers.iter().flat_map(|&b| home(b as usize)).collect();
    let third: BTreeSet<i64> = home(position as usize).into_iter().collect();
    // The solver's edit: the first letter, in generator order, that moves no buffer slot and takes the third piece
    // to another position.
    let mut cache = BTreeMap::new();
    let (edited_path, edited, after) = (1..=1200)
        .find_map(|m| {
            let slots = moved(&mut cache, m);
            if !slots.is_disjoint(&buffer_slots) || slots.is_disjoint(&third) {
                return None;
            }
            let edited_path = [path.clone(), vec![m]].concat();
            let edited = star_word(&edited_path);
            let after = effect(&edited);
            let moved_now: BTreeSet<i64> = after.source().iter().map(|&slot| i64::from(slot)).collect();
            moved_now.is_disjoint(&third).then_some((edited_path, edited, after))
        })
        .expect("a letter that moves the third piece away and no buffer");
    for &letter in &edited_path {
        assert!(moved(&mut cache, letter).is_disjoint(&buffer_slots), "every prefix keeps the buffers fixed");
    }
    assert_eq!(
        edited,
        [vec![-edited_path[edited_path.len() - 1]], star_word(&path), vec![edited_path[edited_path.len() - 1]]]
            .concat()
    );
    assert_ne!(after.net_sha256, unedited.net_sha256, "the edit changes the star");
    assert_eq!(after.moved_pieces, unedited.moved_pieces, "still a cycle of the two buffers and one third piece");
    let moved_now: BTreeSet<i64> = after.source().iter().map(|&slot| i64::from(slot)).collect();
    assert!(moved_now.is_superset(&buffer_slots), "the edited star still cycles the buffers");

    // Nothing has previewed: the searches sent no invocation and started no job. The solver names the edited word to
    // the tool's command.
    assert_eq!(layer.run_jobs(), 0);
    let tool = Tool::full_600_cell().unwrap();
    let command = invoke_tool(&mut shell, &mut layer, &tool, &edited).unwrap_or_else(|error| panic!("{error}"));
    let run = command.run();
    let context = tool.context(&run, &base(&command.job.revision));
    let mut bridge = Bridge::new(&mut layer, &run).with_setups(Some(table()));
    let ran = tool.run_in(&edited, context, &mut bridge, &mut Store::default()).expect("the run starts");
    assert_eq!(ran.run.summary.outcome, Outcome::Completed);
    let [one] = &bridge.sent[..] else { panic!("one invocation") };
    assert_eq!(one.invocation.row, ROW);
    assert_eq!(one.invocation.args.canonical(), word_args(&edited).canonical(), "exactly the solver's word");
    assert!(matches!(one.receipt.status, Status::Running(_)), "{:?}", one.receipt.status);
    let receipt = command.acknowledge(&mut layer).expect("the layer holds the tool's job");
    assert!(matches!(receipt.status, Status::Completed(_)), "{receipt:?}");
}
