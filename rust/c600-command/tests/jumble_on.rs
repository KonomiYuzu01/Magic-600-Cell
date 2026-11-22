//! `jumble.on` in the command layer (draft; A5, ONE.54, ONE.55, ONE.80, ONE.81, JI.25, JI.27, LG2.8): the entry from
//! Jumble off into Jumble on at the poses of the session's solved witness, replayed in the jumbling engine under the
//! twist menu the host pinned. Each test names the conformance rows it covers headless with PHS.62 markers. Where the
//! frozen text is silent, the test follows the draft rulings recorded in `src/lib.rs` and says so.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::{JUMBLE_ON_MODE, MODE};
use c600_command::{
    code, draft_args, identity_json, Caller, CancelReason, CommandLayer, Completion, CompletionKind, Direction, Error,
    Fate, HeadlessFrames, Invocation, Kind, MemoryStore, Overlay, PreviewState, Provenance, Receipt, Secret, Shell,
    Status, Table, Transaction, TransactionKind, JUMBLE_ON_ROW, POLICY_ROW, RECORD_ROW, TWIST_ROW, WORKSHEET_LOAD_ROW,
    WORKSHEET_SAVE_ROW,
};
use c600_jumble::levels::{self, Base};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::codes::IDENTITY_MISMATCH;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Derivation, Kernel, ProtectedSet, ProtectionMode, Snapshot};

const SECRET: [u8; 32] = [7; 32];

/// The pinned I_a menu's set and order digests.
const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";

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

fn ctx() -> Arc<Context> {
    static CTX: OnceLock<Arc<Context>> = OnceLock::new();
    CTX.get_or_init(|| Arc::new(Context::load(&repository().join("assets")).expect("the exact model loads"))).clone()
}

fn menu_record() -> Value {
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    json::parse(&fixture).expect("the I_a fixture is JSON").get("menu").expect("a menu").clone()
}

/// The I_a menu of `research/jumbling/fixtures/wj-I_a.json`, the menu KN3's tests pin.
fn pinned() -> Menu {
    Menu::from_record(&ctx(), &menu_record()).expect("the pinned I_a menu")
}

/// The I_a menu with its base reversed: the same set in another order, so another jumbling identity block.
fn reversed() -> Menu {
    let mut record = menu_record();
    if let Value::Object(fields) = &mut record {
        if let Some(Value::Array(base)) = fields.get_mut("base") {
            base.reverse();
        }
    }
    Menu::from_record(&ctx(), &record).expect("the reversed I_a menu")
}

/// Menu item `index` at cap 0, pressed directly. At cap 0 items 1 to 11 are retained and 12 to 59 are jumble items.
fn item(index: u64) -> Binding {
    Binding {
        request: wire::menu_request(SET, ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: SET.to_owned(), order: ORDER.to_owned() }),
    }
}

