//! ADR-D01.9's replacement of a shown preview (owner, 22 November 2026, item 4; draft): a new preview may replace the
//! one the solver is shown, by a command the solver invokes; the replacement takes a new preview ID when it builds,
//! carries the mark of the change, and the replaced preview can no longer be committed. After a replacement the commit
//! key is ignored for a settle interval, so a key-down never commits a preview the solver has not seen. A preview a
//! run prepared waits out the interval too, since the solver did not ask for it. A recording never replaces a preview.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use c600_command::{
    code, draft_args, identity_json, word_draft, Caller, Clock, CommandLayer, Error, FrameChain, HeadlessFrames,
    Invocation, Kind, MemoryRecorder, MemoryStore, Overlay, PlayStatus, PreviewState, Provenance, Receipt, Recorded,
    RecordedStep, Recording, Secret, Shell, Status, Table, ONE_KEY_ROW, RECORDING_PLAY_ROW, RECORD_ROW, SETTLE_DRAFT,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
/// A tool's query row, which the solver invokes to run the tool.
const TOOL: &str = "tool.propose";
/// The row a recording's unattended step plays here: a view row on the view lane with no job flag, which a tool's entry
/// serves, so the step runs as a program job whose run is inside the playback (LP4-9).
const ROTATE: &str = "camera.rotate";

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    Arc::clone(KERNEL.get_or_init(|| {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
        Arc::new(Kernel::load(&assets).unwrap_or_else(|e| panic!("the model in assets/ does not load: {e}")))
    }))
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// A clock the test moves by hand, in milliseconds.
#[derive(Clone, Default)]
struct Manual(Arc<AtomicU64>);

impl Manual {
    fn advance(&self, ms: u64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }

    fn back(&self, ms: u64) {
        self.0.fetch_sub(ms, Ordering::SeqCst);
    }
}

impl Clock for Manual {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst))
    }
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

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    clock: Manual,
    sent: u64,
}

/// A session at the solved state that protects nothing, with the test reader, a tool row, a served `camera.rotate`, a
/// recorder holding `r1`, one completed `camera.rotate`, and the manual clock at zero.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    let overlay = Overlay { module: "tools/propose".into(), entry: "run".into() };
    layer.register_overlay(TOOL, Kind::Query, &[], overlay).unwrap();
    layer.serve_row(ROTATE, Overlay { module: "tools/camera".into(), entry: "rotate".into() }).unwrap();
    let recorder = MemoryRecorder::new();
    let recordings = [("r1".to_owned(), rotation())].into_iter().collect();
    recorder.put(Recorded { recordings, playbacks: Default::default() });
    layer.set_recorder(Box::new(recorder)).unwrap();
    let clock = Manual::default();
    layer.set_clock(Box::new(clock.clone()));
    let shell = Shell::new(Secret::fixed(SECRET), "shell");
    Fixture { layer, shell, frames: HeadlessFrames::new(), clock, sent: 0 }
}

impl Fixture {
    /// The run of a command the solver invoked: the shell invokes `row`, and the host takes the job and runs it.
    fn solver_run(&mut self, row: &str, args: Value) -> String {
        let receipt = self.shell.invoke(&mut self.layer, row, args).unwrap();
        let Status::Running(job) = receipt.status else { panic!("{row}: {:?}", receipt.status) };
        let program = self.layer.take_program_job().expect("the row's job");
        assert_eq!(program.job, job);
        program.run()
    }

    fn tool_run(&mut self) -> String {
        self.solver_run(TOOL, json::object([]))
    }

    /// The run of a playback's unattended step: the solver plays `r1`, the playback dispatches its one step, a served
    /// view row, and the host takes the step's job. Returns the step's run.
    fn playback_run(&mut self) -> String {
        let args = json::object([("recording", text("r1"))]);
        let receipt = self.shell.invoke(&mut self.layer, RECORDING_PLAY_ROW, args).unwrap();
        assert!(matches!(receipt.status, Status::Running(_)), "the play: {:?}", receipt.status);
        self.layer.run_jobs();
        let program = self.layer.take_program_job().expect("the step's job");
        assert_eq!(program.row, ROTATE);
        program.run()
    }

