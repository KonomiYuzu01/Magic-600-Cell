//! `jumble.off` in the command layer (draft; A5, ONE.54, ONE.80, JI.27, FP.31): A5's exit from Jumble on back to Jumble
//! off at the label checkpoint, where the poses' labels become 600-cell-Full's retained state, and the entry that
//! follows it from the exit's root (JI.25, JI.26, FP.32). Each test names the conformance rows it covers headless with
//! PHS.62 markers. Where the frozen text is silent, the test follows the draft rulings recorded in `src/lib.rs` and
//! says so.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::{JUMBLE_ON_MODE, MODE};
use c600_command::{
    code, digest, draft_args, identity_json, Caller, CancelReason, CommandLayer, Completion, CompletionKind, Direction,
    HeadlessFrames, Invocation, Kind, MemoryStore, Overlay, Receipt, Secret, Shell, Status, Table, Transaction,
    TransactionKind, JUMBLE_OFF_ROW, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, POLICY_ROW, TWIST_ROW,
};
use c600_jumble::codes::MENU_WORD_NEEDS_JUMBLE_ON;
use c600_jumble::levels::{self, Base};
use c600_jumble::nu;
use c600_jumble::q5::transpose;
use c600_jumble::rotation;
use c600_jumble::twist::{Menu, Twist};
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

/// Menu item `index` at cap 0, pressed directly. At cap 0 item 0 is the identity, items 1 to 11 are retained and 12 to
/// 59 are jumble items.
fn item(index: u64) -> Binding {
    Binding {
        request: wire::menu_request(SET, ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: SET.to_owned(), order: ORDER.to_owned() }),
    }
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

/// The retained word of `binding`, a retained item of the pinned menu (JI.35), as a recipe.
fn retained(binding: &Binding) -> Value {
    let resolution = rotation::resolve(&ctx(), Some(&pinned()), binding).expect("the binding resolves");
    word(&rotation::retained_word(&ctx(), &resolution).expect("a retained item"))
}

