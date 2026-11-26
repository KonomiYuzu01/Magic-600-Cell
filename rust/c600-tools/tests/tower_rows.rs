//! The identity tower's kernel rows on 600-cell-Full's host (T2; T1 drafts until the IR freeze, PHS.49): programs of
//! hand-built IR run on `c600-run` against the bridge ([`Bridge`], `c600_tools::bridge::tower`) onto a command layer,
//! whose value store keeps the derived values (`c600_command::values`). The interpreter's halves of the same rows run
//! against a fake host in `c600-run/tests/tower.rs`; the jumbling host's in `c600-jumble-tool/tests/tower_rows.rs`.
//! Source and fixture evidence only.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::values::{BaseKey, Recipe, Source, ValueKey};
use c600_command::{code, CommandLayer, Secret, Table};
use c600_ir::build::{case, exact, field, helper, kernel, let_, module, nat, pat, program, read_snapshot, var, word};
use c600_ir::{Expr, Goal, KernelOp, Level, Program, Puzzle, QTy, Ty};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, ORBITS};
use c600_run::interp::{load, Host, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome, Row};
use c600_run::tower::HandleTy;
use c600_run::value::Value;
use c600_tools::bridge::tower::{RESULT_BOUND, STATE_SIZE};
use c600_tools::bridge::Bridge;

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// Two fixed words of 600-cell-Full's letters.
const W1: [i32; 6] = [17, -305, 911, 4, -1200, 600];
const W2: [i32; 4] = [33, -88, 1021, -7];

const FULL: Puzzle = Puzzle::Full;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel_model() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL.get_or_init(|| Arc::new(Kernel::load(&repository().join("assets")).expect("the model loads"))).clone()
}

/// A session at solved with `orbits` protected.
fn layer(orbits: &[i64]) -> CommandLayer {
    let k = kernel_model();
    let protected = k.protected_set(orbits).expect("orbits of the model");
    CommandLayer::new(
        k.clone(),
        Table::h08(),
        Secret::fixed([5; 32]),
        Default::default(),
        "tower",
        k.solved(),
        protected,
    )
}

/// The kernel's own derivation of `letters` from `base` with no protected orbit.
fn derived_state(base: &Snapshot, letters: &[i32]) -> Snapshot {
    let k = kernel_model();
    let wide: Vec<i64> = letters.iter().map(|&l| i64::from(l)).collect();
    let recipe = k.word(&wide).expect("a word");
    match k.derive(base, &recipe, &ProtectedSet::none()).expect("no certificate fails") {
        Derivation::Derived(d) => d.state,
        Derivation::Refused(r) => panic!("{}", r.refusal),
    }
}

fn q(ty: Ty) -> QTy {
    exact(ty)
}

fn snapshot() -> Expr {
    kernel(KernelOp::Snapshot(FULL), vec![], q(Ty::State(FULL)))
}

fn derive(s: Expr, w: Expr) -> Expr {
    kernel(KernelOp::Derive, vec![s, w], q(Ty::Derivation(FULL)))
}

fn project(s: Expr, level: Level) -> Expr {
    kernel(KernelOp::Project(level), vec![s], q(Ty::Partial(Box::new(q(Ty::LevelValue(FULL, level))))))
}

fn digest(v: Expr, level: Level) -> Expr {
    kernel(KernelOp::Digest, vec![v], q(Ty::Digest(FULL, level)))
}

fn compare(a: Expr, b: Expr, level: Level) -> Expr {
    kernel(KernelOp::Compare(level), vec![a, b], q(Ty::Partial(Box::new(q(Ty::Verdict(FULL, level, None))))))
}

/// `then` with the derived state of `d` bound to `name`; a refusal stops the run.
fn derived(d: Expr, name: &str, then: Expr) -> Expr {
    case(d, vec![(pat("derived", &[name]), then)], None)
}

/// `then` with the total value of the partial result `p` bound to `name`.
fn total(p: Expr, name: &str, then: Expr) -> Expr {
    case(p, vec![(pat("total", &[name]), then)], None)
}

