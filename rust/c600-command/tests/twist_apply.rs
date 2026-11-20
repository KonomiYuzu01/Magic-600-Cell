//! `twist.apply` in the command layer (draft; amendment A6, KN3.7 to KN3.9, ONE.46, ONE.78): a live twist whose `cap`
//! and `rotation` carry KN3.7's exact request binding, resolved against the twist menu the host pinned and committed on
//! 600-cell-Full as the item's retained word, with no preview. Each test names the conformance rows it covers headless
//! with PHS.62 markers. Where the frozen text is silent, the test follows the draft rulings recorded in `src/lib.rs`
//! and says so.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, policy_from_name, Caller, CancelReason, CommandLayer, Completion, CompletionKind, Direction,
    HeadlessFrames, Invocation, Kind, MemoryStore, Overlay, PreviewState, Receipt, Secret, Shell, Status, Table,
    Transaction, TransactionKind, POLICY_ROW, TWIST_ROW,
};
use c600_jumble::codes::MENU_WORD_NEEDS_JUMBLE_ON;
use c600_jumble::rotation;
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::codes::RECIPE_SHAPE;
use c600_kernel::json::{self, Value};
use c600_kernel::request::codes::{MENU_BINDING_MISMATCH, ROTATION_REQUEST_MALFORMED, TWIST_OFF_MENU};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Derivation, Kernel, ProtectedSet, ProtectionMode, Snapshot, ORBITS};

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

/// The I_a menu of `research/jumbling/fixtures/wj-I_a.json`, the menu KN3's tests pin.
fn pinned() -> Menu {
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let record = json::parse(&fixture).expect("the I_a fixture is JSON").get("menu").expect("a menu").clone();
    Menu::from_record(&ctx(), &record).expect("the pinned I_a menu")
}

fn pin(order: &str) -> MenuPin {
    MenuPin { set: SET.to_owned(), order: order.to_owned() }
}

