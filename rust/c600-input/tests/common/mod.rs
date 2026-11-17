//! The fixture the routing, repeat and twist tests share: the command layer over the model in `assets/`, its shell, and
//! a router on the draft default map; for the twist keys, the I_a twist menu that the host pins.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{CommandLayer, HeadlessFrames, Kind, MemoryStore, Overlay, Receipt, Secret, Shell, Status, Table};
use c600_input::{
    default_specs, BindingSpec, Context, Entry, KeyEvent, KeyMap, Layout, ListFocus, Routed, Router, RoutingClass, Slot,
};
use c600_jumble::rotation;
use c600_jumble::twist::Menu;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{menu_request, Binding, MenuPin, Operation};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};

pub const SECRET: [u8; 32] = [7; 32];

/// The I_a twist menu's set and order digests: the menu that `c600-command`'s twist tests pin.
pub const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
pub const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";

/// The grip slots of the twist tests: four caps spread over the model, as integers, the form `twist.apply`'s `cap`
/// takes.
pub const CAPS: [u32; 4] = [0, 7, 123, 599];

/// The language-defined tool row the tests bind to a key, and its chord.
pub const TOOL: &str = "tools.count";
pub const TOOL_KEY: &str = "Ctrl+Shift+KeyT";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

pub fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

pub fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}

/// An inline literal recipe of one word.
pub fn word(letters: &[i64]) -> Value {
    parse(&format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#))
}

pub fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// The applied slot map of the tests: slot `n` holds cap `c{n}` in frame `f{n}`.
pub fn slots(tag: &str) -> Vec<Slot> {
    (0..4).map(|n| Slot { cap: text(&format!("{tag}c{n}")), frame: text(&format!("{tag}f{n}")) }).collect()
}

/// The applied slot map of the twist tests: slot `n` holds cap `CAPS[n]`, as an integer, in frame `f{n}`.
pub fn cap_slots() -> Vec<Slot> {
    CAPS.iter()
        .enumerate()
        .map(|(n, cap)| Slot { cap: Value::Int(i64::from(*cap)), frame: text(&format!("f{n}")) })
        .collect()
}

/// The jumbling engine's exact model of `assets/`, which a twist menu is read in.
pub fn exact() -> Arc<c600_jumble::Context> {
    static EXACT: OnceLock<Arc<c600_jumble::Context>> = OnceLock::new();
    let load = || Arc::new(c600_jumble::Context::load(&repository().join("assets")).expect("the exact model loads"));
    EXACT.get_or_init(load).clone()
}

/// The I_a twist menu of `research/jumbling/fixtures/wj-I_a.json`, read in the exact model.
pub fn menu() -> Menu {
    static RECORD: OnceLock<Value> = OnceLock::new();
    let record = RECORD.get_or_init(|| {
        let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).unwrap();
        json::parse(&fixture).unwrap().get("menu").expect("the fixture has a menu").clone()
    });
    Menu::from_record(&exact(), record).expect("the I_a menu")
}

/// The I_a menu's two digests, as the host gives them to the router with the menu it pins in the layer.
pub fn pin() -> MenuPin {
    MenuPin { set: SET.to_owned(), order: ORDER.to_owned() }
}

/// KN3.7's binding of item `index` of the pinned menu at `cap`, pressed directly.
pub fn item(cap: u32, index: u64) -> Binding {
    Binding { request: menu_request(SET, ORDER, index), cap, operation: Operation::MenuDirect, menu: Some(pin()) }
}

/// What a twist key of item `index` must send as `twist.apply`'s `rotation`: KN3.7's binding without its cap, built
/// with the kernel's own binding form.
pub fn rotation(index: u64) -> Value {
    let Value::Object(mut fields) = item(0, index).to_json() else { panic!("a binding is an object") };
    fields.remove("cap");
    Value::Object(fields)
}

/// The retained word of item `index` at `cap`, as KN3's resolution gives it against the pinned menu.
pub fn retained(cap: u32, index: u64) -> Vec<i64> {
    static MENU: OnceLock<Menu> = OnceLock::new();
    let menu = MENU.get_or_init(menu);
    let resolution = rotation::resolve(&exact(), Some(menu), &item(cap, index)).expect("the item resolves");
    rotation::retained_word(&exact(), &resolution).expect("a retained item")
}

/// The state after `letters` from `base`, derived by the kernel outside any session.
pub fn derived(base: &Snapshot, letters: &[i64]) -> Snapshot {
    let k = kernel();
    match k.derive(base, &k.word(letters).unwrap(), &ProtectedSet::none()).unwrap() {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("unexpected refusal: {}", refused.refusal),
    }
}

