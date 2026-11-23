//! CT-EV4-06 on the shell's key map (EV4.3, EV4.4, EV4.7; LP4-8): the shell's map is the checked map the input router
//! uses ([`c600_command::keys`]), and `Shell::bind` runs EV4.7's checks of the whole map before a binding applies:
//! duplicates and the prefix-code property separately in each routing class, and every check under each declared
//! layout, so the DT-M1 US and French counterexample fails under its conflicting layout while US is active. A refused
//! binding leaves the map, and what its keys do, as they were. `c600-input/tests/keymap.rs` runs the same checks on the
//! specs alone. CT-EV4-06 (synthetic, part: the checks through `Shell::bind` on the shell's map; with
//! `c600-input/tests/keymap.rs` the whole by union).
//!
//! The layouts are tables in `c600_command::keys`, not a platform's: none of this is Windows input or IME evidence
//! (CT-EV4-05 stays a windows test). No assertion reads a refusal's detail text.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::keys::{default_specs, Arg, BindingSpec, KeyMap, Layout, RoutingClass};
use c600_command::{code, CommandLayer, HeadlessFrames, MemoryStore, Receipt, Secret, Shell, Status, Table};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

use RoutingClass::{Global, List};

const SECRET: [u8; 32] = [7; 32];
const TOGGLE: &str = "keyboard.toggle";

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

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    /// Previews, presents and commits `letters` by the shell's keys.
    fn commit(&mut self, letters: &[i64]) {
        self.shell.draft = Some(c600_command::word_draft(&word(letters)));
        let receipt = self.press("Ctrl+Enter").unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        self.layer.run_jobs();
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let receipt = self.press("Ctrl+Shift+Enter").unwrap();
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    }

    fn press(&mut self, sequence: &str) -> Result<Receipt, c600_command::Error> {
        self.shell.press(&mut self.layer, sequence, false).map(|receipt| receipt.expect("not a repeat"))
    }

    /// Binds `spec`, expecting a refusal with `code`, and checks that the map stayed as it was.
    fn refused(&mut self, spec: BindingSpec, expected: &str) {
        let before = self.shell.keymap().clone();
        let error = self.shell.bind(spec).unwrap_err();
        assert_eq!(error.code, expected);
        assert_eq!(self.shell.keymap(), &before, "a refused binding changes nothing");
    }
}

fn spec(sequence: &[&str], class: RoutingClass, row: &str) -> BindingSpec {
    BindingSpec::new(sequence, class, row, vec![])
}

/// CT-EV4-06, one map (EV4.7; LP4-8): the shell starts on the router's draft default map under US QWERTY, declared
/// with French AZERTY, so the key route and the router resolve the same chords to the same rows; an applied map under
/// AZERTY moves undo to the key that types `z` there.
#[test]
fn the_shell_and_the_router_share_one_checked_map() {
    let mut f = fixture();
    assert_eq!(f.shell.keymap(), &KeyMap::default_for("us-qwerty").unwrap());
    assert_eq!(f.shell.keymap().specs(), default_specs().as_slice());
    assert_eq!(f.shell.keymap().declared(), ["us-qwerty", "fr-azerty"]);

    f.commit(&[1]);
    f.commit(&[2]);
    let undone = f.press("Ctrl+'z'").unwrap();
    assert!(matches!(undone.status, Status::Committed(_)), "a character intent resolves under the active layout");
    f.shell.apply_keymap(KeyMap::default_for("fr-azerty").unwrap());
    assert_eq!(f.press("Ctrl+KeyZ").unwrap_err().code, code::UNKNOWN_ROW, "KeyZ types `w` on AZERTY");
    let undone = f.press("Ctrl+KeyW").unwrap();
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);
}

/// CT-EV4-06, duplicates (EV4.3, EV4.7): a chord sequence the map binds in one routing class is refused there with
/// `EV4:input.binding_conflict`, in the global class and in the list class alike, and the same sequence binds in
/// another class. A second binding of a whole prefix-group sequence is a duplicate, not a prefix conflict.
#[test]
fn duplicates_fail_per_routing_class() {
    let mut f = fixture();
    f.refused(spec(&["F9"], Global, "display.frame.toggle"), code::BINDING_CONFLICT);
    f.refused(spec(&["ArrowUp"], List, "object.inspect"), code::BINDING_CONFLICT);
    f.refused(spec(&["Ctrl+KeyG", "KeyF"], Global, TOGGLE), code::BINDING_CONFLICT);
    f.refused(spec(&["Ctrl+'z'"], Global, TOGGLE), code::BINDING_CONFLICT);
    f.shell.bind(spec(&["F9"], List, "list.move")).unwrap();
    f.shell.bind(spec(&["F2"], Global, TOGGLE)).unwrap();
    f.refused(spec(&["F2"], Global, "display.frame.toggle"), code::BINDING_CONFLICT);
    assert_eq!(f.shell.keymap().bindings().len(), default_specs().len() + 2);
}