/// The digest at `level` of the state `s`.
fn state_digest(s: Expr, level: Level) -> Expr {
    total(project(s, level), "v", digest(var("v"), level))
}

/// The yes/no/unknown of a verdict as 1, 0 or 2.
fn decided(verdict: Expr) -> Expr {
    case(
        verdict,
        vec![(pat("yes", &["_"]), nat(1)), (pat("no", &["_"]), nat(0)), (pat("unknown", &["_"]), nat(2))],
        None,
    )
}

fn prog(result: Ty, body: Expr) -> Program {
    program(vec![module("m", vec![helper("f", vec![], q(result), body).exported()])])
}

fn run_on(host: &mut dyn Host, result: Ty, body: Expr) -> Run {
    load(&prog(result, body)).expect("the program loads").run("m", "f", vec![], host, &Options::new(WIDE)).unwrap()
}

fn run(layer: &mut CommandLayer, result: Ty, body: Expr) -> Run {
    let mut bridge = Bridge::new(layer, "tower");
    run_on(&mut bridge, result, body)
}

fn completed(run: &Run) {
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?} {:?}", run.summary.code, run.summary.stop);
}

fn nat_of(run: &Run) -> u64 {
    match run.value.as_ref() {
        Some(Value::Int(n)) => u64::try_from(&**n).expect("a small natural"),
        other => panic!("{other:?}: {:?}", run.summary.code),
    }
}

fn kernel_rows(run: &Run) -> Vec<&Row> {
    run.summary.rows.iter().filter(|row| row.at.ends_with("#kernel")).collect()
}

/// CT-LG2-07 (part: the run-time half on 600-cell-Full's host; not run: the jumbling host's import, in
/// `c600-jumble-tool/tests/tower_rows.rs`, and the source half, T1b). CT-LG2-02 (part: 600-cell-Full's host's half: the
/// jumbling puzzle's snapshot and import are not granted; not run: the jumbling host's half, in
/// `c600-jumble-tool/tests/tower_rows.rs`, and the source half, T1b). A derived state keeps its base's identity and
/// level: `derive` answers a `State<Full>` handle that projects totally at the labels and the position and compares
/// with its base at the same level, and its labels digest is the kernel's derivation of the same literal letters. The
/// host refuses `import`, the jumbling puzzle's row (KN2.14), and the jumbling snapshot, as not granted.
#[test]
fn ct_lg2_07_a_full_derived_state_keeps_its_base_identity_and_level() {
    let mut layer = layer(&[]);
    let want = derived_state(&kernel_model().solved(), &W1);
    for level in [Level::Labels, Level::Position] {
        let body = derived(derive(snapshot(), word(&W1)), "s", state_digest(var("s"), level));
        let r = run(&mut layer, Ty::Digest(FULL, level), body);
        completed(&r);
        let out = String::from_utf8(r.output.clone().unwrap()).unwrap();
        assert!(
            out.contains(&format!("\"level\":\"{}\"", level.name())) && out.contains("\"puzzle\":\"Full\""),
            "{out}"
        );
        if level == Level::Labels {
            assert!(out.contains(want.digest()), "the kernel's derivation of the same letters: {out}");
        }
    }
    // The derived state against its base and against a second derivation of the same word, at both levels.
    for level in [Level::Labels, Level::Position] {
        let differs = derived(
            derive(snapshot(), word(&W1)),
            "s",
            total(compare(snapshot(), var("s"), level), "v", decided(var("v"))),
        );
        let r = run(&mut layer, Ty::Nat, differs);
        completed(&r);
        assert_eq!(nat_of(&r), 0, "{level:?}: the word moves the state");
        let same = let_(
            "a",
            derive(snapshot(), word(&W1)),
            let_(
                "b",
                derive(snapshot(), word(&W1)),
                derived(
                    var("a"),
                    "x",
                    derived(var("b"), "y", total(compare(var("x"), var("y"), level), "v", decided(var("v")))),
                ),
            ),
        );
        let r = run(&mut layer, Ty::Nat, same);
        completed(&r);
        assert_eq!(nat_of(&r), 1, "{level:?}");
    }
    // The jumbling puzzle's rows are not granted on this host.
    let mut bridge = Bridge::new(&mut layer, "tower");
    let ty = q(Ty::State(Puzzle::Jumbling));
    for op in [KernelOp::Import, KernelOp::Snapshot(Puzzle::Jumbling)] {
        let request = Request::Kernel { op, args: &[], ty: &ty };
        assert_eq!(bridge.tariff(&request).err(), Some(NOT_GRANTED), "{op:?}");
    }
}

