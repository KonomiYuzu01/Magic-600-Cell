//! `filter.apply` and its filter preview (draft; LP3's C6, its first common need): the gate `fresh_filter_preview`
//! (LI.41), the filter preview `filter.preview` makes (H15.4) and its freshness (KN0.9), the argument forms `FilterRef`
//! and `FilterPreviewRef`, and the workspace inverse of an applied filter.
//!
//! CT-LI-11 (source, whole): a fresh filter preview applies with a workspace completion while a word preview waits
//! unpresented; a state change or a change to a record the filter read refuses the apply with
//! `KN5:stale.filter_preview` and leaves the workspace as it was; an unread workspace edit keeps it fresh.
//!
//! The frozen text is silent on how a `FilterRef` names its records, so these tests follow the draft rulings recorded
//! in `src/lib.rs` and `src/filter.rs`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::filter::preview_ref;
use c600_command::{
    code, draft_args, CommandLayer, Completion, CompletionKind, Direction, Error, HeadlessFrames, JobOutput, Kind,
    MemoryStore, Overlay, Provenance, Receipt, RecordValue, ResultRef, Secret, Shell, Status, Table, Transaction,
    TransactionKind, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

const LAYER: &str = "structure.layer";
const PREVIEW: &str = "filter.preview";
const APPLY: &str = "filter.apply";
/// A language-defined workspace row the tests register: its job's completion is an undoable action of its own (LP2's
/// reading 18).
const NOTE: &str = "tool.full.note";
/// A stored workspace row whose undo class is `none`, which the tests serve: no undo reaches its job's writes.
const PLAY: &str = "recording.play";

/// The record the test's predicate job writes, and the record its apply job writes.
const PREDICATE: &str = "layer";
const FILTERED: &str = "filtered";

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

fn letters(moves: &[i64]) -> Value {
    Value::Array(moves.iter().map(|&letter| Value::Int(letter)).collect())
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/representations".into(), entry: entry.into() }
}

fn layer_args(cell: i64, layer: i64) -> Value {
    json::object([("cell", Value::Int(cell)), ("layer", Value::Int(layer))])
}

fn apply_args(preview: Value) -> Value {
    json::object([("preview", preview)])
}

fn record_args(name: &str, payload: Value) -> Value {
    json::object([("payload", payload), ("provenance", text("block")), ("record", text(name))])
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

/// A `FilterRef`: the result reference of the job that prepared the predicate.
fn filter_ref(job: u64, digest: &str) -> Value {
    json::object([("digest", text(digest)), ("job", Value::Int(job as i64))])
}

/// The job a `FilterPreviewRef` names.
fn job_of(reference: &Value) -> u64 {
    match reference.get("job") {
        Some(Value::Int(job)) => *job as u64,
        other => panic!("{other:?}"),
    }
}

/// An output whose records each have the provenance block `block`.
fn output(result: Value, records: &[(&str, Value)]) -> JobOutput {
    let block = |payload: &Value| RecordValue { payload: payload.clone(), provenance: "block".to_owned() };
    let records = records.iter().map(|(name, payload)| ((*name).to_owned(), block(payload)));
    JobOutput { value: result, views: Vec::new(), records: records.collect() }
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// The host's provenance reader as the tests stand it in: it accepts every block.
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

/// A session at the solved state that protects nothing, on H-08's table with the layer's drafts (`workspace.set`), with
/// the predicate row, `filter.preview`, `filter.apply` and `recording.play` served, the language-defined row
/// registered, and the test reader.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    serve(&mut layer);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// What a host does at every open: serves its rows again and registers its language-defined row.
fn serve(layer: &mut CommandLayer) {
    layer.serve_row(LAYER, overlay("layer_predicate")).unwrap();
    layer.serve_row(PREVIEW, overlay("filter_counts")).unwrap();
    layer.serve_row(APPLY, overlay("apply_filter")).unwrap();
    layer.serve_row(PLAY, overlay("play")).unwrap();
    layer.register_overlay(NOTE, Kind::Workspace, &[], overlay("note")).unwrap();
}

/// A reopen of `layer`'s store, which replays its actions with the test reader.
fn reopen(layer: &CommandLayer) -> CommandLayer {
    let store = layer.store().clone();
    let mut reopened = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    reopened.set_provenance(Box::new(Reader));
    serve(&mut reopened);
    reopened
}

impl Fixture {
    /// Invokes `row` through the command route, takes its job and returns the job's ID.
    fn start(&mut self, row: &str, args: Value) -> u64 {
        let started = self.shell.invoke(&mut self.layer, row, args).unwrap();
        let Status::Running(job) = started.status else { panic!("{row}: {:?}", started.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job), "{row}");
        job
    }

    fn finish(&mut self, job: u64, out: JobOutput) -> Receipt {
        self.layer.finish_program_job_with(job, Ok(out)).expect("the job was running")
    }

    /// Prepares a predicate whose job writes `payload` to the predicate's record, and returns its `FilterRef`.
    fn predicate(&mut self, payload: Value) -> Value {
        let job = self.start(LAYER, layer_args(1, 2));
        let out = output(Value::Null, &[(PREDICATE, payload)]);
        assert!(completed(&self.finish(job, out.clone())));
        filter_ref(job, &out.digest())
    }

    /// The receipt of `filter.preview` for `predicate`, at the current revision, before its job runs.
    fn ask_preview(&mut self, predicate: Value) -> Receipt {
        let args = json::object([("predicate", predicate), ("revision", self.layer.revision().to_json())]);
        self.shell.invoke(&mut self.layer, PREVIEW, args).unwrap()
    }

    /// A filter preview of `predicate`: its job completes with counts, and the layer keeps it under the job's ID. Its
    /// `FilterPreviewRef` names the job and the current epoch.
    fn preview(&mut self, predicate: Value) -> Value {
        let receipt = self.ask_preview(predicate);
        let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job));
        let counts = json::object([("kept", Value::Int(12)), ("total", Value::Int(600))]);
        let before = self.layer.revision().clone();
        let done = self.finish(job, JobOutput::of(counts.clone()));
        let reference = ResultRef { job: Some(job), digest: JobOutput::of(counts).digest() };
        assert_eq!(
            done.status,
            Status::Completed(Completion { kind: CompletionKind::Query, revision: before, result: Some(reference) })
        );
        let reference = self.layer.filter_preview(job).expect("kept").reference();
        assert_eq!(reference, preview_ref(job, self.layer.revision().epoch));
        reference
    }

    /// `filter.apply` of filter preview `preview`, its job writing `payload` to the filtered record.
    fn apply(&mut self, preview: &Value, payload: Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, APPLY, apply_args(preview.clone())).unwrap();
        let Status::Running(job) = receipt.status else { return receipt };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job));
        self.finish(job, output(Value::Null, &[(FILTERED, payload)]))
    }

    /// A job of `recording.play`, a stored row whose undo class is `none`, whose output writes `payload` to `name`: no
    /// undo reaches the write.
    fn play(&mut self, name: &str, payload: Value) -> Receipt {
        let job = self.start(PLAY, json::object([("from", Value::Int(0)), ("recording", text("r1"))]));
        self.finish(job, output(Value::Null, &[(name, payload)]))
    }

    fn refused(&mut self, row: &str, args: Value) -> &'static str {
        let receipt = self.shell.invoke(&mut self.layer, row, args).unwrap();
        assert!(matches!(receipt.status, Status::Refused(_)), "{row}: {:?}", receipt.status);
        assert!(self.layer.take_program_job().is_none(), "{row}");
        code_of(&receipt)
    }

    fn write(&mut self, name: &str, payload: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, RECORD_ROW, record_args(name, payload)).unwrap()
    }

    /// Admits and runs a word preview of `recipe`, unpresented, and returns its ID.
    fn word_preview(&mut self, recipe: &Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        id
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let id = self.word_preview(recipe);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let press = self.shell.prepare(&self.layer, "operation.execute", preview_args(id)).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// The undo or redo of the action on top of its stack.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    /// The record's version and value as the layer holds it.
    fn record(&self, name: &str) -> (u64, Option<Value>) {
        let record = self.layer.records().get(name).cloned().unwrap_or_default();
        (record.version, record.value.map(|value| value.payload))
    }

    /// What a refused or failed apply may not change: the revision, the records and the durable log.
    fn fixed(&self) -> (String, String, usize) {
        let (revision, records) = (self.layer.revision(), self.layer.records());
        (format!("{revision:?}"), format!("{records:?}"), self.layer.store().log.len())
    }
}

