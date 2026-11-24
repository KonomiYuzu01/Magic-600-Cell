//! LP4-9: semantic recording, playback and conversion on the command layer (chapter 04 EV4.10 to EV4.23; chapter 05
//! PS1.30 and PS1.44; chapter 13's EV4 codes). Each test names the conformance rows it covers headless.
//!
//! Undo (X1, owner decision of 23 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md`,
//! item 2.7): ADR-D01.14 and .15 (two histories, multi-step rollback) extend to EV4.15, so the undo parts here run on the
//! move history (`session.undo`) and the layout history (`layout.undo`) as the command layer has them. Each test that
//! reads an undo target names its history; where EV4.15's single order would read differently, the test says so.
//!
//! The recorder is `MemoryRecorder`; the session store's recorder and the crash cases on a disk are in `c600-session`
//! and `c600-durable`. The provenance reader is this file's: a block is JSON naming a producer, a profile and parts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use c600_command::recording::{
    self, child, eligibility, historical, mapped, play_from, Advance, Historical, Start, Unattended, UserReason,
};
use c600_command::{
    code, identity_json, ActionHint, CancelReason, ChildOutcome, CommandLayer, Direction, Eligibility, Error, Fate,
    HeadlessFrames, JobOutput, Kind, MemoryRecorder, MemoryStore, ModuleVersions, Overlay, PlayStatus, PlaybackState,
    Provenance, Receipt, RecordValue, RecordedStep, Recording, Secret, Shell, Status, Stop, Table, Target, POLICY_ROW,
    RECORDING_CONVERT_ROW, RECORDING_PLAY_ROW, RECORDING_START_ROW, RECORDING_STOP_ROW, RECORD_ROW, TWIST_ROW,
};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
/// A view row on the view lane with no job flag: unattended under EV4.22. An entry serves it here.
const ROTATE: &str = "camera.rotate";
/// A workspace row with a `RevisionRef` argument and no job flag: unattended. An entry serves it here.
const BLOCK: &str = "block.add_current";
/// A workspace row on the layout history (ADR-D01.14): unattended. An entry serves it here.
const LAYOUT: &str = "work.layout.set";
/// A no-job query on lane `none`: unattended by the integrator's default. An entry serves it here.
const FILTER_PREVIEW: &str = "filter.preview";
/// The predicate row a filter preview reads, and the apply row whose gate is `fresh_filter_preview`.
const PREDICATE_ROW: &str = "structure.layer";
const APPLY: &str = "filter.apply";
/// A language-defined query row: a job row, so a user step.
const TOOL: &str = "tool.m.go";

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

fn letters(moves: &[i64]) -> Value {
    Value::Array(moves.iter().map(|&letter| Value::Int(letter)).collect())
}

fn code_of(receipt: &Receipt) -> Option<&'static str> {
    receipt.status.code()
}

fn status(name: &str) -> Value {
    json::object([("name", text(name))])
}

// ----- the test reader: a block is `{"producer", "profile", "parts"}`

fn block(producer: &str, profile: &str, parts: &[&str]) -> String {
    let parts = Value::Array(parts.iter().map(|part| text(part)).collect());
    json::object([("producer", text(producer)), ("profile", text(profile)), ("parts", parts)]).canonical()
}

fn manual() -> String {
    block("manual", "none", &[])
}

#[derive(Clone, Copy)]
struct Reader;

impl Reader {
    fn read(block: &[u8]) -> Result<Value, Error> {
        json::parse(block)
            .ok()
            .filter(|value| value.get("producer").and_then(Value::as_str).is_some())
            .ok_or_else(|| Error::new(code::PROVENANCE_INCOMPATIBLE, "not a test block"))
    }
}

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        Reader::read(block).map(|_| ())
    }

    /// BND.84 in a solving session, as this file reads it: a program's word is refused, and so is a word with a part
    /// that is refused.
    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        let read = Reader::read(block)?;
        if read.get("producer").and_then(Value::as_str) == Some("program") {
            let profile = read.get("profile").and_then(Value::as_str).unwrap_or_default();
            return Err(Error::new(code::NOT_GRANTED, format!("a program's word ({profile}) is not previewed here")));
        }
        for part in self.parts(block) {
            self.admit(part.as_bytes())?;
        }
        Ok(())
    }

    fn parts(&self, block: &[u8]) -> Vec<String> {
        let parts = Reader::read(block).ok().and_then(|read| read.get("parts").cloned());
        parts
            .and_then(|parts| {
                parts.as_array().map(|p| p.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
            })
            .unwrap_or_default()
    }

    fn assemble(&self, target: &str, source: Option<&str>, parts: &[String]) -> Result<String, Error> {
        assert_eq!(source.is_some(), target == "source", "a source conversion hands its module text");
        let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
        Ok(match target {
            "source" => block("program", "solving", &parts),
            _ => block("manual", "none", &parts),
        })
    }
}

/// Module versions the test changes: the active closure digest of each module, by name.
#[derive(Clone, Default)]
struct Versions(Arc<std::sync::Mutex<BTreeMap<String, String>>>);

impl Versions {
    fn set(&self, module: &str, digest: Option<&str>) {
        let mut map = self.0.lock().unwrap();
        match digest {
            Some(digest) => map.insert(module.to_owned(), digest.to_owned()),
            None => map.remove(module),
        };
    }
}

impl ModuleVersions for Versions {
    fn active(&self, module: &str) -> Option<String> {
        self.0.lock().unwrap().get(module).cloned()
    }
}

// ----- the fixture

struct F {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    recorder: MemoryRecorder,
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/fixture".into(), entry: entry.into() }
}

/// What the host does at every open: the reader, the rows its entries serve, the language-defined row, the recorder.
fn equip(mut layer: CommandLayer, recorder: &MemoryRecorder) -> CommandLayer {
    layer.set_provenance(Box::new(Reader));
    layer.set_settle(Duration::ZERO);
    for (row, entry) in [
        (ROTATE, "rotate"),
        (BLOCK, "block"),
        (LAYOUT, "layout"),
        (FILTER_PREVIEW, "filter_counts"),
        (PREDICATE_ROW, "layer_predicate"),
        (APPLY, "apply_filter"),
    ] {
        layer.serve_row(row, overlay(entry)).unwrap();
    }
    let tool = Overlay { module: "m".into(), entry: "go".into() };
    layer.register_overlay(TOOL, Kind::Query, &[], tool).unwrap();
    layer.set_program_capacity(4);
    layer.set_recorder(Box::new(recorder.clone())).unwrap();
    layer
}

fn fixture() -> F {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let recorder = MemoryRecorder::new();
    let layer = equip(layer, &recorder);
    F { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), recorder }
}

/// A reopen of `f`'s journal and recorder, as after a crash: the journal replays, the recorder's states recover.
fn reopen(f: &F, prefix: &str) -> F {
    let store = f.layer.store().clone();
    let layer = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    let layer = equip(layer, &f.recorder);
    F {
        layer,
        shell: Shell::new(Secret::fixed(SECRET), prefix),
        frames: HeadlessFrames::new(),
        recorder: f.recorder.clone(),
    }
}

impl F {
    fn press(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    fn write(&mut self, name: &str, moves: &[i64]) -> Receipt {
        self.write_with(name, moves, &manual())
    }

    fn write_with(&mut self, name: &str, moves: &[i64], provenance: &str) -> Receipt {
        let args =
            json::object([("payload", letters(moves)), ("provenance", text(provenance)), ("record", text(name))]);
        self.press(RECORD_ROW, args)
    }

    /// Previews `draft` and presents it; the preview's ID.
    fn preview(&mut self, draft: Value) -> u64 {
        let receipt = self.press("operation.preview", json::object([("draft", draft)]));
        let Status::Running(id) = receipt.status else { panic!("the preview: {:?}", receipt.status) };
        self.layer.run_jobs();
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        id
    }

    fn execute(&mut self, id: u64) -> Receipt {
        self.press("operation.execute", json::object([("preview", Value::Int(id as i64))]))
    }

    fn commit(&mut self, moves: &[i64]) -> Receipt {
        let id = self.preview(json::object([("recipe", word(moves))]));
        self.execute(id)
    }

    /// Invokes a served row and reports its job with `output`; the final receipt.
    fn served(&mut self, row: &str, args: Value, output: Value) -> Receipt {
        let receipt = self.press(row, args);
        let Status::Running(job) = receipt.status else { return receipt };
        let taken = self.layer.take_program_job().expect("the row's job");
        assert_eq!(taken.job, job);
        self.layer.finish_program_job_with(job, Ok(JobOutput::of(output))).expect("reported")
    }

    fn rotate(&mut self) -> Receipt {
        self.served(ROTATE, rotate_args(), Value::Null)
    }

    fn start(&mut self, name: &str) {
        let receipt = self.press(RECORDING_START_ROW, json::object([("name", text(name))]));
        assert!(matches!(receipt.status, Status::Completed(_)), "start: {:?}", receipt.status);
    }

    fn stop(&mut self, name: &str) -> Recording {
        let receipt = self.press(RECORDING_STOP_ROW, json::object([("recording", text(name))]));
        assert!(matches!(receipt.status, Status::Completed(_)), "stop: {:?}", receipt.status);
        self.layer.recording(name).expect("stored").clone()
    }

    fn play(&mut self, name: &str, from: Option<i64>) -> Receipt {
        let mut args = vec![("recording".to_owned(), text(name))];
        if let Some(from) = from {
            args.push(("from".to_owned(), Value::Int(from)));
        }
        self.press(RECORDING_PLAY_ROW, Value::Object(args.into_iter().collect()))
    }

    /// Runs the layer's jobs and reports every program job until none is left: a workspace row's job writes the record
    /// `out/<row>`, so its completion is an action an undo reaches; another job returns `null`.
    fn drive(&mut self) {
        loop {
            self.layer.run_jobs();
            let Some(job) = self.layer.take_program_job() else { break };
            let mut output = JobOutput::of(Value::Null);
            if job.kind == Kind::Workspace {
                let value = RecordValue { payload: letters(&[job.job as i64]), provenance: manual() };
                output.records.insert(format!("out/{}", job.row), value);
            }
            self.layer.finish_program_job_with(job.job, Ok(output)).expect("reported");
        }
        self.layer.run_jobs();
    }

    fn state(&self, name: &str) -> PlaybackState {
        self.layer.playback(name).expect("a playback state").clone()
    }

    fn convert(&mut self, name: &str, target: &str) -> Receipt {
        self.press(RECORDING_CONVERT_ROW, json::object([("recording", text(name)), ("target", text(target))]))
    }

    fn log(&self) -> usize {
        self.layer.store().log.len()
    }

    fn receipt(&self, command: &str, request: &str) -> Receipt {
        self.layer.kernel_outcome(&c600_command::Caller::Shell, command, request)
    }

    /// Puts recordings in the recorder, as one read from elsewhere, and reinstalls it.
    fn put(&mut self, recordings: Vec<Recording>) {
        let mut held = self.recorder.held();
        for recording in recordings {
            held.recordings.insert(recording.name.clone(), recording);
        }
        self.recorder.put(held);
        self.layer.set_recorder(Box::new(self.recorder.clone())).unwrap();
    }
}

fn rotate_args() -> Value {
    json::object([("rotation", text("quarter")), ("view", text("main"))])
}

// ----- synthetic recordings

fn receipt_json(command: &str, status: Value) -> Value {
    json::object([("command", text(command)), ("request", text("r")), ("job", Value::Null), ("status", status)])
}

fn completed() -> Value {
    json::object([("name", text("completed")), ("kind", text("workspace"))])
}

fn committed() -> Value {
    json::object([("name", text("committed"))])
}

fn step(row: &str, arguments: Value, outcome: Option<Value>) -> RecordedStep {
    RecordedStep {
        row: row.into(),
        arguments,
        word: None,
        depends: Vec::new(),
        module: None,
        action: None,
        command: "shell/0".into(),
        request: "r".into(),
        outcome: outcome.map(|status| receipt_json("shell/0", status)),
    }
}

/// A committed word step: `operation.execute` with its stored word and its block.
fn word_step(moves: &[i64], action: u64, provenance: Option<&str>) -> RecordedStep {
    RecordedStep {
        word: Some(json::object([
            ("recipe", word(moves)),
            ("source", Value::Null),
            ("provenance", provenance.map_or(Value::Null, text)),
        ])),
        action: Some(action),
        ..step("operation.execute", json::object([]), Some(committed()))
    }
}

/// An undo or redo step of the move history whose recorded hint targets `target`.
fn undo_step(row: &str, target: u64, action: u64) -> RecordedStep {
    let hint = ActionHint { order: action - 1, target }.to_json();
    RecordedStep { action: Some(action), ..step(row, json::object([("hint", hint)]), Some(committed())) }
}

fn recording(name: &str, steps: Vec<RecordedStep>) -> Recording {
    Recording {
        name: name.into(),
        session: "s1".into(),
        identity: identity_json(&kernel().identity()),
        started: Value::Null,
        steps,
    }
}

fn completed_rotate() -> RecordedStep {
    step(ROTATE, rotate_args(), Some(completed()))
}

fn completed_write(name: &str, moves: &[i64], action: u64) -> RecordedStep {
    let args = json::object([("payload", letters(moves)), ("provenance", text(&manual())), ("record", text(name))]);
    RecordedStep { action: Some(action), ..step(RECORD_ROW, args, Some(completed())) }
}

// ----- CT-EV4-11, CT-EV4-08, CT-PS1-15, CT-PS1-24: capture

/// CT-EV4-11 (whole; EV4.10, EV4.11).
/// CT-EV4-08 (part: a recording retains semantic commands, canonical arguments, identities, word and action
/// references, revisions and receipt bindings; not run: that playback has no rebinding operation, which
/// `ct_ev4_08_playback_has_no_rebinding_operation` runs, and source conversion's checks and grants on the real checker,
/// which `c600-durable`'s `tests/ps1_44_carrier.rs` runs).
/// CT-PS1-15 (part: the round trip of a recording whose steps hold no module digest or block; not run: the round
/// trip of a step with a module digest or a word block, which `ct_ps1_15_a_module_digest_and_a_word_block_round_trip`
/// runs).
/// CT-PS1-24 (part: completed, committed and refused receipts round-trip unchanged; no token or operation preview ID
/// is stored; not run: failed and cancelled receipts, and a missing terminal's absence after a round trip, which
/// `ct_ps1_24_every_recorded_outcome_round_trips_and_a_missing_terminal_stays_absent` runs).
/// CT-EV4-19 (part: a start while recording and a stop with none refuse with `EV4:input.recording_state`; not run: the
/// rest of EV4.22's table, which `ct_ev4_19_the_play_table_rewinds_and_job_stop` runs).
///
/// A completed record write, a completed preview, a
/// committed word, a refused policy change and a completed view step each append one step; an unresolved row, a retry
/// that the outcome lookup answers and the `recording.*` rows append none. A served step still running at the stop is
/// recorded `running`, never as a completion. No stored argument holds a token, the operation preview ID or a
/// confirmation, and no token's binding or nonce appears anywhere in the recording's bytes.
#[test]
fn ct_ev4_11_capture_appends_one_step_per_resolved_invocation() {
    let mut f = fixture();
    f.start("r1");
    assert_eq!(
        code_of(&f.press(RECORDING_START_ROW, json::object([("name", text("r2"))]))),
        Some(code::RECORDING_STATE)
    );
    assert!(matches!(f.write("w", &[1]).status, Status::Completed(_)));
    let id = f.preview(json::object([("recipe", word(&[2, -3]))]));
    let press =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))])).unwrap();
    let binding = press.token.as_ref().expect("a commit's token").binding.to_json().canonical();
    let committed = f.shell.send(&mut f.layer, press.clone());
    assert!(matches!(committed.status, Status::Committed(_)), "{:?}", committed.status);
    assert_eq!(f.shell.send(&mut f.layer, press), committed, "a retry answers from the lookup");
    let policy = json::object([("confirm", Value::Bool(false)), ("policy", text("strict"))]);
    assert_eq!(code_of(&f.press(POLICY_ROW, policy)), Some(code::CONFIRMATION_REQUIRED));
    assert!(f.shell.invoke(&mut f.layer, "no.such.row", json::object([])).is_err(), "the shell resolves no row");
    assert!(matches!(f.rotate().status, Status::Completed(_)));
    let pending = f.press(ROTATE, rotate_args());
    assert!(matches!(pending.status, Status::Running(_)));
    let r1 = f.stop("r1");

    let rows: Vec<&str> = r1.steps.iter().map(|step| step.row.as_str()).collect();
    assert_eq!(rows, [RECORD_ROW, "operation.preview", "operation.execute", POLICY_ROW, ROTATE, ROTATE]);
    let names: Vec<Option<&str>> = r1.steps.iter().map(RecordedStep::status).collect();
    let expected = ["completed", "completed", "committed", "refused", "completed", "running"].map(Some);
    assert_eq!(names, expected);
    assert_eq!(historical(&r1.steps[5]), Historical::Unknown, "a pending step never masquerades as a completion");
    for (i, step) in r1.steps.iter().enumerate() {
        let receipt = f.receipt(&step.command, &step.request);
        assert_eq!(step.outcome.as_ref(), Some(&receipt.to_json()), "step {i}: the receipt binding");
        let fields = step.arguments.as_object().unwrap();
        assert!(!fields.contains_key("token") && !fields.contains_key("confirm"), "step {i}: {fields:?}");
        assert_eq!(step.depends.first(), Some(&step.depends[0]), "step {i}: the revision it read");
    }
    assert_eq!(r1.steps[2].arguments, json::object([]), "no operation preview ID");
    assert_eq!(r1.steps[2].word.as_ref().and_then(|w| w.get("recipe")), Some(&word(&[2, -3])));
    assert_eq!((r1.steps[0].action, r1.steps[2].action), (Some(1), Some(2)), "the actions the steps took (LI.9)");
    assert_eq!(r1.steps[3].arguments, json::object([("policy", text("strict"))]), "no confirmation");
    let bytes = r1.to_json().canonical();
    assert!(!bytes.contains(&binding) && !bytes.contains("\"nonce\""), "no token is stored");
    assert_eq!(r1.identity, identity_json(&kernel().identity()));
    assert_eq!(Recording::from_json(&r1.to_json()), Some(r1.clone()), "every field round-trips");
    assert_eq!(f.recorder.held().recordings.get("r1"), Some(&r1));
    for (row, args) in [
        (RECORDING_STOP_ROW, json::object([("recording", text("r1"))])),
        (RECORDING_STOP_ROW, json::object([("recording", text("r9"))])),
    ] {
        assert_eq!(code_of(&f.press(row, args)), Some(code::RECORDING_STATE), "stop with no recording");
    }
}