/// The exact inverse of `twist`, at its grip.
fn inverse(twist: &Twist) -> Twist {
    Twist::new(&ctx(), twist.grip, transpose(&twist.matrix)).expect("the exact inverse")
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
/// protected piece.
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

/// The flat reachability claim of the same replay: the solved claim carried along `word`.
fn flat_claim(word: &[Twist]) -> levels::Reachability {
    let ctx = ctx();
    let menu = Arc::new(pinned());
    let identity = ctx.identity(Some(&menu));
    let solved = levels::Snapshot::solved(&ctx, Some(menu));
    let derivation = levels::derive(&ctx, Base::Snapshot(&solved), &identity, word, ProtectionMode::Net, &[]);
    match derivation.expect("no engine error") {
        Ok(levels::Derivation::Derived(derived)) => derived.reachability().clone(),
        _ => panic!("the word does not replay from solved"),
    }
}

/// The exit record (draft) that A5's label checkpoint makes when it ends the episode `episode` along the jumbling
/// journal `journal` at the poses `poses` with the labels `labels`, ν = 0 shown by `source`.
fn exit_record(episode: &str, journal: &[Twist], poses: &str, labels: &str, source: &str) -> Value {
    let certificate = json::object([
        ("c5_convention", text(nu::C5_CONVENTION)),
        ("contract_revision", text(c600_jumble::CONTRACT_REVISION)),
        ("frame_agrees", Value::Bool(true)),
        ("nu", Value::Array(vec![Value::Int(0); levels::NU_LEN])),
        ("nu_source", text(source)),
        ("program", text(nu::CERTIFICATE_PROGRAM_SHA256)),
    ]);
    let derivation =
        json::object([("letters", Value::Int(journal.len() as i64)), ("word", text(&levels::word_digest(journal)))]);
    json::object([
        ("certificate", certificate),
        ("episode", text(episode)),
        ("identity", identity_json(&ctx().identity(Some(&pinned())))),
        ("journal", derivation),
        ("labels", text(labels)),
        ("poses", text(poses)),
    ])
}

/// `Digest<literal word>` of 600-cell-Full's letters: the SHA-256 of the letters as little-endian int32.
fn literal_digest(letters: &[i64]) -> String {
    let bytes: Vec<u8> = letters.iter().flat_map(|&letter| (letter as i32).to_le_bytes()).collect();
    c600_digest::sha256_hex(&bytes)
}

/// The crossing record (draft; FP.31's `exit_root`) of an entry from the exit with the event `crossing` and the record
/// `exit`, along the retained suffix `letters`.
fn exit_root(crossing: &str, exit: &Value, letters: &[i64]) -> Value {
    json::object([
        ("crossing", text(crossing)),
        ("entrance", text("exit_root")),
        ("exit", text(&digest(exit))),
        ("identity", identity_json(&ctx().identity(Some(&pinned())))),
        ("letters", Value::Int(letters.len() as i64)),
        ("word", text(&literal_digest(letters))),
    ])
}

/// `twist.apply`'s arguments for `binding`: its cap, and the rest of the binding as `rotation`.
fn twist_args(binding: &Binding) -> Value {
    let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
    let cap = fields.remove("cap").expect("a binding has a cap");
    json::object([("cap", cap), ("rotation", Value::Object(fields))])
}

/// The object `fields` with the field `name` replaced or, for `None`, removed.
fn with(fields: &Value, name: &str, value: Option<Value>) -> Value {
    let Value::Object(mut fields) = fields.clone() else { panic!("an object") };
    match value {
        Some(value) => fields.insert(name.to_owned(), value),
        None => fields.remove(name),
    };
    Value::Object(fields)
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

/// `jumble.off`'s arguments naming the revision `revision` with one field edited.
fn revision_with(revision: &Value, edit: impl FnOnce(&mut BTreeMap<String, Value>)) -> Value {
    let Value::Object(mut fields) = revision.clone() else { panic!("a revision is an object") };
    edit(&mut fields);
    json::object([("revision", Value::Object(fields))])
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
    /// The `jumble.off` press, naming the current revision, prepared with its token and not sent.
    fn press(&mut self) -> Invocation {
        let revision = self.layer.revision().to_json();
        self.shell.prepare(&self.layer, JUMBLE_OFF_ROW, json::object([("revision", revision)])).unwrap()
    }

    fn exit(&mut self) -> Receipt {
        let press = self.press();
        self.shell.send(&mut self.layer, press)
    }

    /// The `jumble.on` press, naming the current revision, prepared with its token and not sent.
    fn enter_press(&mut self) -> Invocation {
        let revision = self.layer.revision().to_json();
        self.shell.prepare(&self.layer, JUMBLE_ON_ROW, json::object([("revision", revision)])).unwrap()
    }

    fn enter(&mut self) -> Receipt {
        let press = self.enter_press();
        self.shell.send(&mut self.layer, press)
    }

    /// `jumble.twist.menu` for `binding`, naming the current revision: with Jumble on, a pose move.
    fn menu_twist(&mut self, binding: &Binding) -> Receipt {
        let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
        let cap = fields.remove("cap").expect("a binding has a cap");
        let revision = self.layer.revision().to_json();
        let args = json::object([("cap", cap), ("letter", Value::Object(fields)), ("revision", revision)]);
        let press = self.shell.prepare(&self.layer, JUMBLE_TWIST_ROW, args).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// `twist.apply` for `binding`: with Jumble off, a retained commit.
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

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let id = self.preview(recipe);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let press =
            self.shell.prepare(&self.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))]));
        self.shell.send(&mut self.layer, press.unwrap())
    }

    /// The undo or redo of the action on top of its stack; `confirm` confirms a release (LI.26).
    fn step_with(&mut self, direction: Direction, confirm: bool) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        let mut args = BTreeMap::from([("hint".to_owned(), hint.to_json())]);
        if confirm {
            args.insert("confirm".to_owned(), Value::Bool(true));
        }
        self.shell.invoke(&mut self.layer, row, Value::Object(args)).unwrap()
    }

    fn step(&mut self, direction: Direction) -> Receipt {
        self.step_with(direction, false)
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

    /// What no refusal may change: the state digest, the revision, the durable log, the policy, the poses and the
    /// retained state, which with Jumble on is not the revision's digest.
    fn fixed(&self) -> (String, String, usize, ProtectionMode, Option<String>, String) {
        let r = self.layer.revision();
        let poses = self.layer.jumbled().map(|jumbled| jumbled.digest(&ctx()));
        let retained = self.layer.snapshot().digest().to_owned();
        (r.digest.clone(), format!("{r:?}"), self.layer.store().log.len(), self.layer.policy(), poses, retained)
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

/// CT-ONE-17 (part: a snapshot held across an exit keeps its labels and digest; not run: a snapshot of the jumbled
/// puzzle's own level, which LP3's runtime opens).
/// CT-JI-01 (part: an exit at the label checkpoint makes the poses' labels the retained state; not run: branch
/// navigation, restore, import and the roots and void facts, which are not built), CT-ONE-03 (part: an exit commits
/// with no preview object and no job), A5, ONE.54, JI.27, FP.31 (draft: the journal kind `exit`). At the entry's poses
/// from solved the identity labels show ν = 0; after a retained item as a pose move a retained witness shows it; after
/// a jumble item and its exact inverse the ν row computes it. Each exit is a new node with the labels' digest: the
/// revision's mode becomes `jumble_off`, its state revision advances, its head is the exit's event and its digest the
/// label digest, while the workspace and protection revisions stay. The retained state is the labels' state of
/// 600-cell-Full, and the journal keeps the exit's record (draft: the episode, the identity block, the jumbling
/// journal, the pose and label digests and A5's certificate).
#[test]
fn an_exit_takes_the_labels_of_its_poses() {
    let solved = kernel().solved();
    let mut f = fixture(&[]);
    assert_eq!(f.layer.availability(JUMBLE_OFF_ROW).unwrap_err().code, MENU_WORD_NEEDS_JUMBLE_ON);
    assert!(committed(&f.enter()));
    assert_eq!(f.layer.availability(JUMBLE_OFF_ROW), Ok(()));
    let on = f.layer.revision().clone();
    let held = f.layer.snapshot().clone();
    let held_labels = held.labels().to_vec();
    let receipt = f.exit();
    let Status::Committed(record) = &receipt.status else { panic!("{:?}", receipt.status) };
    assert_eq!((held.digest(), held.labels()), (solved.digest(), held_labels.as_slice()), "a held snapshot");
    assert_eq!((record.pre.as_str(), record.post.as_str()), (on.digest.as_str(), solved.digest()));
    let off = f.layer.revision().clone();
    assert_eq!((off.mode, off.head.as_str(), off.digest.as_str()), (MODE, "e2", solved.digest()));
    assert_eq!((off.state, off.workspace, off.protection), (on.state + 1, on.workspace, on.protection));
    assert_eq!(record.revision, off);
    assert!(f.layer.jumbled().is_none());
    assert_eq!(f.layer.snapshot().digest(), solved.digest());
    assert_eq!(receipt.job, None);
    assert!(f.layer.preview().is_none());
    assert_eq!(f.layer.run_jobs(), 0);
    assert_eq!(f.layer.availability(TWIST_ROW), Ok(()));
    assert_eq!(f.layer.availability(JUMBLE_OFF_ROW).unwrap_err().code, MENU_WORD_NEEDS_JUMBLE_ON);

    let log = f.layer.store().log.clone();
    assert_eq!(log.len(), 2);
    let exit = &log[1];
    let shape = (exit.kind, exit.row.as_str(), &exit.recipe, exit.literal.as_str(), exit.confirm);
    assert_eq!(shape, (TransactionKind::Exit, JUMBLE_OFF_ROW, &Value::Null, "", false));
    assert_eq!(exit.rotation, None);
    assert_eq!(exit.receipt(), receipt);
    assert_eq!(exit.to_json().get("kind"), Some(&text("exit")));
    let expected = exit_record("e1", &[], &on.digest, solved.digest(), "identity_labels");
    assert_eq!(exit.crossing.as_ref(), Some(&expected));

    // A retained item as a pose move: the retained witness shows ν = 0, and the labels are its retained word's.
    let mut f = fixture(&[]);
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    let on = f.layer.revision().clone();
    assert!(committed(&f.exit()));
    let labels = derived(&solved, &word(&[1, -2, 1]));
    assert_eq!(f.layer.snapshot().digest(), labels.digest());
    assert_eq!(f.layer.revision().digest, labels.digest());
    let expected = exit_record("e1", &[resolved(&item(6))], &on.digest, labels.digest(), "retained_witness");
    assert_eq!(f.layer.store().log[2].crossing.as_ref(), Some(&expected));

    // A jumble item and its exact inverse after a retained commit: the ν row shows ν = 0 at the entry's poses.
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[2]))));
    let labels = f.layer.snapshot().clone();
    assert!(committed(&f.enter()));
    let entry = f.layer.revision().clone();
    let (there, back) = (resolved(&item(12)), inverse(&resolved(&item(12))));
    assert!(committed(&f.menu_twist(&item(12))));
    assert_ne!(f.layer.revision().digest, entry.digest);
    assert!(committed(&f.menu_twist(&item_of(&back))));
    assert_eq!(f.layer.revision().digest, entry.digest, "the exact inverse returns to the entry's poses");
    assert!(committed(&f.exit()));
    assert_eq!((f.layer.revision().head.as_str(), f.layer.snapshot().digest()), ("e5", labels.digest()));
    let mut moves = primitives(&[2]);
    moves.extend([there, back]);
    let expected = exit_record("e2", &moves, &entry.digest, labels.digest(), "supplied");
    assert_eq!(f.layer.store().log[4].crossing.as_ref(), Some(&expected));
}