/// `twist.apply`'s arguments for `binding`: its cap, and the rest of the binding as `rotation`.
fn twist_args(binding: &Binding) -> Value {
    let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
    let cap = fields.remove("cap").expect("a binding has a cap");
    json::object([("cap", cap), ("rotation", Value::Object(fields))])
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn derived(base: &Snapshot, recipe: &Value) -> Snapshot {
    let k = kernel();
    match k.derive(base, &k.recipe(recipe).unwrap(), &ProtectedSet::none()).unwrap() {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("unexpected refusal: {}", refused.refusal),
    }
}

/// `Digest<literal word>` of 600-cell-Full's letters: the SHA-256 of the letters as little-endian int32.
fn word_digest(letters: &[i64]) -> String {
    let bytes: Vec<u8> = letters.iter().flat_map(|&letter| (letter as i32).to_le_bytes()).collect();
    c600_digest::sha256_hex(&bytes)
}

/// The jumbling engine's replay of `letters` from solved under the pinned menu, with no protected piece: what an
/// entry with that solved witness must reach.
fn replayed(letters: &[i64]) -> levels::Derived {
    let ctx = ctx();
    let menu = Arc::new(pinned());
    let twists: Vec<_> = letters.iter().map(|&letter| levels::primitive(&ctx, letter).expect("a letter")).collect();
    let identity = ctx.identity(Some(&menu));
    let solved = levels::Snapshot::solved(&ctx, Some(menu));
    let derivation = levels::derive(&ctx, Base::Snapshot(&solved), &identity, &twists, ProtectionMode::Net, &[]);
    match derivation.expect("no engine error") {
        Ok(levels::Derivation::Derived(derived)) => derived,
        _ => panic!("the replay of {letters:?} from solved is refused"),
    }
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/full".into(), entry: entry.into() }
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

fn running(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Running(_))
}

/// What a reopen must restore: the policy, the head, the state, the digest, the workspace and protection revisions and
/// the mode.
fn restored(layer: &CommandLayer) -> (ProtectionMode, String, u64, String, u64, u64, &'static str) {
    let r = layer.revision();
    (layer.policy(), r.head.clone(), r.state, r.digest.clone(), r.workspace, r.protection, r.mode)
}

/// The host's provenance reader as these tests stand it in: it accepts every block.
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

/// A session that starts at `start` and protects `orbits`, with no twist menu pinned.
fn session(start: Snapshot, orbits: &[i64]) -> Fixture {
    let k = kernel();
    let protected = k.protected_set(orbits).unwrap();
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// A session at the solved state that protects `orbits`, with no twist menu pinned.
fn unpinned(orbits: &[i64]) -> Fixture {
    session(kernel().solved(), orbits)
}

/// `unpinned` with the I_a menu pinned.
fn fixture(orbits: &[i64]) -> Fixture {
    let mut f = unpinned(orbits);
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    f
}

/// A reopen of `layer`'s store, which replays its journal: with `menu` pinned before the replay
/// (`CommandLayer::open_with_menu`), or with no menu (`CommandLayer::open`).
fn reopen_with(layer: &CommandLayer, menu: Option<Menu>) -> Fixture {
    let (store, table, secret) = (layer.store().clone(), Table::h08().with_drafts(), Secret::fixed(SECRET));
    let layer = match menu {
        Some(menu) => CommandLayer::open_with_menu(kernel(), table, secret, store, ctx(), menu),
        None => CommandLayer::open(kernel(), table, secret, store),
    };
    Fixture { layer: layer.unwrap(), shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// `jumble.on`'s arguments naming the revision `revision` with one field edited.
fn revision_with(revision: &Value, edit: impl FnOnce(&mut BTreeMap<String, Value>)) -> Value {
    let Value::Object(mut fields) = revision.clone() else { panic!("a revision is an object") };
    edit(&mut fields);
    json::object([("revision", Value::Object(fields))])
}

impl Fixture {
    /// The `jumble.on` press, naming the current revision, prepared with its token and not sent.
    fn press(&mut self) -> Invocation {
        let revision = self.layer.revision().to_json();
        self.shell.prepare(&self.layer, JUMBLE_ON_ROW, json::object([("revision", revision)])).unwrap()
    }

    fn enter(&mut self) -> Receipt {
        let press = self.press();
        self.shell.send(&mut self.layer, press)
    }

    fn twist(&mut self, binding: &Binding) -> Receipt {
        let press = self.shell.prepare(&self.layer, TWIST_ROW, twist_args(binding)).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// Previews `recipe` and runs the job; returns the preview's ID.
    fn preview(&mut self, recipe: &Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(job) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        job
    }

    fn present(&mut self, id: u64) {
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let id = self.preview(recipe);
        self.present(id);
        let press =
            self.shell.prepare(&self.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))]));
        self.shell.send(&mut self.layer, press.unwrap())
    }

    /// The undo or redo of the action on top of its stack; no action here releases an orbit, so none confirms.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    /// `protection.orbit.set` with `orbits`, confirming no release.
    fn protect(&mut self, orbits: &[i64]) -> Receipt {
        let orbits = Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect());
        self.shell.invoke(&mut self.layer, "protection.orbit.set", json::object([("orbits", orbits)])).unwrap()
    }

    fn set_policy(&mut self, policy: &str) -> Receipt {
        self.shell
            .invoke(
                &mut self.layer,
                POLICY_ROW,
                json::object([("confirm", Value::Bool(true)), ("policy", text(policy))]),
            )
            .unwrap()
    }

    /// What no refusal may change: the state digest, the revision, the durable log, the policy and the poses.
    fn fixed(&self) -> (String, String, usize, ProtectionMode, Option<String>) {
        let r = self.layer.revision();
        let poses = self.layer.jumbled().map(|jumbled| jumbled.digest(&ctx()));
        (r.digest.clone(), format!("{r:?}"), self.layer.store().log.len(), self.layer.policy(), poses)
    }

    /// Delivers `invocation` on `caller`'s channel, which must be refused with `expected` and change nothing.
    fn refused_on(&mut self, caller: &Caller, invocation: Invocation, expected: &str, what: &str) -> Receipt {
        let before = self.fixed();
        let receipt = self.layer.invoke(caller, invocation);
        assert_eq!(self.fixed(), before, "{what}");
        match &receipt.status {
            Status::Refused(error) if error.code == expected => receipt,
            status => panic!("{what}: expected {expected}, got {status:?}"),
        }
    }

    fn refused(&mut self, invocation: Invocation, expected: &str, what: &str) -> Receipt {
        self.refused_on(&Caller::Shell, invocation, expected, what)
    }
}