/// CT-EV4-08 (part: by union with `ct_ev4_11_capture_appends_one_step_per_resolved_invocation`, which runs what a
/// recording retains, and `c600-durable`'s `tests/ps1_44_carrier.rs`, which runs a source conversion through the real
/// checker and the module set's grants: here, that playback has no rebinding operation; EV4.10 to EV4.14).
/// The command table's recording rows are the four, and their arguments name a recording, a capture's name, a step and
/// a target: none names a step's argument, module or revision to replace. An unattended step is played as a new
/// invocation of its recorded row with its recorded canonical arguments, and its job is bound to the module digest the
/// step recorded. A user step's action submits the recorded arguments: one that submits other arguments is refused
/// with `EV4:input.recording_state` and writes nothing, and the step action that follows plays the step as recorded.
/// The recording is the same after the playback.
#[test]
fn ct_ev4_08_playback_has_no_rebinding_operation() {
    // Every row that names a recording takes only a recording, a capture's name, a step (`from`) and a target.
    let table = Table::h08().with_drafts();
    let named: Vec<&c600_command::Row> = table
        .rows()
        .filter(|row| row.id.starts_with("recording.") || row.arguments.values().any(|ty| ty == "RecordingRef"))
        .collect();
    for row in &named {
        let kinds: Vec<&str> = row.arguments.values().map(String::as_str).collect();
        let replaces = kinds.iter().any(|ty| !matches!(*ty, "RecordingRef" | "Text" | "Nat?" | "draft | source"));
        assert!(!replaces, "{} takes {kinds:?}: no recording row rebinds a step", row.id);
    }
    let ids: Vec<&str> = named.iter().map(|row| row.id.as_str()).collect();
    for id in [RECORDING_START_ROW, RECORDING_STOP_ROW, RECORDING_PLAY_ROW, RECORDING_CONVERT_ROW] {
        assert!(ids.contains(&id), "{id}");
    }

    let mut f = fixture();
    let digest = "ab".repeat(32);
    let versions = Versions::default();
    versions.set("tools/fixture", Some(&digest));
    f.layer.set_module_versions(Box::new(versions));
    let rotated = RecordedStep { module: Some(digest.clone()), ..completed_rotate() };
    let save = json::object([("name", text("m")), ("word", json::object([("recipe", word(&[1]))]))]);
    let b = recording("b", vec![rotated.clone(), step("macro.save", save.clone(), Some(completed()))]);
    f.put(vec![b.clone()]);
    let played = f.play("b", None);
    assert!(matches!(played.status, Status::Running(_)), "{:?}", played.status);
    f.layer.run_jobs();
    let child = f.layer.take_program_job().expect("the rotation's job");
    assert_eq!((child.row.as_str(), &child.args), (ROTATE, &rotated.arguments), "the recorded canonical arguments");
    assert_eq!(child.module.as_deref(), Some(digest.as_str()), "bound to the module digest the step recorded");
    assert!(child.command != rotated.command && child.command.starts_with(&f.state("b").playback), "a new one");
    f.layer.finish_program_job_with(child.job, Ok(JobOutput::of(Value::Null))).expect("reported");
    f.drive();
    assert_eq!(f.state("b").status, PlayStatus::Waiting(1));

    let mut other = save.as_object().cloned().unwrap();
    other.insert("name".into(), text("other"));
    other.insert("confirm".into(), Value::Bool(true));
    let rebound = f.shell.prepare(&f.layer, "macro.save", Value::Object(other)).unwrap();
    let (writes, before) = (f.recorder.writes(), f.state("b"));
    assert_eq!(f.layer.begin_step("b", &rebound).unwrap_err().code, code::RECORDING_STATE, "other arguments");
    assert_eq!((f.recorder.writes(), f.state("b")), (writes, before), "nothing written");
    let saved = f.shell.step(&mut f.layer, "b", true).unwrap();
    assert!(matches!(saved.status, Status::Completed(_)), "{:?}", saved.status);
    assert!(f.layer.record("macro/m").is_some() && f.layer.record("macro/other").is_none(), "played as recorded");
    assert_eq!(f.state("b").status, PlayStatus::Finished);
    assert_eq!(f.layer.recording("b"), Some(&b), "the recording is the same");
}

// ----- CT-PS1-15 and CT-PS1-24: the round trip of every field and every recorded outcome

/// The recording `name` after a round trip through its canonical bytes (PS1.30's draft form), the memory recorder and
/// a reopen of the journal and the recorder.
fn round_trips(f: &F, name: &str) -> Recording {
    let recording = f.layer.recording(name).expect("stored").clone();
    let bytes = recording.to_json().canonical();
    let read = Recording::from_json(&json::parse(bytes.as_bytes()).unwrap()).expect("the bytes read");
    assert_eq!(read, recording, "every field round-trips through the bytes");
    for step in &recording.steps {
        assert_eq!(RecordedStep::from_json(&step.to_json()).as_ref(), Some(step), "{}", step.row);
    }
    assert_eq!(f.recorder.held().recordings.get(name), Some(&recording), "the recorder holds it");
    let g = reopen(f, "again");
    assert_eq!(g.layer.recording(name), Some(&recording), "a reopen reads it back");
    recording
}

/// CT-PS1-15 (part: by union with `ct_ev4_09_no_token_is_stored_or_replayed`, which plays commits with a new explicit
/// action and token, and `ct_ev4_11_capture_appends_one_step_per_resolved_invocation`: here, the round trip of steps
/// with a module digest and a word block; PS1.30, EV4.10). A capture holds a rotation and a block step that an entry of
/// a user module's active version served, each with that version's digest, the block step with the revision it named;
/// a preview of a record's word and its commit, whose word step keeps the word, its source and the record's block and
/// names the action it took; and a record write with its action. Every field round-trips unchanged through the
/// canonical bytes, the recorder and a reopen: canonical arguments, word and action references, revisions, module
/// digests, receipt bindings and the provenance block.
#[test]
fn ct_ps1_15_a_module_digest_and_a_word_block_round_trip() {
    let mut f = fixture();
    let digest = "cd".repeat(32);
    let versions = Versions::default();
    versions.set("tools/fixture", Some(&digest));
    f.layer.set_module_versions(Box::new(versions));
    let own = block("manual", "none", &[&manual()]);
    assert!(matches!(f.write_with("word", &[1, 2], &own).status, Status::Completed(_)));
    f.start("all");
    assert!(matches!(f.rotate().status, Status::Completed(_)));
    let revision = f.layer.revision().to_json();
    let args = json::object([("mode", text("exact")), ("positions", letters(&[3])), ("revision", revision.clone())]);
    assert!(matches!(f.served(BLOCK, args.clone(), Value::Null).status, Status::Completed(_)));
    let id = f.preview(json::object([("record", text("word"))]));
    assert!(matches!(f.execute(id).status, Status::Committed(_)));
    assert!(matches!(f.write("w", &[3]).status, Status::Completed(_)));
    f.stop("all");

    let all = round_trips(&f, "all");
    let rows: Vec<&str> = all.steps.iter().map(|step| step.row.as_str()).collect();
    assert_eq!(rows, [ROTATE, BLOCK, "operation.preview", "operation.execute", RECORD_ROW]);
    let modules: Vec<Option<&str>> = all.steps.iter().map(|step| step.module.as_deref()).collect();
    assert_eq!(modules, [Some(digest.as_str()), Some(digest.as_str()), None, None, None], "the served steps' digest");
    assert_eq!((all.steps[1].arguments.clone(), all.steps[1].depends.get(1)), (args, Some(&revision)));
    let execute = &all.steps[3];
    assert_eq!(execute.block(), Some(own.as_str()), "the word step keeps the record's block");
    assert_eq!(execute.word.as_ref().and_then(|w| w.get("recipe")), Some(&word(&[1, 2])));
    assert!(execute.action.is_some() && all.steps[4].action.is_some(), "the actions the steps took");
    for (i, step) in all.steps.iter().enumerate() {
        let receipt = f.receipt(&step.command, &step.request);
        assert_eq!(step.outcome.as_ref(), Some(&receipt.to_json()), "step {i}: the receipt binding");
    }
}

/// CT-PS1-24 (whole; PS1.30, EV4.11, EV4.19). One capture holds a completed record write and preview, a committed word,
/// a refused policy change, a rotation whose entry failed, a rotation a newer request superseded (`cancelled`) and the
/// rotation that superseded it: each step's outcome is its invocation's terminal receipt, and every one round-trips
/// unchanged through the bytes, the recorder and a reopen, with no token, binding, nonce, confirmation or operation
/// preview ID stored. A second capture holds a rotation whose transient receipt a restart lost before the stop: it
/// has no terminal record, and it stays absent after the round trip; playback and both conversions stop at that step
/// with `EV4:input.recording_step_outcome_unknown`, filling nothing in. A played child's new binding is kept in the
/// playback state and never replaces the captured invocation.
#[test]
fn ct_ps1_24_every_recorded_outcome_round_trips_and_a_missing_terminal_stays_absent() {
    let mut f = fixture();
    f.start("o");
    assert!(matches!(f.write("w", &[1]).status, Status::Completed(_)));
    let id = f.preview(json::object([("recipe", word(&[2]))]));
    let press =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))])).unwrap();
    let binding = press.token.as_ref().expect("a commit's token").binding.to_json().canonical();
    assert!(matches!(f.shell.send(&mut f.layer, press).status, Status::Committed(_)));
    let policy = json::object([("confirm", Value::Bool(false)), ("policy", text("strict"))]);
    assert_eq!(code_of(&f.press(POLICY_ROW, policy)), Some(code::CONFIRMATION_REQUIRED));
    let failing = f.press(ROTATE, rotate_args());
    let Status::Running(job) = failing.status else { panic!("{:?}", failing.status) };
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(job));
    let failed = f.layer.finish_program_job_with(job, Err(Error::new(code::ROW_NOT_BUILT, "the entry failed")));
    assert!(matches!(failed.map(|r| r.status), Some(Status::Failed(_))));
    let superseded = f.press(ROTATE, rotate_args());
    assert!(matches!(superseded.status, Status::Running(_)), "{:?}", superseded.status);
    assert!(matches!(f.press(ROTATE, rotate_args()).status, Status::Running(_)));
    f.drive();
    f.stop("o");

    let o = round_trips(&f, "o");
    let names: Vec<Option<&str>> = o.steps.iter().map(RecordedStep::status).collect();
    let want = ["completed", "completed", "committed", "refused", "failed", "cancelled", "completed"].map(Some);
    assert_eq!(names, want);
    for (i, step) in o.steps.iter().enumerate() {
        let receipt = f.receipt(&step.command, &step.request);
        assert_eq!(step.outcome.as_ref(), Some(&receipt.to_json()), "step {i}: the receipt, unchanged");
    }
    let bytes = o.to_json().canonical();
    for held in [binding.as_str(), "\"nonce\"", "\"token\"", "\"confirm\""] {
        assert!(!bytes.contains(held), "the recording holds {held}");
    }
    assert_eq!(o.steps[2].arguments, json::object([]), "no operation preview ID");

    // A played child: a new command and request, kept in the state; the captured step is never replaced.
    let played = f.play("o", None);
    assert!(matches!(played.status, Status::Running(_)), "{:?}", played.status);
    f.drive();
    let state = f.state("o");
    assert_eq!(state.status, PlayStatus::Waiting(1), "the preview is a user step");
    let last = state.last.as_ref().expect("the record write's child");
    assert_ne!(last.get("command"), Some(&text(&o.steps[0].command)));
    assert_eq!(f.layer.recording("o"), Some(&o), "the captured invocation stays");

    // A missing terminal.
    f.start("m");
    let lost = f.press(ROTATE, rotate_args());
    assert!(matches!(lost.status, Status::Running(_)), "{:?}", lost.status);
    f.layer.restart(Secret::fixed(SECRET));
    f.stop("m");
    let m = round_trips(&f, "m");
    assert_eq!((m.steps.len(), m.steps[0].outcome.as_ref(), m.steps[0].status()), (1, None, None), "absent");
    assert_eq!(historical(&m.steps[0]), Historical::Unknown);
    let played = f.play("m", None);
    f.drive();
    let stop = Stop::Code(code::RECORDING_STEP_OUTCOME_UNKNOWN.to_owned());
    assert_eq!((f.state("m").step, f.state("m").status), (0, PlayStatus::Stopped(stop)));
    assert_eq!(code_of(&f.receipt(&played.command, &played.request)), Some(code::RECORDING_STEP_OUTCOME_UNKNOWN));
    for target in ["draft", "source"] {
        assert_eq!(code_of(&f.convert("m", target)), Some(code::RECORDING_STEP_OUTCOME_UNKNOWN), "{target}");
    }
    assert_eq!(f.layer.recording("m"), Some(&m), "still absent: nothing filled it in");
}