    /// One envelope on run `run`'s channel, with no token.
    fn runtime(&mut self, run: &str, row: &str, args: Value) -> Receipt {
        self.sent += 1;
        let served = self.layer.table().row(row).is_some_and(|found| found.overlay.is_some());
        let invocation = Invocation {
            command: format!("{run}/{}", self.sent),
            row: row.to_owned(),
            args,
            token: None,
            epoch: self.layer.revision().epoch,
            budget: served.then_some(c600_command::Budget::CEILING),
        };
        self.layer.invoke(&Caller::Runtime { run: run.to_owned() }, invocation)
    }

    /// Run `run` prepares a preview of `moves`, which builds; returns its ID.
    fn run_preview(&mut self, run: &str, moves: &[i64]) -> u64 {
        let receipt = self.runtime(run, "operation.preview", draft_args(&word(moves)));
        let Status::Running(id) = receipt.status else { panic!("the run's preview: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        id
    }

    /// The solver previews `moves` from the editor's draft with the preview key; the preview builds, and no frame shows
    /// it yet. Returns its ID.
    fn solver_preview(&mut self, moves: &[i64]) -> u64 {
        self.shell.draft = Some(word_draft(&word(moves)));
        let receipt = self.shell.press(&mut self.layer, "Ctrl+Enter", false).unwrap().unwrap();
        let Status::Running(id) = receipt.status else { panic!("not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        id
    }

    /// The display's next frame, which shows the pending preview `id`.
    fn show(&mut self, id: u64) {
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        assert_eq!(self.layer.preview_state(id), Some(PreviewState::Presented));
    }

    /// A frame that shows no preview, though `id` is pending (another view, a transition).
    fn blank(&mut self, id: u64) {
        let blank = self.frames.other(None, FrameChain::BuiltIn);
        assert!(self.shell.present(&mut self.layer, id, &blank).is_err(), "the frame shows no preview");
    }

    /// The solver's commit chord, pressed for the preview the shell holds as presented.
    fn chord(&mut self) -> Receipt {
        self.shell.press(&mut self.layer, "Ctrl+Shift+Enter", false).unwrap().unwrap()
    }

    fn execute(&mut self, id: u64) -> Receipt {
        let args = json::object([("preview", Value::Int(id as i64))]);
        self.shell.invoke(&mut self.layer, "operation.execute", args).unwrap()
    }

    fn transactions(&self) -> usize {
        self.layer.store().log.len()
    }
}

fn code_of(receipt: &Receipt) -> Option<&'static str> {
    receipt.status.code()
}

/// `r1`: one `camera.rotate` that completed when it was recorded, an unattended step under EV4.22's table.
fn rotation() -> Recording {
    let completed = json::object([
        ("command", text("shell/1")),
        ("request", text("r")),
        ("job", Value::Int(1)),
        ("status", json::object([("name", text("completed")), ("kind", text("view_published"))])),
    ]);
    let step = RecordedStep {
        row: ROTATE.into(),
        arguments: json::object([("rotation", text("quarter")), ("view", text("main"))]),
        word: None,
        depends: Vec::new(),
        module: None,
        action: None,
        command: "shell/1".into(),
        request: "r".into(),
        outcome: Some(completed),
    };
    let identity = identity_json(&kernel().identity());
    Recording { name: "r1".into(), session: "s1".into(), identity, started: Value::Null, steps: vec![step] }
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

/// The state digest after `moves` from the solved state.
fn end_of(moves: &[i64]) -> String {
    let k = kernel();
    let recipe = k.recipe(&word(moves)).unwrap();
    match k.derive(&k.solved(), &recipe, &k.protected_set(&[]).unwrap()).unwrap() {
        c600_kernel::Derivation::Derived(d) => d.state.digest().to_owned(),
        c600_kernel::Derivation::Refused(_) => panic!("the word derives"),
    }
}

/// The run of a command the solver invoked replaces the preview the solver is shown. The replacement takes a new ID
/// when it builds, names the preview it replaced in its summary, and the replaced preview ends: the chord, pressed for
/// it, commits nothing, and a commit that names it is `preview_replaced`.
#[test]
fn a_run_replaces_the_shown_preview_with_a_new_id_and_the_mark() {
    let mut f = fixture();
    let solved = f.layer.revision().digest.clone();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    assert_eq!(f.layer.preview().unwrap().replaced, None, "the first preview replaced nothing");
    assert!(f.layer.preview().unwrap().summary().get("replaced").is_none());

    let theirs = f.run_preview(&run, &[2]);
    assert_ne!(theirs, mine);
    let pending = f.layer.preview().unwrap();
    assert_eq!((pending.id, pending.state, pending.replaced), (theirs, PreviewState::Ready, Some(mine)));
    assert_eq!(pending.summary().get("replaced"), Some(&Value::Int(mine as i64)), "the preview view's mark");
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Replaced));

    let journal = f.transactions();
    let press = f.chord();
    assert!(!committed(&press), "the chord for the replaced preview: {:?}", press.status);
    let named = f.execute(mine);
    assert_eq!(code_of(&named), Some(code::PREVIEW_REPLACED), "{:?}", named.status);
    assert_eq!(f.transactions(), journal);
    assert_eq!(f.layer.revision().digest, solved, "nothing was committed");
}

/// After the replacement is shown, the commit key is ignored for the settle interval: a press on the first frame, or
/// any press before the interval has passed, is `busy.preview_settling`, journals nothing and leaves the preview
/// pending and presented. The first press after the interval commits the replacement. The key stays available
/// throughout, since the interval ends with the clock and not with an action; `settle_remaining` says how long is left.
#[test]
fn the_commit_key_is_ignored_for_the_settle_interval() {
    let mut f = fixture();
    assert_eq!(f.layer.settle_interval(), SETTLE_DRAFT);
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    assert_eq!(f.layer.settle_remaining(), None, "a preview that replaced nothing does not wait");
    let theirs = f.run_preview(&run, &[2]);
    assert_eq!(f.layer.settle_remaining(), Some(SETTLE_DRAFT), "no frame shows the replacement yet");
    f.show(theirs);

    let journal = f.transactions();
    for (wait, left) in [(0, 250), (100, 150), (149, 1)] {
        f.clock.advance(wait);
        assert_eq!(f.layer.settle_remaining(), Some(Duration::from_millis(left)));
        assert!(f.layer.availability("operation.execute").is_ok(), "the key stays available");
        let press = f.chord();
        assert_eq!(code_of(&press), Some(code::PREVIEW_SETTLING), "{left} ms left: {:?}", press.status);
        assert!(matches!(press.status, Status::Refused(_)));
        assert_eq!(f.transactions(), journal, "an ignored press journals nothing");
        assert_eq!(f.layer.preview_state(theirs), Some(PreviewState::Presented));
    }
    f.clock.advance(1);
    assert_eq!(f.layer.settle_remaining(), Some(Duration::ZERO));
    assert!(committed(&f.chord()), "the first press after the interval commits");
    assert_eq!(f.layer.revision().digest, end_of(&[2]));
    assert_eq!(f.layer.preview_state(theirs), Some(PreviewState::Committed));
}

/// The interval counts from the first frame that shows the replacement, not from its build: time that passes while no
/// frame shows it does not count, and a presentation that ends and starts again, or moves to a new device epoch,
/// waits the whole interval again.
#[test]
fn the_interval_counts_from_the_frame_that_shows_the_replacement() {
    let mut f = fixture();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let theirs = f.run_preview(&run, &[2]);
    f.clock.advance(10_000);
    f.show(theirs);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "built long ago, shown now");

    f.clock.advance(300);
    let blank = f.frames.other(None, FrameChain::BuiltIn);
    assert!(f.shell.present(&mut f.layer, theirs, &blank).is_err(), "the frame shows no preview");
    assert_eq!(f.layer.preview_state(theirs), Some(PreviewState::Ready));
    f.show(theirs);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "shown again: the interval starts again");

