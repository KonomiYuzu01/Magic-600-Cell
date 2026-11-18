//! LX-2's metered and forced-kill half for C9 (RTE.42; CT-RTE-10 and CT-LG4-09 in part): trials against a runtime
//! child binary the caller names, so that the decisive runs use the product build's own `c600-runtime` unchanged.
//!
//! Each trial runs one job on [`c600_runtime::Runtime`]. Every job first proposes SCENARIO.json's runtime word, so
//! the engine-owned pending preview exists before the fault; then:
//!
//! - `control` returns at once;
//! - `loop_metered` runs a `repeat` loop that makes no kernel call until the meter's step limit (100,000) cuts it off
//!   with `KN7:budget.steps`;
//! - `recursion_metered` recurses without end until its live frames reach the memory limit (1 MiB) and the meter cuts
//!   it off with `KN7:budget.memory` (the meter has no separate recursion dimension: a frame is charged as memory);
//! - `alloc_metered` grows a list with `push` in a `repeat` loop until the meter's memory limit (64 KiB, kept small
//!   because each `push` copies the list) cuts it off with `KN7:budget.memory`;
//! - `forced_kill` has the caller kill the child's actual process (`Killer`, `Child::kill`) inside the proposal's
//!   perform, the barrier; the parent must report a typed `Lost` fault;
//! - `alloc_ceiling`, the one native trial ([`Trial::NATIVE`]), doubles a list with `concat` under a meter far above
//!   the ceiling ([`CEILING_METER`]) on the injection device of option (a) (top integrator, 18 November 2026): the
//!   bin `c600-lx2-child`, product source (`c600_runtime::child::main`) under a global allocator that refuses past
//!   [`CEILING`] live heap bytes. The standard library reports the failed allocation and aborts the child; the parent
//!   must report a typed `Lost` fault, and the child's standard error must show the allocation failure. The device is
//!   never shipped: `tests/device.rs` checks that no other package depends on this one and that c600-runtime has no
//!   global allocator.
//!
//! After each trial the same runtime runs the control job again: it must complete, in a fresh child after a fault and
//! in the same child otherwise. The pending preview must still be the runtime word. A metered trial and the control
//! also run in process (`run_in_process`), and the two executors must agree.
//!
//! [`session`] runs the same trials with a session open (RTE.42 conditions 1 and 2): the journal's records and the
//! state hash after the same commands must equal the control's, and the session must stay open.
//!
//! Not here: LX-2's other native faults (unmetered recursion, native loop with its watchdog), which need a hook in
//! c600-runtime that LP3 owns; the barrier's release acknowledgement; and conditions 3 and 4 (the renderer gate,
//! LX-4's echo bar and E2 commits during fault windows). Evidence kind: source and synthetic until the owner's machine
//! runs it on the product build; nothing here is timing.

#![forbid(unsafe_code)]

use std::path::Path;
use std::time::Duration;

pub mod session;

use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::{code, Amounts, Outcome};
use c600_run::value::Value;
use c600_runtime::{run_in_process, Config, Ended, Fault, FaultKind, Job, JobOptions, Runtime};

pub const FORMAT: &str = "magic600-c9-lx2-faults-v1";
/// SCENARIO.json's runtime word, which every job proposes before its fault.
pub const RUNTIME_WORD: [i32; 3] = [7, -11, 1200];
pub const STEP_LIMIT: u128 = 100_000;
pub const MEMORY_LIMIT: u128 = 1 << 20;
/// The allocation trial's memory limit.
pub const ALLOC_LIMIT: u128 = 1 << 16;
/// The wall-clock safety stop of one run; a trial that reaches it has failed.
pub const DEADLINE: Duration = Duration::from_secs(60);
/// The injection device's ceiling variable: live heap bytes, decimal (`c600-lx2-child`).
pub const CEILING_VARIABLE: &str = "C600_LX2_CEILING";
/// The native allocation trial's ceiling on the injection child's live heap bytes.
pub const CEILING: usize = 32 << 20;
/// The native allocation trial's meter limit: far above [`CEILING`], a safety net only, so that the allocator and not
/// the meter stops the program.
pub const CEILING_METER: u128 = 1 << 30;
/// What the standard library writes to standard error when an allocation fails, before it aborts.
const ALLOCATION_FAILED: &str = "memory allocation of";
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const HEADER: &str = "edition \"6cs-draft-1\";\nmodule lx2 requires [];\n\n";
const PROPOSE: &str = "    let p = propose(w, target(\"operation.preview\"));\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trial {
    Control,
    LoopMetered,
    RecursionMetered,
    AllocMetered,
    ForcedKill,
    /// Native: the injection child's allocator refuses past [`CEILING`].
    AllocCeiling,
}

