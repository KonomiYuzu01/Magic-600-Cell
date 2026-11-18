//! c600-command's KN5 layer over KN4's journal ([`JournalStore`]): durable commits, the reopen that replays them through
//! the kernel, a fresh random epoch per open, and the outcome rules across a reopen. Each test names the conformance
//! rows it covers headless, with PHS.62 markers; Windows, display and performance parts are not run here.

mod common;

use c600_command::{
    code, draft_args, Caller, CommandLayer, Completion, CompletionKind, Direction, Error, HeadlessFrames, Invocation,
    Kind, Overlay, Provenance, Receipt, Reopen, Revision, Secret, Shell, Status, Table, POLICY_ROW, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{ProtectedSet, ProtectionMode};
use c600_session::disk::{Fault, MemDisk};
use c600_session::JournalStore;

const SECRET: [u8; 32] = [7; 32];

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}

fn word(letters: &[i64]) -> Value {
    parse(&format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#))
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

struct Fixture {
    layer: CommandLayer<JournalStore>,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A new session at the solved state on an empty memory disk.
fn create(disk: &MemDisk) -> Fixture {
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let layer = CommandLayer::new(k.clone(), Table::h08(), Secret::fixed(SECRET), store, "s1", k.solved(), none());
    assert!(layer.recovery().is_none(), "{:?}", layer.recovery());
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// The session the disk holds, reopened under a new layer and shell with `secret`, as a restart
/// (`MemDisk::end_holder`).
fn reopen(disk: &MemDisk, secret: [u8; 32]) -> Fixture {
    disk.end_holder();
    let store = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let layer = match CommandLayer::open(common::kernel(), Table::h08(), Secret::fixed(secret), store) {
        Ok(layer) => layer,
        Err(error) => panic!("the session does not reopen: {error:?}"),
    };
    Fixture { layer, shell: Shell::new(Secret::fixed(secret), "shell"), frames: HeadlessFrames::new() }
}

fn none() -> ProtectedSet {
    ProtectedSet::none()
}

impl Fixture {
    /// Previews `recipe`, runs the job and presents the preview; returns the preview receipt and its ID.
    fn preview(&mut self, recipe: &Value) -> (Receipt, u64) {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        let done = self.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert!(matches!(done.status, Status::Completed(Completion { kind: CompletionKind::PreviewReady, .. })));
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        (done, id)
    }

    /// Previews, presents and commits `recipe`; returns the commit's envelope and receipt.
    fn commit(&mut self, recipe: &Value) -> (Invocation, Receipt) {
        let (_, id) = self.preview(recipe);
        let args = json::object([("preview", Value::Int(id as i64))]);
        let press = self.shell.prepare(&self.layer, "operation.execute", args).unwrap();
        let receipt = self.shell.send(&mut self.layer, press.clone());
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
        (press, receipt)
    }

    fn step(&mut self, direction: Direction, confirm: bool) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        let mut args = vec![("hint", hint.to_json())];
        if confirm {
            args.push(("confirm", Value::Bool(true)));
        }
        let args = Value::Object(args.into_iter().map(|(k, v)| (k.to_owned(), v)).collect());
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    fn protect(&mut self, orbits: &str) -> Receipt {
        let args = json::object([("orbits", parse(orbits))]);
        self.shell.invoke(&mut self.layer, "protection.orbit.set", args).unwrap()
    }

    /// The revision without its epoch, the state digest, the protected orbits and both action hints.
    fn kept(&self) -> String {
        let mut revision = self.layer.revision().clone();
        revision.epoch = 0;
        format!(
            "{revision:?} {} {:?} {:?} {:?}",
            self.layer.snapshot().digest(),
            self.layer.protected().orbits(),
            self.layer.undo_hint(Direction::Undo),
            self.layer.undo_hint(Direction::Redo)
        )
    }
}

/// CT-ONE-21 (part: a reopen from the journal keeps the revision, the move tree's undo and redo stacks and the durable
/// receipts, answers `unknown_outcome` for the transient ones and revives no token; not run: drafts and handles, which
/// this layer does not hold), CT-KN5-04 (part: a retried delivery after a reopen answers its stored receipt and commits
/// nothing, and the same command ID with other content is refused; not run: the shell's transport), KN5.23.
#[test]
fn a_reopened_session_keeps_its_moves_stacks_and_receipts() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    let (press_a, a) = f.commit(&word(&[1]));
    let (built, id) = f.preview(&word(&[2]));
    let press_b =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))])).unwrap();
    let b = f.shell.send(&mut f.layer, press_b);
    assert!(matches!(b.status, Status::Committed(_)));
    let after_b = f.layer.snapshot().digest().to_owned();
    let u = f.step(Direction::Undo, false);
    assert!(matches!(u.status, Status::Committed(_)), "{:?}", u.status);
    // A press prepared for a pending preview and never delivered.
    let (_, pending) = f.preview(&word(&[3]));
    let early = f
        .shell
        .prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(pending as i64))]))
        .unwrap();
    let kept = f.kept();
    let epoch = f.layer.revision().epoch;
    drop(f);

    let mut g = reopen(&disk, [9; 32]);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(g.kept(), kept);
    assert!(g.layer.revision().epoch != epoch && g.layer.revision().epoch != 1);
    for receipt in [&a, &b, &u] {
        assert_eq!(g.layer.outcome(&receipt.command, &receipt.request).as_ref(), Some(receipt));
    }
    // The preview's receipts were transient: the reopened layer knows nothing of them (LI.30).
    let lost = g.layer.kernel_outcome(&Caller::Shell, &built.command, &built.request);
    assert_eq!(lost.status, Status::UnknownOutcome);
    // A retried delivery of A answers its receipt and changes nothing; other content under A's ID is refused.
    let before = g.kept();
    assert_eq!(g.layer.invoke(&Caller::Shell, press_a.clone()), a);
    let mut reused = press_a.clone();
    reused.args = json::object([("preview", Value::Int(99))]);
    assert_eq!(code_of(&g.layer.invoke(&Caller::Shell, reused)), code::COMMAND_ID_REUSED);
    assert_eq!(g.layer.outcome(&a.command, &a.request).as_ref(), Some(&a));
    assert_eq!(g.kept(), before);
    // A press of the earlier open, delivered now, is an unknown envelope of another epoch: nothing runs.
    assert_eq!(early.epoch, epoch);
    assert_eq!(code_of(&g.layer.invoke(&Caller::Shell, early)), code::EPOCH_CHANGED);
    assert_eq!(g.kept(), before);
    // The stacks replayed: the redo reaches B's state again, and the new shell's command IDs name the new epoch.
    let r = g.step(Direction::Redo, false);
    assert!(matches!(r.status, Status::Committed(_)), "{:?}", r.status);
    assert!(r.command.starts_with(&format!("shell/{}/", g.layer.revision().epoch)), "{}", r.command);
    assert_eq!(g.layer.snapshot().digest(), after_b);
    let kept = g.kept();
    drop(g);
    let h = reopen(&disk, [9; 32]);
    assert_eq!(h.kept(), kept);
    assert_eq!(h.layer.outcome(&r.command, &r.request), Some(r));
}

