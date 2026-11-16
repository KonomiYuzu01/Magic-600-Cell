//! The command layer against the model in `assets/`: the commit steps, undo and redo, tokens, lanes, restart and
//! read-only recovery. Each test names the conformance test it covers headless. `tests/test_lp3_loop.py` at the
//! repository root replays the oracle cases through the layer in a release build.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::token::args_digest;
use c600_command::{
    code, draft_args, word_draft, ActionHint, Caller, CancelReason, CommandLayer, Completion, CompletionKind,
    Direction, Error, Fate, Invocation, Kind, Lane, MemoryStore, Overlay, PreviewRule, PreviewState, Receipt, Secret,
    Shell, Status, Table, Token, TokenBinding,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{archive, Derivation, Kernel, ProtectedSet, Snapshot, Verdict};

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

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}

fn word(letters: &[i64]) -> Value {
    parse(&format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#))
}

fn case(name: &str) -> Value {
    let path = repository().join("work/experiments/language-lx/inputs/oracle").join(format!("{name}.json.gz"));
    let bytes = std::fs::read(path).expect("the frozen case file is readable");
    json::parse(&archive::gunzip(&bytes, 1 << 30).unwrap()).unwrap()
}

/// O06's recipes under its protected set `[1]`: those the oracle accepts and those it refuses as protected.
fn o06() -> (Vec<Value>, Vec<Value>) {
    let (mut clear, mut moves) = (Vec::new(), Vec::new());
    for step in case("O06").get("steps").and_then(Value::as_array).unwrap() {
        let recipe = step.get("recipe").unwrap().clone();
        match step.get("expect").and_then(|e| e.get("accepted")) {
            Some(Value::Bool(true)) => clear.push(recipe),
            _ => moves.push(recipe),
        }
    }
    (clear, moves)
}

fn derived(base: &Snapshot, recipe: &Value) -> Snapshot {
    let k = kernel();
    match k.derive(base, &k.recipe(recipe).unwrap(), &ProtectedSet::none()).unwrap() {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("unexpected refusal: {}", refused.refusal),
    }
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/full".into(), entry: entry.into() }
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
}

fn fixture(protected: &[i64]) -> Fixture {
    let k = kernel();
    let protected = k.protected_set(protected).unwrap();
    let start = k.solved();
    let layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell") }
}

impl Fixture {
    /// Previews `recipe` through the command route and runs the job; returns the preview's ID.
    fn preview(&mut self, recipe: &Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(job) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        let done = self.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert!(
            matches!(done.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. })),
            "{:?}",
            done.status
        );
        job
    }

    fn present(&mut self, id: u64) {
        self.shell.present(&mut self.layer, id).unwrap();
    }

    /// The commit chord through the command route, bound to the presented preview.
    fn execute(&mut self) -> Receipt {
        let id = self.shell.presented().expect("a presented preview").id;
        self.shell.invoke(&mut self.layer, "operation.execute", preview_args(id)).unwrap()
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let id = self.preview(recipe);
        self.present(id);
        self.execute()
    }

    fn undo(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    /// What no refusal may change: the state digest, the state revision, the head and the durable log.
    fn fixed(&self) -> (String, u64, String, usize) {
        let r = self.layer.revision();
        (r.digest.clone(), r.state, r.head.clone(), self.layer.store().log.len())
    }
}

#[test]
fn the_stored_table_reads() {
    let table = Table::h08();
    assert_eq!(table.rows().count(), 174);
    let execute = table.row("operation.execute").unwrap();
    assert_eq!((execute.kind, execute.lane, execute.preview), (Kind::Commit, Lane::Commit, PreviewRule::Required));
    assert!(execute.has_gate("fresh_preview") && execute.has_gate("protection") && execute.takes_token());
    let preview = table.row("operation.preview").unwrap();
    assert_eq!((preview.kind, preview.lane, preview.job), (Kind::Query, Lane::Exclusive, true));
    assert_eq!(table.row("session.undo").unwrap().action_guard.as_deref(), Some("action_hint"));
    assert!(table.services().iter().any(|s| s == "kernel.undo_hint"));
    assert_eq!(table.rows().filter(|row| row.kind == Kind::Commit).count(), 14);
}