/// What a run ended with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observed {
    Completed,
    CutOff(&'static str),
    Fault(FaultKind, &'static str),
    Other(String),
}

impl Observed {
    pub fn describe(&self) -> String {
        match self {
            Observed::Completed => "completed".into(),
            Observed::CutOff(code) => format!("cut off: {code}"),
            Observed::Fault(kind, code) => format!("fault {kind:?}: {code}"),
            Observed::Other(what) => format!("other: {what}"),
        }
    }
}

impl Trial {
    /// The trials on the product's own runtime child.
    pub const ALL: [Trial; 5] =
        [Trial::Control, Trial::LoopMetered, Trial::RecursionMetered, Trial::AllocMetered, Trial::ForcedKill];
    /// The native trials, which run only on the injection child.
    pub const NATIVE: [Trial; 1] = [Trial::AllocCeiling];

    pub fn name(self) -> &'static str {
        match self {
            Trial::Control => "control",
            Trial::LoopMetered => "loop_metered",
            Trial::RecursionMetered => "recursion_metered",
            Trial::AllocMetered => "alloc_metered",
            Trial::ForcedKill => "forced_kill",
            Trial::AllocCeiling => "alloc_ceiling",
        }
    }

    /// Whether the trial needs the injection child.
    pub fn native(self) -> bool {
        Trial::NATIVE.contains(&self)
    }

    /// The job's source: module `lx2`, entry `command work(w: Word) -> Nat`. The entry is a command, as a tool's
    /// previewing entry must be (LG3.6).
    pub fn source(self) -> String {
        let body = match self {
            Trial::Control | Trial::ForcedKill => "    return 1;\n",
            Trial::LoopMetered => {
                "    return repeat(1000000000000, 0, fn(acc: Nat) -> Nat {\n        return exact_add(acc, 1);\n    });\n"
            }
            Trial::RecursionMetered => "    return down(0);\n",
            Trial::AllocMetered => {
                "    let grown = repeat(1000000000000, list<Nat>(), fn(done: List<Nat>) -> List<Nat> {\n        return push(done, length(done));\n    });\n    return length(grown);\n"
            }
            // Doubles the list each round, so it passes the ceiling in a few dozen rounds.
            Trial::AllocCeiling => {
                "    let seed = push(list<Nat>(), 0);\n    let grown = repeat(1000000000000, seed, fn(done: List<Nat>) -> List<Nat> {\n        return concat(done, done);\n    });\n    return length(grown);\n"
            }
        };
        let down = match self {
            Trial::RecursionMetered => "\nfn down(n: Nat) -> Nat effects [] {\n    return down(exact_add(n, 1));\n}\n",
            _ => "",
        };
        format!("{HEADER}export command work(w: Word) -> Nat effects [preview] {{\n{PROPOSE}{body}}}\n{down}")
    }

    pub fn limits(self) -> Amounts {
        match self {
            Trial::LoopMetered => Amounts { steps: STEP_LIMIT, ..WIDE },
            Trial::RecursionMetered => Amounts { memory: MEMORY_LIMIT, ..WIDE },
            Trial::AllocMetered => Amounts { memory: ALLOC_LIMIT, ..WIDE },
            Trial::AllocCeiling => Amounts { memory: CEILING_METER, ..WIDE },
            _ => WIDE,
        }
    }

    pub fn job(self) -> Job {
        Job {
            sources: vec![self.source().into_bytes()],
            module: "lx2".into(),
            entry: "work".into(),
            args: vec![Value::word(&RUNTIME_WORD)],
            // One summary row per operation would carry every live frame's path: quadratic at depth.
            options: JobOptions { record: false, ..JobOptions::new(self.limits()) },
        }
    }

    pub fn expected(self) -> Observed {
        match self {
            Trial::Control => Observed::Completed,
            Trial::LoopMetered => Observed::CutOff(code::STEPS),
            Trial::RecursionMetered | Trial::AllocMetered => Observed::CutOff(code::MEMORY),
            Trial::ForcedKill | Trial::AllocCeiling => {
                Observed::Fault(FaultKind::Lost, c600_runtime::parent::RUNTIME_LOST)
            }
        }
    }

    /// Whether the trial ends in a fault of the child, so that it has no in-process counterpart.
    pub fn faults(self) -> bool {
        matches!(self.expected(), Observed::Fault(..))
    }
}

