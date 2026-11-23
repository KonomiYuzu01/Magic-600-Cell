//! `jumble.twist.menu` in the command layer (draft; JI.27, JI.35, FP.31, ONE.78): a live twist of an exact item of the
//! twist menu the host pinned, whose `cap` and `letter` carry KN3.7's request binding, committed at once with no
//! preview. With Jumble off a retained item dispatches to its retained word on 600-cell-Full, as `twist.apply` does,
//! and a jumble item needs Jumble on; with Jumble on every item is a pose move in the jumbling engine, journaled as
//! FP.31's `jumble_move`. Each test names the conformance rows it covers headless with PHS.62 markers. Where the frozen
//! text is silent, the test follows the draft rulings recorded in `src/lib.rs` and says so.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::{JUMBLE_ON_MODE, MODE};
use c600_command::{
    code, draft_args, Caller, CancelReason, CommandLayer, Completion, CompletionKind, Direction, HeadlessFrames,
    Invocation, Kind, MemoryStore, Overlay, PreviewState, Receipt, Secret, Shell, Status, Table, Transaction,
    TransactionKind, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, POLICY_ROW, TWIST_ROW,
};
use c600_jumble::codes::{MENU_WORD_NEEDS_JUMBLE_ON, PIECE_WOULD_MOVE, WORD_BLOCKED_PREFIX};
use c600_jumble::levels::{self, Base};
use c600_jumble::q5::transpose;
use c600_jumble::rotation;
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;
use c600_kernel::codes::RECIPE_SHAPE;
use c600_kernel::json::{self, Value};
use c600_kernel::request::codes::{MENU_BINDING_MISMATCH, ROTATION_REQUEST_MALFORMED, TWIST_OFF_MENU};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Derivation, Kernel, ProtectedSet, ProtectionMode, Snapshot};

const SECRET: [u8; 32] = [7; 32];

/// The pinned I_a menu's set and order digests, and the order digest of the same menu with its base reversed.
const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";
const REVERSED_ORDER: &str = "1a87a4209a933f47a568eac9e881697614bf53e1f7411e817f5764a3a5f92a78";

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

/// `research/jumbling/fixtures/wj-I_a.json`: the I_a menu and a jumbling journal under it.
fn fixture_json() -> &'static Value {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let path = repository().join("research/jumbling/fixtures/wj-I_a.json");
        json::parse(&std::fs::read(path).expect("the I_a fixture")).expect("the I_a fixture is JSON")
    })
}

/// The I_a menu, the menu KN3's tests pin.
fn pinned() -> Menu {
    Menu::from_record(&ctx(), fixture_json().get("menu").expect("a menu")).expect("the pinned I_a menu")
}

/// The I_a menu with its base reversed: the same set in another order, so another jumbling identity block.
fn reversed() -> Menu {
    let mut record = fixture_json().get("menu").expect("a menu").clone();
    if let Value::Object(fields) = &mut record {
        if let Some(Value::Array(base)) = fields.get_mut("base") {
            base.reverse();
        }
    }
    Menu::from_record(&ctx(), &record).expect("the reversed I_a menu")
}

fn pin(order: &str) -> MenuPin {
    MenuPin { set: SET.to_owned(), order: order.to_owned() }
}

