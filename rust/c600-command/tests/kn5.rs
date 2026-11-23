//! The KN5 attack leads against the command layer (L3 to L11, L15, L18) and the CT-KN5-04 attack. Each test names the
//! conformance rows it covers headless with PHS.62 markers. Where the frozen text decides, the expected code follows
//! it; where it is silent, the test follows the draft rulings recorded in `src/lib.rs` and says so.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use c600_command::token::args_digest;
use c600_command::{
    code, digest, draft_args, identity_json, Caller, CancelReason, CommandLayer, Completion, CompletionKind, Direction,
    HeadlessFrames, Invocation, Kind, MemoryStore, Overlay, PreviewBinding, PreviewDeps, PreviewState, Receipt, Secret,
    Shell, Status, Table, Token, TokenBinding, TransactionKind,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{archive, Derivation, Kernel, ProtectedSet, Snapshot};

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

/// O06's recipes under its protected set `[1]`: those the oracle accepts and those it refuses as protected.
fn o06() -> (Vec<Value>, Vec<Value>) {
    let path = repository().join("work/experiments/language-lx/inputs/oracle/O06.json.gz");
    let bytes = std::fs::read(path).expect("the frozen case file is readable");
    let case = json::parse(&archive::gunzip(&bytes, 1 << 30).unwrap()).unwrap();
    let (mut clear, mut moves) = (Vec::new(), Vec::new());
    for step in case.get("steps").and_then(Value::as_array).unwrap() {
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

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn running(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Running(_))
}

/// One edit of a token's binding (L9).
type Edit = Box<dyn Fn(&mut TokenBinding)>;

fn edit(change: impl Fn(&mut TokenBinding) + 'static) -> Edit {
    Box::new(change)
}

fn deps(change: fn(&mut PreviewDeps)) -> Edit {
    Box::new(move |binding: &mut TokenBinding| change(&mut binding.preview.as_mut().expect("a preview").deps))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

fn fixture(protected: &[i64]) -> Fixture {
    let k = kernel();
    let protected = k.protected_set(protected).unwrap();
    let start = k.solved();
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    // ADR-D01.9's settle interval is tested in tests/settle.rs; these sessions commit on the first presented press.
    layer.set_settle(Duration::ZERO);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    /// Previews `recipe` through the command route and runs the job; returns the preview's ID.
    fn preview(&mut self, recipe: &Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(job) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        job
    }

    /// The next headless frame, which shows the pending preview through the built-in chain, reported for `id`.
    fn present(&mut self, id: u64) {
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
    }

    /// A run inside a command the solver invoked (finding B2 of the KN5 boundary attack, ONE.34): the shell invokes a
    /// tool's query row, and the host takes its job and runs it as the job's run (`ProgramJob::run`), which lasts until
    /// the host reports the job. Only such a run prepares a preview.
    fn solver_run(&mut self) -> String {
        self.layer.register_overlay("tool.full.run", Kind::Query, &[], overlay("run")).unwrap();
        let receipt = self.shell.invoke(&mut self.layer, "tool.full.run", json::object([])).unwrap();
        assert!(matches!(receipt.status, Status::Running(_)), "the tool's row: {:?}", receipt.status);
        self.layer.take_program_job().expect("the tool's job").run()
    }

    /// The commit chord for preview `id`, prepared and not sent.
    fn press(&mut self, id: u64) -> Invocation {
        self.shell.prepare(&self.layer, "operation.execute", preview_args(id)).unwrap()
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let id = self.preview(recipe);
        self.present(id);
        let press = self.press(id);
        self.shell.send(&mut self.layer, press)
    }

    fn cancel(&mut self, id: u64) -> Receipt {
        self.shell.invoke(&mut self.layer, "operation.preview.cancel", preview_args(id)).unwrap()
    }

    /// The undo or redo chord for the current hint, prepared and not sent; `confirm` confirms a release (LI.26).
    fn step_press(&mut self, direction: Direction, confirm: bool) -> Invocation {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        let mut args = vec![("hint".to_owned(), hint.to_json())];
        if confirm {
            args.push(("confirm".to_owned(), Value::Bool(true)));
        }
        self.shell.prepare(&self.layer, row, Value::Object(args.into_iter().collect())).unwrap()
    }

    fn step(&mut self, direction: Direction, confirm: bool) -> Receipt {
        let press = self.step_press(direction, confirm);
        self.shell.send(&mut self.layer, press)
    }

    /// `protection.orbit.set` with `orbits`; `confirm` confirms a release (LI.26).
    fn protect(&mut self, orbits: &str, confirm: bool) -> Receipt {
        let mut args = vec![("orbits".to_owned(), parse(orbits))];
        if confirm {
            args.push(("confirm".to_owned(), Value::Bool(true)));
        }
        self.shell.invoke(&mut self.layer, "protection.orbit.set", Value::Object(args.into_iter().collect())).unwrap()
    }

    /// A commit chord for the pending preview, minted with the launch's secret and `nonce` and bound as the shell binds
    /// it, with the newest reported frame's stamp, then changed by `change`: a test minter, which skips the shell's gate
    /// (L3 (f), L9).
    fn forge(&mut self, nonce: u64, change: impl FnOnce(&mut TokenBinding)) -> Invocation {
        let frame = self.layer.frame().map(|f| f.stamp.clone()).unwrap_or_else(|| self.frames.next());
        let p = self.layer.preview().expect("a pending preview");
        let preview = PreviewBinding { id: p.id, literal: p.literal.clone(), deps: p.deps.clone(), frame };
        let args = preview_args(p.id);
        let revision = self.layer.revision().clone();
        let command = self.shell.command_id();
        let mut binding = TokenBinding {
            nonce,
            session: revision.session.clone(),
            epoch: revision.epoch,
            command: command.clone(),
            row: "operation.execute".into(),
            args: args_digest(&args),
            identity: self.layer.identity_binding(),
            preview: Some(preview),
            revision: None,
            hint: None,
        };
        change(&mut binding);
        let token = Some(Token::mint(&Secret::fixed(SECRET), binding));
        Invocation { command, row: "operation.execute".into(), args, token, epoch: revision.epoch, budget: None }
    }

    /// What no refusal may change: the state digest, the revision, the protected set and the durable log.
    fn fixed(&self) -> (String, String, Vec<usize>, usize) {
        let r = self.layer.revision();
        (r.digest.clone(), format!("{r:?}"), self.layer.protected().orbits().to_vec(), self.layer.store().log.len())
    }

    /// Delivers `invocation` on the shell's channel, which must be refused with `expected` and change nothing.
    fn refused(&mut self, invocation: Invocation, expected: &str, what: &str) -> Receipt {
        self.refused_on(&Caller::Shell, invocation, expected, what)
    }

    /// Delivers `invocation` on `caller`'s channel, which must be refused with `expected` and change nothing.
    fn refused_on(&mut self, caller: &Caller, invocation: Invocation, expected: &str, what: &str) -> Receipt {
        let before = self.fixed();
        let receipt = self.layer.invoke(caller, invocation);
        assert_eq!(receipt.status.code(), Some(expected), "{what}: {:?}", receipt.status);
        assert!(matches!(receipt.status, Status::Refused(_)), "{what}: {:?}", receipt.status);
        assert_eq!(self.fixed(), before, "{what}");
        receipt
    }
}

/// CT-KN5-04 (part: an envelope that does not authenticate, with a forged authenticator or from a runtime caller, sent
/// first under the genuine press's command ID never takes the ID, so the genuine press then commits once; every
/// envelope's retry answers its own stored receipt; a later envelope whose request digest equals the committed one's is
/// a retry of it whatever its authenticator, since the retry lookup precedes authentication (chapter 08 section 7), and
/// it commits nothing; not run: the shell's transport), L11 (a), L18 (c), ruling 4. The control case sends no
/// first envelope. CT-BND-20 (part: a runtime envelope never takes the press's command ID, so the press commits once,
/// headless at the layer; not run: the actual Windows half, on the owner's machine (C9)).
#[test]
fn an_envelope_that_does_not_authenticate_never_takes_the_command_id() {
    for attack in ["forged", "runtime", "control"] {
        let mut f = fixture(&[]);
        let p = f.preview(&word(&[1]));
        f.present(p);
        let genuine = f.press(p);
        let token = genuine.token.clone().expect("a commit chord carries a token");
        let first = match attack {
            "forged" => {
                let forged = Token::mint(&Secret::fixed([9; 32]), token.binding.clone());
                Some((Caller::Shell, Invocation { token: Some(forged), ..genuine.clone() }))
            }
            // The genuine envelope sent on a run's channel: the host stamps the runtime as its caller (ruling 4).
            "runtime" => Some((Caller::Runtime { run: "run-1".into() }, genuine.clone())),
            _ => None,
        };
        let refusal = first.clone().map(|(caller, first)| {
            let expected = if attack == "forged" { code::TOKEN_INVALID } else { code::COMMIT_ROW_REFUSED };
            f.refused_on(&caller, first, expected, attack)
        });
        let receipt = f.layer.invoke(&Caller::Shell, genuine.clone());
        assert!(committed(&receipt), "{attack}: {:?}", receipt.status);
        assert_eq!(f.layer.outcome(&genuine.command, &receipt.request), Some(receipt.clone()), "{attack}");
        let fixed = f.fixed();
        assert_eq!(f.layer.invoke(&Caller::Shell, genuine.clone()), receipt, "{attack}");
        if let (Some((caller, first)), Some(refusal)) = (first, refusal) {
            assert_eq!(f.layer.invoke(&caller, first), refusal, "{attack}");
        }
        let late = Token::mint(&Secret::fixed([10; 32]), token.binding.clone());
        assert_eq!(
            f.layer.invoke(&Caller::Shell, Invocation { token: Some(late), ..genuine.clone() }),
            receipt,
            "{attack}"
        );
        let mut other = genuine.clone();
        other.args = preview_args(p + 1);
        assert_eq!(code_of(&f.layer.invoke(&Caller::Shell, other)), code::COMMAND_ID_REUSED, "{attack}");
        assert_eq!(f.fixed(), fixed, "{attack}");
        assert_eq!(f.layer.store().log.len(), 1, "{attack}");
    }
}

/// CT-KN5-01 (part: a press whose preview ended before its delivery is refused by how the preview ended, and nothing
/// changes: a slot that holds another preview answers `preview_replaced`, an empty slot `no_fresh_preview`
/// (ruling 1), and a token for a preview never presented `no_fresh_preview`; not run: a display adapter's own frame
/// report, for which the headless frames stand in), CT-ONE-04 (part: the same cases; not run: the shell's display),
/// CT-ONE-05 (part: a preview replaced or cancelled after the press; not run: the input timing), L3.
#[test]
fn a_press_whose_preview_ended_is_refused_by_how_it_ended() {
    let cases = [
        ("replaced", code::PREVIEW_REPLACED),
        ("cancelled", code::NO_FRESH_PREVIEW),
        ("cancelled, then another presented", code::PREVIEW_REPLACED),
        ("undone", code::NO_FRESH_PREVIEW),
        ("committed", code::NO_FRESH_PREVIEW),
        ("never presented", code::NO_FRESH_PREVIEW),
    ];
    for (case, expected) in cases {
        let mut f = fixture(&[]);
        assert!(committed(&f.commit(&word(&[1]))));
        let p = f.preview(&word(&[2]));
        let press = match case {
            "never presented" => {
                // The shell mints a commit token only for the preview it presented (L3); a test minter skips its gate.
                let gate = f.shell.prepare(&f.layer, "operation.execute", preview_args(p)).unwrap_err();
                assert_eq!(gate.code, code::NO_FRESH_PREVIEW);
                f.forge(1 << 40, |_| {})
            }
            _ => {
                f.present(p);
                f.press(p)
            }
        };
        match case {
            "replaced" => {
                f.preview(&word(&[3]));
            }
            "cancelled" => assert!(matches!(f.cancel(p).status, Status::Completed(_))),
            "cancelled, then another presented" => {
                assert!(matches!(f.cancel(p).status, Status::Completed(_)));
                let q = f.preview(&word(&[3]));
                f.present(q);
            }
            "undone" => assert!(committed(&f.step(Direction::Undo, false))),
            "committed" => {
                let first = f.press(p);
                assert!(committed(&f.shell.send(&mut f.layer, first)));
            }
            _ => assert_eq!(f.layer.preview_state(p), Some(PreviewState::Ready)),
        }
        f.refused(press, expected, case);
    }
}

/// CT-KN5-09 (part: an action that does not move the puzzle, the undo of a protection change or a workspace overlay's
/// completion, keeps the head, the state revision, the pending preview and the running jobs; the protection revision is
/// one of the preview's dependencies (KN0.8), so a press from before a protection undo is stale, while a press from
/// before a workspace completion commits; the commit then ends the query job and keeps the file job; a record write
/// and its undo are in `kn5_stages.rs`), KN5.18, L4.
#[test]
fn an_action_that_moves_nothing_keeps_the_preview_and_the_jobs() {
    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.inspect", Kind::Query, &[], overlay("inspect")).unwrap();
    let granted = c600_command::Registration::default().granting(c600_command::Grant::File);
    f.layer.register_overlay_with("tool.full.export", Kind::File, &[], overlay("export"), granted).unwrap();
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();
    f.layer.set_program_capacity(3);
    assert!(committed(&f.commit(&word(&[1]))));
    assert!(matches!(f.protect("[1]", false).status, Status::Completed(_)));
    let query = f.shell.invoke(&mut f.layer, "tool.full.inspect", json::object([])).unwrap();
    let export = f.shell.invoke(&mut f.layer, "tool.full.export", json::object([])).unwrap();
    let note = f.shell.invoke(&mut f.layer, "tool.full.note", json::object([])).unwrap();
    assert!(running(&query) && running(&export) && running(&note));
    let inspect = f.layer.take_program_job().unwrap();
    assert_eq!(inspect.row, "tool.full.inspect");
    let p = f.preview(&word(&[2]));
    f.present(p);
    let press = f.press(p);

    // The undo of the protection change releases orbit 1, so it confirms (LI.26).
    let before = f.layer.revision().clone();
    let undone = f.step(Direction::Undo, true);
    let Status::Committed(record) = &undone.status else { panic!("{:?}", undone.status) };
    assert_eq!(record.pre, record.post);
    let after = f.layer.revision().clone();
    assert_eq!((&after.head, after.state, &after.digest), (&before.head, before.state, &before.digest));
    assert_eq!((after.workspace, after.protection), (before.workspace + 1, before.protection + 1));
    assert!(f.layer.protected().orbits().is_empty());
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));
    for receipt in [&query, &export, &note] {
        assert_eq!(f.layer.outcome(&receipt.command, &receipt.request).as_ref(), Some(receipt));
    }
    f.refused(press, code::REVISION_CHANGED, "a press from before the protection undo");

    // A workspace overlay's completion advances the workspace revision only.
    let p = f.preview(&word(&[2]));
    f.present(p);
    let press = f.press(p);
    assert_eq!(f.layer.take_program_job().unwrap().row, "tool.full.export");
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(taken.row, "tool.full.note");
    let before = f.layer.revision().clone();
    let done = f.layer.finish_program_job(taken.job, Ok(Value::String("note".into()))).unwrap();
    assert!(matches!(done.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
    let after = f.layer.revision().clone();
    assert_eq!((&after.head, after.state, after.protection), (&before.head, before.state, before.protection));
    assert_eq!(after.workspace, before.workspace + 1);
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));
    assert_eq!(f.layer.outcome(&query.command, &query.request).as_ref(), Some(&query));
    assert!(committed(&f.shell.send(&mut f.layer, press)));

    // The commit moves the puzzle: it ends the query job and keeps the file job.
    let ended = f.layer.outcome(&query.command, &query.request).unwrap();
    assert_eq!(ended.status, Status::Cancelled(CancelReason::ReplacedRevision));
    assert_eq!(f.layer.outcome(&export.command, &export.request).as_ref(), Some(&export));
}

/// CT-KN5-11 (part: an undo press raced by another durable action is `undo_superseded`, whether a commit or a workspace
/// overlay's completion landed first, since the hint check comes before the bound revisions (draft ruling 2); a view
/// completion takes no action order, so the press still commits; not run: LI.9's checkpoint vector, since
/// `session.checkpoint.save` is not built), CT-LI-09 (part, draft: the same races), L5.
#[test]
fn a_raced_undo_is_superseded_by_any_durable_action() {
    for race in ["commit", "workspace", "view"] {
        let mut f = fixture(&[]);
        f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();
        f.layer.register_overlay("tool.full.view", Kind::View, &[], overlay("view")).unwrap();
        assert!(committed(&f.commit(&word(&[1]))));
        let row = match race {
            "workspace" => Some("tool.full.note"),
            "view" => Some("tool.full.view"),
            _ => None,
        };
        let job = row.map(|row| {
            assert!(running(&f.shell.invoke(&mut f.layer, row, json::object([])).unwrap()), "{race}");
            f.layer.take_program_job().unwrap().job
        });
        let press = f.step_press(Direction::Undo, false);
        match job {
            None => assert!(committed(&f.commit(&word(&[2])))),
            Some(job) => assert!(f.layer.finish_program_job(job, Ok(Value::Null)).is_some()),
        }
        if race == "view" {
            assert!(committed(&f.shell.send(&mut f.layer, press)), "{race}");
        } else {
            f.refused(press, code::UNDO_SUPERSEDED, race);
        }
    }
}

/// CT-KN5-04 (part: no receipt and no lookup answers `accepted`; a press refused as busy keeps its refusal under its
/// command ID, so its identical retry after the lane is free answers the stored `commit_in_flight` and commits nothing;
/// a later delivery of a recorded command ID with other content never rewrites its record; not run: concurrent
/// deliveries, which a layer behind `&mut self` serializes), KN5.21, L6, L18 (b), draft ruling 3.
#[test]
fn a_refusal_is_answered_once_and_accepted_is_never_shown() {
    let mut f = fixture(&[]);
    let mut seen = Vec::new();
    let asked = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
    let Status::Running(p) = asked.status else { panic!("{:?}", asked.status) };
    seen.push(asked.clone());
    assert_eq!(f.layer.run_jobs(), 1);
    seen.push(f.layer.outcome(&asked.command, &asked.request).unwrap());
    f.present(p);
    let held = f.press(p);
    f.layer.hold_commit_lane(true);
    let busy = f.refused(held.clone(), code::COMMIT_IN_FLIGHT, "a held lane");
    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.invoke(&Caller::Shell, held.clone()), busy);
    assert_eq!(f.layer.outcome(&held.command, &busy.request), Some(busy.clone()));
    assert_eq!(f.layer.store().log.len(), 0);
    let press = f.press(p);
    let receipt = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&receipt));
    for (sent, answer) in [(&held, &busy), (&press, &receipt)] {
        let mut other = sent.clone();
        other.args = preview_args(p + 1);
        let reused = f.layer.invoke(&Caller::Shell, other);
        assert_eq!(code_of(&reused), code::COMMAND_ID_REUSED);
        let kept = f.layer.outcome(&sent.command, &answer.request).unwrap();
        assert_eq!(&kept, answer);
        seen.extend([reused, kept]);
    }
    seen.extend([busy, receipt]);
    assert!(seen.iter().all(|r| r.status != Status::Accepted), "{seen:?}");
    assert_eq!(f.layer.store().log.len(), 1);
}