/// The runtime's configuration for `trial` on the child binary `runtime`: a native trial arms the injection child's
/// ceiling, for the trial's child and for the fresh child that recovers after it.
pub fn config(runtime: &Path, trial: Trial) -> Config {
    let mut config = Config::new(runtime);
    if trial.native() {
        config.env.push((CEILING_VARIABLE.into(), CEILING.to_string().into()));
    }
    config
}

/// What `trial` ended with. A native allocation trial counts as lost only when the child's standard error shows the
/// standard library's allocation failure, so that a child lost for another reason does not pass for the ceiling.
pub fn observe_trial(trial: Trial, result: Result<Ended, Fault>) -> Observed {
    match result {
        Err(fault) if trial == Trial::AllocCeiling && !fault.stderr.contains(ALLOCATION_FAILED) => {
            Observed::Other(format!("lost without the ceiling's allocation failure: {}", fault.detail))
        }
        result => observe(result),
    }
}

/// The caller's host, as a session answers a proposal: it keeps the proposed word as the engine-owned pending
/// preview, and runs `on_perform` inside the perform, which is the barrier where a forced kill lands.
#[derive(Default)]
pub struct Session {
    pub preview: Option<Vec<i32>>,
    pub performs: usize,
    pub on_perform: Option<Box<dyn FnMut()>>,
}

impl Host for Session {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Propose { .. } => Ok(RowTariff { kernel_cost: 11, result_bound: 4096 }),
            _ => Err(code::NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.performs += 1;
        let Request::Propose { word, .. } = request else { return Err(code::NOT_GRANTED) };
        self.preview = Some(word.to_vec());
        if let Some(hook) = &mut self.on_perform {
            hook();
        }
        Ok(Value::receipt(vec![
            ("base", Value::text("lx2-base")),
            ("effect", Value::text("lx2 runtime word")),
            ("id", Value::text("preview-1")),
            ("protection", Value::tag("clear")),
        ]))
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(code::NOT_GRANTED)
    }
}

pub fn observe(result: Result<Ended, Fault>) -> Observed {
    match result {
        Ok(Ended::Ran(run)) => match (run.summary.outcome, run.summary.code) {
            (Outcome::Completed, _) => Observed::Completed,
            (Outcome::CutOff, Some(code)) => Observed::CutOff(code),
            (outcome, code) => Observed::Other(format!("{outcome:?} {code:?}")),
        },
        Ok(Ended::Refused(refused)) => Observed::Other(format!("refused: {refused:?}")),
        Err(fault) => Observed::Fault(fault.kind, fault.code()),
    }
}

/// One trial's record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub trial: Trial,
    pub observed: Observed,
    /// The pending preview was still the runtime word after the trial.
    pub preview_kept: bool,
    /// The control job completed on the same runtime afterwards.
    pub recovered: bool,
    /// Child processes started over the trial and the recovery run.
    pub spawns: u64,
    /// The same job in process, for the trials that end without a fault.
    pub in_process: Option<Observed>,
}

impl Record {
    /// Whether the record shows what LX-2's level requires.
    pub fn agrees(&self) -> bool {
        let fault = matches!(self.observed, Observed::Fault(..));
        self.observed == self.trial.expected()
            && self.preview_kept
            && self.recovered
            && self.spawns == if fault { 2 } else { 1 }
            && self.in_process.as_ref().is_none_or(|o| *o == self.observed)
    }
}