/// CT-LI-11, fresh: `filter.apply` of a fresh filter preview completes with a workspace completion that writes its
/// records in one durable action and moves the workspace revision once. It reads neither the word-preview slot nor a
/// presentation: it completes while a word preview waits unpresented, and that preview stays as it was.
#[test]
fn a_fresh_filter_preview_applies_without_a_word_preview_or_a_presentation() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1, 2, 3]));
    let preview = f.preview(predicate);
    let waiting = f.word_preview(&word(&[1]));
    let shown = f.layer.preview_state(waiting);
    let before = f.layer.revision().clone();
    let log = f.layer.store().log.len();

    let done = f.apply(&preview, letters(&[4, 5]));
    let Status::Completed(Completion { kind, revision, result }) = &done.status else { panic!("{:?}", done.status) };
    assert_eq!(*kind, CompletionKind::Workspace);
    let mut moved = before.clone();
    moved.workspace += 1;
    assert_eq!((revision, f.layer.revision()), (&moved, &moved));
    assert!(result.is_some());
    assert_eq!(f.record(FILTERED), (1, Some(letters(&[4, 5]))));
    assert_eq!(f.layer.store().log.len(), log + 1);
    assert_eq!(f.layer.store().log.last().map(|t| t.kind), Some(TransactionKind::Workspace));
    assert_eq!(f.layer.preview_state(waiting), shown);

    // The applied preview stays fresh: its filter did not read the record the apply wrote.
    assert!(completed(&f.apply(&preview, letters(&[6]))));
}

