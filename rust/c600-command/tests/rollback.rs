//! ADR-D01.15's multi-step rollback (owner, 22 November 2026, item 4; draft; the integrator's ruling of 22 November
//! 2026): `session.rollback {hint, token}` rolls the move history back or forward to the state just after the hint's
//! target action (0: the session's start) as k ordinary undo or redo steps under one gesture. Each step is its own
//! transaction with its own admission and receipt under command `{gesture}#{i}`; the layer holds the writer for the
//! whole gesture, stops at the first step that does not commit and reports j of k. A crash leaves a complete prefix,
//! which a retry finds by LI.30's lookup of the step IDs; there is no all-or-nothing across a crash.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, rollback_step, CommandLayer, Direction, Fate, HeadlessFrames, Invocation, MemoryStore, Receipt,
    Rollback, Secret, Shell, Status, Table, Token, TransactionKind, EXPLORE_ROW, ROLLBACK_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

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

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state with no protected orbit, on H-08's table.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

fn reopen(store: MemoryStore) -> CommandLayer {
    CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), store).unwrap()
}

impl Fixture {
    /// Previews, presents and commits `letters` through the command route; returns its action order.
    fn commit(&mut self, letters: &[i64]) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(&word(letters))).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let args = json::object([("preview", Value::Int(id as i64))]);
        let receipt = self.shell.invoke(&mut self.layer, "operation.execute", args).unwrap();
        assert!(committed(&receipt), "{:?}", receipt.status);
        self.last_order()
    }

    /// Sets the protected orbits, with the solver's confirmation when `confirm`; returns its action order.
    fn protect(&mut self, orbits: &[i64], confirm: bool) -> u64 {
        let orbits = Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect());
        let args = json::object([("confirm", Value::Bool(confirm)), ("orbits", orbits)]);
        let receipt = self.shell.invoke(&mut self.layer, "protection.orbit.set", args).unwrap();
        assert!(receipt.status.code().is_none(), "{:?}", receipt.status);
        self.last_order()
    }

    /// The solver's rollback press to the state just after `target`, with `extra` arguments beside the hint.
    fn roll_with(&mut self, target: u64, extra: &[(&str, Value)]) -> Receipt {
        let hint = self.layer.rollback_hint(target).expect("an eligible target");
        let mut args = json::object([("hint", hint.to_json())]);
        if let Value::Object(fields) = &mut args {
            fields.extend(extra.iter().map(|(name, value)| ((*name).to_owned(), value.clone())));
        }
        self.shell.invoke(&mut self.layer, ROLLBACK_ROW, args).unwrap()
    }

    fn roll(&mut self, target: u64) -> Receipt {
        self.roll_with(target, &[])
    }

    /// The shell's invocation of `row` with `args` under command ID `command`, with a genuine token for it.
    fn renamed(&mut self, row: &str, args: Value, command: &str) -> Invocation {
        let mut invocation = self.shell.prepare(&self.layer, row, args).unwrap();
        invocation.command = command.to_owned();
        let mut binding = invocation.token.take().expect("a commit row takes a token").binding;
        binding.command = command.to_owned();
        invocation.token = Some(Token::mint(&Secret::fixed(SECRET), binding));
        invocation
    }

    fn last_order(&self) -> u64 {
        self.layer.store().log.last().map(|t| t.order).unwrap_or(0)
    }

    fn digest(&self) -> String {
        self.layer.revision().digest.clone()
    }

    /// Nothing changed since `order` and `digest` were read: no action was journaled and the state stayed.
    fn unchanged(&self, order: u64, digest: &str) {
        assert_eq!(self.last_order(), order, "a refusal journaled an action");
        assert_eq!(self.digest(), digest, "a refusal moved the state");
    }

    /// The journal's actions after the first `skip`, as (kind, row, command, target).
    fn steps(&self, skip: usize) -> Vec<(TransactionKind, String, String, Option<u64>)> {
        let log = &self.layer.store().log;
        log.iter().skip(skip).map(|t| (t.kind, t.row.clone(), t.command.clone(), t.target)).collect()
    }
}

/// The state digest after committing each of `words` from the solved state.
fn reached(words: &[&[i64]]) -> String {
    let mut g = fixture();
    for letters in words {
        g.commit(letters);
    }
    g.digest()
}

