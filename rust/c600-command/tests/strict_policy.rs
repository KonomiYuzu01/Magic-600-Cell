//! The protection policy in the command layer (draft; KN2.4, KN2.7, LB2.3, LI.26): `protection.policy.set` switches a
//! session between Net, which judges the end of the complete operation, and Strict, which judges every executed literal
//! prefix. Each test names the conformance rows it covers headless with PHS.62 markers. Where the frozen text is silent,
//! the test follows the draft rulings recorded in `src/lib.rs` and says so.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, policy_from_name, policy_name, Caller, CommandLayer, Completion, CompletionKind, Direction,
    Error, HeadlessFrames, Invocation, MemoryStore, PreviewState, Receipt, Secret, Shell, Status, Table, Transaction,
    TransactionKind, POLICY_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, ProtectionMode, Verdict};

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

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects `orbits`; it starts under Net.
fn fixture(orbits: &[i64]) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(orbits).unwrap());
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// A reopen of `layer`'s store, which replays its journal.
fn reopen(layer: &CommandLayer) -> Fixture {
    let store = layer.store().clone();
    let layer = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// What a reopen must restore: the policy, the head, the state, the digest and the workspace and protection revisions.
fn restored(layer: &CommandLayer) -> (ProtectionMode, String, u64, String, u64, u64) {
    let r = layer.revision();
    (layer.policy(), r.head.clone(), r.state, r.digest.clone(), r.workspace, r.protection)
}

impl Fixture {
    /// `protection.policy.set` with `policy` through the command route.
    fn set_policy(&mut self, policy: &str) -> Receipt {
        self.shell
            .invoke(
                &mut self.layer,
                POLICY_ROW,
                json::object([("confirm", Value::Bool(true)), ("policy", text(policy))]),
            )
            .unwrap()
    }

    /// Previews `recipe` and runs the job; returns the preview's ID.
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

    /// The undo or redo of the action on top of its stack; no action here releases an orbit, so none confirms.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    /// What no refusal may change: the state digest, the revision, the durable log and the policy.
    fn fixed(&self) -> (String, String, usize, ProtectionMode) {
        let r = self.layer.revision();
        (r.digest.clone(), format!("{r:?}"), self.layer.store().log.len(), self.layer.policy())
    }

    /// Delivers `invocation` on `caller`'s channel, which must be refused with `expected` and change nothing.
    fn refused_on(&mut self, caller: &Caller, invocation: Invocation, expected: &str, what: &str) -> Error {
        let before = self.fixed();
        let receipt = self.layer.invoke(caller, invocation);
        assert_eq!(self.fixed(), before, "{what}");
        match receipt.status {
            Status::Refused(error) if error.code == expected => error,
            status => panic!("{what}: expected {expected}, got {status:?}"),
        }
    }

    fn refused(&mut self, invocation: Invocation, expected: &str, what: &str) -> Error {
        self.refused_on(&Caller::Shell, invocation, expected, what)
    }
}

/// CT-LB2-02 (part: the command route on 600-cell-Full; the kernel half is in `c600-kernel/tests/kernel.rs`; not run:
/// the captured lock, LB2.3's coverage report and `LB2:protection.incomplete_coverage`, which are LP3's library rows),
/// KN2.4, KN2.7, KN5.9, LB2.3. A session starts under Net. With an orbit protected, a word with an identity net map and
/// a moving prefix commits under Net; after `protection.policy.set` to Strict, a workspace action that keeps the state,
/// the same word builds a preview whose summary names the mode and the first refused prefix, and its commit is refused
/// `ORBIT_WOULD_MOVE` with nothing changed. Without a protected orbit Strict commits the word.
#[test]
fn ct_lb2_02_strict_refuses_in_the_route_a_word_that_net_commits() {
    let k = kernel();
    let identity = word(&[1, -1]);
    let mut f = fixture(&[0]);
    assert_eq!(f.layer.policy(), ProtectionMode::Net);

    // Under Net the word's net map moves no sticker, so its preview is clear without the Strict keys, and it commits.
    let p = f.preview(&identity);
    let summary = f.layer.preview().unwrap().summary();
    assert_eq!(
        (summary.get("verdict"), summary.get("net_moved_stickers")),
        (Some(&text("clear")), Some(&Value::Int(0)))
    );
    assert_eq!((summary.get("mode"), summary.get("first_refused_prefix")), (None, None));
    let start = f.layer.revision().digest.clone();
    f.present(p);
    let press = f.press(p);
    let receipt = f.shell.send(&mut f.layer, press);
    assert!(committed(&receipt), "{:?}", receipt.status);
    assert_eq!(f.layer.revision().digest, start);

    // The switch to Strict answers completed(workspace), advances the workspace and protection revisions, keeps the
    // state and journals the policy.
    let before = f.layer.revision().clone();
    let switched = f.set_policy("strict");
    assert!(completed(&switched), "{:?}", switched.status);
    let after = f.layer.revision().clone();
    assert_eq!((&after.head, after.state, &after.digest), (&before.head, before.state, &before.digest));
    assert_eq!((after.workspace, after.protection), (before.workspace + 1, before.protection + 1));
    assert_eq!(f.layer.policy(), ProtectionMode::Strict);
    let last = f.layer.store().log.last().unwrap().clone();
    assert_eq!((last.kind, last.policy, last.confirm), (TransactionKind::Policy, Some(ProtectionMode::Strict), true));
    assert_eq!((&last.pre, &last.post, last.receipt()), (&before.digest, &before.digest, switched));

    // Under Strict the same word builds a preview (KN5.9): its first prefix, word([1]), moves the protected orbit.
    let q = f.preview(&identity);
    let preview = f.layer.preview().unwrap();
    let strict = k.strict_protection(&k.word(&[1, -1]).unwrap(), f.layer.protected()).unwrap();
    assert_eq!(preview.protection, strict);
    assert_eq!(
        (strict.mode, strict.verdict, strict.first_refused_prefix),
        (ProtectionMode::Strict, Verdict::Moves, Some(1))
    );
    assert!(!strict.conflicts.is_empty() && strict.conflicts.iter().all(|row| row.orbit == 0));
    assert!(preview.end.is_none());
    let summary = preview.summary();
    assert_eq!(
        (summary.get("mode"), summary.get("first_refused_prefix")),
        (Some(&text("strict")), Some(&Value::Int(1)))
    );
    assert_eq!((summary.get("verdict"), summary.get("end")), (Some(&text("moves")), Some(&Value::Null)));
    assert_eq!(summary.get("net_moved_stickers"), Some(&Value::Int(0)));
    f.present(q);
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::ORBIT_WOULD_MOVE);
    let press = f.press(q);
    f.refused(press, code::ORBIT_WOULD_MOVE, "the commit under Strict");
    assert_eq!(f.layer.preview_state(q), Some(PreviewState::Presented));

    // Without a protected orbit Strict clears every prefix: the word builds a clear preview and commits.
    let mut g = fixture(&[]);
    assert!(completed(&g.set_policy("strict")));
    let r = g.preview(&identity);
    let summary = g.layer.preview().unwrap().summary();
    assert_eq!((summary.get("mode"), summary.get("first_refused_prefix")), (Some(&text("strict")), Some(&Value::Null)));
    assert_eq!(summary.get("verdict"), Some(&text("clear")));
    g.present(r);
    let press = g.press(r);
    assert!(committed(&g.shell.send(&mut g.layer, press)));
}