/// The filter preview binds what KN0.9 names: the epoch, the model identity, the mode, the state revision and the
/// digest of the resolved filter, which holds the content digest and version of every record read to resolve it. The
/// protection and workspace revisions are not among them.
#[test]
fn a_filter_preview_binds_the_state_and_the_records_its_filter_read() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let preview = f.preview(predicate.clone());
    let kept = f.layer.filter_preview(job_of(&preview)).unwrap().clone();
    assert_eq!(kept.reference(), preview);
    assert!(f.layer.filter_fresh(&kept));
    assert_eq!(kept.deps, f.layer.filter_deps(&kept.resolved.digest()));
    assert_eq!(kept.resolved.source, predicate);
    let reads: Vec<(&str, u64)> = kept.resolved.reads.iter().map(|r| (r.record.as_str(), r.version)).collect();
    assert_eq!(reads, [(PREDICATE, 1)]);
    let keys: Vec<String> = match kept.deps.to_json() {
        Value::Object(fields) => fields.keys().cloned().collect(),
        other => panic!("{other:?}"),
    };
    assert_eq!(keys, ["epoch", "filter", "identity", "mode", "state"]);
    assert_eq!(kept.deps.state, f.layer.revision().state);
}

/// CT-LI-11, an unread edit: a write to a record the filter did not read leaves the filter preview fresh.
#[test]
fn an_unread_edit_leaves_the_filter_preview_fresh() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let preview = f.preview(predicate);
    assert!(completed(&f.write("notes", letters(&[9]))));
    assert!(completed(&f.apply(&preview, letters(&[2]))));
}

/// CT-LI-11, a state change: a commit makes the filter preview stale, and `filter.apply` is refused with
/// `KN5:stale.filter_preview` without a job or a workspace change.
#[test]
fn a_state_change_makes_the_filter_preview_stale() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let preview = f.preview(predicate);
    assert!(committed(&f.commit(&word(&[1]))));
    let before = f.fixed();
    assert_eq!(f.refused(APPLY, apply_args(preview.clone())), code::STALE_FILTER_PREVIEW);
    assert_eq!(f.fixed(), before);
}

