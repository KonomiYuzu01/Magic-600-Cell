//! Attack tests on ADR-D01's row 2, the multi-step rollback `session.rollback` (ADR-D01.15). They come from the
//! fable-solver's attack on 6d32e6bf: the leads R2-1 to R2-4 and R2-7 to R2-9 were confirmed and are closed here; R2-6
//! and R2-10 are readings for the integrator, and their tests pin the behaviour as it stands; R2-5 was a reading too,
//! and the owner's decision 4.2 of 23 November 2026 closed it with the step ID `{gesture}#{i}/{k}` (LP4-15), so its
//! test now passes on the fix; the other tests are the attacker's refuted paths, kept as guards.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, rollback_step, Caller, CommandLayer, Completion, CompletionKind, Direction, DisplayStatus, Error,
    Fate, HeadlessFrames, Invocation, JobOutput, MemoryStore, Overlay, Provenance, Receipt, RecordValue, Refresh,
    Revision, Secret, Shell, Status, Table, Token, LAYOUT_UNDO_ROW, RECORD_ROW, ROLLBACK_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const THEME: &str = "theme.select";

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

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
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

struct Reader;

impl Provenance for Reader {
    fn check(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }
    fn admit(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

fn serve(layer: &mut CommandLayer) {
    layer.serve_row(THEME, Overlay { module: "tools/design".into(), entry: "theme".into() }).unwrap();
}

fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    serve(&mut layer);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

fn open(store: MemoryStore) -> CommandLayer {
    let mut g = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    g.set_provenance(Box::new(Reader));
    serve(&mut g);
    g
}

fn recovery(store: MemoryStore) -> Option<&'static str> {
    open(store).recovery().map(|e| e.code)
}

impl Fixture {
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

    fn protect(&mut self, orbits: &[i64], confirm: bool) -> u64 {
        let orbits = Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect());
        let args = json::object([("confirm", Value::Bool(confirm)), ("orbits", orbits)]);
        let receipt = self.shell.invoke(&mut self.layer, "protection.orbit.set", args).unwrap();
        assert!(receipt.status.code().is_none(), "{:?}", receipt.status);
        self.last_order()
    }

    fn write(&mut self, name: &str, payload: &str) -> u64 {
        let args = json::object([("payload", text(payload)), ("provenance", text("block")), ("record", text(name))]);
        let receipt = self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap();
        assert!(receipt.status.code().is_none(), "{:?}", receipt.status);
        self.last_order()
    }

