//! Conformance tests for chapter 02's LG4 rows (LP2, M2 and M3): CT-LG4-01, CT-LG4-02, CT-LG4-04, CT-LG4-06 and
//! CT-LG4-08, each naming the part it runs. CT-LG4-05 has no test: the IR draft has no pointwise or whole-state
//! contract, so nothing raises `LG4:input.pointwise_failed` yet. The determinism rows (CT-DET-*) are another suite's.

mod conformance;

use std::cell::Cell;

use c600_run::boundary::Store;
use c600_run::interp::{Cancel, CancelFlag, Options};
use c600_run::meter::code::{CANCELLED_REVISION_REPLACED, STEPS};
use c600_run::meter::{Amounts, Dim, Interruption, Outcome};
use c600_run::provenance::{Author, Profile};
use c600_run::value::Value;
use conformance::*;

const BUILT_IN: (Author, Profile) = (Author::BuiltIn, Profile::Solving);

/// Lets `left` operations through, then reports a replaced revision. An interruption is observed before each
/// operation and adds no row (DET.56), so `left` counts rows.
struct After {
    left: Cell<usize>,
}

impl Cancel for After {
    fn pending(&self) -> Option<Interruption> {
        match self.left.get() {
            0 => Some(Interruption::RevisionReplaced),
            n => {
                self.left.set(n - 1);
                None
            }
        }
    }
}

fn after(left: usize) -> After {
    After { left: Cell::new(left) }
}

/// The given limits with one dimension set to `value`.
fn with(limits: Amounts, dim: Dim, value: u128) -> Amounts {
    let mut limits = limits;
    match dim {
        Dim::Steps => limits.steps = value,
        Dim::KernelCost => limits.kernel_cost = value,
        Dim::Memory => limits.memory = value,
        Dim::Output => limits.output = value,
    }
    limits
}

/// A host record read whole, and a recursion over one of its fields that only a budget stops.
const LAYOUT: &str = "export fn look() -> record { a: Nat, b: Text } effects [snapshot] {\n    \
    return read_snapshot<record { a: Nat, b: Text }>(\"r\");\n}\n\n\
    export fn count(n: Nat) -> Nat effects [snapshot] {\n    \
    return count(exact_add(n, read_snapshot<record { a: Nat, b: Text }>(\"r\").a));\n}\n";

/// CT-LG4-01 (part: route RT1, the only route LQ9 admits; not run: a second admitted route, so "admitted routes
/// preserve the logical contract" is shown for RT1 alone): repeated runs, and hosts that lay out the same record's
/// fields in either order, give identical summaries and result bytes, and a run cut off by its budget stops at the
/// same position each time.
#[test]
fn ct_lg4_01_repeats_and_host_layouts_give_identical_bytes_and_stops() {
    let (_, loaded) = build(&[&m(LAYOUT)]);
    let layouts =
        [vec![("a", Value::int(2)), ("b", Value::text("x"))], vec![("b", Value::text("x")), ("a", Value::int(2))]];
    let cut = Options { limits: with(WIDE, Dim::Steps, 300), ..Options::new(WIDE) };
    let (mut looks, mut counts) = (Vec::new(), Vec::new());
    for fields in &layouts {
        for _ in 0..2 {
            let mut engine = Engine::with(&[("r", Value::record(fields.clone()))]);
            looks.push(loaded.run("m", "look", vec![], &mut engine, &Options::new(WIDE)).unwrap());
            counts.push(loaded.run("m", "count", vec![Value::int(0)], &mut engine, &cut).unwrap());
        }
    }
    let first = &looks[0];
    assert_eq!(first.summary.outcome, Outcome::Completed, "{:?}", first.summary.code);
    assert_eq!(first.output.as_deref(), Some(&b"{\"a\":{\"int\":\"2\"},\"b\":\"x\"}\n"[..]));
    for r in &looks[1..] {
        assert_eq!((&r.summary, &r.output), (&first.summary, &first.output));
    }
    let first = &counts[0];
    assert_eq!((first.summary.outcome, first.summary.code), (Outcome::CutOff, Some(STEPS)));
    for r in &counts[1..] {
        assert_eq!(r.summary, first.summary);
    }
}

/// Proposes the whole state word.
const PROPOSE_STATE: &str = "export fn go() -> Text effects [snapshot, preview] {\n    \
    return receipt_id(propose(read_snapshot<Word>(\"state\"), target(\"t\")));\n}\n";

/// Whether a row is a host call: a state read (`#kernel`) or a preview (`#invoke`). Encoding passes also charge
/// kernel cost (DET.59), so the cost alone does not tell.
fn host_row(at: &str) -> bool {
    at.ends_with("#kernel") || at.ends_with("#invoke")
}