/// CT-LI-11, a record the filter read: writing the predicate's record again, by `workspace.set` or by a second run of
/// the predicate row, makes the filter preview stale. The predicate no longer resolves either: a new preview of it is
/// refused with `KN2:input.named_object_invalid`.
#[test]
fn a_change_to_a_record_the_filter_read_makes_it_stale() {
    for rerun in [false, true] {
        let mut f = fixture();
        let predicate = f.predicate(letters(&[1]));
        let preview = f.preview(predicate.clone());
        if rerun {
            f.predicate(letters(&[1]));
        } else {
            assert!(completed(&f.write(PREDICATE, letters(&[1]))));
        }
        let before = f.fixed();
        assert_eq!(f.refused(APPLY, apply_args(preview.clone())), code::STALE_FILTER_PREVIEW, "{rerun}");
        assert_eq!(f.fixed(), before, "{rerun}");
        assert_eq!(f.ask_preview(predicate).status.code(), Some(code::NAMED_OBJECT_INVALID), "{rerun}");
    }
}

/// A `FilterPreviewRef` that names no filter preview the layer keeps is stale: an unknown job, the job of the
/// predicate row, the job of `filter.apply` itself, and the filter preview's job with another epoch. One that is not
/// `{"epoch": Nat, "job": Nat}`, the bare job ID among them, is refused with `KN5:input.argument_form` before the
/// gate.
#[test]
fn a_missing_or_malformed_filter_preview_is_refused() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let layer_job = match predicate.get("job") {
        Some(Value::Int(job)) => *job,
        other => panic!("{other:?}"),
    };
    let preview = f.preview(predicate);
    let Status::Completed(_) = f.apply(&preview, letters(&[2])).status else { panic!("not applied") };
    let apply_job = f.layer.store().log.last().and_then(|t| t.job).unwrap();
    let before = f.fixed();
    let (job, epoch) = (job_of(&preview), f.layer.revision().epoch);
    for named in [preview_ref(999, epoch), preview_ref(layer_job as u64, epoch), preview_ref(apply_job, epoch)] {
        let what = format!("{named:?}");
        assert_eq!(f.refused(APPLY, apply_args(named)), code::STALE_FILTER_PREVIEW, "{what}");
    }
    assert_eq!(f.refused(APPLY, apply_args(preview_ref(job, epoch ^ 1))), code::STALE_FILTER_PREVIEW);
    let n = |value: u64| Value::Int(value as i64);
    let malformed = [
        n(job),
        json::object([("epoch", n(epoch)), ("job", Value::Int(-1))]),
        json::object([("epoch", Value::Int(-1)), ("job", n(job))]),
        json::object([("job", n(job))]),
        json::object([("epoch", n(epoch)), ("extra", Value::Null), ("job", n(job))]),
        json::object([("epoch", text("1")), ("job", n(job))]),
        text("1"),
        Value::Null,
        letters(&[1]),
    ];
    for malformed in malformed {
        let what = format!("{malformed:?}");
        assert_eq!(f.refused(APPLY, apply_args(malformed)), code::ARGUMENT_FORM, "{what}");
    }
    assert_eq!(f.fixed(), before);
}

/// The gate holds again when the apply's output is published (KN0.9): a filter preview that went stale while the job
/// ran fails the job with `KN5:stale.filter_preview`, and nothing it wrote is written.
#[test]
fn a_filter_preview_that_goes_stale_while_the_apply_runs_fails_it() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let preview = f.preview(predicate);
    let job = f.start(APPLY, apply_args(preview.clone()));
    assert!(completed(&f.write(PREDICATE, letters(&[2]))));
    let before = f.fixed();
    let done = f.finish(job, output(Value::Null, &[(FILTERED, letters(&[3]))]));
    assert_eq!(code_of(&done), code::STALE_FILTER_PREVIEW);
    assert!(matches!(done.status, Status::Failed(_)));
    assert_eq!(f.fixed(), before);
    assert_eq!(f.record(FILTERED), (0, None));
}