// ----- CT-EV4-12: the eligibility table

/// EV4.22's table, as this test reads it, for a row of `kind`, `lane`, `job` and `gates`.
fn expected(kind: &str, lane: &str, job: bool, gates: &[&str]) -> bool {
    let gated = gates.contains(&"confirm") || gates.contains(&"file_check");
    let user =
        lane == "commit" || kind == "commit" || gated || kind == "file" || kind == "app" || lane == "exclusive" || job;
    !user && matches!((kind, lane), ("view" | "workspace", "view" | "none") | ("query", "none"))
}

/// CT-EV4-12 (part: the table enumerated whole and H-08's rows; not run: rows played live, which
/// `ct_ev4_12_playback_plays_only_the_unattended_rows` and
/// `ct_ev4_12_a_query_plays_and_file_app_and_exclusive_rows_wait` run, and the filter freshness check at play, which
/// the `ct_ev4_16` tests run). EV4.17, EV4.18, EV4.22: the eligibility table
/// enumerated whole, across every kind, lane, job flag and
/// gate that H-08's grammar has (6 kinds, 5 lanes, 2 job flags, 4 gate sets: 240 rows), against the test's own reading
/// of the table; then every row of H-08 with the drafts. Kind `view` or `workspace` on lane `view` or `none` and kind
/// `query` on lane `none`, with no job and no `confirm` or `file_check` gate, are unattended; every commit-lane,
/// `confirm`, `file_check`, `file`, `app`, exclusive and job row waits. Of H-08's rows, the integrator's additions also
/// wait: the rows whose change the layer confirms itself and the `recording.*` rows. Camera rotation, an eligible
/// workspace edit and a no-job query in lane none play (`tests/recording.rs`'s other tests play them); the gate
/// `fresh_filter_preview` keeps a row eligible, and its freshness is checked when it plays.
#[test]
fn ct_ev4_12_the_eligibility_table_enumerated_whole() {
    let kinds = ["view", "workspace", "query", "file", "app", "commit"];
    let lanes = ["none", "view", "program", "exclusive", "commit"];
    let gate_sets: [&[&str]; 4] = [&[], &["confirm"], &["file_check"], &["fresh_filter_preview"]];
    let mut rows = Vec::new();
    for kind in kinds {
        for lane in lanes {
            for job in [false, true] {
                for (g, gates) in gate_sets.iter().enumerate() {
                    let id = format!("t.{kind}.{lane}.{job}.{g}");
                    let gates: Vec<Value> = gates.iter().map(|gate| text(gate)).collect();
                    rows.push(json::object([
                        ("id", text(&id)),
                        ("kind", text(kind)),
                        ("lane", text(lane)),
                        ("undo", text("none")),
                        ("preview", text("none")),
                        ("gates", Value::Array(gates)),
                        ("job", Value::Bool(job)),
                    ]));
                }
            }
        }
    }
    let table = Table::parse(json::object([("commands", Value::Array(rows))]).canonical().as_bytes()).unwrap();
    let mut unattended = 0;
    for row in table.rows() {
        let parts: Vec<&str> = row.id.split('.').collect();
        let gates: Vec<&str> = gate_sets[parts[4].parse::<usize>().unwrap()].to_vec();
        let want = expected(parts[1], parts[2], parts[3] == "true", &gates);
        let got = eligibility(row);
        assert_eq!(got.unattended(), want, "{}: {got:?}", row.id);
        unattended += usize::from(want);
        if !want && (parts[2] == "commit" || parts[1] == "commit") {
            assert_eq!(got, Eligibility::User(UserReason::CommitLane), "{}", row.id);
        }
    }
    // view|workspace on view|none, query on none; no job; gates none or fresh_filter_preview: 5 x 2 = 10.
    assert_eq!((table.rows().count(), unattended), (240, 10));

    let h08 = Table::h08().with_drafts();
    for row in h08.rows() {
        let gates: Vec<&str> = row.gates.iter().map(String::as_str).collect();
        let kind = format!("{:?}", row.kind).to_lowercase();
        let lane = format!("{:?}", row.lane).to_lowercase();
        let added = row.id == POLICY_ROW
            || row.id == "protection.orbit.set"
            || row.action_guard.is_some()
            || recording::recording_row(&row.id);
        let want = expected(&kind, &lane, row.job, &gates) && !added;
        assert_eq!(eligibility(row).unattended(), want, "{}: {:?}", row.id, eligibility(row));
    }
    let of = |id: &str| eligibility(h08.row(id).unwrap());
    assert_eq!(of(ROTATE), Eligibility::Unattended(Unattended::ViewOrWorkspace));
    assert_eq!(of(RECORD_ROW), Eligibility::Unattended(Unattended::ViewOrWorkspace));
    assert_eq!(of(FILTER_PREVIEW), Eligibility::Unattended(Unattended::QueryDefault));
    assert_eq!(of(APPLY), Eligibility::Unattended(Unattended::ViewOrWorkspace), "the filter gate keeps it eligible");
    assert_eq!(of("session.checkpoint.save"), Eligibility::User(UserReason::CommitLane));
    assert_eq!(of("operation.execute"), Eligibility::User(UserReason::CommitLane));
    assert_eq!(of("macro.save"), Eligibility::User(UserReason::Confirm));
    assert_eq!(of("protection.orbit.set"), Eligibility::User(UserReason::Confirm));
    assert_eq!(of(POLICY_ROW), Eligibility::User(UserReason::Confirmation));
    assert_eq!(of("operation.preview"), Eligibility::User(UserReason::ExclusiveLane));
    assert_eq!(of("job.stop"), Eligibility::User(UserReason::FileOrApp));
    assert_eq!(of(RECORDING_PLAY_ROW), Eligibility::User(UserReason::Job));
    assert_eq!(of(RECORDING_START_ROW), Eligibility::User(UserReason::Recording));
}

/// CT-EV4-12 (part: a workspace edit, a camera rotation and a layout step play unattended, and confirm, policy, job
/// and commit rows wait; not run: a no-job query played live and file, app, `file_check` and exclusive rows
/// waiting, which `ct_ev4_12_a_query_plays_and_file_app_and_exclusive_rows_wait` runs). One recording holds a
/// workspace edit, a camera rotation, a layout step, a `confirm` row, a policy change, a language-defined job row and
/// a word commit. The first three play
/// unattended, each as a new invocation; the playback stops at each user step, where `recording.play` finishes, and
/// the solver's step action plays it. Nothing the playback dispatches is a commit: the journal holds only the edit's
/// and the layout step's actions until the solver's own presses.
#[test]
fn ct_ev4_12_playback_plays_only_the_unattended_rows() {
    let mut f = fixture();
    let r = recording(
        "mixed",
        vec![
            completed_write("w", &[1], 1),
            completed_rotate(),
            step(LAYOUT, json::object([("layout", text("wide"))]), Some(completed())),
            step(
                "macro.save",
                json::object([("name", text("m")), ("word", json::object([("recipe", word(&[1]))]))]),
                Some(completed()),
            ),
            step(POLICY_ROW, json::object([("policy", text("strict"))]), Some(completed())),
            step(TOOL, json::object([]), Some(json::object([("name", text("completed")), ("kind", text("query"))]))),
            word_step(&[4], 3, None),
        ],
    );
    f.put(vec![r]);
    let before = f.log();
    let played = f.play("mixed", None);
    assert!(matches!(played.status, Status::Running(_)), "{:?}", played.status);
    f.drive();
    let state = f.state("mixed");
    assert_eq!((state.step, &state.status), (3, &PlayStatus::Waiting(3)), "the confirm row waits");
    assert_eq!(f.log(), before + 2, "the edit and the layout step, no commit");
    assert_eq!(f.layer.record("w").map(|v| v.payload.clone()), Some(letters(&[1])));
    assert!(matches!(f.receipt(&played.command, &played.request).status, Status::Completed(_)), "the play finished");

    // Each user step: the shell's step action, with the solver's confirmation where the row needs one.
    let saved = f.shell.step(&mut f.layer, "mixed", true).unwrap();
    assert!(matches!(saved.status, Status::Completed(_)), "macro.save: {:?}", saved.status);
    assert_eq!(f.state("mixed").status, PlayStatus::Waiting(4));
    let refused = f.shell.step(&mut f.layer, "mixed", false).unwrap();
    assert_eq!(code_of(&refused), Some(code::CONFIRMATION_REQUIRED), "no recorded confirmation revives");
    assert_eq!(f.state("mixed").status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::Refused)));
    let again = f.play("mixed", Some(4));
    assert!(matches!(again.status, Status::Running(_)), "{:?}", again.status);
    f.drive();
    assert_eq!(f.state("mixed").status, PlayStatus::Waiting(4));
    assert!(matches!(f.shell.step(&mut f.layer, "mixed", true).unwrap().status, Status::Completed(_)));
    let tool = f.shell.step(&mut f.layer, "mixed", false).unwrap();
    assert!(matches!(tool.status, Status::Running(_)), "the tool's job: {:?}", tool.status);
    f.drive();
    assert_eq!(f.state("mixed").status, PlayStatus::Waiting(6), "a word commit waits");
    let fresh = f.shell.step_preview(&mut f.layer, "mixed").unwrap();
    let Status::Running(id) = fresh.status else { panic!("{:?}", fresh.status) };
    f.layer.run_jobs();
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    let commit = f.shell.step(&mut f.layer, "mixed", false).unwrap();
    assert!(matches!(commit.status, Status::Committed(_)), "{:?}", commit.status);
    assert_eq!(f.state("mixed").status, PlayStatus::Finished);
}

/// A predicate the filter reads: a `structure.layer` job writes record `layer`; its `FilterRef` names the job and its
/// output. The job's completion is a workspace action on the move history.
fn predicate(f: &mut F) -> Value {
    let receipt = f.press(PREDICATE_ROW, json::object([("cell", Value::Int(1)), ("layer", Value::Int(2))]));
    let Status::Running(job) = receipt.status else { panic!("the predicate: {:?}", receipt.status) };
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(job));
    let mut output = JobOutput::of(Value::Null);
    output.records.insert("layer".into(), RecordValue { payload: letters(&[1, 2, 3]), provenance: manual() });
    let digest = output.digest();
    let done = f.layer.finish_program_job_with(job, Ok(output)).expect("reported");
    assert!(matches!(done.status, Status::Completed(_)), "{:?}", done.status);
    json::object([("digest", text(&digest)), ("job", Value::Int(job as i64))])
}

/// `filter.preview`'s arguments for `predicate` at the current revision.
fn filter_args(f: &F, predicate: &Value) -> Value {
    json::object([("predicate", predicate.clone()), ("revision", f.layer.revision().to_json())])
}

/// A filter preview of `predicate` that the layer keeps: its job and its `FilterPreviewRef`.
fn kept_filter(f: &mut F, predicate: &Value) -> (u64, Value) {
    let receipt = f.press(FILTER_PREVIEW, filter_args(f, predicate));
    let Status::Running(job) = receipt.status else { panic!("the filter preview: {:?}", receipt.status) };
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(job));
    let counts = json::object([("kept", Value::Int(12)), ("total", Value::Int(600))]);
    let done = f.layer.finish_program_job_with(job, Ok(JobOutput::of(counts))).expect("reported");
    assert!(matches!(done.status, Status::Completed(_)), "{:?}", done.status);
    (job, f.layer.filter_preview(job).expect("kept").reference())
}

/// A completed query's recorded outcome.
fn query_done() -> Value {
    json::object([("name", text("completed")), ("kind", text("query"))])
}

/// The receipt of the child the playback of `name` settled last.
fn last_child(f: &F, name: &str) -> Receipt {
    let state = f.state(name);
    let last = state.last.as_ref().expect("a settled child");
    let field = |key: &str| last.get(key).and_then(Value::as_str).unwrap().to_owned();
    f.receipt(&field("command"), &field("request"))
}

/// CT-EV4-12 (part: by union with `ct_ev4_12_the_eligibility_table_enumerated_whole`, which enumerates the table,
/// `ct_ev4_12_playback_plays_only_the_unattended_rows`, and `ct_ev4_16_stale_steps_stop_and_identities_refuse` and
/// `ct_ev4_16_a_kept_filter_preview_gone_stale_stops_unretargeted`, which check filter freshness at play without
/// retargeting: here, a no-job query in lane none played live, and file, app, `file_check` and exclusive rows waiting
/// live; EV4.17, EV4.18, EV4.22). A `filter.preview`, a query on lane `none` with no job flag, plays unattended by the
/// integrator's default as a new invocation and completes as a query: it takes no action and moves no undo target. A
/// keymap export (`file`), `job.stop` (`app`), a keymap apply (`file_check` and `confirm`) and a log check (exclusive
/// lane and a job) each wait for the solver's step action, and nothing is dispatched for them.
#[test]
fn ct_ev4_12_a_query_plays_and_file_app_and_exclusive_rows_wait() {
    let mut f = fixture();
    let predicate = predicate(&mut f);
    let query = step(FILTER_PREVIEW, filter_args(&f, &predicate), Some(query_done()));
    let waits = [
        ("file.keymap.export", json::object([("destination", text("keys.json")), ("keymap", text("current"))])),
        ("job.stop", json::object([("job", Value::Int(1))])),
        ("file.keymap.apply", json::object([("checked", text("keys.json"))])),
        ("file.log.check", json::object([("file", text("log.json"))])),
    ];
    let mut steps = vec![query];
    steps.extend(waits.iter().map(|(row, args)| step(row, args.clone(), Some(completed()))));
    f.put(vec![recording("q", steps)]);
    let (log, undo) = (f.log(), f.layer.undo_hint(Direction::Undo));
    let played = f.play("q", None);
    assert!(matches!(played.status, Status::Running(_)), "{:?}", played.status);
    f.drive();
    assert_eq!(f.state("q").status, PlayStatus::Waiting(1), "the query played; the file row waits");
    let child = last_child(&f, "q");
    let Status::Completed(done) = &child.status else { panic!("{:?}", child.status) };
    assert_eq!(done.kind, c600_command::CompletionKind::Query, "a query, by the default");
    assert!(child.command.starts_with(&f.state("q").playback), "a new invocation: {}", child.command);
    assert_eq!((f.log(), f.layer.undo_hint(Direction::Undo)), (log, undo), "no action and no undo target");
    for (at, (row, _)) in waits.iter().enumerate().skip(1) {
        let at = at as u64 + 1;
        assert!(matches!(f.play("q", Some(at as i64)).status, Status::Running(_)), "{row}");
        f.layer.run_jobs();
        assert!(f.layer.take_program_job().is_none(), "{row}: nothing is dispatched");
        assert_eq!(f.state("q").status, PlayStatus::Waiting(at), "{row} waits");
    }
    assert_eq!(f.log(), log);
}

// ----- CT-EV4-13: EV4.19's two tables

