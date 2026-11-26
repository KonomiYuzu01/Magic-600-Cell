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
//!   key whose row the layer builds and that changes the head (`session.undo` and `session.redo` on Ctrl+KeyZ and
//!   Ctrl+KeyY) is [`Press::Other`]: the host shows the new head, and the control fails, since its replay knows only
//!   twists.
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
//! - LP4-3's write-check-run flow (a draft; [`Turns::register_row`] and the calls after it): a checked and activated
//!   user module's entry is registered as a language overlay row, a `query` row with its label and no grant
//!   (`CommandLayer::register_overlay_with`), with its key bound in the one checked key map the router and the shell
//!   share (LP4-8), which the palette reads (`Shell::read_palette`). The solver invokes it from the palette
//!   ([`Turns::invoke`], the shell's `Shell::invoke`) or by its key (the router), and the host takes its program job
//!   ([`Turns::take_job`]) and reports it ([`Turns::finish_job`]); the run itself is the editor library's
//!   (`renderer/bridge/editor`), which binds the job to the module's active version. The run serves no row, so it prepares no preview: the solver puts its word
//!   in the editor's draft ([`Turns::set_draft`]) and previews it with `operation.preview` (Ctrl+Enter), which the host
//!   builds ([`Turns::run_jobs`]) and the view shows; the confirmed frames that show it are reported to the layer
//!   ([`Turns::present`]), and the solver commits it with `operation.execute` (Ctrl+Shift+Enter), whose token the
//!   shell mints from the press, and undoes it with `session.undo` (Ctrl+KeyZ). A text field's focus
//!   ([`Turns::set_focus`]) keeps the keys it takes, a commit chord among them (DT-M1 C4).
//!
//! - ADR-D02 B5 (a draft): [`Turns::twist_letter`] commits any item of the pinned menu at any cap as
//!   `jumble.twist.menu` through the shell's route, with the arguments a twist key gives, so that a session fixture's
//!   word can reach a D02 sample state; [`Turns::state_digests`] gives the head's digest and the two pose-array
//!   digests by which the shell binds that sample to the session.
//!
//! Nothing here is Windows/DirectX, input or performance evidence.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_command::{
    CommandLayer, Error as LayerError, Kind, Overlay, PresentedFrame, PreviewState, ProgramJob, Receipt, Registration,
    Secret, Shell, Status, Table, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, TWIST_ROW,
};
use c600_digest::sha256_hex;
use c600_input::{
    default_specs, Arg, BindingSpec, Chord, ChordIntent, Context as Keys, Focus, KeyEvent, KeyMap, Layout, Routed,
    Router, RoutingClass, Slot,
};
use c600_jumble::rotation;
use c600_jumble::state::{State, Status as Admissibility};
use c600_jumble::twist::Menu;
use c600_jumble::Context as Exact;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{menu_request, Binding, MenuPin};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};
use c600_session::disk::FsDisk;
use c600_session::JournalStore;

use crate::fixture::Fixture;
use crate::preload::load_exact;
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
    /// The pinned menu's two digests, which a menu letter names ([`Turns::twist_letter`]).
    pin: MenuPin,
    /// The router's specs: the puzzle's draft defaults and the keys of the rows registered since ([`specs`]).
    specs: Vec<BindingSpec>,
    /// The arguments of every committed press, in order.
    committed: Vec<Value>,
    refused: u64,
    /// Key-downs that changed the head through another row.
    others: u64,
}

fn menu(exact: &Exact, record: &Value) -> Result<Menu, String> {
    Menu::from_record(exact, record).map_err(|error| format!("the twist menu does not read: {error}"))
}

/// The router's draft default specs for `puzzle`. With Jumble on, every binding of `twist.apply` binds
/// `jumble.twist.menu` instead, on the same chord and destination: its `rotation` becomes the row's `letter`, and the
/// row's `revision` is the layer's revision at the press.
pub fn specs(puzzle: Puzzle) -> Vec<BindingSpec> {
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
    specs
}

/// The layouts the router's map declares.
const DECLARED: [Layout; 2] = [Layout::US, Layout::FRENCH];

/// The router's draft default map under the active layout for `puzzle` ([`specs`]).
pub fn keymap(puzzle: Puzzle) -> Result<KeyMap, String> {
    KeyMap::apply(&specs(puzzle), &DECLARED, LAYOUT).map_err(|findings| format!("the key map: {findings:?}"))
}

/// The engine's pending preview as the host shows it: its ID, its literal word's digest, its certified effect's digest
/// and the layer's state revision it was built on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub id: u64,
    pub literal: String,
    pub effect: String,
    pub state: u64,
}