/// A `FilterRef` must name a completed predicate job with its result digest whose writes still stand, or the row that
/// takes it is refused with `KN2:input.named_object_invalid`; one that is not a result reference is refused with
/// `KN5:input.argument_form`.
#[test]
fn a_filter_ref_names_a_predicate_whose_writes_stand() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let (job, digest) = match (predicate.get("job"), predicate.get("digest")) {
        (Some(Value::Int(job)), Some(Value::String(digest))) => (*job as u64, digest.clone()),
        other => panic!("{other:?}"),
    };
    let preview = f.preview(predicate.clone());
    for named in [filter_ref(999, &digest), filter_ref(job, "00"), filter_ref(job_of(&preview), &digest)] {
        let what = format!("{named:?}");
        assert_eq!(f.ask_preview(named).status.code(), Some(code::NAMED_OBJECT_INVALID), "{what}");
    }
    let malformed = [
        json::object([("digest", text(&digest)), ("job", text("1"))]),
        json::object([("job", Value::Int(job as i64))]),
        json::object([("digest", text(&digest)), ("extra", Value::Null), ("job", Value::Int(job as i64))]),
        json::object([("digest", text(&digest)), ("job", Value::Int(-1))]),
        text(&digest),
    ];
    for named in malformed {
        let what = format!("{named:?}");
        assert_eq!(f.ask_preview(named).status.code(), Some(code::ARGUMENT_FORM), "{what}");
    }
    assert!(f.layer.take_program_job().is_none());
    // The predicate still resolves after the refusals.
    f.preview(predicate);
}

/// A predicate whose record is written again while its filter preview's job runs no longer resolves when the job
/// completes: the query completes, but the layer keeps no filter preview for it, so an apply of it is stale.
#[test]
fn a_predicate_written_over_while_its_preview_runs_keeps_no_filter_preview() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let receipt = f.ask_preview(predicate);
    let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(job));
    assert!(completed(&f.write(PREDICATE, letters(&[2]))));
    let done = f.finish(job, JobOutput::of(Value::Int(1)));
    assert!(matches!(done.status, Status::Completed(Completion { kind: CompletionKind::Query, .. })), "{done:?}");
    assert!(f.layer.filter_preview(job).is_none());
    let before = f.fixed();
    let named = preview_ref(job, f.layer.revision().epoch);
    assert_eq!(f.refused(APPLY, apply_args(named)), code::STALE_FILTER_PREVIEW);
    assert_eq!(f.fixed(), before);
}

/// A reopen keeps no filter preview (they are transient), but it rebuilds the predicates from the journal, so a
/// predicate prepared before the reopen previews and applies after it. Job IDs start again after a reopen, so the new
/// filter preview takes the job ID of the one before it; the reference from before the reopen names the earlier epoch
/// and stays stale for the hint and the shell, and only the new reference applies.
#[test]
fn a_reopen_keeps_no_filter_preview_but_its_predicates() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let preview = f.preview(predicate.clone());
    let mut g = Fixture {
        layer: reopen(&f.layer),
        shell: Shell::new(Secret::fixed(SECRET), "shell"),
        frames: HeadlessFrames::new(),
    };
    assert!(g.layer.filter_preview(job_of(&preview)).is_none());
    assert_eq!(g.refused(APPLY, apply_args(preview.clone())), code::STALE_FILTER_PREVIEW);
    let again = g.preview(predicate);
    assert_eq!(job_of(&again), job_of(&preview), "job IDs start again after a reopen");
    assert_ne!(again, preview);
    let before = g.fixed();
    let hint = g.layer.availability_for(APPLY, &apply_args(preview.clone())).map_err(|error| error.code);
    assert_eq!(hint, Err(code::STALE_FILTER_PREVIEW));
    assert_eq!(g.refused(APPLY, apply_args(preview)), code::STALE_FILTER_PREVIEW);
    assert_eq!(g.fixed(), before);
    assert!(completed(&g.apply(&again, letters(&[2]))));
    assert_eq!(g.record(FILTERED), (1, Some(letters(&[2]))));
}