/// CT-KN5-04 (part: a command ID never recorded answers `unknown_outcome`; a stale-epoch envelope that authenticates
/// keeps its refusal under its command ID for the epoch, so its lookup and its retry answer it and the ID with other
/// content is refused, and after the next restart it answers `unknown_outcome`; not run: a lookup through the shell's
/// transport), KN5.23, LI.30 (draft), L7.
#[test]
fn an_unknown_id_is_unknown_and_a_stale_envelope_keeps_its_refusal_for_the_epoch() {
    let mut f = fixture(&[]);
    for caller in [Caller::Shell, Caller::Runtime { run: "run-1".into() }] {
        assert_eq!(f.layer.kernel_outcome(&caller, "shell/404", "0").status, Status::UnknownOutcome);
    }
    assert_eq!(f.layer.outcome("shell/404", "0"), None);
    let early = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
    f.layer.restart(Secret::fixed(SECRET));
    f.shell.restart(Secret::fixed(SECRET));
    let stale = f.refused(early.clone(), code::EPOCH_CHANGED, "an envelope of the ended epoch");
    assert_eq!(f.layer.outcome(&early.command, &stale.request), Some(stale.clone()));
    assert_eq!(f.layer.invoke(&Caller::Shell, early.clone()), stale);
    let epoch = f.layer.revision().epoch;
    let other = Invocation { epoch, args: draft_args(&word(&[2])), ..early.clone() };
    assert_eq!(code_of(&f.layer.invoke(&Caller::Shell, other)), code::COMMAND_ID_REUSED);
    assert_eq!(f.layer.outcome(&early.command, &stale.request), Some(stale.clone()));
    assert_eq!(f.layer.run_jobs(), 0);
    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome(&early.command, &stale.request).unwrap().status, Status::UnknownOutcome);
}

