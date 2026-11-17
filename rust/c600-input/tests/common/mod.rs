//! The fixture the routing and repeat tests share: the command layer over the model in `assets/`, its shell, and a
//! router on the draft default map.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{CommandLayer, HeadlessFrames, Kind, MemoryStore, Overlay, Receipt, Secret, Shell, Status, Table};
use c600_input::{
    default_specs, BindingSpec, Context, Entry, KeyEvent, KeyMap, Layout, ListFocus, Routed, Router, RoutingClass, Slot,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

pub const SECRET: [u8; 32] = [7; 32];

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
