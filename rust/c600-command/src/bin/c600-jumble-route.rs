//! The jumbling oracle route (KN5, draft): M5's JSON-lines adapter protocol served through the command layer with
//! Jumble on, so that the differential harness compares J1 with `jumble.on` and `jumble.twist.menu` step by step (LP3
//! plan, PHS.23's "Oracle equality at every committed step", its jumbling half: M5's oracle through the command layer
//! with Jumble on, C8).
//!
//! Usage: `c600-jumble-route <assets directory>`. After loading the 600-cell-Full kernel and the exact jumbling model,
//! the route writes one ready line, then answers one request per line on standard input with one line on standard
//! output, as M5's adapter (`c600-jumble/src/bin/adapter.rs`) does: `{"ok":true,"result":...}` or
//! `{"ok":false,"error":{"kind":"input"|"engine","message":...}}`. A refusal is a result. An error is an input the
//! route does not read, or an answer of the command layer the route does not expect.
//!
//! Requests (`op`):
//! - `start` (`menu`, a menu record): a new session at the solved state with no protected orbit, the menu pinned
//!   ([`CommandLayer::pin_menu`]) and `jumble.on` pressed. Free mode (a null menu) is an input error: the route presses
//!   items of the pinned menu only.
//! - `apply` (`step`, as the harness gives it: a `record`, or a `letter` with `grip` and `item`): one press of
//!   `jumble.twist.menu` that names the item's exact request (KN3.7's binding: the menu request of the item's index at
//!   the grip, pressed directly under the pinned set and order) and the current revision. A record names the item whose
//!   matrix it holds. A record the jumbling engine does not read as a twist, or one that is no item of the menu at its
//!   grip, is `invalid` and reaches no press, since no request names it. A committed press is `admissible`, and
//!   `KN2:input.word_blocked_prefix` is `blocked`. The identity item is an input error: the command layer refuses it
//!   with `KN2:input.recipe_shape` (a draft of `jumble.twist.menu`), where J1 moves its inside pieces onto their own
//!   poses. A `budget` or a protected piece is an input error too: a pose move is classified exactly, with no protected
//!   piece.
//! - `undo` and `redo`: `session.undo` and `session.redo` of the pose move on top of their stacks, with the layer's
//!   hint; the status is `undone` or `redone`. The route undoes no further than the crossing, so Jumble stays on.
//! - `quit`.
//!
//! An engine error ends the session: the layer and the route's stacks may no longer agree, so a new `start` must
//! come first.
//!
//! Each step answers M5's five fields: `status`; `moved`, the count of pieces whose pose changed, and `moved_set`, the
//! harness's hash of their ascending IDs; `digest`, the revision's pose digest, which must be the jumbling engine's
//! digest of the head; and `lattice`, the head's lattice digest. A refused press must leave the revision as it was in
//! every field, and every pose.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use c600_command::{
    CommandLayer, Direction, MemoryStore, Secret, Shell, Status, Table, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW,
};
use c600_jumble::codes::WORD_BLOCKED_PREFIX;
use c600_jumble::observe::moved_set_hash;
use c600_jumble::state::Pose;
use c600_jumble::twist::{Menu, RecordError, Twist};
use c600_jumble::Context;
use c600_kernel::codes::RECIPE_SHAPE;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Kernel, ProtectedSet};

const USAGE: &str = "usage: c600-jumble-route <assets directory>";

enum Fail {
    Input(String),
    Engine(String),
}

fn input(message: impl Into<String>) -> Fail {
    Fail::Input(message.into())
}

fn engine(message: impl Into<String>) -> Fail {
    Fail::Engine(message.into())
}

fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

/// An integer in `0..below`, as M5's adapter reads a letter's grip and item.
fn index(value: Option<&Value>, what: &str, below: usize) -> Result<usize, Fail> {
    match value {
        Some(Value::Int(n)) if (0..below as i64).contains(n) => Ok(*n as usize),
        _ => Err(input(format!("{what} must be an integer in 0..{}", below.saturating_sub(1)))),
    }
}