/// CT-EV4-13 (whole; EV4.19): both tables. Historical: `completed` plays (or waits when not eligible), `committed`
/// waits, `refused`, `failed` and `cancelled` are passed over and stay in the recording, and `accepted`, `running`, no
/// terminal record and `unknown_outcome` stop playback and conversion with `EV4:input.recording_step_outcome_unknown`.
/// New child: `accepted` and `running` are pending, `completed` advances either step, `committed` advances only a user
/// step, and `refused`, `failed`, `cancelled` and `unknown_outcome` stop with that outcome.
/// CT-ERR-18 (part: an unknown recorded outcome stops play and conversion with `recording_step_outcome_unknown`; not
/// run: the stale, identity, state and forwarding clauses, which other tests run).
#[test]
fn ct_ev4_13_every_historical_and_child_outcome() {
    for (name, want) in [
        (Some("completed"), Historical::Completed),
        (Some("committed"), Historical::Committed),
        (Some("refused"), Historical::PassedOver),
        (Some("failed"), Historical::PassedOver),
        (Some("cancelled"), Historical::PassedOver),
        (Some("accepted"), Historical::Unknown),
        (Some("running"), Historical::Unknown),
        (Some("unknown_outcome"), Historical::Unknown),
        (None, Historical::Unknown),
    ] {
        assert_eq!(historical(&step(ROTATE, rotate_args(), name.map(status))), want, "{name:?}");
    }
    let revision = kernel_revision();
    let done = Status::Completed(c600_command::Completion {
        kind: c600_command::CompletionKind::Workspace,
        revision: revision.clone(),
        result: None,
    });
    let commit = Status::Committed(c600_command::CommitRecord {
        event: "e1".into(),
        pre: "a".into(),
        post: "b".into(),
        revision,
    });
    let error = Error::new(code::ROW_NOT_BUILT, "x");
    for unattended in [true, false] {
        assert_eq!(child(&Status::Accepted, unattended), Advance::Pending);
        assert_eq!(child(&Status::Running(1), unattended), Advance::Pending);
        assert_eq!(child(&done, unattended), Advance::Advances);
        assert_eq!(child(&commit, unattended), if unattended { Advance::Held } else { Advance::Advances });
        assert_eq!(child(&Status::Refused(error.clone()), unattended), Advance::Stops(ChildOutcome::Refused));
        assert_eq!(child(&Status::Failed(error.clone()), unattended), Advance::Stops(ChildOutcome::Failed));
        assert_eq!(
            child(&Status::Cancelled(CancelReason::ByUser), unattended),
            Advance::Stops(ChildOutcome::Cancelled)
        );
        assert_eq!(child(&Status::UnknownOutcome, unattended), Advance::Stops(ChildOutcome::UnknownOutcome));
    }

    // At playback: passed-over steps stay visible and are skipped; a missing terminal stops play and conversion.
    let mut f = fixture();
    let skipped = ["refused", "failed", "cancelled"].map(|name| step(ROTATE, rotate_args(), Some(status(name))));
    let mut steps = skipped.to_vec();
    steps.push(completed_rotate());
    steps.push(step(ROTATE, rotate_args(), Some(status("running"))));
    steps.push(completed_rotate());
    f.put(vec![recording("h", steps)]);
    let played = f.play("h", None);
    f.drive();
    let state = f.state("h");
    let stop = Stop::Code(code::RECORDING_STEP_OUTCOME_UNKNOWN.to_owned());
    assert_eq!((state.step, &state.status), (4, &PlayStatus::Stopped(stop)));
    assert_eq!(f.layer.recording("h").unwrap().steps.len(), 6, "the passed-over steps stay in the recording");
    let answer = f.receipt(&played.command, &played.request);
    assert_eq!(code_of(&answer), Some(code::RECORDING_STEP_OUTCOME_UNKNOWN));
    assert_eq!(code_of(&f.convert("h", "draft")), Some(code::RECORDING_STEP_OUTCOME_UNKNOWN));

    // New children: a refusal (a row nothing serves), a failure (the entry fails), a cancellation (a newer request of
    // the view row supersedes the step) and a lost outcome (a restart) each stop with that outcome, with no retry.
    for (outcome, how) in [
        (ChildOutcome::Refused, "unserved"),
        (ChildOutcome::Failed, "fails"),
        (ChildOutcome::Cancelled, "superseded"),
        (ChildOutcome::UnknownOutcome, "restart"),
    ] {
        let mut f = fixture();
        let first = match how {
            "unserved" => step("session.timer.start", json::object([]), Some(completed())),
            _ => completed_rotate(),
        };
        f.put(vec![recording("c", vec![first, completed_rotate()])]);
        f.play("c", None);
        f.layer.run_jobs();
        match how {
            "fails" => {
                let job = f.layer.take_program_job().unwrap();
                f.layer.finish_program_job_with(job.job, Err(Error::new(code::ROW_NOT_BUILT, "the entry failed")));
            }
            "superseded" => {
                let _ = f.press(ROTATE, rotate_args());
            }
            "restart" => f.layer.restart(Secret::fixed(SECRET)),
            _ => {}
        }
        let writes = f.recorder.writes();
        f.layer.run_jobs();
        let state = f.state("c");
        assert_eq!((state.step, &state.status), (0, &PlayStatus::Stopped(Stop::Outcome(outcome))), "{how}");
        assert!(state.pending.is_none() && state.last.is_some(), "{how}: the receipt is kept in `last`");
        f.drive();
        assert_eq!(f.recorder.writes(), writes, "{how}: no retry");
    }
}

fn kernel_revision() -> c600_command::Revision {
    fixture().layer.revision().clone()
}

// ----- CT-EV4-15: undo classification (X1: two histories)

/// CT-EV4-15 (part: the edit and the layout step become undo targets, the rotation, the timer row and the cursor
/// invent none, and a later failure keeps earlier actions; not run: a no-job query step played, which
/// `ct_ev4_15_a_query_an_edit_and_a_rotation_keep_their_undo_classes` runs). EV4.21; X1, owner decision of 23 November
/// 2026, item 2.7. A playback of an undoable workspace edit
/// (`workspace.set`, on the move history: EV4.15's single order would hold it in the same place), a camera rotation
/// (`undo: none`), a no-job query and a layout step (`work.layout.set`, on the layout history, which EV4.15's single
/// order would not keep apart): only the edit becomes the move history's undo target and only the layout step the
/// layout history's; the rotation and the query invent none; the cursor's writes go to the recorder, never to the
/// journal, and take no action number. A later step that fails leaves the earlier completed actions applied.
#[test]
fn ct_ev4_15_each_played_step_keeps_its_undo_class() {
    let mut f = fixture();
    let revision = f.layer.revision().to_json();
    let r = recording(
        "u",
        vec![
            completed_write("w", &[1], 1),
            completed_rotate(),
            step(
                BLOCK,
                json::object([("mode", text("exact")), ("positions", letters(&[3])), ("revision", revision)]),
                Some(completed()),
            ),
            step(LAYOUT, json::object([("layout", text("wide"))]), Some(completed())),
            step("session.timer.start", json::object([]), Some(completed())),
        ],
    );
    f.put(vec![r]);
    assert!(f.layer.undo_hint(Direction::Undo).is_none() && f.layer.layout_hint(Direction::Undo).is_none());
    f.play("u", None);
    f.drive();
    let state = f.state("u");
    assert_eq!(
        state.status,
        PlayStatus::Stopped(Stop::Code(code::STALE_RECORDING_STEP.to_owned())),
        "the block step names a revision the edit moved"
    );
    assert_eq!(state.step, 2);
    let edit = state.actions.iter().find(|entry| entry.recorded == 1).map(|entry| entry.played).expect("mapped");
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(edit), "the move history");
    assert!(f.layer.layout_hint(Direction::Undo).is_none());
    assert_eq!(f.log(), 1, "one action: the edit; the cursor took none");

    // From the layout step: the layout history's target is the layout step's action, the move history's is still the
    // edit, and the timer row fails (nothing serves it) with the earlier actions kept.
    let again = f.play("u", Some(3));
    assert!(matches!(again.status, Status::Running(_)), "{:?}", again.status);
    f.drive();
    let state = f.state("u");
    assert_eq!(state.status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::Refused)));
    assert_eq!(f.log(), 2);
    assert_eq!(f.layer.layout_hint(Direction::Undo).map(|hint| hint.target), Some(2), "the layout history");
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(edit), "the move history");
    assert_eq!(f.layer.record("w").map(|v| v.payload.clone()), Some(letters(&[1])), "the earlier action stays");
}

/// CT-EV4-15 (whole; EV4.21; X1, owner decision of 23 November 2026, item 2.7: the move history, where EV4.15's
/// single order would read the same, since no layout step is involved). A playback of a no-job query
/// (`filter.preview`), an undoable workspace edit (`workspace.set`), a camera rotation and a timer row (both
/// `undo: none`; an entry serves the timer row here), then a rotation whose entry fails: the query completes
/// read-only and the rotation with a receipt, and neither takes an action; the timer row's completion is a durable
/// workspace action of undo class none, so it too invents no undo target; only the edit becomes the move history's
/// undo target; the later failure stops the playback and leaves the edit applied. The cursor is never an undo target:
/// its writes take no action, and the undo after the edit's reaches the action before the playback, leaving the
/// cursor where it was.
#[test]
fn ct_ev4_15_a_query_an_edit_and_a_rotation_keep_their_undo_classes() {
    let mut f = fixture();
    f.layer.serve_row("session.timer.start", overlay("timer")).unwrap();
    let predicate = predicate(&mut f);
    let query = step(FILTER_PREVIEW, filter_args(&f, &predicate), Some(query_done()));
    let timer = step("session.timer.start", json::object([]), Some(completed()));
    let steps = vec![query, completed_write("w", &[1], 1), completed_rotate(), timer, completed_rotate()];
    f.put(vec![recording("u", steps)]);
    let (log, before) = (f.log(), f.layer.undo_hint(Direction::Undo).map(|hint| hint.target));
    assert_eq!(before, Some(log as u64), "the predicate's action");
    let writes = f.recorder.writes();
    f.play("u", None);
    // Each child's job, settled by hand: the step it plays, its row, its receipt, then the log and the undo target.
    let mut settled = Vec::new();
    loop {
        f.layer.run_jobs();
        let Some(job) = f.layer.take_program_job() else { break };
        let at = f.state("u").step;
        let result = match at {
            4 => Err(Error::new(code::ROW_NOT_BUILT, "the entry failed")),
            _ => Ok(JobOutput::of(Value::Null)),
        };
        let receipt = f.layer.finish_program_job_with(job.job, result).expect("reported");
        let undo = f.layer.undo_hint(Direction::Undo).map(|hint| hint.target);
        settled.push((at, job.row, receipt.status.name(), f.log(), undo));
    }
    let edit = log as u64 + 1;
    let row = |row: &str| row.to_owned();
    let want = [
        (0, row(FILTER_PREVIEW), "completed", log, before),
        (2, row(ROTATE), "completed", log + 1, Some(edit)),
        (3, row("session.timer.start"), "completed", log + 2, Some(edit)),
        (4, row(ROTATE), "failed", log + 2, Some(edit)),
    ];
    assert_eq!(settled, want, "only the edit becomes an undo target; the timer row's action is undo class none");
    let state = f.state("u");
    assert_eq!((state.step, &state.status), (4, &PlayStatus::Stopped(Stop::Outcome(ChildOutcome::Failed))));
    let map: Vec<(u64, u64)> = state.actions.iter().map(|entry| (entry.recorded, entry.played)).collect();
    assert_eq!(map, [(1, edit)], "the map holds the edit's action");
    assert_eq!(f.log(), log + 2, "the edit's and the timer row's actions; the query, rotation and cursor took none");
    assert!(f.recorder.writes() > writes, "the cursor's writes went to the recorder");
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(edit), "the move history");
    assert!(f.layer.layout_hint(Direction::Undo).is_none(), "and nothing on the layout history");
    assert_eq!(f.layer.record("w").map(|v| v.payload.clone()), Some(letters(&[1])), "kept after the failure");

    let hint = f.layer.undo_hint(Direction::Undo).unwrap();
    assert!(matches!(f.press("session.undo", json::object([("hint", hint.to_json())])).status, Status::Committed(_)));
    assert!(f.layer.record("w").is_none(), "the edit is undone");
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), before, "next, the action before the play");
    assert_eq!(f.state("u"), state, "the cursor is no undo target and stays where it was");
}

// ----- CT-EV4-16: staleness and identity

/// CT-EV4-16 (part: a current revision plays, a stale one stops with its arguments as recorded, and a model or module
/// mismatch refuses before play or stops during it; not run: a kept filter preview gone stale, since only a
/// preview ID never kept is tried, which `ct_ev4_16_a_kept_filter_preview_gone_stale_stops_unretargeted` runs).
/// CT-ERR-18 (part: stale-step stops and identity refusals; not run: unknown outcomes, state and conversion refusals
/// and forwarded child failures, which other tests run).
/// An unattended step naming the current
/// revision plays; the same step after the revision moved stops with `EV4:stale.recording_step`, its arguments as
/// recorded. A filter apply naming a filter preview that is not fresh stops as stale too, with nothing retargeted. A
/// recording made under another model identity is refused before play, and a step whose module digest is not the
/// active version is refused before play or, when the version changes after the play started, stops during it, both
/// with `EV4:input.recording_identity_mismatch`; a module with no active version stops with
/// `EV4:input.recording_step_unresolved`.
#[test]
fn ct_ev4_16_stale_steps_stop_and_identities_refuse() {
    let mut f = fixture();
    let block_step = |revision: Value| {
        step(
            BLOCK,
            json::object([("mode", text("exact")), ("positions", letters(&[3])), ("revision", revision)]),
            Some(completed()),
        )
    };
    let current = f.layer.revision().to_json();
    f.put(vec![recording("now", vec![block_step(current.clone())])]);
    f.play("now", None);
    f.drive();
    assert_eq!(f.state("now").status, PlayStatus::Finished, "the current revision plays");
    let later = recording("later", vec![block_step(current.clone())]);
    f.put(vec![later]);
    f.play("later", None);
    f.drive();
    let stale = PlayStatus::Stopped(Stop::Code(code::STALE_RECORDING_STEP.to_owned()));
    assert_eq!(f.state("later").status, stale, "the block's own completion moved the revision");
    assert_eq!(
        f.layer.recording("later").unwrap().steps[0].arguments.get("revision"),
        Some(&current),
        "not retargeted"
    );

    // A filter preview that is not kept, or not fresh.
    let gone = json::object([(
        "preview",
        json::object([("epoch", Value::Int(f.layer.revision().epoch as i64)), ("job", Value::Int(999))]),
    )]);
    f.put(vec![recording("filter", vec![step(APPLY, gone.clone(), Some(completed()))])]);
    let played = f.play("filter", None);
    f.drive();
    assert_eq!(f.state("filter").status, stale);
    assert_eq!(code_of(&f.receipt(&played.command, &played.request)), Some(code::STALE_RECORDING_STEP));

    // Another model identity.
    let mut other = recording("other", vec![completed_rotate()]);
    other.identity = json::object([("model_id", text("another"))]);
    f.put(vec![other]);
    let refused = f.play("other", None);
    assert_eq!(code_of(&refused), Some(code::RECORDING_IDENTITY_MISMATCH));
    assert!(f.layer.playback("other").is_none(), "refused before play: no state is written");

    // A module digest (PS1.30's `module`): an unattended step of a row a user module's version `bbbb` served is refused
    // before play while another version is active, and stops during it when the version changes after the start; with
    // no version active, it is unresolved. A job row's user step is checked at the step action the same way.
    let versions = Versions::default();
    versions.set("tools/fixture", Some("aaaa"));
    f.layer.set_module_versions(Box::new(versions.clone()));
    let served = RecordedStep { module: Some("bbbb".into()), ..completed_rotate() };
    f.put(vec![recording("pinned", vec![completed_rotate(), served])]);
    assert_eq!(code_of(&f.play("pinned", None)), Some(code::RECORDING_IDENTITY_MISMATCH));
    assert!(f.layer.playback("pinned").is_none(), "refused before play: no state is written");
    versions.set("tools/fixture", Some("bbbb"));
    let started = f.play("pinned", None);
    assert!(matches!(started.status, Status::Running(_)), "{:?}", started.status);
    versions.set("tools/fixture", Some("cccc"));
    f.drive();
    let mismatch = PlayStatus::Stopped(Stop::Code(code::RECORDING_IDENTITY_MISMATCH.to_owned()));
    assert_eq!((f.state("pinned").step, f.state("pinned").status), (1, mismatch), "stopped during play");
    assert_eq!(code_of(&f.receipt(&started.command, &started.request)), Some(code::RECORDING_IDENTITY_MISMATCH));
    versions.set("tools/fixture", None);
    assert!(matches!(f.play("pinned", None).status, Status::Running(_)), "an unresolved module is no mismatch");
    f.drive();
    let unresolved = PlayStatus::Stopped(Stop::Code(code::RECORDING_STEP_UNRESOLVED.to_owned()));
    assert_eq!(f.state("pinned").status, unresolved);

    versions.set("m", Some("x1"));
    let tool = RecordedStep { module: Some("x0".into()), ..completed_tool() };
    f.put(vec![recording("tool", vec![tool])]);
    assert_eq!(code_of(&f.play("tool", None)), Some(code::RECORDING_IDENTITY_MISMATCH));
    versions.set("m", Some("x0"));
    f.play("tool", None);
    f.drive();
    assert_eq!(f.state("tool").status, PlayStatus::Waiting(0), "a job row waits for the step action");
    versions.set("m", Some("x1"));
    let step_refused = f.shell.step(&mut f.layer, "tool", false).unwrap_err();
    assert_eq!(step_refused.code, code::RECORDING_IDENTITY_MISMATCH);
    let mismatch = PlayStatus::Stopped(Stop::Code(code::RECORDING_IDENTITY_MISMATCH.to_owned()));
    assert_eq!(f.state("tool").status, mismatch);
    assert!(f.layer.take_program_job().is_none(), "no job ran");
}