/// Menu item `index` at cap 0, pressed directly. At cap 0 item 0 is the identity, items 1 to 11 are retained and 12 to
/// 59 are jumble items.
fn item(index: u64) -> Binding {
    Binding {
        request: wire::menu_request(SET, ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(pin(ORDER)),
    }
}

/// The quarter turn about the cap's first axis, which is not on the menu: pressed directly it is off the menu, and
/// dragged it lands on a jumble item.
fn quarter(operation: Operation) -> Binding {
    Binding { request: wire::cayley_request(["1:0", "0:0", "0:0"]), cap: 0, operation, menu: Some(pin(ORDER)) }
}

/// The direct press of `twist`, an item of the pinned menu at its grip.
fn item_of(twist: &Twist) -> Binding {
    let items = pinned().for_cap(&ctx(), twist.grip).expect("the menu at the grip");
    let index = items.iter().position(|item| item.matrix == twist.matrix).expect("an item of the pinned menu");
    Binding { cap: twist.grip as u32, ..item(index as u64) }
}

/// The exact twist the pinned menu resolves `binding` to.
fn resolved(binding: &Binding) -> Twist {
    rotation::resolve(&ctx(), Some(&pinned()), binding).expect("the binding resolves").twist
}

/// The jumbling engine's primitive twists of 600-cell-Full's `letters`.
fn primitives(letters: &[i64]) -> Vec<Twist> {
    letters.iter().map(|&letter| levels::primitive(&ctx(), letter).expect("a letter")).collect()
}

/// The first `n` twists of the I_a fixture's jumbling journal.
fn journal(n: usize) -> Vec<Twist> {
    let records = fixture_json().get("journal").and_then(|journal| journal.get("records")).and_then(Value::as_array);
    let records = records.expect("the fixture's journal");
    records[..n].iter().map(|record| Twist::from_record(&ctx(), record).expect("a journal twist")).collect()
}

/// The pose digest of the jumbling engine's replay of `word` from solved under the pinned menu, flat and with no
/// protected piece: what a chain of pose moves must reach.
fn poses(word: &[Twist]) -> String {
    let ctx = ctx();
    let menu = Arc::new(pinned());
    let identity = ctx.identity(Some(&menu));
    let solved = levels::Snapshot::solved(&ctx, Some(menu));
    let derivation = levels::derive(&ctx, Base::Snapshot(&solved), &identity, word, ProtectionMode::Net, &[]);
    match derivation.expect("no engine error") {
        Ok(levels::Derivation::Derived(derived)) => derived.digest(&ctx),
        _ => panic!("the word does not replay from solved"),
    }
}

/// `jumble.twist.menu`'s arguments for `binding`, naming `revision`: the binding's cap, and the rest of it as
/// `letter`.
fn menu_args(binding: &Binding, revision: &Value) -> Value {
    let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
    let cap = fields.remove("cap").expect("a binding has a cap");
    json::object([("cap", cap), ("letter", Value::Object(fields)), ("revision", revision.clone())])
}

/// `menu_args` with one argument replaced, added or, for `None`, removed.
fn args_with(binding: &Binding, revision: &Value, name: &str, value: Option<Value>) -> Value {
    let Value::Object(mut fields) = menu_args(binding, revision) else { panic!("the arguments are an object") };
    match value {
        Some(value) => fields.insert(name.to_owned(), value),
        None => fields.remove(name),
    };
    Value::Object(fields)
}

/// `menu_args` with one field of `letter` replaced, added or, for `None`, removed.
fn letter_with(binding: &Binding, revision: &Value, name: &str, value: Option<Value>) -> Value {
    let Some(Value::Object(mut fields)) = menu_args(binding, revision).get("letter").cloned() else {
        panic!("a letter object")
    };
    match value {
        Some(value) => fields.insert(name.to_owned(), value),
        None => fields.remove(name),
    };
    args_with(binding, revision, "letter", Some(Value::Object(fields)))
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

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects `orbits`, with no twist menu pinned.
fn unpinned(orbits: &[i64]) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(orbits).unwrap());
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// `unpinned` with the I_a menu pinned.
fn fixture(orbits: &[i64]) -> Fixture {
    let mut f = unpinned(orbits);
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    f
}

/// `fixture` with Jumble on at the solved state's poses.
fn entered(orbits: &[i64]) -> Fixture {
    let mut f = fixture(orbits);
    assert!(committed(&f.enter()));
    f
}

/// A reopen of `store`, which replays its journal: with `menu` pinned before the replay
/// (`CommandLayer::open_with_menu`), or with no menu (`CommandLayer::open`).
fn reopen_store(store: MemoryStore, menu: Option<Menu>) -> Fixture {
    let (table, secret) = (Table::h08().with_drafts(), Secret::fixed(SECRET));
    let layer = match menu {
        Some(menu) => CommandLayer::open_with_menu(kernel(), table, secret, store, ctx(), menu),
        None => CommandLayer::open(kernel(), table, secret, store),
    };
    Fixture { layer: layer.unwrap(), shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// A reopen of `layer`'s store.
fn reopen_with(layer: &CommandLayer, menu: Option<Menu>) -> Fixture {
    reopen_store(layer.store().clone(), menu)
}

impl Fixture {
    /// The `jumble.twist.menu` press for `binding`, naming the current revision, prepared with its token and not sent.
    fn press(&mut self, binding: &Binding) -> Invocation {
        let args = menu_args(binding, &self.layer.revision().to_json());
        self.shell.prepare(&self.layer, JUMBLE_TWIST_ROW, args).unwrap()
    }

    fn menu_twist(&mut self, binding: &Binding) -> Receipt {
        let press = self.press(binding);
        self.shell.send(&mut self.layer, press)
    }

    /// `jumble.on`, naming the current revision.
    fn enter(&mut self) -> Receipt {
        let revision = self.layer.revision().to_json();
        let press = self.shell.prepare(&self.layer, JUMBLE_ON_ROW, json::object([("revision", revision)])).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// `twist.apply` for `binding`.
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

/// CT-EV4-04 (part: the menu-twist leg in the command layer, a pose move admitted and committed once with no preview
/// and no job; not run here: held-key repeats, in `c600-input/tests/repeat.rs`), CT-ONE-03 (part: a live menu
/// twist commits with no preview object; not run: the view model's fields), JI.35, FP.31 (draft: the journal kind
/// `jumble_move`). With Jumble on an item of the pinned menu, a retained one too, is a pose move: the jumbling engine
/// derives the item's exact twist from the head's poses, the revision's digest becomes the new pose digest and the
/// state revision advances, while the mode, the retained state and the other revisions stay. The journal keeps the
/// move's rotation record and the literal of its twist, with no recipe and no crossing record.
#[test]
fn a_pose_move_derives_the_item_from_the_head_s_poses() {
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[2]))));
    assert!(committed(&f.enter()));
    assert_eq!(f.layer.availability(JUMBLE_TWIST_ROW), Ok(()));
    let before = f.layer.revision().clone();
    let labels = f.layer.snapshot().digest().to_owned();
    let press = f.press(&item(12));
    assert!(f.layer.preview().is_none());
    assert_eq!(f.layer.run_jobs(), 0);
    assert_eq!(f.layer.revision(), &before);

    let receipt = f.shell.send(&mut f.layer, press);
    let Status::Committed(record) = &receipt.status else { panic!("{:?}", receipt.status) };
    let twist = resolved(&item(12));
    let mut moves = primitives(&[2]);
    moves.push(twist.clone());
    let pose = poses(&moves);
    assert_ne!(pose, before.digest);
    assert_eq!((record.pre.as_str(), record.post.as_str()), (before.digest.as_str(), pose.as_str()));
    assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&ctx()), pose);
    let after = f.layer.revision().clone();
    assert_eq!(after.mode, JUMBLE_ON_MODE);
    assert_eq!(
        (after.state, after.workspace, after.protection),
        (before.state + 1, before.workspace, before.protection)
    );
    assert_eq!((after.head.as_str(), after.digest.as_str()), ("e3", pose.as_str()));
    assert_eq!(record.revision, after);
    assert_eq!(f.layer.snapshot().digest(), labels, "the retained state stays");
    assert_eq!(receipt.job, None);
    assert!(f.layer.preview().is_none());
    assert_eq!(f.layer.run_jobs(), 0);

    let resolution = rotation::resolve(&ctx(), Some(&pinned()), &item(12)).unwrap();
    let log = f.layer.store().log.clone();
    assert_eq!(log.len(), 3);
    let moved = &log[2];
    let shape = (moved.kind, moved.row.as_str(), &moved.recipe, moved.crossing.as_ref());
    assert_eq!(shape, (TransactionKind::JumbleMove, JUMBLE_TWIST_ROW, &Value::Null, None));
    assert_eq!(moved.rotation.as_ref(), Some(&resolution.to_json()));
    assert_eq!(moved.literal, levels::word_digest(std::slice::from_ref(&twist)));
    assert_eq!(moved.to_json().get("kind"), Some(&text("jumble_move")));
    assert_eq!(moved.receipt(), receipt);

    // From Jumble on at solved, at one cap: a jumble item, a retained item and a drag, each a pose move.
    let mut g = entered(&[]);
    let mut moves = Vec::new();
    let drag = Binding { cap: 7, ..quarter(Operation::MenuDrag) };
    for binding in [Binding { cap: 7, ..item(20) }, Binding { cap: 7, ..item(1) }, drag] {
        let receipt = g.menu_twist(&binding);
        assert!(committed(&receipt), "{:?}", receipt.status);
        moves.push(resolved(&binding));
        assert_eq!(g.layer.revision().digest, poses(&moves));
        let last = g.layer.store().log.last().unwrap().clone();
        let resolution = rotation::resolve(&ctx(), Some(&pinned()), &binding).unwrap();
        assert_eq!((last.kind, last.rotation), (TransactionKind::JumbleMove, Some(resolution.to_json())));
    }
    assert_eq!(g.layer.snapshot().digest(), kernel().solved().digest());
}

