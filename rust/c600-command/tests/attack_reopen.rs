//! Attack tests on undo, the workspace and a reopen (classes F and B of the boundary attack): after the same actions,
//! a live layer and a reopen of its store must be one session. A session that differs after a reopen is a session
//! whose undo, records or protection the solver did not choose. Findings file: `kn5-boundary-attack-findings.md` in
//! the integrator's notes folder.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use c600_command::{
    draft_args, CommandLayer, Direction, Error, HeadlessFrames, JobOutput, Kind, MemoryStore, Overlay, Provenance,
    Receipt, RecordValue, Secret, Shell, Status, Table, POLICY_ROW, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const NOTE_ROW: &str = "tool.note";
const NAMES: [&str; 3] = ["a", "b", "c"];

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

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn letters(letters: &[i64]) -> Value {
    Value::Array(letters.iter().map(|&letter| Value::Int(letter)).collect())
}

struct Reader;

impl Provenance for Reader {
    fn check(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

/// One action of the solver, chosen by the seed alone, so the live layer and the reopened one get the same list.
#[derive(Clone, Debug)]
enum Op {
    Commit(i64),
    Undo,
    Redo,
    Write(usize, [i64; 2]),
    Protect(u8),
    Policy(bool),
    Job(usize, i64),
    CommitRecord(usize),
}

struct Rng(u64);

impl Rng {
    fn next(&mut self, below: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % below
    }
}

fn ops(seed: u64, count: usize) -> Vec<Op> {
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    (0..count)
        .map(|_| match rng.next(16) {
            0..=2 => Op::Commit(1 + rng.next(6) as i64),
            3..=5 => Op::Undo,
            6..=7 => Op::Redo,
            8..=10 => Op::Write(rng.next(3) as usize, [1 + rng.next(6) as i64, 1 + rng.next(6) as i64]),
            11 => Op::Protect(rng.next(8) as u8),
            12 => Op::Policy(rng.next(2) == 0),
            13..=14 => Op::Job(rng.next(3) as usize, 1 + rng.next(6) as i64),
            _ => Op::CommitRecord(rng.next(3) as usize),
        })
        .collect()
}

struct Session {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

fn equip(mut layer: CommandLayer, prefix: &str) -> Session {
    layer.set_provenance(Box::new(Reader));
    // ADR-D01.9's settle interval is tested in tests/settle.rs; these sessions commit on the first presented press.
    layer.set_settle(Duration::ZERO);
    let overlay = Overlay { module: "notes".into(), entry: "note".into() };
    layer.register_overlay(NOTE_ROW, Kind::Workspace, &[], overlay).unwrap();
    Session { layer, shell: Shell::new(Secret::fixed(SECRET), prefix), frames: HeadlessFrames::new() }
}

fn fresh() -> Session {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    equip(CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected), "live")
}

fn reopened(store: MemoryStore, prefix: &str) -> Session {
    let table = Table::h08().with_drafts();
    equip(CommandLayer::open(kernel(), table, Secret::fixed(SECRET), store).expect("the journal reopens"), prefix)
}

/// A receipt without the IDs a reopen renumbers: the status's name and its code.
fn answer(receipt: Result<Receipt, Error>) -> String {
    match receipt {
        Ok(receipt) => format!("{} {:?}", receipt.status.name(), receipt.status.code()),
        Err(error) => format!("shell {}", error.code),
    }
}

impl Session {
    fn execute(&mut self, args: Value) -> String {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", args);
        let Ok(Receipt { status: Status::Running(id), .. }) = receipt else { return answer(receipt) };
        self.layer.run_jobs();
        if self.frames.present(&mut self.shell, &mut self.layer, id).is_err() {
            return "not presented".into();
        }
        let args = json::object([("preview", Value::Int(id as i64))]);
        answer(self.shell.invoke(&mut self.layer, "operation.execute", args))
    }

    fn step(&mut self, direction: Direction) -> String {
        let Some(hint) = self.layer.undo_hint(direction) else { return "no target".into() };
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        let args = json::object([("confirm", Value::Bool(true)), ("hint", hint.to_json())]);
        answer(self.shell.invoke(&mut self.layer, row, args))
    }

    fn apply(&mut self, op: &Op) -> String {
        match op {
            Op::Commit(letter) => self.execute(draft_args(&word(&[*letter]))),
            Op::Undo => self.step(Direction::Undo),
            Op::Redo => self.step(Direction::Redo),
            Op::Write(name, value) => {
                let args = json::object([
                    ("payload", letters(value)),
                    ("provenance", text("block")),
                    ("record", text(NAMES[*name])),
                ]);
                answer(self.shell.invoke(&mut self.layer, RECORD_ROW, args))
            }
            Op::Protect(mask) => {
                let orbits = (0..3).filter(|bit| mask >> bit & 1 == 1).map(Value::Int).collect();
                let args = json::object([("confirm", Value::Bool(true)), ("orbits", Value::Array(orbits))]);
                answer(self.shell.invoke(&mut self.layer, "protection.orbit.set", args))
            }
            Op::Policy(strict) => {
                let args = json::object([
                    ("confirm", Value::Bool(true)),
                    ("policy", text(if *strict { "strict" } else { "net" })),
                ]);
                answer(self.shell.invoke(&mut self.layer, POLICY_ROW, args))
            }
            Op::Job(name, letter) => {
                let receipt = self.shell.invoke(&mut self.layer, NOTE_ROW, json::object([]));
                let Ok(Receipt { status: Status::Running(job), .. }) = receipt else { return answer(receipt) };
                self.layer.take_program_job().expect("the job the press started");
                let value = RecordValue { payload: letters(&[*letter]), provenance: "run".into() };
                let output = JobOutput {
                    value: Value::Null,
                    views: Vec::new(),
                    records: [(NAMES[*name].to_owned(), value)].into(),
                };
                answer(
                    self.layer.finish_program_job_with(job, Ok(output)).ok_or_else(|| Error::new("none", "no receipt")),
                )
            }
            Op::CommitRecord(name) => {
                self.execute(json::object([("draft", json::object([("record", text(NAMES[*name]))]))]))
            }
        }
    }

    /// What the solver's session is, without the epoch: the revision, the state, the protection, the records with
    /// their versions and the two undo targets.
    fn view(&self) -> String {
        let r = self.layer.revision();
        format!(
            "{} {} {} {} state {} workspace {} protection {} {}\n  snapshot {}\n  protected {:?} {:?}\n  records {:?}\n  \
             undo {:?} redo {:?}",
            r.session,
            r.identity,
            r.mode,
            r.head,
            r.state,
            r.workspace,
            r.protection,
            r.digest,
            self.layer.snapshot().digest(),
            self.layer.protected().orbits(),
            self.layer.policy(),
            self.layer.records(),
            self.layer.undo_hint(Direction::Undo),
            self.layer.undo_hint(Direction::Redo),
        )
    }
}

/// Runs `count` actions of `seed` on a live layer, reopens its store after `fork` of them, runs the rest on the reopen
/// too, and then steps both all the way back and forward again. Returns the first difference.
fn difference(seed: u64, count: usize, fork: usize) -> Option<String> {
    let list = ops(seed, count);
    let mut live = fresh();
    let mut other = None;
    for (index, op) in list.iter().enumerate() {
        if index == fork {
            other = Some(reopened(live.layer.store().clone(), "after-reopen"));
            let other = other.as_ref().unwrap();
            if other.view() != live.view() {
                return Some(format!(
                    "seed {seed}: at the reopen after {fork} actions\nlive:   {}\nreopen: {}",
                    live.view(),
                    other.view()
                ));
            }
        }
        let answered = live.apply(op);
        if let Some(other) = &mut other {
            let theirs = other.apply(op);
            if theirs != answered || other.view() != live.view() {
                return Some(format!(
                    "seed {seed}: action {index} {op:?} of {list:?}, reopened after {fork}\nlive:   {answered}\n{}\nreopen: \
                     {theirs}\n{}",
                    live.view(),
                    other.view()
                ));
            }
        }
    }
    let mut other = other.unwrap_or_else(|| reopened(live.layer.store().clone(), "after-reopen"));
    // A second reopen, of the whole history, joins the walk back and forward.
    let mut last = reopened(live.layer.store().clone(), "at-the-end");
    for direction in [Direction::Undo, Direction::Redo] {
        for walked in 0..=count * 2 {
            let (a, b, c) = (live.step(direction), other.step(direction), last.step(direction));
            if a != b || a != c || live.view() != other.view() || live.view() != last.view() {
                return Some(format!(
                    "seed {seed}: step {walked} of the walk {direction:?} after {list:?}\nlive:   {a}\n{}\nreopen after \
                     {fork}: {b}\n{}\nreopen at the end: {c}\n{}",
                    live.view(),
                    other.view(),
                    last.view()
                ));
            }
            if a == "no target" {
                break;
            }
        }
    }
    None
}

/// Guard (classes F and B, attacker 3: a reopen in the middle). KN5.19 [B], ONE.50 [B], ONE.53 [D], KN4's replay: for
/// seeded lists of the solver's actions (word commits, record-draft commits, undo, redo, record writes, protection
/// and policy changes, a tool row's record write), a reopen in the middle and a reopen at the end are the same session
/// as the live layer: the same answers to the same later actions, the same revision counters, state, protected set,
/// policy, records and record versions, and the same undo and redo targets all the way back and forward.
#[test]
fn a_reopen_in_the_middle_is_the_same_session() {
    let mut differences = Vec::new();
    for seed in 1..=10 {
        if let Some(found) = difference(seed, 36, 12 + (seed as usize % 3) * 6) {
            differences.push(found);
        }
    }
    assert!(differences.is_empty(), "{} of 10 seeds differ; the first:\n{}", differences.len(), differences[0]);
}

/// The seeded lists are not vacuous: over the ten seeds every kind of action is answered `committed` or `completed`
/// at least once, and a tool row's record write lands while an earlier write of the same record is the undo target
/// (the layer's history cut) at least once.
#[test]
fn the_seeded_lists_reach_every_action() {
    let mut reached = std::collections::BTreeMap::new();
    let mut cuts = 0;
    for seed in 1..=10 {
        let mut live = fresh();
        for op in ops(seed, 36) {
            let before = live.layer.undo_hint(Direction::Undo);
            let answered = live.apply(&op);
            if answered.starts_with("committed") || answered.starts_with("completed") {
                let kind = format!("{op:?}");
                *reached.entry(kind.split('(').next().unwrap().to_owned()).or_insert(0) += 1;
                if matches!(op, Op::Job(..)) && before.is_some() && live.layer.undo_hint(Direction::Undo) != before {
                    cuts += 1;
                }
            }
        }
    }
    let kinds: Vec<&str> = reached.keys().map(String::as_str).collect();
    assert_eq!(kinds, ["Commit", "CommitRecord", "Job", "Policy", "Protect", "Redo", "Undo", "Write"], "{reached:?}");
    assert!(cuts > 0, "no tool write cut an undo target: {reached:?}");
    eprintln!("reached {reached:?}, history cuts {cuts}");
}