/// An apply whose job writes no record completes but changes nothing an undo could restore, so it takes no undo
/// entry: the undo stack's top stays the predicate's completion before it, before and after a reopen.
#[test]
fn an_apply_that_writes_no_record_takes_no_undo_entry() {
    let mut f = fixture();
    assert!(completed(&f.write("w", letters(&[1]))));
    let predicate = f.predicate(letters(&[1]));
    let top = f.layer.undo_hint(Direction::Undo).map(|hint| hint.target);
    let preview = f.preview(predicate);
    let job = f.start(APPLY, apply_args(preview.clone()));
    assert!(completed(&f.finish(job, JobOutput::of(Value::Null))));
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), top);
    let g = reopen(&f.layer);
    assert_eq!(g.undo_hint(Direction::Undo), f.layer.undo_hint(Direction::Undo));
}

/// A predicate row's completion is an undoable action of its own: H-08 gives `structure.layer` the undo class
/// `workspace` (LB7.2, since the command table's freeze). So is a language-defined workspace row's (LG3.5, LB7.2; LP2's
/// reading 18). When a job of either writes a record an applied filter wrote, the apply keeps its place beneath it:
/// the job's undo gives back the apply's value, and the apply's undo the value before it. With the apply undone first,
/// the job's action clears the redo stack, as every new undoable action does, and its undo gives back the empty
/// record. A reopen, which replays the language-defined row's completion before the host registers the row again,
/// offers the same undo and redo.
#[test]
fn a_job_that_writes_over_an_applied_filter_is_its_own_undo_entry() {
    for (writer, undone) in [(LAYER, false), (LAYER, true), (NOTE, false), (NOTE, true)] {
        let mut f = fixture();
        assert!(completed(&f.write("w", letters(&[1]))));
        let predicate = f.predicate(letters(&[1]));
        let below = f.layer.undo_hint(Direction::Undo).map(|hint| hint.target);
        let preview = f.preview(predicate);
        assert!(completed(&f.apply(&preview, letters(&[7]))));
        if undone {
            assert!(committed(&f.step(Direction::Undo)));
            assert!(f.layer.undo_hint(Direction::Redo).is_some());
        }
        let args = if writer == LAYER { layer_args(1, 2) } else { json::object([]) };
        let job = f.start(writer, args);
        assert!(completed(&f.finish(job, output(Value::Null, &[(FILTERED, letters(&[9]))]))));
        assert_eq!(f.record(FILTERED).1, Some(letters(&[9])), "{writer} {undone}");
        assert!(f.layer.undo_hint(Direction::Redo).is_none(), "{writer} {undone}");
        let g = reopen(&f.layer);
        for direction in [Direction::Undo, Direction::Redo] {
            assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{writer} {undone} {direction:?}");
        }
        assert!(committed(&f.step(Direction::Undo)));
        let applied = if undone { None } else { Some(letters(&[7])) };
        assert_eq!(f.record(FILTERED).1, applied, "{writer} {undone}: the job's undo");
        if !undone {
            assert!(committed(&f.step(Direction::Undo)));
            assert_eq!(f.record(FILTERED).1, None, "{writer}: the apply's undo");
        }
        assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), below, "{writer} {undone}");
        assert_eq!(f.record("w").1, Some(letters(&[1])), "{writer} {undone}");
    }
}