/// CT-ONE-03 (part: an exit without a token, or with one bound to other arguments, is refused), CT-ONE-11 (part: an
/// exit refused at each step of ONE.80's order leaves every digest unchanged; not run: the head hash, which no fault
/// hook provokes with Jumble on), KN5.18, L15. The draft order, as for `jumble.on`: the pinned menu
/// (`KN5:input.row_not_built`) and the `revision` argument's form before the token; then the token, the commit lane and
/// the token's binding; then the mode (with Jumble off, `KN5:input.menu_word_needs_jumble_on`: there is no episode to
/// exit); then the named revision, which is current only if every field is equal (EV4.14); then A5's checkpoint. No
/// protection gate applies (H-08 gives the row `model_match` only): with orbit 0 protected under Strict, an exit to
/// labels that a pose move made before the protection moved it commits, and the protected set and the policy stay.
/// A pose move cannot move a protected orbit's pieces (the owner's reading of 18 November 2026 for finding B5; the
/// round trip is `b5_a_round_trip_through_jumble_does_not_move_a_protected_orbit` in `attack_protection.rs`), so the
/// exit lands a protected orbit where it was when it was protected.
#[test]
fn refusals_follow_the_exit_order_and_change_nothing() {
    let mut f = unpinned(&[]);
    assert_eq!(f.layer.availability(JUMBLE_OFF_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    let press = f.press();
    let refused = f.refused(press.clone(), code::ROW_NOT_BUILT, "no pinned menu");
    let mut forged = f.press();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::ROW_NOT_BUILT, "no pinned menu, with a forged token");
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert_eq!(f.layer.invoke(&Caller::Shell, press), refused, "a refused press keeps its answer");

    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();
    let now = f.layer.revision().to_json();
    let cases = [
        ("no revision", json::object([])),
        ("an argument the row does not take", json::object([("revision", now.clone()), ("x", Value::Null)])),
        ("a confirmation", json::object([("confirm", Value::Bool(true)), ("revision", now.clone())])),
        ("a revision that is text", json::object([("revision", text("now"))])),
        ("a revision without its digest", revision_with(&now, |r| drop(r.remove("digest")))),
        ("a revision with an extra field", revision_with(&now, |r| drop(r.insert("x".into(), Value::Null)))),
        ("a negative state revision", revision_with(&now, |r| drop(r.insert("state".into(), Value::Int(-1))))),
    ];
    for (what, args) in cases {
        let asked = f.shell.prepare(&f.layer, JUMBLE_OFF_ROW, args).unwrap();
        f.refused(asked, code::ARGUMENT_FORM, what);
    }
    let mut forged = f.shell.prepare(&f.layer, JUMBLE_OFF_ROW, json::object([("revision", text("now"))])).unwrap();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::ARGUMENT_FORM, "a malformed revision with a forged token: the form first");
    let mut forged = f.press();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::TOKEN_INVALID, "a forged token with Jumble off: the token first");
    f.layer.hold_commit_lane(true);
    let held = f.press();
    f.refused(held, code::COMMIT_IN_FLIGHT, "a held lane with Jumble off: the lane first");
    f.layer.hold_commit_lane(false);
    let press = f.press();
    f.refused(press, MENU_WORD_NEEDS_JUMBLE_ON, "Jumble off already");

    assert!(committed(&f.enter()));
    let press = f.press();
    f.refused(Invocation { token: None, ..press.clone() }, code::TOKEN_INVALID, "no token");
    let mut forged = press.clone();
    forged.token.as_mut().expect("a token").binding.nonce += 1_000;
    f.refused(forged, code::TOKEN_INVALID, "a forged token");
    // An authentic token's refusal attaches to its command ID, so this case takes a press of its own.
    let moved = revision_with(&f.layer.revision().to_json(), |r| drop(r.insert("workspace".into(), Value::Int(9))));
    let other = f.press();
    f.refused(Invocation { args: moved, ..other }, code::TOKEN_MISMATCH, "a token bound to other arguments");
    f.layer.hold_commit_lane(true);
    assert_eq!(f.layer.availability(JUMBLE_OFF_ROW).unwrap_err().code, code::COMMIT_IN_FLIGHT);
    let held = f.press();
    let busy = f.refused(held.clone(), code::COMMIT_IN_FLIGHT, "a held lane");
    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.invoke(&Caller::Shell, held), busy, "a refused press keeps its answer");

    assert!(committed(&f.menu_twist(&item(1))));
    f.refused(press.clone(), code::REVISION_CHANGED, "a press from before a pose move");
    let replay = Invocation { command: "replay/1".into(), ..press };
    f.refused(replay, code::TOKEN_USED, "a refusal after the token step spends the token");
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

    // Item 1's retained word moves orbit 0, which `twist.apply` refuses; the exit to its labels commits under Strict.
    let labels = derived(&kernel().solved(), &retained(&item(1)));
    assert!(committed(&f.exit()));
    assert_eq!(f.layer.snapshot().digest(), labels.digest());
    let exit = f.layer.store().log.last().unwrap().clone();
    assert_eq!((exit.kind, f.layer.policy()), (TransactionKind::Exit, ProtectionMode::Strict));
    assert_eq!(exit.orbits, [0]);
    assert_eq!(f.layer.protected().orbits().to_vec(), vec![0]);
    let press = f.shell.prepare(&f.layer, TWIST_ROW, twist_args(&item(1))).unwrap();
    f.refused(press, code::ORBIT_WOULD_MOVE, "after the exit the retained rows judge orbit 0 again");
}