/// A focused list of `n` entries `e0`, `e1`, ... showing pieces `p0`, `p1`, ..., with entry `selected` selected.
pub fn list(n: usize, selected: usize) -> ListFocus {
    ListFocus {
        list: text("repairs"),
        result: json::object([("job", Value::Int(9)), ("digest", text("r"))]),
        entries: (0..n).map(|i| Entry { id: text(&format!("e{i}")), object: text(&format!("p{i}")) }).collect(),
        selected,
    }
}

/// The default specs with the tool row bound to [`TOOL_KEY`].
pub fn specs_with_tool() -> Vec<BindingSpec> {
    let mut specs = default_specs();
    specs.push(BindingSpec::new(&[TOOL_KEY], RoutingClass::Global, TOOL, vec![]));
    specs
}

pub struct Fixture {
    pub layer: CommandLayer,
    pub shell: Shell,
    pub router: Router,
    /// The headless display's frames, which present the pending preview through the built-in chain.
    pub frames: HeadlessFrames,
    prefix: String,
}

/// A fresh layer at the solved state, a shell whose command IDs start with `prefix`, and a router on the default map
/// under `layout`, with the tool row registered and bound.
pub fn fixture(prefix: &str, layout: &str) -> Fixture {
    let k = kernel();
    let protected = k.protected_set(&[]).unwrap();
    let start = k.solved();
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let overlay = Overlay { module: "tools/count".into(), entry: "count".into() };
    layer.register_overlay(TOOL, Kind::Query, &[], overlay).unwrap();
    let keymap = KeyMap::apply(&specs_with_tool(), &[Layout::US, Layout::FRENCH], layout).unwrap();
    let router = Router::new(keymap, Context::new(slots("")));
    let shell = Shell::new(Secret::fixed(SECRET), prefix);
    Fixture { layer, shell, router, frames: HeadlessFrames::new(), prefix: prefix.to_owned() }
}

impl Fixture {
    /// Pins the I_a menu as the host does: in the layer (`CommandLayer::pin_menu`) and, by its two digests, in the
    /// router's context. The applied slot map becomes [`cap_slots`], with the grip on none of them.
    pub fn pin_menu(&mut self) {
        let menu = menu();
        assert_eq!((menu.identity.as_str(), menu.order.as_str()), (SET, ORDER), "the I_a menu's digests");
        self.layer.pin_menu(exact(), menu).expect("the menu's model is the session's model");
        self.router.context.menu = Some(pin());
        self.router.context.rebind(cap_slots());
    }

    pub fn key(&mut self, event: KeyEvent) -> Routed {
        self.router.key(&mut self.shell, &mut self.layer, event)
    }

    pub fn down(&mut self, chord: &str) -> Routed {
        self.key(KeyEvent::down(chord))
    }

    pub fn up(&mut self, chord: &str) -> Routed {
        self.key(KeyEvent::up(chord))
    }

    /// A key-down and its key-up; the key-down's routing.
    pub fn tap(&mut self, chord: &str) -> Routed {
        let routed = self.down(chord);
        self.up(chord);
        routed
    }

    /// How many invocations reached the layer from this shell, counted by asking the layer's outcome lookup
    /// (`kernel.outcome`, no side effect) for each command ID in turn. It agrees with the shell's own record.
    pub fn reached(&self) -> usize {
        let mut n = 0;
        while self.layer.outcome(&format!("{}/{}", self.prefix, n + 1), "probe").is_some() {
            n += 1;
        }
        assert_eq!(n, self.shell.sent.len(), "every prepared invocation reached the layer");
        n
    }

    /// Runs the queued preview job and presents the preview the receipt names.
    pub fn build_and_present(&mut self, receipt: &Receipt) -> u64 {
        let Status::Running(job) = receipt.status else { panic!("not running: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, job).unwrap();
        job
    }

    /// Previews `letters` through the key route and presents it.
    pub fn preview(&mut self, letters: &[i64]) -> u64 {
        self.shell.draft = Some(c600_command::word_draft(&word(letters)));
        let routed = self.tap("Ctrl+Enter");
        let receipt = routed.receipt().expect("the preview reached the layer").clone();
        self.build_and_present(&receipt)
    }

    /// Commits `letters` through the key route.
    pub fn commit(&mut self, letters: &[i64]) -> Receipt {
        self.preview(letters);
        let routed = self.tap("Ctrl+Shift+Enter");
        let receipt = routed.receipt().expect("the commit reached the layer").clone();
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
        receipt
    }
}

/// The receipt's code, for a refused or failed receipt.
pub fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

/// The row and the absolute arguments an event's dispatch sent.
pub fn sent(routed: &Routed) -> (String, Value) {
    let dispatch = routed.dispatch().unwrap_or_else(|| panic!("nothing was dispatched: {routed:?}"));
    (dispatch.invocation.row.clone(), dispatch.invocation.args.clone())
}

/// The row an event's dispatch sent.
pub fn row(routed: &Routed) -> String {
    sent(routed).0
}