/// Menu item `index` at cap 0, pressed directly. At cap 0 items 1 to 11 are retained, 0 is the identity and 12 to 59
/// are jumble items.
fn item(index: u64) -> Binding {
    Binding {
        request: wire::menu_request(SET, ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(pin(ORDER)),
    }
}

/// The quarter turn about the cap's first axis, which is not on the menu.
fn quarter(operation: Operation) -> Binding {
    Binding { request: wire::cayley_request(["1:0", "0:0", "0:0"]), cap: 0, operation, menu: Some(pin(ORDER)) }
}

/// `twist.apply`'s arguments for `binding`: its cap, and the rest of the binding as `rotation`.
fn args(binding: &Binding) -> Value {
    let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
    let cap = fields.remove("cap").expect("a binding has a cap");
    json::object([("cap", cap), ("rotation", Value::Object(fields))])
}

/// `args` with one argument replaced or added.
fn with(binding: &Binding, name: &str, value: Value) -> Value {
    let Value::Object(mut fields) = args(binding) else { panic!("the arguments are an object") };
    fields.insert(name.to_owned(), value);
    Value::Object(fields)
}

/// `args` with one field of `rotation` replaced, added or, for `None`, removed.
fn rotation_with(binding: &Binding, name: &str, value: Option<Value>) -> Value {
    let Some(Value::Object(mut fields)) = args(binding).get("rotation").cloned() else { panic!("a rotation object") };
    match value {
        Some(value) => fields.insert(name.to_owned(), value),
        None => fields.remove(name),
    };
    with(binding, "rotation", Value::Object(fields))
}

/// The retained word KN3's resolution gives `binding` against the pinned menu.
fn retained(binding: &Binding) -> Vec<i64> {
    let resolution = rotation::resolve(&ctx(), Some(&pinned()), binding).expect("the binding resolves");
    rotation::retained_word(&ctx(), &resolution).expect("a retained item")
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

/// The orbits `letters` moves.
fn moved_orbits(letters: &[i64]) -> BTreeSet<i64> {
    let k = kernel();
    let effect = k.effect(&k.word(letters).unwrap()).unwrap();
    effect.support.iter().map(|row| i64::from(row.orbit)).collect()
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

/// What a reopen must restore: the policy, the head, the state, the digest and the workspace and protection revisions.
fn restored(layer: &CommandLayer) -> (ProtectionMode, String, u64, String, u64, u64) {
    let r = layer.revision();
    (layer.policy(), r.head.clone(), r.state, r.digest.clone(), r.workspace, r.protection)
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

/// A reopen of `layer`'s store, which replays its journal; no menu is pinned.
fn reopen(layer: &CommandLayer) -> Fixture {
    let store = layer.store().clone();
    let layer = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    /// The twist press for `binding`, prepared with its token and not sent.
    fn press(&mut self, binding: &Binding) -> Invocation {
        self.shell.prepare(&self.layer, TWIST_ROW, args(binding)).unwrap()
    }

    fn twist(&mut self, binding: &Binding) -> Receipt {
        let press = self.press(binding);
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

    /// `protection.orbit.set` with `orbits`; `confirm` confirms a release (LI.26).
    fn protect(&mut self, orbits: &[i64], confirm: bool) -> Receipt {
        let orbits = Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect());
        let mut args = vec![("orbits".to_owned(), orbits)];
        if confirm {
            args.push(("confirm".to_owned(), Value::Bool(true)));
        }
        self.shell.invoke(&mut self.layer, "protection.orbit.set", Value::Object(args.into_iter().collect())).unwrap()
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

    /// What no refusal may change: the state digest, the revision, the durable log and the policy.
    fn fixed(&self) -> (String, String, usize, ProtectionMode) {
        let r = self.layer.revision();
        (r.digest.clone(), format!("{r:?}"), self.layer.store().log.len(), self.layer.policy())
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

/// CT-EV4-04 (part: the twist leg in the command layer, a twist admitted and committed once with no preview and no
/// job; not run: held-key repeats, which are the shell's and LP3's `c600-input`), CT-BND-19 (part: the layer holds
/// nothing for a twist before its press commits, neither a preview nor a job nor a revision change; not run: the view
/// model's fields, which are LP3's), KN3.8, JI.35. A menu item resolves to its retained word at the binding's cap, and
/// the commit journals the resolution's record as `rotation` (draft).
#[test]
fn a_twist_commits_the_retained_word_of_its_menu_item() {
    let k = kernel();
    let mut f = fixture(&[]);
    assert_eq!(f.layer.availability(TWIST_ROW), Ok(()));
    let start = f.layer.revision().clone();
    let press = f.press(&item(1));
    assert!(f.layer.preview().is_none() && f.layer.frame().is_none());
    assert_eq!(f.layer.run_jobs(), 0);
    assert_eq!(f.layer.revision(), &start);

    let receipt = f.shell.send(&mut f.layer, press);
    let Status::Committed(record) = &receipt.status else { panic!("{:?}", receipt.status) };
    let moved = derived(&k.solved(), &word(&[1]));
    assert_eq!((&record.pre, record.post.as_str()), (&start.digest, moved.digest()));
    let after = f.layer.revision().clone();
    assert_eq!((after.state, after.workspace, after.protection), (start.state + 1, start.workspace, start.protection));
    assert_eq!((after.head.as_str(), after.digest.as_str()), ("e1", moved.digest()));
    assert_eq!(receipt.job, None);
    assert!(f.layer.preview().is_none());
    assert_eq!(f.layer.run_jobs(), 0);

    let resolution = rotation::resolve(&ctx(), Some(&pinned()), &item(1)).unwrap();
    let log = f.layer.store().log.clone();
    assert_eq!(log.len(), 1);
    assert_eq!((log[0].kind, log[0].row.as_str(), &log[0].recipe), (TransactionKind::Commit, TWIST_ROW, &word(&[1])));
    assert_eq!(log[0].rotation.as_ref(), Some(&resolution.to_json()));
    assert_eq!(log[0].receipt(), receipt);

    // A longer retained word, the same item at another cap, and a drag that lands on a retained item.
    let drag = Binding {
        request: wire::half_turn_request(["0:0", "1:0", "0:0"]),
        cap: 0,
        operation: Operation::MenuDrag,
        menu: Some(pin(ORDER)),
    };
    let elsewhere = Binding { cap: 7, ..item(1) };
    assert_eq!((retained(&item(6)), retained(&elsewhere)), (vec![1, -2, 1], vec![15]));
    for binding in [item(6), elsewhere, drag] {
        let before = f.layer.snapshot().clone();
        let receipt = f.twist(&binding);
        assert!(committed(&receipt), "{:?}", receipt.status);
        let last = f.layer.store().log.last().unwrap().clone();
        let letters = retained(&binding);
        assert_eq!(last.recipe, word(&letters));
        assert_eq!(f.layer.revision().digest, derived(&before, &word(&letters)).digest());
        let resolution = rotation::resolve(&ctx(), Some(&pinned()), &binding).unwrap();
        assert_eq!(last.rotation, Some(resolution.to_json()));
    }
    let log = f.layer.store().log.clone();
    let kinds: Vec<Option<Value>> =
        log.iter().map(|t| t.rotation.as_ref()?.get("choice")?.get("kind").cloned()).collect();
    let direct = Some(text("menu_direct"));
    assert_eq!(kinds, [direct.clone(), direct.clone(), direct, Some(text("menu_drag"))]);
}

/// KN5.18 and the draft ruling recorded in `src/lib.rs`: without a pinned twist menu, `twist.apply` is refused with
/// `KN5:input.row_not_built` before its arguments are read, so LP3's placeholder arguments answer it too. A pin makes
/// the row available, a refused press keeps its answer, and a later pin replaces the earlier one.
#[test]
fn the_row_needs_a_pinned_menu() {
    let mut f = unpinned(&[]);
    assert_eq!(f.layer.availability(TWIST_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    let press = f.press(&item(1));
    let refused = f.refused(press.clone(), code::ROW_NOT_BUILT, "no pinned menu");
    let placeholder = json::object([
        ("cap", text("c1")),
        ("rotation", json::object([("axis", Value::Int(3)), ("inverse", Value::Bool(false))])),
    ]);
    let asked = f.shell.prepare(&f.layer, TWIST_ROW, placeholder).unwrap();
    f.refused(asked, code::ROW_NOT_BUILT, "LP3's placeholder arguments without a pin");

    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert_eq!(f.layer.availability(TWIST_ROW), Ok(()));
    assert_eq!(f.layer.invoke(&Caller::Shell, press), refused);
    assert!(committed(&f.twist(&item(1))));
    f.layer.pin_menu(ctx(), pinned()).unwrap();
    assert!(committed(&f.twist(&item(2))));
    assert_eq!(f.layer.store().log.len(), 2);
}

/// KN3.7, KN3.8, JI.35 and H-08's argument form (draft: `rotation` is KN3.7's binding without the cap the row names;
/// every input refusal comes before the token, so it changes nothing). The table's argument names answer
/// `KN5:input.argument_form`; a binding that does not read answers `KN2:input.rotation_request_malformed`; the
/// resolution's refusals keep their codes; a jumble item answers `KN5:input.menu_word_needs_jumble_on`, and the menu's
/// identity item `KN2:input.recipe_shape`. After them all, a twist still commits.
#[test]
fn input_refusals_change_nothing() {
    let mut f = fixture(&[]);
    let free = Binding {
        request: wire::cayley_request(["0:0", "0:0", "0:0"]),
        cap: 0,
        operation: Operation::Free { n: 1, d: 1 },
        menu: None,
    };
    let placeholder = json::object([("axis", Value::Int(3)), ("inverse", Value::Bool(false))]);
    let cases = [
        ("no rotation", json::object([("cap", Value::Int(0))]), code::ARGUMENT_FORM),
        ("an argument the row does not take", with(&item(1), "grip", Value::Int(0)), code::ARGUMENT_FORM),
        ("a confirmation", with(&item(1), "confirm", Value::Bool(true)), code::ARGUMENT_FORM),
        ("a rotation that is text", with(&item(1), "rotation", text("M|1")), ROTATION_REQUEST_MALFORMED),
        ("LP3's placeholder rotation", with(&item(1), "rotation", placeholder), ROTATION_REQUEST_MALFORMED),
        (
            "a rotation with its own cap",
            rotation_with(&item(1), "cap", Some(Value::Int(0))),
            ROTATION_REQUEST_MALFORMED,
        ),
        ("a rotation without its menu", rotation_with(&item(1), "menu", None), ROTATION_REQUEST_MALFORMED),
        ("a cap that is text", with(&item(1), "cap", text("c1")), ROTATION_REQUEST_MALFORMED),
        ("a negative cap", with(&item(1), "cap", Value::Int(-1)), ROTATION_REQUEST_MALFORMED),
        ("a cap past the last pole", with(&item(1), "cap", Value::Int(600)), ROTATION_REQUEST_MALFORMED),
        (
            "a menu operation with a null menu",
            rotation_with(&item(1), "menu", Some(Value::Null)),
            ROTATION_REQUEST_MALFORMED,
        ),
        ("an index outside the menu", args(&item(60)), ROTATION_REQUEST_MALFORMED),
        (
            "a menu index dragged",
            args(&Binding { operation: Operation::MenuDrag, ..item(1) }),
            ROTATION_REQUEST_MALFORMED,
        ),
        ("a free binding", args(&free), MENU_BINDING_MISMATCH),
        ("another menu's order", args(&Binding { menu: Some(pin(REVERSED_ORDER)), ..item(1) }), MENU_BINDING_MISMATCH),
        ("a direct rotation off the menu", args(&quarter(Operation::MenuDirect)), TWIST_OFF_MENU),
        ("a jumble item", args(&item(12)), MENU_WORD_NEEDS_JUMBLE_ON),
        ("a drag to a jumble item", args(&quarter(Operation::MenuDrag)), MENU_WORD_NEEDS_JUMBLE_ON),
        ("the identity item", args(&item(0)), RECIPE_SHAPE),
    ];
    for (what, arguments, expected) in cases {
        let asked = f.shell.prepare(&f.layer, TWIST_ROW, arguments).unwrap();
        f.refused(asked, expected, what);
    }

    // The input refusals come before the token's step 3: a forged token with a jumble item answers the input, and with
    // a retained item the token.
    for (binding, expected) in [(item(12), MENU_WORD_NEEDS_JUMBLE_ON), (item(1), code::TOKEN_INVALID)] {
        let mut forged = f.press(&binding);
        forged.token.as_mut().expect("a token").binding.nonce += 1_000;
        f.refused(forged, expected, "a forged token");
    }
    assert!(committed(&f.twist(&item(1))));
}

/// LI.6, KN5.18 (draft: a twist's token binds the state and protection revisions, so a press from before a commit, a
/// protection change or a policy change is `KN5:stale.revision_changed`, which comes before step 8's protection check;
/// a workspace completion advances neither revision, so a press from before it commits), CT-ONE-11 (part: step 7, a
/// head that differs from the head's recorded state, is `KN5:stale.state_hash_mismatch`).
#[test]
fn a_press_binds_the_state_and_protection_revisions() {
    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.note", Kind::Workspace, &[], overlay("note")).unwrap();

    let press = f.press(&item(1));
    assert!(committed(&f.twist(&item(2))));
    f.refused(press, code::REVISION_CHANGED, "a press from before a commit");

    // Item 1's word moves every orbit: with orbit 0 protected a fresh press is refused at step 8, and a press from
    // before the protection change at the revision binding before it. The release with its confirmation is a
    // protection change too.
    let press = f.press(&item(1));
    assert!(completed(&f.protect(&[0], false)));
    f.refused(press, code::REVISION_CHANGED, "a press from before a protection change");
    let press = f.press(&item(1));
    f.refused(press, code::ORBIT_WOULD_MOVE, "a fresh press that moves the protected orbit");
    let press = f.press(&item(1));
    assert!(completed(&f.protect(&[], true)));
    f.refused(press, code::REVISION_CHANGED, "a press from before the release");

    let press = f.press(&item(1));
    assert!(completed(&f.set_policy("strict")));
    f.refused(press, code::REVISION_CHANGED, "a press from before a policy change");

    let press = f.press(&item(1));
    assert!(running(&f.shell.invoke(&mut f.layer, "tool.full.note", json::object([])).unwrap()));
    let job = f.layer.take_program_job().unwrap().job;
    assert!(completed(&f.layer.finish_program_job(job, Ok(text("note"))).unwrap()));
    assert!(committed(&f.shell.send(&mut f.layer, press)), "a workspace completion keeps the press fresh");

    let head = f.layer.snapshot().clone();
    f.layer.fault_replace_head(derived(&head, &word(&[3])));
    let press = f.press(&item(1));
    f.refused(press, code::STATE_HASH_MISMATCH, "a head that differs");
    f.layer.fault_replace_head(head);
    assert!(committed(&f.twist(&item(1))));
}

/// CT-EV4-04 (part: a twist is admitted once: a retried delivery answers the stored receipt and commits nothing more,
/// also after a restart and a reopen; the same command ID with other content is `KN5:input.command_id_reused`, and the
/// spent token under another command ID is `KN5:auth.token_used`), CT-KN5-04 (part: the same for a twist), KN5.20,
/// KN5.21.
#[test]
fn a_retried_twist_answers_its_receipt_and_commits_once() {
    let mut f = fixture(&[]);
    let press = f.press(&item(1));
    let done = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&done), "{:?}", done.status);
    let kept = f.fixed();
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done);
    assert_eq!(f.fixed(), kept);
    let other = Invocation { args: args(&item(2)), ..press.clone() };
    f.refused(other, code::COMMAND_ID_REUSED, "other content under the command ID");
    let replay = Invocation { command: "replay/1".into(), ..press.clone() };
    f.refused(replay, code::TOKEN_USED, "the spent token under another command ID");

    f.layer.restart(Secret::fixed(SECRET));
    assert_eq!(f.layer.outcome(&press.command, &done.request), Some(done.clone()));
    assert_eq!(f.layer.invoke(&Caller::Shell, press.clone()), done);
    assert_eq!(f.layer.store().log.len(), 1);

    let mut g = reopen(&f.layer);
    assert_eq!(g.layer.outcome(&press.command, &done.request), Some(done.clone()));
    assert_eq!(g.layer.invoke(&Caller::Shell, press), done);
    assert_eq!(g.layer.store().log.len(), 1);
}

/// CT-EV4-04 (part: busy input is not queued: while the commit lane is held, the row is unavailable and a twist is
/// refused `KN5:busy.commit_in_flight`; nothing waits to commit when the lane frees, and the refused press keeps its
/// answer), L15.
#[test]
fn a_held_commit_lane_refuses_a_twist_and_queues_nothing() {
    let mut f = fixture(&[]);
    f.layer.hold_commit_lane(true);
    assert_eq!(f.layer.availability(TWIST_ROW).unwrap_err().code, code::COMMIT_IN_FLIGHT);
    let held = f.press(&item(1));
    let busy = f.refused(held.clone(), code::COMMIT_IN_FLIGHT, "a held lane");
    f.layer.hold_commit_lane(false);
    assert_eq!(f.layer.availability(TWIST_ROW), Ok(()));
    assert_eq!(f.layer.run_jobs(), 0);
    assert_eq!(f.layer.store().log.len(), 0);
    assert_eq!(f.layer.invoke(&Caller::Shell, held), busy);
    assert!(committed(&f.twist(&item(1))));
}

/// KN2.4, KN2.7, KN5.9 and step 8 (draft: a twist is judged under the protection policy in force, as a word commit is):
/// item 1's word moves every orbit, so with orbit 0 protected its twist is `KN2:protection.orbit_would_move` under Net
/// and under Strict with nothing changed, and with no orbit protected it commits under both. Since the word moves every
/// orbit at its end, the two policies agree on it.
#[test]
fn a_twist_is_judged_under_the_protection_in_force() {
    assert_eq!(moved_orbits(&[1]).len(), ORBITS);
    for policy in ["net", "strict"] {
        let mut f = fixture(&[0]);
        let mut g = fixture(&[]);
        if policy == "strict" {
            assert!(completed(&f.set_policy(policy)) && completed(&g.set_policy(policy)));
        }
        let want = policy_from_name(policy).unwrap();
        assert_eq!((f.layer.policy(), g.layer.policy()), (want, want));
        let press = f.press(&item(1));
        f.refused(press, code::ORBIT_WOULD_MOVE, policy);
        assert!(committed(&g.twist(&item(1))), "{policy}");
    }
}

/// KN5.18, KN5.20 and the move tree (draft: a twist is a commit node like a word commit, so its undo and redo are the
/// ordinary inverse and repeat, journaled without a rotation; a reopen replays the twist from its recorded recipe, runs
/// neither the producer nor the chooser again (KN3.9) and needs no menu; the menu is a host setting, so a reopened
/// layer refuses `twist.apply` until the host pins one again).
#[test]
fn a_twist_undoes_redoes_and_replays_without_a_menu() {
    let mut f = fixture(&[]);
    let start = f.layer.revision().digest.clone();
    assert!(committed(&f.twist(&item(6))));
    let moved = f.layer.revision().digest.clone();
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.revision().digest, start);
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.layer.revision().digest, moved);
    assert!(committed(&f.twist(&Binding { cap: 7, ..item(3) })));
    let log = f.layer.store().log.clone();
    let kinds: Vec<TransactionKind> = log.iter().map(|t| t.kind).collect();
    let expected = [TransactionKind::Commit, TransactionKind::Undo, TransactionKind::Redo, TransactionKind::Commit];
    assert_eq!(kinds, expected);
    let rotations: Vec<bool> = log.iter().map(|t| t.rotation.is_some()).collect();
    assert_eq!(rotations, [true, false, false, true]);

    let mut g = reopen(&f.layer);
    assert_eq!(restored(&g.layer), restored(&f.layer));
    assert_eq!(g.layer.store().log, log);
    assert_eq!(g.layer.availability(TWIST_ROW).unwrap_err().code, code::ROW_NOT_BUILT);
    let press = g.press(&item(2));
    g.refused(press, code::ROW_NOT_BUILT, "a reopened layer without a pin");
    g.layer.pin_menu(ctx(), pinned()).unwrap();
    assert!(committed(&g.twist(&item(2))));
    assert!(committed(&g.step(Direction::Undo)));
    assert!(committed(&g.step(Direction::Undo)));
    assert_eq!(g.layer.revision().digest, moved);
    let h = reopen(&g.layer);
    assert_eq!(restored(&h.layer), restored(&g.layer));
}

/// The journal form (draft): a twist's commit carries `rotation`, its resolution's record, as an 18th key, and reads
/// back to itself; no other transaction carries one. A twist without its rotation, a rotation on a word commit or on
/// an undo, and a rotation that is not a record of a menu resolution on 600-cell-Full do not read.
#[test]
fn a_twist_journals_its_rotation() {
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[2]))));
    assert!(committed(&f.twist(&item(1))));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    let log = f.layer.store().log.clone();
    let keys: Vec<usize> = log.iter().map(|t| t.to_json().as_object().unwrap().len()).collect();
    assert_eq!(keys, [17, 18, 17, 17]);
    for transaction in &log {
        assert_eq!(Transaction::from_json(&transaction.to_json()).as_ref(), Some(transaction));
    }
    let rotation = log[1].rotation.clone().expect("the twist's rotation");
    assert_eq!(log[1].to_json().get("rotation"), Some(&rotation));

    let object_of = |transaction: &Transaction| match transaction.to_json() {
        Value::Object(fields) => fields,
        _ => panic!("a transaction is an object"),
    };
    let carrying = |transaction: &Transaction, rotation: Option<Value>| {
        let mut fields = object_of(transaction);
        match rotation {
            Some(rotation) => fields.insert("rotation".to_owned(), rotation),
            None => fields.remove("rotation"),
        };
        Value::Object(fields)
    };
    let edited = |edit: fn(&mut BTreeMap<String, Value>)| {
        let Value::Object(mut fields) = rotation.clone() else { panic!("a rotation record is an object") };
        edit(&mut fields);
        carrying(&log[1], Some(Value::Object(fields)))
    };
    let choice = |kind: &str, index: i64| json::object([("index", Value::Int(index)), ("kind", text(kind))]);
    let drag = json::object([
        ("distance2", Value::Int(0)),
        ("index", Value::Int(1)),
        ("kind", text("menu_drag")),
        ("ties", Value::Array(vec![Value::Int(1)])),
    ]);
    let malformed = [
        (carrying(&log[1], None), "a twist without its rotation"),
        (carrying(&log[1], Some(Value::Null)), "a null rotation"),
        (carrying(&log[0], Some(rotation.clone())), "a word commit with a rotation"),
        (carrying(&log[2], Some(rotation.clone())), "an undo with a rotation"),
        (edited(|r| drop(r.insert("request_digest".into(), text(&"0".repeat(64))))), "another request digest"),
        (edited(|r| drop(r.insert("cap".into(), Value::Int(1)))), "a cap that is not the binding's"),
        (edited(|r| drop(r.insert("binding".into(), json::object([])))), "a binding that does not read"),
        (edited(|r| drop(r.insert("extra".into(), Value::Null))), "an extra field"),
        (edited(|r| drop(r.remove("matrix"))), "no matrix"),
        (edited(|r| drop(r.insert("matrix".into(), text("identity")))), "a matrix that is not exact"),
        (carrying(&log[1], Some(with_choice(&rotation, choice("free", 1)))), "a free choice"),
        (edited(|r| drop(r.insert("choice".into(), json::object([("kind", text("menu_direct"))])))), "no index"),
        (carrying(&log[1], Some(with_choice(&rotation, choice("menu_direct", -1)))), "a negative index"),
        (carrying(&log[1], Some(with_choice(&rotation, drag))), "a drag's choice for a direct binding"),
    ];
    for (value, what) in malformed {
        assert_eq!(Transaction::from_json(&value), None, "{what}");
    }
    let other = carrying(&log[1], Some(with_choice(&rotation, choice("menu_direct", 59))));
    assert!(Transaction::from_json(&other).is_some(), "the journal form does not check the item against a menu");
}