/// JI.35 and the draft rulings in `src/lib.rs`: with Jumble off a retained item dispatches to the retained action, its
/// retained word committed as `twist.apply` commits it, with the same literal, state and rotation record and the row's
/// own name in the journal; a jumble item, pressed or reached by a drag, is `KN5:input.menu_word_needs_jumble_on`, and
/// the menu's identity item is `KN2:input.recipe_shape` in both modes. After the entry the jumble item moves the poses.
#[test]
fn with_jumble_off_an_item_dispatches_by_its_kind() {
    let mut f = fixture(&[]);
    let mut g = fixture(&[]);
    assert_eq!(f.layer.availability(JUMBLE_TWIST_ROW), Ok(()));
    let start = f.layer.revision().clone();
    let receipt = f.menu_twist(&item(6));
    let Status::Committed(record) = &receipt.status else { panic!("{:?}", receipt.status) };
    assert!(committed(&g.twist(&item(6))));
    let moved = derived(&kernel().solved(), &word(&[1, -2, 1]));
    assert_eq!((record.pre.as_str(), record.post.as_str()), (start.digest.as_str(), moved.digest()));
    let r = f.layer.revision().clone();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (MODE, "e1", moved.digest()));
    assert_eq!((r.state, r.workspace, r.protection), (start.state + 1, start.workspace, start.protection));
    assert_eq!(record.revision, r);
    assert!(f.layer.jumbled().is_none());
    let (ours, theirs) = (f.layer.store().log[0].clone(), g.layer.store().log[0].clone());
    let shape = (ours.kind, ours.row.as_str(), &ours.recipe);
    assert_eq!(shape, (TransactionKind::Commit, JUMBLE_TWIST_ROW, &word(&[1, -2, 1])));
    assert_eq!((&ours.literal, &ours.post, &ours.rotation), (&theirs.literal, &theirs.post, &theirs.rotation));
    assert_eq!(ours.receipt(), receipt);

    let cases = [
        (item(12), MENU_WORD_NEEDS_JUMBLE_ON, "a jumble item"),
        (quarter(Operation::MenuDrag), MENU_WORD_NEEDS_JUMBLE_ON, "a drag to a jumble item"),
        (item(0), RECIPE_SHAPE, "the identity item"),
    ];
    for (binding, expected, what) in cases {
        let press = f.press(&binding);
        f.refused(press, expected, what);
    }
    assert!(committed(&f.enter()));
    let press = f.press(&item(0));
    f.refused(press, RECIPE_SHAPE, "the identity item with Jumble on");
    assert!(committed(&f.menu_twist(&item(12))));
    let kinds: Vec<TransactionKind> = f.layer.store().log.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [TransactionKind::Commit, TransactionKind::Crossing, TransactionKind::JumbleMove]);
}

/// KN2.4, KN2.7, KN1.11, JI.5 and the owner's reading of 18 November 2026 for finding B5 (protection holds inside
/// Jumble). With Jumble off a retained item's word is judged under the protection set and policy in force, so with
/// orbit 0 protected item 1 is `KN2:protection.orbit_would_move` under Net and under Strict, with nothing changed. With
/// Jumble on the same item is a pose move, judged at the pose level (`m_A(w) = 1`) on the jumbling engine's pieces that
/// carry orbit 0: the engine's `KN2:protection.piece_would_move` under both policies, naming a piece of orbit 0, with
/// nothing changed. No label verdict crosses into Jumble (chapter 03, KN1.11); the pose verdict is the engine's own.
/// From solved, item 1 moves pieces of every orbit, so the protected piece moves only after a release the solver
/// confirmed (LI.26): unconfirmed, the release is `KN5:input.confirmation_required`; confirmed inside Jumble, the same
/// pose move commits under both policies.
#[test]
fn protection_judges_a_retained_item_and_a_pose_move() {
    for policy in ["net", "strict"] {
        let mut f = fixture(&[0]);
        if policy == "strict" {
            assert!(completed(&f.set_policy(policy)));
        }
        let press = f.press(&item(1));
        f.refused(press, code::ORBIT_WOULD_MOVE, policy);
        assert!(committed(&f.enter()), "{policy}");
        let press = f.press(&item(1));
        let refused = f.refused(press, PIECE_WOULD_MOVE, &format!("{policy}: a pose move of orbit 0's pieces"));
        let Status::Refused(error) = &refused.status else { panic!("{policy}: a refusal") };
        let piece = error.detail.rsplit(' ').next().and_then(|piece| piece.parse::<usize>().ok());
        let piece = piece.unwrap_or_else(|| panic!("{policy}: the refusal names a piece: {}", error.detail));
        assert_eq!(ctx().model.orbit_id[piece], 0, "{policy}: the refused piece {piece} carries orbit 0");

        let none = || json::object([("orbits", Value::Array(Vec::new()))]);
        let unconfirmed = f.shell.prepare(&f.layer, "protection.orbit.set", none()).unwrap();
        f.refused(unconfirmed, code::CONFIRMATION_REQUIRED, &format!("{policy}: an unconfirmed release"));
        let Value::Object(mut confirmed) = none() else { panic!("an object") };
        confirmed.insert("confirm".into(), Value::Bool(true));
        let released = f.shell.invoke(&mut f.layer, "protection.orbit.set", Value::Object(confirmed)).unwrap();
        assert!(completed(&released), "{policy}: {:?}", released.status);
        let receipt = f.menu_twist(&item(1));
        assert!(committed(&receipt), "{policy}: {:?}", receipt.status);
        assert_eq!(f.layer.revision().digest, poses(&[resolved(&item(1))]), "{policy}");
        assert!(f.layer.protected().orbits().is_empty(), "{policy}");
    }
}

/// Findings B7 and B5 (the owner's readings of 18 November 2026): a reopen judges a stored pose move again under the
/// protected orbits in force at its place in the journal, since the replay derives it as the live move does. A journal
/// whose pose move moved orbit 0 while no orbit was protected reopens intact. The same journal edited so that orbit 0
/// is protected from the session's start, which the live layer would have refused, reopens in read-only recovery
/// before the pose move with the engine's `KN2:protection.piece_would_move` (a stored commit's refusal is B7's draft
/// `KN5:consistency.unadmitted_journal_action`; the replay passes a pose move's on as the engine gives it).
#[test]
fn a_reopen_judges_a_stored_pose_move_under_the_protection_in_force() {
    let mut f = entered(&[]);
    let at_entry = f.layer.revision().clone();
    assert!(committed(&f.menu_twist(&item(1))));
    let honest = f.layer.store().clone();
    let reopened = reopen_store(honest.clone(), Some(pinned()));
    assert!(reopened.layer.recovery().is_none(), "the honest journal: {:?}", reopened.layer.recovery());

    let mut edited = honest;
    edited.genesis.as_mut().expect("a genesis").orbits = vec![0];
    for transaction in &mut edited.log {
        transaction.orbits = vec![0];
    }
    let reopened = reopen_store(edited, Some(pinned()));
    let recovery = reopened.layer.recovery().expect("the edited journal reopens in read-only recovery");
    assert_eq!(recovery.code, PIECE_WOULD_MOVE, "{recovery:?}");
    let r = reopened.layer.revision();
    assert_eq!((&r.head, &r.digest), (&at_entry.head, &at_entry.digest), "the recovery is before the pose move");
}