/// CT-LI-05 (part: a crossing commits its mode, head, digest and state revision together with its `crossing` record;
/// a store fault is in `a_crossing_under_a_store_fault_commits_whole_or_not_at_all`; not run: an equal-label restore,
/// which the layer does not build).
/// CT-ONE-17 (part: a snapshot opened before a crossing keeps its labels and digest across the entry; not run: a
/// snapshot of the jumbled puzzle's own level, which LP3's runtime opens).
/// CT-ONE-03 (part: a crossing commits with no preview object), CT-JI-08 (part: the solved-witness entry with both of
/// its sides; the exit-root entry is in `tests/jumble_off.rs`), JI.25, ONE.55, ONE.80. After a word commit and
/// a twist from solved, `jumble.on` enters at the jumbling engine's replay of their literal letters from solved, whose
/// labels are the current labels. The revision's mode becomes `jumble_on`, its state revision advances, its head is the
/// crossing's event and its digest the pose digest; the retained state stays; the journal keeps the crossing's record
/// (draft: the entrance, the identity block, and the witness's letter count and `Digest<literal word>`).
#[test]
fn an_entry_crosses_at_the_poses_of_the_solved_witness() {
    let mut f = fixture(&[]);
    assert!(f.layer.jumbled().is_none());
    assert_eq!(f.layer.revision().mode, MODE);
    assert!(committed(&f.commit(&word(&[2]))));
    assert!(committed(&f.twist(&item(6))));
    assert_eq!(f.layer.availability(JUMBLE_ON_ROW), Ok(()));
    let before = f.layer.revision().clone();
    let labels = f.layer.snapshot().digest().to_owned();
    let held = f.layer.snapshot().clone();
    let held_labels = held.labels().to_vec();

    let receipt = f.enter();
    let Status::Committed(record) = &receipt.status else { panic!("{:?}", receipt.status) };
    assert_eq!((held.digest(), held.labels()), (labels.as_str(), held_labels.as_slice()), "a held snapshot");
    let letters = [2, 1, -2, 1];
    let pose = replayed(&letters).digest(&ctx());
    assert_ne!(pose, labels);
    let jumbled = f.layer.jumbled().expect("Jumble is on");
    assert_eq!(jumbled.digest(&ctx()), pose);
    assert_eq!(jumbled.identity(), &ctx().identity(Some(&pinned())));
    match levels::labels(&ctx(), jumbled.state()).expect("the poses' labels") {
        levels::Partial::Total(total) => assert_eq!(total.digest(), labels),
        _ => panic!("the entry's poses have total labels"),
    }
    assert_eq!((record.pre.as_str(), record.post.as_str()), (labels.as_str(), pose.as_str()));
    let after = f.layer.revision().clone();
    assert_eq!(after.mode, JUMBLE_ON_MODE);
    assert_eq!(
        (after.state, after.workspace, after.protection),
        (before.state + 1, before.workspace, before.protection)
    );
    assert_eq!((after.head.as_str(), after.digest.as_str()), ("e3", pose.as_str()));
    assert_eq!(record.revision, after);
    assert_eq!(f.layer.snapshot().digest(), labels);
    assert_eq!(receipt.job, None);
    assert!(f.layer.preview().is_none());
    assert_eq!(f.layer.run_jobs(), 0);

    let log = f.layer.store().log.clone();
    assert_eq!(log.len(), 3);
    let crossing = &log[2];
    let shape = (crossing.kind, crossing.row.as_str(), &crossing.recipe, crossing.literal.as_str());
    assert_eq!(shape, (TransactionKind::Crossing, JUMBLE_ON_ROW, &Value::Null, ""));
    assert_eq!(crossing.rotation, None);
    assert_eq!(crossing.receipt(), receipt);
    let expected = json::object([
        ("entrance", text("solved_witness")),
        ("identity", identity_json(&ctx().identity(Some(&pinned())))),
        ("letters", Value::Int(4)),
        ("word", text(&word_digest(&letters))),
    ]);
    assert_eq!(crossing.crossing.as_ref(), Some(&expected));
}

/// CT-LI-05 (part: a crossing commits its complete tuple or none: a write the store rolls back leaves the mode, head,
/// state revision, digest and journal as they were, and the next press crosses; a write whose fate the store cannot
/// tell holds the layer in read-only recovery at Jumble off until a restart settles it, which adopts the whole crossing
/// when the write happened and nothing when it did not; not run: an equal-label restore and its durable void, since the
/// layer builds no restore row).
#[test]
fn a_crossing_under_a_store_fault_commits_whole_or_not_at_all() {
    let mut f = fixture(&[]);
    assert!(committed(&f.twist(&item(6))));
    let (before, log) = (restored(&f.layer), f.layer.store().log.len());
    f.layer.store_mut().inject.push_back(Fate::RolledBack(Error::new("PS1:io.injected", "injected rollback")));
    let receipt = f.enter();
    assert!(!committed(&receipt), "{:?}", receipt.status);
    assert_eq!((restored(&f.layer), f.layer.store().log.len()), (before.clone(), log));
    assert!(f.layer.jumbled().is_none());
    assert!(committed(&f.enter()));
    assert_eq!(f.layer.revision().mode, JUMBLE_ON_MODE);

    for written in [false, true] {
        let mut f = fixture(&[]);
        assert!(committed(&f.twist(&item(6))));
        let before = restored(&f.layer);
        f.layer.store_mut().inject.push_back(Fate::Unknown);
        f.layer.store_mut().unknown_writes = written;
        let receipt = f.enter();
        assert_eq!(receipt.status, Status::UnknownOutcome, "{written}");
        assert!(f.layer.recovery().is_some(), "{written}");
        assert_eq!(restored(&f.layer), before, "{written}");
        assert!(f.layer.jumbled().is_none(), "{written}");
        f.layer.restart(Secret::fixed([8; 32]));
        f.shell.restart(Secret::fixed([8; 32]));
        assert!(f.layer.recovery().is_none(), "{written}");
        let after = restored(&f.layer);
        if written {
            assert_eq!((after.2, after.6), (before.2 + 1, JUMBLE_ON_MODE), "{written}");
            assert!(f.layer.jumbled().is_some(), "{written}");
            let outcome = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
            assert!(committed(&outcome), "{:?}", outcome.status);
        } else {
            assert_eq!(after, before, "{written}");
            assert!(f.layer.jumbled().is_none(), "{written}");
        }
    }
}