/// CT-KN5-07.
#[test]
fn overlay_rows_are_never_commit_or_app_rows() {
    let mut table = Table::h08();
    for kind in [Kind::Commit, Kind::App] {
        let error = table.register_overlay("tool.x", kind, &[], overlay("x")).unwrap_err();
        assert_eq!(error.code, code::OVERLAY_ROW_KIND);
    }
    for taken in ["operation.execute", "kernel.undo_hint"] {
        let error = table.register_overlay(taken, Kind::Query, &[], overlay("x")).unwrap_err();
        assert_eq!(error.code, code::OVERLAY_ROW_CONFLICT);
    }
    let error = table.register_overlay("tool.t", Kind::Query, &[("t", "UserActionToken")], overlay("t")).unwrap_err();
    assert_eq!(error.code, code::OVERLAY_ROW_KIND);
    let row = table.register_overlay("tool.q", Kind::Query, &[("word", "WordRef")], overlay("q")).unwrap();
    assert_eq!((row.lane, row.takes_token(), row.job), (Lane::Program, false, true));
    assert_eq!(table.register_overlay("tool.v", Kind::View, &[], overlay("v")).unwrap().lane, Lane::View);
}

#[test]
fn a_word_commits_undoes_and_redoes() {
    let mut f = fixture(&[]);
    let start = f.layer.snapshot().clone();
    let recipe = word(&[1, 2, -1]);
    let end = derived(&start, &recipe);
    let id = f.preview(&recipe);
    f.present(id);
    let receipt = f.execute();
    let Status::Committed(record) = &receipt.status else { panic!("{:?}", receipt.status) };
    assert_eq!((record.pre.as_str(), record.post.as_str()), (start.digest(), end.digest()));
    assert_eq!((record.revision.state, record.revision.head.as_str()), (1, record.event.as_str()));
    assert_eq!(f.layer.snapshot().digest(), end.digest());
    assert_eq!(f.layer.preview_state(id), Some(PreviewState::Committed));
    assert_eq!(f.layer.undo_hint(Direction::Undo), Some(ActionHint { order: 1, target: 1 }));

    let undo = f.undo(Direction::Undo);
    let Status::Committed(back) = &undo.status else { panic!("{:?}", undo.status) };
    assert_eq!((back.pre.as_str(), back.post.as_str()), (end.digest(), start.digest()));
    // An equal digest after an undo is a new state revision, and the head is the journal's new head.
    assert_eq!((back.revision.state, back.revision.head.as_str()), (2, "e0"));
    assert_ne!(back.event, record.event);
    assert_eq!(f.layer.undo_hint(Direction::Undo), None);
    assert_eq!(f.layer.undo_hint(Direction::Redo), Some(ActionHint { order: 2, target: 1 }));

    let redo = f.undo(Direction::Redo);
    let Status::Committed(again) = &redo.status else { panic!("{:?}", redo.status) };
    assert_eq!((again.post.as_str(), again.revision.state), (end.digest(), 3));
    assert_eq!(f.layer.undo_hint(Direction::Undo), Some(ActionHint { order: 3, target: 3 }));
    assert_eq!(f.layer.store().log.len(), 3);
}

/// CT-ONE-03.
#[test]
fn a_commit_needs_its_token_and_a_presented_preview() {
    let mut f = fixture(&[]);
    let before = f.fixed();
    let error = f.shell.invoke(&mut f.layer, "operation.execute", preview_args(1)).unwrap_err();
    assert_eq!(error.code, code::NO_FRESH_PREVIEW);
    let id = f.preview(&word(&[1]));

    // A token naming a ready preview that was never presented.
    let p = f.layer.preview().unwrap().clone();
    let binding = TokenBinding {
        nonce: 900,
        session: "s1".into(),
        epoch: 1,
        command: "t/1".into(),
        row: "operation.execute".into(),
        args: args_digest(&preview_args(id)),
        identity: kernel().identity().model_id,
        preview: Some(c600_command::PreviewBinding { id, literal: p.literal.clone(), deps: p.deps.clone() }),
        revision: None,
        hint: None,
    };
    let token = Some(Token::mint(&Secret::fixed(SECRET), binding));
    let unpresented = Invocation {
        command: "t/1".into(),
        row: "operation.execute".into(),
        args: preview_args(id),
        token,
        caller: Caller::Shell,
        epoch: 1,
    };
    assert_eq!(code_of(&f.layer.invoke(unpresented)), code::NO_FRESH_PREVIEW);
    let no_token = Invocation {
        command: "t/2".into(),
        row: "operation.execute".into(),
        args: preview_args(id),
        token: None,
        caller: Caller::Shell,
        epoch: 1,
    };
    assert_eq!(code_of(&f.layer.invoke(no_token)), code::TOKEN_INVALID);

    // A token bound to other arguments.
    f.present(id);
    let mut other = f.shell.prepare(&f.layer, "operation.execute", preview_args(id)).unwrap();
    other.args = preview_args(id + 7);
    assert_eq!(code_of(&f.layer.invoke(other)), code::TOKEN_MISMATCH);
    assert_eq!(f.fixed(), before);
    assert!(matches!(f.execute().status, Status::Committed(_)));
}