/// The cut runs on through the other records of an apply it drops. The apply writes `filtered` and `b`, after
/// earlier writes of `w` and `b`, and a job of `recording.play`, a stored row whose undo class is `none` and so one no
/// undo reaches, then writes `filtered`:
/// - The apply leaves the undo stack, and so does the earlier write of `b`: its undo would write over the apply's `b`,
///   which no step reaches now. The write of `w` keeps its place, and so does the predicate's completion beneath it.
/// - A write of `b` after the apply keeps its place: its undo gives back the apply's `b`.
/// - With the apply undone first, its writes are gone: only the apply leaves the redo stack, and the earlier write of
///   `b` keeps its place on the undo stack.
///
/// A reopen offers the same undo and redo, and the undo steps that remain write no step over another.
#[test]
fn a_cut_runs_on_through_the_other_records_of_an_applied_filter() {
    for case in ["plain", "written after", "undone"] {
        let mut f = fixture();
        let top = |f: &Fixture| f.layer.undo_hint(Direction::Undo).map(|hint| hint.target);
        let predicate = f.predicate(letters(&[1]));
        let first = top(&f);
        assert!(completed(&f.write("w", letters(&[1]))));
        let below = top(&f);
        assert!(completed(&f.write("b", letters(&[2]))));
        let earlier = top(&f);
        let preview = f.preview(predicate);
        let job = f.start(APPLY, apply_args(preview));
        let out = output(Value::Null, &[(FILTERED, letters(&[7])), ("b", letters(&[8]))]);
        assert!(completed(&f.finish(job, out)));
        let mut after = None;
        match case {
            "written after" => {
                assert!(completed(&f.write("b", letters(&[3]))));
                after = top(&f);
            }
            "undone" => {
                assert!(committed(&f.step(Direction::Undo)));
                assert_eq!((f.record("b").1, f.record(FILTERED).1), (Some(letters(&[2])), None));
            }
            _ => {}
        }
        assert!(completed(&f.play(FILTERED, letters(&[9]))));
        assert!(f.layer.undo_hint(Direction::Redo).is_none(), "{case}");
        let g = reopen(&f.layer);
        for direction in [Direction::Undo, Direction::Redo] {
            assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{case} {direction:?}");
        }
        match case {
            "plain" => assert_eq!(top(&f), below),
            "written after" => {
                assert_eq!(top(&f), after);
                assert!(committed(&f.step(Direction::Undo)));
                assert_eq!(f.record("b").1, Some(letters(&[8])), "the apply's `b` is given back");
                assert_eq!(top(&f), below);
            }
            _ => {
                assert_eq!(top(&f), earlier);
                assert!(committed(&f.step(Direction::Undo)));
                assert_eq!(f.record("b").1, None);
                assert_eq!(top(&f), below);
            }
        }
        assert!(committed(&f.step(Direction::Undo)));
        assert_eq!(top(&f), first, "{case}: the predicate's completion is left");
        let b = if case == "undone" { None } else { Some(letters(&[8])) };
        assert_eq!((f.record("w").1, f.record("b").1, f.record(FILTERED).1), (None, b, Some(letters(&[9]))), "{case}");
    }
}

/// The cut runs on through the redo stack too. An apply that wrote `filtered` and `z` and a later write of `z` are
/// both undone. A write of `filtered` by a job of `recording.play`, whose undo class is `none`, then drops the apply
/// from the redo stack, and with it the later write of `z` beneath it: that write's redo would start from the apply's
/// `z`, which no redo brings back now. The undo stack keeps its place, and a reopen offers the same undo and redo.
#[test]
fn the_cut_runs_on_through_the_redo_stack() {
    let mut f = fixture();
    let top = |f: &Fixture| f.layer.undo_hint(Direction::Undo).map(|hint| hint.target);
    let predicate = f.predicate(letters(&[1]));
    let preview = f.preview(predicate);
    assert!(completed(&f.write("w", letters(&[1]))));
    let below = top(&f);
    let job = f.start(APPLY, apply_args(preview));
    assert!(completed(&f.finish(job, output(Value::Null, &[(FILTERED, letters(&[7])), ("z", letters(&[8]))]))));
    assert!(completed(&f.write("z", letters(&[3]))));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!((f.record("z").1, f.record(FILTERED).1), (None, None));
    assert!(f.layer.undo_hint(Direction::Redo).is_some());
    assert!(completed(&f.play(FILTERED, letters(&[9]))));
    assert!(f.layer.undo_hint(Direction::Redo).is_none(), "both undone actions leave the redo stack");
    assert_eq!(top(&f), below);
    let g = reopen(&f.layer);
    for direction in [Direction::Undo, Direction::Redo] {
        assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{direction:?}");
    }
}