    f.clock.advance(300);
    f.frames.recover();
    f.show(theirs);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "a new device epoch starts a new presentation");

    f.clock.advance(100);
    f.show(theirs);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "a later frame of the same run does not restart it");
    f.clock.advance(150);
    f.show(theirs);
    assert!(committed(&f.chord()), "250 ms after the presentation started");
}

/// A replacement of a replacement the solver never saw keeps the mark of the preview they did see, so the settle
/// interval still applies; a preview the solver prepares that replaces only previews nobody was shown carries no mark
/// and commits on its first presented press, as before ADR-D01.9.
#[test]
fn the_mark_names_the_preview_the_solver_last_saw() {
    let mut f = fixture();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let first = f.run_preview(&run, &[2]);
    let second = f.run_preview(&run, &[3]);
    assert_eq!(f.layer.preview_state(first), Some(PreviewState::Replaced));
    assert_eq!(f.layer.preview().map(|p| (p.id, p.replaced)), Some((second, Some(mine))));
    f.show(second);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING));

    let mut g = fixture();
    let unseen = g.solver_preview(&[1]);
    let again = g.solver_preview(&[2]);
    assert_eq!(g.layer.preview_state(unseen), Some(PreviewState::Replaced));
    assert_eq!(g.layer.preview().map(|p| (p.id, p.replaced)), Some((again, None)));
    g.show(again);
    assert_eq!(g.layer.settle_remaining(), None);
    assert!(committed(&g.chord()), "no shown preview was replaced, so nothing waits");
}