/// CT-KN5-04 (part: a workspace overlay's completion is durable with its receipt, so its lookup and its retry answer
/// the same `completed(workspace)` after a restart and after a reopen and write nothing; a record write is in
/// `kn5_stages.rs`), KN5.23, L8.
#[test]
fn a_workspace_completion_keeps_its_receipt_across_a_restart_and_a_reopen() {
    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();
    let asked = f.shell.prepare(&f.layer, "tool.full.note", json::object([])).unwrap();
    let started = f.shell.send(&mut f.layer, asked.clone());
    let Status::Running(job) = started.status else { panic!("{:?}", started.status) };
    assert_eq!(f.layer.take_program_job().unwrap().job, job);
    let done = f.layer.finish_program_job(job, Ok(Value::String("note".into()))).unwrap();
    assert!(matches!(done.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
    assert_eq!(f.layer.store().log.len(), 1);
    assert_eq!(f.layer.store().log[0].kind, TransactionKind::Workspace);
    assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), done);
    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome(&asked.command, &done.request), Some(done.clone()));
    assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), done);
    assert_eq!(f.layer.store().log.len(), 1);
    let store = f.layer.store().clone();
    let mut reopened = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), store).unwrap();
    assert_eq!(reopened.outcome(&asked.command, &done.request), Some(done.clone()));
    assert_eq!(reopened.invoke(&Caller::Shell, asked), done);
    assert_eq!((reopened.store().log.len(), reopened.revision().workspace), (1, 1));
}