    fn theme(&mut self, name: &str, payload: &str) -> u64 {
        let started = self.shell.invoke(&mut self.layer, THEME, json::object([("family", text("dawn"))])).unwrap();
        let Status::Running(job) = started.status else { panic!("not admitted: {:?}", started.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job));
        let value = RecordValue { payload: text(payload), provenance: "block".into() };
        let records = [(name.to_owned(), value)].into_iter().collect();
        let output = JobOutput { value: Value::Null, views: Vec::new(), records };
        let done = self.layer.finish_program_job_with(job, Ok(output)).unwrap();
        assert!(matches!(done.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
        self.last_order()
    }

    fn roll_args(&self, target: u64, extra: &[(&str, Value)]) -> Value {
        let hint = self.layer.rollback_hint(target).expect("an eligible target");
        let mut args = json::object([("hint", hint.to_json())]);
        if let Value::Object(fields) = &mut args {
            fields.extend(extra.iter().map(|(name, value)| ((*name).to_owned(), value.clone())));
        }
        args
    }

    fn roll_with(&mut self, target: u64, extra: &[(&str, Value)]) -> Receipt {
        let args = self.roll_args(target, extra);
        self.shell.invoke(&mut self.layer, ROLLBACK_ROW, args).unwrap()
    }

    fn roll(&mut self, target: u64) -> Receipt {
        self.roll_with(target, &[])
    }

    fn renamed(&mut self, row: &str, args: Value, command: &str) -> Invocation {
        let mut invocation = self.shell.prepare(&self.layer, row, args).unwrap();
        invocation.command = command.to_owned();
        let mut binding = invocation.token.take().expect("a commit row takes a token").binding;
        binding.command = command.to_owned();
        invocation.token = Some(Token::mint(&Secret::fixed(SECRET), binding));
        invocation
    }

    fn roll_as(&mut self, target: u64, command: &str) -> Receipt {
        let args = self.roll_args(target, &[]);
        let press = self.renamed(ROLLBACK_ROW, args, command);
        self.shell.send(&mut self.layer, press)
    }

    fn undo_as(&mut self, command: &str) -> Receipt {
        let hint = self.layer.undo_hint(Direction::Undo).unwrap();
        let undo = self.renamed("session.undo", json::object([("hint", hint.to_json())]), command);
        self.shell.send(&mut self.layer, undo)
    }

    fn last_order(&self) -> u64 {
        self.layer.store().log.last().map(|t| t.order).unwrap_or(0)
    }

    fn digest(&self) -> String {
        self.layer.revision().digest.clone()
    }
}

// ----- Closed findings -----

/// R2-1: a press under an empty command ID would land steps `#1/2`, `#2/2`, which name no gesture, so a reopen would
/// refuse the journal the session wrote. The press is refused, and nothing lands.
#[test]
fn an_empty_gesture_id_is_refused() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(rollback_step("#1/1"), None);
    assert_eq!(code_of(&f.roll_as(a, "")), code::ARGUMENT_FORM);
    assert_eq!((f.last_order(), f.digest()), (order, digest));
    assert!(committed(&f.roll(a)));
    assert_eq!(recovery(f.layer.store().clone()), None);
}

/// R2-2: a crafted journal with a step number at `u64::MAX` opens in read-only recovery and never panics.
#[test]
fn a_crafted_step_number_opens_in_recovery() {
    let mut f = fixture();
    f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    assert!(committed(&f.undo_as("u1")));
    assert!(committed(&f.undo_as("u2")));
    let unadmitted = Some(Some(code::UNADMITTED_JOURNAL_ACTION));
    // The attacker's journal: a commit under the rollback row as step `u64::MAX`, an undo as step 2.
    let max = u64::MAX;
    let mut store = f.layer.store().clone();
    store.log[1].row = ROLLBACK_ROW.to_owned();
    store.log[1].command = format!("x#{max}/{max}");
    store.log[3].row = ROLLBACK_ROW.to_owned();
    store.log[3].command = format!("x#2/{max}");
    assert_eq!(catch_unwind(AssertUnwindSafe(|| recovery(store))).ok(), unadmitted);
    // Two undos as steps 1 and `u64::MAX` of one gesture of `u64::MAX` steps.
    let mut store = f.layer.store().clone();
    for (at, step) in [(3, 1), (4, max)] {
        store.log[at].row = ROLLBACK_ROW.to_owned();
        store.log[at].command = format!("x#{step}/{max}");
    }
    assert_eq!(catch_unwind(AssertUnwindSafe(|| recovery(store))).ok(), unadmitted);
    // A lone first step of a gesture of `u64::MAX` steps replays, and its lookup stops at the steps that exist.
    let mut store = f.layer.store().clone();
    store.log[4].row = ROLLBACK_ROW.to_owned();
    store.log[4].command = format!("x#1/{max}");
    let g = catch_unwind(AssertUnwindSafe(|| open(store))).expect("no panic");
    assert_eq!(g.recovery(), None);
    assert_eq!(g.rollback_steps("x").len(), 1);
}

/// R2-3: the rollback row journals only undo and redo steps. A commit, a protection change and a record write
/// journaled under `session.rollback` do not replay.
#[test]
fn the_rollback_row_replays_only_undo_and_redo_steps() {
    let mut f = fixture();
    f.commit(&[1]);
    f.protect(&[0], false);
    f.write("r", "v");
    for at in 0..3 {
        let mut store = f.layer.store().clone();
        store.log[at].row = ROLLBACK_ROW.to_owned();
        store.log[at].command = format!("g{at}#1/1");
        assert_eq!(recovery(store), Some(code::UNADMITTED_JOURNAL_ACTION), "{:?}", f.layer.store().log[at].kind);
    }
}

/// R2-4: LI.30's lookup of a gesture's steps reads only the rollback steps the layer wrote: neither an ordinary undo
/// under `x#1/1` nor the press of a gesture named `g#1/2` is a step of a gesture, before a restart or after one.
#[test]
fn the_lookup_reads_only_the_layers_steps() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    f.commit(&[4]);
    assert!(committed(&f.undo_as("x#1/1")));
    assert!(f.layer.rollback_steps("x").is_empty());
    let press = f.roll_as(a, "g#1/2");
    assert!(committed(&press), "{:?}", press.status);
    assert!(f.layer.rollback_steps("g").is_empty());
    assert_eq!(f.layer.rollback_steps("g#1/2").len(), 2);
    // A gesture `x` would reuse the ordinary undo's ID as its first step: refused, and still no step after a restart.
    let (order, digest) = (f.last_order(), f.digest());
    let refused = f.roll_as(0, "x");
    assert_eq!(code_of(&refused), code::COMMAND_ID_REUSED);
    assert_eq!((f.last_order(), f.digest()), (order, digest));
    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome("x", &refused.request).map(|r| r.status), Some(Status::UnknownOutcome));
    assert!(f.layer.rollback_steps("x").is_empty());
    assert_eq!(f.layer.rollback_steps("g#1/2").len(), 2);
}