/// CT-KN5-01 and CT-ONE-04.
#[test]
fn a_preview_replaced_after_its_presentation_is_not_committed() {
    let mut f = fixture(&[]);
    let a = f.preview(&word(&[1]));
    f.present(a);
    let b = f.preview(&word(&[2]));
    assert_eq!(f.layer.preview_state(a), Some(PreviewState::Replaced));
    assert_eq!(f.shell.presented().unwrap().id, a);
    let before = f.fixed();
    assert_eq!(code_of(&f.execute()), code::PREVIEW_REPLACED);
    assert_eq!(f.fixed(), before);
    f.present(b);
    assert!(matches!(f.execute().status, Status::Committed(_)));
}

/// CT-ONE-05.
#[test]
fn a_token_from_before_a_commit_and_its_undo_is_refused() {
    let mut f = fixture(&[]);
    let start = f.layer.snapshot().digest().to_owned();
    let p = f.preview(&word(&[1]));
    f.present(p);
    let early = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap();
    assert!(matches!(f.execute().status, Status::Committed(_)));
    assert!(matches!(f.undo(Direction::Undo).status, Status::Committed(_)));
    let q = f.preview(&word(&[1]));
    f.present(q);
    let fresh = f.layer.preview().unwrap().clone();
    assert_eq!((fresh.base.digest(), fresh.deps.state), (start.as_str(), 2));
    let before = f.fixed();
    // The early press named the committed preview, which is no longer pending.
    let old = early.token.clone().unwrap().binding;
    assert_eq!(code_of(&f.shell.send(&mut f.layer, early)), code::NO_FRESH_PREVIEW);
    // Even a token naming the new preview, if it carries the dependencies from before the commit, is stale at step 6:
    // the digest is equal, the revision is not.
    let mut binding = old.clone();
    binding.nonce = 901;
    binding.command = "t/revisited".into();
    binding.args = args_digest(&preview_args(q));
    binding.preview =
        Some(c600_command::PreviewBinding { id: q, literal: fresh.literal.clone(), deps: old.preview.unwrap().deps });
    let revisited = Invocation {
        command: "t/revisited".into(),
        row: "operation.execute".into(),
        args: preview_args(q),
        token: Some(Token::mint(&Secret::fixed(SECRET), binding)),
        caller: Caller::Shell,
        epoch: 1,
    };
    assert_eq!(code_of(&f.layer.invoke(revisited)), code::REVISION_CHANGED);
    assert_eq!(f.fixed(), before);
    assert!(matches!(f.execute().status, Status::Committed(_)));
}