/// CT-LG2-11 (part: the run-time half on 600-cell-Full's host; not run: the jumbling host's, in
/// `c600-jumble-tool/tests/tower_rows.rs`, and the source half, T1b). `derive` takes a Full word's literal letters and
/// derives them under the session's protection: with every orbit protected the word is refused as a result,
/// `refused({ prefix, outcome, code })` with the whole word's length under Net, `KN2:protection.orbit_would_move`, and
/// no value is kept; the same word with no protected orbit derives. Its tariff is the `replay` row's for its letters.
#[test]
fn ct_lg2_11_a_full_derivation_takes_literal_letters_and_refuses_as_a_result() {
    let all: Vec<i64> = (0..ORBITS as i64).collect();
    let mut protected = layer(&all);
    let refused =
        Ty::Record(vec![("prefix".into(), q(Ty::Nat)), ("outcome".into(), q(Ty::Text)), ("code".into(), q(Ty::Text))]);
    let body = case(derive(snapshot(), word(&W1)), vec![(pat("refused", &["r"]), var("r"))], None);
    let r = run(&mut protected, refused.clone(), body);
    completed(&r);
    let out = String::from_utf8(r.output.clone().unwrap()).unwrap();
    assert!(out.contains("KN2:protection.orbit_would_move"), "{out}");
    assert!(out.contains("\"outcome\":\"protected\""), "{out}");
    assert!(out.contains(&format!("\"prefix\":{{\"int\":\"{}\"}}", W1.len())), "{out}");
    assert_eq!(protected.values().len(), 0, "a refusal keeps no value");
    let prefix = case(
        derive(snapshot(), word(&W1)),
        vec![(pat("refused", &["r"]), field(var("r"), "prefix")), (pat("derived", &["_"]), nat(0))],
        None,
    );
    let r = run(&mut protected, Ty::Nat, prefix.clone());
    assert_eq!(nat_of(&r), W1.len() as u64);
    let derive_row = kernel_rows(&r)[1];
    let replay = c600_tools::kernel_rows::replay_tariff(W1.len());
    assert_eq!((derive_row.kernel_cost, derive_row.memory), (replay.kernel_cost, RESULT_BOUND));

    let mut open = layer(&[]);
    let r = run(&mut open, Ty::Nat, prefix);
    assert_eq!(nat_of(&r), 0, "derived with no protected orbit");
    assert_eq!(open.values().len(), 1);
}

