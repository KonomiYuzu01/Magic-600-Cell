//! The jumbled puzzle through the command layer (LP2's pull requests 212, 217, 218 and 220), shared by the tests that
//! take up Jumble on: a 600-cell-Full session at solved with no protected orbit, a W-J fixture's menu pinned
//! (`CommandLayer::pin_menu`) and `jumble.on` pressed. Every move is one `jumble.twist.menu` press of the menu item whose
//! matrix the fixture's record holds, at its grip, naming the current revision, as LP2's route
//! (`c600-command/src/bin/c600-jumble-route.rs`) presses it; every undo is one `session.undo` with the layer's hint.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::JUMBLE_ON_MODE;
use c600_command::{
    CommandLayer, Direction, MemoryStore, Receipt, Secret, Shell, Status, Table, JUMBLE_OFF_ROW, JUMBLE_ON_ROW,
    JUMBLE_TWIST_ROW,
};
use c600_display::plan::{Mode, RevisionRef};
use c600_jumble::state::State;
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context as Engine;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Kernel, ProtectedSet};

const SECRET: [u8; 32] = [11; 32];

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn shared_engine() -> &'static Arc<Engine> {
    static ENGINE: OnceLock<Arc<Engine>> = OnceLock::new();
    ENGINE.get_or_init(|| Arc::new(Engine::load(&repository().join("assets")).expect("the jumbling model loads")))
}

fn shared_kernel() -> &'static Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL.get_or_init(|| Arc::new(Kernel::load(&repository().join("assets")).expect("the 600-cell-Full model loads")))
}

/// The exact jumbling model of `assets/`, loaded once.
pub fn engine() -> &'static Engine {
    shared_engine()
}

/// The 600-cell-Full kernel of `assets/`, loaded once.
pub fn kernel() -> &'static Kernel {
    shared_kernel()
}

/// A W-J fixture, `research/jumbling/fixtures/wj-<name>.json`: J1's journal of a walk under one menu, with J1's
/// references (each record's moved count and the `start`, `mid` and `end` stages).
pub struct Fixture {
    pub name: &'static str,
    pub doc: Value,
    pub twists: Vec<Twist>,
}

impl Fixture {
    pub fn load(name: &'static str) -> Fixture {
        let path = repository().join(format!("research/jumbling/fixtures/wj-{name}.json"));
        let doc = json::parse(&std::fs::read(path).expect("the fixture")).expect("the fixture is JSON");
        let twists = Fixture::records(&doc)
            .iter()
            .map(|record| Twist::from_record(engine(), record).expect("a journal record"))
            .collect();
        Fixture { name, doc, twists }
    }

    fn records(doc: &Value) -> &[Value] {
        doc.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).expect("the journal's records")
    }

    /// The fixture's journal records.
    pub fn journal(&self) -> &[Value] {
        Fixture::records(&self.doc)
    }

    /// The fixture's menu, read again from its record.
    pub fn menu(&self) -> Menu {
        Menu::from_record(engine(), self.doc.get("menu").expect("a menu")).expect("the fixture's menu")
    }

    /// J1's reference block of stage `stage`.
    pub fn stage(&self, stage: &str) -> &Value {
        self.doc.get("stages").and_then(|s| s.get(stage)).expect("a recorded stage")
    }
}

/// Whether the press or step committed.
pub fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

/// A session of the jumbled puzzle through the command layer, Jumble on.
pub struct Session {
    pub layer: CommandLayer,
    shell: Shell,
    menu: Menu,
    /// The pinned menu's items by grip, in item order.
    items: HashMap<usize, Vec<Twist>>,
}