/// CT-KN5-03 (R-18).
#[test]
fn a_held_commit_chord_commits_once() {
    let mut f = fixture(&[]);
    f.shell.draft = Some(word_draft(&word(&[1])));
    let preview = f.shell.press(&mut f.layer, "Ctrl+Enter", false).unwrap().unwrap();
    f.layer.run_jobs();
    f.present(preview.job.unwrap());
    let first = f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", false).unwrap().unwrap();
    assert!(matches!(first.status, Status::Committed(_)));
    for _ in 0..5 {
        assert_eq!(f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", true).unwrap(), None);
    }
    assert_eq!(f.layer.store().log.len(), 1);
    let error = f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", false).unwrap_err();
    assert_eq!(error.code, code::NO_FRESH_PREVIEW);
}

/// CT-KN5-04.
#[test]
fn a_retried_delivery_commits_at_most_once() {
    let mut f = fixture(&[]);
    let p = f.preview(&word(&[1]));
    f.present(p);
    let invocation = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap();
    let first = f.layer.invoke(invocation.clone());
    assert!(matches!(first.status, Status::Committed(_)));
    assert_eq!(f.layer.invoke(invocation.clone()), first);
    assert_eq!(f.layer.store().log.len(), 1);
    let mut other = invocation;
    other.args = preview_args(p + 1);
    assert_eq!(code_of(&f.layer.invoke(other)), code::COMMAND_ID_REUSED);
}

/// CT-KN5-06 (R-10).
#[test]
fn a_draft_changed_after_the_press_changes_nothing_committed() {
    let mut f = fixture(&[]);
    let start = f.layer.snapshot().clone();
    f.shell.draft = Some(word_draft(&word(&[1])));
    let preview = f.shell.press(&mut f.layer, "Ctrl+Enter", false).unwrap().unwrap();
    f.shell.draft = Some(word_draft(&word(&[2, 2])));
    f.layer.run_jobs();
    f.present(preview.job.unwrap());
    let receipt = f.shell.press(&mut f.layer, "Ctrl+Shift+Enter", false).unwrap().unwrap();
    let Status::Committed(record) = receipt.status else { panic!("{:?}", receipt.status) };
    assert_eq!(record.post, derived(&start, &word(&[1])).digest());
    let k = kernel();
    let literal = k.effect(&k.recipe(&word(&[1])).unwrap()).unwrap().expansion_sha256;
    assert_eq!(f.layer.store().log[0].literal, literal);
}

/// CT-KN5-09, CT-KN7-04 and CT-LI-04's job half.
#[test]
fn undo_goes_ahead_while_a_program_job_runs() {
    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.inspect", Kind::Query, &[], overlay("inspect")).unwrap();
    f.layer.register_overlay("tool.full.export", Kind::File, &[], overlay("export")).unwrap();
    f.layer.set_program_capacity(2);
    assert!(matches!(f.commit(&word(&[1])).status, Status::Committed(_)));
    let query = f.shell.invoke(&mut f.layer, "tool.full.inspect", json::object([])).unwrap();
    assert!(matches!(
        f.shell.invoke(&mut f.layer, "tool.full.export", json::object([])).unwrap().status,
        Status::Running(_)
    ));
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(taken.row, "tool.full.inspect");
    assert!(matches!(f.undo(Direction::Undo).status, Status::Committed(_)));
    assert_eq!(
        f.layer.outcome(&query.command, &query.request).unwrap().status,
        Status::Cancelled(CancelReason::ReplacedRevision)
    );
    assert_eq!(f.layer.finish_program_job(taken.job, Ok(Value::Null)), None);
    let export = f.layer.take_program_job().unwrap();
    assert_eq!(export.row, "tool.full.export");
    let done = f.layer.finish_program_job(export.job, Ok(Value::String("bytes".into()))).unwrap();
    assert!(matches!(done.status, Status::Completed(Completion { kind: CompletionKind::File, .. })));
}

/// CT-KN5-11 and CT-LI-09's stale hint.
#[test]
fn an_undo_naming_a_superseded_action_is_refused() {
    let mut f = fixture(&[]);
    f.commit(&word(&[1]));
    let old = f.layer.undo_hint(Direction::Undo).unwrap();
    f.commit(&word(&[2]));
    let current = f.layer.undo_hint(Direction::Undo).unwrap();
    assert_eq!((old, current), (ActionHint { order: 1, target: 1 }, ActionHint { order: 2, target: 2 }));
    let before = f.fixed();
    let receipt = f.shell.invoke(&mut f.layer, "session.undo", json::object([("hint", old.to_json())])).unwrap();
    assert_eq!(code_of(&receipt), code::UNDO_SUPERSEDED);
    assert_eq!(f.fixed(), before);
}

/// CT-KN5-12.
#[test]
fn availability_follows_the_preview_lifecycle() {
    let mut f = fixture(&[]);
    let mut last = f.layer.generation();
    let mut advanced = |layer: &CommandLayer| {
        let now = layer.generation();
        let moved = now > last;
        last = now;
        moved
    };
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::NO_FRESH_PREVIEW);
    f.shell.draft = Some(word_draft(&word(&[1])));
    let preview = f.shell.press(&mut f.layer, "Ctrl+Enter", false).unwrap().unwrap();
    let id = preview.job.unwrap();
    assert!(advanced(&f.layer));
    assert_eq!(f.layer.preview_state(id), Some(PreviewState::Building));
    assert_eq!(f.layer.availability("operation.preview").unwrap_err().code, code::LANE_FULL);
    f.layer.run_jobs();
    assert!(advanced(&f.layer));
    assert!(f.layer.availability("operation.preview").is_ok());
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::NO_FRESH_PREVIEW);
    f.present(id);
    assert!(advanced(&f.layer));
    assert!(f.layer.availability("operation.execute").is_ok());
    f.layer.hold_commit_lane(true);
    assert!(advanced(&f.layer));
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::COMMIT_IN_FLIGHT);
    f.layer.hold_commit_lane(false);
    assert!(advanced(&f.layer));
    let cancel = f.shell.press(&mut f.layer, "Escape", false).unwrap().unwrap();
    assert!(matches!(cancel.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
    assert!(advanced(&f.layer));
    assert_eq!(f.layer.preview_state(id), Some(PreviewState::Cancelled));
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::NO_FRESH_PREVIEW);
    assert_eq!(f.layer.revision().state, 0);
}