/// R2-4: an ordinary undo under `g#3/4` after gesture `g` was cut at 2 of 4 by a crash is no third step of it, live
/// or after a reopen, though its ID is the one the gesture's third step would have had.
#[test]
fn an_ordinary_command_does_not_extend_a_gesture() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    f.commit(&[4]);
    f.commit(&[5]);
    f.layer.store_mut().inject.extend([Fate::Durable, Fate::Durable, Fate::Unknown]);
    assert_eq!(f.roll_as(a, "g").status, Status::UnknownOutcome);
    f.layer = open(f.layer.store().clone());
    f.shell.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.recovery(), None);
    let steps = f.layer.rollback_steps("g");
    assert_eq!(steps.iter().map(|s| s.command.as_str()).collect::<Vec<_>>(), ["g#1/4", "g#2/4"]);
    assert!(committed(&f.undo_as("g#3/4")));
    assert_eq!(f.layer.rollback_steps("g"), steps);
    let g = open(f.layer.store().clone());
    assert_eq!(g.recovery(), None);
    assert_eq!(g.rollback_steps("g"), steps);
}

/// R2-7: a reopen admits only what a live press writes: no gesture whose ID an earlier action used, and no gesture
/// whose steps span two epochs.
#[test]
fn a_reused_gesture_id_or_a_gesture_across_epochs_does_not_replay() {
    // (a) A live press under a used ID is refused, and a journal with that gesture's steps does not replay.
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    f.commit(&[4]);
    assert!(committed(&f.undo_as("g")));
    assert_eq!(code_of(&f.roll_as(a, "g")), code::COMMAND_ID_REUSED);
    assert!(committed(&f.roll(a)));
    assert_eq!(recovery(f.layer.store().clone()), None);
    let mut store = f.layer.store().clone();
    let n = store.log.len();
    store.log[n - 2].command = "g#1/2".into();
    store.log[n - 1].command = "g#2/2".into();
    assert_eq!(recovery(store), Some(code::UNADMITTED_JOURNAL_ACTION));

    // (b) A gesture of one step, a restart (a new epoch), and an ordinary undo relabelled as the gesture's step 2, with
    // the gesture's step relabelled as step 1 of 2 so that only the epoch differs.
    let mut h = fixture();
    h.commit(&[1]);
    let b = h.commit(&[2]);
    h.commit(&[3]);
    let first = h.roll(b);
    assert!(committed(&first));
    h.layer.restart(Secret::fixed(SECRET));
    h.shell.restart(Secret::fixed(SECRET));
    assert!(committed(&h.undo_as("later")));
    assert_eq!(recovery(h.layer.store().clone()), None);
    let mut store = h.layer.store().clone();
    let n = store.log.len();
    assert_ne!(store.log[n - 2].revision.epoch, store.log[n - 1].revision.epoch);
    assert_eq!(store.log[n - 2].command, format!("{}#1/1", first.command));
    store.log[n - 2].command = format!("{}#1/2", first.command);
    assert_eq!(recovery(store.clone()), None, "a gesture cut at 1 of 2");
    store.log[n - 1].row = ROLLBACK_ROW.to_owned();
    store.log[n - 1].command = format!("{}#2/2", first.command);
    assert_eq!(recovery(store), Some(code::UNADMITTED_JOURNAL_ACTION));
}