/// A completed invocation of the language-defined query row of module `m`.
fn completed_tool() -> RecordedStep {
    step(TOOL, json::object([]), Some(json::object([("name", text("completed")), ("kind", text("query"))])))
}

/// CT-EV4-16 (part: by union with `ct_ev4_16_stale_steps_stop_and_identities_refuse`: here, a kept filter preview that
/// went stale; EV4.14, EV4.22). A `filter.apply` step names a filter preview the layer keeps: while it is fresh the
/// step plays and applies. Once a commit moves the state, the layer still keeps that filter preview but it is no
/// longer fresh, and the step stops with `EV4:stale.recording_step` at it, applying nothing, its arguments as
/// recorded. A fresh filter preview of the same predicate does not change that: the step still names the stale one,
/// and nothing retargets it.
#[test]
fn ct_ev4_16_a_kept_filter_preview_gone_stale_stops_unretargeted() {
    let mut f = fixture();
    let predicate = predicate(&mut f);
    let (job, kept) = kept_filter(&mut f, &predicate);
    let apply = json::object([("preview", kept)]);
    f.put(vec![recording("apply", vec![step(APPLY, apply.clone(), Some(completed()))])]);
    f.play("apply", None);
    f.drive();
    assert_eq!(f.state("apply").status, PlayStatus::Finished, "a fresh filter preview plays");
    assert!(f.layer.record("out/filter.apply").is_some(), "and applies");

    assert!(matches!(f.commit(&[1]).status, Status::Committed(_)), "the state moves");
    let preview = f.layer.filter_preview(job).expect("still kept").clone();
    assert!(!f.layer.filter_fresh(&preview), "and no longer fresh");
    let log = f.log();
    let stale = PlayStatus::Stopped(Stop::Code(code::STALE_RECORDING_STEP.to_owned()));
    let played = f.play("apply", Some(0));
    assert!(matches!(played.status, Status::Running(_)), "{:?}", played.status);
    f.drive();
    assert_eq!((f.state("apply").step, f.state("apply").status), (0, stale.clone()));
    assert_eq!(code_of(&f.receipt(&played.command, &played.request)), Some(code::STALE_RECORDING_STEP));
    assert_eq!(f.log(), log, "nothing applied");

    let (fresh, _) = kept_filter(&mut f, &predicate);
    assert_ne!(fresh, job);
    assert!(matches!(f.play("apply", Some(0)).status, Status::Running(_)));
    f.drive();
    assert_eq!(f.state("apply").status, stale, "the step names the stale filter preview, not the fresh one");
    assert_eq!(f.log(), log);
    assert_eq!(f.layer.recording("apply").unwrap().steps[0].arguments, apply, "not retargeted");
}

// ----- CT-EV4-14 and CT-PS1-25: the crash points, synthetic

/// CT-EV4-14 and CT-PS1-25 (synthetic, whole; EV4.20, PS1.44: the recorder's injected fates stand in for the crash,
/// and a
/// reopen of the journal and the recorder for the restart; `c600-durable` runs them on M6's journal and the session
/// store's recorder on a memory disk). Three crash points:
///
/// 1. after `pending` is durable but before dispatch: the write landed and its answer was lost, so the layer does not
///    dispatch; the reopen's lookup finds no receipt for the command, the playback stops on `unknown_outcome`, and
///    nothing is ever dispatched under that command;
/// 2. after a durable child completion but before the cursor advance: the reopen's lookup finds the committed record
///    write, advances once and maps its action, and a second reopen advances nothing more;
/// 3. during a manual continuation: the step action's `pending` is durable and the shell never sent the row; the
///    reopen stops on `unknown_outcome`, and only an explicit `from` opens a new playback (`EV4:input.recording_state`
///    otherwise).
///
/// An unrelated invocation never moves the cursor, and the cursor is never an undo target (X1: the move history's
/// target after the playback is the newest record write, as EV4.15's single order would have it too).
#[test]
fn ct_ev4_14_crash_points_recover_by_lookup_first() {
    // 1. Pending landed, dispatch never happened.
    let mut f = fixture();
    f.put(vec![recording("c", vec![completed_write("w", &[1], 1), completed_write("v", &[2], 2)])]);
    f.play("c", None);
    f.recorder.set_unknown_writes(true);
    f.recorder.inject(Fate::Unknown);
    f.layer.run_jobs();
    let held = f.recorder.held().playbacks["c"].clone();
    let pending = held.pending.clone().expect("the pending entry landed");
    assert_eq!((held.step, f.log()), (0, 0), "nothing was dispatched");
    let g = reopen(&f, "after");
    let state = g.state("c");
    assert_eq!(state.status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)));
    assert_eq!((state.step, state.pending.is_none(), g.log()), (0, true, 0), "no retry");
    assert_eq!(state.last.as_ref().and_then(|last| last.get("command")), Some(&text(&pending.command)));
    assert_eq!(g.receipt(&pending.command, &pending.request).status, Status::UnknownOutcome);
    assert_eq!(g.recorder.held().playbacks["c"], state, "recovered in one write");

    // 2. The child's record write is durable; the cursor's advance is lost, and so is the next step's pending entry.
    let mut f = fixture();
    f.put(vec![recording("c", vec![completed_write("w", &[1], 1), completed_write("v", &[2], 2)])]);
    f.play("c", None);
    f.recorder.inject(Fate::Durable); // the first step's pending entry
    f.recorder.inject(Fate::Unknown); // the advance, which does not land
    f.recorder.inject(Fate::Unknown); // the second step's pending entry, which does not land either
    f.layer.run_jobs();
    assert_eq!(f.log(), 1, "the first child's action is durable, and the second was never dispatched");
    let held = f.recorder.held().playbacks["c"].clone();
    assert_eq!((held.step, held.pending.is_some()), (0, true), "the advance did not land");
    let mut g = reopen(&f, "after");
    let state = g.state("c");
    assert_eq!((state.step, state.pending.is_none()), (1, true), "advanced once by the lookup");
    let map: Vec<(u64, u64)> = state.actions.iter().map(|e| (e.recorded, e.played)).collect();
    assert_eq!(map, [(1, 1)]);
    assert_eq!(state.status, PlayStatus::Stopped(Stop::Code(code::ENGINE_LOST.to_owned())), "no live job");
    let h = reopen(&g, "again");
    assert_eq!(h.state("c"), state, "a second reopen advances nothing");
    assert_eq!(g.layer.record("w").map(|v| v.payload.clone()), Some(letters(&[1])));
    // An unrelated invocation does not move the cursor; a play continues from it.
    assert!(matches!(g.write("x", &[5]).status, Status::Completed(_)));
    assert_eq!(g.state("c").step, 1);
    g.play("c", None);
    g.drive();
    assert_eq!((g.state("c").status, g.log()), (PlayStatus::Finished, 3));
    assert_eq!(g.layer.undo_hint(Direction::Undo).map(|h| h.target), Some(3), "the newest action, never the cursor");

    // 3. A manual continuation: pending durable, the row never sent.
    let mut f = fixture();
    f.put(vec![recording("m", vec![word_step(&[1], 1, None), completed_write("w", &[1], 2)])]);
    f.play("m", None);
    f.drive();
    assert_eq!(f.state("m").status, PlayStatus::Waiting(0));
    let fresh = f.shell.step_preview(&mut f.layer, "m").unwrap();
    let Status::Running(id) = fresh.status else { panic!("{:?}", fresh.status) };
    f.layer.run_jobs();
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    let (row, args) = f.layer.step_arguments("m").unwrap();
    let mut args = args.as_object().cloned().unwrap();
    args.insert("preview".into(), Value::Int(id as i64));
    let invocation = f.shell.prepare(&f.layer, &row, Value::Object(args)).unwrap();
    f.layer.begin_step("m", &invocation).unwrap();
    assert!(f.recorder.held().playbacks["m"].pending.is_some(), "pending is durable before the send");
    let writes = f.recorder.writes();
    assert!(matches!(f.write("x", &[5]).status, Status::Completed(_)), "an unrelated invocation");
    assert_eq!((f.recorder.writes(), f.state("m").step), (writes, 0), "does not move the cursor or settle the step");
    let mut g = reopen(&f, "after");
    let state = g.state("m");
    assert_eq!(state.status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::UnknownOutcome)));
    assert_eq!(g.layer.revision().state, 0, "nothing committed");
    assert_eq!(code_of(&g.play("m", None)), Some(code::RECORDING_STATE), "unknown_outcome needs an explicit `from`");
    let rewound = g.play("m", Some(0));
    assert!(matches!(rewound.status, Status::Running(_)), "{:?}", rewound.status);
    g.drive();
    let rewound = g.state("m");
    assert_ne!(rewound.playback, state.playback, "a new playback");
    assert_eq!((rewound.step, &rewound.status), (0, &PlayStatus::Waiting(0)));
    assert_eq!(g.recorder.held().playbacks["m"], rewound, "the explicit `from` is saved");
}

// ----- CT-EV4-17 and CT-EV4-18: conversion

/// The letters of the draft record a conversion of `name` wrote.
fn draft_letters(f: &F, name: &str) -> Option<Vec<i64>> {
    f.layer.record(&format!("draft/recording/{name}")).and_then(RecordValue::operation)
}

/// CT-EV4-17 (whole; EV4.23) on both targets, the five vectors. Every refusal writes nothing: the journal, the records
/// and the revision stay as they were. CT-ERR-18 (part: a conversion refusal keeps `recording_not_convertible`; not
/// run: the stale, identity, outcome and forwarding clauses, which other tests run). (1) A camera rotation alone
/// refuses the empty draft and becomes one permitted source
/// call. (2) `macro.add(W)` then `operation.execute(W)` gives W once. (3) W1, W2, undo W2, W3 gives W1 W3. (4) An undo
/// of an action outside the recording refuses. (5) `jumble.on` between word steps refuses. Undo here is the move
/// history's `session.undo` (X1), which EV4.23's table names, so EV4.15's single order reads the same.
#[test]
fn ct_ev4_17_the_five_conversion_vectors_on_both_targets() {
    let mut f = fixture();
    let staged = RecordedStep {
        action: Some(1),
        ..step(
            "macro.add",
            json::object([("macro", json::object([("recipe", word(&[7]))])), ("phase", text("macro"))]),
            Some(completed()),
        )
    };
    let jumble_on = RecordedStep { action: Some(3), ..step("jumble.on", json::object([]), Some(committed())) };
    f.put(vec![
        recording("camera", vec![completed_rotate()]),
        recording("staged", vec![staged, word_step(&[7], 2, None)]),
        recording(
            "undone",
            vec![
                word_step(&[1], 1, None),
                word_step(&[2], 2, None),
                undo_step("session.undo", 2, 3),
                word_step(&[3], 4, None),
            ],
        ),
        recording("older", vec![word_step(&[1], 5, None), undo_step("session.undo", 4, 6)]),
        recording("jumble", vec![word_step(&[1], 1, None), jumble_on, word_step(&[2], 4, None)]),
    ]);
    let unchanged = |f: &F| (f.log(), f.layer.revision().clone(), f.layer.records().len());

    let before = unchanged(&f);
    let refused = f.convert("camera", "draft");
    assert_eq!(code_of(&refused), Some(code::RECORDING_NOT_CONVERTIBLE), "an empty draft");
    assert_eq!(unchanged(&f), before);
    assert!(matches!(f.convert("camera", "source").status, Status::Completed(_)));
    let (_, conversion) = f.layer.conversion().unwrap();
    assert_eq!((conversion.letters.len(), conversion.calls.len()), (0, 1), "one permitted call");
    assert_eq!(conversion.calls[0].get("row"), Some(&text(ROTATE)));

    for target in ["draft", "source"] {
        assert!(matches!(f.convert("staged", target).status, Status::Completed(_)), "{target}");
        assert_eq!(f.layer.conversion().unwrap().1.letters, [7], "{target}: W once");
        assert!(matches!(f.convert("undone", target).status, Status::Completed(_)), "{target}");
        assert_eq!(f.layer.conversion().unwrap().1.letters, [1, 3], "{target}: W1 W3");
        for name in ["older", "jumble"] {
            let before = unchanged(&f);
            let refused = f.convert(name, target);
            assert_eq!(code_of(&refused), Some(code::RECORDING_NOT_CONVERTIBLE), "{name} {target}");
            let Status::Refused(error) = &refused.status else { panic!("{:?}", refused.status) };
            assert!(
                error.detail.contains("step 1") && error.detail.contains(target),
                "{name} {target}: {}",
                error.detail
            );
            assert_eq!(unchanged(&f), before, "{name} {target}: nothing written");
        }
    }
    assert_eq!(draft_letters(&f, "undone"), Some(vec![1, 3]));
    assert_eq!(draft_letters(&f, "staged"), Some(vec![7]));
    let source = f.layer.record("source/recording/undone").expect("the source record").payload.clone();
    let module = source.get("source").and_then(|s| s.get("text")).and_then(Value::as_str).unwrap().to_owned();
    assert!(module.contains("propose(word[1, 3], target(\"operation.preview\"))"), "{module}");
}