/// CT-KN5-01 (part: a token minted with the launch's secret and bound to anything but this press is refused, and
/// nothing changes: another preview ID answers `preview_replaced`; another session, epoch, command, row, argument
/// digest, literal digest or identity-block field `token_mismatch` (draft ruling 5 for the session and the epoch); a
/// changed PreviewDeps field `revision_changed`; a frame stamp outside the preview's presentation `no_fresh_preview`
/// (ONE.41); the unchanged binding commits; not run: the shell's display), CT-ONE-03 (part: the same matrix for a word
/// commit; the live twist and the crossing run in `twist_apply.rs`, `jumble_twist_menu.rs`, `jumble_on.rs` and
/// `jumble_off.rs`), CT-ERR-09 (part: the replaced preview and the bound content fields), CT-LG3-05 (part: at
/// `operation.execute` a changed epoch, identity, mode, state, protection or draft field of the preview's dependencies
/// refuses `revision_changed`; not run: a program's proposal refused at `operation.preview` on a moved head, in the
/// other mode or under another model), KN5.14, KN5.16, L9.
/// CT-ONE-26 (part: the token binds the presented preview, its ID, literal digest, dependencies and frame stamp, in the
/// headless layer; not run: the on-screen half (C9)).
#[test]
fn every_bound_field_of_a_token_is_checked() {
    let mut f = fixture(&[]);
    let p = f.preview(&word(&[1]));
    f.present(p);
    let mut cases: Vec<(String, &str, Edit)> = vec![
        ("session".into(), code::TOKEN_MISMATCH, edit(|b| b.session = "s2".into())),
        ("epoch".into(), code::TOKEN_MISMATCH, edit(|b| b.epoch += 1)),
        ("command".into(), code::TOKEN_MISMATCH, edit(|b| b.command = "shell/elsewhere".into())),
        ("row".into(), code::TOKEN_MISMATCH, edit(|b| b.row = "session.undo".into())),
        ("arguments".into(), code::TOKEN_MISMATCH, edit(move |b| b.args = args_digest(&preview_args(p + 1)))),
        ("preview ID".into(), code::PREVIEW_REPLACED, edit(|b| b.preview.as_mut().unwrap().id += 1)),
        ("literal".into(), code::TOKEN_MISMATCH, edit(|b| b.preview.as_mut().unwrap().literal = "0".repeat(64))),
        ("frame".into(), code::NO_FRESH_PREVIEW, edit(|b| b.preview.as_mut().unwrap().frame.sequence += 1)),
        ("deps.epoch".into(), code::REVISION_CHANGED, deps(|d| d.epoch += 1)),
        ("deps.identity".into(), code::REVISION_CHANGED, deps(|d| d.identity.push('x'))),
        ("deps.mode".into(), code::REVISION_CHANGED, deps(|d| d.mode = "jumble_on")),
        ("deps.state".into(), code::REVISION_CHANGED, deps(|d| d.state += 1)),
        ("deps.protection".into(), code::REVISION_CHANGED, deps(|d| d.protection += 1)),
        ("deps.draft".into(), code::REVISION_CHANGED, deps(|d| d.draft.push('x'))),
    ];
    let Value::Object(block) = identity_json(&kernel().identity()) else { panic!("the identity block is an object") };
    for field in block.keys() {
        let mut changed = block.clone();
        let value = changed.get_mut(field).unwrap();
        *value = match value {
            Value::String(text) => Value::String(format!("{text}-other")),
            _ => Value::String("other".into()),
        };
        let identity = digest(&Value::Object(changed));
        cases.push((format!("identity.{field}"), code::TOKEN_MISMATCH, edit(move |b| b.identity = identity.clone())));
    }
    assert_eq!(cases.len(), 19);
    for (nonce, (field, expected, change)) in (100..).zip(&cases) {
        let press = f.forge(nonce, |b| change(b));
        f.refused(press, expected, field);
    }
    let control = f.forge(99, |_| {});
    assert!(committed(&f.layer.invoke(&Caller::Shell, control)));
}