/// The input refusals of `input_refusals_change_nothing` that answer the same in both modes, at the revision `now`.
fn input_cases(now: &Value) -> Vec<(&'static str, Value, &'static str)> {
    let one = item(1);
    let free = Binding {
        request: wire::cayley_request(["0:0", "0:0", "0:0"]),
        cap: 0,
        operation: Operation::Free { n: 1, d: 1 },
        menu: None,
    };
    let placeholder = json::object([("axis", Value::Int(3)), ("inverse", Value::Bool(false))]);
    let Value::Object(mut renamed) = menu_args(&one, now) else { panic!("the arguments are an object") };
    let letter = renamed.remove("letter").expect("a letter");
    renamed.insert("rotation".to_owned(), letter);
    let Value::Object(mut undigested) = now.clone() else { panic!("a revision is an object") };
    undigested.remove("digest");
    let both = json::object([("cap", Value::Int(0)), ("letter", text("M|1")), ("revision", text("now"))]);
    let malformed = ROTATION_REQUEST_MALFORMED;
    vec![
        ("no letter", args_with(&one, now, "letter", None), code::ARGUMENT_FORM),
        ("no cap", args_with(&one, now, "cap", None), code::ARGUMENT_FORM),
        ("no revision", args_with(&one, now, "revision", None), code::ARGUMENT_FORM),
        ("a rotation instead of a letter", Value::Object(renamed), code::ARGUMENT_FORM),
        ("an argument the row does not take", args_with(&one, now, "grip", Some(Value::Int(0))), code::ARGUMENT_FORM),
        ("a confirmation", args_with(&one, now, "confirm", Some(Value::Bool(true))), code::ARGUMENT_FORM),
        ("a revision that is text", args_with(&one, now, "revision", Some(text("now"))), code::ARGUMENT_FORM),
        (
            "a revision without its digest",
            args_with(&one, now, "revision", Some(Value::Object(undigested))),
            code::ARGUMENT_FORM,
        ),
        ("a malformed revision and letter: the form first", both, code::ARGUMENT_FORM),
        ("a letter that is text", args_with(&one, now, "letter", Some(text("M|1"))), malformed),
        ("LP3's placeholder letter", args_with(&one, now, "letter", Some(placeholder)), malformed),
        ("a letter with its own cap", letter_with(&one, now, "cap", Some(Value::Int(0))), malformed),
        ("a letter without its menu", letter_with(&one, now, "menu", None), malformed),
        ("a cap that is text", args_with(&one, now, "cap", Some(text("c1"))), malformed),
        ("a negative cap", args_with(&one, now, "cap", Some(Value::Int(-1))), malformed),
        ("a cap past the last pole", args_with(&one, now, "cap", Some(Value::Int(600))), malformed),
        ("a menu operation with a null menu", letter_with(&one, now, "menu", Some(Value::Null)), malformed),
        ("an index outside the menu", menu_args(&item(60), now), malformed),
        ("a menu index dragged", menu_args(&Binding { operation: Operation::MenuDrag, ..item(1) }, now), malformed),
        ("a free binding", menu_args(&free, now), MENU_BINDING_MISMATCH),
        (
            "another menu's order",
            menu_args(&Binding { menu: Some(pin(REVERSED_ORDER)), ..one }, now),
            MENU_BINDING_MISMATCH,
        ),
        ("a direct rotation off the menu", menu_args(&quarter(Operation::MenuDirect), now), TWIST_OFF_MENU),
        ("the identity item", menu_args(&item(0), now), RECIPE_SHAPE),
    ]
}

/// CT-LG2-16 (part: a menu drag that the kernel call refuses (a dragged menu index, or a drag onto a jumble item with
/// Jumble off) is refused and changes nothing; not run: the language half).
/// KN3.7, KN3.8, JI.35, CT-JI-06 (part: the live menu route resolves exact items under the pin, and another order is
/// refused; not run: the draft route, alternate keymaps and Q7, which are LP3's) and H-08's argument form (draft:
/// `letter` is KN3.7's binding without the cap the row names, the form of `twist.apply`'s `rotation`; every input
/// refusal comes before the token, so it changes nothing). The table's argument names and the `revision` argument's
/// form answer `KN5:input.argument_form`; a binding that does not read answers `KN2:input.rotation_request_malformed`;
/// the resolution's refusals keep their codes. With Jumble off a jumble item answers
/// `KN5:input.menu_word_needs_jumble_on` before a forged token; with Jumble on the other refusals keep their codes and
/// a jumble item reaches the token. After them all, a menu twist still commits.
#[test]
fn input_refusals_change_nothing() {
    let mut f = fixture(&[]);
    let now = f.layer.revision().to_json();
    let mut cases = input_cases(&now);
    cases.push(("a jumble item", menu_args(&item(12), &now), MENU_WORD_NEEDS_JUMBLE_ON));
    cases.push(("a drag to a jumble item", menu_args(&quarter(Operation::MenuDrag), &now), MENU_WORD_NEEDS_JUMBLE_ON));
    for (what, arguments, expected) in cases {
        let asked = f.shell.prepare(&f.layer, JUMBLE_TWIST_ROW, arguments).unwrap();
        f.refused(asked, expected, what);
    }
    for (binding, expected) in [(item(12), MENU_WORD_NEEDS_JUMBLE_ON), (item(1), code::TOKEN_INVALID)] {
        let mut forged = f.press(&binding);
        forged.token.as_mut().expect("a token").binding.nonce += 1_000;
        f.refused(forged, expected, "a forged token with Jumble off");
    }
    let stale = args_with(&item(1), &now, "revision", Some(text("now")));
    let mut forged = f.shell.prepare(&f.layer, JUMBLE_TWIST_ROW, stale).unwrap();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::ARGUMENT_FORM, "a malformed revision with a forged token: the form first");

    assert!(committed(&f.enter()));
    let now = f.layer.revision().to_json();
    for (what, arguments, expected) in input_cases(&now) {
        let asked = f.shell.prepare(&f.layer, JUMBLE_TWIST_ROW, arguments).unwrap();
        f.refused(asked, expected, &format!("{what}, with Jumble on"));
    }
    let mut forged = f.press(&item(12));
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::TOKEN_INVALID, "a jumble item with a forged token, with Jumble on");
    assert!(committed(&f.menu_twist(&item(12))));
}