/// CT-EV4-18 (whole; EV4.23). A redo restores a word at its recorded place; an undo or a redo of a recorded non-word
/// action and a checkpoint save add no draft letters but refuse source, each at its own step; completed non-word steps
/// enter the draft report; source takes only calls an overlay row may invoke (KN5.4) with no `confirm` or `file_check`
/// gate and refuses a file or app row and a row no overlay row may invoke. A successful draft conversion is one
/// undoable action that writes a Macro-phase draft and keeps every contributing word's block in its block's parts,
/// each once, two blocks included. X1: the conversion's record write is the move history's undo target, as
/// `workspace.set`'s is; EV4.15's single order would read the same.
#[test]
fn ct_ev4_18_conversion_details() {
    let mut f = fixture();
    let b1 = block("manual", "none", &[]);
    let checkpoint = RecordedStep {
        action: Some(4),
        ..step("session.checkpoint.save", json::object([("name", text("c"))]), Some(committed()))
    };
    let export = step("file.keymap.export", json::object([]), Some(completed()));
    let gated = step("macro.save", json::object([("name", text("m"))]), Some(completed()));
    f.put(vec![
        recording(
            "redo",
            vec![
                word_step(&[1], 1, Some(&b1)),
                word_step(&[2], 2, None),
                undo_step("session.undo", 1, 3),
                undo_step("session.redo", 1, 4),
            ],
        ),
        recording(
            "nonword",
            vec![
                completed_write("w", &[1], 1),
                word_step(&[2], 2, None),
                undo_step("session.undo", 1, 3),
                checkpoint,
                completed_rotate(),
            ],
        ),
        recording("file", vec![word_step(&[1], 1, None), export]),
        recording("gated", vec![word_step(&[1], 1, None), gated]),
    ]);

    // One undoable action, on the move history, keeping the contributing block.
    let before = f.log();
    assert!(f.layer.record("draft/recording/redo").is_none());
    let converted = f.convert("redo", "draft");
    assert!(matches!(converted.status, Status::Completed(_)), "{:?}", converted.status);
    assert_eq!(f.log(), before + 1, "one action");
    assert_eq!(f.layer.conversion().unwrap().1.letters, [1, 2], "the redo restores word 1 at its recorded place");
    let value = f.layer.record("draft/recording/redo").unwrap().clone();
    let sheet = c600_command::Worksheet::from_payload(&value.payload).expect("an operation draft");
    assert_eq!(sheet.phases, [vec![], vec![1, 2], vec![]], "the Macro phase holds the literal word");
    assert_eq!(Reader.parts(value.provenance.as_bytes()), [b1.clone()], "the part is kept");
    let undo = f.layer.undo_hint(Direction::Undo).unwrap();
    assert_eq!(undo.target, f.log() as u64, "the move history's target is the conversion");
    let undone = f.press("session.undo", json::object([("hint", undo.to_json())]));
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
    assert!(f.layer.record("draft/recording/redo").is_none(), "the undo removes the draft");

    assert!(matches!(f.convert("nonword", "draft").status, Status::Completed(_)));
    let (_, conversion) = f.layer.conversion().unwrap();
    assert_eq!(conversion.letters, [2]);
    let report: Vec<(i64, String)> = conversion
        .report
        .iter()
        .map(|e| {
            (
                e.get("step").and_then(Value::as_i64).unwrap(),
                e.get("contributes").and_then(Value::as_str).unwrap().to_owned(),
            )
        })
        .collect();
    let want = [(0, "nothing"), (1, "letters"), (2, "nothing"), (3, "nothing"), (4, "nothing")];
    assert_eq!(report, want.map(|(s, c)| (s, c.to_owned())), "every completed non-word step is in the report");
    for name in ["nonword", "file", "gated"] {
        let before = f.log();
        assert_eq!(code_of(&f.convert(name, "source")), Some(code::RECORDING_NOT_CONVERTIBLE), "{name}");
        assert_eq!(f.log(), before, "{name}: nothing written");
    }
    assert!(matches!(f.convert("file", "draft").status, Status::Completed(_)), "a file row is in the draft's report");
    assert!(matches!(f.convert("gated", "draft").status, Status::Completed(_)), "so is a `confirm` row");

    // Each source refusal at its own step. The non-word action here is `macro.add`, a workspace row an overlay row may
    // invoke with no gate, so it is a source call itself and the refusal is the undo's, the redo's or the save's.
    let staged = RecordedStep {
        action: Some(1),
        ..step(
            "macro.add",
            json::object([("macro", json::object([("recipe", word(&[5]))])), ("phase", text("macro"))]),
            Some(completed()),
        )
    };
    let checkpoint = RecordedStep {
        action: Some(2),
        ..step("session.checkpoint.save", json::object([("name", text("c"))]), Some(committed()))
    };
    f.put(vec![
        recording("staged", vec![staged.clone(), word_step(&[2], 2, None)]),
        recording(
            "redone",
            vec![
                staged.clone(),
                word_step(&[2], 2, None),
                undo_step("session.undo", 1, 3),
                undo_step("session.redo", 1, 4),
            ],
        ),
        recording("redo", vec![staged, word_step(&[2], 2, None), undo_step("session.redo", 1, 3)]),
        recording("saved", vec![word_step(&[1], 1, None), checkpoint]),
        recording("written", vec![word_step(&[1], 1, None), completed_write("w", &[1], 2)]),
    ]);
    assert!(matches!(f.convert("staged", "source").status, Status::Completed(_)));
    let calls = &f.layer.conversion().unwrap().1.calls;
    assert_eq!(calls.iter().map(|c| c.get("row").cloned()).collect::<Vec<_>>(), [Some(text("macro.add"))]);
    let staged_report = ["nothing", "letters", "nothing", "nothing"];
    for (name, at, row, want, reported) in [
        ("redone", 2, "session.undo", vec![2], &staged_report[..]),
        ("redo", 2, "session.redo", vec![2], &staged_report[..3]),
        ("saved", 1, "session.checkpoint.save", vec![1], &["letters", "nothing"][..]),
        ("written", 1, RECORD_ROW, vec![1], &["letters", "nothing"][..]),
    ] {
        let before = f.log();
        let refused = f.convert(name, "source");
        let Status::Refused(error) = &refused.status else { panic!("{name}: {:?}", refused.status) };
        assert_eq!(error.code, code::RECORDING_NOT_CONVERTIBLE, "{name}");
        assert!(error.detail.starts_with(&format!("step {at} (`{row}`)")), "{name}: {}", error.detail);
        assert_eq!(f.log(), before, "{name}: nothing written");
        assert!(matches!(f.convert(name, "draft").status, Status::Completed(_)), "{name}");
        let (_, conversion) = f.layer.conversion().unwrap();
        assert_eq!(conversion.letters, want, "{name}: no letters from step {at}");
        let report: Vec<&str> = conversion.report.iter().filter_map(|e| e.get("contributes")?.as_str()).collect();
        assert_eq!(report, reported, "{name}: every step is in the draft's report, an undo's and a redo's too");
    }

    // A draft that keeps two blocks: each contributing word's block once, in recorded order, on both targets.
    let b2 = block("import", "none", &[]);
    f.put(vec![recording(
        "blocks",
        vec![word_step(&[1], 1, Some(&b1)), word_step(&[2], 2, Some(&b2)), word_step(&[3], 3, Some(&b1))],
    )]);
    let before = f.log();
    assert!(matches!(f.convert("blocks", "draft").status, Status::Completed(_)));
    assert_eq!(f.log(), before + 1, "one action");
    let value = f.layer.record("draft/recording/blocks").unwrap().clone();
    let sheet = c600_command::Worksheet::from_payload(&value.payload).expect("an operation draft");
    assert_eq!(sheet.phases, [vec![], vec![1, 2, 3], vec![]], "the Macro phase holds the literal word");
    assert_eq!(Reader.parts(value.provenance.as_bytes()), [b1.clone(), b2.clone()], "both blocks are kept");
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), Some(f.log() as u64), "undoable");
    assert!(matches!(f.convert("blocks", "source").status, Status::Completed(_)));
    let source = f.layer.record("source/recording/blocks").unwrap().provenance.clone();
    assert_eq!(Reader.parts(source.as_bytes()), [b1, b2], "and the source's");
}

// ----- CT-EV4-19 and CT-EV4-20: the play table, rewinds and the map

/// CT-EV4-19 (part: a play or `from` outside the table refuses, a rewind opens an empty map, W, rewind, W, undo undoes
/// the second W only, a play ends at the next user step, `job.stop` halts it, and each child has its own receipt; not
/// run: a start while recording and a stop with none, and the table's rows on the vectors, which other tests run).
/// CT-ERR-18 (part: state refusals keep their own code; not run: the clauses other tests run).
/// EV4.22: a play or `from` outside EV4.22's table
/// refuses with `EV4:input.recording_state` (a start while recording and a stop with none are in
/// `ct_ev4_11_capture_appends_one_step_per_resolved_invocation`); each row of the table gives its outcome after
/// reconciliation (`the_format_packets_playback_vectors` runs the table on the vectors, and the rows run live here): a
/// rewind opens a new playback with an empty map, and W, rewind, W, undo undoes the second W only (the move history,
/// X1; EV4.15's single order would read the same); a play ends at the next user step or the end; `job.stop` halts its
/// job with `cancelled(by_user)` and keeps the pending child for the next reconciliation; every child has a command ID
/// and a receipt of its own, and the captured step is never replaced.
#[test]
fn ct_ev4_19_the_play_table_rewinds_and_job_stop() {
    let mut f = fixture();
    f.put(vec![recording("w", vec![completed_write("w", &[1], 10), undo_step("session.undo", 10, 11)])]);
    assert_eq!(code_of(&f.play("w", Some(2))), Some(code::RECORDING_STATE), "from = steps");
    assert_eq!(code_of(&f.play("w", Some(-1))), Some(code::RECORDING_STATE), "a negative from");
    assert_eq!(code_of(&f.play("nope", None)), Some(code::NAMED_OBJECT_INVALID));
    f.play("w", None);
    assert_eq!(code_of(&f.play("w", None)), Some(code::RECORDING_STATE), "status running");
    f.drive();
    let first = f.state("w");
    assert_eq!(first.status, PlayStatus::Waiting(1), "a play ends at the next user step");
    let w1 = first.actions[0].played;
    // Rewind: a new playback with an empty map; W plays again as a new action.
    f.play("w", Some(0));
    f.drive();
    let second = f.state("w");
    assert_ne!(second.playback, first.playback);
    let w2 = second.actions[0].played;
    assert_eq!((second.actions.len(), w2 > w1), (1, true), "the map holds only this playback's W");
    let undone = f.shell.step(&mut f.layer, "w", false).unwrap();
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
    assert_eq!(f.state("w").status, PlayStatus::Finished, "the play ended at the end");
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|h| h.target), Some(w1), "the first W stays applied");
    assert_eq!(code_of(&f.play("w", None)), Some(code::RECORDING_STATE), "finished needs `from`");

    // Every child is a new invocation with its own command ID and receipt; the captured step keeps its own.
    let recorded = f.layer.recording("w").unwrap().steps[0].clone();
    let last = second.last.as_ref().expect("the child's receipt");
    let child = last.get("command").and_then(Value::as_str).unwrap().to_owned();
    assert_ne!(child, recorded.command);
    assert!(child.starts_with(&second.playback), "{child}");
    let request = last.get("request").and_then(Value::as_str).unwrap();
    assert_eq!(&f.receipt(&child, request).to_json(), last, "the kernel keeps the child's receipt");
    assert_eq!(f.layer.recording("w").unwrap().steps[0], recorded, "never replaced");

    // `job.stop` halts a live playback; its pending child is reconciled once, when it reports.
    f.put(vec![recording("r", vec![completed_rotate(), completed_rotate()])]);
    let played = f.play("r", None);
    let Status::Running(job) = played.status else { panic!("{:?}", played.status) };
    f.layer.run_jobs();
    let taken = f.layer.take_program_job().expect("the first step's job");
    let stopped = f.press("job.stop", json::object([("job", Value::Int(job as i64))]));
    assert!(matches!(stopped.status, Status::Completed(_)), "{:?}", stopped.status);
    assert_eq!(f.receipt(&played.command, &played.request).status, Status::Cancelled(CancelReason::ByUser));
    let state = f.state("r");
    assert_eq!(state.status, PlayStatus::Stopped(Stop::Code(code::BY_USER.to_owned())));
    assert!(state.pending.is_some(), "the pending child is kept");
    let again = f.press("job.stop", json::object([("job", Value::Int(job as i64))]));
    assert_eq!(code_of(&again), Some(code::ROW_NOT_BUILT), "the job is no longer live");
    f.layer.finish_program_job(taken.job, Ok(Value::Null)).unwrap();
    assert_eq!((f.state("r").step, f.state("r").pending.is_none()), (1, true), "the child's completion, once");
    f.play("r", None);
    f.drive();
    assert_eq!(f.state("r").status, PlayStatus::Finished);
}

/// CT-EV4-20 (whole; EV4.18, PS1.44; X1: `session.undo` and `session.redo` on the move history, which EV4.15's
/// single order
/// would read the same, since no layout step is involved). W recorded as action 1 plays as a new action; a
/// `workspace.set` of X outside the playback (the vector's `macro.add(X)`, which this layer does not build) takes the
/// next action and leaves the cursor; the undo user step's hint then targets X, not the action W played as, so the
/// playback stops with `EV4:stale.recording_step`, X is not undone and W stays applied. Once the solver undoes X
/// themselves, the hint targets W and the step undoes it; a redo step's target is the original action number, so the
/// same map serves it. A rewind's empty map has no entry for the recorded target, and the step stops the same way.
#[test]
fn ct_ev4_20_the_action_map_guards_undo_and_redo() {
    let mut f = fixture();
    let steps = vec![completed_write("w", &[1], 1), undo_step("session.undo", 1, 2), undo_step("session.redo", 1, 3)];
    f.put(vec![recording("w", steps)]);
    f.play("w", None);
    f.drive();
    let played = f.state("w").actions[0].played;
    assert!(matches!(f.write("x", &[9]).status, Status::Completed(_)));
    let x = f.layer.undo_hint(Direction::Undo).unwrap().target;
    assert_eq!((x, f.state("w").step), (played + 1, 1), "X leaves the cursor");
    let refused = f.shell.step(&mut f.layer, "w", false).unwrap_err();
    assert_eq!(refused.code, code::STALE_RECORDING_STEP);
    assert_eq!(f.state("w").status, PlayStatus::Stopped(Stop::Code(code::STALE_RECORDING_STEP.to_owned())));
    assert_eq!((f.layer.record("x").is_some(), f.layer.record("w").is_some()), (true, true), "nothing undone");

    let hint = f.layer.undo_hint(Direction::Undo).unwrap();
    let solver = f.press("session.undo", json::object([("hint", hint.to_json())]));
    assert!(matches!(solver.status, Status::Committed(_)), "{:?}", solver.status);
    f.play("w", None);
    f.drive();
    assert_eq!(f.state("w").status, PlayStatus::Waiting(1));
    assert!(matches!(f.shell.step(&mut f.layer, "w", false).unwrap().status, Status::Committed(_)));
    assert!(f.layer.record("w").is_none(), "W undone");
    assert_eq!(f.state("w").status, PlayStatus::Waiting(2));
    assert_eq!(
        f.layer.undo_hint(Direction::Redo).map(|h| h.target),
        Some(played),
        "a redo targets the original action"
    );
    assert!(matches!(f.shell.step(&mut f.layer, "w", false).unwrap().status, Status::Committed(_)));
    assert!(f.layer.record("w").is_some(), "W redone");
    assert_eq!(f.state("w").status, PlayStatus::Finished);

    // A rewind to the undo step: the new playback's map has no entry for the recorded target.
    f.play("w", Some(1));
    f.drive();
    assert_eq!(f.state("w").actions, []);
    assert_eq!(f.shell.step(&mut f.layer, "w", false).unwrap_err().code, code::STALE_RECORDING_STEP);
    assert!(f.layer.record("w").is_some(), "nothing undone");
}