/// The settle interval applies to every replacement of a shown preview, the solver's own preview key included: the
/// key-down that commits may come before the solver has seen the new preview.
#[test]
fn the_solvers_own_replacement_also_settles() {
    let mut f = fixture();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let next = f.solver_preview(&[2]);
    assert_eq!(f.layer.preview().map(|p| p.replaced), Some(Some(mine)));
    f.show(next);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING));
    f.clock.advance(SETTLE_DRAFT.as_millis() as u64);
    assert!(committed(&f.chord()));
    assert_eq!(f.layer.revision().digest, end_of(&[2]));
}

/// The interval is the host's to set from the tuned token data: a zero interval waits for nothing beyond the
/// presentation, and a longer one is kept.
#[test]
fn the_host_sets_the_interval() {
    let mut f = fixture();
    f.layer.set_settle(Duration::from_millis(400));
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let next = f.solver_preview(&[2]);
    f.show(next);
    f.clock.advance(399);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING));
    f.layer.set_settle(Duration::ZERO);
    assert!(committed(&f.chord()), "a zero interval");
}

/// A recording never replaces a preview: a run inside a `recording.play` job, the run of a step the playback dispatched,
/// is refused with `not_granted` while a preview is pending, shown or not, and the pending preview stays. Once the
/// step is reported and the playback ends, a tool's run
/// replaces the shown preview as before. With no preview pending, the recording's run prepares one, which replaced
/// nothing. (A run that the recording's run invoked is inside the recording too:
/// `the_mark_passes_on_and_a_nested_run_is_inside_the_recording`.)
#[test]
fn a_recording_never_replaces_a_preview() {
    let mut f = fixture();
    let playback = f.playback_run();
    let mine = f.solver_preview(&[1]);
    let refused = f.runtime(&playback, "operation.preview", draft_args(&word(&[2])));
    assert_eq!(code_of(&refused), Some(code::NOT_GRANTED), "{:?}", refused.status);
    assert_eq!(f.layer.preview().map(|p| p.id), Some(mine), "the ready preview stays");
    f.show(mine);
    let refused = f.runtime(&playback, "operation.preview", draft_args(&word(&[2])));
    assert_eq!(code_of(&refused), Some(code::NOT_GRANTED), "{:?}", refused.status);
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Presented), "the shown preview stays");

    // The step is reported and the playback ends; a tool's run replaces the shown preview.
    let job = playback.strip_prefix("job-").and_then(|n| n.parse().ok()).expect("a job's run");
    f.layer.finish_program_job(job, Ok(Value::Null)).expect("the step is reported");
    f.layer.run_jobs();
    assert_eq!(f.layer.playback("r1").map(|state| state.status.clone()), Some(PlayStatus::Finished));
    let tool = f.tool_run();
    let theirs = f.run_preview(&tool, &[2]);
    assert_eq!(f.layer.preview().map(|p| p.replaced), Some(Some(mine)));
    f.show(theirs);
    f.clock.advance(SETTLE_DRAFT.as_millis() as u64);
    assert!(committed(&f.chord()));

    let mut g = fixture();
    let playback = g.playback_run();
    let played = g.run_preview(&playback, &[3]);
    assert_eq!(g.layer.preview().map(|p| (p.id, p.replaced)), Some((played, None)));
}