/// CT-ONE-21 (part: the protected set and its undo through the original permission path survive a reopen; not run:
/// the shell's confirmation dialog), LI.26 (draft).
#[test]
fn a_protection_change_is_durable_and_its_release_needs_confirmation() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    f.commit(&word(&[1]));
    let set = f.protect("[1]");
    assert!(matches!(set.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
    assert_eq!(f.layer.revision().protection, 1);
    let kept = f.kept();
    drop(f);
    let mut g = reopen(&disk, SECRET);
    assert_eq!(g.kept(), kept);
    assert_eq!(g.layer.outcome(&set.command, &set.request), Some(set));
    // Undoing the change releases the orbit: it needs the solver's confirmation (LI.26).
    assert_eq!(code_of(&g.step(Direction::Undo, false)), code::CONFIRMATION_REQUIRED);
    let released = g.step(Direction::Undo, true);
    assert!(matches!(released.status, Status::Committed(_)), "{:?}", released.status);
    assert!(g.layer.protected().orbits().is_empty());
    let kept = g.kept();
    drop(g);
    let h = reopen(&disk, SECRET);
    assert_eq!(h.kept(), kept);
    assert!(h.layer.undo_hint(Direction::Redo).is_some());
}

/// CT-ONE-21 (part: the protection policy, its change's receipt and its undo survive a reopen from the journal, so a
/// reopened session judges under the policy the live one had and redoes the change), KN5.20, LB2.3 (draft).
#[test]
fn a_policy_change_and_its_undo_are_durable_in_the_journal() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    f.commit(&word(&[1]));
    assert_eq!(f.layer.policy(), ProtectionMode::Net);
    let args = json::object([("confirm", Value::Bool(true)), ("policy", Value::String("strict".into()))]);
    let set = f.shell.invoke(&mut f.layer, POLICY_ROW, args).unwrap();
    assert!(matches!(set.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
    let kept = (f.kept(), f.layer.policy());
    assert_eq!(kept.1, ProtectionMode::Strict);
    drop(f);
    let mut g = reopen(&disk, SECRET);
    assert_eq!((g.kept(), g.layer.policy()), kept);
    assert_eq!(g.layer.outcome(&set.command, &set.request), Some(set));
    let undone = g.step(Direction::Undo, false);
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
    assert_eq!(g.layer.policy(), ProtectionMode::Net);
    let kept = (g.kept(), g.layer.policy());
    drop(g);
    let h = reopen(&disk, SECRET);
    assert_eq!((h.kept(), h.layer.policy()), kept);
    assert!(h.layer.undo_hint(Direction::Redo).is_some());
}

/// L10 (draft): every open draws a fresh random epoch the session never had, also with the same injected secret; a
/// token and an envelope of an earlier open do nothing, and the shell's command IDs do not repeat across opens.
#[test]
fn every_open_draws_a_new_epoch() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    f.commit(&word(&[1]));
    let hint = f.layer.undo_hint(Direction::Undo).unwrap();
    let undo = f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    let query = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
    let first = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap().command;
    drop(f);
    let mut epochs = vec![1];
    let mut ids = vec![first];
    for _ in 0..3 {
        let mut g = reopen(&disk, SECRET);
        let epoch = g.layer.revision().epoch;
        assert!(!epochs.contains(&epoch), "{epoch} repeats");
        epochs.push(epoch);
        let before = g.kept();
        for envelope in [undo.clone(), query.clone()] {
            let receipt = g.layer.invoke(&Caller::Shell, envelope);
            assert!(matches!(receipt.status, Status::Refused(_)), "{:?}", receipt.status);
        }
        assert_eq!(g.kept(), before);
        let id = g.shell.prepare(&g.layer, "operation.preview", draft_args(&word(&[2]))).unwrap().command;
        assert!(!ids.contains(&id), "{id} repeats");
        ids.push(id);
        drop(g);
        let mut store = JournalStore::open(Box::new(disk.clone())).unwrap();
        let history = store.history().unwrap();
        assert_eq!(history.epochs, epochs[1..]);
    }
}

/// CT-KN4-02 (part: an append that fails and is cut back is rolled back, and the session stays as before, also after a
/// reopen; not run: every write index of every row), CT-ONE-22 (part: an append whose fate is unknown puts the layer
/// into read-only recovery, and the journal decides the fate at the restart and at the reopen), KN5.23.
#[test]
fn a_failed_append_rolls_back_and_an_unknown_fate_is_settled_by_the_journal() {
    for case in ["rolled back", "unknown, absent", "unknown, written"] {
        let disk = MemDisk::new();
        let mut f = create(&disk);
        f.commit(&word(&[1]));
        let start = f.kept();
        let (_, id) = f.preview(&word(&[2]));
        let press =
            f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))])).unwrap();
        let next = disk.writes();
        match case {
            "rolled back" => disk.fail_write(next, Fault::Error),
            "unknown, absent" => disk.fail_from(next),
            _ => {
                disk.fail_write(next + 1, Fault::Persisted);
                disk.fail_write(next + 2, Fault::Error);
            }
        }
        let receipt = f.shell.send(&mut f.layer, press.clone());
        assert_eq!(f.kept(), start, "{case}");
        if case == "rolled back" {
            assert!(matches!(receipt.status, Status::Failed(_)), "{case}: {:?}", receipt.status);
            assert!(f.layer.recovery().is_none(), "{case}");
            drop(f);
            assert_eq!(reopen(&disk, SECRET).kept(), start, "{case}");
            continue;
        }
        assert_eq!(receipt.status, Status::UnknownOutcome, "{case}");
        assert_eq!(f.layer.recovery().unwrap().code, code::COMMIT_FATE, "{case}");
        let refused = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[3]))).unwrap();
        assert_eq!(code_of(&refused), code::READ_ONLY_RECOVERY, "{case}");
        disk.heal();
        f.layer.restart(Secret::fixed(SECRET));
        f.shell.restart(Secret::fixed(SECRET));
        assert!(f.layer.recovery().is_none(), "{case}");
        let settled = f.layer.kernel_outcome(&Caller::Shell, &receipt.command, &receipt.request);
        let written = case == "unknown, written";
        assert_eq!(matches!(settled.status, Status::Committed(_)), written, "{case}: {:?}", settled.status);
        assert_eq!(f.kept() != start, written, "{case}");
        let kept = f.kept();
        disk.crash();
        drop(f);
        let g = reopen(&disk, SECRET);
        assert_eq!(g.kept(), kept, "{case}");
        let found = g.layer.kernel_outcome(&Caller::Shell, &receipt.command, &receipt.request);
        assert_eq!(found, settled, "{case}");
    }
}