/// R2-8: a press ends the report of the gesture before it, so a press refused before its first step leaves no stale
/// "j of k" behind.
#[test]
fn a_press_ends_the_report_of_the_gesture_before_it() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    assert!(committed(&f.roll(a)));
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((2, 2)));
    let hint = f.layer.rollback_hint(0).unwrap();
    f.commit(&[5]);
    let stale = f.shell.invoke(&mut f.layer, ROLLBACK_ROW, json::object([("hint", hint.to_json())])).unwrap();
    assert_eq!(code_of(&stale), code::UNDO_SUPERSEDED);
    assert!(f.layer.rollback().is_none());
}

/// R2-9: the argument-aware availability hint refuses the arguments only the layer names, as every press does.
#[test]
fn availability_refuses_the_arguments_only_the_layer_names() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    let plain = f.roll_args(a, &[]);
    assert!(f.layer.availability_for(ROLLBACK_ROW, &plain).is_ok());
    let extra = [("gesture", text("g")), ("step", Value::Int(1)), ("of", Value::Int(1))];
    let args = f.roll_args(a, &extra);
    assert_eq!(f.layer.availability_for(ROLLBACK_ROW, &args).map_err(|e| e.code), Err(code::ARGUMENT_FORM));
    assert_eq!(code_of(&f.roll_with(a, &extra)), code::ARGUMENT_FORM);
}

// ----- Readings for the integrator -----

struct Display(Vec<u64>);

impl Refresh for Display {
    fn refresh(&mut self, revision: &Revision) -> Result<(), Error> {
        self.0.push(revision.state);
        Ok(())
    }
}

/// R2-6 (a reading): a gesture whose step j has an unknown fate answers `unknown_outcome`, though j-1 steps landed
/// durably, so the shell's report refreshes no display until the shell reads the outcome again (KN5.22, R-02).
#[test]
fn an_unknown_step_answers_unknown_outcome_over_its_landed_prefix() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    f.commit(&[4]);
    let state = f.layer.revision().state;
    f.layer.store_mut().inject.extend([Fate::Durable, Fate::Durable, Fate::Unknown]);
    let args = f.roll_args(a, &[]);
    let press = f.shell.prepare(&f.layer, ROLLBACK_ROW, args).unwrap();
    let mut display = Display(Vec::new());
    let report = f.shell.send_reported(&mut f.layer, press, &mut display);
    assert_eq!(report.receipt.status, Status::UnknownOutcome);
    assert_eq!(f.layer.revision().state, state + 2, "two steps landed");
    assert_eq!(report.display, DisplayStatus::NotDue);
    assert!(display.0.is_empty());
}

// ----- R2-5, closed by the owner's decision 4.2 of 23 November 2026 (LP4-15) -----