/// CT-LG2-09 and CT-LG2-10 (part: 600-cell-Full's host's half of the verdicts; not run: the jumbling host's
/// goals and off-lattice reads, in `c600-jumble-tool/tests/tower_rows.rs`, and the source half, T1b). 600-cell-Full
/// holds the sticker test at the labels: `check` answers `yes` with its certificate at solved and `no` on a derived
/// scrambled state, never `unknown`; the verdict carries the labels level, which the interpreter checks against the
/// declared type.
#[test]
fn ct_lg2_10_the_full_sticker_test_answers_at_the_labels() {
    let mut layer = layer(&[]);
    let check = |s: Expr| {
        let ty = q(Ty::Verdict(FULL, Level::Labels, Some(Goal::StickerTest)));
        decided(kernel(KernelOp::Check(Goal::StickerTest), vec![s], ty))
    };
    let r = run(&mut layer, Ty::Nat, check(snapshot()));
    completed(&r);
    assert_eq!(nat_of(&r), 1, "solved");
    let r = run(&mut layer, Ty::Nat, derived(derive(snapshot(), word(&W1)), "s", check(var("s"))));
    completed(&r);
    assert_eq!(nat_of(&r), 0, "scrambled");
    // The reachability claim of a revision and of a derived state is witnessed.
    let claim = |s: Expr| {
        let reach = kernel(KernelOp::Reachability, vec![s], q(Ty::Reachability(FULL)));
        let arms = ["by_theorem", "promised", "refuted", "open"].map(|ctor| (pat(ctor, &["_"]), nat(0)));
        let mut arms = arms.to_vec();
        arms.insert(0, (pat("witnessed", &["_"]), nat(1)));
        case(reach, arms, None)
    };
    assert_eq!(nat_of(&run(&mut layer, Ty::Nat, claim(snapshot()))), 1);
    let from_derived = derived(derive(snapshot(), word(&W1)), "s", claim(var("s")));
    assert_eq!(nat_of(&run(&mut layer, Ty::Nat, from_derived)), 1);
}

/// A host that evicts the layer's every stored value before each kernel row it performs.
struct Evicting<'b, 'l> {
    bridge: Bridge<'l>,
    evictions: &'b mut usize,
}

impl Host for Evicting<'_, '_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.bridge.tariff(request)
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        *self.evictions += self.bridge.values().evict(None);
        self.bridge.perform(request)
    }
    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.bridge.sink(sink, value, bytes)
    }
}

/// The chained program of CT-DET-08: W2 derived from W1's derived state, its labels digest, with a second alias of W1's
/// derived state held across it.
fn chained() -> Expr {
    let s1 = || derive(snapshot(), word(&W1));
    let_(
        "held",
        s1(),
        derived(s1(), "a", derived(derive(var("a"), word(&W2)), "b", state_digest(var("b"), Level::Labels))),
    )
}

/// What CT-DET-08 compares: the result bytes, every charge row and the run's used amounts.
fn observed(run: &Run) -> (Option<Vec<u8>>, Vec<Row>, Amounts) {
    completed(run);
    (run.output.clone(), run.summary.rows.clone(), run.summary.used)
}

/// CT-DET-08 (part: the command layer's value store on 600-cell-Full's host; not run: the jumbling host's, in
/// `c600-jumble-tool/tests/tower_rows.rs`, and a host row-answer cache, which 1.0's host does not keep (owner, 20
/// November 2026, item 1)). The same program over a cold store, a warm store, a store whose values were evicted, a
/// store evicted before every kernel row (each base recomputed from its stored derivation), a store where another
/// run holds the same values, and a fresh session: the result bytes, every declared charge and the used amounts are
/// equal. Two live aliases of one stored value are each charged their full logical size (DET.24).
#[test]
fn ct_det_08_full_values_cold_warm_evicted_and_shared_charge_alike() {
    let mut first = layer(&[]);
    let cold = observed(&run(&mut first, Ty::Digest(FULL, Level::Labels), chained()));
    let want = derived_state(&derived_state(&kernel_model().solved(), &W1), &W2);
    assert!(String::from_utf8(cold.0.clone().unwrap()).unwrap().contains(want.digest()));
    assert_eq!(first.values().counts().misses, 2, "W1's and W2's values");

    let warm = observed(&run(&mut first, Ty::Digest(FULL, Level::Labels), chained()));
    assert!(first.values().counts().hits > 0, "a warm store hits");
    assert_eq!(warm, cold, "warm");

    assert_eq!(first.values_mut().evict(None), 2);
    let evicted = observed(&run(&mut first, Ty::Digest(FULL, Level::Labels), chained()));
    assert!(first.values().counts().recomputed > 0, "the evicted values are recomputed");
    assert_eq!(evicted, cold, "evicted");

    let mut evictions = 0;
    let every = {
        let mut host = Evicting { bridge: Bridge::new(&mut first, "evicting"), evictions: &mut evictions };
        observed(&run_on(&mut host, Ty::Digest(FULL, Level::Labels), chained()))
    };
    assert!(evictions > 0, "values were evicted between the rows");
    assert_eq!(every, cold, "evicted before every row");

    // Another run holds the same value through its handle, in the same epoch's store.
    let holder = derived(derive(snapshot(), word(&W1)), "s", nat(1));
    assert_eq!(nat_of(&run(&mut first, Ty::Nat, holder)), 1);
    let shared = observed(&run(&mut first, Ty::Digest(FULL, Level::Labels), chained()));
    assert_eq!(shared, cold, "shared");

    let mut second = layer(&[]);
    assert_eq!(observed(&run(&mut second, Ty::Digest(FULL, Level::Labels), chained())), cold, "a fresh session");

    // Each alias of W1's value is charged in full when it is admitted (DET.24): the two derive rows from the snapshot,
    // a miss and a hit, charge alike, and each state a case binds is charged the state's logical size.
    let w1 = c600_tools::kernel_rows::replay_tariff(W1.len()).kernel_cost;
    let derives: Vec<&Row> = cold.1.iter().filter(|row| row.at.ends_with("#kernel") && row.kernel_cost == w1).collect();
    assert_eq!(derives.len(), 2);
    let charge = |row: &Row| (row.steps, row.kernel_cost, row.memory, row.output, row.released);
    assert_eq!(charge(derives[0]), charge(derives[1]), "two aliases of W1's value");
    let branches: Vec<u128> = cold.1.iter().filter(|row| row.at.ends_with("#branch")).map(|row| row.memory).collect();
    assert_eq!(branches, [STATE_SIZE; 3], "each derived state and level value bound is charged its logical size");
}

