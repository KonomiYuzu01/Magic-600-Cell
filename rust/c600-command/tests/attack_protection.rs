//! Attack tests on protection (class D of the boundary attack): ways for a committed action to move a protected orbit
//! with no refusal and no confirmed release. Each test asserts the rule, so it fails while the bypass exists; a test
//! that fails on the attacked tree is `#[ignore]`d with what was observed. Findings file:
//! `kn5-boundary-attack-findings.md` in the integrator's notes folder; the IDs below are its IDs.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, CommandLayer, Direction, HeadlessFrames, MemoryStore, Receipt, Secret, Shell, Status, Table,
    TransactionKind, JUMBLE_OFF_ROW, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, POLICY_ROW,
};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

/// The I_a menu's set digest and order digest (`research/jumbling/fixtures/wj-I_a.json`), as KN3's tests pin them.
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

fn pinned() -> Menu {
    let path = repository().join("research/jumbling/fixtures/wj-I_a.json");
    let fixture = json::parse(&std::fs::read(path).expect("the I_a fixture")).expect("the I_a fixture is JSON");
    Menu::from_record(&ctx(), fixture.get("menu").expect("a menu")).expect("the pinned I_a menu")
}

/// Menu item `index` at cap 0, pressed directly. At cap 0 items 1 to 11 are retained items.
fn item(index: u64) -> Binding {
    Binding {
        request: wire::menu_request(SET, ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: SET.to_owned(), order: ORDER.to_owned() }),
    }
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(_))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects `orbits`, with the I_a menu pinned.
fn fixture(orbits: &[i64]) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(orbits).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.pin_menu(ctx(), pinned()).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    /// A crossing row's press, naming the current revision.
    fn cross(&mut self, row: &str) -> Receipt {
        let revision = self.layer.revision().to_json();
        self.shell.invoke(&mut self.layer, row, json::object([("revision", revision)])).unwrap()
    }

    /// `jumble.twist.menu` for `binding`, naming the current revision.
    fn menu_twist(&mut self, binding: &Binding) -> Receipt {
        let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
        let cap = fields.remove("cap").expect("a binding has a cap");
        let revision = self.layer.revision().to_json();
        let args = json::object([("cap", cap), ("letter", Value::Object(fields)), ("revision", revision)]);
        self.shell.invoke(&mut self.layer, JUMBLE_TWIST_ROW, args).unwrap()
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let args = json::object([("preview", Value::Int(id as i64))]);
        self.shell.invoke(&mut self.layer, "operation.execute", args).unwrap()
    }

    fn step(&mut self, direction: Direction, confirm: bool) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        let args = if confirm {
            json::object([("confirm", Value::Bool(true)), ("hint", hint.to_json())])
        } else {
            json::object([("hint", hint.to_json())])
        };
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    fn protect(&mut self, orbits: &[i64], confirm: bool) -> Receipt {
        let orbits = Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect());
        let args = if confirm {
            json::object([("confirm", Value::Bool(true)), ("orbits", orbits)])
        } else {
            json::object([("orbits", orbits)])
        };
        self.shell.invoke(&mut self.layer, "protection.orbit.set", args).unwrap()
    }
}