/// ADR-D01.15: one press rolls the move history back to the state just after an earlier action as k undo steps, each
/// its own journaled undo whose row is `session.rollback` and whose command is the gesture's `{gesture}#{i}`; a press
/// to an action on the redo stack rolls forward as redo steps, and target 0 reaches the session's start. The history
/// dock groups the steps by gesture, and a reopen rebuilds the same place.
#[test]
fn one_press_rolls_back_or_forward_as_k_steps_of_one_gesture() {
    let mut f = fixture();
    let solved = f.digest();
    let a = f.commit(&[1]);
    let b = f.commit(&[2]);
    let c = f.commit(&[3]);
    let d = f.commit(&[4]);
    assert_eq!(f.layer.availability(ROLLBACK_ROW), Ok(()));

    let back = f.roll(a);
    assert!(committed(&back), "{:?}", back.status);
    let gesture = back.command.clone();
    assert_eq!(f.digest(), reached(&[&[1]]));
    let undo =
        |i: u64, target: u64| (TransactionKind::Undo, ROLLBACK_ROW.to_owned(), format!("{gesture}#{i}"), Some(target));
    assert_eq!(f.steps(4), [undo(1, d), undo(2, c), undo(3, b)]);
    let report =
        Rollback { gesture: gesture.clone(), target: a, direction: Direction::Undo, of: 3, landed: 3, stop: None };
    assert_eq!(f.layer.rollback(), Some(&report));
    let steps = f.layer.rollback_steps(&gesture);
    assert_eq!(steps.len(), 3);
    assert!(steps.iter().all(committed), "{steps:?}");
    assert_eq!(back.status, steps[2].status, "the press answers with the last step's receipt");
    for (i, t) in f.layer.store().log.iter().skip(4).enumerate() {
        assert_eq!(rollback_step(&t.command), Some((gesture.as_str(), i as u64 + 1)), "the dock's grouping key");
    }
    assert_eq!(f.layer.rollback_hint(a), None, "the head's own action is no target");

    let forward = f.roll(d);
    assert!(committed(&forward), "{:?}", forward.status);
    assert_eq!(f.digest(), reached(&[&[1], &[2], &[3], &[4]]));
    let gesture = forward.command.clone();
    let redo =
        |i: u64, target: u64| (TransactionKind::Redo, ROLLBACK_ROW.to_owned(), format!("{gesture}#{i}"), Some(target));
    assert_eq!(f.steps(7), [redo(1, b), redo(2, c), redo(3, d)]);

    assert!(committed(&f.roll(0)));
    assert_eq!(f.digest(), solved, "target 0 is the session's start");
    assert_eq!(f.layer.rollback().map(|r| (r.direction, r.of, r.landed)), Some((Direction::Undo, 4, 4)));
    assert_eq!(f.layer.rollback_hint(0), None, "the session's start is the head");

    let g = reopen(f.layer.store().clone());
    assert_eq!(g.recovery(), None);
    assert_eq!(g.revision().digest, solved);
    assert_eq!(g.undo_hint(Direction::Redo).map(|hint| hint.target), Some(a));
}

/// A session with nothing to undo or redo offers no rollback: the row is unavailable with `undo_superseded`, and
/// neither the session's start nor an action that never happened has a hint.
#[test]
fn nothing_to_roll_back_offers_no_target() {
    let mut f = fixture();
    assert_eq!(f.layer.availability(ROLLBACK_ROW).unwrap_err().code, code::UNDO_SUPERSEDED);
    assert_eq!(f.layer.rollback_hint(0), None);
    let a = f.commit(&[1]);
    assert_eq!(f.layer.rollback_hint(a), None);
    assert_eq!(f.layer.rollback_hint(a + 7), None);
    assert!(f.layer.rollback_hint(0).is_some());
}