/// R2-5: "j of k" is durable. The journal still holds no step argument (`gesture`, `step`, `of`, the hint), only their
/// digest in `request`, but each step's command ID is `{gesture}#{i}/{k}`, so after a reopen a gesture cut at 2 of 3
/// and a whole gesture of 2 journal apart and the step IDs alone tell them apart. The format does not change.
#[test]
fn a_cut_gesture_and_a_whole_one_journal_apart() {
    let mut cut = fixture();
    cut.commit(&[1]);
    cut.commit(&[2]);
    cut.commit(&[3]);
    let failure = Error::new(code::COMMIT_FATE, "disk");
    cut.layer.store_mut().inject.extend([Fate::Durable, Fate::Durable, Fate::RolledBack(failure)]);
    let press = cut.roll(0);
    assert_eq!(cut.layer.rollback().map(|r| (r.of, r.landed)), Some((3, 2)));
    let mut whole = fixture();
    let a = whole.commit(&[1]);
    whole.commit(&[2]);
    whole.commit(&[3]);
    let other = whole.roll(a);
    assert_eq!(whole.layer.rollback().map(|r| (r.of, r.landed)), Some((2, 2)));
    assert_eq!(press.command, other.command);
    let strip = |f: &Fixture| -> Vec<_> {
        let log = f.layer.store().log.iter().skip(3);
        log.map(|t| c600_command::Transaction { request: String::new(), ..t.clone() }).collect()
    };
    let (cut_log, whole_log) = (strip(&cut), strip(&whole));
    assert_ne!(cut_log, whole_log);
    let commands = |log: &[c600_command::Transaction]| log.iter().map(|t| t.command.clone()).collect::<Vec<_>>();
    let gesture = &press.command;
    assert_eq!(commands(&cut_log), [format!("{gesture}#1/3"), format!("{gesture}#2/3")]);
    assert_eq!(commands(&whole_log), [format!("{gesture}#1/2"), format!("{gesture}#2/2")]);
    let blank = |log: Vec<c600_command::Transaction>| {
        log.into_iter().map(|t| c600_command::Transaction { command: String::new(), ..t }).collect::<Vec<_>>()
    };
    assert_eq!(blank(cut_log), blank(whole_log), "only the step IDs differ");
    let (g, h) = (open(cut.layer.store().clone()), open(whole.layer.store().clone()));
    assert_eq!((g.recovery(), h.recovery()), (None, None));
    assert_eq!((g.rollback(), h.rollback()), (None, None), "the report itself is transient");
    let extent = |layer: &CommandLayer, gesture: &str| {
        let steps = layer.rollback_steps(gesture);
        assert!(steps.iter().all(committed), "{steps:?}");
        let (_, landed, of) = rollback_step(&steps.last().unwrap().command).unwrap();
        assert_eq!(landed, steps.len() as u64);
        (landed, of)
    };
    assert_eq!(extent(&g, &press.command), (2, 3), "cut at 2 of 3");
    assert_eq!(extent(&h, &other.command), (2, 2), "whole at 2 of 2");
}

// ----- Refuted paths -----

/// A gesture ID that contains `#` and `/`, itself in a step's form, lands and replays; its steps parse to the whole
/// gesture.
#[test]
fn gesture_id_with_hash() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let press = f.roll_as(a, "a#1/1");
    assert!(committed(&press));
    assert_eq!(f.layer.rollback_steps("a#1/1").len(), 2);
    assert!(f.layer.rollback_steps("a").is_empty());
    assert_eq!(rollback_step("a#1/1#2/2"), Some(("a#1/1", 2, 2)));
    assert_eq!(recovery(f.layer.store().clone()), None);
    // A second gesture `a` of one step would write `a#1/1`, the press ID of the first gesture: refused.
    let again = f.roll_as(0, "a");
    assert_eq!(code_of(&again), code::COMMAND_ID_REUSED);
}

/// No run reaches the row, with or without a token.
#[test]
fn no_run_reaches_the_row() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    let (order, digest) = (f.last_order(), f.digest());
    let args = f.roll_args(a, &[]);
    let genuine = f.shell.prepare(&f.layer, ROLLBACK_ROW, args.clone()).unwrap();
    let caller = Caller::Runtime { run: "tool-run".into() };
    let with_token = f.layer.invoke(&caller, Invocation { command: "r1".into(), ..genuine.clone() });
    assert_eq!(code_of(&with_token), code::COMMIT_ROW_REFUSED);
    let bare = Invocation { command: "r2".into(), token: None, ..genuine.clone() };
    assert_eq!(code_of(&f.layer.invoke(&caller, bare)), code::COMMIT_ROW_REFUSED);
    assert_eq!((f.last_order(), f.digest()), (order, digest));
    // The genuine press still works afterwards (the run spent no nonce).
    assert!(committed(&f.shell.send(&mut f.layer, genuine)));
}