/// A recording does not end the solver's preview to prepare its own in the empty slot: a run inside a
/// `recording.play` job that cancels a preview it did not ask for is refused with `not_granted`, and the preview stays.
/// It may still cancel the preview its own run prepared.
#[test]
fn a_recording_never_ends_the_solvers_preview() {
    let mut f = fixture();
    let playback = f.playback_run();
    let mine = f.solver_preview(&[1]);
    let cancel = json::object([("preview", Value::Int(mine as i64))]);
    let refused = f.runtime(&playback, "operation.preview.cancel", cancel);
    assert_eq!(code_of(&refused), Some(code::NOT_GRANTED), "{:?}", refused.status);
    assert_eq!(f.layer.preview().map(|p| (p.id, p.state)), Some((mine, PreviewState::Ready)), "the preview stays");
    let again = f.runtime(&playback, "operation.preview", draft_args(&word(&[2])));
    assert_eq!(code_of(&again), Some(code::NOT_GRANTED), "{:?}", again.status);

    let mut g = fixture();
    let playback = g.playback_run();
    let played = g.run_preview(&playback, &[3]);
    let cancel = json::object([("preview", Value::Int(played as i64))]);
    let own = g.runtime(&playback, "operation.preview.cancel", cancel);
    assert!(matches!(own.status, Status::Completed(_)), "its own preview: {:?}", own.status);
    assert_eq!(g.layer.preview_state(played), Some(PreviewState::Cancelled));
}

/// The mark names a preview the solver was shown even when that presentation has lapsed before the replacement (a
/// blank frame): the replacement still carries the mark and waits out the settle interval.
#[test]
fn a_lapsed_presentation_still_marks_the_replacement() {
    let mut f = fixture();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let blank = f.frames.other(None, FrameChain::BuiltIn);
    assert!(f.shell.present(&mut f.layer, mine, &blank).is_err(), "the frame shows no preview");
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Ready), "the presentation lapsed");

    let theirs = f.run_preview(&run, &[2]);
    assert_eq!(f.layer.preview().map(|p| (p.id, p.replaced)), Some((theirs, Some(mine))));
    f.show(theirs);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING));
    f.clock.advance(SETTLE_DRAFT.as_millis() as u64);
    assert!(committed(&f.chord()));
    assert_eq!(f.layer.revision().digest, end_of(&[2]));
}

/// ADR-D01.12's one-key press commits the solver's own macro while a replacement waits out its interval: it is the
/// solver's own press on their own record, not the commit key on a preview they may not have seen.
#[test]
fn the_one_key_press_does_not_wait_for_a_replacement() {
    let mut f = fixture();
    let letters = Value::Array(vec![Value::Int(3)]);
    let args = json::object([("payload", letters), ("provenance", text("block")), ("record", text("own"))]);
    let receipt = f.shell.invoke(&mut f.layer, RECORD_ROW, args).unwrap();
    assert!(matches!(receipt.status, Status::Completed(_)), "{:?}", receipt.status);

    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let theirs = f.run_preview(&run, &[2]);
    f.show(theirs);
    let press =
        f.shell.invoke(&mut f.layer, ONE_KEY_ROW, json::object([("draft", json::object([("record", text("own"))]))]));
    assert!(committed(&press.unwrap()), "the one-key press");
    assert_eq!(f.layer.revision().digest, end_of(&[3]));
    assert_eq!(f.layer.preview_state(theirs), Some(PreviewState::Replaced));
}