/// A5, JI.27 (the label checkpoint's first fact): the first eight twists of the I_a fixture's journal move the poses
/// off the lattice, so the exit is `KN5:input.jumble_off_pieces_off_lattice`, names the count only and changes nothing,
/// although the row is available: its availability does not run the checkpoint. A refused press keeps its answer.
/// After the eight moves are undone the exit commits at the solved labels. Not run: a nonzero ν, a displaced centre
/// and two pieces in one chamber, which no short prefix of the I_a journal reaches on the lattice; the jumbling
/// engine's own tests cover each fact.
#[test]
fn off_the_lattice_the_exit_is_refused() {
    let mut f = fixture(&[]);
    assert!(committed(&f.enter()));
    let moves = journal(8);
    for (index, twist) in moves.iter().enumerate() {
        let receipt = f.menu_twist(&item_of(twist));
        assert!(committed(&receipt), "journal twist {index}: {:?}", receipt.status);
    }
    assert_eq!(f.layer.revision().digest, poses(&moves));
    assert_eq!(f.layer.availability(JUMBLE_OFF_ROW), Ok(()));
    let press = f.press();
    let receipt = f.refused(press.clone(), code::JUMBLE_OFF_PIECES_OFF_LATTICE, "pieces off the lattice");
    let Status::Refused(error) = &receipt.status else { panic!("{:?}", receipt.status) };
    assert!(error.detail.starts_with("Jumble off: pieces off the lattice: "), "{}", error.detail);
    assert_eq!(f.layer.invoke(&Caller::Shell, press), receipt, "a refused press keeps its answer");
    for _ in 0..moves.len() {
        assert!(committed(&f.step(Direction::Undo)));
    }
    assert!(committed(&f.exit()));
    assert_eq!(f.layer.snapshot().digest(), kernel().solved().digest());
}

/// CT-JI-09 (part: a retained commit, an entry, a pose move, an exit, a retained twist and a protection change form
/// one action chain whose undo and redo run through the exit both ways; not run: the void facts and the inverse
/// classes, which are not built), CT-JI-01 (part: an exit's undo restores the exact poses before it and the entry's
/// retained state, and its redo the same labels; the exit's outcome survives its undo and a reopen; not run: unequal
/// poses with equal labels, branch navigation, restore and import), ONE.81, JI.27 (draft). Every step has a new state
/// revision and a committed receipt. The undo of the protection change releases orbit 0, so it confirms (LI.26). A
/// reopen under the menu replays the whole chain.
#[test]
fn undo_and_redo_carry_the_exit_through_one_chain() {
    use TransactionKind::{Commit, Crossing, Exit, JumbleMove, Protection, Redo, Undo};
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[2]))));
    let off = f.layer.revision().clone();
    let start = f.layer.snapshot().clone();
    assert!(committed(&f.enter()));
    let entry = f.layer.revision().clone();
    assert!(committed(&f.menu_twist(&item(6))));
    let posed = f.layer.revision().clone();
    let exit = f.exit();
    assert!(committed(&exit), "{:?}", exit.status);
    let exited = f.layer.revision().clone();
    let labels = derived(&start, &word(&[1, -2, 1]));
    assert_eq!((exited.mode, exited.head.as_str(), exited.digest.as_str()), (MODE, "e4", labels.digest()));
    assert!(committed(&f.twist(&item(6))));
    let twisted = f.layer.revision().clone();
    assert_eq!(twisted.digest, derived(&labels, &word(&[1, -2, 1])).digest());
    assert!(completed(&f.protect(&[0])));

    let hint = f.layer.undo_hint(Direction::Undo).expect("the protection change's undo");
    let asked = f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    f.refused(asked, code::CONFIRMATION_REQUIRED, "a release without its confirmation");
    assert!(committed(&f.step_with(Direction::Undo, true)));
    assert!(f.layer.protected().orbits().is_empty());
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.revision().digest, exited.digest);

    let state = f.layer.revision().state;
    let undone = f.step(Direction::Undo);
    let Status::Committed(record) = &undone.status else { panic!("{:?}", undone.status) };
    assert_eq!((record.pre.as_str(), record.post.as_str()), (exited.digest.as_str(), posed.digest.as_str()));
    let r = f.layer.revision().clone();
    let at = (r.mode, r.head.as_str(), r.digest.as_str(), r.state);
    assert_eq!(at, (JUMBLE_ON_MODE, posed.head.as_str(), posed.digest.as_str(), state + 1));
    assert_eq!(f.layer.jumbled().expect("Jumble is on again").digest(&ctx()), posed.digest);
    assert_eq!(f.layer.snapshot().digest(), start.digest(), "the retained state is the entry's again");
    assert_eq!(f.layer.outcome(&exit.command, &exit.request), Some(exit.clone()), "the exit's outcome survives");
    assert_eq!(f.layer.availability(TWIST_ROW).unwrap_err().code, code::RETAINED_TOOL_NEEDS_JUMBLE_OFF);
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!((f.layer.revision().mode, &f.layer.revision().digest), (JUMBLE_ON_MODE, &entry.digest));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!((f.layer.revision().mode, &f.layer.revision().digest), (MODE, &off.digest));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.revision().digest, kernel().solved().digest());
    assert!(f.layer.undo_hint(Direction::Undo).is_none());

    for to in [&off, &entry, &posed, &exited, &twisted] {
        let redone = f.step(Direction::Redo);
        assert!(committed(&redone), "{}: {:?}", to.head, redone.status);
        let r = f.layer.revision();
        assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (to.mode, to.head.as_str(), to.digest.as_str()));
    }
    assert!(f.layer.jumbled().is_none());
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.layer.protected().orbits().to_vec(), vec![0]);
    assert!(f.layer.undo_hint(Direction::Redo).is_none());

    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    let mut chain = vec![Commit, Crossing, JumbleMove, Exit, Commit, Protection];
    chain.extend([Undo; 6]);
    chain.extend([Redo; 6]);
    assert_eq!(kinds, chain);

    let g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.store().log, log);
    assert_eq!(g.layer.protected().orbits().to_vec(), vec![0]);
    assert_eq!(g.layer.outcome(&exit.command, &exit.request), Some(exit));
}