/// CT-ONE-03 (part: a menu twist without a token, or with one bound to other arguments, is refused), CT-ONE-11 (part:
/// a menu twist refused at each step of ONE.80's order leaves every digest unchanged), KN5.18, L15, LI.6. The draft
/// order: the pinned menu (`KN5:input.row_not_built`, also for LP3's placeholder arguments) and the input before the
/// token; then the token, the commit lane and the token's binding; then the named revision, which is current only if
/// every field is equal (EV4.14), so a workspace completion makes a press stale, unlike a `twist.apply` press, and the
/// token's state and protection revisions; then the head hash; then protection: a retained item's label gate, or a
/// pose move's pose verdict in the jumbling engine (finding B5; `protection_judges_a_retained_item_and_a_pose_move`).
/// With Jumble on a press from before the entry or before a pose move is stale too, and a held lane refuses a pose
/// move. The head hash is provoked with Jumble off only: no fault hook replaces the poses.
#[test]
fn refusals_follow_the_commit_order() {
    let mut f = unpinned(&[]);
    assert_eq!(f.layer.availability(JUMBLE_TWIST_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    let press = f.press(&item(1));
    let refused = f.refused(press.clone(), code::ROW_NOT_BUILT, "no pinned menu");
    let mut forged = f.press(&item(1));
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::ROW_NOT_BUILT, "no pinned menu, with a forged token");
    let placeholder = json::object([("axis", Value::Int(3)), ("inverse", Value::Bool(false))]);
    let lp3 = json::object([("cap", text("c1")), ("letter", placeholder), ("revision", f.layer.revision().to_json())]);
    let asked = f.shell.prepare(&f.layer, JUMBLE_TWIST_ROW, lp3).unwrap();
    f.refused(asked, code::ROW_NOT_BUILT, "LP3's placeholder arguments without a pin");
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert_eq!(f.layer.availability(JUMBLE_TWIST_ROW), Ok(()));
    assert_eq!(f.layer.invoke(&Caller::Shell, press), refused, "a refused press keeps its answer");

    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();
    let press = f.press(&item(1));
    f.refused(Invocation { token: None, ..press.clone() }, code::TOKEN_INVALID, "no token");
    let mut forged = press.clone();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::TOKEN_INVALID, "a forged token");
    // An authentic token's refusal attaches to its command ID, so this case takes a press of its own.
    let other = f.press(&item(1));
    let moved = menu_args(&item(2), &f.layer.revision().to_json());
    f.refused(Invocation { args: moved, ..other }, code::TOKEN_MISMATCH, "a token bound to other arguments");

    f.layer.hold_commit_lane(true);
    assert_eq!(f.layer.availability(JUMBLE_TWIST_ROW).unwrap_err().code, code::COMMIT_IN_FLIGHT);
    let held = f.press(&item(1));
    let busy = f.refused(held.clone(), code::COMMIT_IN_FLIGHT, "a held lane");
    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.invoke(&Caller::Shell, held), busy, "a refused press keeps its answer");

    assert!(committed(&f.menu_twist(&item(2))));
    f.refused(press, code::REVISION_CHANGED, "a press from before a commit");
    let press = f.press(&item(1));
    assert!(completed(&f.protect(&[0])));
    f.refused(press, code::REVISION_CHANGED, "a press from before a protection change");
    let press = f.press(&item(1));
    f.refused(press, code::ORBIT_WOULD_MOVE, "a fresh press that moves the protected orbit");
    let press = f.press(&item(1));
    assert!(completed(&f.set_policy("strict")));
    f.refused(press, code::REVISION_CHANGED, "a press from before a policy change");
    let press = f.press(&item(1));
    assert!(running(&f.shell.invoke(&mut f.layer, "tool.full.note", json::object([])).unwrap()));
    let job = f.layer.take_program_job().unwrap().job;
    assert!(completed(&f.layer.finish_program_job(job, Ok(text("note"))).unwrap()));
    f.refused(press, code::REVISION_CHANGED, "a press from before a workspace completion");

    // The head hash comes before the protection gate, which would refuse item 1 here.
    let head = f.layer.snapshot().clone();
    f.layer.fault_replace_head(derived(&head, &word(&[3])));
    let press = f.press(&item(1));
    f.refused(press, code::STATE_HASH_MISMATCH, "a head that differs");
    f.layer.fault_replace_head(head);

    // A pose move cannot move a protected orbit's pieces (finding B5), so the pose moves below come after a release
    // of orbit 0 that the solver confirmed.
    let none = json::object([("confirm", Value::Bool(true)), ("orbits", Value::Array(Vec::new()))]);
    assert!(completed(&f.shell.invoke(&mut f.layer, "protection.orbit.set", none).unwrap()));
    let stale = f.press(&item(12));
    assert!(committed(&f.enter()));
    f.refused(stale, code::REVISION_CHANGED, "a press from before the entry");
    let stale = f.press(&item(12));
    assert!(committed(&f.menu_twist(&item(13))));
    f.refused(stale, code::REVISION_CHANGED, "a press from before a pose move");
    f.layer.hold_commit_lane(true);
    assert_eq!(f.layer.availability(JUMBLE_TWIST_ROW).unwrap_err().code, code::COMMIT_IN_FLIGHT);
    let held = f.press(&item(12));
    f.refused(held, code::COMMIT_IN_FLIGHT, "a held lane with Jumble on");
    f.layer.hold_commit_lane(false);
    assert!(committed(&f.menu_twist(&item(12))));
}