/// One session: the layer with Jumble on under the pinned menu, its shell, and the pose moves on the undo and redo
/// stacks.
struct Session {
    ctx: Arc<Context>,
    layer: CommandLayer,
    shell: Shell,
    /// The pinned menu, read again from its record: the layer keeps the pinned copy.
    menu: Menu,
    /// The menu's twists by cap, in item order.
    cap_twists: HashMap<usize, Vec<Twist>>,
    undo: usize,
    redo: usize,
}

impl Session {
    /// The pose of every piece at the head.
    fn poses(&self) -> Result<Vec<Pose>, Fail> {
        let state = self.layer.jumbled().ok_or_else(|| engine("Jumble is off"))?.state();
        Ok((0..state.pose_ids().len()).map(|piece| state.pose_of(piece).clone()).collect())
    }

    /// The pieces whose pose differs from `before`, ascending.
    fn moved(&self, before: &[Pose]) -> Result<Vec<u32>, Fail> {
        let state = self.layer.jumbled().ok_or_else(|| engine("Jumble is off"))?.state();
        Ok((0..before.len())
            .filter(|&piece| state.pose_of(piece) != &before[piece])
            .map(|piece| piece as u32)
            .collect())
    }

    fn step_json(&self, status: &str, moved: &[u32]) -> Result<Value, Fail> {
        let derived = self.layer.jumbled().ok_or_else(|| engine("Jumble is off"))?;
        let digest = &self.layer.revision().digest;
        if *digest != derived.digest(&self.ctx) {
            return Err(engine("the revision's digest is not the jumbling engine's digest of the head"));
        }
        Ok(json::object([
            ("status", string(status)),
            ("moved", Value::Int(moved.len() as i64)),
            ("moved_set", string(&moved_set_hash(moved))),
            ("digest", string(digest)),
            ("lattice", string(&derived.state().lattice_digest())),
        ]))
    }

    fn twists(&mut self, grip: usize) -> Result<&[Twist], Fail> {
        if !self.cap_twists.contains_key(&grip) {
            let twists = self.menu.for_cap(&self.ctx, grip).map_err(|error| engine(error.to_string()))?;
            self.cap_twists.insert(grip, twists);
        }
        Ok(&self.cap_twists[&grip])
    }

    /// The grip and item index a harness step names; `Ok(Err(message))` for a step that is `invalid`.
    fn item(&mut self, step: &Value) -> Result<Result<(usize, usize), String>, Fail> {
        match step.get("kind").and_then(Value::as_str) {
            Some("record") => {
                let record = step.get("record").ok_or_else(|| input("a record step needs a record"))?;
                let twist = match Twist::from_record(&self.ctx, record) {
                    Ok(twist) => twist,
                    Err(RecordError::Invalid(message)) => return Ok(Err(message)),
                    Err(RecordError::Unsupported(message)) => return Err(input(message)),
                };
                let name = self.menu.name.clone();
                match self.twists(twist.grip)?.iter().position(|item| item.matrix == twist.matrix) {
                    Some(item) => Ok(Ok((twist.grip, item))),
                    None => Ok(Err(format!("the twist is not in menu {name}"))),
                }
            }
            Some("letter") => {
                let grip = index(step.get("grip"), "letter grip", c600_jumble::model::POLES)?;
                let item = index(step.get("item"), "letter item", self.menu.items.len())?;
                Ok(Ok((grip, item)))
            }
            _ => Err(input("a step kind must be record or letter")),
        }
    }