/// KN5.23, ONE.58 and JI.27's recovery row (draft): a reopen with the session's twist menu pinned before the replay
/// (`CommandLayer::open_with_menu`) replays the exit at A5's checkpoint, makes the same exit record and restores the
/// mode, the head and the retained state; undo and redo through the exit work there. Without the menu, or under a
/// menu with another identity block, the entry does not replay: the layer is in read-only recovery at Jumble off
/// before it. A stored exit whose record or row differs from the replay's does not replay: the layer is in read-only
/// recovery after the last action before it, at Jumble on, never half-crossed.
#[test]
fn a_reopen_replays_the_exit_under_its_menu() {
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[2]))));
    let off = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    let entry = f.layer.revision().clone();
    assert!(committed(&f.menu_twist(&item(12))));
    assert!(committed(&f.menu_twist(&item_of(&inverse(&resolved(&item(12)))))));
    assert!(committed(&f.exit()));
    let exited = f.layer.revision().clone();
    assert!(committed(&f.twist(&item(6))));
    assert!(completed(&f.set_policy("strict")));
    let log = f.layer.store().log.clone();
    assert_eq!(log[4].kind, TransactionKind::Exit);

    let mut g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.store().log, log);
    assert!(g.layer.jumbled().is_none());
    for _ in 0..3 {
        assert!(committed(&g.step(Direction::Undo)));
    }
    let r = g.layer.revision().clone();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (JUMBLE_ON_MODE, "e4", entry.digest.as_str()));
    assert_eq!(g.layer.snapshot().digest(), off.digest);
    assert!(committed(&g.step(Direction::Redo)));
    assert_eq!((g.layer.revision().mode, &g.layer.revision().digest), (MODE, &exited.digest));
    let h = reopen_with(&g.layer, Some(pinned()));
    assert!(h.layer.recovery().is_none(), "{:?}", h.layer.recovery());
    assert_eq!(restored(&h.layer), restored(&g.layer));

    for (menu, what) in [(None, "no menu"), (Some(reversed()), "another identity block")] {
        let h = reopen_with(&f.layer, menu);
        assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "{what}");
        let r = h.layer.revision();
        let at = (r.mode, r.head.as_str(), r.digest.as_str());
        assert_eq!(at, (MODE, off.head.as_str(), off.digest.as_str()), "{what}");
        assert!(h.layer.jumbled().is_none(), "{what}");
        assert_eq!(h.layer.availability(JUMBLE_OFF_ROW).unwrap_err().code, code::READ_ONLY_RECOVERY, "{what}");
    }

    let before = log[3].revision.clone();
    let edits: [(&str, fn(&mut Transaction)); 3] = [
        ("an exit record with other labels", |t| {
            t.crossing = t.crossing.as_ref().map(|record| with(record, "labels", Some(text(&"0".repeat(64)))));
        }),
        ("an exit record with another ν source", |t| {
            let certificate = t.crossing.as_ref().and_then(|record| record.get("certificate")).cloned().unwrap();
            let certificate = with(&certificate, "nu_source", Some(text("identity_labels")));
            t.crossing = t.crossing.as_ref().map(|record| with(record, "certificate", Some(certificate)));
        }),
        ("an exit journaled under another row", |t| t.row = JUMBLE_ON_ROW.to_owned()),
    ];
    for (what, edit) in edits {
        let mut store = f.layer.store().clone();
        edit(&mut store.log[4]);
        let h = reopen_store(store, Some(pinned()));
        assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "{what}");
        let r = h.layer.revision();
        let at = (r.mode, r.head.as_str(), r.digest.as_str(), r.state);
        assert_eq!(at, (JUMBLE_ON_MODE, before.head.as_str(), before.digest.as_str(), before.state), "{what}");
        assert_eq!(h.layer.jumbled().expect("Jumble is on").digest(&ctx()), entry.digest, "{what}");
    }
}

/// CT-EV4-04 (part: an exit is admitted once: a retried delivery answers the stored receipt and exits nothing more,
/// also after a restart, after the exit's undo and after a reopen; the same command ID with other content is
/// `KN5:input.command_id_reused`, and the spent token under another command ID is `KN5:auth.token_used`), CT-KN5-04
/// (part: the same for an exit), CT-JI-09 (part: the original outcome lookup survives undo and restart), KN5.20,
/// KN5.21, LI.30.
#[test]
fn a_retried_exit_answers_its_receipt_and_exits_once() {
    let mut f = fixture(&[]);
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    let press = f.press();
    let done = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&done), "{:?}", done.status);
    let kept = f.fixed();
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done);
    assert_eq!(f.fixed(), kept);
    let revision = press.args.get("revision").expect("a revision").clone();
    let moved = revision_with(&revision, |r| drop(r.insert("workspace".into(), Value::Int(9))));
    let other = Invocation { args: moved, ..press.clone() };
    f.refused(other, code::COMMAND_ID_REUSED, "other content under the command ID");
    let replay = Invocation { command: "replay/1".into(), ..press.clone() };
    f.refused(replay, code::TOKEN_USED, "the spent token under another command ID");

    f.layer.restart(Secret::fixed(SECRET));
    f.shell.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome(&press.command, &done.request), Some(done.clone()));
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done);
    assert!(f.layer.jumbled().is_none(), "a restart keeps Jumble off");
    assert!(committed(&f.step(Direction::Undo)));
    assert!(f.layer.jumbled().is_some());
    assert_eq!(f.layer.outcome(&press.command, &done.request), Some(done.clone()), "after the exit's undo");
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done, "after the exit's undo");
    assert_eq!(f.layer.store().log.len(), 4);

    let mut g = reopen_with(&f.layer, Some(pinned()));
    assert_eq!(g.layer.outcome(&press.command, &done.request), Some(done.clone()));
    assert_eq!(g.layer.invoke(&Caller::Shell, press), done);
    assert_eq!(g.layer.store().log.len(), 4);
    assert!(g.layer.jumbled().is_some());
}