/// CT-EV4-06, the prefix-code property (EV4.7): a complete binding that is also a prefix of another sequence in its
/// routing class is refused with `EV4:input.binding_prefix_conflict`, whichever is bound first; in another class the
/// same chord binds. A binding with no chord is a prefix of every sequence. The key route resolves a whole bound
/// sequence and refuses a bare prefix group.
#[test]
fn a_complete_binding_that_is_also_a_prefix_fails_per_routing_class() {
    let mut f = fixture();
    f.refused(spec(&["Ctrl+KeyG"], Global, TOGGLE), code::BINDING_PREFIX_CONFLICT);
    f.refused(spec(&["F9", "KeyA"], Global, "display.frame.toggle"), code::BINDING_PREFIX_CONFLICT);
    f.refused(spec(&[], Global, TOGGLE), code::BINDING_PREFIX_CONFLICT);
    f.shell.bind(spec(&["Ctrl+KeyG"], List, "list.move")).unwrap();

    assert_eq!(f.press("Ctrl+KeyG").unwrap_err().code, code::UNKNOWN_ROW, "a prefix group alone binds no row");
    let group = f.press("Ctrl+KeyG KeyF").unwrap();
    assert_eq!(group.status.code(), Some(code::ROW_NOT_BUILT), "the group's row reached the layer");
}

/// CT-EV4-06, under each declared layout (EV4.7; DT-M1 Q2 (b) and K5): a physical chord that a character intent of the
/// map resolves onto under a declared layout is refused with `EV4:input.binding_layout_mismatch` while another layout is
/// active, since the conflict is reported before the map applies under any of them. The DT-M1 US and French
/// counterexample: `Shift+KeyQ` by position and `A` by character are two chords on US QWERTY and one on French
/// AZERTY. A character that one declared layout does not type (`é` on US QWERTY) is refused the same way.
#[test]
fn the_us_and_french_counterexample_fails_under_its_conflicting_layout() {
    let mut f = fixture();
    f.refused(spec(&["Ctrl+KeyW"], Global, TOGGLE), code::BINDING_LAYOUT_MISMATCH);
    f.shell.bind(spec(&["Shift+KeyQ"], Global, TOGGLE)).unwrap();
    f.refused(spec(&["'A'"], Global, "display.frame.toggle"), code::BINDING_LAYOUT_MISMATCH);
    f.refused(spec(&["Ctrl+'é'"], Global, "display.frame.toggle"), code::BINDING_LAYOUT_MISMATCH);

    // Declared for US alone, the same pair binds: the conflict is AZERTY's.
    let us = KeyMap::apply(&default_specs(), &[Layout::US], "us-qwerty").unwrap();
    f.shell.apply_keymap(us);
    f.shell.bind(spec(&["Shift+KeyQ"], Global, TOGGLE)).unwrap();
    f.shell.bind(spec(&["'A'"], Global, "display.frame.toggle")).unwrap();
}

/// A refused binding leaves what the keys do (EV4.7): after the refusals, undo is still bound to its chord and undoes,
/// and Escape still closes the transient layer and never cancels the presented preview (DT-M1 C4): the cancel key
/// does.
#[test]
fn a_refused_binding_leaves_what_the_keys_do() {
    let mut f = fixture();
    f.commit(&[1]);
    f.refused(spec(&["Ctrl+KeyZ"], Global, TOGGLE), code::BINDING_LAYOUT_MISMATCH);
    f.refused(spec(&["Escape"], Global, "operation.preview.cancel"), code::BINDING_CONFLICT);
    let undone = f.press("Ctrl+KeyZ").unwrap();
    assert!(matches!(undone.status, Status::Committed(_)), "{:?}", undone.status);

    f.shell.draft = Some(c600_command::word_draft(&word(&[3])));
    let preview = f.press("Ctrl+Enter").unwrap();
    let Status::Running(id) = preview.status else { panic!("{:?}", preview.status) };
    f.layer.run_jobs();
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    let escape = f.press("Escape").unwrap();
    assert_eq!(escape.status.code(), Some(code::ROW_NOT_BUILT), "Escape reaches `transient.close`");
    assert!(f.shell.presented().is_some(), "Escape cancels no preview");
    let cancel = f.press("Ctrl+Shift+Backspace").unwrap();
    assert!(matches!(cancel.status, Status::Completed(_)), "{:?}", cancel.status);
}

/// The shell's key route resolves only what it holds itself (EV4.3): a binding whose relative name needs the router's
/// focus, grip or menu is refused before any envelope with `KN5:input.argument_form`, and nothing reaches the layer; a
/// binding with a fixed absolute argument sends it as bound.
#[test]
fn the_key_route_refuses_a_name_only_the_router_resolves() {
    let mut f = fixture();
    let before = f.shell.sent.len();
    assert_eq!(f.press("KeyQ").unwrap_err().code, code::ARGUMENT_FORM, "a grip slot is the router's");
    assert_eq!(f.press("KeyJ").unwrap_err().code, code::ARGUMENT_FORM, "a menu item is the router's");
    assert_eq!(f.shell.sent.len(), before, "no envelope was sent");
    let zoom = vec![
        ("view", Arg::Fixed(Value::String("global".into()))),
        ("scale", Arg::Fixed(json::object([("step", Value::Int(2))]))),
    ];
    f.shell.bind(BindingSpec::new(&["F2"], Global, "camera.zoom", zoom)).unwrap();
    f.press("F2").unwrap();
    assert_eq!(f.shell.sent.len(), before + 1);
}