/// A forged step envelope (a genuine token, the ID and arguments of a step) commits nothing new: before the gesture
/// it is refused, after it the step ID is taken.
#[test]
fn a_step_envelope_commits_nothing() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let extra = [("gesture", text("g")), ("step", Value::Int(1)), ("of", Value::Int(2))];
    let args = f.roll_args(a, &extra);
    let forged = f.renamed(ROLLBACK_ROW, args, "g#1/2");
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.shell.send(&mut f.layer, forged)), code::ARGUMENT_FORM);
    assert_eq!((f.last_order(), f.digest()), (order, digest));

    let press = f.roll_as(a, "h");
    assert!(committed(&press));
    let order = f.last_order();
    let step = f.layer.rollback_steps("h")[0].clone();
    let hint = f.layer.undo_hint(Direction::Redo).unwrap();
    assert_eq!(step.command, "h#1/2");
    let redo = f.renamed("session.redo", json::object([("hint", hint.to_json())]), "h#1/2");
    assert_eq!(code_of(&f.shell.send(&mut f.layer, redo)), code::COMMAND_ID_REUSED);
    assert_eq!(f.layer.outcome("h#1/2", &step.request).map(|r| r.status), Some(step.status.clone()));
    assert_eq!(f.last_order(), order);
}

/// Retry idempotency: the same press delivered again answers the same receipt and journals nothing; the same ID with
/// other content is refused; after a restart the press is `unknown_outcome` and its steps are durable.
#[test]
fn retry_idempotency() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let args = f.roll_args(a, &[]);
    let press = f.shell.prepare(&f.layer, ROLLBACK_ROW, args).unwrap();
    let first = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&first));
    let (order, digest) = (f.last_order(), f.digest());
    let again = f.shell.send(&mut f.layer, press.clone());
    assert_eq!(again, first);
    assert_eq!((f.last_order(), f.digest()), (order, digest.clone()));
    let other = f.roll_as(0, &press.command);
    assert_eq!(code_of(&other), code::COMMAND_ID_REUSED);
    f.layer.restart(Secret::fixed(SECRET));
    let late = f.shell.send(&mut f.layer, press.clone());
    assert_eq!(late.status, Status::UnknownOutcome);
    assert_eq!(f.layer.rollback_steps(&press.command).len(), 2);
    assert_eq!((f.last_order(), f.digest()), (order, digest.clone()));
    // A reopen: the same envelope is of another epoch, and a new press under the same ID names the earlier gesture,
    // though its one step's ID `{gesture}#1/1` is new (R2-5).
    let mut g = open(f.layer.store().clone());
    assert_eq!(code_of(&g.invoke(&Caller::Shell, press.clone())), code::EPOCH_CHANGED);
    f.layer = g;
    f.shell.restart(Secret::fixed(SECRET));
    let renewed = f.roll_as(0, &press.command);
    assert_eq!(code_of(&renewed), code::COMMAND_ID_REUSED);
    assert_eq!((f.last_order(), f.digest()), (order, digest));
}