/// A5, ONE.54, JI.25 (the solved-witness entry's refusal): a session that did not start at solved holds no retained
/// word from solved, so its `jumble.on` is `KN5:input.jumble_on_labels_only` and changes nothing, even at a solved head:
/// no witness is generated from labels. A session that starts at solved enters at once with the empty witness.
#[test]
fn labels_alone_are_refused() {
    let k = kernel();
    let mut f = session(derived(&k.solved(), &word(&[1])), &[]);
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert_eq!(f.layer.availability(JUMBLE_ON_ROW), Ok(()), "availability does not promise acceptance");
    let press = f.press();
    f.refused(press, code::JUMBLE_ON_LABELS_ONLY, "a session that started elsewhere");
    assert!(committed(&f.commit(&word(&[-1]))));
    assert_eq!(f.layer.snapshot().digest(), k.solved().digest());
    let press = f.press();
    f.refused(press, code::JUMBLE_ON_LABELS_ONLY, "a solved head reached from elsewhere");
    assert!(f.layer.jumbled().is_none());

    let mut g = fixture(&[]);
    assert!(committed(&g.enter()));
    assert_eq!(g.layer.revision().digest, replayed(&[]).digest(&ctx()));
    let record = g.layer.store().log[0].crossing.clone().expect("the entry's record");
    assert_eq!((record.get("letters"), record.get("word")), (Some(&Value::Int(0)), Some(&text(&word_digest(&[])))));
}

/// CT-ONE-03 (part: a crossing without a token, or with one bound to other arguments, is refused), CT-ONE-11 (part: a
/// crossing refused at each step of ONE.80's order leaves every digest unchanged), KN5.18, L15. The draft order: the
/// pinned menu (`KN5:input.row_not_built`) and the `revision` argument's form before the token; then the token, the
/// commit lane and the token's binding; then the mode; then the named revision, which is current only if every field
/// is equal (EV4.14), so a workspace completion makes a press stale, unlike a twist's; then the head hash.
#[test]
fn refusals_follow_the_crossing_order_and_change_nothing() {
    let mut f = unpinned(&[]);
    assert_eq!(f.layer.availability(JUMBLE_ON_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    let mut forged = f.press();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::ROW_NOT_BUILT, "no pinned menu, with a forged token");
    let press = f.press();
    f.refused(press, code::ROW_NOT_BUILT, "no pinned menu");

    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();
    let now = f.layer.revision().to_json();
    let cases = [
        ("no revision", json::object([])),
        ("an argument the row does not take", json::object([("revision", now.clone()), ("x", Value::Null)])),
        ("a confirmation", json::object([("confirm", Value::Bool(true)), ("revision", now.clone())])),
        ("a revision with an extra field", revision_with(&now, |r| drop(r.insert("x".into(), Value::Null)))),
        ("a revision that is text", json::object([("revision", text("now"))])),
        ("a revision without its digest", revision_with(&now, |r| drop(r.remove("digest")))),
        ("a negative state revision", revision_with(&now, |r| drop(r.insert("state".into(), Value::Int(-1))))),
        ("a mode that is not text", revision_with(&now, |r| drop(r.insert("mode".into(), Value::Int(0))))),
    ];
    for (what, args) in cases {
        let asked = f.shell.prepare(&f.layer, JUMBLE_ON_ROW, args).unwrap();
        f.refused(asked, code::ARGUMENT_FORM, what);
    }
    let mut forged = f.shell.prepare(&f.layer, JUMBLE_ON_ROW, json::object([("revision", text("now"))])).unwrap();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::ARGUMENT_FORM, "a malformed revision with a forged token: the form first");

    let press = f.press();
    f.refused(Invocation { token: None, ..press.clone() }, code::TOKEN_INVALID, "no token");
    let mut forged = press.clone();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::TOKEN_INVALID, "a forged token");
    // An authentic token's refusal attaches to its command ID, so this case takes a press of its own.
    let moved = revision_with(&now, |r| drop(r.insert("workspace".into(), Value::Int(9))));
    let other = f.press();
    f.refused(Invocation { args: moved, ..other }, code::TOKEN_MISMATCH, "a token bound to other arguments");

    f.layer.hold_commit_lane(true);
    assert_eq!(f.layer.availability(JUMBLE_ON_ROW).unwrap_err().code, code::COMMIT_IN_FLIGHT);
    let held = f.press();
    let busy = f.refused(held.clone(), code::COMMIT_IN_FLIGHT, "a held lane");
    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.invoke(&Caller::Shell, held), busy, "a refused press keeps its answer");

    assert!(committed(&f.twist(&item(1))));
    f.refused(press, code::REVISION_CHANGED, "a press from before a commit");
    let press = f.press();
    assert!(completed(&f.protect(&[0])));
    f.refused(press, code::REVISION_CHANGED, "a press from before a protection change");
    let press = f.press();
    assert!(completed(&f.set_policy("strict")));
    f.refused(press, code::REVISION_CHANGED, "a press from before a policy change");
    let press = f.press();
    assert!(running(&f.shell.invoke(&mut f.layer, "tool.full.note", json::object([])).unwrap()));
    let job = f.layer.take_program_job().unwrap().job;
    assert!(completed(&f.layer.finish_program_job(job, Ok(text("note"))).unwrap()));
    f.refused(press, code::REVISION_CHANGED, "a press from before a workspace completion");

    let head = f.layer.snapshot().clone();
    f.layer.fault_replace_head(derived(&head, &word(&[3])));
    let press = f.press();
    f.refused(press, code::STATE_HASH_MISMATCH, "a head that differs");
    f.layer.fault_replace_head(head);

    // No protection gate: orbit 0 is protected under Strict, and the entry moves no piece.
    let stale = f.press();
    assert!(committed(&f.enter()));
    assert_eq!(f.layer.availability(JUMBLE_ON_ROW).unwrap_err().code, code::RETAINED_TOOL_NEEDS_JUMBLE_OFF);
    let again = f.press();
    f.refused(again, code::RETAINED_TOOL_NEEDS_JUMBLE_OFF, "Jumble on already");
    f.refused(stale, code::RETAINED_TOOL_NEEDS_JUMBLE_OFF, "a stale press with Jumble on: the mode first");
    let mut forged = f.press();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::TOKEN_INVALID, "a forged token with Jumble on: the token first");
    f.layer.hold_commit_lane(true);
    let held = f.press();
    f.refused(held, code::COMMIT_IN_FLIGHT, "a held lane with Jumble on: the lane first");
    f.layer.hold_commit_lane(false);
}