/// The row ID of a user module's entry in the palette: `tool.<module>.<entry>` (a draft).
pub fn row_id(module: &str, entry: &str) -> String {
    format!("tool.{module}.{entry}")
}

/// What a new and a reopened session both read: the menu record, the exact model, the pinned menu, and the router on
/// the puzzle's map with the menu pinned by its two digests.
struct Pinned {
    exact: Arc<Exact>,
    record: Value,
    menu: Menu,
    router: Router,
    /// The menu's two digests, which a menu letter names ([`Turns::twist_letter`]).
    pin: MenuPin,
}

impl Pinned {
    /// `exact` gives the exact model, [`load_exact`] or a load the host started earlier (the `preload` module).
    fn read(
        exact: impl FnOnce() -> Result<Arc<Exact>, String>,
        menu_file: &Path,
        puzzle: Puzzle,
    ) -> Result<Pinned, String> {
        let text = std::fs::read(menu_file).map_err(|error| format!("the twist menu file: {error}"))?;
        let file = json::parse(&text).map_err(|error| format!("the twist menu file is not JSON: {error:?}"))?;
        let record = file.get("menu").cloned().ok_or("the twist menu file has no `menu` record")?;
        let exact = exact()?;
        let menu = menu(&exact, &record)?;
        let pin = MenuPin { set: menu.identity.clone(), order: menu.order.clone() };
        let mut keys = Keys::new(vec![Slot { cap: Value::Int(CAP), frame: Value::Null }]);
        keys.menu = Some(pin.clone());
        keys.grip = Some(0);
        Ok(Pinned { exact, record, menu, router: Router::new(keymap(puzzle)?, keys), pin })
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
        Turns::open_with(kernel, || load_exact(assets), folder, menu_file, puzzle)
    }