/// H-08's note on `protection.policy.set` ("invalidates any open preview"), KN0.8, KN5.18 (draft: a policy change
/// advances the protection revision, which every preview depends on, as a protection change does, so a press from
/// before it is stale while the slot keeps the preview; a new preview commits). Either direction does, and so does a
/// change to the policy already in force.
#[test]
fn a_policy_change_makes_an_open_preview_stale() {
    let mut f = fixture(&[]);
    for policy in ["strict", "net", "net"] {
        let p = f.preview(&word(&[2]));
        f.present(p);
        let press = f.press(p);
        assert!(completed(&f.set_policy(policy)), "{policy}");
        assert_eq!(f.layer.policy(), policy_from_name(policy).unwrap());
        assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));
        assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::REVISION_CHANGED);
        f.refused(press, code::REVISION_CHANGED, policy);
        let q = f.preview(&word(&[2]));
        f.present(q);
        let press = f.press(q);
        assert!(committed(&f.shell.send(&mut f.layer, press)), "{policy}");
    }
}

/// CT-KN5-09 (part: the undo and redo of a policy change are workspace inverses that answer `committed` with equal
/// puzzle hashes, put the policy before or after the change in force again and advance the workspace and protection
/// revisions; no direction confirms: the owner's reading of finding LD1 gates the row, and an undo or redo is the
/// solver's own press (draft)), KN5.18, LI.26. The stacks are last in,
/// first out, so an undo runs under the policy of its original action: a commit made under Net with an orbit protected
/// is undone and redone under Net once the policy change above it is undone.
#[test]
fn a_policy_change_undoes_and_redoes_as_a_workspace_inverse() {
    let mut f = fixture(&[0]);
    assert!(committed(&f.commit(&word(&[1, -1]))));
    assert!(completed(&f.set_policy("strict")));
    let steps = [
        (Direction::Undo, ProtectionMode::Net),
        (Direction::Redo, ProtectionMode::Strict),
        (Direction::Undo, ProtectionMode::Net),
    ];
    for (direction, policy) in steps {
        let before = f.layer.revision().clone();
        let receipt = f.step(direction);
        let Status::Committed(record) = &receipt.status else { panic!("{direction:?}: {:?}", receipt.status) };
        assert_eq!(record.pre, record.post);
        let after = f.layer.revision().clone();
        assert_eq!((&after.head, after.state, &after.digest), (&before.head, before.state, &before.digest));
        assert_eq!((after.workspace, after.protection), (before.workspace + 1, before.protection + 1));
        assert_eq!(f.layer.policy(), policy, "{direction:?}");
        let last = f.layer.store().log.last().unwrap();
        assert_eq!((last.policy, last.confirm, last.receipt()), (Some(policy), false, receipt.clone()));
    }
    for direction in [Direction::Undo, Direction::Redo] {
        let receipt = f.step(direction);
        assert!(committed(&receipt), "the commit's {direction:?}: {:?}", receipt.status);
        assert_eq!(f.layer.policy(), ProtectionMode::Net);
        assert_eq!(f.layer.store().log.last().unwrap().policy, None);
    }
}