/// CT-KN5-04 (part: an entry is admitted once: a retried delivery answers the stored receipt and enters nothing more,
/// also after a reopen; the spent token under another command ID is `KN5:auth.token_used`), KN5.20, KN5.21.
#[test]
fn a_retried_entry_answers_its_receipt_and_enters_once() {
    let mut f = fixture(&[]);
    let press = f.press();
    let done = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&done), "{:?}", done.status);
    let kept = f.fixed();
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done);
    assert_eq!(f.fixed(), kept);
    let replay = Invocation { command: "replay/1".into(), ..press.clone() };
    f.refused(replay, code::TOKEN_USED, "the spent token under another command ID");

    let mut g = reopen_with(&f.layer, Some(pinned()));
    assert_eq!(g.layer.outcome(&press.command, &done.request), Some(done.clone()));
    assert_eq!(g.layer.invoke(&Caller::Shell, press), done);
    assert_eq!(g.layer.store().log.len(), 1);
}

/// CT-LG2-08 (part: a call made for the other mode is refused at invocation after a real mode change, with the mode
/// refusal checked before a stale press; not run: the static check, and a program's call, which LP3's host makes).
/// LG2.8 and the draft rulings in `src/lib.rs`: with Jumble on, `operation.preview`, `operation.execute` and
/// `twist.apply` are `KN5:input.retained_tool_needs_jumble_off` and change nothing: a preview before its draft is read,
/// a commit after its token's binding, a twist after its token, so a jumble item answers the mode too. So are LB7.2's
/// `worksheet.save` and `worksheet.load`, before their arguments are read. Availability says the same, and so does
/// KN2.4's `positions` read at the head (draft; CT-LB2-04), whose retained state is the one the crossing entered from,
/// and the structure reads of `structure.compare` and `filter.preview` (draft), which name the retained model's cells
/// and positions. The protection, policy and record rows and a program row stay, their actions' `pre` and `post` are
/// the pose digest, and a program job reads the revision with its mode.
#[test]
fn the_retained_workflow_needs_jumble_off() {
    let mut f = fixture(&[]);
    f.layer.set_provenance(Box::new(Reader));
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();
    let p = f.preview(&word(&[2]));
    f.present(p);
    let execute =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(p as i64))])).unwrap();
    assert!(committed(&f.enter()));
    let on = f.layer.revision().clone();

    let needs_off = code::RETAINED_TOOL_NEEDS_JUMBLE_OFF;
    for row in
        ["operation.preview", "operation.execute", TWIST_ROW, JUMBLE_ON_ROW, WORKSHEET_SAVE_ROW, WORKSHEET_LOAD_ROW]
    {
        assert_eq!(f.layer.availability(row).unwrap_err().code, needs_off, "{row}");
    }
    for row in ["protection.orbit.set", POLICY_ROW, RECORD_ROW, "tool.full.note", "session.undo"] {
        assert_eq!(f.layer.availability(row), Ok(()), "{row}");
    }
    let kept = f.fixed();
    assert_eq!(f.layer.positions(&[0, 177_119]).unwrap_err().code, needs_off);
    assert_eq!(f.layer.compare_cells(0, 1).unwrap_err().code, needs_off);
    assert_eq!(f.layer.cell_layers(0).unwrap_err().code, needs_off);
    assert_eq!(f.layer.layer_counts(0, 0).unwrap_err().code, needs_off);
    assert_eq!(f.fixed(), kept);
    let asked = f.shell.prepare(&f.layer, "operation.preview", draft_args(&word(&[2]))).unwrap();
    f.refused(asked, needs_off, "a preview");
    let asked = f.shell.prepare(&f.layer, "operation.preview", json::object([("draft", text("none"))])).unwrap();
    f.refused(asked, needs_off, "a preview whose draft does not read");
    f.refused(execute, needs_off, "the press for the preview the entry discarded");
    for binding in [item(1), item(12)] {
        let press = f.shell.prepare(&f.layer, TWIST_ROW, twist_args(&binding)).unwrap();
        f.refused(press, needs_off, "a twist");
    }
    let mut forged = f.shell.prepare(&f.layer, TWIST_ROW, twist_args(&item(12))).unwrap();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::TOKEN_INVALID, "a jumble item's twist with a forged token");
    let save = json::object([("draft", json::object([("record", text("w"))])), ("name", text("sheet"))]);
    let asked = f.shell.prepare(&f.layer, WORKSHEET_SAVE_ROW, save).unwrap();
    f.refused(asked, needs_off, "a work sheet's save");
    let named = json::object([("record", text("sheet")), ("version", Value::Int(1))]);
    let asked = f.shell.prepare(&f.layer, WORKSHEET_LOAD_ROW, json::object([("worksheet", named)])).unwrap();
    f.refused(asked, needs_off, "a work sheet's reuse");

    assert!(completed(&f.protect(&[0])));
    assert!(completed(&f.set_policy("strict")));
    let payload = Value::Array(vec![Value::Int(1), Value::Int(2)]);
    let write = json::object([("payload", payload), ("provenance", text("block")), ("record", text("w"))]);
    assert!(completed(&f.shell.invoke(&mut f.layer, RECORD_ROW, write).unwrap()));
    assert!(running(&f.shell.invoke(&mut f.layer, "tool.full.note", json::object([])).unwrap()));
    let job = f.layer.take_program_job().unwrap();
    assert_eq!((job.revision.mode, &job.revision), (JUMBLE_ON_MODE, f.layer.revision()));
    assert!(completed(&f.layer.finish_program_job(job.job, Ok(text("note"))).unwrap()));

    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    use TransactionKind::{Crossing, Policy, Protection, Record, Workspace};
    assert_eq!(kinds, [Crossing, Protection, Policy, Record, Workspace]);
    for t in &log[1..] {
        assert_eq!(
            (t.pre.as_str(), t.post.as_str(), t.revision.mode),
            (on.digest.as_str(), on.digest.as_str(), JUMBLE_ON_MODE)
        );
    }
    let r = f.layer.revision();
    assert_eq!((r.state, r.head.as_str(), r.digest.as_str()), (on.state, on.head.as_str(), on.digest.as_str()));
}