fn with_choice(rotation: &Value, choice: Value) -> Value {
    let Value::Object(mut fields) = rotation.clone() else { panic!("a rotation record is an object") };
    fields.insert("choice".to_owned(), choice);
    Value::Object(fields)
}

/// LI.32 and KN5.18 (draft: a twist ends what read the state it replaces, as a word commit does): the pending preview
/// is discarded and the slot left empty, so a press for it is `KN5:stale.no_fresh_preview` (ruling 1); a running and a
/// queued query job are cancelled with `replaced_revision` and a late result is dropped; a file job keeps running.
#[test]
fn a_twist_ends_the_preview_and_the_jobs_that_read_the_state() {
    let mut f = fixture(&[]);
    f.layer.register_overlay("tool.full.inspect", Kind::Query, &[], overlay("inspect")).unwrap();
    f.layer.register_overlay("tool.full.scan", Kind::Query, &[], overlay("scan")).unwrap();
    f.layer.register_overlay("tool.full.export", Kind::File, &[], overlay("export")).unwrap();
    f.layer.set_program_capacity(3);
    let p = f.preview(&word(&[2]));
    f.present(p);
    let execute =
        f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(p as i64))])).unwrap();
    let inspect = f.shell.invoke(&mut f.layer, "tool.full.inspect", json::object([])).unwrap();
    let scan = f.shell.invoke(&mut f.layer, "tool.full.scan", json::object([])).unwrap();
    let export = f.shell.invoke(&mut f.layer, "tool.full.export", json::object([])).unwrap();
    assert!(running(&inspect) && running(&scan) && running(&export));
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(taken.row, "tool.full.inspect");

    assert!(committed(&f.twist(&item(1))));
    assert_eq!(f.layer.preview_state(p), Some(PreviewState::Discarded));
    assert!(f.layer.preview().is_none());
    f.refused(execute, code::NO_FRESH_PREVIEW, "the press for the discarded preview");
    for receipt in [&inspect, &scan] {
        let ended = f.layer.outcome(&receipt.command, &receipt.request).unwrap();
        assert_eq!(ended.status, Status::Cancelled(CancelReason::ReplacedRevision));
    }
    assert_eq!(f.layer.outcome(&export.command, &export.request).as_ref(), Some(&export));
    assert!(f.layer.finish_program_job(taken.job, Ok(Value::Null)).is_none());
    assert_eq!(f.layer.take_program_job().unwrap().row, "tool.full.export");
    assert_eq!(f.layer.store().log.len(), 1);
}

/// KN5.15: the runtime may not invoke a commit row, so its `twist.apply` is `KN5:input.commit_row_refused` and changes
/// nothing, with or without a pinned menu.
#[test]
fn the_runtime_may_not_twist() {
    for mut f in [fixture(&[]), unpinned(&[])] {
        let runtime = Caller::Runtime { run: "run-1".into() };
        let epoch = f.layer.revision().epoch;
        let theirs = Invocation {
            command: "run-1/1".into(),
            row: TWIST_ROW.into(),
            args: args(&item(1)),
            token: None,
            epoch,
            budget: None,
        };
        f.refused_on(&runtime, theirs, code::COMMIT_ROW_REFUSED, "the runtime");
    }
}