/// CT-ONE-22 (part: a restart that cannot read the journal back keeps an unknown fate unknown and stays in read-only
/// recovery, so no later action takes the unknown one's place in the order; a later restart adopts it, and the reopen
/// replays every committed action once; not run: a real disk's read errors), KN5.23.
#[test]
fn a_restart_that_cannot_read_the_journal_keeps_an_unknown_fate_unknown() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    f.commit(&word(&[1]));
    let (_, id) = f.preview(&word(&[2]));
    let press =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))])).unwrap();
    // The action's frame is written and synced, the sync still reports an error, and the cut back fails.
    let next = disk.writes();
    disk.fail_write(next + 1, Fault::Persisted);
    disk.fail_write(next + 2, Fault::Error);
    let receipt = f.shell.send(&mut f.layer, press);
    assert_eq!(receipt.status, Status::UnknownOutcome);
    // The restart reads the journal back and syncs it; that sync fails, so the store cannot tell the fate.
    disk.fail_write(disk.writes(), Fault::Error);
    f.layer.restart(Secret::fixed(SECRET));
    f.shell.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.recovery().map(|e| e.code), Some(code::COMMIT_FATE), "the fate stays unknown");
    let refused = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[3]))).unwrap();
    assert_eq!(code_of(&refused), code::READ_ONLY_RECOVERY);
    let unsettled = f.layer.kernel_outcome(&Caller::Shell, &receipt.command, &receipt.request);
    assert_eq!(unsettled.status, Status::UnknownOutcome);
    // The next restart reads the journal, which holds the action, and adopts it.
    f.layer.restart(Secret::fixed(SECRET));
    f.shell.restart(Secret::fixed(SECRET));
    assert!(f.layer.recovery().is_none(), "{:?}", f.layer.recovery());
    let settled = f.layer.kernel_outcome(&Caller::Shell, &receipt.command, &receipt.request);
    assert!(matches!(settled.status, Status::Committed(_)), "{:?}", settled.status);
    f.commit(&word(&[3]));
    let kept = f.kept();
    disk.crash();
    drop(f);
    let g = reopen(&disk, SECRET);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(g.kept(), kept, "every committed action replays once");
    assert_eq!(g.layer.kernel_outcome(&Caller::Shell, &receipt.command, &receipt.request), settled);
}