/// A run that replaced the shown preview and then cancels its own replacement (a run may cancel a preview no frame
/// shows) leaves the slot empty and the shown preview replaced: a run can end the shown preview this way, though a
/// direct cancel of it is refused (the reading of row 5). The next preview still carries the mark of the preview the
/// solver last saw and waits out the settle interval.
#[test]
fn a_run_that_cancels_its_replacement_keeps_the_mark() {
    let mut f = fixture();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let cancel = |id: u64| json::object([("preview", Value::Int(id as i64))]);
    let direct = f.runtime(&run, "operation.preview.cancel", cancel(mine));
    assert_eq!(code_of(&direct), Some(code::PREVIEW_SHOWN), "the direct cancel is refused");
    let first = f.run_preview(&run, &[2]);
    assert_eq!(f.layer.preview().map(|p| p.replaced), Some(Some(mine)));
    let own = f.runtime(&run, "operation.preview.cancel", cancel(first));
    assert!(matches!(own.status, Status::Completed(_)), "its own unshown preview: {:?}", own.status);
    assert!(f.layer.preview().is_none(), "nothing is pending");
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Replaced));

    let second = f.run_preview(&run, &[3]);
    assert_eq!(f.layer.preview().map(|p| (p.id, p.replaced)), Some((second, Some(mine))), "the mark is kept");
    f.show(second);
    let journal = f.transactions();
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING));
    assert_eq!(f.transactions(), journal);
    f.clock.advance(SETTLE_DRAFT.as_millis() as u64);
    assert!(committed(&f.chord()));
    assert_eq!(f.layer.revision().digest, end_of(&[3]));
}

/// Every way a shown preview's presentation lapses keeps the mark on its replacement: a blank frame (above), a lost
/// presentation surface (`presentation_unavailable`) and a frame drawn through a user stage, which the solver sees.
#[test]
fn every_lapse_of_the_presentation_keeps_the_mark() {
    for lapse in ["unavailable", "user stage"] {
        let mut f = fixture();
        let run = f.tool_run();
        let mine = f.solver_preview(&[1]);
        f.show(mine);
        if lapse == "unavailable" {
            f.shell.presentation_unavailable(&mut f.layer);
        } else {
            let pending = f.layer.preview().unwrap().clone();
            let staged = f.frames.other(Some(&pending), FrameChain::UserStage);
            assert!(f.shell.present(&mut f.layer, mine, &staged).is_err(), "{lapse}");
        }
        let theirs = f.run_preview(&run, &[2]);
        assert_eq!(f.layer.preview().map(|p| p.replaced), Some(Some(mine)), "{lapse}");
        if lapse == "unavailable" {
            f.frames.recover();
        }
        f.show(theirs);
        assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "{lapse}");
    }
}

/// A preview a run prepared always waits out the settle interval once a frame shows it, whether it replaced a preview
/// nobody saw or filled an empty slot, a recording's included: the solver did not ask for it, so a key-down may come
/// before they have seen it (the reading of row 5). A preview the solver prepares into an empty slot does not wait.
#[test]
fn a_preview_a_run_prepared_always_settles() {
    let mut f = fixture();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    let theirs = f.run_preview(&run, &[2]);
    assert_eq!(f.layer.preview_state(mine), Some(PreviewState::Replaced));
    assert_eq!(f.layer.preview().map(|p| p.replaced), Some(None), "nobody saw the replaced preview");
    assert_eq!(f.layer.settle_remaining(), Some(SETTLE_DRAFT));
    f.show(theirs);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "a run's replacement of an unshown preview");
    f.clock.advance(SETTLE_DRAFT.as_millis() as u64);
    assert!(committed(&f.chord()));
    assert_eq!(f.layer.revision().digest, end_of(&[2]));

    let mut g = fixture();
    let playback = g.playback_run();
    let played = g.run_preview(&playback, &[3]);
    g.show(played);
    assert_eq!(code_of(&g.chord()), Some(code::PREVIEW_SETTLING), "a recording's preview in an empty slot");
    g.clock.advance(SETTLE_DRAFT.as_millis() as u64);
    assert!(committed(&g.chord()));

    let mut h = fixture();
    let own = h.solver_preview(&[1]);
    assert_eq!(h.layer.settle_remaining(), None);
    h.show(own);
    assert!(committed(&h.chord()), "the solver's own preview in an empty slot");
}