/// ADR-D01.15 and LI.26: each step takes `session.undo`'s permission path under the protected set in force when it
/// runs, so a step that releases a protected orbit without confirmation is refused, and the gesture stops there with
/// its earlier steps landed and reports j of k. The press answers with the last landed step's receipt; the stopped
/// step's refusal is in the report and in the gesture's lookup.
#[test]
fn the_gesture_stops_at_the_first_refusal_and_reports_j_of_k() {
    let mut f = fixture();
    let solved = f.digest();
    let p1 = f.protect(&[0], false);
    let p2 = f.protect(&[], true);
    let a = f.commit(&[1]);
    assert_eq!((p1, p2, a), (1, 2, 3));

    let press = f.roll(0);
    assert!(committed(&press), "{:?}", press.status);
    let gesture = press.command.clone();
    let report = f.layer.rollback().cloned().unwrap();
    assert_eq!((report.of, report.landed), (3, 2));
    let Some(Status::Refused(error)) = &report.stop else { panic!("{report:?}") };
    assert_eq!(error.code, code::CONFIRMATION_REQUIRED);
    assert_eq!(f.digest(), solved, "the commit and the release's undo landed");
    assert_eq!(f.layer.protected().orbits(), [0], "the third step would release orbit 0");
    let steps = f.layer.rollback_steps(&gesture);
    assert_eq!(steps.len(), 3);
    assert!(committed(&steps[0]) && committed(&steps[1]));
    assert_eq!(code_of(&steps[2]), code::CONFIRMATION_REQUIRED);
    assert_eq!(f.layer.store().log.len(), 5, "the refused step journals nothing");

    // A gesture whose first step is refused lands nothing and answers with the refusal.
    let (order, digest) = (f.last_order(), f.digest());
    let refused = f.roll(0);
    assert_eq!(code_of(&refused), code::CONFIRMATION_REQUIRED);
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((1, 0)));
    f.unchanged(order, &digest);
}

/// LI.26: the press's `confirm` confirms every release in the span, and each step that releases journals it.
#[test]
fn the_press_confirms_every_release_in_its_span() {
    let mut f = fixture();
    f.protect(&[0], false);
    f.protect(&[0, 1], false);
    f.protect(&[], true);
    f.commit(&[1]);
    let press = f.roll_with(0, &[("confirm", Value::Bool(true))]);
    assert!(committed(&press), "{:?}", press.status);
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed, r.stop.is_none())), Some((4, 4, true)));
    assert!(f.layer.protected().orbits().is_empty());
    let confirms: Vec<bool> = f.layer.store().log.iter().skip(4).map(|t| t.confirm).collect();
    assert_eq!(confirms, [true; 4]);
    assert_eq!(reopen(f.layer.store().clone()).recovery(), None, "each release replays with its confirmation");
}

/// ADR-D01.15 and LI.30: a crash inside the gesture leaves a complete prefix of steps. After a restart the gesture's
/// lookup finds the landed prefix, and a step whose fate the store could not tell answers `unknown_outcome` until the
/// store says; a retry is a new gesture to the same target from a fresh hint, which takes only the remaining steps.
#[test]
fn a_crash_leaves_a_complete_prefix_that_a_retry_finds() {
    for written in [false, true] {
        let mut f = fixture();
        let a = f.commit(&[1]);
        f.commit(&[2]);
        f.commit(&[3]);
        f.commit(&[4]);
        f.layer.store_mut().inject.extend([Fate::Durable, Fate::Unknown]);
        f.layer.store_mut().unknown_writes = written;
        let press = f.roll(a);
        assert_eq!(press.status, Status::UnknownOutcome, "written {written}");
        let gesture = press.command.clone();
        assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((3, 1)));
        assert_eq!(f.layer.recovery().map(|e| e.code), Some(code::COMMIT_FATE));

        f.layer.restart(Secret::fixed(SECRET));
        assert_eq!(f.layer.recovery(), None, "written {written}");
        assert_eq!(f.layer.rollback(), None, "a restart clears the report");
        let steps = f.layer.rollback_steps(&gesture);
        let landed = if written { 2 } else { 1 };
        assert_eq!(steps.len(), 2, "written {written}: {steps:?}");
        assert!(steps[..landed].iter().all(committed), "written {written}: {steps:?}");
        if !written {
            assert_eq!(steps[1].status, Status::UnknownOutcome, "a lost transient receipt");
        }
        let prefix: &[&[i64]] = if written { &[&[1], &[2]] } else { &[&[1], &[2], &[3]] };
        assert_eq!(f.digest(), reached(prefix), "written {written}: a complete prefix");

        let retry = f.roll(a);
        assert!(committed(&retry), "written {written}: {:?}", retry.status);
        assert_eq!(f.layer.rollback().map(|r| r.of), Some(3 - landed as u64));
        assert_eq!(f.digest(), reached(&[&[1]]));
        let g = reopen(f.layer.store().clone());
        assert_eq!(g.recovery(), None, "written {written}");
        assert_eq!(g.revision().digest, f.digest());
    }
}