/// L10 (draft): every open and every restart draws a new random epoch, also with one injected secret, so an earlier
/// epoch's token or envelope is refused and nothing it names runs, and a new shell's command IDs never meet an earlier
/// shell's; CT-ONE-21 (part: a reopen from the in-memory store keeps the moves and revives no permission; not run: the
/// journal, which `c600-session/tests/kn5_command.rs` covers), KN5.21.
#[test]
fn one_injected_secret_never_repeats_an_epoch() {
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[1]))));
    let undo = f.step_press(Direction::Undo, false);
    let asked = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
    assert!(running(&f.shell.send(&mut f.layer, asked.clone())));
    let (mut epochs, mut sent) = (vec![1], f.shell.sent.clone());
    let mut store = f.layer.store().clone();
    for _ in 0..3 {
        let mut layer = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), store).unwrap();
        let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
        for restart in [false, true] {
            if restart {
                layer.restart(Secret::fixed(SECRET));
            }
            let epoch = layer.revision().epoch;
            assert!(!epochs.contains(&epoch), "epoch {epoch} repeats {epochs:?}");
            epochs.push(epoch);
            assert_eq!(layer.revision().state, 1);
            let before = (layer.revision().clone(), layer.store().log.len());
            // The undo token verifies under the same secret, but its envelope names the ended epoch; relabelled with
            // the current epoch under a new command ID, its binding still names the ended one (draft ruling 5). After
            // the restart, the refusals of the epoch before answer `unknown_outcome` (KN5.23).
            for envelope in [&undo, &asked] {
                let receipt = layer.invoke(&Caller::Shell, envelope.clone());
                if restart {
                    assert_eq!(receipt.status, Status::UnknownOutcome, "{}", envelope.row);
                } else {
                    assert_eq!(code_of(&receipt), code::EPOCH_CHANGED, "{}", envelope.row);
                }
            }
            let relabelled = Invocation { command: format!("{}/{epoch}", undo.command), epoch, ..undo.clone() };
            assert_eq!(code_of(&layer.invoke(&Caller::Shell, relabelled)), code::TOKEN_MISMATCH);
            assert_eq!(layer.run_jobs(), 0);
            assert_eq!((layer.revision().clone(), layer.store().log.len()), before);
            // A new shell's command IDs carry the epoch, so they meet no earlier shell's.
            let first = shell.prepare(&layer, "operation.preview", draft_args(&word(&[3]))).unwrap();
            assert!(!sent.contains(&first.command), "{} repeats", first.command);
            sent.push(first.command.clone());
            assert!(running(&shell.send(&mut layer, first)));
        }
        store = layer.store().clone();
    }
}