    /// One `jumble.twist.menu` press of item `item` at cap `grip`.
    fn press(&mut self, grip: usize, item: usize) -> Result<Value, Fail> {
        let (before, head) = (self.poses()?, self.layer.revision().to_json().canonical());
        let pin = MenuPin { set: self.menu.identity.clone(), order: self.menu.order.clone() };
        let binding = Binding {
            request: wire::menu_request(&pin.set, &pin.order, item as u64),
            cap: grip as u32,
            operation: Operation::MenuDirect,
            menu: Some(pin),
        };
        let Value::Object(mut letter) = binding.to_json() else { return Err(engine("a binding is no object")) };
        let cap = letter.remove("cap").ok_or_else(|| engine("a binding has no cap"))?;
        let revision = self.layer.revision().to_json();
        let args = json::object([("cap", cap), ("letter", Value::Object(letter)), ("revision", revision)]);
        let press = self.shell.prepare(&self.layer, JUMBLE_TWIST_ROW, args);
        let press = press.map_err(|error| engine(format!("the shell stopped the press: {}", error.code)))?;
        let receipt = self.shell.send(&mut self.layer, press);
        match &receipt.status {
            Status::Committed(record) if record.post == self.layer.revision().digest => {
                (self.undo, self.redo) = (self.undo + 1, 0);
                let moved = self.moved(&before)?;
                self.step_json("admissible", &moved)
            }
            Status::Refused(error) if error.code == WORD_BLOCKED_PREFIX || error.code == RECIPE_SHAPE => {
                if self.layer.revision().to_json().canonical() != head || !self.moved(&before)?.is_empty() {
                    return Err(engine(format!("a refused press ({}) changed the head", error.code)));
                }
                if error.code == RECIPE_SHAPE {
                    return Err(input(format!("the command layer refuses the identity item ({RECIPE_SHAPE})")));
                }
                self.step_json("blocked", &[])
            }
            other => Err(engine(format!("`{JUMBLE_TWIST_ROW}` answered {other:?}"))),
        }
    }

    /// `session.undo` or `session.redo` of the pose move on top of its stack.
    fn back(&mut self, direction: Direction) -> Result<Value, Fail> {
        let (row, status, depth, none) = match direction {
            Direction::Undo => ("session.undo", "undone", self.undo, "no pose move to undo: the route keeps Jumble on"),
            Direction::Redo => ("session.redo", "redone", self.redo, "no pose move to redo"),
        };
        if depth == 0 {
            return Err(input(none));
        }
        let hint = self.layer.undo_hint(direction).ok_or_else(|| engine(format!("`{row}` has no target")))?;
        let before = self.poses()?;
        let receipt = self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())]));
        let receipt = receipt.map_err(|error| engine(format!("the shell stopped `{row}`: {}", error.code)))?;
        match &receipt.status {
            Status::Committed(record) if record.post == self.layer.revision().digest => {
                (self.undo, self.redo) = match direction {
                    Direction::Undo => (self.undo - 1, self.redo + 1),
                    Direction::Redo => (self.undo + 1, self.redo - 1),
                };
                let moved = self.moved(&before)?;
                self.step_json(status, &moved)
            }
            other => Err(engine(format!("`{row}` answered {other:?}"))),
        }
    }
}

struct Route {
    kernel: Arc<Kernel>,
    ctx: Arc<Context>,
    sessions: u64,
    session: Option<Session>,
}

impl Route {
    fn session(&mut self) -> Result<&mut Session, Fail> {
        self.session.as_mut().ok_or_else(|| input("start must come first"))
    }

    fn start(&mut self, request: &Value) -> Result<Value, Fail> {
        let record = match request.get("menu") {
            None | Some(Value::Null) => {
                return Err(input("the route presses items of a pinned menu: free mode has none"))
            }
            Some(record) => record,
        };
        let pinned = Menu::from_record(&self.ctx, record).map_err(Fail::Input)?;
        let menu = Menu::from_record(&self.ctx, record).map_err(Fail::Input)?;
        self.sessions += 1;
        let (secret, solved) = (Secret::launch(), self.kernel.solved());
        let mut layer = CommandLayer::new(
            self.kernel.clone(),
            Table::h08(),
            secret.clone(),
            MemoryStore::default(),
            &format!("jumble-route-{}", self.sessions),
            solved,
            ProtectedSet::none(),
        );
        layer
            .pin_menu(self.ctx.clone(), pinned)
            .map_err(|error| input(format!("the pin was refused: {}", error.code)))?;
        let mut shell = Shell::new(secret, "jumble-route");
        let revision = layer.revision().to_json();
        let press = shell.prepare(&layer, JUMBLE_ON_ROW, json::object([("revision", revision)]));
        let press = press.map_err(|error| engine(format!("the shell stopped `{JUMBLE_ON_ROW}`: {}", error.code)))?;
        let receipt = shell.send(&mut layer, press);
        if !matches!(receipt.status, Status::Committed(_)) {
            return Err(engine(format!("`{JUMBLE_ON_ROW}` answered {:?}", receipt.status)));
        }
        let ctx = self.ctx.clone();
        let session = Session { ctx, layer, shell, menu, cap_twists: HashMap::new(), undo: 0, redo: 0 };
        self.session.insert(session).step_json("start", &[])
    }