/// CT-ONE-22 (part: a damaged journal opens in read-only recovery after the last action that reads, which refuses
/// proposals and commits and answers reads; not run: a runtime's reads), KN5.23.
#[test]
fn a_damaged_journal_opens_in_read_only_recovery() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    let (_, a) = f.commit(&word(&[1]));
    let after_a = f.layer.snapshot().digest().to_owned();
    f.commit(&word(&[2]));
    drop(f);
    // A bit inside the last frame's payload (its checksum is the frame's last 32 bytes).
    let mut bytes = disk.durable("journal").unwrap();
    let inside = bytes.len() - 40;
    bytes[inside] ^= 0x20;
    disk.overwrite("journal", &bytes);
    let mut g = reopen(&disk, SECRET);
    assert_eq!(g.layer.recovery().unwrap().code, code::READ_ONLY_RECOVERY);
    assert_eq!(g.layer.snapshot().digest(), after_a);
    assert_eq!(g.layer.outcome(&a.command, &a.request), Some(a));
    let refused = g.shell.invoke(&mut g.layer, "operation.preview", draft_args(&word(&[3]))).unwrap();
    assert_eq!(code_of(&refused), code::READ_ONLY_RECOVERY);
    assert_eq!(g.layer.availability("session.undo").unwrap_err().code, code::READ_ONLY_RECOVERY);
    assert_eq!(disk.durable("journal").unwrap(), bytes, "nothing is written after the damage");
}