/// Redo after a partial rollback, and a rollback forward over it; an ordinary commit truncates the redo stack and a
/// hint into it is superseded.
#[test]
fn redo_after_a_partial_rollback() {
    let mut f = fixture();
    f.protect(&[0], false);
    f.protect(&[], true);
    let a = f.commit(&[1]);
    let b = f.commit(&[2]);
    let top = f.digest();
    let press = f.roll(0);
    assert!(committed(&press));
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((4, 3)));
    let refused = f.roll(b);
    assert_eq!(code_of(&refused), code::CONFIRMATION_REQUIRED, "the redo of a release needs its confirmation");
    let forward = f.roll_with(b, &[("confirm", Value::Bool(true))]);
    assert!(committed(&forward));
    assert_eq!(f.layer.rollback().map(|r| (r.direction, r.of, r.landed)), Some((Direction::Redo, 3, 3)));
    assert_eq!(f.digest(), top);
    assert_eq!(recovery(f.layer.store().clone()), None);
    assert!(committed(&f.roll(a)));
    let hint = f.layer.rollback_hint(b).unwrap();
    f.commit(&[3]);
    assert_eq!(f.layer.rollback_hint(b), None);
    let stale = f.shell.invoke(&mut f.layer, ROLLBACK_ROW, json::object([("hint", hint.to_json())])).unwrap();
    assert_eq!(code_of(&stale), code::UNDO_SUPERSEDED);
}

/// Layout-history rows inside the span are skipped, the layout history is untouched, and a layout row is no target.
#[test]
fn layout_rows_in_the_span() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    let l1 = f.theme("design/theme", "dawn");
    f.commit(&[2]);
    let l2 = f.theme("design/theme", "dusk");
    f.commit(&[3]);
    assert_eq!(f.layer.rollback_hint(l1), None);
    assert_eq!(f.layer.rollback_hint(l2), None);
    let layout = f.layer.layout_hint(Direction::Undo);
    let press = f.roll(a);
    assert!(committed(&press), "{:?}", press.status);
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((2, 2)));
    assert_eq!(f.layer.layout_hint(Direction::Undo).map(|h| h.target), layout.map(|h| h.target));
    assert_eq!(f.layer.record("design/theme").map(|v| v.payload.clone()), Some(text("dusk")));
    assert_eq!(recovery(f.layer.store().clone()), None);
    // A layout undo journaled under the rollback row does not replay.
    let hint = f.layer.layout_hint(Direction::Undo).unwrap();
    let receipt = f.shell.invoke(&mut f.layer, LAYOUT_UNDO_ROW, json::object([("hint", hint.to_json())])).unwrap();
    assert!(committed(&receipt), "{:?}", receipt.status);
    let mut store = f.layer.store().clone();
    let n = store.log.len();
    store.log[n - 1].row = ROLLBACK_ROW.to_owned();
    store.log[n - 1].command = "lay#1/1".into();
    assert!(recovery(store).is_some());
}

/// A record write and a protection change in the span are undone in order, and a crafted step that releases without
/// its confirmation or comes from a run does not replay.
#[test]
fn workspace_actions_and_releases_in_the_span() {
    let mut f = fixture();
    f.commit(&[1]);
    f.write("r", "one");
    f.protect(&[0], false);
    f.write("r", "two");
    f.protect(&[], true);
    f.commit(&[2]);
    let press = f.roll_with(0, &[("confirm", Value::Bool(true))]);
    assert!(committed(&press), "{:?}", press.status);
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((6, 6)));
    assert!(f.layer.record("r").is_none());
    assert_eq!(recovery(f.layer.store().clone()), None);
    let mut found = false;
    for at in 6..f.layer.store().log.len() {
        let mut store = f.layer.store().clone();
        store.log[at].confirm = false;
        if recovery(store) == Some(code::UNADMITTED_JOURNAL_ACTION) {
            found = true;
        }
    }
    assert!(found, "no step needed its confirmation");
    let mut store = f.layer.store().clone();
    store.log[6].caller = Caller::Runtime { run: "tool-run".into() };
    assert_eq!(recovery(store), Some(code::UNADMITTED_JOURNAL_ACTION));
}