/// CT-LG3-03 and CT-LB7-01 (part: an invoked workspace change of a stored row, `filter.apply`, is one undoable action
/// whose undo and redo each move the workspace revision once; a language-defined row's is in `tests/job_output.rs`).
/// The workspace inverse (H-08: `filter.apply` undoes in the workspace): an undo writes back what each record held
/// before the apply, absent ones included, and a redo writes the apply's records again; each moves every written
/// record's version and the workspace revision once. An earlier record write keeps its place on the stack beneath
/// the apply, above the predicate's completion, and a reopen replays them and offers the same undo and redo.
#[test]
fn an_applied_filter_undoes_and_redoes_in_the_workspace() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let first = f.layer.undo_hint(Direction::Undo).map(|hint| hint.target);
    assert!(completed(&f.write(FILTERED, letters(&[1]))));
    let preview = f.preview(predicate);
    let receipt = f.shell.invoke(&mut f.layer, APPLY, apply_args(preview.clone())).unwrap();
    let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
    f.layer.take_program_job().unwrap();
    let out = output(Value::Null, &[(FILTERED, letters(&[7])), ("count", letters(&[5]))]);
    assert!(completed(&f.finish(job, out)));
    assert_eq!((f.record(FILTERED), f.record("count")), ((2, Some(letters(&[7]))), (1, Some(letters(&[5])))));
    let workspace = f.layer.revision().workspace;

    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!((f.record(FILTERED), f.record("count")), ((3, Some(letters(&[1]))), (2, None)));
    assert_eq!(f.layer.revision().workspace, workspace + 1);
    let g = reopen(&f.layer);
    assert_eq!(g.records(), f.layer.records());
    for direction in [Direction::Undo, Direction::Redo] {
        assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{direction:?}");
    }

    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!((f.record(FILTERED), f.record("count")), ((4, Some(letters(&[7]))), (3, Some(letters(&[5])))));
    assert_eq!(f.layer.revision().workspace, workspace + 2);
    let g = reopen(&f.layer);
    assert_eq!(g.records(), f.layer.records());
    for direction in [Direction::Undo, Direction::Redo] {
        assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{direction:?}");
    }

    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!((f.record(FILTERED), f.record("count")), ((6, None), (4, None)));
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|hint| hint.target), first, "the predicate's completion");
}

/// The journal form (draft): an applied filter's workspace completion, its undo and its redo carry `records` and read
/// back to themselves; an undo's records may hold no value, an apply's may not. An undo or redo that carries
/// `records` and a `record` as well does not read.
#[test]
fn an_applied_filter_and_its_undo_journal_their_records() {
    let mut f = fixture();
    let predicate = f.predicate(letters(&[1]));
    let preview = f.preview(predicate);
    assert!(completed(&f.apply(&preview, letters(&[7]))));
    assert!(completed(&f.write("w", letters(&[3]))));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    use TransactionKind::{Record, Redo, Undo, Workspace};
    assert_eq!(kinds, [Workspace, Workspace, Record, Undo, Undo, Redo]);
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }
    let undo = &log[4];
    assert_eq!(undo.records.len(), 1);
    assert_eq!(undo.records[0].value, None);
    assert_eq!(log[5].records, log[1].records);

    let object_of = |transaction: &Transaction| match transaction.to_json() {
        Value::Object(fields) => fields,
        _ => panic!("a transaction is an object"),
    };
    let mut both = object_of(undo);
    both.insert("record".into(), log[2].to_json().get("record").unwrap().clone());
    assert_eq!(Transaction::from_json(&Value::Object(both)), None, "an undo with a record and records");
    let mut empty = object_of(&log[1]);
    empty.insert("records".into(), undo.to_json().get("records").unwrap().clone());
    assert_eq!(Transaction::from_json(&Value::Object(empty)), None, "a workspace completion with a write of no value");
}