/// B5 (class D; the solver's own presses, so no attacker is needed). The owner's rule: a protected piece moves only
/// after a release the solver confirmed. Frozen text: `jumble.twist.menu` has H-08's `protection` gate; KN1.11 [D]
/// "Inside Jumble the verdict is pose-level: m_A(w) = 1"; ONE.80 [D] a crossing runs "A5's exact crossing/protection
/// checks"; JI.5 [B] "Net and Strict protection use poses inside Jumble"; JI.27 [D] "The sole crossing exception is a
/// protection change required at the new level: record its changed fields and inverse, validate it through the normal
/// protection path".
///
/// Orbit 0 is protected, and with Jumble off the retained item 1 is refused: it would move orbit 0. Three presses
/// later the same item's labels are the head state, orbit 0 is still in the protected set, no press was refused and
/// none asked for a release: `jumble.on`, the same item as a pose move, `jumble.off`. The crate documents the cause as
/// a draft (a pose move is judged with no protected piece, and the exit checks no protected orbit), and two existing
/// tests pin it (`protection_judges_a_retained_item_and_no_pose`, `refusals_follow_the_exit_order_and_change_nothing`).
///
/// The assertion is the part both readings of the frozen text share. Reading 1 (the gate's): the pose move is refused.
/// Reading 2 (the draft's, "no label-to-pose transport"): the label-level protection has no force inside Jumble, and
/// then JI.27 asks the entry to record that protection change, through the path that confirms a release.
///
/// Observed on 4c20ce30, under Net and under Strict: the direct twist is refused with
/// `KN2:protection.orbit_would_move`; then the entry, the pose move and the exit all answer `committed`; the entry's
/// journal row keeps orbits `[0]` with `confirm: false`; the head digest is no longer the solved digest and the
/// protected set is still `[0]`.
#[test]
#[ignore = "B5: fails on 4c20ce30, a round trip through Jumble moves a protected orbit with no refusal or release"]
fn b5_a_round_trip_through_jumble_does_not_move_a_protected_orbit() {
    let mut bypassed = Vec::new();
    for policy in ["net", "strict"] {
        let mut f = fixture(&[0]);
        if policy == "strict" {
            let args = json::object([("confirm", Value::Bool(true)), ("policy", Value::String(policy.into()))]);
            assert!(completed(&f.shell.invoke(&mut f.layer, POLICY_ROW, args).unwrap()));
        }
        let solved = f.layer.revision().digest.clone();
        let direct = f.menu_twist(&item(1));
        assert_eq!(direct.status.code(), Some(code::ORBIT_WOULD_MOVE), "{policy}: the item moves orbit 0");

        let entry = f.cross(JUMBLE_ON_ROW);
        let moved = f.menu_twist(&item(1));
        let exit = f.cross(JUMBLE_OFF_ROW);

        let log = &f.layer.store().log;
        let entered = log.iter().find(|t| t.kind == TransactionKind::Crossing);
        let released = entered.is_some_and(|t| t.confirm || !t.orbits.contains(&0));
        let round_trip = committed(&entry) && committed(&moved) && committed(&exit);
        if round_trip && !released {
            bypassed.push(format!(
                "{policy}: the entry, the move and the exit committed, the entry recorded no release, the head went \
                 from {solved} to {} and the protected set is {:?}",
                f.layer.revision().digest,
                f.layer.protected().orbits(),
            ));
        }
    }
    assert!(bypassed.is_empty(), "{bypassed:#?}");
}

/// Guard (refuted, class D). A commit that moved orbit 0 before the orbit was protected cannot be undone past the
/// protection: the protection change is the undo target first, and its undo releases the orbit, so it needs the
/// solver's confirmation (LI.26). A redo cannot bring a word back over a protection that came later either: the
/// protection change is a new action and empties the redo stack.
#[test]
fn undo_and_redo_do_not_step_over_a_later_protection() {
    // The retained word of item 1 moves orbit 0; find a plain word that does, by trying letters under protection.
    let mut probe = fixture(&[0]);
    let letter = (1..40)
        .find(|&l| probe.commit(&word(&[l])).status.code() == Some(code::ORBIT_WOULD_MOVE))
        .expect("a letter that moves orbit 0");

    // Undo: commit the letter, protect orbit 0, then press undo. The target is the protection change, never the word.
    let mut f = fixture(&[]);
    assert!(committed(&f.commit(&word(&[letter]))));
    let after_word = f.layer.revision().digest.clone();
    assert!(completed(&f.protect(&[0], false)));
    let unconfirmed = f.step(Direction::Undo, false);
    assert!(matches!(unconfirmed.status, Status::Refused(_)), "{:?}", unconfirmed.status);
    assert_eq!(f.layer.revision().digest, after_word);
    assert_eq!(f.layer.protected().orbits().to_vec(), vec![0]);

    // Redo: commit the letter, undo it, protect orbit 0. The word is no redo target any more.
    let mut g = fixture(&[]);
    assert!(committed(&g.commit(&word(&[letter]))));
    assert!(committed(&g.step(Direction::Undo, false)));
    let solved = g.layer.revision().digest.clone();
    assert!(g.layer.undo_hint(Direction::Redo).is_some());
    assert!(completed(&g.protect(&[0], false)));
    assert!(g.layer.undo_hint(Direction::Redo).is_none(), "the word is still a redo target under the protection");
    assert_eq!(g.layer.revision().digest, solved);
}

/// Guard (refuted, class D). The undo of a protection change that the solver later redoes is judged again: after the
/// protection is undone (a confirmed release) and the word that moves the orbit is committed, the redo of the
/// protection is gone, so no redo protects an orbit a commit has since moved without the solver setting it again.
#[test]
fn a_redo_of_a_protection_does_not_outlive_a_commit() {
    let mut probe = fixture(&[0]);
    let letter = (1..40)
        .find(|&l| probe.commit(&word(&[l])).status.code() == Some(code::ORBIT_WOULD_MOVE))
        .expect("a letter that moves orbit 0");
    let mut f = fixture(&[]);
    assert!(completed(&f.protect(&[0], false)));
    assert!(committed(&f.step(Direction::Undo, true)), "the confirmed release");
    assert!(f.layer.protected().orbits().is_empty());
    assert!(committed(&f.commit(&word(&[letter]))));
    assert!(f.layer.undo_hint(Direction::Redo).is_none());
}