/// KN2.8 in the jumbling engine (draft: a pose move derives its twist under JI's admissibility). The first eight twists
/// of the I_a fixture's journal are admissible from solved and move the poses off the lattice; there the retained
/// primitive 3, at grip 1, is blocked, so its menu twist is `KN2:input.word_blocked_prefix` and changes nothing, while
/// primitive 1, at grip 0, still moves. A cap's ball is invariant under the cap's own rotations, so the exact inverse of
/// a twist is admissible where the twist landed: the nine moves undo one by one, each to the poses of its word's
/// prefix, back to the entry's.
#[test]
fn a_blocked_twist_is_refused_at_its_prefix() {
    let mut f = entered(&[]);
    let entry = f.layer.revision().digest.clone();
    assert_eq!(entry, poses(&[]));
    let mut word = journal(8);
    for (index, twist) in word.iter().enumerate() {
        let receipt = f.menu_twist(&item_of(twist));
        assert!(committed(&receipt), "journal twist {index}: {:?}", receipt.status);
    }
    assert_eq!(f.layer.revision().digest, poses(&word));
    let [blocked, open] = [3, 1].map(|letter| levels::primitive(&ctx(), letter).expect("a letter"));
    assert_eq!((blocked.grip, open.grip), (1, 0));
    let press = f.press(&item_of(&blocked));
    f.refused(press, WORD_BLOCKED_PREFIX, "a twist the poses block");
    assert!(committed(&f.menu_twist(&item_of(&open))));
    word.push(open);
    assert_eq!(f.layer.revision().digest, poses(&word));

    for k in (0..word.len()).rev() {
        let undone = f.step(Direction::Undo);
        assert!(committed(&undone), "the undo to {k} moves: {:?}", undone.status);
        assert_eq!(f.layer.revision().digest, poses(&word[..k]), "the undo to {k}");
    }
    assert_eq!((f.layer.revision().mode, f.layer.revision().digest.as_str()), (JUMBLE_ON_MODE, entry.as_str()));
}

/// JI.27 (draft: the undo of a Jumble move replays the exact inverse of its twist and the redo the twist again, and
/// each must reach the recorded poses of its target). After a retained item and the entry, three pose moves at one cap
/// reach the jumbling engine's replay of their whole word from solved. Their undos step back through the recorded
/// poses, each with the inverse's literal, a committed receipt and a new state revision; the undo of the entry returns
/// to Jumble off, and the redos replay the entry and each move again with the twist's literal. A new move after an undo
/// clears the redo.
#[test]
fn undo_and_redo_step_pose_moves_by_their_exact_inverse() {
    let mut f = fixture(&[]);
    assert!(committed(&f.menu_twist(&item(6))));
    let off = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    let bindings = [Binding { cap: 7, ..item(20) }, Binding { cap: 7, ..item(1) }, Binding { cap: 7, ..item(33) }];
    let twists: Vec<Twist> = bindings.iter().map(resolved).collect();
    let mut revisions = vec![f.layer.revision().clone()];
    for binding in &bindings {
        assert!(committed(&f.menu_twist(binding)));
        revisions.push(f.layer.revision().clone());
    }
    let mut word = primitives(&[1, -2, 1]);
    word.extend(twists.iter().cloned());
    assert_eq!(revisions[3].digest, poses(&word));

    for (index, twist) in twists.iter().enumerate().rev() {
        let state = f.layer.revision().state;
        let undone = f.step(Direction::Undo);
        let Status::Committed(record) = &undone.status else { panic!("{:?}", undone.status) };
        let (from, to) = (&revisions[index + 1], &revisions[index]);
        assert_eq!((record.pre.as_str(), record.post.as_str()), (from.digest.as_str(), to.digest.as_str()));
        let r = f.layer.revision().clone();
        assert_eq!(
            (r.mode, r.head.as_str(), r.digest.as_str()),
            (JUMBLE_ON_MODE, to.head.as_str(), to.digest.as_str())
        );
        assert_eq!(r.state, state + 1);
        assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&ctx()), to.digest);
        let inverse = Twist::new(&ctx(), twist.grip, transpose(&twist.matrix)).expect("the exact inverse");
        let last = f.layer.store().log.last().unwrap().clone();
        assert_eq!(
            (last.kind, last.literal, last.rotation),
            (TransactionKind::Undo, levels::word_digest(&[inverse]), None)
        );
    }
    assert!(committed(&f.step(Direction::Undo)));
    let r = f.layer.revision().clone();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (MODE, off.head.as_str(), off.digest.as_str()));
    assert!(f.layer.jumbled().is_none());

    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.layer.revision().digest, revisions[0].digest);
    for (index, twist) in twists.iter().enumerate() {
        let redone = f.step(Direction::Redo);
        let Status::Committed(record) = &redone.status else { panic!("{:?}", redone.status) };
        let (from, to) = (&revisions[index], &revisions[index + 1]);
        assert_eq!((record.pre.as_str(), record.post.as_str()), (from.digest.as_str(), to.digest.as_str()));
        assert_eq!((f.layer.revision().head.as_str(), f.layer.revision().mode), (to.head.as_str(), JUMBLE_ON_MODE));
        let last = f.layer.store().log.last().unwrap().clone();
        assert_eq!(
            (last.kind, last.literal),
            (TransactionKind::Redo, levels::word_digest(std::slice::from_ref(twist)))
        );
    }

    assert!(committed(&f.step(Direction::Undo)));
    let other = Binding { cap: 7, ..item(2) };
    assert!(committed(&f.menu_twist(&other)));
    assert!(f.layer.undo_hint(Direction::Redo).is_none());
    word.truncate(word.len() - 1);
    word.push(resolved(&other));
    assert_eq!(f.layer.revision().digest, poses(&word));
    let kinds: Vec<TransactionKind> = f.layer.store().log.iter().map(|t| t.kind).collect();
    use TransactionKind::{Commit, Crossing, JumbleMove, Redo, Undo};
    let expected =
        [Commit, Crossing, JumbleMove, JumbleMove, JumbleMove, Undo, Undo, Undo, Undo, Redo, Redo, Redo, Redo];
    assert_eq!(kinds, [&expected[..], &[Undo, JumbleMove]].concat());
}

/// The chain bound (draft: the jumbling engine nests each derivation's evidence in the next, so after 64 chained moves
/// the layer replays the jumbling journal flat from solved, which must reach the chained poses). Each of seventy pose
/// moves reaches the flat replay of its word's prefix; a reopen under the menu restores them, and seventy undos step
/// back through every prefix to the entry's poses and seventy redos forward to the last move's.
#[test]
fn a_long_pose_chain_replays_flat_and_undoes() {
    let mut f = entered(&[]);
    let menu = pinned();
    let bindings: Vec<Binding> = (0..70).map(|index| item(1 + index % 59)).collect();
    let word: Vec<Twist> = bindings
        .iter()
        .map(|b| rotation::resolve(&ctx(), Some(&menu), b).expect("the binding resolves").twist)
        .collect();
    let expected: Vec<String> = (0..=word.len()).map(|k| poses(&word[..k])).collect();
    assert_eq!(f.layer.revision().digest, expected[0]);
    for (index, binding) in bindings.iter().enumerate() {
        let receipt = f.menu_twist(binding);
        assert!(committed(&receipt), "move {index}: {:?}", receipt.status);
        assert_eq!(f.layer.revision().digest, expected[index + 1], "move {index}");
    }
    let last = f.layer.revision().clone();
    assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&ctx()), last.digest);

    let mut g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    for k in (0..word.len()).rev() {
        assert!(committed(&g.step(Direction::Undo)), "the undo to {k}");
        assert_eq!(g.layer.revision().digest, expected[k], "the undo to {k}");
    }
    for k in 1..=word.len() {
        assert!(committed(&g.step(Direction::Redo)), "the redo to {k}");
        assert_eq!(g.layer.revision().digest, expected[k], "the redo to {k}");
    }
    assert_eq!(g.layer.jumbled().expect("Jumble is on").digest(&ctx()), last.digest);
}

