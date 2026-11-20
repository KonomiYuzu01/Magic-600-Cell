//! `jumble.repair.list` in the command layer (BND.2, the 17 October exception; JI.12, JI.33; draft): the repair list
//! at a cap the solver names, as a job on the exclusive lane that passes the kernel's list on whole and words what it
//! says. These tests are CT-LB3-05's and CT-BND-17's command-layer half, a job that names its cap (KN5). The kernel
//! half is `c600-jumble-tool/tests/lb3.rs` (M8's `observe::cuts::repair_list`) and the checker half
//! `c600-check/tests/exceptions.rs` (the built-in column's repair-list exception). Where the frozen text is silent, the
//! tests follow the draft recorded in `src/lib.rs` and say so.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, digest, CancelReason, CommandLayer, Completion, CompletionKind, Direction, Invocation, Kind, Lane,
    MemoryStore, Overlay, Profile, Receipt, Secret, Shell, Status, Table, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW,
    REPAIR_LIST_ROW,
};
use c600_jumble::codes::MENU_WORD_NEEDS_JUMBLE_ON;
use c600_jumble::levels::{self, Base};
use c600_jumble::observe::cuts::repair_list;
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Kernel, ProtectionMode};

const SECRET: [u8; 32] = [7; 32];

/// The pinned I_a menu's set and order digests.
const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";

/// The caps that are misaligned after the first two records of the I_a journal (c600-jumble-tool's lb3 test).
const MISALIGNED: [i64; 2] = [57, 86];

/// Words a field of the list must not hold: the list ranks, recommends and skips nothing.
const LEAKS: [&str; 8] = ["recommend", "suggest", "next", "skip", "best", "rank", "score", "target"];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn ctx() -> Arc<Context> {
    static CTX: OnceLock<Arc<Context>> = OnceLock::new();
    CTX.get_or_init(|| Arc::new(Context::load(&repository().join("assets")).expect("the exact model loads"))).clone()
}

/// `research/jumbling/fixtures/wj-I_a.json`: the I_a menu and a jumbling journal under it.
fn fixture_json() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let path = repository().join("research/jumbling/fixtures/wj-I_a.json");
        json::parse(&std::fs::read(path).expect("the I_a fixture")).expect("the I_a fixture is JSON")
    })
}

fn pinned() -> Menu {
    Menu::from_record(&ctx(), fixture_json().get("menu").expect("a menu")).expect("the pinned I_a menu")
}

/// The first `n` twists of the I_a fixture's jumbling journal.
fn journal(n: usize) -> Vec<Twist> {
    let records = fixture_json().get("journal").and_then(|journal| journal.get("records")).and_then(Value::as_array);
    let records = records.expect("the fixture's journal");
    records[..n].iter().map(|record| Twist::from_record(&ctx(), record).expect("a journal twist")).collect()
}

/// The direct press of `twist`, an item of the pinned menu at its grip.
fn item_of(twist: &Twist) -> Binding {
    let items = pinned().for_cap(&ctx(), twist.grip).expect("the menu at the grip");
    let index = items.iter().position(|item| item.matrix == twist.matrix).expect("an item of the pinned menu");
    Binding {
        request: wire::menu_request(SET, ORDER, index as u64),
        cap: twist.grip as u32,
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: SET.to_owned(), order: ORDER.to_owned() }),
    }
}

/// The kernel's repair list at `cap` after the first `n` twists of the journal, replayed from solved under the pinned
/// menu: what the layer's job must pass on.
fn kernel_list(n: usize, cap: i64) -> Value {
    let ctx = ctx();
    let menu = Arc::new(pinned());
    let identity = ctx.identity(Some(&menu));
    let solved = levels::Snapshot::solved(&ctx, Some(menu));
    let derivation = levels::derive(&ctx, Base::Snapshot(&solved), &identity, &journal(n), ProtectionMode::Net, &[]);
    let Ok(levels::Derivation::Derived(derived)) = derivation.expect("no engine error") else {
        panic!("the journal does not replay from solved")
    };
    repair_list(&ctx, derived.state(), cap, None, None, None).expect("no engine error")
}