impl Session {
    /// A new session at solved under `fixture`'s menu with `jumble.on` pressed: it must commit.
    pub fn entered(fixture: &Fixture) -> Session {
        let kernel = shared_kernel().clone();
        let (start, none) = (kernel.solved(), ProtectedSet::none());
        let table = Table::h08();
        let store = MemoryStore::default();
        let name = format!("c8-{}", fixture.name);
        let mut layer = CommandLayer::new(kernel, table, Secret::fixed(SECRET), store, &name, start, none);
        layer.pin_menu(shared_engine().clone(), fixture.menu()).expect("the pin");
        let mut session = Session {
            layer,
            shell: Shell::new(Secret::fixed(SECRET), "shell"),
            menu: fixture.menu(),
            items: HashMap::new(),
        };
        let revision = session.layer.revision().to_json();
        let press = session.shell.prepare(&session.layer, JUMBLE_ON_ROW, json::object([("revision", revision)]));
        let receipt = session.shell.send(&mut session.layer, press.expect("the shell prepares the entry"));
        assert!(committed(&receipt), "`{JUMBLE_ON_ROW}` answered {:?}", receipt.status);
        session
    }

    /// The item index of `twist` in the pinned menu at its grip.
    fn item(&mut self, twist: &Twist) -> u64 {
        let menu = &self.menu;
        let items = self
            .items
            .entry(twist.grip)
            .or_insert_with(|| menu.for_cap(engine(), twist.grip).expect("the menu's items"));
        items.iter().position(|item| item.matrix == twist.matrix).expect("an item of the pinned menu") as u64
    }

    /// One `jumble.twist.menu` press of `twist`'s item at its grip, naming the current revision.
    pub fn press(&mut self, twist: &Twist) -> Receipt {
        let pin = MenuPin { set: self.menu.identity.clone(), order: self.menu.order.clone() };
        let binding = Binding {
            request: wire::menu_request(&pin.set, &pin.order, self.item(twist)),
            cap: twist.grip as u32,
            operation: Operation::MenuDirect,
            menu: Some(pin),
        };
        let Value::Object(mut letter) = binding.to_json() else { panic!("a binding is an object") };
        let cap = letter.remove("cap").expect("a binding has a cap");
        let revision = self.layer.revision().to_json();
        let args = json::object([("cap", cap), ("letter", Value::Object(letter)), ("revision", revision)]);
        let press = self.shell.prepare(&self.layer, JUMBLE_TWIST_ROW, args).expect("the shell prepares the press");
        self.shell.send(&mut self.layer, press)
    }

    /// `session.undo` of the action on top of the undo stack, with the layer's hint.
    pub fn undo(&mut self) -> Receipt {
        let hint = self.layer.undo_hint(Direction::Undo).expect("an action to undo");
        let args = json::object([("hint", hint.to_json())]);
        self.shell.invoke(&mut self.layer, "session.undo", args).expect("the shell invokes the undo")
    }

    /// `jumble.off`, naming the current revision: the exit to the full puzzle.
    pub fn off(&mut self) -> Receipt {
        let revision = self.layer.revision().to_json();
        let args = json::object([("revision", revision)]);
        self.shell.invoke(&mut self.layer, JUMBLE_OFF_ROW, args).expect("the shell invokes the exit")
    }

    /// The head's poses: the jumbling engine's state the layer holds.
    pub fn state(&self) -> &State {
        self.layer.jumbled().expect("Jumble is on").state()
    }

    /// The head as KN0.2's revision of the display contract: the layer's revision with the jumbled puzzle's identity
    /// block, which names the pinned menu, and the head event's order as its number.
    pub fn base(&self) -> RevisionRef {
        let r = self.layer.revision();
        assert_eq!(r.mode, JUMBLE_ON_MODE);
        let head = r.head.strip_prefix('e').and_then(|order| order.parse().ok()).expect("a head event e<order>");
        RevisionRef {
            session: r.session.clone(),
            epoch: r.epoch,
            identity: self.layer.jumbled().expect("Jumble is on").identity().clone(),
            mode: Mode::JumbleOn,
            head,
            state: r.state,
            workspace: r.workspace,
            protection: r.protection,
            digest: r.digest.clone(),
        }
    }
}

/// The pieces whose pose differs between `before` and `after`, ascending.
pub fn moved(before: &State, after: &State) -> Vec<u32> {
    let (old, new) = (before.poses(), after.poses());
    let (old_ids, new_ids) = (before.pose_ids(), after.pose_ids());
    (0..old_ids.len())
        .filter(|&piece| old[old_ids[piece] as usize] != new[new_ids[piece] as usize])
        .map(|piece| piece as u32)
        .collect()
}