/// CT-LG4-02 (part: whole-state reads and previews; not run: kernel decision calls and their oracle agreement
/// coverage, which wait for the kernel, M4): a whole-state read and a preview are one host row each whatever the size
/// of the value, and the same operations run for a one-letter and a 4,800-letter state; the interpreter has no request
/// per sticker or per letter (the request list is matched without a wildcard in `conformance_lg3.rs`).
#[test]
fn ct_lg4_02_a_whole_state_operation_is_one_host_row() {
    let source = m(PROPOSE_STATE);
    let mut operations = Vec::new();
    for size in [1, 4800] {
        let letters: Vec<i32> = (0..size).map(|i| i % 1200 + 1).collect();
        let mut engine = Engine::with(&[("state", Value::word(&letters))]);
        let (mut store, options) = (Store::default(), Options::new(WIDE));
        let ran = run_as(&source, "go", BUILT_IN, vec![], vec![], &mut engine, &mut store, &options);
        assert_eq!(ran.run.summary.outcome, Outcome::Completed, "{:?}", ran.run.summary.code);
        assert_eq!(engine.previews, [letters], "the whole word in one request");
        let ats: Vec<_> = ran.run.summary.rows.iter().map(|r| r.at.clone()).collect();
        let host: Vec<_> = ats.iter().filter(|at| host_row(at)).collect();
        assert_eq!(host, ["m.go:0.0#kernel", "m.go:0#invoke"], "one read and one preview");
        operations.push(ats);
    }
    assert_eq!(operations[0], operations[1]);
}

/// CT-LG4-04 (part: the four deterministic resources; not run: cache hits and eviction, as the interpreter keeps no
/// cache yet, and the wall-clock safety stop, which is not built): a run that reads the state, proposes and returns a
/// text stops with each resource's own code when that resource alone is one unit short of what the run used, at the
/// same position on a repeat; a kernel cost one unit short of the preview's tariff stops before the engine is asked.
#[test]
fn ct_lg4_04_each_deterministic_resource_is_exhausted_separately() {
    let source = m(PROPOSE_STATE);
    let go = |limits: Amounts| {
        let mut engine = Engine::with(&[("state", Value::word(&[1, -2, 3]))]);
        let (mut store, options) = (Store::default(), Options::new(limits));
        let ran = run_as(&source, "go", BUILT_IN, vec![], vec![], &mut engine, &mut store, &options);
        (ran.run, engine.previews.len())
    };
    let (full, previews) = go(WIDE);
    assert_eq!((full.summary.outcome, previews), (Outcome::Completed, 1));
    let used = full.summary.used;
    assert!(Dim::ALL.iter().all(|d| used.get(*d) > 0), "{used:?}");
    for dim in Dim::ALL {
        let limits = with(WIDE, dim, used.get(dim) - 1);
        let (first, _) = go(limits);
        let (again, _) = go(limits);
        assert_eq!((first.summary.outcome, first.summary.code), (Outcome::CutOff, Some(dim.code())), "{dim:?}");
        assert_eq!(first.summary.stop.as_ref().map(|s| s.phase), Some(dim.name()), "{dim:?}");
        assert_eq!(first.summary, again.summary, "{dim:?}");
    }
    let rows = &full.summary.rows;
    let invoke = rows.iter().position(|r| r.at.ends_with("#invoke")).expect("the preview's row");
    let through: u128 = rows[..=invoke].iter().map(|r| r.kernel_cost).sum();
    let (short, previews) = go(with(WIDE, Dim::KernelCost, through - 1));
    assert_eq!(short.summary.stop.as_ref().map(|s| s.at.as_str()), Some("m.go:0#invoke"));
    assert_eq!(previews, 0, "the preview's kernel cost is charged before the engine is asked");
}

/// A view of the state at the run's revision.
const VIEW: &str = "export view show() -> Unit effects [snapshot, view] {\n    \
    return publish_view(read_snapshot<Nat>(\"n\"));\n}\n";