/// A host that answers the snapshot read `old` with a handle kept from before a restart.
struct Old<'l> {
    bridge: Bridge<'l>,
    old: Value,
}

impl Host for Old<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Snapshot { query: "old", .. } => Ok(RowTariff { kernel_cost: 0, result_bound: RESULT_BOUND }),
            _ => self.bridge.tariff(request),
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query: "old", .. } => Ok(self.old.clone()),
            _ => self.bridge.perform(request),
        }
    }
    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.bridge.sink(sink, value, bytes)
    }
}

/// A host that keeps every kernel row's answer.
struct Keeping<'l> {
    bridge: Bridge<'l>,
    kept: Vec<Value>,
}

impl Host for Keeping<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.bridge.tariff(request)
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let answer = self.bridge.perform(request)?;
        if matches!(request, Request::Kernel { .. }) {
            self.kept.push(answer.clone());
        }
        Ok(answer)
    }
    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.bridge.sink(sink, value, bytes)
    }
}

/// CT-ONE-21 (part: the value store's half, ONE.70's "handles and derived values" on 600-cell-Full's host; not run:
/// the session store's half, in `c600-session/tests/conformance_recovery.rs`, the layer's previews, drafts,
/// receipts and permissions, in `c600-command/tests/layer.rs` and `kn5_stages.rs`, and the jumbling host's, in
/// `c600-jumble-tool/tests/tower_rows.rs`). After the command layer's restart the value store is a new epoch's, empty:
/// a handle kept from the old epoch is refused with `KN2:stale.epoch_changed` at the row's tariff, the derived value is
/// gone, and its stored derivation, whose base names the revision and not a handle, recomputes the same digest under
/// the new epoch, as a new run's derivation does. A chained derivation's stored recipe carries its base's, so it
/// recomputes from the revision too.
#[test]
fn ct_one_21_derived_values_end_at_a_restart_and_their_derivations_recompute() {
    let mut layer = layer(&[]);
    let body = || derived(derive(snapshot(), word(&W1)), "s", state_digest(var("s"), Level::Labels));
    let (before, kept) = {
        let mut host = Keeping { bridge: Bridge::new(&mut layer, "before"), kept: Vec::new() };
        let r = run_on(&mut host, Ty::Digest(FULL, Level::Labels), body());
        completed(&r);
        (r.output.unwrap(), host.kept)
    };
    let Value::Sum(derivation) = &kept[1] else { panic!("a derivation: {:?}", kept[1]) };
    let handle = derivation.payload()[0].clone();
    let Value::Handle(h) = &handle else { panic!("a state handle") };
    assert_eq!((h.ty(), h.id() >> 32, h.size()), (HandleTy::State(FULL), 1, STATE_SIZE));
    let key = layer.values().keys().next().cloned().expect("the derived value's entry");
    assert!(layer.values().holds(&key));
    let recipe = layer.values().recipe(&key).cloned().expect("its stored derivation");
    let revision = layer.revision().clone();
    match (&key, &recipe) {
        (ValueKey::Derived { base: BaseKey::Revision { head, digest, .. }, .. }, Recipe::Derive { base, .. }) => {
            assert_eq!((head, digest), (&revision.head, &revision.digest), "the base is named by its revision");
            assert!(matches!(base, Source::Root(BaseKey::Revision { .. })));
        }
        other => panic!("{other:?}"),
    }
    // A chained value's recipe, kept before the restart.
    let chain = derived(
        derive(snapshot(), word(&W1)),
        "a",
        derived(derive(var("a"), word(&W2)), "b", state_digest(var("b"), Level::Labels)),
    );
    let chain_out = {
        let r = run(&mut layer, Ty::Digest(FULL, Level::Labels), chain);
        completed(&r);
        r.output.unwrap()
    };
    let chain_key = layer.values().keys().find(|k| matches!(k, ValueKey::Derived { base: BaseKey::Digest(_), .. }));
    let chain_recipe = layer.values().recipe(chain_key.expect("W2's entry")).cloned().unwrap();

    layer.restart(Secret::fixed([6; 32]));
    assert_ne!(layer.revision().epoch, revision.epoch, "a new epoch");
    assert!(layer.values().is_empty(), "the value store is not durable");
    assert_eq!(layer.values().ordinal(), 2);
    assert!(!layer.values().holds(&key), "the derived value is gone");

    // The old handle refuses at the row's tariff.
    let read_old = project(read_snapshot("old", q(Ty::State(FULL))), Level::Labels);
    let r = {
        let mut host = Old { bridge: Bridge::new(&mut layer, "after"), old: handle };
        run_on(&mut host, Ty::Nat, let_("p", read_old, nat(0)))
    };
    assert_eq!(r.summary.code, Some(code::EPOCH_CHANGED));
    assert_eq!(kernel_rows(&r).last().map(|row| (row.steps, row.kernel_cost, row.memory)), Some((1, 0, 0)));

    // The stored derivations recompute the same digests under the new epoch.
    let mut bridge = Bridge::new(&mut layer, "recompute");
    let digest = bridge.recompute(&recipe).expect("the revision is still the head");
    assert!(String::from_utf8(before.clone()).unwrap().contains(&digest));
    let chained_digest = bridge.recompute(&chain_recipe).expect("the chain leads back to the revision");
    assert!(String::from_utf8(chain_out).unwrap().contains(&chained_digest));
    drop(bridge);
    let r = run(&mut layer, Ty::Digest(FULL, Level::Labels), body());
    completed(&r);
    assert_eq!(r.output.unwrap(), before, "a new run's derivation in the new epoch");

    // A stored derivation whose revision is no longer the head does not resolve.
    let committed = derived_state(layer.snapshot(), &W2);
    assert_ne!(committed.digest(), layer.snapshot().digest());
    let mut moved: CommandLayer = CommandLayer::new(
        kernel_model(),
        Table::h08(),
        Secret::fixed([7; 32]),
        Default::default(),
        "tower",
        committed,
        kernel_model().protected_set(&[]).unwrap(),
    );
    let refused = Bridge::new(&mut moved, "elsewhere").recompute(&recipe);
    assert_eq!(refused, Err(c600_tools::bridge::tower::VALUE_UNAVAILABLE));
}