/// The journal form (draft: FP.31's `exit`): an exit is the action kind `exit` with its record under the key
/// `crossing`, an 18th key, and reads back to itself; an entry's record and an exit's are not interchangeable. An exit
/// without its record, an exit's record on a twist, an entry or an undo, an exit with a recipe, a confirmation or a
/// rotation, and a record that is not an exit's do not read: ν not zero or not 45 values, an unknown ν source, a frame
/// that does not agree, an unnamed program, convention or contract revision, an episode that is not an event, a
/// 600-cell-Full identity block, a negative letter count, a digest that is not a lower-case SHA-256, a missing or an
/// extra field. The form does not check the labels against the poses, which a reopen's replay does.
#[test]
fn an_exit_journals_its_record() {
    let mut f = fixture(&[]);
    assert!(committed(&f.twist(&item(1))));
    assert!(committed(&f.enter()));
    assert!(committed(&f.exit()));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    let log = f.layer.store().log.clone();
    let keys: Vec<usize> = log.iter().map(|t| t.to_json().as_object().unwrap().len()).collect();
    assert_eq!(keys, [18, 18, 18, 17, 17]);
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }
    assert_eq!(log[2].to_json().get("kind"), Some(&text("exit")));
    let record = log[2].crossing.clone().expect("the exit's record");
    assert_eq!(log[2].to_json().get("crossing"), Some(&record));
    let entry = log[1].crossing.clone().expect("the entry's record");

    let exit = |record: Value| with(&log[2].to_json(), "crossing", Some(record));
    let certificate = record.get("certificate").expect("a certificate").clone();
    let journal = record.get("journal").expect("a journal").clone();
    let certified =
        |name: &str, value: Option<Value>| exit(with(&record, "certificate", Some(with(&certificate, name, value))));
    let journaled =
        |name: &str, value: Option<Value>| exit(with(&record, "journal", Some(with(&journal, name, value))));
    let edited = |name: &str, value: Option<Value>| exit(with(&record, name, value));
    let mut nonzero = vec![Value::Int(0); levels::NU_LEN];
    nonzero[7] = Value::Int(1);
    let short = vec![Value::Int(0); levels::NU_LEN - 1];
    let full = identity_json(&kernel().identity());
    let malformed = [
        (with(&log[2].to_json(), "crossing", None), "an exit without its record"),
        (exit(Value::Null), "a null record"),
        (exit(entry.clone()), "an exit with an entry's record"),
        (with(&log[1].to_json(), "crossing", Some(record.clone())), "an entry with an exit's record"),
        (with(&log[0].to_json(), "crossing", Some(record.clone())), "a twist with an exit's record"),
        (with(&log[3].to_json(), "crossing", Some(record.clone())), "an undo with an exit's record"),
        (with(&log[2].to_json(), "recipe", Some(word(&[1]))), "an exit with a recipe"),
        (with(&log[2].to_json(), "confirm", Some(Value::Bool(true))), "an exit with a confirmation"),
        (with(&log[2].to_json(), "rotation", log[0].rotation.clone()), "an exit with a rotation"),
        (certified("nu", Some(Value::Array(nonzero))), "ν not zero"),
        (certified("nu", Some(Value::Array(short))), "44 values of ν"),
        (certified("nu_source", Some(text("guess"))), "an unknown ν source"),
        (certified("frame_agrees", Some(Value::Bool(false))), "a frame that does not agree"),
        (certified("program", Some(text("00"))), "a short program digest"),
        (certified("c5_convention", Some(text(""))), "an unnamed convention"),
        (certified("contract_revision", None), "no contract revision"),
        (certified("extra", Some(Value::Null)), "a certificate with an extra field"),
        (journaled("letters", Some(Value::Int(-1))), "a negative letter count"),
        (journaled("letters", Some(text("1"))), "a letter count that is text"),
        (journaled("word", Some(text(&"A".repeat(64)))), "a word digest in upper case"),
        (edited("episode", Some(text("x1"))), "an episode that is not an event"),
        (edited("episode", Some(text("e"))), "an event without its order"),
        (edited("identity", Some(full)), "a 600-cell-Full identity block"),
        (edited("identity", None), "no identity block"),
        (edited("labels", Some(text("00"))), "a short label digest"),
        (edited("poses", None), "no pose digest"),
        (edited("extra", Some(Value::Null)), "an extra field"),
    ];
    for (value, what) in malformed {
        assert_eq!(Transaction::from_json(&value), None, "{what}");
    }
    let other = edited("labels", Some(text(&"0".repeat(64))));
    assert!(Transaction::from_json(&other).is_some(), "the journal form does not check the labels against the poses");
}

/// CT-ONE-17 (part: an exit cancels the jobs on the replaced revision; not run: a snapshot opened before it keeps its
/// level, which is the runtime's), ONE.55, KN5.18, LI.32 (draft: an exit ends what read the state it replaces, as a
/// commit does): it cancels a running and a queued query job with `replaced_revision` and drops a late result, while a
/// file job keeps running on the revision it read, whose mode is `jumble_on`. With Jumble on there is no preview to
/// end.
#[test]
fn an_exit_ends_the_jobs_that_read_the_state() {
    let mut f = fixture(&[]);
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

    assert!(committed(&f.exit()));
    for receipt in [&inspect, &scan] {
        let ended = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert_eq!(ended.status, Status::Cancelled(CancelReason::ReplacedRevision));
    }
    assert_eq!(f.layer.outcome(&export.command, &export.request).as_ref(), Some(&export));
    assert!(f.layer.finish_program_job(taken.job, Ok(Value::Null)).is_none());
    let export = f.layer.take_program_job().unwrap();
    assert_eq!((export.row.as_str(), export.revision.mode), ("tool.full.export", JUMBLE_ON_MODE));
    assert_eq!(f.layer.store().log.len(), 2);
}

/// KN5.15: the runtime may not invoke a commit row, so its `jumble.off` is `KN5:input.commit_row_refused` and changes
/// nothing, with Jumble on or off and with or without a pinned menu.
#[test]
fn the_runtime_may_not_exit() {
    let mut on = fixture(&[]);
    assert!(committed(&on.enter()));
    for mut f in [on, fixture(&[]), unpinned(&[])] {
        let runtime = Caller::Runtime { run: "run-1".into() };
        let epoch = f.layer.revision().epoch;
        let args = json::object([("revision", f.layer.revision().to_json())]);
        let theirs = Invocation {
            command: "run-1/1".into(),
            row: JUMBLE_OFF_ROW.into(),
            args,
            token: None,
            epoch,
            budget: None,
        };
        f.refused_on(&runtime, theirs, code::COMMIT_ROW_REFUSED, "the runtime");
    }
}