/// CT-LI-05 (part: a crossing's undo and its redo each commit the whole recorded tuple with a new state revision; not
/// run: an equal-label restore and its durable void, which the layer does not build).
/// CT-JI-08 (part: the solved-witness entry survives undo and redo using its own sides; the exit-root entry is in
/// `tests/jumble_off.rs`; not run: branch navigation), ONE.81, JI.27. An undo of the entry returns to the exact Jumble
/// off state before it and a redo replays the solved witness again, each with a new state revision and a committed
/// receipt; the retained workflow works again after the undo, and a retained commit there clears the entry's redo.
/// The draft rulings: while Jumble is on, a pin of a menu with another identity block is
/// `KN1:input.identity_mismatch`, and so is a redo of the entry under one; both change nothing.
#[test]
fn undo_and_redo_restore_the_entry_s_own_sides() {
    let mut f = fixture(&[]);
    assert!(committed(&f.twist(&item(6))));
    let off = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    let on = f.layer.revision().clone();
    let log = f.layer.store().log.clone();
    let record = log[1].crossing.clone().expect("the entry's record");
    assert_eq!(record.get("word"), Some(&text(&log[0].literal)), "the witness is the twist's literal word");

    let error = f.layer.pin_menu(ctx(), reversed()).unwrap_err();
    assert_eq!(error.code, IDENTITY_MISMATCH);
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert_eq!(f.layer.revision(), &on);

    let undone = f.step(Direction::Undo);
    let Status::Committed(record) = &undone.status else { panic!("{:?}", undone.status) };
    assert_eq!((record.pre.as_str(), record.post.as_str()), (on.digest.as_str(), off.digest.as_str()));
    let r = f.layer.revision().clone();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (MODE, off.head.as_str(), off.digest.as_str()));
    assert_eq!((r.state, r.workspace, r.protection), (on.state + 1, on.workspace, on.protection));
    assert!(f.layer.jumbled().is_none());
    assert_eq!(f.layer.availability(TWIST_ROW), Ok(()));
    let pieces = [0, 177_119];
    assert_eq!(f.layer.positions(&pieces), Ok(kernel().positions(f.layer.snapshot(), &pieces).unwrap()));
    assert_eq!(f.layer.compare_cells(0, 1), Ok(kernel().compare_cells(0, 1).unwrap()));
    assert_eq!(f.layer.layer_counts(0, 0), Ok(kernel().layer_counts(0, 0).unwrap()));

    let redone = f.step(Direction::Redo);
    let Status::Committed(record) = &redone.status else { panic!("{:?}", redone.status) };
    assert_eq!((record.pre.as_str(), record.post.as_str()), (off.digest.as_str(), on.digest.as_str()));
    let r = f.layer.revision().clone();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (JUMBLE_ON_MODE, on.head.as_str(), on.digest.as_str()));
    assert_eq!(r.state, on.state + 2);
    assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&ctx()), on.digest);

    assert!(committed(&f.step(Direction::Undo)));
    f.layer.pin_menu(ctx(), reversed()).unwrap();
    let hint = f.layer.undo_hint(Direction::Redo).expect("the entry's redo");
    let asked = f.shell.prepare(&f.layer, "session.redo", json::object([("hint", hint.to_json())])).unwrap();
    f.refused(asked, IDENTITY_MISMATCH, "a redo under another identity block");
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.layer.revision().digest, on.digest);

    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.revision().digest, kernel().solved().digest());
    assert!(committed(&f.twist(&item(1))));
    assert!(f.layer.undo_hint(Direction::Redo).is_none());
    assert!(committed(&f.enter()));
    assert_eq!(f.layer.revision().digest, replayed(&[1]).digest(&ctx()));

    let kinds: Vec<TransactionKind> = f.layer.store().log.iter().map(|t| t.kind).collect();
    use TransactionKind::{Commit, Crossing, Redo, Undo};
    assert_eq!(kinds, [Commit, Crossing, Undo, Redo, Undo, Redo, Undo, Undo, Commit, Crossing]);
}