/// B7 for the rollback's steps: a reopen admits a step whose command is `{gesture}#{i}` only as the live gesture writes
/// them, the first step of a gesture or the step after its gesture's step before it in the same direction, and never a
/// step ID an earlier action used; otherwise the replay stops in read-only recovery. A lone first step is an undo
/// any solver press could have made, so it replays.
#[test]
fn a_reopen_admits_steps_only_in_their_gestures_order() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    f.commit(&[4]);
    let gesture = f.roll(a).command;
    let unadmitted = |store: MemoryStore| {
        let g = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), store).unwrap();
        g.recovery().map(|e| e.code)
    };
    let edit = |at: usize, command: &str| {
        let mut store = f.layer.store().clone();
        store.log[at].command = command.to_owned();
        store
    };
    assert_eq!(unadmitted(f.layer.store().clone()), None);
    assert_eq!(unadmitted(edit(5, &format!("{gesture}#3"))), Some(code::UNADMITTED_JOURNAL_ACTION), "a skipped step");
    assert_eq!(unadmitted(edit(5, "other#2")), Some(code::UNADMITTED_JOURNAL_ACTION), "another gesture's step 2");
    assert_eq!(unadmitted(edit(4, "no-step")), Some(code::UNADMITTED_JOURNAL_ACTION), "no gesture and step");
    assert_eq!(unadmitted(edit(4, &format!("{gesture}#0"))), Some(code::UNADMITTED_JOURNAL_ACTION), "step 0");
    assert_eq!(unadmitted(edit(5, &format!("{gesture}#1"))), Some(code::UNADMITTED_JOURNAL_ACTION), "a reused step");
    let mut renamed = edit(5, "other#1");
    renamed.log[6].command = "other#2".into();
    assert_eq!(unadmitted(renamed), None, "a gesture split in two is two gestures");

    // An ordinary undo journaled as a lone first step replays as that step.
    let mut h = fixture();
    h.commit(&[1]);
    h.commit(&[2]);
    let hint = h.layer.undo_hint(Direction::Undo).unwrap();
    assert!(committed(
        &h.shell.invoke(&mut h.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap()
    ));
    let mut store = h.layer.store().clone();
    store.log[2].row = ROLLBACK_ROW.to_owned();
    store.log[2].command = "lone#1".into();
    assert_eq!(unadmitted(store.clone()), None);
    store.log[2].command = "lone#2".into();
    assert_eq!(unadmitted(store), Some(code::UNADMITTED_JOURNAL_ACTION));
}

/// The token binds the hint and the revisions as `session.undo`'s does: a hint whose order changed is
/// `undo_superseded` and commits nothing.
#[test]
fn a_stale_hint_rolls_back_nothing() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    let hint = f.layer.rollback_hint(a).unwrap();
    f.commit(&[3]);
    let (order, digest) = (f.last_order(), f.digest());
    let press = f.shell.invoke(&mut f.layer, ROLLBACK_ROW, json::object([("hint", hint.to_json())])).unwrap();
    assert_eq!(code_of(&press), code::UNDO_SUPERSEDED);
    assert_eq!(f.layer.rollback(), None);
    f.unchanged(order, &digest);
}

/// Only the layer names a rollback's gesture and steps: a press that carries `gesture`, `step` or `of` is refused
/// with `argument_form` before its token is read. A press whose command ID would make a step ID an earlier action
/// used is refused with `command_id_reused`. Neither changes anything.
#[test]
fn a_press_names_no_gesture_or_step_and_reuses_no_step_id() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let (order, digest) = (f.last_order(), f.digest());
    for (name, value) in [("gesture", Value::String("g".into())), ("step", Value::Int(1)), ("of", Value::Int(2))] {
        let press = f.roll_with(a, &[(name, value)]);
        assert_eq!(code_of(&press), code::ARGUMENT_FORM, "{name}");
        f.unchanged(order, &digest);
    }

    let hint = f.layer.undo_hint(Direction::Undo).unwrap();
    let undo = f.renamed("session.undo", json::object([("hint", hint.to_json())]), "g#1");
    assert!(committed(&f.shell.send(&mut f.layer, undo)));
    let (order, digest) = (f.last_order(), f.digest());
    let hint = f.layer.rollback_hint(a).unwrap();
    let press = f.renamed(ROLLBACK_ROW, json::object([("hint", hint.to_json())]), "g");
    assert_eq!(code_of(&f.shell.send(&mut f.layer, press)), code::COMMAND_ID_REUSED);
    f.unchanged(order, &digest);
}