/// CT-JI-08 (part: the exit-root entry with an empty suffix; not run: branch navigation, restore, import and the void
/// facts, which are not built), JI.25, JI.26, JI.27 ("Enter again"), FP.31 (draft: the record; the empty suffix),
/// FP.32. The exit stays active across a policy change, which is no node of the move tree, and `jumble.on` enters
/// from its root: at the exit's own poses, with the retained state at the exit's labels, and the journal keeps the
/// record `exit_root` with the exit's event, the digest of the exit's record and the empty suffix. A pose move and a
/// second exit end that episode, and the second exit is the active one: the next entry roots at it.
#[test]
fn an_entry_at_an_exit_enters_at_the_exit_s_poses() {
    let solved = kernel().solved();
    let t6 = resolved(&item(6));
    let mut f = fixture(&[]);
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    let posed = f.layer.revision().clone();
    assert!(committed(&f.exit()));
    let labels = derived(&solved, &word(&[1, -2, 1]));
    assert!(completed(&f.set_policy("strict")));
    let off = f.layer.revision().clone();
    let receipt = f.enter();
    let Status::Committed(record) = &receipt.status else { panic!("{:?}", receipt.status) };
    assert_eq!((record.pre.as_str(), record.post.as_str()), (labels.digest(), posed.digest.as_str()));
    let on = f.layer.revision().clone();
    let at = (on.mode, on.head.as_str(), on.digest.as_str(), on.state);
    assert_eq!(at, (JUMBLE_ON_MODE, "e5", posed.digest.as_str(), off.state + 1));
    assert_eq!((on.workspace, on.protection), (off.workspace, off.protection));
    assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&ctx()), posed.digest);
    assert_eq!(f.layer.snapshot().digest(), labels.digest(), "the retained state stays at the exit's labels");
    let log = f.layer.store().log.clone();
    let exit = log[2].crossing.clone().expect("the exit's record");
    let entry = &log[4];
    assert_eq!((entry.kind, entry.row.as_str()), (TransactionKind::Crossing, JUMBLE_ON_ROW));
    assert_eq!(entry.crossing.as_ref(), Some(&exit_root("e3", &exit, &[])));
    assert_eq!(Transaction::from_json(&entry.to_json()).as_ref(), Some(entry));

    assert!(committed(&f.menu_twist(&item(6))));
    let twice = f.layer.revision().clone();
    assert_eq!(twice.digest, poses(&[t6.clone(), t6.clone()]));
    assert!(committed(&f.exit()));
    let again = derived(&labels, &word(&[1, -2, 1]));
    assert_eq!(f.layer.snapshot().digest(), again.digest());
    let second = exit_record("e5", &[t6.clone(), t6], &twice.digest, again.digest(), "retained_witness");
    assert_eq!(f.layer.store().log[6].crossing.as_ref(), Some(&second));
    assert!(committed(&f.enter()));
    assert_eq!(f.layer.revision().digest, twice.digest);
    assert_eq!(f.layer.store().log[7].crossing.as_ref(), Some(&exit_root("e7", &second, &[])));
}

/// CT-JI-08 (part: the exit-root entry after retained moves, with its undo and redo on its own sides; not run: branch
/// navigation and the void facts), JI.25 to JI.27, JI.29, FP.31, FP.32. After an exit, a retained twist and a word
/// commit, `jumble.on` replays their literal letters, the retained suffix, from the exit's own poses: the poses are
/// the jumbling journal's from solved, and the record names the exit and the suffix's letter count and
/// `Digest<literal word>`. An undo of the entry restores the Jumble off state after the suffix, not the exit's, and a
/// redo makes the same entry again. With the word commit undone too, the entry replays the shorter suffix and clears
/// the redo stack.
#[test]
fn an_entry_after_retained_moves_replays_the_suffix_from_the_exit() {
    let mut f = fixture(&[]);
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    assert!(committed(&f.exit()));
    assert!(committed(&f.twist(&item(6))));
    let twisted = f.layer.revision().clone();
    assert!(committed(&f.commit(&word(&[2]))));
    let off = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    let on = f.layer.revision().clone();
    let mut moves = vec![resolved(&item(6))];
    moves.extend(primitives(&[1, -2, 1, 2]));
    assert_eq!((on.mode, on.head.as_str(), on.digest.clone()), (JUMBLE_ON_MODE, "e6", poses(&moves)));
    assert_eq!(f.layer.snapshot().digest(), off.digest);
    let log = f.layer.store().log.clone();
    let exit = log[2].crossing.clone().expect("the exit's record");
    assert_eq!(log[5].crossing.as_ref(), Some(&exit_root("e3", &exit, &[1, -2, 1, 2])));

    assert!(committed(&f.step(Direction::Undo)));
    let r = f.layer.revision().clone();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (MODE, off.head.as_str(), off.digest.as_str()));
    assert!(f.layer.jumbled().is_none());
    assert_eq!(f.layer.snapshot().digest(), off.digest, "the state after the suffix, not the exit's");
    assert!(committed(&f.step(Direction::Redo)));
    let r = f.layer.revision().clone();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (JUMBLE_ON_MODE, "e6", on.digest.as_str()));
    assert_eq!(f.layer.jumbled().expect("Jumble is on").digest(&ctx()), on.digest);

    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.revision().digest, twisted.digest);
    assert!(committed(&f.enter()));
    moves.truncate(4);
    assert_eq!(f.layer.revision().digest, poses(&moves));
    let last = f.layer.store().log.last().cloned().expect("the entry");
    assert_eq!(last.crossing.as_ref(), Some(&exit_root("e3", &exit, &[1, -2, 1])));
    assert!(f.layer.undo_hint(Direction::Redo).is_none());
}

/// CT-JI-09 (part: an entry from an exit root joins one action chain with the solved-witness entry, pose moves, the
/// exit and a retained twist, and undo and redo run through both entries both ways; not run: the void facts and the
/// inverse classes, which are not built), CT-JI-08 (part: each entry's undo and redo use its own sides), ONE.81,
/// JI.27, JI.29 (draft), KN5.23: every step lands on its recorded mode, head, digest and retained state, and a reopen
/// under the menu replays the whole chain with the same records.
#[test]
fn undo_and_redo_carry_the_exit_root_entry_through_one_chain() {
    use TransactionKind::{Commit, Crossing, Exit, JumbleMove, Redo, Undo};
    type Step = fn(&mut Fixture) -> Receipt;
    let steps: [Step; 6] = [
        |f| f.enter(),
        |f| f.menu_twist(&item(6)),
        |f| f.exit(),
        |f| f.twist(&item(6)),
        |f| f.enter(),
        |f| f.menu_twist(&item(12)),
    ];
    let mut f = fixture(&[]);
    let at = |f: &Fixture| {
        let r = f.layer.revision();
        (r.mode, r.head.clone(), r.digest.clone(), f.layer.snapshot().digest().to_owned())
    };
    let mut points = vec![at(&f)];
    for (index, step) in steps.iter().enumerate() {
        let receipt = step(&mut f);
        assert!(committed(&receipt), "step {index}: {:?}", receipt.status);
        points.push(at(&f));
    }
    for k in (0..steps.len()).rev() {
        assert!(committed(&f.step(Direction::Undo)), "the undo to point {k}");
        assert_eq!(at(&f), points[k], "the undo to point {k}");
    }
    assert!(f.layer.undo_hint(Direction::Undo).is_none());
    for (k, point) in points.iter().enumerate().skip(1) {
        assert!(committed(&f.step(Direction::Redo)), "the redo to point {k}");
        assert_eq!(&at(&f), point, "the redo to point {k}");
    }
    assert!(f.layer.undo_hint(Direction::Redo).is_none());

    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    let mut chain = vec![Crossing, JumbleMove, Exit, Commit, Crossing, JumbleMove];
    chain.extend([Undo; 6]);
    chain.extend([Redo; 6]);
    assert_eq!(kinds, chain);
    let g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.store().log, log);
    assert_eq!(at(&g), at(&f));
}