/// CT-KN7-05 (R-16).
#[test]
fn concurrent_requests_cannot_pass_a_one_job_lane() {
    let mut f = fixture(&[]);
    let first = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
    let second = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
    assert!(matches!(first.status, Status::Running(_)));
    assert_eq!(code_of(&second), code::LANE_FULL);
    f.layer.register_overlay("tool.q", Kind::Query, &[], overlay("q")).unwrap();
    let program = f.shell.invoke(&mut f.layer, "tool.q", json::object([])).unwrap();
    assert!(matches!(program.status, Status::Running(_)));
    assert_eq!(code_of(&f.shell.invoke(&mut f.layer, "tool.q", json::object([])).unwrap()), code::LANE_FULL);
    assert_eq!(f.layer.run_jobs(), 1);
}

/// CT-ONE-11.
#[test]
fn a_commit_refused_at_each_step_changes_nothing() {
    let (clear, moves) = o06();
    let mut f = fixture(&[1]);
    let p = f.preview(&clear[0]);
    f.present(p);
    let before = f.fixed();

    // Step 3: a forged authenticator, then a spent token.
    let mut forged = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap();
    forged.token = Some(Token::mint(&Secret::fixed([9; 32]), forged.token.unwrap().binding));
    assert_eq!(code_of(&f.layer.invoke(forged)), code::TOKEN_INVALID);
    let spent = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap();
    // Step 4: the lane is busy; the token authenticated, so it is spent. Lane admission comes before step 5, so a
    // genuine token bound to other arguments is refused as busy too.
    let mut other = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap();
    other.args = preview_args(p + 7);
    f.layer.hold_commit_lane(true);
    assert_eq!(code_of(&f.layer.invoke(spent.clone())), code::COMMIT_IN_FLIGHT);
    assert_eq!(code_of(&f.layer.invoke(other)), code::COMMIT_IN_FLIGHT);
    f.layer.hold_commit_lane(false);
    let replay = Invocation { command: "replay/1".into(), ..spent };
    assert_eq!(code_of(&f.layer.invoke(replay)), code::TOKEN_USED);
    assert_eq!(f.fixed(), before);

    // Step 6: a protection change makes the preview stale even though the set is the same.
    f.layer.set_protected(kernel().protected_set(&[1]).unwrap());
    assert_eq!(code_of(&f.execute()), code::REVISION_CHANGED);
    assert_eq!(f.fixed(), before);

    // Step 7: the head differs from the preview's start.
    let p = f.preview(&clear[0]);
    f.present(p);
    let saved = f.layer.snapshot().clone();
    f.layer.fault_replace_head(derived(&saved, &word(&[3])));
    assert_eq!(code_of(&f.execute()), code::STATE_HASH_MISMATCH);
    f.layer.fault_replace_head(saved);
    assert_eq!(f.fixed(), before);

    // Step 8: a protected orbit would move. The preview is built (KN5.9); its commit is refused.
    let m = f.preview(&moves[0]);
    assert_eq!(f.layer.preview().unwrap().protection.verdict, Verdict::Moves);
    f.present(m);
    assert_eq!(code_of(&f.execute()), code::ORBIT_WOULD_MOVE);
    assert_eq!(f.fixed(), before);

    // Step 9: a rolled-back transaction fails the commit and leaves the preview committable.
    let p = f.preview(&clear[0]);
    f.present(p);
    f.layer.store_mut().inject.push_back(Fate::RolledBack(Error::new("PS1:io.injected", "injected rollback")));
    let failed = f.execute();
    assert_eq!(failed.status, Status::Failed(Error::new("PS1:io.injected", "injected rollback")));
    assert_eq!(f.fixed(), before);
    assert!(matches!(f.execute().status, Status::Committed(_)));
}