/// The chain bound keeps a long session within a test thread's stack (draft): the jumbling engine nests each
/// derivation's evidence in the next, and some hundreds of chained derivations overflow a debug test thread's stack,
/// so a thousand pose moves commit only because the layer replays the jumbling journal flat after 64 chained moves.
/// They reach the flat replay of their word.
#[test]
fn a_thousand_pose_moves_stay_within_the_stack() {
    let mut f = entered(&[]);
    let menu = pinned();
    let bindings: Vec<Binding> = (0..1000).map(|index| item(1 + index % 59)).collect();
    for (index, binding) in bindings.iter().enumerate() {
        let receipt = f.menu_twist(binding);
        assert!(committed(&receipt), "move {index}: {:?}", receipt.status);
    }
    let word: Vec<Twist> = bindings
        .iter()
        .map(|b| rotation::resolve(&ctx(), Some(&menu), b).expect("the binding resolves").twist)
        .collect();
    let target = poses(&word);
    assert_eq!(f.layer.revision().digest, target);
    assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&ctx()), target);
}

/// KN5.23, ONE.58 and JI.27's recovery row (draft): a retained commit through the row replays without a menu, as a
/// `twist.apply` commit does. A reopen with the session's twist menu pinned before the replay
/// (`CommandLayer::open_with_menu`) replays each pose move's recorded exact twist (KN3.9), under the policy in force,
/// and restores the mode, the head, the revisions and the poses; undo, redo and new moves work there. Without the
/// menu, or under a menu with another identity block, the layer is in read-only recovery at Jumble off before the
/// crossing. A jumble move journaled under another row, or a retained commit while Jumble is on, does not replay: the
/// layer is in read-only recovery after the last action before it, at Jumble on.
#[test]
fn a_reopen_replays_pose_moves_under_the_menu() {
    let mut f = fixture(&[]);
    assert!(committed(&f.menu_twist(&item(6))));
    let g = reopen_with(&f.layer, None);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    let off = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    assert!(completed(&f.set_policy("strict")));
    assert!(committed(&f.menu_twist(&Binding { cap: 7, ..item(20) })));
    assert!(committed(&f.menu_twist(&Binding { cap: 7, ..item(33) })));
    let moved = f.layer.revision().clone();
    assert!(committed(&f.step(Direction::Undo)));
    let log = f.layer.store().log.clone();

    let mut g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.jumbled().expect("Jumble is on").digest(&ctx()), f.layer.revision().digest);
    assert_eq!(g.layer.store().log, log);
    assert!(committed(&g.step(Direction::Redo)));
    assert_eq!(g.layer.revision().digest, moved.digest);
    assert!(committed(&g.menu_twist(&Binding { cap: 7, ..item(1) })));
    let h = reopen_with(&g.layer, Some(pinned()));
    assert!(h.layer.recovery().is_none(), "{:?}", h.layer.recovery());
    assert_eq!(restored(&h.layer), restored(&g.layer));
    assert_eq!(h.layer.jumbled().expect("Jumble is on").digest(&ctx()), g.layer.revision().digest);

    for (menu, what) in [(None, "no menu"), (Some(reversed()), "another identity block")] {
        let h = reopen_with(&f.layer, menu);
        assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "{what}");
        let r = h.layer.revision();
        let at = (r.mode, r.head.as_str(), r.digest.as_str());
        assert_eq!(at, (MODE, off.head.as_str(), off.digest.as_str()), "{what}");
        assert!(h.layer.jumbled().is_none(), "{what}");
    }

    let before = log[2].revision.clone();
    assert_eq!(log[3].kind, TransactionKind::JumbleMove);
    let edits: [(&str, fn(&mut Transaction)); 2] = [
        ("a jumble move under another row", |t| t.row = TWIST_ROW.to_owned()),
        ("a retained commit while Jumble is on", |t| {
            t.kind = TransactionKind::Commit;
            t.recipe = word(&[1]);
            t.rotation = None;
        }),
    ];
    for (what, edit) in edits {
        let mut store = f.layer.store().clone();
        edit(&mut store.log[3]);
        let h = reopen_store(store, Some(pinned()));
        assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "{what}");
        let r = h.layer.revision();
        let at = (r.mode, r.head.as_str(), r.digest.as_str(), r.state);
        assert_eq!(at, (JUMBLE_ON_MODE, before.head.as_str(), before.digest.as_str(), before.state), "{what}");
        assert_eq!(h.layer.jumbled().expect("Jumble is on").digest(&ctx()), before.digest, "{what}");
    }
}

