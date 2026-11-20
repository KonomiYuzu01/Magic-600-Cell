//! The shell's turns (C9, charter E2; a draft): KN5's command layer over M6's journal in a fresh session folder, with
//! LP3's input router in front of it, so that a twist key the window receives becomes a live twist commit.
//!
//! - The session is `JournalStore` on `FsDisk` (the product's disk, with the session lock) in a folder that must be new
//!   or empty, at the solved state, protecting nothing. The layer pins the twist menu the shell names (draft: the I_a
//!   menu of `research/jumbling/fixtures/wj-I_a.json`, the menu KN3's and c600-input's tests pin; G6 owns the menu).
//! - The session turns one puzzle ([`Puzzle`], the shell's M600_PUZZLE). With Jumble off it is 600-cell-Full, and a
//!   twist key commits `twist.apply`. With Jumble on (LX-4's jumble-on half; a draft) it is the jumbled puzzle: a new
//!   session commits `jumble.on` from the solved state at once, under the pinned menu, and a twist key commits
//!   `jumble.twist.menu`, a pose move of the jumbling engine.
//! - Or the session is an existing one (charter E3, a draft): [`Turns::reopen`] reopens a fresh copy of a session
//!   fixture's folder ([`crate::fixture`]) with `CommandLayer::open_with_menu` and the same menu. It must be in the
//!   fixture's puzzle, its head must be the fixture's, and the fixture's committed presses become the session's first
//!   presses, so the control below covers them as well as the run's.
//! - Keys go through `c600_input::Router` on its draft default map (US QWERTY): `KeyJ`, `KeyK` and `KeyL`, with Shift
//!   for the inverse, are its twist keys. With Jumble on the same keys bind `jumble.twist.menu` instead of
//!   `twist.apply` ([`keymap`]): the same item of the pinned menu as the row's `letter`, with the grip's cap and the
//!   layer's revision. c600-input binds no key to that row yet, so this map is the host's draft. The router's grip is
//!   slot 0, cap 0 (draft: `grip.rebind` and `grip.select` are not built, so the cap never changes). Every other key
//!   reaches the router too; the rows it names that the layer does not build are refused there and change nothing. A
//!   key whose row the layer builds and that changes the head (`session.undo` and `session.redo` on Ctrl+Z and Ctrl+Y)
//!   is [`Press::Other`]: the host shows the new head, and the control fails, since its replay knows only twists.
//! - A measured press is a key-down (not a repeat) whose binding is the puzzle's twist row: committed, or refused by
//!   the router or the layer. A repeat of a twist key is dropped by the router (EV4.6), so it is never a press.
//! - The correctness control ([`Turns::check`]) runs once, when the shell closes the session: the live head, the head
//!   a reopen of the journal replays (`CommandLayer::open_with_menu`), and an independent replay of every committed
//!   press from the solved state must agree, and the head the shell last saw presented must be the replay's head after
//!   as many presses as that frame's engine revision counts. With Jumble off the replay is the kernel's: each press's
//!   binding resolved against the menu outside the layer, its retained word derived by the kernel. With Jumble on it
//!   is the jumbling engine's: the solved poses under the pinned menu, then each press's binding resolved the same
//!   way and its twist applied exactly. Engine revision [`ENGINE_REVISION`] is the solved labels the host adopts at
//!   its start in both puzzles; with Jumble on the next one is the solved poses `jumble.on` entered at.
//!
//! Nothing here is Windows/DirectX, input or performance evidence.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_command::{CommandLayer, Secret, Shell, Status, Table, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, TWIST_ROW};
use c600_input::{default_specs, Arg, Chord, Context as Keys, KeyEvent, KeyMap, Layout, Routed, Router, Slot};
use c600_jumble::rotation;
use c600_jumble::state::{State, Status as Admissibility};
use c600_jumble::twist::Menu;
use c600_jumble::Context as Exact;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{Binding, MenuPin};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};
use c600_session::disk::FsDisk;
use c600_session::JournalStore;

use crate::fixture::Fixture;
use crate::ENGINE_REVISION;