/// KN5.23, ONE.58 and JI.27's recovery row (draft), FP.31: a reopen under the menu replays an entry from an exit root
/// from the replayed exit's poses and makes the same record. A stored entry whose record names another letter count,
/// another suffix, another exit or another crossing event, or the solved witness's form, does not replay: the layer is
/// in read-only recovery at Jumble off after the retained twist before it, never half-crossed. Without the menu the
/// first entry does not replay.
#[test]
fn a_reopen_replays_the_exit_root_entry_under_its_menu() {
    let mut f = fixture(&[]);
    let start = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    assert!(committed(&f.exit()));
    assert!(committed(&f.twist(&item(6))));
    let off = f.layer.revision().clone();
    assert!(committed(&f.enter()));
    let log = f.layer.store().log.clone();
    assert_eq!(log[4].kind, TransactionKind::Crossing);

    let g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.store().log, log);
    assert_eq!(g.layer.jumbled().map(|jumbled| jumbled.digest(&ctx())), Some(f.layer.revision().digest.clone()));
    let h = reopen_with(&f.layer, None);
    assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "no menu");
    let r = h.layer.revision();
    assert_eq!((r.mode, r.head.as_str(), r.digest.as_str()), (MODE, start.head.as_str(), start.digest.as_str()));

    let record = log[4].crossing.clone().expect("the entry's record");
    let solved_form = json::object([
        ("entrance", text("solved_witness")),
        ("identity", identity_json(&ctx().identity(Some(&pinned())))),
        ("letters", Value::Int(3)),
        ("word", text(&literal_digest(&[1, -2, 1]))),
    ]);
    let edits = [
        (with(&record, "letters", Some(Value::Int(2))), "another letter count"),
        (with(&record, "word", Some(text(&literal_digest(&[1, -2])))), "another suffix"),
        (with(&record, "exit", Some(text(&"0".repeat(64)))), "another exit"),
        (with(&record, "crossing", Some(text("e2"))), "another crossing event"),
        (solved_form, "the solved witness's form"),
    ];
    for (edited, what) in edits {
        let mut store = f.layer.store().clone();
        store.log[4].crossing = Some(edited);
        let h = reopen_store(store, Some(pinned()));
        assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "{what}");
        let r = h.layer.revision();
        let at = (r.mode, r.head.as_str(), r.digest.as_str(), r.state);
        assert_eq!(at, (MODE, off.head.as_str(), off.digest.as_str(), off.state), "{what}");
        assert!(h.layer.jumbled().is_none(), "{what}");
    }
}

/// JI.27 (draft ruling: an exit's poses keep the jumbling identity block they were made under). With Jumble off a
/// menu with another identity block may be pinned, but an entry from the exit's root under it is
/// `KN1:input.identity_mismatch` and changes nothing, at the exit and after a retained commit. With the exit's menu
/// pinned again, the entry commits.
#[test]
fn an_entry_from_an_exit_root_needs_the_exit_s_menu() {
    let mut f = fixture(&[]);
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    assert!(committed(&f.exit()));
    f.layer.pin_menu(ctx(), reversed()).unwrap();
    let press = f.enter_press();
    f.refused(press, IDENTITY_MISMATCH, "an entry at the exit under another menu");
    assert!(committed(&f.commit(&word(&[2]))));
    let press = f.enter_press();
    f.refused(press, IDENTITY_MISMATCH, "an entry after a retained commit under another menu");
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert!(committed(&f.enter()));
    let log = f.layer.store().log.clone();
    let exit = log[2].crossing.clone().expect("the exit's record");
    assert_eq!(log.last().and_then(|t| t.crossing.as_ref()), Some(&exit_root("e3", &exit, &[2])));
}

/// A long chain through an exit (draft): an entry from an exit root derives the suffix on the exit's poses with their
/// flat claim kept flat, where the layer once replayed the jumbling journal from solved at the 64th chained derivation.
/// After 63 pose moves, an exit and a retained twist, the entry's poses are the journal's from solved, its claim is
/// the flat claim of one replay of the whole journal from solved, its undo and redo reach them again, a pose move and
/// an exit work from them, and a reopen under the menu replays the whole chain.
#[test]
fn an_entry_after_a_long_chain_keeps_the_journal_s_flat_claim() {
    let t6 = resolved(&item(6));
    let mut f = fixture(&[]);
    assert!(committed(&f.enter()));
    for index in 0..63 {
        assert!(committed(&f.menu_twist(&item(6))), "pose move {index}");
    }
    assert!(committed(&f.exit()));
    assert!(committed(&f.twist(&item(6))));
    assert!(committed(&f.enter()));
    let mut moves = vec![t6.clone(); 63];
    moves.extend(primitives(&[1, -2, 1]));
    let on = f.layer.revision().clone();
    assert_eq!(on.digest, poses(&moves));
    let claim = f.layer.jumbled().expect("Jumble is on").reachability().clone();
    assert_eq!(claim, flat_claim(&moves), "the entry's claim is the journal's flat claim from solved");
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.layer.revision().digest, on.digest);
    assert!(committed(&f.menu_twist(&item(6))));
    moves.push(t6);
    assert_eq!(f.layer.revision().digest, poses(&moves));
    assert!(committed(&f.exit()));
    let g = reopen_with(&f.layer, Some(pinned()));
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.store().log, f.layer.store().log);
}