/// The store's own refusals: a disk that holds a journal takes no new session, and an empty one holds none to open.
#[test]
fn a_store_refuses_a_second_session_and_opens_no_empty_one() {
    let disk = MemDisk::new();
    create(&disk);
    let again = JournalStore::create(Box::new(disk.clone())).err().unwrap();
    assert_eq!(again.code, c600_session::code::PERSISTENCE_FORMAT);
    let empty = JournalStore::open(Box::new(MemDisk::new())).unwrap();
    let refused = CommandLayer::open(common::kernel(), Table::h08(), Secret::fixed(SECRET), empty).err().unwrap();
    assert_eq!(refused.code, c600_session::code::UNKNOWN_SESSION);
}

/// CT-KN5-04 (part: a workspace overlay's completion is written with its receipt in one journal frame, so after a reopen
/// its lookup and its identical retry answer the same `completed(workspace)` and the retry writes nothing; a record
/// write is the next test), KN5.23, L8.
#[test]
fn a_workspace_completion_and_its_receipt_are_one_durable_frame() {
    let disk = MemDisk::new();
    let mut f = create(&disk);
    let overlay = Overlay { module: "tools/full".into(), entry: "note".into() };
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay).unwrap();
    let asked = f.shell.prepare(&f.layer, "tool.full.note", json::object([])).unwrap();
    let started = f.shell.send(&mut f.layer, asked.clone());
    let Status::Running(job) = started.status else { panic!("{:?}", started.status) };
    assert_eq!(f.layer.take_program_job().unwrap().job, job);
    let journal = disk.durable("journal").unwrap().len();
    let done = f.layer.finish_program_job(job, Ok(Value::String("note".into()))).unwrap();
    assert!(matches!(done.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
    assert!(disk.durable("journal").unwrap().len() > journal);
    let kept = f.kept();
    drop(f);
    let mut g = reopen(&disk, SECRET);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(g.kept(), kept);
    assert_eq!(g.layer.revision().workspace, 1);
    assert_eq!(g.layer.outcome(&asked.command, &done.request), Some(done.clone()));
    let writes = disk.writes();
    assert_eq!(g.layer.invoke(&Caller::Shell, asked), done);
    assert_eq!(disk.writes(), writes);
}

/// CT-KN5-04 (part: a record write and its undo are written to the journal with their receipts, so a reopen replays them
/// without a provenance reader to the same record, version and revision, and the write's lookup and its identical retry
/// answer the same `completed(workspace)` and write nothing; not run: the shell's transport), KN5.20, KN5.23, PS1.16.
#[test]
fn a_record_write_and_its_undo_are_durable_in_the_journal() {
    struct Reader;
    impl Provenance for Reader {
        fn check(&self, _: &[u8]) -> Result<(), Error> {
            Ok(())
        }
        fn admit(&self, _: &[u8]) -> Result<(), Error> {
            Ok(())
        }
    }
    let disk = MemDisk::new();
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k.clone(), table.clone(), Secret::fixed(SECRET), store, "s1", k.solved(), none());
    layer.set_provenance(Box::new(Reader));
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let args = json::object([
        ("payload", Value::Array(vec![Value::Int(1), Value::Int(2)])),
        ("provenance", Value::String("block".into())),
        ("record", Value::String("w".into())),
    ]);
    let write = shell.prepare(&layer, RECORD_ROW, args).unwrap();
    let written = shell.send(&mut layer, write.clone());
    assert!(matches!(written.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. })));
    let hint = layer.undo_hint(Direction::Undo).expect("the write is undoable");
    let undone = shell.invoke(&mut layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
    let records = layer.records().clone();
    assert_eq!((records["w"].value.as_ref(), records["w"].version), (None, 2));
    let revision = Revision { epoch: 0, ..layer.revision().clone() };
    drop((layer, shell));

    let store = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let mut reopened = match CommandLayer::open(common::kernel(), table, Secret::fixed([9; 32]), store) {
        Ok(layer) => layer,
        Err(error) => panic!("the session does not reopen: {error:?}"),
    };
    assert!(reopened.recovery().is_none(), "{:?}", reopened.recovery());
    assert_eq!(reopened.records(), &records);
    assert_eq!(Revision { epoch: 0, ..reopened.revision().clone() }, revision);
    assert_eq!(reopened.outcome(&write.command, &written.request), Some(written.clone()));
    let writes = disk.writes();
    assert_eq!(reopened.invoke(&Caller::Shell, write), written);
    assert_eq!((disk.writes(), reopened.records()), (writes, &records));
}