/// The layer's and the shell's token secret: a fixed draft value, as LX-2's sessions use.
const SECRET: [u8; 32] = [7; 32];
/// The session's name in its journal.
const SESSION: &str = "c9-shell";
/// The active keyboard layout of the router's draft default map.
const LAYOUT: &str = "us-qwerty";
/// The cap of the router's one grip slot.
const CAP: i64 = 0;
/// The correctness control's name in the run row.
pub const CONTROL: &str = "journal-reopen-and-kernel-replay";

/// The puzzle a session turns: charter E2's `puzzle`, the shell's M600_PUZZLE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Puzzle {
    /// 600-cell-Full with Jumble off: a twist key commits `twist.apply`.
    JumbleOff,
    /// The jumbled puzzle: a new session commits `jumble.on` at its start, and a twist key commits
    /// `jumble.twist.menu`.
    JumbleOn,
}

impl Puzzle {
    /// The shell's names, in the order of its `EventLog.Puzzles`; a puzzle's index is its code at the C boundary.
    pub const NAMES: [&'static str; 2] = ["jumble-off", "jumble-on"];

    /// The puzzle's name in the shell's run row and in a session fixture's record.
    pub fn name(self) -> &'static str {
        Puzzle::NAMES[self as usize]
    }

    /// The puzzle of a name in [`Puzzle::NAMES`].
    pub fn from_name(name: &str) -> Option<Puzzle> {
        [Puzzle::JumbleOff, Puzzle::JumbleOn].into_iter().find(|puzzle| puzzle.name() == name)
    }

    /// The row a twist key commits.
    pub fn twist_row(self) -> &'static str {
        match self {
            Puzzle::JumbleOff => TWIST_ROW,
            Puzzle::JumbleOn => JUMBLE_TWIST_ROW,
        }
    }

    /// The argument of the twist row that carries KN3.7's binding without its cap.
    fn binding_field(self) -> &'static str {
        match self {
            Puzzle::JumbleOff => "rotation",
            Puzzle::JumbleOn => "letter",
        }
    }

    /// The engine revision that shows a new session's head: [`ENGINE_REVISION`], the solved labels, with Jumble off;
    /// the next one, the solved poses `jumble.on` entered at, with Jumble on. Each committed press adds one.
    pub fn opened_at(self) -> u64 {
        match self {
            Puzzle::JumbleOff => ENGINE_REVISION,
            Puzzle::JumbleOn => ENGINE_REVISION + 1,
        }
    }
}

/// The session's head, as the host adopts it.
#[derive(Clone)]
pub enum Head {
    /// Jumble off: 600-cell-Full's labels.
    Retained(Snapshot),
    /// Jumble on: the jumbling engine's poses under the pinned menu, with the exact model that reads them.
    Jumbled(Arc<Exact>, State),
}

impl Head {
    /// KN0.2's digest of the head: the labels' with Jumble off, the poses' with Jumble on.
    pub fn digest(&self) -> String {
        match self {
            Head::Retained(state) => state.digest().to_owned(),
            Head::Jumbled(exact, state) => state.digest(exact),
        }
    }
}

/// What one key-down did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// Not a measured press: no twist key, or a repeat.
    None,
    /// A twist key committed the puzzle's twist row.
    Committed,
    /// A twist key that the router or the layer refused; the head did not change.
    Refused,
    /// Not a measured press, but another row changed the head (the draft map's `session.undo` and `session.redo`).
    /// The control's replay knows only twists, so a run with one fails its control.
    Other,
}

pub struct Turns {
    kernel: Arc<Kernel>,
    exact: Arc<Exact>,
    puzzle: Puzzle,
    /// The menu record, read again for the reopen and the replay (a `Menu` is not `Clone`).
    record: Value,
    folder: PathBuf,
    layer: Option<CommandLayer<JournalStore>>,
    shell: Shell,
    router: Router,
    /// The arguments of every committed press, in order.
    committed: Vec<Value>,
    refused: u64,
    /// Key-downs that changed the head through another row.
    others: u64,
}

fn menu(exact: &Exact, record: &Value) -> Result<Menu, String> {
    Menu::from_record(exact, record).map_err(|error| format!("the twist menu does not read: {error}"))
}

