//! The jumbled puzzle through the command layer (LP2's pull requests 212, 217, 218 and 220), shared by the tests that
//! take up Jumble on: a 600-cell-Full session at solved with no protected orbit, a W-J fixture's menu pinned
//! (`CommandLayer::pin_menu`) and `jumble.on` pressed. Every move is one `jumble.twist.menu` press of the menu item whose
//! matrix the fixture's record holds, at its grip, naming the current revision, as LP2's route
//! (`c600-command/src/bin/c600-jumble-route.rs`) presses it; every undo or redo is one `session.undo` or `session.redo`
//! with the layer's hint.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::JUMBLE_ON_MODE;
use c600_command::{
    CommandLayer, Completion, CompletionKind, Direction, Invocation, MemoryStore, Profile, Receipt, Secret, Shell,
    Status, Table, JUMBLE_OFF_ROW, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, POLICY_ROW, REPAIR_LIST_ROW,
};
use c600_display::plan::{Mode, RevisionRef};
use c600_jumble::state::State;
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context as Engine;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Kernel, ProtectionMode};

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

/// Whether a protection or policy change completed.
pub fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
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
        Session::entered_protecting(fixture, &[])
    }

    /// [`Session::entered`] with the 600-cell-Full orbits `orbits` protected from the session's start. The entry has no
    /// protection gate (H-08 gives `jumble.on` `model_match` only), so it commits.
    pub fn entered_protecting(fixture: &Fixture, orbits: &[i64]) -> Session {
        let kernel = shared_kernel().clone();
        let start = kernel.solved();
        let protected = kernel.protected_set(orbits).expect("orbits of the model");
        let table = Table::h08();
        let store = MemoryStore::default();
        let name = format!("c8-{}", fixture.name);
        let mut layer = CommandLayer::new(kernel, table, Secret::fixed(SECRET), store, &name, start, protected);
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

    /// [`Session::entered`] in a solving session with LP2's drafted rows admitted (`Table::h08().with_drafts()`,
    /// `Profile::Solving`), as `c600-command/tests/repair_list.rs` builds it, so that `jumble.repair.list` is a row.
    pub fn entered_solving(fixture: &Fixture) -> Session {
        let kernel = shared_kernel().clone();
        let (start, protected) = (kernel.solved(), kernel.protected_set(&[]).expect("no orbit"));
        let table = Table::h08().with_drafts();
        let store = MemoryStore::default();
        let name = format!("c8-{}-solving", fixture.name);
        let secret = Secret::fixed(SECRET);
        let mut layer =
            CommandLayer::new_with_profile(kernel, table, secret, store, &name, start, protected, Profile::Solving);
        layer.pin_menu(shared_engine().clone(), fixture.menu()).expect("the pin");
        let mut session = Session {
            layer,
            shell: Shell::new(Secret::fixed(SECRET), "shell"),
            menu: fixture.menu(),
            items: HashMap::new(),
        };
        assert!(committed(&session.on()), "`{JUMBLE_ON_ROW}` commits");
        session
    }

    /// The solver's `jumble.repair.list` at `cap`, naming the current revision as its snapshot, run by
    /// `CommandLayer::run_jobs`: the job and the list the layer keeps for it. The invocation must start one job, and
    /// the job must complete as a query.
    pub fn list(&mut self, cap: i64) -> (u64, Value) {
        let snapshot = self.layer.revision().to_json();
        let args = json::object([("snapshot", snapshot), ("cap", Value::Int(cap)), ("max_chunk", Value::Int(8))]);
        let receipt = self.invoke(REPAIR_LIST_ROW, args);
        let Status::Running(job) = receipt.status else { panic!("the list is not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1, "one job runs");
        let settled = self.layer.outcome(&receipt.command, &receipt.request).expect("a settled receipt");
        let kind = match &settled.status {
            Status::Completed(Completion { kind, .. }) => Some(kind),
            _ => None,
        };
        assert_eq!(kind, Some(&CompletionKind::Query), "the list completes as a query: {:?}", settled.status);
        let value = self.layer.repair_list(job).cloned().expect("the kept list");
        (job, value)
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

    /// `twist` as a letter of the pinned menu: its grip and its item's index there.
    pub fn letter(&mut self, twist: &Twist) -> [u32; 2] {
        [twist.grip as u32, self.item(twist) as u32]
    }

    /// The direct binding of `twist`'s item at its grip under the pinned menu (KN3.7).
    pub fn binding(&mut self, twist: &Twist) -> Binding {
        let pin = MenuPin { set: self.menu.identity.clone(), order: self.menu.order.clone() };
        Binding {
            request: wire::menu_request(&pin.set, &pin.order, self.item(twist)),
            cap: twist.grip as u32,
            operation: Operation::MenuDirect,
            menu: Some(pin),
        }
    }

    /// The `jumble.twist.menu` press of `binding`, naming the current revision, prepared with its token and not sent.
    pub fn prepare(&mut self, binding: &Binding) -> Invocation {
        let Value::Object(mut letter) = binding.to_json() else { panic!("a binding is an object") };
        let cap = letter.remove("cap").expect("a binding has a cap");
        let revision = self.layer.revision().to_json();
        let args = json::object([("cap", cap), ("letter", Value::Object(letter)), ("revision", revision)]);
        self.shell.prepare(&self.layer, JUMBLE_TWIST_ROW, args).expect("the shell prepares the press")
    }

    /// Sends a prepared invocation through the shell.
    pub fn send(&mut self, invocation: Invocation) -> Receipt {
        self.shell.send(&mut self.layer, invocation)
    }

    /// One `jumble.twist.menu` press of `twist`'s item at its grip, naming the current revision.
    pub fn press(&mut self, twist: &Twist) -> Receipt {
        let binding = self.binding(twist);
        let press = self.prepare(&binding);
        self.send(press)
    }

    /// Row `row` with `args` through the shell, which mints its token.
    pub fn invoke(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).expect("the shell invokes the row")
    }

    /// `session.undo` or `session.redo` of the action on top of its stack, with the layer's hint.
    pub fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an action to undo or redo");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.invoke(row, json::object([("hint", hint.to_json())]))
    }

    /// `session.undo` of the action on top of the undo stack, with the layer's hint.
    pub fn undo(&mut self) -> Receipt {
        self.step(Direction::Undo)
    }

    /// `session.redo` of the action on top of the redo stack, with the layer's hint.
    pub fn redo(&mut self) -> Receipt {
        self.step(Direction::Redo)
    }

    /// `jumble.on`, naming the current revision.
    pub fn on(&mut self) -> Receipt {
        let revision = self.layer.revision().to_json();
        self.invoke(JUMBLE_ON_ROW, json::object([("revision", revision)]))
    }

    /// `protection.orbit.set` with `orbits`; `confirm` confirms a release (LI.26).
    pub fn protect(&mut self, orbits: &[i64], confirm: bool) -> Receipt {
        let orbits = Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect());
        let args = if confirm {
            json::object([("confirm", Value::Bool(true)), ("orbits", orbits)])
        } else {
            json::object([("orbits", orbits)])
        };
        self.invoke("protection.orbit.set", args)
    }

    /// The protection policy `policy`, `"net"` or `"strict"`, with its confirmation.
    pub fn set_policy(&mut self, policy: &str) -> Receipt {
        let args = json::object([("confirm", Value::Bool(true)), ("policy", Value::String(policy.to_owned()))]);
        self.invoke(POLICY_ROW, args)
    }

    /// What no refusal may change: the revision's digest and its whole form, the journal's length, the policy, the
    /// poses (with Jumble on) and the retained state, which with Jumble on is not the revision's digest.
    pub fn fixed(&self) -> (String, String, usize, ProtectionMode, Option<String>, String) {
        let r = self.layer.revision();
        let poses = self.layer.jumbled().map(|jumbled| jumbled.digest(engine()));
        let retained = self.layer.snapshot().digest().to_owned();
        (r.digest.clone(), format!("{r:?}"), self.layer.store().log.len(), self.layer.policy(), poses, retained)
    }

    /// `jumble.off`, naming the current revision: the exit to the full puzzle.
    pub fn off(&mut self) -> Receipt {
        let revision = self.layer.revision().to_json();
        self.invoke(JUMBLE_OFF_ROW, json::object([("revision", revision)]))
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