/// CT-KN5-04 (part: the runtime's and the shell's command IDs are separate spaces, and the request digest covers the
/// caller, so a runtime envelope under the shell's next command ID neither takes the ID nor reaches the shell as its
/// receipt; not run: the host's run IDs), L11 (b), ruling 4. The run is inside a command the solver invoked (finding
/// B2), which took `shell/1`, so the shell's next command ID is `shell/2`.
#[test]
fn the_runtime_and_the_shell_keep_separate_command_ids() {
    let mut f = fixture(&[]);
    let runtime = Caller::Runtime { run: f.solver_run() };
    let theirs = Invocation {
        command: "shell/2".into(),
        row: "operation.preview".into(),
        args: draft_args(&word(&[1])),
        token: None,
        epoch: 1,
        budget: None,
    };
    let proposed = f.layer.invoke(&runtime, theirs.clone());
    assert!(running(&proposed), "{:?}", proposed.status);
    assert_eq!(f.layer.run_jobs(), 1);
    let mine = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
    assert_eq!(mine.command, "shell/2");
    let pressed = f.shell.send(&mut f.layer, mine.clone());
    assert!(running(&pressed), "{:?}", pressed.status);
    assert_ne!((pressed.job, &pressed.request), (proposed.job, &proposed.request));
    assert_eq!(f.layer.run_jobs(), 1);
    let theirs_now = f.layer.kernel_outcome(&runtime, "shell/2", &proposed.request);
    assert!(matches!(theirs_now.status, Status::Completed(_)) && theirs_now.job == proposed.job);
    assert_eq!(f.layer.invoke(&runtime, theirs), theirs_now);
    let mine_now = f.layer.outcome("shell/2", &pressed.request).unwrap();
    assert!(matches!(mine_now.status, Status::Completed(_)) && mine_now.job == pressed.job);
    let crossed = f.layer.kernel_outcome(&Caller::Shell, "shell/2", &proposed.request);
    assert_eq!(code_of(&crossed), code::COMMAND_ID_REUSED);
}