/// A press with a token minted for `session.undo`, or with the hint or `confirm` changed after minting, or in
/// read-only recovery, lands nothing.
#[test]
fn token_and_recovery() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let (order, digest) = (f.last_order(), f.digest());
    let hint = f.layer.undo_hint(Direction::Undo).unwrap();
    let mut undo = f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    undo.row = ROLLBACK_ROW.to_owned();
    assert_eq!(code_of(&f.shell.send(&mut f.layer, undo)), code::TOKEN_MISMATCH);
    let args = f.roll_args(a, &[]);
    let mut press = f.shell.prepare(&f.layer, ROLLBACK_ROW, args).unwrap();
    press.args = f.roll_args(0, &[]);
    assert_eq!(code_of(&f.shell.send(&mut f.layer, press)), code::TOKEN_MISMATCH);
    let args = f.roll_args(a, &[]);
    let mut press = f.shell.prepare(&f.layer, ROLLBACK_ROW, args).unwrap();
    press.args = f.roll_args(a, &[("confirm", Value::Bool(true))]);
    assert_eq!(code_of(&f.shell.send(&mut f.layer, press)), code::TOKEN_MISMATCH);
    assert_eq!((f.last_order(), f.digest()), (order, digest.clone()));
    f.layer.store_mut().inject.extend([Fate::Unknown]);
    assert_eq!(f.roll(a).status, Status::UnknownOutcome);
    let args = json::object([("hint", Value::Null)]);
    let next = f.shell.invoke(&mut f.layer, ROLLBACK_ROW, args).unwrap();
    assert_eq!(code_of(&next), code::READ_ONLY_RECOVERY);
    assert_eq!((f.last_order(), f.digest()), (order, digest));
}

/// A store failure in the middle of the gesture stops it with the prefix landed, and the journal replays.
#[test]
fn a_rolled_back_step_stops_the_gesture() {
    let mut f = fixture();
    f.commit(&[1]);
    f.commit(&[2]);
    f.commit(&[3]);
    let failure = Error::new(code::COMMIT_FATE, "disk");
    f.layer.store_mut().inject.extend([Fate::Durable, Fate::RolledBack(failure)]);
    let press = f.roll(0);
    assert!(committed(&press), "{:?}", press.status);
    let report = f.layer.rollback().cloned().unwrap();
    assert_eq!((report.of, report.landed), (3, 1));
    assert!(matches!(report.stop, Some(Status::Failed(_))));
    assert_eq!(f.layer.recovery(), None);
    assert_eq!(recovery(f.layer.store().clone()), None);
    assert!(committed(&f.roll(0)));
    assert_eq!(f.layer.rollback().map(|r| (r.of, r.landed)), Some((2, 2)));
}

/// A pending presented preview ends with the first step, and its commit token commits nothing afterwards; a hint that
/// names the own action of the head has no span.
#[test]
fn pending_preview_and_head_target() {
    let mut f = fixture();
    let a = f.commit(&[1]);
    let b = f.commit(&[2]);
    let receipt = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[3]))).unwrap();
    let Status::Running(id) = receipt.status else { panic!("{:?}", receipt.status) };
    assert_eq!(f.layer.run_jobs(), 1);
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    let args = json::object([("preview", Value::Int(id as i64))]);
    let execute = f.shell.prepare(&f.layer, "operation.execute", args).unwrap();
    // A crafted hint to the head: a genuine token, the current order, the top of the undo stack.
    let mut hint = f.layer.rollback_hint(a).unwrap();
    hint.target = b;
    let (order, digest) = (f.last_order(), f.digest());
    let head = f.shell.invoke(&mut f.layer, ROLLBACK_ROW, json::object([("hint", hint.to_json())])).unwrap();
    assert_eq!(code_of(&head), code::UNDO_SUPERSEDED);
    hint.target = 99;
    let none = f.shell.invoke(&mut f.layer, ROLLBACK_ROW, json::object([("hint", hint.to_json())])).unwrap();
    assert_eq!(code_of(&none), code::UNDO_SUPERSEDED);
    assert_eq!((f.last_order(), f.digest()), (order, digest));
    assert!(committed(&f.roll(a)));
    let (order, digest) = (f.last_order(), f.digest());
    let late = f.shell.send(&mut f.layer, execute);
    assert!(late.status.code().is_some());
    assert_eq!((f.last_order(), f.digest()), (order, digest));
}