/// CT-LG4-06 (part: superseded view runs at the runtime; the view queue that revision bursts supersede is
/// `c600-view/tests/view_rows.rs`'s, and a burst of commits through the command layer runs in
/// `c600-tools/tests/views.rs` and `c600-input/tests/twist.rs`; not run: the queue driving a program's real runs, since
/// no host binds a run's `publish_view` output to a view (LP3's host)): a view run whose revision is replaced stops
/// with `KN7:cancelled.revision_replaced` at whichever operation the replacement reaches, publishes nothing after it,
/// and changes no workspace record, preview or committed state; what a view publishes is its own revision's state. In
/// a burst of three revisions whose first two are replaced before their runs start, only the third publishes.
#[test]
fn ct_lg4_06_a_replaced_revision_stops_view_work_before_it_publishes() {
    let source = m(VIEW);
    let seven = b"{\"int\":\"7\"}\n".to_vec();
    let show = |cancel: Option<&dyn Cancel>| {
        let (mut engine, mut store) = (Engine::with(&[("n", Value::int(7))]), Store::default());
        let hash = engine.state_hash();
        let options = Options { cancel, ..Options::new(WIDE) };
        let ran = run_as(&source, "show", BUILT_IN, vec![], vec![], &mut engine, &mut store, &options);
        assert!(engine.previews.is_empty() && store.records.is_empty() && engine.state_hash() == hash);
        (ran.run, engine.published)
    };
    let (full, published) = show(None);
    assert_eq!(full.summary.outcome, Outcome::Completed);
    assert_eq!(published, [seven.clone()]);
    let rows = full.summary.rows.len();
    let publish = full.summary.rows.iter().position(|r| r.at.ends_with("#publish_view")).expect("a publish row");
    for k in 0..rows {
        let replaced = after(k);
        let (run, published) = show(Some(&replaced));
        assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Cancelled, Some(CANCELLED_REVISION_REPLACED)));
        let expected = if k <= publish { vec![] } else { vec![seven.clone()] };
        assert_eq!(published, expected, "replaced after {k} of {rows} operations");
    }

    let replaced = CancelFlag::default();
    replaced.cancel(Interruption::RevisionReplaced);
    let (mut engine, mut store) = (Engine::default(), Store::default());
    for n in 1..=3 {
        engine.snapshot.insert("n".into(), Value::int(n));
        let cancel: Option<&dyn Cancel> = if n < 3 { Some(&replaced) } else { None };
        let options = Options { cancel, ..Options::new(WIDE) };
        let ran = run_as(&source, "show", BUILT_IN, vec![], vec![], &mut engine, &mut store, &options);
        let outcome = if n < 3 { Outcome::Cancelled } else { Outcome::Completed };
        assert_eq!(ran.run.summary.outcome, outcome, "revision {n}");
    }
    assert_eq!(engine.published, [b"{\"int\":\"3\"}\n".to_vec()]);
}

/// Previews a word, then reads the state.
const PREVIEW_THEN_READ: &str = "export fn go(w: Word) -> Nat effects [snapshot, preview] {\n    \
    let r = propose(w, target(\"t\"));\n    return read_snapshot<Nat>(\"n\");\n}\n";

/// CT-LG4-08 (part: runtime faults; not run: a direct kernel binding, which waits for the command layer, LP3, and the
/// kernel, M4, and LX-2's own run, which is an LP3 acceptance run on the owner's machine): a run that previewed and
/// then stops on a budget, on an interruption or on the engine's refusal leaves the engine's journal and state hash
/// as they were, and the engine kept only the preview.
#[test]
fn ct_lg4_08_runtime_faults_leave_the_journal_and_state_hash_intact() {
    let source = m(PREVIEW_THEN_READ);
    let go = |engine: &mut Engine, options: &Options| {
        let mut store = Store::default();
        run_as(&source, "go", BUILT_IN, vec![Value::word(&[1, -2])], vec![manual()], engine, &mut store, options)
    };
    let mut engine = Engine::with(&[("n", Value::int(3))]);
    let full = go(&mut engine, &Options::new(WIDE));
    assert_eq!(full.run.summary.outcome, Outcome::Completed);
    let rows = &full.run.summary.rows;
    let preview = rows.iter().position(|r| r.at.ends_with("#invoke")).expect("the preview's row");
    let through_preview: u128 = rows[..=preview].iter().map(|r| r.steps).sum();

    let interrupted = after(preview + 1);
    let faults = [
        (Options { limits: with(WIDE, Dim::Steps, through_preview), ..Options::new(WIDE) }, None, Outcome::CutOff),
        (Options { cancel: Some(&interrupted), ..Options::new(WIDE) }, None, Outcome::Cancelled),
        (Options::new(WIDE), Some(STALE), Outcome::Error),
    ];
    for (options, refuse_reads, outcome) in faults {
        let mut engine = Engine::with(&[("n", Value::int(3))]);
        engine.refuse_reads = refuse_reads;
        let hash = engine.state_hash();
        let ran = go(&mut engine, &options);
        assert_eq!(ran.run.summary.outcome, outcome, "{:?}", ran.run.summary.code);
        assert!(matches!(ran.shell[0].answer, c600_run::boundary::Answer::Previewed(_)), "{outcome:?}");
        assert_eq!((engine.state_hash(), engine.journal.len()), (hash, 0), "{outcome:?}");
        assert_eq!((engine.previews.len(), engine.published.len()), (1, 0), "{outcome:?}");
    }
}