/// CT-KN5-04 (part: a runtime envelope without the token its row takes does not authenticate, so its refusal is kept
/// for its own envelope and never takes the runtime's command ID; not run: the host's run IDs), ruling 4. The run is
/// inside a command the solver invoked (finding B2).
#[test]
fn a_runtime_envelope_without_its_rows_token_never_takes_the_command_id() {
    let mut f = fixture(&[]);
    let run = f.solver_run();
    let runtime = Caller::Runtime { run: run.clone() };
    let scramble = Invocation {
        command: format!("{run}/1"),
        row: "jumble.scramble".into(),
        args: json::object([("policy", Value::String("uniform".into()))]),
        token: None,
        epoch: f.layer.revision().epoch,
        budget: None,
    };
    let refused = f.layer.invoke(&runtime, scramble.clone());
    assert_eq!(code_of(&refused), code::TOKEN_INVALID);
    let proposal = Invocation { row: "operation.preview".into(), args: draft_args(&word(&[1])), ..scramble.clone() };
    let admitted = f.layer.invoke(&runtime, proposal);
    assert!(running(&admitted), "the command ID is still free: {:?}", admitted.status);
    assert_eq!(f.layer.invoke(&runtime, scramble), refused, "the refusal answers its own envelope again");
}

/// CT-KN5-10 (part: the layer's public functions that take `&mut self` are the allowlisted entry points, its test hooks
/// exist only in builds with debug assertions or the `test-hooks` feature, and the shell reaches the layer through
/// `invoke`, `present` and `presentation_unavailable` alone besides reads, and through `begin_step` for EV4.20's step
/// action, which writes a playback's pending entry before the shell sends the ordinary row (LP4-9); each row's delivery
/// against the restriction table runs whole in `tests/r05_rows.rs`), R-05, KN5.1, L15. The layer's sources are
/// `src/layer.rs` and its playback module `src/layer/playback.rs`.
#[test]
fn the_layer_has_no_public_mutator_beside_its_entry_points() {
    const HOOK: &str = "#[cfg(any(debug_assertions, feature = \"test-hooks\"))]";
    let sources = [include_str!("../src/layer.rs"), include_str!("../src/layer/playback.rs")];
    let lines: Vec<&str> = sources.iter().flat_map(|source| source.lines()).map(str::trim).collect();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("pub fn ") else { continue };
        let mut signature = String::new();
        for part in &lines[i..] {
            signature.push_str(part);
            if part.contains('{') {
                break;
            }
        }
        if signature.contains("&mut self") {
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            let attributes = lines[..i].iter().rev().take_while(|l| l.starts_with("#[") || l.starts_with("///"));
            let hook = attributes.clone().any(|l| *l == HOOK);
            found.push((name, hook));
        }
    }
    let expected = [
        ("register_overlay", false),
        // KN5.3's registration with the label and the granted capabilities (EV4.1; LP4-4).
        ("register_overlay_with", false),
        ("serve_row", false),
        ("declare_overlay", false),
        ("store_mut", true),
        ("set_program_capacity", false),
        ("set_budget_ceiling", false),
        ("present", false),
        // RM.26's other adapter report: presentation cannot be confirmed.
        ("presentation_unavailable", false),
        // ADR-D01.9's settle interval (owner, 22 November 2026, item 4): the host's clock and the tuned interval.
        ("set_clock", false),
        ("set_settle", false),
        ("set_provenance", false),
        ("set_views", false),
        ("pin_menu", false),
        ("hold_commit_lane", true),
        ("fault_replace_head", true),
        ("restart", false),
        ("invoke", false),
        // KN5.1's `invoke` with the host's blocks for the arguments (BND.22), beside the envelope.
        ("invoke_with_inputs", false),
        ("run_jobs", false),
        ("take_program_job", false),
        ("finish_program_job", false),
        ("finish_program_job_with", false),
        // LP4-9 (`src/layer/playback.rs`): the host's recorder and module versions, and EV4.20's step action.
        ("set_recorder", false),
        ("set_module_versions", false),
        ("begin_step", false),
    ];
    let expected: Vec<(String, bool)> = expected.iter().map(|(name, hook)| ((*name).to_owned(), *hook)).collect();
    assert_eq!(found, expected);

    let shell = include_str!("../src/shell.rs");
    let mut calls: Vec<&str> = shell
        .match_indices("layer.")
        .map(|(i, _)| {
            let rest = &shell[i + "layer.".len()..];
            &rest[..rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len())]
        })
        .filter(|name| !name.is_empty())
        .collect();
    calls.sort_unstable();
    calls.dedup();
    // `layout_hint` is the layout history's hint (ADR-D01.14), read as `undo_hint` is.
    // `step_arguments` and `step_draft` read the step a playback waits at (EV4.20, LP4-9).
    let reads =
        ["frame", "identity_binding", "layout_hint", "revision", "step_arguments", "step_draft", "table", "undo_hint"];
    let mut expected = vec!["begin_step", "invoke", "present", "presentation_unavailable"];
    expected.extend(reads);
    expected.sort_unstable();
    assert_eq!(calls, expected);
}