/// CT-KN5-04 (part: a policy change is durable with its receipt, so its lookup and its retry answer the same
/// `completed(workspace)` after a restart and after a reopen and change nothing, and the same command ID with other
/// content is refused), KN5.18, KN5.20.
#[test]
fn a_policy_change_keeps_its_receipt_across_a_restart_and_a_reopen() {
    let mut f = fixture(&[]);
    let args = json::object([("confirm", Value::Bool(true)), ("policy", text("strict"))]);
    let asked = f.shell.prepare(&f.layer, POLICY_ROW, args).unwrap();
    assert!(asked.token.is_none(), "a policy change takes no token");
    let done = f.shell.send(&mut f.layer, asked.clone());
    assert!(completed(&done), "{:?}", done.status);
    let kept = f.fixed();
    assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), done);
    let other =
        Invocation { args: json::object([("confirm", Value::Bool(true)), ("policy", text("net"))]), ..asked.clone() };
    f.refused(other, code::COMMAND_ID_REUSED, "other content under the command ID");
    assert_eq!(f.fixed(), kept);

    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome(&asked.command, &done.request), Some(done.clone()));
    assert_eq!(f.layer.invoke(&Caller::Shell, asked.clone()), done);
    assert_eq!((f.layer.store().log.len(), f.layer.policy()), (1, ProtectionMode::Strict));

    let mut g = reopen(&f.layer);
    assert_eq!(g.layer.outcome(&asked.command, &done.request), Some(done.clone()));
    assert_eq!(g.layer.invoke(&Caller::Shell, asked), done);
    assert_eq!((g.layer.store().log.len(), g.layer.policy()), (1, ProtectionMode::Strict));
}