/// The explore state pauses the rollback as every commit row but the layout history's (ADR-D01.39), before its token
/// is spent.
#[test]
fn the_explore_state_pauses_a_rollback() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    let on = f.shell.invoke(&mut f.layer, EXPLORE_ROW, json::object([("on", Value::Bool(true))])).unwrap();
    assert!(on.status.code().is_none(), "{:?}", on.status);
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.roll(a)), code::COMMITS_PAUSED);
    f.unchanged(order, &digest);
}

/// The integrator's ruling, point 3: the layer holds the writer for the whole gesture, so each step's admission is
/// based on its own pre-step state, the steps take consecutive orders, and a press another writer prepared before the
/// gesture is refused as stale afterwards and folds nothing into it.
#[test]
fn an_interleaved_writer_is_refused_as_stale() {
    let mut f = fixture();
    f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let hint = f.layer.undo_hint(Direction::Undo).unwrap();
    let other = f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    let press = f.roll(0);
    assert!(committed(&press), "{:?}", press.status);
    let log = &f.layer.store().log;
    assert_eq!(log.len(), 6);
    for pair in log[3..].windows(2) {
        assert_eq!(pair[1].pre, pair[0].post, "a step is based on the step before it");
        assert_eq!(pair[1].order, pair[0].order + 1, "no action between two steps");
    }
    assert_eq!(log[3].pre, log[2].post, "the first step is based on the head the press saw");

    let (order, digest) = (f.last_order(), f.digest());
    let late = f.shell.send(&mut f.layer, other);
    assert_eq!(code_of(&late), code::UNDO_SUPERSEDED);
    f.unchanged(order, &digest);
}

/// LI.30's retry of the same envelope: the press sent again with the same command ID and request answers the same
/// receipt and takes no step, and each step's lookup by command and request answers its own receipt.
#[test]
fn a_retried_press_answers_the_same_receipts() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let hint = f.layer.rollback_hint(a).unwrap();
    let press = f.shell.prepare(&f.layer, ROLLBACK_ROW, json::object([("hint", hint.to_json())])).unwrap();
    let first = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&first), "{:?}", first.status);
    let steps = f.layer.rollback_steps(&first.command);
    assert_eq!(steps.len(), 2);
    let (order, digest) = (f.last_order(), f.digest());

    let again = f.shell.send(&mut f.layer, press);
    assert_eq!(again, first);
    f.unchanged(order, &digest);
    assert_eq!(f.layer.rollback_steps(&first.command), steps);
    for step in &steps {
        assert_eq!(f.layer.outcome(&step.command, &step.request).as_ref(), Some(step));
    }
}

/// Redo after a gesture that stopped at j of k: the landed steps are ordinary undos, so a forward rollback redoes
/// exactly them, with the solver's confirmation for the release, and reaches the state before the gesture; a later
/// `session.redo` has nothing left.
#[test]
fn a_partial_rollback_is_redone_step_by_step() {
    let mut f = fixture();
    f.protect(&[0], false);
    let released = f.protect(&[], true);
    let a = f.commit(&[1]);
    let before = f.digest();
    let press = f.roll(0);
    assert!(committed(&press), "{:?}", press.status);
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((3, 2)));
    assert_eq!(f.layer.undo_hint(Direction::Redo).map(|hint| hint.target), Some(released));

    let forward = f.roll_with(a, &[("confirm", Value::Bool(true))]);
    assert!(committed(&forward), "{:?}", forward.status);
    assert_eq!(f.layer.rollback().map(|r| (r.direction, r.of, r.landed)), Some((Direction::Redo, 2, 2)));
    assert_eq!(f.digest(), before);
    assert!(f.layer.protected().orbits().is_empty());
    assert_eq!(f.layer.undo_hint(Direction::Redo), None);
    assert_eq!(reopen(f.layer.store().clone()).recovery(), None);
}