/// `value` without the layer's `statement`.
fn unworded(value: &Value) -> Value {
    let mut fields = value.as_object().expect("a list is an object").clone();
    assert!(fields.remove("statement").is_some(), "the layer words the list");
    Value::Object(fields)
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value.get(name).unwrap_or_else(|| panic!("no field `{name}`"))
}

fn list_args(snapshot: &Value, cap: Value) -> Value {
    json::object([("snapshot", snapshot.clone()), ("cap", cap), ("max_chunk", Value::Int(8))])
}

/// `list_args` with one argument replaced, added or, for `None`, removed.
fn args_with(snapshot: &Value, name: &str, value: Option<Value>) -> Value {
    let Value::Object(mut fields) = list_args(snapshot, Value::Int(57)) else { panic!("an object") };
    match value {
        Some(value) => fields.insert(name.to_owned(), value),
        None => fields.remove(name),
    };
    Value::Object(fields)
}

/// Every field name in `value`, at any depth.
fn keys(value: &Value, names: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            for (name, inner) in fields {
                names.insert(name.clone());
                keys(inner, names);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, names)),
        _ => {}
    }
}

fn refusal(receipt: &Receipt) -> &'static str {
    assert_eq!(receipt.job, None, "a refusal takes no job: {:?}", receipt.status);
    receipt.status.code().unwrap_or_else(|| panic!("not refused: {:?}", receipt.status))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
}

/// A session at the solved state in `profile` with no protected orbit, the overlay query row `tool.repairs` and no
/// twist menu pinned.
fn unpinned(profile: Profile) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let store = MemoryStore::default();
    let mut layer =
        CommandLayer::new_with_profile(k, table, Secret::fixed(SECRET), store, "s1", start, protected, profile);
    let overlay = Overlay { module: "tools/jumbling".into(), entry: "repairs".into() };
    layer.register_overlay("tool.repairs", Kind::Query, &[], overlay).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell") }
}

/// `unpinned` with the I_a menu pinned and Jumble off.
fn pinned_session(profile: Profile) -> Fixture {
    let mut f = unpinned(profile);
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    f
}

/// `pinned_session` with Jumble on and the first `n` twists of the journal made as menu twists.
fn after(n: usize, profile: Profile) -> Fixture {
    let mut f = pinned_session(profile);
    let revision = f.layer.revision().to_json();
    let press = f.shell.prepare(&f.layer, JUMBLE_ON_ROW, json::object([("revision", revision)])).unwrap();
    let entered = f.shell.send(&mut f.layer, press);
    assert!(matches!(entered.status, Status::Committed(_)), "{:?}", entered.status);
    for twist in journal(n) {
        let binding = item_of(&twist);
        let Value::Object(mut letter) = binding.to_json() else { panic!("a binding is an object") };
        let cap = letter.remove("cap").expect("a binding has a cap");
        let revision = f.layer.revision().to_json();
        let args = json::object([("cap", cap), ("letter", Value::Object(letter)), ("revision", revision)]);
        let press = f.shell.prepare(&f.layer, JUMBLE_TWIST_ROW, args).unwrap();
        let moved = f.shell.send(&mut f.layer, press);
        assert!(matches!(moved.status, Status::Committed(_)), "{:?}", moved.status);
    }
    f
}

impl Fixture {
    /// The solver's `jumble.repair.list` at `cap`, naming the current revision.
    fn list(&mut self, cap: i64) -> Receipt {
        let args = list_args(&self.layer.revision().to_json(), Value::Int(cap));
        self.shell.invoke(&mut self.layer, REPAIR_LIST_ROW, args).unwrap()
    }