/// CT-LI-05 (part: a reopen replays a crossing whole, or stays in read-only recovery before it, never half-crossed;
/// not run: a restore's durable void under recovery, which waits for the restore row).
/// KN5.23, ONE.58 and JI.27's recovery row (draft): a reopen with the session's twist menu pinned before the replay
/// (`CommandLayer::open_with_menu`) replays the entry's witness, makes the same crossing record and restores the mode,
/// the head and the poses, and its undo and redo work there. Without the menu, or under a menu with another identity
/// block, the crossing does not replay: the layer is in read-only recovery after the last action before it, at Jumble
/// off, never half-crossed.
#[test]
fn a_reopen_replays_the_entry_under_its_menu() {
    let mut f = fixture(&[]);
    assert!(committed(&f.twist(&item(6))));
    let off = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    assert!(completed(&f.set_policy("strict")));
    let log = f.layer.store().log.clone();

    let mut g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.jumbled().expect("Jumble is on").digest(&ctx()), f.layer.revision().digest);
    assert_eq!(g.layer.store().log, log);
    assert!(committed(&g.step(Direction::Undo)));
    assert!(committed(&g.step(Direction::Undo)));
    assert_eq!((g.layer.revision().mode, g.layer.revision().digest.as_str()), (MODE, off.digest.as_str()));
    assert!(committed(&g.step(Direction::Redo)));
    assert_eq!(g.layer.revision().digest, f.layer.revision().digest);
    let h = reopen_with(&g.layer, Some(pinned()));
    assert_eq!(restored(&h.layer), restored(&g.layer));

    for (menu, what) in [(None, "no menu"), (Some(reversed()), "another identity block")] {
        let h = reopen_with(&f.layer, menu);
        assert!(h.layer.recovery().is_some(), "{what}");
        let r = h.layer.revision();
        assert_eq!(
            (r.mode, r.head.as_str(), r.digest.as_str()),
            (MODE, off.head.as_str(), off.digest.as_str()),
            "{what}"
        );
        assert!(h.layer.jumbled().is_none(), "{what}");
        assert_eq!(h.layer.availability(JUMBLE_ON_ROW).unwrap_err().code, code::READ_ONLY_RECOVERY, "{what}");
    }
    let h = reopen_with(&f.layer, None);
    assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY));
}