// ----- CT-EV4-09, CT-EV4-10, CT-BND-03, CT-PS1-17: tokens, commits and provenance

/// The I_a menu's set and order digests (`tests/twist_apply.rs`).
const MENU_SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const MENU_ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";

fn pinned_menu() -> (Arc<Context>, Menu) {
    let context = Arc::new(Context::load(&repository().join("assets")).expect("the exact model loads"));
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let record = json::parse(&fixture).expect("JSON").get("menu").expect("a menu").clone();
    let menu = Menu::from_record(&context, &record).expect("the pinned I_a menu");
    (context, menu)
}

/// `twist.apply`'s arguments for retained menu item `index` at cap 0, pressed directly.
fn twist_args(index: u64) -> Value {
    let binding = Binding {
        request: wire::menu_request(MENU_SET, MENU_ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: MENU_SET.to_owned(), order: MENU_ORDER.to_owned() }),
    };
    let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
    let cap = fields.remove("cap").expect("a cap");
    json::object([("cap", cap), ("rotation", Value::Object(fields))])
}

/// CT-EV4-09 (whole; EV4.12, EV4.18, ONE.78).
/// CT-PS1-15 (part: commit playback needs a new explicit action and token; not run: the round trip of a step with
/// a module digest or a word block, which `ct_ps1_15_a_module_digest_and_a_word_block_round_trip` runs).
/// No recording, playback state or
/// conversion holds a token or a preview ID, and none is replayed: a recorded preview and a recorded word commit
/// wait for the solver's step actions, the commit's with a new whole-word preview of the stored word, never the
/// recorded preview ID; a recorded checkpoint save waits too, and its step action mints a new token and fills the
/// current revision (the layer does not build `session.checkpoint.save`, so its send is refused, which stops the
/// playback). A recorded live twist (ONE.78) takes no preview: its step action commits it as a press does. A
/// recording of a commit converts to a draft and to source whose records and calls hold no token, binding, nonce,
/// confirmation or preview ID.
#[test]
fn ct_ev4_09_no_token_is_stored_or_replayed() {
    let mut f = fixture();
    f.start("t");
    assert!(matches!(f.commit(&[1]).status, Status::Committed(_)));
    let mut t = f.stop("t");
    let rows: Vec<&str> = t.steps.iter().map(|step| step.row.as_str()).collect();
    assert_eq!(rows, ["operation.preview", "operation.execute"]);
    let old = t.steps[0].outcome.as_ref().and_then(|o| o.get("job")).and_then(Value::as_i64);
    let old = old.expect("the recorded preview's job") as u64;
    let checkpoint = json::object([("name", text("c")), ("revision", f.layer.revision().to_json())]);
    t.name = "t2".into();
    t.steps.push(step("session.checkpoint.save", checkpoint, Some(committed())));
    f.put(vec![t.clone()]);
    for step in &t.steps {
        let fields = step.arguments.as_object().unwrap();
        assert!(!fields.contains_key("token") && !fields.contains_key("preview"), "{}", step.row);
    }
    let before = f.layer.revision().state;
    f.play("t2", None);
    f.drive();
    assert_eq!(f.state("t2").status, PlayStatus::Waiting(0), "the preview is a user step (exclusive lane, job)");
    assert_eq!(f.layer.revision().state, before, "nothing committed unattended");
    let bytes = f.layer.playback("t2").unwrap().to_json().canonical();
    assert!(!bytes.contains("\"token\"") && !bytes.contains("\"nonce\""), "{bytes}");

    let preview = f.shell.step(&mut f.layer, "t2", false).unwrap();
    assert!(matches!(preview.status, Status::Running(_)), "{:?}", preview.status);
    f.layer.run_jobs();
    assert_eq!(f.state("t2").status, PlayStatus::Waiting(1), "a word commit waits");
    let fresh = f.shell.step_preview(&mut f.layer, "t2").unwrap();
    let Status::Running(id) = fresh.status else { panic!("{:?}", fresh.status) };
    assert_ne!(id, old, "a new preview, never the recorded one");
    f.layer.run_jobs();
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    assert!(matches!(f.shell.step(&mut f.layer, "t2", false).unwrap().status, Status::Committed(_)));
    assert_eq!((f.layer.revision().state, f.state("t2").status), (before + 1, PlayStatus::Waiting(2)));

    let (row, args) = f.layer.step_arguments("t2").unwrap();
    let mut args = args.as_object().cloned().unwrap();
    args.insert("revision".into(), f.layer.revision().to_json());
    let press = f.shell.prepare(&f.layer, &row, Value::Object(args)).unwrap();
    assert!(press.token.is_some(), "the solver's action mints a new token");
    f.layer.begin_step("t2", &press).unwrap();
    let saved = f.shell.send(&mut f.layer, press);
    assert_eq!(code_of(&saved), Some(code::ROW_NOT_BUILT));
    assert_eq!(f.state("t2").status, PlayStatus::Stopped(Stop::Outcome(ChildOutcome::Refused)));

    // A live twist: recorded with its retained word, played with no preview.
    let mut f = fixture();
    let (context, menu) = pinned_menu();
    f.layer.pin_menu(context, menu).unwrap();
    f.start("tw");
    assert!(matches!(f.press(TWIST_ROW, twist_args(1)).status, Status::Committed(_)));
    let tw = f.stop("tw");
    assert!(tw.steps[0].letters().is_some_and(|letters| !letters.is_empty()), "the twist's word is kept");
    assert!(tw.steps[0].arguments.get("token").is_none());
    f.play("tw", None);
    f.drive();
    assert_eq!(f.state("tw").status, PlayStatus::Waiting(0));
    assert_eq!(f.layer.step_draft("tw").unwrap_err().code, code::RECORDING_STATE, "no word preview for a twist");
    let state = f.layer.revision().state;
    assert!(matches!(f.shell.step(&mut f.layer, "tw", false).unwrap().status, Status::Committed(_)));
    assert_eq!((f.layer.revision().state, f.state("tw").status), (state + 1, PlayStatus::Finished));
    assert_eq!(f.layer.run_jobs(), 0, "no preview job");

    // A conversion's output: a recorded commit, made with a token, and a view step convert to a draft and to source
    // whose records (payload and block) and calls hold no token, its binding or nonce, confirmation or preview ID.
    let mut f = fixture();
    f.start("c");
    let id = f.preview(json::object([("recipe", word(&[1, 2]))]));
    let press =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))])).unwrap();
    let binding = press.token.as_ref().expect("a commit's token").binding.to_json().canonical();
    assert!(matches!(f.shell.send(&mut f.layer, press).status, Status::Committed(_)));
    assert!(matches!(f.rotate().status, Status::Completed(_)));
    f.stop("c");
    for target in ["draft", "source"] {
        assert!(matches!(f.convert("c", target).status, Status::Completed(_)), "{target}");
        let value = f.layer.record(&format!("{target}/recording/c")).unwrap().clone();
        let bytes = format!("{}{}", value.payload.canonical(), value.provenance);
        for held in [binding.as_str(), "\"nonce\"", "\"token\"", "\"confirm\""] {
            assert!(!bytes.contains(held), "{target} holds {held}: {bytes}");
        }
        let (_, conversion) = f.layer.conversion().unwrap();
        assert_eq!(conversion.letters, [1, 2], "{target}: the word, never its preview");
        for call in &conversion.calls {
            let fields = call.get("arguments").and_then(Value::as_object).unwrap();
            assert!(["token", "preview", "confirm"].iter().all(|k| !fields.contains_key(*k)), "{target}: {fields:?}");
        }
    }
    let source = f.layer.record("source/recording/c").unwrap().payload.clone();
    let calls = source.get("source").and_then(|s| s.get("calls")).and_then(Value::as_array).unwrap();
    let rows: Vec<&str> = calls.iter().filter_map(|call| call.get("row")?.as_str()).collect();
    assert_eq!(rows, ["operation.preview", ROTATE], "the stored calls name their rows, with no execution permission");
}

/// CT-EV4-10 (whole; EV4.13, EV4.16, EV4.23, on this file's test reader).
/// CT-BND-03 (part: a word carried through a workspace record, a recording and a conversion to source keeps its block,
/// BND.32; the file route is in `c600-run/tests/boundary.rs` and the macro route in `tests/rebind_and_macro_base.rs`,
/// on this crate's test readers).
/// CT-PS1-17 (part: a word survives a recording and a source conversion with its block, on this file's test reader;
/// not run: workspace save and read and the labels of a state-derived word, which `c600-durable`'s
/// `tests/ct_bnd_03_recording_blocks.rs` runs on C5's reader; rename, since no H-08 row renames a workspace record; and
/// the file round trip, since the layer builds no file export or import of a word).
/// A word written with a block, previewed from its record and committed is recorded
/// with that block, and both conversions keep it as a part. A research run's word that a recording carries is not
/// laundered: the draft conversion's preview reads its part and is refused, and so is the fresh preview of its word
/// step. A source conversion's own block is a program's, which this session's reader refuses at preview (BND.84). A
/// recorded confirmation never revives (`ct_ev4_12_playback_plays_only_the_unattended_rows`, and the policy step here).
#[test]
fn ct_ev4_10_provenance_survives_and_nothing_launders() {
    let mut f = fixture();
    let own = block("manual", "none", &[]);
    assert!(matches!(f.write_with("word", &[1, 2], &own).status, Status::Completed(_)));
    f.start("p");
    let id = f.preview(json::object([("record", text("word"))]));
    assert!(matches!(f.execute(id).status, Status::Committed(_)));
    let p = f.stop("p");
    assert_eq!(p.steps[1].block(), Some(own.as_str()), "the recording keeps the record's block");
    for target in ["draft", "source"] {
        assert!(matches!(f.convert("p", target).status, Status::Completed(_)), "{target}");
        let value = f.layer.record(&format!("{target}/recording/p")).unwrap().clone();
        assert_eq!(Reader.parts(value.provenance.as_bytes()), [own.clone()], "{target}: the block is kept as a part");
    }
    let source = f.layer.record("source/recording/p").unwrap().provenance.clone();
    assert_eq!(Reader.admit(source.as_bytes()).unwrap_err().code, code::NOT_GRANTED, "a source word is a program's");

    let research = block("program", "research", &[]);
    f.put(vec![recording("laundry", vec![word_step(&[3], 1, Some(&research))])]);
    assert!(matches!(f.convert("laundry", "draft").status, Status::Completed(_)));
    let draft = json::object([("draft", json::object([("record", text("draft/recording/laundry"))]))]);
    assert_eq!(code_of(&f.press("operation.preview", draft)), Some(code::NOT_GRANTED), "the part is read at preview");
    f.play("laundry", None);
    f.drive();
    assert_eq!(f.state("laundry").status, PlayStatus::Waiting(0));
    assert_eq!(f.shell.step_preview(&mut f.layer, "laundry").unwrap_err().code, code::NOT_GRANTED);

    f.put(vec![recording(
        "policy",
        vec![step(POLICY_ROW, json::object([("policy", text("strict"))]), Some(completed()))],
    )]);
    f.play("policy", None);
    f.drive();
    assert_eq!(f.state("policy").status, PlayStatus::Waiting(0), "a layer-confirmed row waits");
    let refused = f.shell.step(&mut f.layer, "policy", false).unwrap();
    assert_eq!(code_of(&refused), Some(code::CONFIRMATION_REQUIRED));
}

// ----- the format packet's playback vectors (EV4.14, EV4.18, EV4.22, PS1.44)

/// `PS1:input.noncanonical_value`, which the vectors name; the command layer registers no constant for it.
const NONCANONICAL: &str = "PS1:input.noncanonical_value";

fn vectors(group: &str) -> Vec<Value> {
    let path = repository().join("docs/architecture/1.0/language/format-packet/vectors.json");
    let all = json::parse(&std::fs::read(path).unwrap()).unwrap();
    all.get(group).and_then(Value::as_array).unwrap_or_else(|| panic!("{group}")).to_vec()
}

fn state_of(given: &Value) -> Option<PlaybackState> {
    let state = given.get("state").filter(|state| !matches!(state, Value::Null))?;
    let step = state.get("step").and_then(Value::as_i64).unwrap() as u64;
    let mut playback = PlaybackState::new("p", "d", step);
    let (stopped, outcome) = (state.get("code").and_then(Value::as_str), state.get("outcome").and_then(Value::as_str));
    playback.status = match state.get("status").and_then(Value::as_str).unwrap() {
        "running" => PlayStatus::Running,
        "waiting" => PlayStatus::Waiting(step),
        "finished" => PlayStatus::Finished,
        "stopped" => match (stopped, outcome) {
            (_, Some(outcome)) => PlayStatus::Stopped(Stop::Outcome(ChildOutcome::from_name(outcome).unwrap())),
            (Some(code), None) => PlayStatus::Stopped(Stop::Code(code.to_owned())),
            (None, None) => PlayStatus::Stopped(Stop::Code(code::STALE_RECORDING_STEP.to_owned())),
        },
        other => panic!("{other}"),
    };
    Some(playback)
}

/// EV4.22's table through [`play_from`], as `recording.play` reads it after reconciliation.
fn play_vector(given: &Value) -> Value {
    let child = given.get("state").and_then(|s| s.get("child")).and_then(Value::as_str);
    let steps = given.get("steps").and_then(Value::as_i64).unwrap() as u64;
    let from = given.get("from").and_then(Value::as_i64);
    if from.is_some_and(|from| from < 0) {
        return text(code::RECORDING_STATE);
    }
    let start = play_from(state_of(given).as_ref(), child.is_some(), steps, from.map(|from| from as u64));
    let entry = |playback: &str, at: u64, map: &str| {
        json::object([("playback", text(playback)), ("step", Value::Int(at as i64)), ("map", text(map))])
    };
    match start {
        Ok(Start::New(at)) => entry("new", at, "empty"),
        Ok(Start::Same(at)) => entry("same", at, "kept"),
        Err(error) => text(error.code),
    }
}

/// EV4.18's map through [`mapped`].
fn undo_vector(actions: &[c600_command::ActionEntry], given: &Value) -> Value {
    let target = given.get("recorded_target").and_then(Value::as_i64).unwrap() as u64;
    let hint = given.get("hint_target").and_then(Value::as_i64).map(|hint| hint as u64);
    match mapped(actions, target, hint) {
        Ok(_) => json::object([("stop", Value::Null), ("undone", Value::Bool(true))]),
        Err(error) => json::object([("stop", text(error.code)), ("undone", Value::Bool(false))]),
    }
}

/// PS1.44's stop through [`PlayStatus::from_json`].
fn stop_vector(stopped: &Value) -> Value {
    let stop = match (stopped.get("code"), stopped.get("outcome")) {
        (Some(code), _) => json::object([("kind", text("code")), ("code", code.clone())]),
        (_, Some(outcome)) => json::object([("kind", text("outcome")), ("outcome", outcome.clone())]),
        _ => panic!("{stopped:?}"),
    };
    match PlayStatus::from_json(&json::object([("kind", text("stopped")), ("stop", stop)])) {
        Some(PlayStatus::Stopped(Stop::Code(code))) => json::object([("code", text(&code))]),
        Some(PlayStatus::Stopped(Stop::Outcome(outcome))) => json::object([("outcome", text(outcome.name()))]),
        _ => text(code::PERSISTENCE_FORMAT),
    }
}