    /// `list` at `cap`, run: the settled receipt and the list it kept.
    fn listed(&mut self, cap: i64) -> (Receipt, Value) {
        let receipt = self.list(cap);
        let Status::Running(job) = receipt.status else { panic!("the list is not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        let settled = self.layer.outcome(&receipt.command, &receipt.request).expect("a settled receipt");
        let value = self.layer.repair_list(job).cloned().unwrap_or_else(|| panic!("no list: {:?}", settled.status));
        match &settled.status {
            Status::Completed(Completion { kind: CompletionKind::Query, revision, result: Some(result) }) => {
                assert_eq!(revision, self.layer.revision(), "the list names the revision it read (KN0.6)");
                assert_eq!((result.job, result.digest.as_str()), (Some(job), digest(&value).as_str()));
            }
            other => panic!("the list does not complete as a query: {other:?}"),
        }
        (settled, value)
    }

    fn from_run(&mut self, run: &str, command: &str, row: &str, args: Value) -> Receipt {
        let epoch = self.layer.revision().epoch;
        let invocation = Invocation { command: command.into(), row: row.into(), args, token: None, epoch };
        self.layer.invoke(&c600_command::Caller::Runtime { run: run.into() }, invocation)
    }

    /// The undo or redo of the action on top of its stack.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }
}

/// CT-LB3-05 and CT-BND-17 (part: the command-layer half, a job that names its cap through the command layer, KN5).
/// After the first two records of the I_a journal, the solver lists the repairs at the misaligned cap 57: the row
/// queues one job on the exclusive lane, which completes as a query naming the revision it read, and its list is the
/// kernel's list at that cap and those poses, whole, with the layer's `statement` added (`null` for a list with rows).
/// The list names its cap, its rows are five integers each in x-then-d order with no mark, and no field's name ranks,
/// recommends or skips. Nothing else is queued: no list is prefetched for another cap.
#[test]
fn ct_lb3_05_a_job_lists_the_kernels_repairs_at_the_cap_the_solver_names() {
    let mut f = after(2, Profile::Solving);
    assert_eq!(f.layer.availability(REPAIR_LIST_ROW), Ok(()));
    let (_, value) = f.listed(57);
    assert_eq!(unworded(&value), kernel_list(2, 57), "the layer passes the kernel's list on whole");
    assert_eq!(field(&value, "status").as_str(), Some("completed"));
    assert_eq!(field(&value, "statement"), &Value::Null, "a list with rows needs no statement");
    assert_eq!(field(&value, "input_digest").as_str(), Some(f.layer.revision().digest.as_str()));
    assert_eq!(field(field(&value, "parameters"), "cap"), &Value::Int(57), "the list names its cap");

    let rows: Vec<Vec<i64>> = field(&value, "rows")
        .as_array()
        .expect("a completed list has rows")
        .iter()
        .map(|row| row.as_array().expect("a row").iter().map(|n| n.as_i64().expect("an integer")).collect())
        .collect();
    assert!(!rows.is_empty(), "pairs at cap 57 lower N");
    assert!(rows.iter().all(|row| row.len() == 5 && row[4] < 0), "a row is x, d, y, z and a negative delta");
    let pairs: Vec<(i64, i64)> = rows.iter().map(|row| (row[0], row[1])).collect();
    assert!(pairs.windows(2).all(|w| w[0] < w[1]), "x in pinned order, then d: {pairs:?}");
    assert!(pairs.iter().all(|&(_, d)| d == MISALIGNED[1]), "cap 86 is the only misaligned cap meeting cap 57");

    let mut names = BTreeSet::new();
    keys(&value, &mut names);
    for name in &names {
        assert!(!LEAKS.iter().any(|leak| name.contains(leak)), "field `{name}`");
    }
    assert_eq!(f.layer.run_jobs(), 0, "no list is prefetched");
}

/// CT-BND-17 (part: an empty list worded as BND.2 states; JI.12). After the first record alone, no misaligned cap meets
/// cap 57, so the kernel's list completes with no rows, and the layer words it: no pair at cap 57 lowers N. The
/// statement speaks only of that cap and that input; a list with rows, and an error, carry none. A closed cap is the
/// kernel's error with no outcomes (BND.4: whether a cap is blocked or closed is a fact about the configuration),
/// passed on as the list's status, not as a failed job.
#[test]
fn ct_bnd_17_an_empty_list_says_that_no_pair_at_its_cap_lowers_n() {
    let mut f = after(1, Profile::Solving);
    let (_, value) = f.listed(57);
    assert_eq!(unworded(&value), kernel_list(1, 57));
    assert_eq!(field(&value, "status").as_str(), Some("completed"));
    assert_eq!(field(&value, "rows"), &Value::Array(Vec::new()));
    assert_eq!(field(&value, "statement").as_str(), Some("No pair at cap 57 lowers N."));

    let mut f = after(2, Profile::Solving);
    let (_, closed) = f.listed(0);
    assert_eq!(unworded(&closed), kernel_list(2, 0));
    assert_eq!(field(&closed, "status").as_str(), Some("error"), "cap 0 is not misaligned");
    assert_eq!((field(&closed, "rows"), field(&closed, "statement")), (&Value::Null, &Value::Null));
    assert_eq!(field(&closed, "outcomes"), &Value::Array(Vec::new()));
}

/// CT-LB3-05 and CT-BND-17 (part: no job for an unnamed cap; BND.64). The row names its cap: without `cap`, or with a
/// `cap` that is not a `CapId` (an integer from 0 to 599), the invocation is refused with `KN5:input.argument_form` and
/// queues nothing; so is a `max_chunk` that is not a `PositiveNat` and a `snapshot` that is not KN0.2's `RevisionRef`
/// (draft: a derived state reference is not built in the headless layer). A snapshot of another revision is
/// `KN5:stale.revision_changed`. No refusal changes the revision. H-08's row has no key and does not repeat, and no
/// jumbling row steps to another cap, so no key skips caps with an empty list.
#[test]
fn no_job_starts_for_a_cap_the_invocation_does_not_name() {
    let mut f = after(2, Profile::Solving);
    let before = format!("{:?}", f.layer.revision());
    let now = f.layer.revision().to_json();
    let malformed = [
        ("cap", None),
        ("cap", Some(Value::Null)),
        ("cap", Some(Value::Int(600))),
        ("cap", Some(Value::Int(-1))),
        ("cap", Some(Value::String("57".into()))),
        ("max_chunk", None),
        ("max_chunk", Some(Value::Int(0))),
        ("snapshot", None),
        ("snapshot", Some(Value::String("head".into()))),
    ];
    for (name, value) in malformed {
        let args = args_with(&now, name, value.clone());
        let receipt = f.shell.invoke(&mut f.layer, REPAIR_LIST_ROW, args.clone()).unwrap();
        assert_eq!(refusal(&receipt), code::ARGUMENT_FORM, "{name} = {value:?}");
    }
    let unnamed = args_with(&now, "cap", None);
    let hint = f.layer.availability_for(REPAIR_LIST_ROW, &unnamed).map_err(|e| e.code);
    assert_eq!(hint, Err(code::ARGUMENT_FORM), "the hint admits no unnamed cap either");
    let Value::Object(mut stale) = now.clone() else { panic!("a revision is an object") };
    stale.insert("state".into(), Value::Int(1));
    let receipt = f.shell.invoke(&mut f.layer, REPAIR_LIST_ROW, list_args(&Value::Object(stale), Value::Int(57)));
    assert_eq!(refusal(&receipt.unwrap()), code::REVISION_CHANGED);
    assert_eq!(f.layer.run_jobs(), 0, "no refusal queued a job");
    assert_eq!(format!("{:?}", f.layer.revision()), before);

    let table = Table::h08();
    let row = table.row(REPAIR_LIST_ROW).expect("H-08's row");
    assert_eq!((row.kind, row.lane, row.job, row.repeat), (Kind::Query, Lane::Exclusive, true, false));
    assert_eq!((row.key_0_4.as_deref(), row.takes_token()), (None, false));
    assert_eq!(row.arguments.get("cap").map(String::as_str), Some("CapId"), "the cap is not optional");
    for row in table.rows().filter(|row| row.id.starts_with("jumble.")) {
        assert!(!["next", "skip", "first", "best"].iter().any(|word| row.id.contains(word)), "{}", row.id);
    }
}

/// The list reads the jumbling engine's poses: without a pinned twist menu the row is not built
/// (`KN5:input.row_not_built`), and with Jumble off it is refused with `KN5:input.menu_word_needs_jumble_on` (draft:
/// the registry has no code of its own for a jumbling query with Jumble off). Availability answers the same.
#[test]
fn the_list_needs_a_pinned_menu_and_jumble_on() {
    let mut f = unpinned(Profile::Solving);
    assert_eq!(f.layer.availability(REPAIR_LIST_ROW).map_err(|e| e.code), Err(code::ROW_NOT_BUILT));
    assert_eq!(refusal(&f.list(57)), code::ROW_NOT_BUILT);
    let mut f = pinned_session(Profile::Solving);
    assert_eq!(f.layer.availability(REPAIR_LIST_ROW).map_err(|e| e.code), Err(MENU_WORD_NEEDS_JUMBLE_ON));
    assert_eq!(refusal(&f.list(57)), MENU_WORD_NEEDS_JUMBLE_ON);
    assert_eq!(f.layer.run_jobs(), 0);
}

/// JI.33: the list holds the exclusive lane, so a second list waits for none and is refused with `KN7:busy.lane_full`
/// while the first is queued. A job in the exclusive lane never blocks undo, and a state change cancels its
/// revision-bound work: the undo commits, the queued list ends `cancelled(replaced_revision)` and runs no more. A
/// completed list is kept only while no action has moved the state since (JI.12: it speaks of its input only), and a
/// restart keeps none.
#[test]
fn a_list_holds_the_exclusive_lane_and_a_state_change_ends_it() {
    let mut f = after(2, Profile::Solving);
    let first = f.list(MISALIGNED[0]);
    let Status::Running(job) = first.status else { panic!("{:?}", first.status) };
    assert_eq!(refusal(&f.list(MISALIGNED[1])), code::LANE_FULL);
    assert_eq!(f.layer.availability(REPAIR_LIST_ROW).map_err(|e| e.code), Err(code::LANE_FULL));

    let undo = f.step(Direction::Undo);
    assert!(matches!(undo.status, Status::Committed(_)), "the list does not block undo: {:?}", undo.status);
    let settled = f.layer.outcome(&first.command, &first.request).expect("a settled receipt");
    assert_eq!(settled.status, Status::Cancelled(CancelReason::ReplacedRevision));
    assert_eq!(f.layer.run_jobs(), 0);
    assert_eq!(f.layer.repair_list(job), None);

    let (receipt, _) = f.listed(MISALIGNED[0]);
    let Some(kept) = receipt.job else { panic!("the list ran as a job") };
    assert!(f.layer.repair_list(kept).is_some());
    let redo = f.step(Direction::Redo);
    assert!(matches!(redo.status, Status::Committed(_)), "{:?}", redo.status);
    assert_eq!(f.layer.repair_list(kept), None, "a state change ends the kept list");

    let (again, _) = f.listed(MISALIGNED[0]);
    let Some(kept) = again.job else { panic!("the list ran as a job") };
    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.repair_list(kept), None, "a new epoch keeps no list");
}

/// BND.64 (draft for the layer): in a solving session, no shipped job starts for a cap the solver has not named. A run
/// may list the repairs only inside a command the solver invoked, whose cap the checker holds to the cap the solver
/// named (c600-check's repair-list exception); a run outside one, such as a recomputation, is refused with
/// `LG3:capability.not_granted` and queues nothing. Outside a solving session BND.64 does not apply, and the run's list
/// is admitted. The solver's own list is admitted in every profile.
#[test]
fn in_a_solving_session_a_run_lists_only_inside_a_command_the_solver_invoked() {
    for profile in Profile::ALL {
        let mut f = after(2, profile);
        let args = list_args(&f.layer.revision().to_json(), Value::Int(57));
        let outside = f.from_run("recompute/1", "r1", REPAIR_LIST_ROW, args.clone());
        if profile == Profile::Solving {
            assert_eq!(refusal(&outside), code::NOT_GRANTED, "{profile:?}");
            assert_eq!(f.layer.run_jobs(), 0, "{profile:?}: no job was queued");

            let tool = f.shell.invoke(&mut f.layer, "tool.repairs", json::object([])).unwrap();
            let Status::Running(job) = tool.status else { panic!("{:?}", tool.status) };
            let taken = f.layer.take_program_job().expect("the tool's job");
            assert_eq!(taken.job, job);
            let inside = f.from_run(&taken.run(), "p1", REPAIR_LIST_ROW, args);
            assert!(matches!(inside.status, Status::Running(_)), "{:?}", inside.status);
        } else {
            assert!(matches!(outside.status, Status::Running(_)), "{profile:?}: {:?}", outside.status);
        }
        assert_eq!(f.layer.run_jobs(), 1, "{profile:?}");
        let mine = f.list(57);
        assert!(matches!(mine.status, Status::Running(_)), "{profile:?}: the solver's list: {:?}", mine.status);
    }
}