/// The journal form (draft: FP.31's `jumble_move`): a pose move is the action kind `jumble_move` and carries its
/// rotation record under `rotation`, an 18th key, as the row's retained commit does; each reads back to itself. A
/// jumble move without its rotation or with a crossing record, a rotation on an undo or an entry, the row's retained
/// commit without its rotation, a rotation record with an extra field, and a jumble move or a crossing with a recipe or
/// a confirmation do not read. The form does not check the row of a jumble move, which a reopen's replay does.
#[test]
fn a_pose_move_journals_its_rotation() {
    let mut f = fixture(&[]);
    assert!(committed(&f.menu_twist(&item(1))));
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(12))));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    let log = f.layer.store().log.clone();
    let keys: Vec<usize> = log.iter().map(|t| t.to_json().as_object().unwrap().len()).collect();
    assert_eq!(keys, [18, 18, 18, 17, 17]);
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }
    assert_eq!(log[2].to_json().get("kind"), Some(&text("jumble_move")));
    let rotation = log[2].rotation.clone().expect("the move's rotation");
    assert_eq!(log[2].to_json().get("rotation"), Some(&rotation));
    let crossing = log[1].crossing.clone().expect("the entry's record");

    let carrying = |transaction: &Transaction, key: &str, value: Option<Value>| {
        let Value::Object(mut fields) = transaction.to_json() else { panic!("a transaction is an object") };
        match value {
            Some(value) => fields.insert(key.to_owned(), value),
            None => fields.remove(key),
        };
        Value::Object(fields)
    };
    let mut extra = rotation.clone();
    if let Value::Object(fields) = &mut extra {
        fields.insert("extra".to_owned(), Value::Null);
    }
    let malformed = [
        (carrying(&log[2], "rotation", None), "a jumble move without its rotation"),
        (carrying(&log[2], "rotation", Some(Value::Null)), "a null rotation"),
        (carrying(&log[2], "crossing", Some(crossing)), "a jumble move with a crossing record"),
        (carrying(&log[3], "rotation", Some(rotation.clone())), "an undo with a rotation"),
        (carrying(&log[1], "rotation", Some(rotation.clone())), "an entry with a rotation"),
        (carrying(&log[0], "rotation", None), "the row's retained commit without its rotation"),
        (carrying(&log[2], "rotation", Some(extra)), "a rotation record with an extra field"),
        (carrying(&log[2], "recipe", Some(word(&[1]))), "a jumble move with a recipe"),
        (carrying(&log[2], "confirm", Some(Value::Bool(true))), "a jumble move with a confirmation"),
        (carrying(&log[1], "recipe", Some(word(&[1]))), "a crossing with a recipe"),
        (carrying(&log[1], "confirm", Some(Value::Bool(true))), "a crossing with a confirmation"),
    ];
    for (value, what) in malformed {
        assert_eq!(Transaction::from_json(&value), None, "{what}");
    }
    let other = carrying(&log[2], "row", Some(text(TWIST_ROW)));
    assert!(Transaction::from_json(&other).is_some(), "the journal form does not check a jumble move's row");
}

/// CT-EV4-04 (part: a menu twist is admitted once: a retried delivery answers the stored receipt and moves nothing
/// more, also after a restart and a reopen; the same command ID with other content is `KN5:input.command_id_reused`,
/// and the spent token under another command ID is `KN5:auth.token_used`), CT-KN5-04 (part: the same for a pose move),
/// KN5.20, KN5.21. A restart keeps Jumble on.
#[test]
fn a_retried_move_answers_its_receipt_and_moves_once() {
    let mut f = entered(&[]);
    let press = f.press(&item(12));
    let done = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&done), "{:?}", done.status);
    let kept = f.fixed();
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done);
    assert_eq!(f.fixed(), kept);
    let revision = press.args.get("revision").expect("a revision").clone();
    let other = Invocation { args: menu_args(&item(13), &revision), ..press.clone() };
    f.refused(other, code::COMMAND_ID_REUSED, "other content under the command ID");
    let replay = Invocation { command: "replay/1".into(), ..press.clone() };
    f.refused(replay, code::TOKEN_USED, "the spent token under another command ID");

    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome(&press.command, &done.request), Some(done.clone()));
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done);
    assert_eq!(f.layer.store().log.len(), 2);
    assert_eq!(f.layer.jumbled().expect("a restart keeps Jumble on").digest(&ctx()), kept.0);

    let mut g = reopen_with(&f.layer, Some(pinned()));
    assert_eq!(g.layer.outcome(&press.command, &done.request), Some(done.clone()));
    assert_eq!(g.layer.invoke(&Caller::Shell, press), done);
    assert_eq!(g.layer.store().log.len(), 2);
}

/// LI.32 and KN5.18 (draft: a menu twist ends what read the state it replaces, as a word commit does): with Jumble off
/// a retained item's commit discards the pending preview, so a press for it is `KN5:stale.no_fresh_preview`; with
/// Jumble on a pose move cancels a running and a queued query job with `replaced_revision` and drops a late result,
/// while a file job keeps running on the revision it read, whose mode is `jumble_on`.
#[test]
fn a_menu_twist_ends_the_preview_and_the_jobs_that_read_the_state() {
    let mut f = fixture(&[]);
    let p = f.preview(&word(&[2]));
    f.present(p);
    let execute =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(p as i64))])).unwrap();
    assert!(committed(&f.menu_twist(&item(1))));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Discarded));
    assert!(f.layer.preview().is_none());
    f.refused(execute, code::NO_FRESH_PREVIEW, "the press for the discarded preview");

    f.layer.register_overlay("tool.full.inspect", Kind::Query, &[], overlay("inspect")).unwrap();
    f.layer.register_overlay("tool.full.scan", Kind::Query, &[], overlay("scan")).unwrap();
    let granted = c600_command::Registration::default().granting(c600_command::Grant::File);
    f.layer.register_overlay_with("tool.full.export", Kind::File, &[], overlay("export"), granted).unwrap();
    f.layer.set_program_capacity(3);
    assert!(committed(&f.enter()));
    let inspect = f.shell.invoke(&mut f.layer, "tool.full.inspect", json::object([])).unwrap();
    let scan = f.shell.invoke(&mut f.layer, "tool.full.scan", json::object([])).unwrap();
    let export = f.shell.invoke(&mut f.layer, "tool.full.export", json::object([])).unwrap();
    assert!(running(&inspect) && running(&scan) && running(&export));
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(taken.row, "tool.full.inspect");

    assert!(committed(&f.menu_twist(&item(12))));
    for receipt in [&inspect, &scan] {
        let ended = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert_eq!(ended.status, Status::Cancelled(CancelReason::ReplacedRevision));
    }
    assert_eq!(f.layer.outcome(&export.command, &export.request).as_ref(), Some(&export));
    assert!(f.layer.finish_program_job(taken.job, Ok(Value::Null)).is_none());
    let export = f.layer.take_program_job().unwrap();
    assert_eq!((export.row.as_str(), export.revision.mode), ("tool.full.export", JUMBLE_ON_MODE));
    assert_eq!(f.layer.store().log.len(), 3);
}

/// KN5.15: the runtime may not invoke a commit row, so its `jumble.twist.menu` is `KN5:input.commit_row_refused` and
/// changes nothing, with or without a pinned menu and with Jumble on.
#[test]
fn the_runtime_may_not_twist() {
    for mut f in [fixture(&[]), unpinned(&[]), entered(&[])] {
        let runtime = Caller::Runtime { run: "run-1".into() };
        let epoch = f.layer.revision().epoch;
        let args = menu_args(&item(1), &f.layer.revision().to_json());
        let theirs = Invocation {
            command: "run-1/1".into(),
            row: JUMBLE_TWIST_ROW.into(),
            args,
            token: None,
            epoch,
            budget: None,
        };
        f.refused_on(&runtime, theirs, code::COMMIT_ROW_REFUSED, "the runtime");
    }
}