/// CT-EV4-19 (part: EV4.22's table, EV4.18's map and a rewind's empty map on the packet's vectors; not run: live play).
///
/// The format packet's `playback`, `playback_from`, `playback_trace` and `playback_arguments` vectors (EV4.14, EV4.18,
/// EV4.22, PS1.44), each through the layer's own functions: PS1.44's map and stop ([`PlaybackState::from_json`],
/// [`PlayStatus::from_json`], read in this layer's draft form), EV4.18's map ([`mapped`]), EV4.22's table
/// ([`play_from`]) and a rewind's empty map. `playback_arguments` states EV4.14's rule on FP's revision form, which the
/// layer does not read; the test reads the rule the same way, and `ct_ev4_16_stale_steps_stop_and_identities_refuse`
/// runs it live on the layer's own revisions and filter previews.
#[test]
fn the_format_packets_playback_vectors() {
    let mut count = 0;
    for vector in vectors("playback") {
        let id = vector.get("id").and_then(Value::as_str).unwrap();
        let given = vector.get("given").unwrap();
        let mut wire = PlaybackState::new("p", "d", 0).to_json();
        if let Value::Object(fields) = &mut wire {
            fields.insert("actions".into(), given.get("actions").unwrap().clone());
        }
        let got = match (PlaybackState::from_json(&wire), given.get("stopped")) {
            (None, _) => text(NONCANONICAL),
            (Some(_), Some(stopped)) => stop_vector(stopped),
            (Some(state), None) => undo_vector(&state.actions, given),
        };
        assert_eq!(&got, vector.get("expected").unwrap(), "{id}");
        count += 1;
    }
    for vector in vectors("playback_from") {
        let id = vector.get("id").and_then(Value::as_str).unwrap();
        assert_eq!(&play_vector(vector.get("given").unwrap()), vector.get("expected").unwrap(), "{id}");
        count += 1;
    }
    for vector in vectors("playback_trace") {
        let id = vector.get("id").and_then(Value::as_str).unwrap();
        let given = vector.get("given").unwrap();
        let mut state = PlaybackState::new("p", "d", 0);
        for event in given.get("events").and_then(Value::as_array).unwrap() {
            if let Some(step) = event.get("step") {
                let field = |name: &str| step.get(name).and_then(Value::as_i64).unwrap() as u64;
                state.map(field("recorded"), field("played"));
            } else if play_vector(event.get("rewind").unwrap()).get("map") == Some(&text("empty")) {
                state = PlaybackState::new("p2", "d", 0);
            }
        }
        assert_eq!(&undo_vector(&state.actions, given), vector.get("expected").unwrap(), "{id}");
        count += 1;
    }
    for vector in vectors("playback_arguments") {
        let id = vector.get("id").and_then(Value::as_str).unwrap();
        let given = vector.get("given").unwrap();
        let named = given.get("named").unwrap();
        let current = match named.get("kind").and_then(Value::as_str).unwrap() {
            "revision" => named.get("value") == given.get("current"),
            _ => given.get("fresh") == Some(&Value::Bool(true)),
        };
        let got = match current {
            true => json::object([("stop", Value::Null), ("played", Value::Bool(true))]),
            false => json::object([("stop", text(code::STALE_RECORDING_STEP)), ("played", Value::Bool(false))]),
        };
        assert_eq!(&got, vector.get("expected").unwrap(), "{id}");
        count += 1;
    }
    assert_eq!(count, 12 + 26 + 3 + 4);
}

// ----- CT-ERR-18

/// CT-ERR-18 (part: the registry rows with their continuations, and a child's failure forwarded unchanged; not run:
/// the stale, identity, outcome, state and conversion behaviours, which the tests that name its parts run).
/// EV4.14, EV4.19, EV4.22, EV4.23; ERR.7, ERR.14: every EV4
/// recording code the layer uses is chapter 13's, with its continuation (a stale step's is `after_refresh`); an
/// identity mismatch refuses before play and stops during it (`ct_ev4_16`); an unknown recorded outcome stops play and
/// conversion with its own code (`ct_ev4_13`); state and conversion refusals keep theirs (`ct_ev4_17`, `ct_ev4_19`);
/// and a child's refusal is forwarded to the play's receipt with its code and detail unchanged (here).
#[test]
fn ct_err_18_recording_codes_keep_their_identities() {
    let registry =
        std::fs::read_to_string(repository().join("docs/architecture/1.0/language/architecture/13-errors.md")).unwrap();
    for (code, continuation) in [
        (code::RECORDING_STEP_UNRESOLVED, "| |"),
        (code::STALE_RECORDING_STEP, "`after_refresh`"),
        (code::RECORDING_IDENTITY_MISMATCH, "`after_user_change`"),
        (code::RECORDING_STEP_OUTCOME_UNKNOWN, "`after_user_change`"),
        (code::RECORDING_STATE, "`after_user_change`"),
        (code::RECORDING_NOT_CONVERTIBLE, "`after_user_change`"),
    ] {
        let row = registry
            .lines()
            .find(|line| line.starts_with(&format!("| `{code}` |")))
            .unwrap_or_else(|| panic!("{code}"));
        assert!(row.contains(continuation), "{code}: {row}");
    }
    let mut f = fixture();
    f.put(vec![recording("child", vec![step("session.timer.start", json::object([]), Some(completed()))])]);
    let played = f.play("child", None);
    f.drive();
    let answer = f.receipt(&played.command, &played.request);
    let Status::Failed(forwarded) = &answer.status else { panic!("{:?}", answer.status) };
    assert_eq!(forwarded.code, code::ROW_NOT_BUILT, "the child's code, unchanged");
    let state = f.state("child");
    let child_error = state.last.as_ref().and_then(|last| last.get("status")).and_then(|s| s.get("error")).unwrap();
    assert_eq!(child_error, &forwarded.to_json(), "and its detail");
}

/// The draft forms round-trip: a target's name, a child outcome's name and a playback state, whose map stays sorted.
#[test]
fn the_draft_forms_round_trip() {
    for target in [Target::Draft, Target::Source] {
        assert_eq!(Target::from_name(target.name()), Some(target));
    }
    for outcome in [ChildOutcome::Refused, ChildOutcome::Failed, ChildOutcome::Cancelled, ChildOutcome::UnknownOutcome]
    {
        assert_eq!(ChildOutcome::from_name(outcome.name()), Some(outcome));
    }
    let mut state = PlaybackState::new("p", "d", 3);
    state.map(2, 9);
    state.map(1, 8);
    state.status = PlayStatus::Stopped(Stop::Outcome(ChildOutcome::Cancelled));
    assert_eq!(PlaybackState::from_json(&state.to_json()), Some(state.clone()));
    assert_eq!(state.actions.iter().map(|e| e.recorded).collect::<Vec<_>>(), [1, 2]);
}

// ----- the recorder's failed and unknown writes, and the generated module's comments

/// A write the recorder refuses, as a full disk would.
fn rolled_back() -> Fate {
    Fate::RolledBack(Error::new(code::PERSISTENCE_FORMAT, "an injected write that rolled back"))
}

/// PS1.44, EV4.20, EV4.22: a recorder write that rolls back leaves the playback state as the recorder's last durable
/// write left it, never as the write would have made it. A job that ends on such a write, at an unattended step's
/// pending entry or at the write that leaves `running` for a user step, answers `failed` with the write's error and
/// leaves its state `running` with no live job, as a restart would find it. The next `recording.play` stops it with
/// `KN5:resource_limit.engine_lost` first, a write made before EV4.22's table decides, so a play the table refuses
/// has still made it; a play the table admits then continues from the cursor. No undo is involved.
#[test]
fn a_rolled_back_state_write_keeps_the_durable_state_and_the_next_play_recovers_it() {
    // At an unattended step's pending entry: nothing is dispatched.
    let mut f = fixture();
    f.put(vec![recording("c", vec![completed_write("w", &[1], 1), completed_write("v", &[2], 2)])]);
    let played = f.play("c", None);
    let durable = f.recorder.held().playbacks["c"].clone();
    assert_eq!((durable.status.clone(), durable.step), (PlayStatus::Running, 0));
    f.recorder.inject(rolled_back());
    f.layer.run_jobs();
    assert_eq!(f.state("c"), durable, "the last durable write, with no pending entry");
    assert_eq!(f.log(), 0, "nothing was dispatched");
    let answer = f.receipt(&played.command, &played.request);
    assert_eq!(answer.status.code(), Some(code::PERSISTENCE_FORMAT), "the job failed with the write's error");
    let again = f.play("c", None);
    assert!(matches!(again.status, Status::Running(_)), "not stuck at `running`: {:?}", again.status);
    f.drive();
    assert_eq!((f.state("c").status, f.log()), (PlayStatus::Finished, 2));
    assert_eq!(f.recorder.held().playbacks["c"], f.state("c"));

    // At the write that leaves `running` for a user step: the job ends, and the next play waits at the step.
    let mut f = fixture();
    f.put(vec![recording("m", vec![word_step(&[1], 1, None)])]);
    f.play("m", None);
    let durable = f.recorder.held().playbacks["m"].clone();
    f.recorder.inject(rolled_back());
    f.layer.run_jobs();
    assert_eq!(f.state("m"), durable, "still `running`, as durably written");
    let writes = f.recorder.writes();
    assert_eq!(code_of(&f.play("m", Some(5))), Some(code::RECORDING_STATE), "the table refuses `from` past the end");
    let recovered = f.recorder.held().playbacks["m"].clone();
    assert_eq!(recovered.status, PlayStatus::Stopped(Stop::Code(code::ENGINE_LOST.to_owned())));
    assert_eq!((f.recorder.writes(), f.state("m")), (writes + 1, recovered), "the recovery was written first");
    assert!(matches!(f.play("m", None).status, Status::Running(_)));
    f.drive();
    assert_eq!(f.state("m").status, PlayStatus::Waiting(0));
    assert_eq!(f.layer.revision().state, 0, "nothing committed");
}

/// EV4.22, PS1.44: `job.stop` answers the fate of its write. One that rolls back stops nothing: the answer is `failed`
/// with the write's error, the job stays live and the state is the durable one. One whose fate is unknown ends the job
/// with `cancelled(by_user)` and answers `unknown_outcome`; the next row reads the recorder again, so a stop that did
/// not land leaves the durable state, whose pending child is reconciled once when it reports, and the next play
/// continues from the cursor after the `engine_lost` recovery. No undo is involved.
#[test]
fn job_stop_answers_the_fate_of_its_write() {
    let mut f = fixture();
    f.put(vec![recording("r", vec![completed_rotate(), completed_rotate()])]);
    let played = f.play("r", None);
    let Status::Running(job) = played.status else { panic!("{:?}", played.status) };
    f.layer.run_jobs();
    let taken = f.layer.take_program_job().expect("the first step's job");
    let before = f.state("r");
    assert!(before.pending.is_some());
    let stop = |f: &mut F| f.press("job.stop", json::object([("job", Value::Int(job as i64))]));

    f.recorder.inject(rolled_back());
    assert_eq!(code_of(&stop(&mut f)), Some(code::PERSISTENCE_FORMAT));
    assert_eq!(f.state("r"), before, "nothing stopped");
    assert!(matches!(f.receipt(&played.command, &played.request).status, Status::Running(_)), "the job is live");

    f.recorder.inject(Fate::Unknown); // the stop does not land
    assert_eq!(stop(&mut f).status, Status::UnknownOutcome);
    assert_eq!(f.receipt(&played.command, &played.request).status, Status::Cancelled(CancelReason::ByUser));
    assert_eq!(code_of(&stop(&mut f)), Some(code::ROW_NOT_BUILT), "the job ended");
    assert_eq!(f.state("r"), before, "the next row read the recorder again: the stop did not land");
    f.layer.finish_program_job(taken.job, Ok(Value::Null)).unwrap();
    let reconciled = f.state("r");
    assert_eq!((reconciled.step, reconciled.pending.is_none()), (1, true), "the child's completion, once");
    assert!(matches!(f.play("r", None).status, Status::Running(_)), "the cursor continues after the recovery");
    f.drive();
    assert_eq!(f.state("r").status, PlayStatus::Finished);
}

/// EV4.19's unknown outcome on the recorder (EV4.11, PS1.30): a `recording.stop` whose write's fate is unknown answers
/// `unknown_outcome` and keeps the capture, and the layer does not assume the write landed. The next row reads the
/// recorder again: a recording that landed is found by `recording.play`; one that did not stays absent, and a second
/// stop stores it whole.
#[test]
fn an_unknown_recording_write_is_read_again_at_the_next_row() {
    let stop = |f: &mut F, name: &str| f.press(RECORDING_STOP_ROW, json::object([("recording", text(name))]));

    // Landed.
    let mut f = fixture();
    f.start("r");
    assert!(matches!(f.write("w", &[1]).status, Status::Completed(_)));
    f.recorder.set_unknown_writes(true);
    f.recorder.inject(Fate::Unknown);
    assert_eq!(stop(&mut f, "r").status, Status::UnknownOutcome);
    assert_eq!((f.layer.recording("r"), f.layer.capturing()), (None, Some("r")), "not assumed; the capture stays");
    assert!(f.recorder.held().recordings.contains_key("r"), "the write landed");
    let played = f.play("r", None);
    assert!(matches!(played.status, Status::Running(_)), "the play read the recorder again: {:?}", played.status);
    assert_eq!(f.layer.recording("r"), f.recorder.held().recordings.get("r"));
    f.drive();
    assert_eq!(f.state("r").status, PlayStatus::Finished);

    // Not landed.
    let mut f = fixture();
    f.start("s");
    assert!(matches!(f.write("w", &[1]).status, Status::Completed(_)));
    f.recorder.inject(Fate::Unknown);
    assert_eq!(stop(&mut f, "s").status, Status::UnknownOutcome);
    assert_eq!(code_of(&f.play("s", None)), Some(code::NAMED_OBJECT_INVALID), "it did not land");
    assert_eq!(f.layer.capturing(), Some("s"));
    let stored = f.stop("s");
    assert_eq!((stored.steps.len(), f.recorder.held().recordings.get("s")), (1, Some(&stored)));
}

/// EV4.13 under LG1.3 (pinned source: UTF-8, LF line ends, no CR; a `#` comment runs to LF): the generated module's
/// comments stay one line whatever the recording's name or a call's arguments hold. CR, LF, NEL, U+2028 and U+2029
/// are written as `\u{...}` escapes, so the text holds no raw line end but LF and no CR, and exactly the header and
/// one line per call are comments. `c600-durable`'s `tests/ps1_44_carrier.rs` parses and lowers such a module.
#[test]
fn a_source_conversion_keeps_its_comments_on_one_line() {
    let name = "r\r\n# x\u{85}\u{2028}\u{2029}end";
    let mut f = fixture();
    let mut rotate = completed_rotate();
    rotate.arguments = json::object([("rotation", text("quarter\u{2028}\r")), ("view", text("main"))]);
    f.put(vec![recording(name, vec![word_step(&[1, 2], 1, None), rotate])]);
    let converted = f.convert(name, "source");
    assert!(matches!(converted.status, Status::Completed(_)), "{:?}", converted.status);
    let record = f.layer.record(&format!("source/recording/{name}")).expect("the source record");
    let source = record.payload.get("source").and_then(|s| s.get("text")).and_then(Value::as_str).unwrap();
    for raw in ['\r', '\u{85}', '\u{2028}', '\u{2029}'] {
        assert!(!source.contains(raw), "{raw:?} in {source}");
    }
    let comments: Vec<&str> = source.lines().filter(|line| line.starts_with('#')).collect();
    assert_eq!(comments.len(), 2, "{source}");
    assert!(comments[0].contains("`r\\u{000d}\\u{000a}# x\\u{0085}\\u{2028}\\u{2029}end`"), "{}", comments[0]);
    assert!(comments[1].starts_with("# call 1: `camera.rotate`") && comments[1].contains("quarter"), "{}", comments[1]);
}