/// CT-LI-10.
#[test]
fn a_protection_conflict_builds_a_preview_and_refuses_its_commit() {
    let (_, moves) = o06();
    let mut f = fixture(&[1]);
    let id = f.preview(&moves[0]);
    let p = f.layer.preview().unwrap();
    assert!(p.end.is_none());
    assert_eq!(p.protection.verdict, Verdict::Moves);
    assert!(!p.protection.conflicts.is_empty());
    f.present(id);
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::ORBIT_WOULD_MOVE);
    // A mechanically refused recipe builds no preview.
    for step in case("O07").get("steps").and_then(Value::as_array).unwrap() {
        let draft = draft_args(step.get("recipe").unwrap());
        let receipt = f.shell.invoke(&mut f.layer, "operation.preview", draft).unwrap();
        assert!(code_of(&receipt).starts_with("KN2:"), "{:?}", receipt.status);
    }
    assert_eq!(f.layer.preview().unwrap().id, id);
}

/// ADR-L02, KN5.15, CT-ONE-20.
#[test]
fn the_runtime_reaches_at_most_a_preview() {
    let mut f = fixture(&[]);
    let runtime = Caller::Runtime { run: "run-1".into() };
    let call = |command: &str, row: &str, args: Value| Invocation {
        command: command.into(),
        row: row.into(),
        args,
        token: None,
        caller: runtime.clone(),
        epoch: 1,
    };
    let proposal = f.layer.invoke(call("rt/1", "operation.preview", draft_args(&word(&[1]))));
    assert!(matches!(proposal.status, Status::Running(_)));
    assert_eq!(f.layer.run_jobs(), 1);
    let rows: Vec<String> =
        f.layer.table().rows().filter(|r| matches!(r.kind, Kind::Commit | Kind::App)).map(|r| r.id.clone()).collect();
    assert_eq!(rows.len(), 19);
    for row in rows {
        let receipt = f.layer.invoke(call(&format!("rt/{row}"), &row, json::object([])));
        assert_eq!(code_of(&receipt), code::COMMIT_ROW_REFUSED, "{row}");
    }
    let mut tokened = call("rt/2", "operation.preview", draft_args(&word(&[2])));
    let binding = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
    tokened.token = Some(Token::mint(
        &Secret::fixed(SECRET),
        TokenBinding {
            nonce: 77,
            session: "s1".into(),
            epoch: 1,
            command: binding.command,
            row: "operation.preview".into(),
            args: args_digest(&tokened.args),
            identity: kernel().identity().model_id,
            preview: None,
            revision: None,
            hint: None,
        },
    ));
    assert_eq!(code_of(&f.layer.invoke(tokened)), code::TOKEN_INVALID);
    // The run is over; the solver commits its preview with nothing of the run.
    f.present(proposal.job.unwrap());
    assert!(matches!(f.execute().status, Status::Committed(_)));
}