/// The injection child as the record names it: its file name and SHA-256, never its path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub file: String,
    pub sha256: String,
}

/// Runs one trial against the child binary `runtime`: the product's own child, or the injection child for a native
/// trial.
pub fn run_trial(runtime: &Path, trial: Trial) -> Record {
    let mut rt = Runtime::new(config(runtime, trial));
    let mut session = Session::default();
    if let Err(fault) = rt.start() {
        let observed = Observed::Fault(fault.kind, fault.code());
        return Record {
            trial,
            observed,
            preview_kept: false,
            recovered: false,
            spawns: rt.spawns(),
            in_process: None,
        };
    }
    if trial == Trial::ForcedKill {
        if let Some(killer) = rt.killer() {
            session.on_perform = Some(Box::new(move || killer.kill()));
        }
    }
    let observed = observe_trial(trial, rt.run(&trial.job(), &mut session, Some(DEADLINE)));
    session.on_perform = None;
    let preview_kept = session.preview.as_deref() == Some(&RUNTIME_WORD[..]);
    let recovered =
        observe(rt.run(&Trial::Control.job(), &mut Session::default(), Some(DEADLINE))) == Observed::Completed;
    let spawns = rt.spawns();
    rt.stop();
    let in_process =
        (!trial.faults()).then(|| observe(Ok(run_in_process(&trial.job(), &mut Session::default(), None))));
    Record { trial, observed, preview_kept, recovered, spawns, in_process }
}

/// The record of every trial as JSON, with the session-level records when they ran (an empty array otherwise). The
/// product's child and the injection child, when native trials ran, are named by file name and SHA-256 only, never by
/// path; each trial row says which child it ran on.
pub fn record_json(
    runtime_name: &str,
    runtime_sha256: &str,
    device: Option<&Device>,
    records: &[Record],
    sessions: &[session::SessionRecord],
) -> String {
    let clean = |text: &str| text.replace(['"', '\\'], "_");
    let child = |trial: Trial| if trial.native() { "injection device" } else { "product" };
    let rows: Vec<String> = records
        .iter()
        .map(|r| {
            let in_process = r.in_process.as_ref().map_or("null".to_string(), |o| format!("\"{}\"", o.describe()));
            format!(
                "    {{\"trial\": \"{}\", \"child\": \"{}\", \"expected\": \"{}\", \"observed\": \"{}\", \"preview_kept\": {}, \"recovered\": {}, \"spawns\": {}, \"in_process\": {in_process}, \"agrees\": {}}}",
                r.trial.name(),
                child(r.trial),
                r.trial.expected().describe(),
                clean(&r.observed.describe()),
                r.preview_kept,
                r.recovered,
                r.spawns,
                r.agrees()
            )
        })
        .collect();
    let device = device.map_or("null".to_string(), |d| {
        format!(
            "{{\"file\": \"{}\", \"sha256\": \"{}\", \"source\": \"product source + injection device\", \"ceiling_variable\": \"{CEILING_VARIABLE}\", \"ceiling_bytes\": {CEILING}, \"meter_memory_bytes\": {CEILING_METER}}}",
            clean(&d.file),
            clean(&d.sha256)
        )
    });
    let agrees = records.iter().all(Record::agrees) && session::all_agree(sessions);
    format!(
        "{{\n  \"format\": \"{FORMAT}\",\n  \"runtime\": {{\"file\": \"{}\", \"sha256\": \"{runtime_sha256}\", \"injection_device\": {device}}},\n  \"limits\": {{\"steps\": {STEP_LIMIT}, \"memory_bytes\": {MEMORY_LIMIT}, \"alloc_memory_bytes\": {ALLOC_LIMIT},\"deadline_ms\": {}}},\n  \"evidence\": \"source and synthetic unless run on the owner's machine with the product build's runtime\",\n  \"agrees\": {agrees},\n  \"trials\": [\n{}\n  ],\n  \"sessions\": {}\n}}\n",
        clean(runtime_name),
        DEADLINE.as_millis(),
        rows.join(",\n"),
        session::sessions_json(sessions)
    )
}