/// The router's draft default map under the active layout for `puzzle`. With Jumble on, every binding of
/// `twist.apply` binds `jumble.twist.menu` instead, on the same chord and destination: its `rotation` becomes the
/// row's `letter`, and the row's `revision` is the layer's revision at the press.
pub fn keymap(puzzle: Puzzle) -> Result<KeyMap, String> {
    let mut specs = default_specs();
    if puzzle == Puzzle::JumbleOn {
        for spec in specs.iter_mut().filter(|spec| spec.command == TWIST_ROW) {
            spec.command = JUMBLE_TWIST_ROW.to_owned();
            for arg in spec.args.iter_mut().filter(|arg| arg.0 == "rotation") {
                arg.0 = "letter".to_owned();
            }
            spec.args.push(("revision".to_owned(), Arg::Revision));
        }
    }
    let declared = [Layout::US, Layout::FRENCH];
    KeyMap::apply(&specs, &declared, LAYOUT).map_err(|findings| format!("the key map: {findings:?}"))
}

/// What a new and a reopened session both read: the menu record, the exact model, the pinned menu, and the router on
/// the puzzle's map with the menu pinned by its two digests.
struct Pinned {
    exact: Arc<Exact>,
    record: Value,
    menu: Menu,
    router: Router,
}

impl Pinned {
    fn read(assets: &Path, menu_file: &Path, puzzle: Puzzle) -> Result<Pinned, String> {
        let text = std::fs::read(menu_file).map_err(|error| format!("the twist menu file: {error}"))?;
        let file = json::parse(&text).map_err(|error| format!("the twist menu file is not JSON: {error:?}"))?;
        let record = file.get("menu").cloned().ok_or("the twist menu file has no `menu` record")?;
        let exact = Arc::new(Exact::load(assets).map_err(|error| format!("the exact model: {error:?}"))?);
        let menu = menu(&exact, &record)?;
        let pin = MenuPin { set: menu.identity.clone(), order: menu.order.clone() };
        let mut keys = Keys::new(vec![Slot { cap: Value::Int(CAP), frame: Value::Null }]);
        keys.menu = Some(pin);
        keys.grip = Some(0);
        Ok(Pinned { exact, record, menu, router: Router::new(keymap(puzzle)?, keys) })
    }
}

/// KN3.7's binding from a committed press's arguments: the fields of `field` (`twist.apply`'s `rotation`,
/// `jumble.twist.menu`'s `letter`) and the row's `cap`, as the layer reads them.
fn binding(args: &Value, field: &str) -> Result<Binding, String> {
    let mut fields = args.get(field).and_then(Value::as_object).cloned().ok_or("a press without its binding")?;
    fields.insert("cap".to_owned(), args.get("cap").cloned().unwrap_or(Value::Null));
    Binding::from_json(&Value::Object(fields)).map_err(|refusal| format!("the press's binding: {refusal:?}"))
}

/// The layer's head digest (KN0.2): the pose digest its revision names with Jumble on, the labels' with Jumble off.
fn head_digest(layer: &CommandLayer<JournalStore>) -> String {
    match layer.jumbled() {
        Some(_) => layer.revision().digest.clone(),
        None => layer.snapshot().digest().to_owned(),
    }
}