/// KN0.8, KN5.9 (draft: a policy change keeps the preview slot and its queued job, and the job builds the preview under
/// the policy in force when it runs, with the dependencies read then): a preview admitted under Net and built after the
/// switch to Strict is a fresh Strict preview whose commit is refused `ORBIT_WOULD_MOVE`.
#[test]
fn a_preview_queued_before_a_policy_change_is_built_under_the_new_policy() {
    let mut f = fixture(&[0]);
    let receipt = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[1, -1]))).unwrap();
    let Status::Running(p) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
    assert!(completed(&f.set_policy("strict")));
    assert_eq!(f.layer.run_jobs(), 1);
    let summary = f.layer.preview().unwrap().summary();
    assert_eq!(
        (summary.get("mode"), summary.get("first_refused_prefix")),
        (Some(&text("strict")), Some(&Value::Int(1)))
    );
    f.present(p);
    assert_eq!(f.layer.availability("operation.execute").unwrap_err().code, code::ORBIT_WOULD_MOVE);
    let press = f.press(p);
    f.refused(press, code::ORBIT_WOULD_MOVE, "the commit of a preview built under Strict");
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Presented));
}

/// KN2.7, KN5.20 (draft: replay walks an undo or a redo under the policy that the journal puts in force before it, so a
/// commit made under Strict and its undo and redo replay to the live layer's head, state and revisions).
#[test]
fn a_commit_made_under_strict_replays_with_its_undo_and_redo() {
    let mut f = fixture(&[]);
    assert!(completed(&f.set_policy("strict")));
    let start = f.layer.revision().digest.clone();
    assert!(committed(&f.commit(&word(&[1]))));
    let moved = f.layer.revision().digest.clone();
    assert_ne!(moved, start);
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.revision().digest, start);
    assert_eq!(restored(&reopen(&f.layer).layer), restored(&f.layer));
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.layer.revision().digest, moved);
    let kinds: Vec<TransactionKind> = f.layer.store().log.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [TransactionKind::Policy, TransactionKind::Commit, TransactionKind::Undo, TransactionKind::Redo]);
    let g = reopen(&f.layer);
    assert_eq!((restored(&g.layer), g.layer.policy()), (restored(&f.layer), ProtectionMode::Strict));
}

/// The journal form (draft): a policy change's transaction and its undo's and redo's carry `policy` as an 18th key,
/// each reads back to itself, and a 17-key journal reads as before. A commit with a policy, a policy change without one
/// or with a record, an undo with both a record and a policy, and a policy that is not `net` or `strict` do not read.
#[test]
fn a_policy_change_journals_its_policy() {
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[1]))));
    assert!(completed(&f.set_policy("strict")));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    let expected = [TransactionKind::Commit, TransactionKind::Policy, TransactionKind::Undo, TransactionKind::Redo];
    assert_eq!(kinds, expected);
    assert_eq!(log[1].to_json().get("kind"), Some(&text("policy")));
    let keys: Vec<usize> = log.iter().map(|t| t.to_json().as_object().unwrap().len()).collect();
    assert_eq!(keys, [17, 18, 18, 18]);
    let policies: Vec<Option<Value>> = log.iter().map(|t| t.to_json().get("policy").cloned()).collect();
    assert_eq!(policies, [None, Some(text("strict")), Some(text("net")), Some(text("strict"))]);
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }

    let object_of = |transaction: &Transaction| match transaction.to_json() {
        Value::Object(fields) => fields,
        _ => panic!("a transaction is an object"),
    };
    let record = json::object([("name", text("w")), ("value", Value::Null)]);
    let (mut commit, mut bare, mut loose, mut null, mut both, mut undo) = (
        object_of(&log[0]),
        object_of(&log[1]),
        object_of(&log[1]),
        object_of(&log[1]),
        object_of(&log[1]),
        object_of(&log[2]),
    );
    commit.insert("policy".into(), text("strict"));
    bare.remove("policy");
    loose.insert("policy".into(), text("Strict"));
    null.insert("policy".into(), Value::Null);
    both.insert("record".into(), record.clone());
    undo.insert("record".into(), record);
    let malformed = [
        (commit, "a commit with a policy"),
        (bare, "a policy change without one"),
        (loose, "a policy that is not `net` or `strict`"),
        (null, "a null policy"),
        (both, "a policy change with a record"),
        (undo, "an undo with a record and a policy"),
    ];
    for (fields, what) in malformed {
        assert_eq!(Transaction::from_json(&Value::Object(fields)), None, "{what}");
    }
    for policy in [ProtectionMode::Net, ProtectionMode::Strict] {
        assert_eq!(policy_from_name(policy_name(policy)), Some(policy));
    }
    assert_eq!(policy_from_name("Strict"), None);
}