    /// [`Turns::open`] with the exact model that `exact` gives, called where [`Turns::open`] loads it.
    pub fn open_with(
        kernel: Arc<Kernel>,
        exact: impl FnOnce() -> Result<Arc<Exact>, String>,
        folder: &Path,
        menu_file: &Path,
        puzzle: Puzzle,
    ) -> Result<Turns, String> {
        if std::fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_some()) {
            return Err("the session folder is not empty; a run never reuses a session".to_owned());
        }
        let Pinned { exact, record, menu, router, pin } = Pinned::read(exact, menu_file, puzzle)?;
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
        let mut turns = Turns::with(kernel, exact, puzzle, record, folder, layer, router, pin, Vec::new());
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
        Turns::reopen_with(kernel, || load_exact(assets), folder, menu_file, fixture)
    }

    /// [`Turns::reopen`] with the exact model that `exact` gives, called where [`Turns::reopen`] loads it.
    pub fn reopen_with(
        kernel: Arc<Kernel>,
        exact: impl FnOnce() -> Result<Arc<Exact>, String>,
        folder: &Path,
        menu_file: &Path,
        fixture: &Fixture,
    ) -> Result<Turns, String> {
        if !folder.is_dir() {
            return Err("the session folder does not exist; a reopen needs a copy of the fixture's session".to_owned());
        }
        let puzzle = fixture.puzzle;
        let Pinned { exact, record, menu, router, pin } = Pinned::read(exact, menu_file, puzzle)?;
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
        Ok(Turns::with(kernel, exact, puzzle, record, folder, layer, router, pin, fixture.committed.clone()))
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
        pin: MenuPin,
        committed: Vec<Value>,
    ) -> Turns {
        // One key map: the shell's palette names the keys the router binds (LP4-8).
        let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
        shell.apply_keymap(router.keymap().clone());
        Turns {
            kernel,
            exact,
            puzzle,
            record,
            folder: folder.to_path_buf(),
            layer: Some(layer),
            shell,
            router,
            pin,
            specs: specs(puzzle),
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

    /// Commits item `item` of the pinned menu at cap `cap` as `jumble.twist.menu` through the shell's route
    /// ([`Turns::invoke`]), the arguments a twist key gives (the router's `menu_item` letter, the cap and the layer's
    /// revision) at any cap and item: a letter of a session fixture's word (ADR-D02 B5). It must commit; its
    /// arguments join the committed presses, so the control replays it. Jumble on only.
    pub fn twist_letter(&mut self, cap: u32, item: u64) -> Result<(), String> {
        if self.puzzle != Puzzle::JumbleOn {
            return Err("a menu letter needs the jumbled puzzle".to_owned());
        }
        let text = |s: &str| Value::String(s.to_owned());
        let request = String::from_utf8_lossy(&menu_request(&self.pin.set, &self.pin.order, item)).into_owned();
        let letter = json::object([
            ("request", Value::String(request)),
            ("operation", json::object([("kind", text("menu_direct"))])),
            ("menu", json::object([("set", text(&self.pin.set)), ("order", text(&self.pin.order))])),
        ]);
        let revision = self.layer()?.revision().to_json();
        let args = json::object([("cap", Value::Int(i64::from(cap))), ("letter", letter), ("revision", revision)]);
        let receipt = self.invoke(JUMBLE_TWIST_ROW, args.clone())?;
        match &receipt.status {
            Status::Committed(_) => {
                self.committed.push(args);
                Ok(())
            }
            Status::Refused(error) | Status::Failed(error) => {
                Err(format!("the letter [{cap}, {item}] did not commit: {}: {}", error.code, error.detail))
            }
            status => Err(format!("the letter [{cap}, {item}] did not commit: the layer answered {}", status.name())),
        }
    }

    /// The head's digests as ADR-D02's sample states name them (B5), by which a sample binds to the session: KN0.2's
    /// digest, and with Jumble on the SHA-256 of the pose ids (u32 little-endian per piece, the jumbling engine's
    /// record) and of the lattice flags (one byte per piece, 1 when its pose is in K⁺), and the count of pieces whose
    /// pose is not in K⁺, from which the shell reads the exact lattice flag (B6). With Jumble off the three are `null`.
    /// The answer is `{"digest", "pose_ids_sha256", "pose_lattice_sha256", "off_lattice"}`.
    pub fn state_digests(&mut self) -> Result<Value, String> {
        let layer = self.layer()?;
        let digest = head_digest(layer);
        let (ids, lattice, off) = match layer.jumbled() {
            Some(jumbled) => {
                let state = jumbled.state();
                let raw: Vec<u8> = state.pose_ids().iter().flat_map(|id| id.to_le_bytes()).collect();
                let poses = state.poses();
                let flags: Vec<u8> =
                    state.pose_ids().iter().map(|&id| u8::from(poses[id as usize].kidx().is_some())).collect();
                let off = flags.iter().filter(|&&flag| flag == 0).count() as i64;
                (Value::String(sha256_hex(&raw)), Value::String(sha256_hex(&flags)), Value::Int(off))
            }
            None => (Value::Null, Value::Null, Value::Null),
        };
        Ok(json::object([
            ("digest", Value::String(digest)),
            ("pose_ids_sha256", ids),
            ("pose_lattice_sha256", lattice),
            ("off_lattice", off),
        ]))
    }

    // ----- LP4-3's write-check-run flow (a draft)

    /// Registers the entry `entry` of user module `module` as a language overlay row (KN5.3, EV4.1): row [`row_id`], a
    /// `query` row with no argument, with `label` and no grant (`Registration::labelled`). With `chord` (the checked
    /// key map's form, such as `Ctrl+Shift+KeyR`), the one key map the router and the shell share binds the row in the
    /// global routing class, so the palette shows the key. A chord that is not one, that the map binds already or that
    /// the map refuses under a declared layout (EV4.7), and a row the layer refuses (`KN5:input.overlay_row_conflict`
    /// for a second registration), change nothing. The registration is not journaled: a reopen registers again. The
    /// answer is the row's ID.
    pub fn register_row(
        &mut self,
        module: &str,
        entry: &str,
        label: &str,
        chord: Option<&str>,
    ) -> Result<String, String> {
        let row = row_id(module, entry);
        let mut specs = self.specs.clone();
        let mut bound = None;
        if let Some(chord) = chord {
            if ChordIntent::parse(chord).is_none() {
                return Err(format!("`{chord}` is not a chord"));
            }
            specs.push(BindingSpec::new(&[chord], RoutingClass::Global, &row, vec![]));
            let map =
                KeyMap::apply(&specs, &DECLARED, LAYOUT).map_err(|findings| format!("the key map: {findings:?}"))?;
            bound = Some(map);
        }
        let overlay = Overlay { module: module.to_owned(), entry: entry.to_owned() };
        let layer = self.layer.as_mut().ok_or("the session is closed")?;
        layer
            .register_overlay_with(&row, Kind::Query, &[], overlay, Registration::labelled(label))
            .map_err(|error| format!("{}: {}", error.code, error.detail))?;
        if let Some(map) = bound {
            self.router.apply(&specs, &DECLARED, LAYOUT).map_err(|findings| format!("the key map: {findings:?}"))?;
            self.shell.apply_keymap(map);
            self.specs = specs;
        }
        Ok(row)
    }

    /// Sets the focus the router routes keys by: a text field (`true`) keeps the keys it takes, its typing, editing
    /// and office chords and a key bound to a commit row (DT-M1 C4), and the global map gets the rest; the puzzle
    /// (`false`) keeps none. Held keys are released, as on a focus loss.
    pub fn set_focus(&mut self, text: bool) {
        self.router.set_focus(if text { Focus::Text } else { Focus::Puzzle });
    }

    /// The palette (EV4.2, KN5.6) as JSON: one entry per row of the table, in its order, with its label, kind, whether
    /// a language module registered it, the key sequences the shell's keymap binds to it and its availability, `null`
    /// or the typed reason it is unavailable. The shell reads it once per revision and generation; reading it starts
    /// no program and changes nothing.
    pub fn palette(&mut self) -> Result<Value, String> {
        let layer = self.layer.as_ref().ok_or("the session is closed")?;
        let palette = self.shell.read_palette(layer);
        let entries = palette.entries().iter().map(|entry| {
            let keys = entry.keys.iter().map(|key| Value::String(key.clone())).collect();
            json::object([
                ("row", Value::String(entry.row.clone())),
                ("label", Value::String(entry.label.clone())),
                ("kind", Value::String(format!("{:?}", entry.kind).to_lowercase())),
                ("language_defined", Value::Bool(entry.language_defined)),
                ("keys", Value::Array(keys)),
                ("unavailable", entry.availability.err().map_or(Value::Null, |code| Value::String(code.to_owned()))),
            ])
        });
        Ok(json::object([
            ("revision", Value::Int(palette.revision.state as i64)),
            ("generation", Value::Int(palette.generation as i64)),
            ("entries", Value::Array(entries.collect())),
        ]))
    }

    /// The palette's route (EV4.2): the shell invokes `row` with absolute arguments `args` (`Shell::invoke`, the key
    /// route's own path, so the same gates apply), and the host builds what the layer queued for it
    /// ([`Turns::run_jobs`]). A language row answers `running` with its program job, which the host takes
    /// ([`Turns::take_job`]).
    pub fn invoke(&mut self, row: &str, args: Value) -> Result<Receipt, String> {
        let layer = self.layer.as_mut().ok_or("the session is closed")?;
        self.shell.invoke(layer, row, args).map_err(|error| format!("{}: {}", error.code, error.detail))
    }

    /// Takes the next program job the layer admitted (a language row's), as the host takes it before it runs the job's
    /// entry. While the host holds it, the run is inside the command that invoked it (ONE.34).
    pub fn take_job(&mut self) -> Option<ProgramJob> {
        self.layer.as_mut()?.take_program_job()
    }

    /// Reports the program job `job`: the entry's returned value, or the error that ended its run. The answer is the
    /// receipt of the solver's command that invoked it, or `None` when the layer no longer holds the job (a commit
    /// ended it).
    pub fn finish_job(&mut self, job: u64, result: Result<Value, LayerError>) -> Option<Receipt> {
        self.layer.as_mut()?.finish_program_job(job, result)
    }

    /// Sets the editor's draft, the word reference `operation.preview`'s key (Ctrl+Enter) reads (`{"recipe": [...]}`),
    /// or clears it.
    pub fn set_draft(&mut self, draft: Option<Value>) {
        self.shell.draft = draft;
    }

    /// Runs the jobs the layer queued that the layer itself runs, such as a word preview's build; a program job stays
    /// for [`Turns::take_job`]. The answer is how many ran.
    pub fn run_jobs(&mut self) -> usize {
        self.layer.as_mut().map_or(0, |layer| layer.run_jobs())
    }

    /// The pending preview while it is fresh, built and not yet ended (`ready` or `presented`) at the layer's state
    /// revision: what the host's view shows.
    pub fn pending(&self) -> Option<Pending> {
        let layer = self.layer.as_ref()?;
        let state = layer.revision().state;
        layer
            .preview()
            .filter(|p| matches!(p.state, PreviewState::Ready | PreviewState::Presented) && p.deps.state == state)
            .map(|p| Pending {
                id: p.id,
                literal: p.literal.clone(),
                effect: p.effect.net_sha256.clone(),
                state: p.deps.state,
            })
    }

    /// Reports one confirmed frame to the layer through the shell (RM.26, ONE.41), as `c600_tools::views` reports a
    /// composed one: for the preview the frame shows, else the pending one, else the one the shell bound, and nothing
    /// when there is none. A frame that does not present that preview ends its presentation; the layer's refusal of it
    /// is the answer.
    pub fn present(&mut self, frame: &PresentedFrame) -> Result<Option<u64>, LayerError> {
        let Some(layer) = self.layer.as_mut() else { return Ok(None) };
        let shown = frame.preview.as_ref().map(|shown| shown.id);
        let pending = layer.preview().map(|preview| preview.id);
        let bound = self.shell.presented().map(|binding| binding.id);
        match shown.or(pending).or(bound) {
            Some(id) => self.shell.present(layer, id, frame).map(|()| Some(id)),
            None => Ok(None),
        }
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