    fn apply(&mut self, request: &Value) -> Result<Value, Fail> {
        let session = self.session()?;
        if !matches!(request.get("budget"), None | Some(Value::Null)) {
            return Err(input("the route reads no budget: a pose move is classified exactly"));
        }
        match request.get("protect") {
            None | Some(Value::Null) => {}
            Some(Value::Array(pieces)) if pieces.is_empty() => {}
            Some(_) => return Err(input("the route protects no piece: no row sets pose-protected pieces")),
        }
        let step = request.get("step").ok_or_else(|| input("apply needs a step"))?;
        match session.item(step)? {
            Ok((grip, item)) => session.press(grip, item),
            Err(message) => {
                let mut result = session.step_json("invalid", &[])?;
                if let Value::Object(fields) = &mut result {
                    fields.insert("message".into(), string(&message));
                }
                Ok(result)
            }
        }
    }

    /// The answer to one request; `None` for `quit`.
    fn handle(&mut self, request: &Value) -> Result<Option<Value>, Fail> {
        let result = match request.get("op").and_then(Value::as_str) {
            Some("quit") => return Ok(None),
            Some("start") => self.start(request)?,
            Some("apply") => self.apply(request)?,
            Some("undo") => self.session()?.back(Direction::Undo)?,
            Some("redo") => self.session()?.back(Direction::Redo)?,
            Some(op) => return Err(input(format!("the route answers start, apply, undo, redo and quit, not {op}"))),
            None => return Err(input("a request needs an op")),
        };
        Ok(Some(result))
    }
}

fn error_line(kind: &str, message: &str) -> String {
    json::object([
        ("ok", Value::Bool(false)),
        ("error", json::object([("kind", string(kind)), ("message", string(message))])),
    ])
    .canonical()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if args.len() != 2 {
        let _ = writeln!(out, "{}", error_line("input", USAGE));
        std::process::exit(2);
    }
    let started = Instant::now();
    let assets = Path::new(&args[1]);
    let loaded = Kernel::load(assets)
        .map_err(|error| format!("model check failed: {error}"))
        .and_then(|kernel| Context::load(assets).map(|ctx| (kernel, ctx)).map_err(|error| error.0));
    let (kernel, ctx) = match loaded {
        Ok(loaded) => loaded,
        Err(message) => {
            let _ = writeln!(out, "{}", error_line("engine", &message));
            std::process::exit(1);
        }
    };
    if kernel.identity().model_id != ctx.model.model_id {
        let _ = writeln!(out, "{}", error_line("engine", "the kernel and the jumbling model differ"));
        std::process::exit(1);
    }
    let ready = json::object([
        ("ok", Value::Bool(true)),
        (
            "result",
            json::object([
                ("ready", Value::Bool(true)),
                ("model_id", string(&ctx.model.model_id)),
                ("load_ms", Value::Int(started.elapsed().as_millis() as i64)),
            ]),
        ),
    ]);
    let _ = writeln!(out, "{}", ready.canonical());
    let _ = out.flush();
    let mut route = Route { kernel: Arc::new(kernel), ctx: Arc::new(ctx), sessions: 0, session: None };
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match json::parse(line.as_bytes()) {
            Err(error) => error_line("input", &format!("request is not JSON: {error}")),
            Ok(request) => match route.handle(&request) {
                Ok(None) => break,
                Ok(Some(result)) => json::object([("ok", Value::Bool(true)), ("result", result)]).canonical(),
                Err(Fail::Input(message)) => error_line("input", &message),
                Err(Fail::Engine(message)) => {
                    route.session = None;
                    error_line("engine", &message)
                }
            },
        };
        if writeln!(out, "{response}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
}