/// KN5.20 (draft: a reopen replays the policy changes and their undo and redo, so a reopened layer has the live
/// layer's policy, head, state and revisions, judges a preview under that policy and undoes the change as the live
/// layer does; a session whose journal holds no policy change opens under Net).
#[test]
fn a_reopened_layer_keeps_the_policy() {
    let mut f = fixture(&[0]);
    assert_eq!(reopen(&f.layer).layer.policy(), ProtectionMode::Net);
    assert!(completed(&f.set_policy("strict")));
    let mut g = reopen(&f.layer);
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.policy(), ProtectionMode::Strict);
    g.preview(&word(&[1, -1]));
    let protection = &g.layer.preview().unwrap().protection;
    assert_eq!((protection.mode, protection.first_refused_prefix), (ProtectionMode::Strict, Some(1)));

    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&g.step(Direction::Undo)));
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(restored(&reopen(&f.layer).layer), restored(&f.layer));
    assert!(committed(&f.step(Direction::Redo)));
    let h = reopen(&f.layer);
    assert_eq!((restored(&h.layer), h.layer.policy()), (restored(&f.layer), ProtectionMode::Strict));
}

/// KN5.15, H-08's argument form and finding LD1 of the KN5 boundary attack (EV4.22; the owner's reading of 18 November
/// 2026): the runtime may not set the policy, since the protection a commit is judged under is the solver's, even with a
/// confirmation; `policy` is `net` or `strict`; and the change needs the solver's confirmation, `confirm: true`, as a
/// release of a protected orbit does, so a playback that sends the row without it changes nothing. Each refusal
/// changes nothing.
#[test]
fn only_the_solver_sets_the_policy_and_only_by_name() {
    let mut f = fixture(&[]);
    assert_eq!(f.layer.availability(POLICY_ROW), Ok(()));
    let runtime = Caller::Runtime { run: "run-1".into() };
    let epoch = f.layer.revision().epoch;
    let args = json::object([("confirm", Value::Bool(true)), ("policy", text("strict"))]);
    let theirs = Invocation { command: "run-1/1".into(), row: POLICY_ROW.into(), args, token: None, epoch };
    f.refused_on(&runtime, theirs, code::NOT_GRANTED, "the runtime");
    let bad = [
        ("an unknown policy", json::object([("policy", text("loose"))])),
        ("a capitalised name", json::object([("policy", text("Strict"))])),
        ("a policy that is not text", json::object([("policy", Value::Bool(true))])),
        ("a confirmation that is not a boolean", json::object([("confirm", text("yes")), ("policy", text("strict"))])),
        ("no policy", json::object([])),
    ];
    for (what, args) in bad {
        let asked = f.shell.prepare(&f.layer, POLICY_ROW, args).unwrap();
        f.refused(asked, code::ARGUMENT_FORM, what);
    }
    let unconfirmed = [
        ("no confirmation", json::object([("policy", text("strict"))])),
        ("a confirmation of false", json::object([("confirm", Value::Bool(false)), ("policy", text("strict"))])),
    ];
    for (what, args) in unconfirmed {
        let asked = f.shell.prepare(&f.layer, POLICY_ROW, args).unwrap();
        f.refused(asked, code::CONFIRMATION_REQUIRED, what);
    }
    assert_eq!(f.layer.policy(), ProtectionMode::Net);
}