/// The journal form (draft): an entry is the action kind `crossing` with its record under the key `crossing`, an 18th
/// key, and reads back to itself; no other action carries one. An entry without its record, a record on a commit or an
/// undo, and a record that is not an entry's from a solved witness under a jumbling identity block do not read. The
/// form does not check the witness against the move tree, which a reopen's replay does.
#[test]
fn an_entry_journals_its_crossing_record() {
    let mut f = fixture(&[]);
    assert!(committed(&f.twist(&item(1))));
    assert!(committed(&f.enter()));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    let log = f.layer.store().log.clone();
    let keys: Vec<usize> = log.iter().map(|t| t.to_json().as_object().unwrap().len()).collect();
    assert_eq!(keys, [18, 18, 17, 17]);
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }
    assert_eq!(log[1].to_json().get("kind"), Some(&text("crossing")));
    let record = log[1].crossing.clone().expect("the entry's record");
    assert_eq!(log[1].to_json().get("crossing"), Some(&record));

    let carrying = |transaction: &Transaction, key: &str, value: Option<Value>| {
        let Value::Object(mut fields) = transaction.to_json() else { panic!("a transaction is an object") };
        match value {
            Some(value) => fields.insert(key.to_owned(), value),
            None => fields.remove(key),
        };
        Value::Object(fields)
    };
    let edited = |edit: &dyn Fn(&mut BTreeMap<String, Value>)| {
        let Value::Object(mut fields) = record.clone() else { panic!("a crossing record is an object") };
        edit(&mut fields);
        carrying(&log[1], "crossing", Some(Value::Object(fields)))
    };
    let full = identity_json(&kernel().identity());
    let malformed = [
        (carrying(&log[1], "crossing", None), "an entry without its record"),
        (carrying(&log[1], "crossing", Some(Value::Null)), "a null record"),
        (carrying(&log[0], "crossing", Some(record.clone())), "a twist with a crossing record"),
        (carrying(&log[2], "crossing", Some(record.clone())), "an undo with a crossing record"),
        (carrying(&log[1], "rotation", log[0].rotation.clone()), "an entry with a rotation"),
        (edited(&|r| drop(r.insert("entrance".into(), text("exit_root")))), "another entrance"),
        (edited(&|r| drop(r.insert("identity".into(), full.clone()))), "a 600-cell-Full identity block"),
        (edited(&|r| drop(r.remove("identity"))), "no identity block"),
        (edited(&|r| drop(r.insert("letters".into(), Value::Int(-1)))), "a negative letter count"),
        (edited(&|r| drop(r.insert("letters".into(), text("1")))), "a letter count that is text"),
        (edited(&|r| drop(r.insert("word".into(), text(&"A".repeat(64))))), "a digest in upper case"),
        (edited(&|r| drop(r.insert("word".into(), text("00")))), "a short digest"),
        (edited(&|r| drop(r.insert("extra".into(), Value::Null))), "an extra field"),
    ];
    for (value, what) in malformed {
        assert_eq!(Transaction::from_json(&value), None, "{what}");
    }
    let other = edited(&|r| drop(r.insert("word".into(), text(&"0".repeat(64)))));
    assert!(Transaction::from_json(&other).is_some(), "the journal form does not check the witness against the tree");
}

/// CT-ONE-17 (part: a crossing cancels the jobs on the replaced revision; not run: a snapshot opened before it keeps
/// its level, which is the runtime's), ONE.55, KN5.18, LI.32: as a commit does, the entry discards the pending preview,
/// so a press for it is refused, cancels a running and a queued query job with `replaced_revision` and drops a late
/// result; a file job keeps running.
#[test]
fn an_entry_ends_the_preview_and_the_jobs_that_read_the_state() {
    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.inspect", Kind::Query, &[], overlay("inspect")).unwrap();
    f.layer.register_overlay("tool.full.scan", Kind::Query, &[], overlay("scan")).unwrap();
    f.layer.register_overlay("tool.full.export", Kind::File, &[], overlay("export")).unwrap();
    f.layer.set_program_capacity(3);
    let p = f.preview(&word(&[2]));
    f.present(p);
    let inspect = f.shell.invoke(&mut f.layer, "tool.full.inspect", json::object([])).unwrap();
    let scan = f.shell.invoke(&mut f.layer, "tool.full.scan", json::object([])).unwrap();
    let export = f.shell.invoke(&mut f.layer, "tool.full.export", json::object([])).unwrap();
    assert!(running(&inspect) && running(&scan) && running(&export));
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(taken.row, "tool.full.inspect");

    assert!(committed(&f.enter()));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Discarded));
    assert!(f.layer.preview().is_none());
    for receipt in [&inspect, &scan] {
        let ended = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert_eq!(ended.status, Status::Cancelled(CancelReason::ReplacedRevision));
    }
    assert_eq!(f.layer.outcome(&export.command, &export.request).as_ref(), Some(&export));
    assert!(f.layer.finish_program_job(taken.job, Ok(Value::Null)).is_none());
    let export = f.layer.take_program_job().unwrap();
    assert_eq!((export.row.as_str(), export.revision.mode), ("tool.full.export", MODE));
    assert_eq!(f.layer.store().log.len(), 1);
}

/// KN5.15: the runtime may not invoke a commit row, so its `jumble.on` is `KN5:input.commit_row_refused` and changes
/// nothing, with or without a pinned menu.
#[test]
fn the_runtime_may_not_enter() {
    for mut f in [fixture(&[]), unpinned(&[])] {
        let runtime = Caller::Runtime { run: "run-1".into() };
        let epoch = f.layer.revision().epoch;
        let args = json::object([("revision", f.layer.revision().to_json())]);
        let theirs =
            Invocation { command: "run-1/1".into(), row: JUMBLE_ON_ROW.into(), args, token: None, epoch, budget: None };
        f.refused_on(&runtime, theirs, code::COMMIT_ROW_REFUSED, "the runtime");
    }
}