/// CT-KN5-04 (part: M6's own `Session::invoke` keeps the command layer's rule: an envelope with a forged authenticator,
/// or the shell's genuine envelope delivered on a program's channel, sent first under the shell's command ID, is
/// refused, keeps its refusal for the epoch and never takes the ID, so the genuine press on the shell's channel then
/// commits once, and each delivery's retry answers its own receipt; not run: the shell's transport), KN5.21, KN5.23,
/// PHS.52 ruling 4: the host stamps the origin from the channel, and the envelope names none, so the same envelope is a
/// commit row a program may not deliver on one channel and the solver's press on the other. The control case sends no
/// first envelope.
#[test]
fn m6_invoke_keeps_the_rule_for_an_envelope_that_does_not_authenticate() {
    use c600_session::{code as m6, Envelope, Origin, Row};
    for attack in ["forged", "program", "control"] {
        let (session, disk) = common::session();
        let preview = common::propose(&session, "p", common::literal(&[1]));
        let row = Row::Execute { preview };
        let mut genuine = common::envelope(&session, "C", row.clone());
        genuine.token = Some(session.shell().mint("C", &row).expect("the stub mints for the pending preview"));
        let first = match attack {
            "forged" => {
                let mut forged = genuine.clone();
                forged.token.as_mut().expect("a token").mac[0] ^= 1;
                Some((Origin::Shell, forged))
            }
            "program" => Some((Origin::Program, genuine.clone())),
            _ => None,
        };
        let (before, writes) = (session.overview(), disk.writes());
        let refusal = first.clone().map(|(origin, first)| {
            let receipt = session.invoke(origin, first);
            let expected = if attack == "forged" { m6::TOKEN_INVALID } else { m6::COMMIT_ROW_REFUSED };
            assert_eq!(common::code(&receipt), expected, "{attack}");
            assert_eq!(session.outcome("C", &receipt.request), receipt, "{attack}: the refusal is kept for the epoch");
            receipt
        });
        assert_eq!((session.overview(), disk.writes()), (before.clone(), writes), "{attack}: nothing changes");
        let receipt = session.invoke(Origin::Shell, genuine.clone());
        common::committed(&receipt);
        assert_eq!(session.outcome("C", &receipt.request), receipt, "{attack}");
        let (after, writes) = (session.overview(), disk.writes());
        assert_eq!(after.revision.state, before.revision.state + 1, "{attack}");
        assert_eq!(session.invoke(Origin::Shell, genuine.clone()), receipt, "{attack}: the retry answers the receipt");
        if let (Some((origin, first)), Some(refusal)) = (first, refusal) {
            // The forged envelope differs in its token; the program's delivery differs only in the stamped origin.
            let covers = if attack == "program" { "the stamped origin" } else { "the token" };
            assert_ne!(refusal.request, receipt.request, "{attack}: the request digest covers {covers}");
            assert_eq!(session.invoke(origin, first), refusal, "{attack}: the first delivery's retry, its refusal");
        }
        let other = Envelope { row: Row::Execute { preview: preview + 1 }, ..genuine.clone() };
        assert_eq!(common::code(&session.invoke(Origin::Shell, other)), m6::COMMAND_ID_REUSED, "{attack}");
        assert_eq!((session.overview(), disk.writes()), (after, writes), "{attack}: one commit");
    }
}