impl Turns {
    /// A new session of `puzzle` in `folder` (created if missing; it must hold nothing yet) at the solved state, with
    /// the twist menu of the record file `menu_file` (an object whose `menu` field is the record) pinned in the layer
    /// and, by its two digests, in the router. With Jumble on the session commits `jumble.on` at once, which must
    /// commit.
    pub fn open(
        kernel: Arc<Kernel>,
        assets: &Path,
        folder: &Path,
        menu_file: &Path,
        puzzle: Puzzle,
    ) -> Result<Turns, String> {
        if std::fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_some()) {
            return Err("the session folder is not empty; a run never reuses a session".to_owned());
        }
        let Pinned { exact, record, menu, router } = Pinned::read(assets, menu_file, puzzle)?;
        let disk = FsDisk::new(folder).map_err(|error| format!("the session folder: {error}"))?;
        let store = JournalStore::create(Box::new(disk)).map_err(|error| format!("the journal: {error:?}"))?;
        let mut layer = CommandLayer::new(
            kernel.clone(),
            Table::h08(),
            Secret::fixed(SECRET),
            store,
            SESSION,
            kernel.solved(),
            ProtectedSet::none(),
        );
        layer.pin_menu(exact.clone(), menu).map_err(|error| format!("the menu pin: {error:?}"))?;
        let mut turns = Turns::with(kernel, exact, puzzle, record, folder, layer, router, Vec::new());
        if puzzle == Puzzle::JumbleOn {
            turns.enter()?;
        }
        Ok(turns)
    }

    /// The existing session that `fixture` describes, reopened in `folder` (a fresh copy of the fixture's session
    /// folder) with the twist menu of `menu_file` pinned as [`Turns::open`] pins it. The reopened session must be in
    /// the fixture's puzzle and its head's digest must be the fixture's; the fixture's committed presses become this
    /// session's first presses.
    pub fn reopen(
        kernel: Arc<Kernel>,
        assets: &Path,
        folder: &Path,
        menu_file: &Path,
        fixture: &Fixture,
    ) -> Result<Turns, String> {
        if !folder.is_dir() {
            return Err("the session folder does not exist; a reopen needs a copy of the fixture's session".to_owned());
        }
        let puzzle = fixture.puzzle;
        let Pinned { exact, record, menu, router } = Pinned::read(assets, menu_file, puzzle)?;
        let disk = FsDisk::new(folder).map_err(|error| format!("the session folder: {error}"))?;
        let store = JournalStore::open(Box::new(disk)).map_err(|error| format!("the journal: {error:?}"))?;
        let layer = CommandLayer::open_with_menu(
            kernel.clone(),
            Table::h08(),
            Secret::fixed(SECRET),
            store,
            exact.clone(),
            menu,
        )
        .map_err(|error| format!("the reopen: {error:?}"))?;
        if layer.jumbled().is_some() != (puzzle == Puzzle::JumbleOn) {
            return Err(format!("the reopened session is not in the fixture's puzzle, {}", puzzle.name()));
        }
        let digest = head_digest(&layer);
        if digest != fixture.digest {
            return Err(format!("the reopened session's state {digest} is not the fixture's {}", fixture.digest));
        }
        Ok(Turns::with(kernel, exact, puzzle, record, folder, layer, router, fixture.committed.clone()))
    }

    #[allow(clippy::too_many_arguments)]
    fn with(
        kernel: Arc<Kernel>,
        exact: Arc<Exact>,
        puzzle: Puzzle,
        record: Value,
        folder: &Path,
        layer: CommandLayer<JournalStore>,
        router: Router,
        committed: Vec<Value>,
    ) -> Turns {
        Turns {
            kernel,
            exact,
            puzzle,
            record,
            folder: folder.to_path_buf(),
            layer: Some(layer),
            shell: Shell::new(Secret::fixed(SECRET), "shell"),
            router,
            committed,
            refused: 0,
            others: 0,
        }
    }

    /// `jumble.on` at the head, naming the layer's revision: the entry from the solved witness.
    fn enter(&mut self) -> Result<(), String> {
        let layer = self.layer.as_mut().ok_or("the session is closed")?;
        let args = json::object([("revision", layer.revision().to_json())]);
        let receipt = self
            .shell
            .invoke(layer, JUMBLE_ON_ROW, args)
            .map_err(|error| format!("`{JUMBLE_ON_ROW}` was not sent: {}: {}", error.code, error.detail))?;
        match &receipt.status {
            Status::Committed(_) if layer.jumbled().is_some() => Ok(()),
            Status::Refused(error) | Status::Failed(error) => {
                Err(format!("`{JUMBLE_ON_ROW}` did not commit: {}: {}", error.code, error.detail))
            }
            status => Err(format!("`{JUMBLE_ON_ROW}` did not commit: the layer answered {}", status.name())),
        }
    }

    fn layer(&mut self) -> Result<&mut CommandLayer<JournalStore>, String> {
        self.layer.as_mut().ok_or_else(|| "the session is closed".to_owned())
    }

    /// The session's puzzle.
    pub fn puzzle(&self) -> Puzzle {
        self.puzzle
    }

    /// Routes one key-down, `chord` in c600-input's form (`Shift+KeyJ`); `repeat` is the platform's auto-repeat flag.
    pub fn key_down(&mut self, chord: &str, repeat: bool) -> Result<Press, String> {
        let chord = Chord::parse(chord).ok_or_else(|| format!("`{chord}` is not a chord"))?;
        let twist_row = self.puzzle.twist_row();
        let layer = self.layer.as_mut().ok_or("the session is closed")?;
        let before = head_digest(layer);
        let routed = self.router.key(&mut self.shell, layer, KeyEvent::Down { chord, repeat, text: None });
        let changed = head_digest(layer) != before;
        let press = match &routed {
            Routed::Dispatched(dispatch) if dispatch.invocation.row == twist_row => {
                if matches!(dispatch.receipt.status, Status::Committed(_)) {
                    self.committed.push(dispatch.invocation.args.clone());
                    Press::Committed
                } else {
                    Press::Refused
                }
            }
            Routed::Refused { row, .. } if row == twist_row => Press::Refused,
            _ => Press::None,
        };
        let press = if changed && press != Press::Committed { Press::Other } else { press };
        match press {
            Press::Refused => self.refused += 1,
            Press::Other => self.others += 1,
            Press::None | Press::Committed => {}
        }
        Ok(press)
    }

    /// Routes a key-up.
    pub fn key_up(&mut self, chord: &str) -> Result<(), String> {
        let chord = Chord::parse(chord).ok_or_else(|| format!("`{chord}` is not a chord"))?;
        let layer = self.layer.as_mut().ok_or("the session is closed")?;
        self.router.key(&mut self.shell, layer, KeyEvent::Up(chord.key));
        Ok(())
    }

    /// The window lost the keyboard focus: the router releases its held keys.
    pub fn focus_lost(&mut self) -> Result<(), String> {
        let layer = self.layer.as_mut().ok_or("the session is closed")?;
        self.router.key(&mut self.shell, layer, KeyEvent::FocusLost);
        Ok(())
    }

    /// The layer's retained state: with Jumble on, the one `jumble.on` entered from.
    pub fn state(&mut self) -> Result<Snapshot, String> {
        Ok(self.layer()?.snapshot().clone())
    }

    /// The layer's head: the labels with Jumble off, the jumbling engine's poses with Jumble on.
    pub fn head(&mut self) -> Result<Head, String> {
        let exact = self.exact.clone();
        let layer = self.layer()?;
        Ok(match layer.jumbled() {
            Some(jumbled) => Head::Jumbled(exact, jumbled.state().clone()),
            None => Head::Retained(layer.snapshot().clone()),
        })
    }

    /// The committed presses, a reopened fixture's first, and the refused ones.
    pub fn presses(&self) -> (u64, u64) {
        (self.committed.len() as u64, self.refused)
    }

    /// The arguments of every committed press, in order, a reopened fixture's first.
    pub fn committed(&self) -> &[Value] {
        &self.committed
    }

    /// The correctness control, once: closes the layer (its lock is released), reopens the journal, replays every
    /// committed press outside the layer, and compares. `shown` is the engine revision of the newest frame the shell
    /// saw presented and its head's digest as the host adopted it (revision 1 is the solved labels; with Jumble on
    /// revision 2 is the solved poses; each committed press adds one). The answer is the run row's
    /// `correctness_detail` object.
    pub fn check(&mut self, shown: Option<(u64, String)>) -> Value {
        let live = self.layer.take().map(|layer| head_digest(&layer));
        let reopened = self.reopened_digest();
        let replayed = match self.puzzle {
            Puzzle::JumbleOff => self.replay(),
            Puzzle::JumbleOn => self.replay_poses(),
        };
        let (shown_revision, shown_digest) = shown.map_or((Value::Null, Value::Null), |(revision, digest)| {
            (Value::Int(revision as i64), Value::String(digest))
        });
        let expected = match (&replayed, &shown_revision) {
            (Ok(prefixes), Value::Int(revision)) if *revision >= 1 => prefixes.get(*revision as usize - 1).cloned(),
            _ => None,
        };
        let last = replayed.as_ref().ok().and_then(|prefixes| prefixes.last().cloned());
        let pass = live.is_some()
            && self.others == 0
            && reopened.as_ref().ok() == live.as_ref()
            && last == live
            && expected.is_some()
            && shown_digest.as_str() == expected.as_deref();
        let text = |value: Option<String>| value.map_or(Value::Null, Value::String);
        fn error<T>(result: &Result<T, String>) -> Value {
            result.as_ref().err().map_or(Value::Null, |e| Value::String(e.clone()))
        }
        json::object([
            ("control", Value::String(CONTROL.to_owned())),
            ("pass", Value::Bool(pass)),
            ("committed", Value::Int(self.committed.len() as i64)),
            ("refused", Value::Int(self.refused as i64)),
            ("other_changes", Value::Int(self.others as i64)),
            ("live", text(live)),
            ("reopened", text(reopened.as_ref().ok().cloned())),
            ("reopen_error", error(&reopened)),
            ("replayed", text(last)),
            ("replay_error", error(&replayed)),
            ("shown_revision", shown_revision),
            ("shown", shown_digest),
            ("expected_shown", text(expected)),
        ])
    }

    /// The digest of the head that a reopen of the closed session's journal replays.
    fn reopened_digest(&self) -> Result<String, String> {
        let disk = FsDisk::new(&self.folder).map_err(|error| format!("the session folder: {error}"))?;
        let store = JournalStore::open(Box::new(disk)).map_err(|error| format!("the journal: {error:?}"))?;
        let table = Table::h08();
        let pinned = menu(&self.exact, &self.record)?;
        let layer = CommandLayer::open_with_menu(
            self.kernel.clone(),
            table,
            Secret::fixed(SECRET),
            store,
            self.exact.clone(),
            pinned,
        )
        .map_err(|error| format!("the reopen: {error:?}"))?;
        Ok(head_digest(&layer))
    }

    /// Jumble off: the digest of the solved state, then of the state after each committed press, by the kernel alone.
    fn replay(&self) -> Result<Vec<String>, String> {
        let pinned = menu(&self.exact, &self.record)?;
        let mut state = self.kernel.solved();
        let mut digests = vec![state.digest().to_owned()];
        for args in &self.committed {
            let resolution = rotation::resolve(&self.exact, Some(&pinned), &binding(args, "rotation")?)
                .map_err(|refusal| format!("a press does not resolve: {refusal:?}"))?;
            let word = rotation::retained_word(&self.exact, &resolution)
                .map_err(|refusal| format!("a press has no retained word: {refusal:?}"))?;
            let recipe = self.kernel.word(&word).map_err(|refusal| format!("a retained word: {refusal:?}"))?;
            state = match self.kernel.derive(&state, &recipe, &ProtectedSet::none()) {
                Ok(Derivation::Derived(derived)) => derived.state,
                Ok(Derivation::Refused(refused)) => return Err(format!("the kernel refused a press: {refused:?}")),
                Err(error) => return Err(format!("the kernel failed: {error:?}")),
            };
            digests.push(state.digest().to_owned());
        }
        Ok(digests)
    }

    /// Jumble on: the digest of the solved labels the host adopts first, then of the solved poses under the pinned
    /// menu, then of the poses after each committed press, by the jumbling engine alone: each press's binding
    /// resolved against the menu outside the layer and its twist applied exactly, which must be admissible.
    fn replay_poses(&self) -> Result<Vec<String>, String> {
        let pinned = Arc::new(menu(&self.exact, &self.record)?);
        let mut state = State::new(Some(pinned.clone()));
        let mut digests = vec![self.kernel.solved().digest().to_owned(), state.digest(&self.exact)];
        let field = self.puzzle.binding_field();
        for args in &self.committed {
            let resolution = rotation::resolve(&self.exact, Some(&*pinned), &binding(args, field)?)
                .map_err(|refusal| format!("a press does not resolve: {refusal:?}"))?;
            let outcome = state
                .apply(&self.exact, &resolution.twist, None, &[])
                .map_err(|error| format!("the jumbling engine failed: {error:?}"))?;
            if outcome.status != Admissibility::Admissible {
                return Err(format!("a press is {} in the replay", outcome.status.as_str()));
            }
            digests.push(state.digest(&self.exact));
        }
        Ok(digests)
    }
}