/// CT-ONE-11 (part: when two commit steps fail together the earlier step's code answers and nothing changes, for each
/// pair of adjacent steps from 3 to 8 and for steps 3 and 8; every pair runs in the next test), CT-KN5-06 (part: a protection
/// release and its undo restore the set, and a press from before them is still stale; a record edit and its undo are
/// in `kn5_stages.rs`), L18 (a), L18 (d), L18 (e).
#[test]
fn the_earliest_failing_step_answers() {
    let (clear, moves) = o06();
    let mut f = fixture(&[1]);

    // Steps 3, 4 and 8: a forged authenticator while the lane is held, for a preview that moves a protected orbit.
    let m = f.preview(&moves[0]);
    f.present(m);
    let mut forged = f.press(m);
    forged.token = Some(Token::mint(&Secret::fixed([9; 32]), forged.token.unwrap().binding));
    f.layer.hold_commit_lane(true);
    f.refused(forged, code::TOKEN_INVALID, "steps 3, 4 and 8");
    // Steps 4 and 5: the lane is held and the token binds other arguments.
    let mut other = f.press(m);
    other.args = preview_args(m + 7);
    f.refused(other, code::COMMIT_IN_FLIGHT, "steps 4 and 5");
    f.layer.hold_commit_lane(false);

    // Steps 5 and 6: a protection change made the preview stale, then another preview replaced it.
    let press = f.press(m);
    assert!(matches!(f.protect("[1]", false).status, Status::Completed(_)));
    let q = f.preview(&clear[0]);
    f.refused(press, code::PREVIEW_REPLACED, "steps 5 and 6");

    // Steps 6 and 7: a stale preview and a head that differs from its start.
    f.present(q);
    let press = f.press(q);
    assert!(matches!(f.protect("[1]", false).status, Status::Completed(_)));
    let saved = f.layer.snapshot().clone();
    f.layer.fault_replace_head(derived(&saved, &word(&[3])));
    f.refused(press, code::REVISION_CHANGED, "steps 6 and 7");
    f.layer.fault_replace_head(saved.clone());

    // Steps 7 and 8: a head that differs, for a preview that moves a protected orbit.
    let m = f.preview(&moves[0]);
    f.present(m);
    let press = f.press(m);
    f.layer.fault_replace_head(derived(&saved, &word(&[3])));
    f.refused(press, code::STATE_HASH_MISMATCH, "steps 7 and 8");
    f.layer.fault_replace_head(saved);

    // Steps 6 and 8, and L18 (d): a release and its undo leave the set as it was; the press is still stale.
    let press = f.press(m);
    assert!(matches!(f.protect("[]", true).status, Status::Completed(_)));
    assert!(committed(&f.step(Direction::Undo, false)));
    assert_eq!(f.layer.protected().orbits().to_vec(), vec![1]);
    f.refused(press, code::REVISION_CHANGED, "steps 6 and 8");

    // Step 8 alone answers once the preview is fresh.
    let m = f.preview(&moves[0]);
    f.present(m);
    let press = f.press(m);
    f.refused(press, code::ORBIT_WOULD_MOVE, "step 8");

    // L18 (e): no row that takes a token repeats on a held key.
    for row in f.layer.table().rows().filter(|row| row.takes_token()) {
        assert!(!row.repeat, "{} takes a token and repeats", row.id);
    }
}

/// CT-ONE-11 (whole): a commit row refused at any step of section 7 from 3 to 8, alone or with any one other step,
/// answers the earliest failing step's code and leaves the state digest, the revision, the protected set and the log
/// unchanged. Each case starts from a fresh session under O06's protected set, presents one preview, prepares its
/// press, then breaks it at the case's steps: 3 a forged authenticator, 4 a held commit lane, 5 other arguments than
/// the token binds, 6 a protection change after the press, 7 a head that differs from the preview's start, and 8 a
/// preview that moves a protected orbit. Steps 1 and 2 are the shell's (`c600-input/tests/routing.rs`), and the
/// session's own gates are in `c600-session/tests/commit.rs`.
#[test]
fn every_failing_step_and_pair_of_steps_answers_the_earliest_step() {
    let (clear, moves) = o06();
    let codes = [
        (3, code::TOKEN_INVALID),
        (4, code::COMMIT_IN_FLIGHT),
        (5, code::TOKEN_MISMATCH),
        (6, code::REVISION_CHANGED),
        (7, code::STATE_HASH_MISMATCH),
        (8, code::ORBIT_WOULD_MOVE),
    ];
    let mut cases: Vec<Vec<u8>> = (3..=8).map(|step| vec![step]).collect();
    for first in 3..=8u8 {
        cases.extend((first + 1..=8).map(|second| vec![first, second]));
    }
    assert_eq!(cases.len(), 6 + 15);
    for steps in cases {
        let what = format!("steps {steps:?}");
        let mut f = fixture(&[1]);
        let p = f.preview(if steps.contains(&8) { &moves[0] } else { &clear[0] });
        f.present(p);
        let mut press = f.press(p);
        if steps.contains(&3) {
            press.token = Some(Token::mint(&Secret::fixed([9; 32]), press.token.unwrap().binding));
        }
        if steps.contains(&5) {
            press.args = preview_args(p + 7);
        }
        if steps.contains(&6) {
            assert!(matches!(f.protect("[1]", false).status, Status::Completed(_)), "{what}");
        }
        if steps.contains(&7) {
            let head = derived(f.layer.snapshot(), &word(&[3]));
            f.layer.fault_replace_head(head);
        }
        if steps.contains(&4) {
            f.layer.hold_commit_lane(true);
        }
        let expected = codes.iter().find(|(step, _)| *step == steps[0]).unwrap().1;
        f.refused(press, expected, &what);
    }
}
