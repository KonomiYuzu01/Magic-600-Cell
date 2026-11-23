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

/// CT-EV4-11 (EV4.10, EV4.11), CT-EV4-08 (part: a recording retains semantic commands, canonical arguments,
/// identities, word and action references, revisions and receipt bindings), CT-PS1-15 (part: the round trip of every
/// `RecordedStep` field), CT-PS1-24 (part: completed, committed and refused receipts round-trip unchanged; a missing
/// terminal stays absent; no token or operation preview ID is stored). A completed record write, a completed preview, a
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

// ----- CT-EV4-12: the eligibility table

/// EV4.22's table, as this test reads it, for a row of `kind`, `lane`, `job` and `gates`.
fn expected(kind: &str, lane: &str, job: bool, gates: &[&str]) -> bool {
    let gated = gates.contains(&"confirm") || gates.contains(&"file_check");
    let user =
        lane == "commit" || kind == "commit" || gated || kind == "file" || kind == "app" || lane == "exclusive" || job;
    !user && matches!((kind, lane), ("view" | "workspace", "view" | "none") | ("query", "none"))
}

/// CT-EV4-12 (EV4.17, EV4.18, EV4.22): the eligibility table enumerated whole, across every kind, lane, job flag and
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

/// CT-EV4-12 (part: at playback). One recording holds a workspace edit, a camera rotation, a no-job query in lane none,
/// a layout step, a `confirm` row, a policy change, a language-defined job row and a word commit. The first four play
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

// ----- CT-EV4-13: EV4.19's two tables

/// CT-EV4-13 (EV4.19): both tables whole. Historical: `completed` plays (or waits when not eligible), `committed`
/// waits, `refused`, `failed` and `cancelled` are passed over and stay in the recording, and `accepted`, `running`, no
/// terminal record and `unknown_outcome` stop playback and conversion with `EV4:input.recording_step_outcome_unknown`.
/// New child: `accepted` and `running` are pending, `completed` advances either step, `committed` advances only a user
/// step, and `refused`, `failed`, `cancelled` and `unknown_outcome` stop with that outcome.
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

/// CT-EV4-15 (EV4.21; X1, owner decision of 23 November 2026, item 2.7). A playback of an undoable workspace edit
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

// ----- CT-EV4-16: staleness and identity

/// CT-EV4-16 (EV4.14), CT-ERR-18 (part: stale-step stops and identity refusals). An unattended step naming the current
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

// ----- CT-EV4-14 and CT-PS1-25: the crash points, synthetic

/// CT-EV4-14 and CT-PS1-25 (EV4.20, PS1.44; synthetic: the recorder's injected fates stand in for the crash, and a
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

/// CT-EV4-17 (EV4.23) on both targets, the five vectors. Every refusal writes nothing: the journal, the records and the
/// revision stay as they were. (1) A camera rotation alone refuses the empty draft and becomes one permitted source
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

/// CT-EV4-18 (EV4.23). A redo restores a word at its recorded place; an undo of a recorded non-word action and a
/// checkpoint save add no draft letters but refuse source; completed non-word steps enter the draft report; source takes
/// only calls an overlay row may invoke (KN5.4) with no `confirm` or `file_check` gate and refuses a file or app row. A
/// successful draft conversion is one undoable action that writes a Macro-phase draft and keeps every contributing
/// word's block in its block's parts. X1: the conversion's record write is the move history's undo target, as
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
}

// ----- CT-EV4-19 and CT-EV4-20: the play table, rewinds and the map

/// CT-EV4-19 (EV4.22), CT-ERR-18 (part: state refusals keep their own code). A play or `from` outside EV4.22's table
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

/// CT-EV4-20 (EV4.18, PS1.44; X1: `session.undo` and `session.redo` on the move history, which EV4.15's single order
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

/// CT-EV4-09 (EV4.12, EV4.18), CT-PS1-15 (part: commit playback needs a new explicit action and token). No recording or
/// playback state holds a token or a preview ID, and none is replayed: a recorded preview and a recorded word commit
/// wait for the solver's step actions, the commit's with a new whole-word preview of the stored word, never the
/// recorded preview ID; a recorded checkpoint save waits too, and its step action mints a new token and fills the
/// current revision (the layer does not build `session.checkpoint.save`, so its send is refused, which stops the
/// playback). A recorded live twist (ONE.78) takes no preview: its step action commits it as a press does.
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
}

/// CT-EV4-10 (EV4.13, EV4.16, EV4.23), CT-BND-03 (part: a word carried through a workspace record, a recording and a
/// conversion to source keeps its block, BND.32), CT-PS1-17 (part: a state-derived word survives a recording and a
/// source conversion with its block). A word written with a block, previewed from its record and committed is recorded
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

/// CT-ERR-18 (EV4.14, EV4.19, EV4.22, EV4.23; ERR.7, ERR.14), whole with the tests that name its parts: every EV4
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