/// Guard (attack on row 5): a commit whose token was minted for the replaced preview commits nothing, whether it names
/// the replaced preview or its envelope is edited to name the replacement after the interval.
#[test]
fn a_commit_minted_before_the_replacement_commits_nothing() {
    let mut f = fixture();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let args = json::object([("preview", Value::Int(mine as i64))]);
    let stale = f.shell.prepare(&f.layer, "operation.execute", args.clone()).unwrap();
    let theirs = f.run_preview(&run, &[2]);
    let journal = f.transactions();
    assert_eq!(code_of(&f.shell.send(&mut f.layer, stale)), Some(code::PREVIEW_REPLACED));
    let mut edited = f.shell.prepare(&f.layer, "operation.execute", args).unwrap();
    f.show(theirs);
    f.clock.advance(1_000);
    edited.args = json::object([("preview", Value::Int(theirs as i64))]);
    let receipt = f.shell.send(&mut f.layer, edited);
    assert!(!committed(&receipt), "{:?}", receipt.status);
    assert_eq!(f.transactions(), journal);
}

/// Guard (attack on row 5): a clock that goes back, a later clock swapped in and a frame on a new device epoch never
/// shorten the wait.
#[test]
fn clock_and_surface_changes_never_shorten_the_wait() {
    let mut f = fixture();
    f.clock.advance(1_000_000);
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let theirs = f.run_preview(&run, &[2]);
    f.show(theirs);
    f.clock.back(500_000);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "a clock that went back");
    f.clock.advance(500_000 + 240);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING));
    let other = Manual::default();
    other.advance(9_000_000);
    f.layer.set_clock(Box::new(other.clone()));
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "a later clock swapped in");
    other.advance(249);
    f.frames.recover();
    f.show(theirs);
    other.advance(249);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING), "a new device epoch waits again");
    other.advance(1);
    assert!(committed(&f.chord()));
}

/// Guard (attack on row 5): a marked replacement that was shown and then hidden marks its own replacement; and a run
/// that a recording's run invoked is inside the recording, so it cannot replace the solver's preview either.
#[test]
fn the_mark_passes_on_and_a_nested_run_is_inside_the_recording() {
    let mut f = fixture();
    let run = f.tool_run();
    let mine = f.solver_preview(&[1]);
    f.show(mine);
    let second = f.run_preview(&run, &[2]);
    f.show(second);
    f.clock.advance(1_000);
    f.blank(second);
    let third = f.run_preview(&run, &[3]);
    assert_eq!(f.layer.preview().map(|p| p.replaced), Some(Some(second)));
    f.show(third);
    assert_eq!(code_of(&f.chord()), Some(code::PREVIEW_SETTLING));

    let mut g = fixture();
    g.layer.set_program_capacity(2);
    let playback = g.playback_run();
    let nested = g.runtime(&playback, TOOL, json::object([]));
    let Status::Running(_) = nested.status else { panic!("the nested tool: {:?}", nested.status) };
    let inner = g.layer.take_program_job().expect("the nested job").run();
    let mine = g.solver_preview(&[1]);
    let refused = g.runtime(&inner, "operation.preview", draft_args(&word(&[2])));
    assert_eq!(code_of(&refused), Some(code::NOT_GRANTED), "{:?}", refused.status);
    assert_eq!(g.layer.preview().map(|p| p.id), Some(mine));
}