/// CT-ONE-21.
#[test]
fn a_restart_ends_previews_keeps_durable_receipts_and_revives_no_permission() {
    let mut f = fixture(&[]);
    let c = f.preview(&word(&[1]));
    f.present(c);
    let press = f.shell.prepare(&f.layer, "operation.execute", preview_args(c)).unwrap();
    let committed = f.shell.send(&mut f.layer, press.clone());
    assert!(matches!(committed.status, Status::Committed(_)));
    let p = f.preview(&word(&[2]));
    f.present(p);
    let early = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap();
    let asked = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[3]))).unwrap();
    let pending = f.shell.send(&mut f.layer, asked.clone());
    f.layer.restart(Secret::fixed([8; 32]));
    f.shell.restart(Secret::fixed([8; 32]));
    assert_eq!((f.layer.revision().epoch, f.layer.revision().state), (2, 1));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Discarded));
    assert_eq!(f.layer.outcome(&committed.command, &committed.request), Some(committed.clone()));
    assert_eq!(f.layer.outcome(&pending.command, &pending.request).unwrap().status, Status::UnknownOutcome);
    // A transport retry keeps its whole envelope, its epoch included, so it finds its receipt across the restart and
    // starts nothing (KN5.21, LI.5, LI.30).
    let before = f.fixed();
    assert_eq!(f.layer.invoke(press.clone()), committed);
    assert_eq!(f.layer.invoke(asked).status, Status::UnknownOutcome);
    assert_eq!(f.fixed(), before);
    // The same command ID with other content, or a lookup with another digest, is refused; the record stays.
    let mut reused = press.clone();
    reused.args = preview_args(c + 9);
    assert_eq!(code_of(&f.layer.invoke(reused)), code::COMMAND_ID_REUSED);
    assert_eq!(code_of(&f.layer.outcome(&committed.command, &pending.request).unwrap()), code::COMMAND_ID_REUSED);
    assert_eq!(f.layer.outcome(&committed.command, &committed.request), Some(committed.clone()));
    // A new envelope from the ended epoch is refused, and so is the old epoch's token.
    let mut stale = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[4]))).unwrap();
    assert_eq!(stale.epoch, 2);
    stale.epoch = 1;
    assert_eq!(code_of(&f.layer.invoke(stale)), code::EPOCH_CHANGED);
    assert_eq!(code_of(&f.layer.invoke(Invocation { epoch: 2, ..early })), code::TOKEN_INVALID);
    assert_eq!(f.fixed(), before);
    assert!(matches!(f.undo(Direction::Undo).status, Status::Committed(_)));
}

/// CT-ONE-22, and the unknown fate settled at the restart.
#[test]
fn an_unknown_commit_fate_enters_read_only_recovery() {
    for written in [false, true] {
        let mut f = fixture(&[]);
        let start = f.layer.snapshot().digest().to_owned();
        f.layer.register_overlay("tool.q", Kind::Query, &[], overlay("q")).unwrap();
        let p = f.preview(&word(&[1]));
        f.present(p);
        f.layer.store_mut().inject.push_back(Fate::Unknown);
        f.layer.store_mut().unknown_writes = written;
        let press = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap();
        let receipt = f.shell.send(&mut f.layer, press.clone());
        assert_eq!(receipt.status, Status::UnknownOutcome);
        assert_eq!(f.layer.recovery().unwrap().code, code::COMMIT_FATE);
        assert_eq!(f.layer.snapshot().digest(), start);
        let proposal = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
        assert_eq!(code_of(&proposal), code::READ_ONLY_RECOVERY);
        assert_eq!(f.layer.availability("session.undo").unwrap_err().code, code::READ_ONLY_RECOVERY);
        let read = f.shell.invoke(&mut f.layer, "tool.q", json::object([])).unwrap();
        assert!(matches!(read.status, Status::Running(_)));
        f.layer.restart(Secret::fixed([8; 32]));
        f.shell.restart(Secret::fixed([8; 32]));
        assert!(f.layer.recovery().is_none());
        let outcome = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
        // A retry of the press finds the settled outcome and commits nothing a second time (KN5.21, KN5.23).
        let settled = (f.layer.revision().clone(), f.layer.snapshot().digest().to_owned());
        assert_eq!(f.layer.invoke(press), outcome);
        assert_eq!((f.layer.revision().clone(), f.layer.snapshot().digest().to_owned()), settled);
        if written {
            assert!(matches!(outcome.status, Status::Committed(_)));
            assert_eq!(f.layer.revision().state, 1);
            assert_ne!(f.layer.snapshot().digest(), start);
        } else {
            assert_eq!(outcome.status, Status::UnknownOutcome);
            assert_eq!(f.layer.revision().state, 0);
            assert_eq!(f.layer.snapshot().digest(), start);
        }
        assert_eq!(f.layer.revision().epoch, 2);
    }
}
