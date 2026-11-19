//! The shell's turns (C9, charter E2; a draft): KN5's command layer over M6's journal in a fresh session folder, with
//! LP3's input router in front of it, so that a twist key the window receives becomes a live `twist.apply` commit.
//!
//! - The session is `JournalStore` on `FsDisk` (the product's disk, with the session lock) in a folder that must be new
//!   or empty, at the solved state, protecting nothing. The layer pins the twist menu the shell names (draft: the I_a
//!   menu of `research/jumbling/fixtures/wj-I_a.json`, the menu KN3's and c600-input's tests pin; G6 owns the menu).
//! - Or the session is an existing one (charter E3, a draft): [`Turns::reopen`] reopens a fresh copy of a session
//!   fixture's folder ([`crate::fixture`]) with `CommandLayer::open_with_menu` and the same menu. Its state must be the
//!   fixture's, and the fixture's committed presses become the session's first presses, so the control below covers
//!   them as well as the run's.
//! - Keys go through `c600_input::Router` on its draft default map (US QWERTY): `KeyJ`, `KeyK` and `KeyL`, with Shift
//!   for the inverse, are its twist keys. The router's grip is slot 0, cap 0 (draft: `grip.rebind` and `grip.select`
//!   are not built, so the cap never changes). Every other key reaches the router too; the rows it names that the layer
//!   does not build are refused there and change nothing. A key whose row the layer builds and that changes the state
//!   (`session.undo` and `session.redo` on Ctrl+Z and Ctrl+Y) is [`Press::Other`]: the host shows the new state, and
//!   the control fails, since its replay knows only twists.
//! - A measured press is a key-down (not a repeat) whose binding is `twist.apply`: committed, or refused by the router
//!   or the layer. A repeat of a twist key is dropped by the router (EV4.6), so it is never a press.
//! - The correctness control ([`Turns::check`]) runs once, when the shell closes the session: the live state, the state
//!   a reopen of the journal replays (`CommandLayer::open_with_menu`), and the kernel's own replay of every committed
//!   press from the solved state (each press's binding resolved against the menu outside the layer, its retained word
//!   derived by the kernel) must agree, and the state the shell last saw presented must be the replay's state after as
//!   many presses as that frame's engine revision counts.
//!
//! Nothing here is Windows/DirectX, input or performance evidence.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_command::{CommandLayer, Secret, Shell, Status, Table, TWIST_ROW};
use c600_input::{Chord, Context as Keys, KeyEvent, KeyMap, Routed, Router, Slot};
use c600_jumble::rotation;
use c600_jumble::twist::Menu;
use c600_jumble::Context as Exact;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{Binding, MenuPin};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};
use c600_session::disk::FsDisk;
use c600_session::JournalStore;

use crate::fixture::Fixture;

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

/// What one key-down did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// Not a measured press: no twist key, or a repeat.
    None,
    /// A twist key committed `twist.apply`.
    Committed,
    /// A twist key that the router or the layer refused; the state did not change.
    Refused,
    /// Not a measured press, but another row changed the state (the draft map's `session.undo` and `session.redo`).
    /// The control's replay knows only twists, so a run with one fails its control.
    Other,
}

pub struct Turns {
    kernel: Arc<Kernel>,
    exact: Arc<Exact>,
    /// The menu record, read again for the reopen and the replay (a `Menu` is not `Clone`).
    record: Value,
    folder: PathBuf,
    layer: Option<CommandLayer<JournalStore>>,
    shell: Shell,
    router: Router,
    /// The arguments of every committed press, in order.
    committed: Vec<Value>,
    refused: u64,
    /// Key-downs that changed the state through another row.
    others: u64,
}

fn menu(exact: &Exact, record: &Value) -> Result<Menu, String> {
    Menu::from_record(exact, record).map_err(|error| format!("the twist menu does not read: {error}"))
}

/// What a new and a reopened session both read: the menu record, the exact model, the pinned menu, and the router on
/// its draft default map with the menu pinned by its two digests.
struct Pinned {
    exact: Arc<Exact>,
    record: Value,
    menu: Menu,
    router: Router,
}

impl Pinned {
    fn read(assets: &Path, menu_file: &Path) -> Result<Pinned, String> {
        let text = std::fs::read(menu_file).map_err(|error| format!("the twist menu file: {error}"))?;
        let file = json::parse(&text).map_err(|error| format!("the twist menu file is not JSON: {error:?}"))?;
        let record = file.get("menu").cloned().ok_or("the twist menu file has no `menu` record")?;
        let exact = Arc::new(Exact::load(assets).map_err(|error| format!("the exact model: {error:?}"))?);
        let menu = menu(&exact, &record)?;
        let pin = MenuPin { set: menu.identity.clone(), order: menu.order.clone() };
        let keymap = KeyMap::default_for(LAYOUT).map_err(|findings| format!("the default key map: {findings:?}"))?;
        let mut keys = Keys::new(vec![Slot { cap: Value::Int(CAP), frame: Value::Null }]);
        keys.menu = Some(pin);
        keys.grip = Some(0);
        Ok(Pinned { exact, record, menu, router: Router::new(keymap, keys) })
    }
}

/// KN3.7's binding from `twist.apply`'s arguments: `rotation`'s fields and the row's `cap`, as the layer reads it.
fn binding(args: &Value) -> Result<Binding, String> {
    let mut fields = args.get("rotation").and_then(Value::as_object).cloned().ok_or("a press without its rotation")?;
    fields.insert("cap".to_owned(), args.get("cap").cloned().unwrap_or(Value::Null));
    Binding::from_json(&Value::Object(fields)).map_err(|refusal| format!("the press's binding: {refusal:?}"))
}

impl Turns {
    /// A new session in `folder` (created if missing; it must hold nothing yet) at the solved state, with the twist
    /// menu of the record file `menu_file` (an object whose `menu` field is the record) pinned in the layer and, by its
    /// two digests, in the router.
    pub fn open(kernel: Arc<Kernel>, assets: &Path, folder: &Path, menu_file: &Path) -> Result<Turns, String> {
        if std::fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_some()) {
            return Err("the session folder is not empty; a run never reuses a session".to_owned());
        }
        let Pinned { exact, record, menu, router } = Pinned::read(assets, menu_file)?;
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
        Ok(Turns::with(kernel, exact, record, folder, layer, router, Vec::new()))
    }

    /// The existing session that `fixture` describes, reopened in `folder` (a fresh copy of the fixture's session
    /// folder) with the twist menu of `menu_file` pinned as [`Turns::open`] pins it. The reopened state's digest must
    /// be the fixture's; the fixture's committed presses become this session's first presses.
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
        let Pinned { exact, record, menu, router } = Pinned::read(assets, menu_file)?;
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
        let digest = layer.snapshot().digest();
        if digest != fixture.digest {
            return Err(format!("the reopened session's state {digest} is not the fixture's {}", fixture.digest));
        }
        Ok(Turns::with(kernel, exact, record, folder, layer, router, fixture.committed.clone()))
    }

    fn with(
        kernel: Arc<Kernel>,
        exact: Arc<Exact>,
        record: Value,
        folder: &Path,
        layer: CommandLayer<JournalStore>,
        router: Router,
        committed: Vec<Value>,
    ) -> Turns {
        Turns {
            kernel,
            exact,
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

    fn layer(&mut self) -> Result<&mut CommandLayer<JournalStore>, String> {
        self.layer.as_mut().ok_or_else(|| "the session is closed".to_owned())
    }

    /// Routes one key-down, `chord` in c600-input's form (`Shift+KeyJ`); `repeat` is the platform's auto-repeat flag.
    pub fn key_down(&mut self, chord: &str, repeat: bool) -> Result<Press, String> {
        let chord = Chord::parse(chord).ok_or_else(|| format!("`{chord}` is not a chord"))?;
        let layer = self.layer.as_mut().ok_or("the session is closed")?;
        let before = layer.snapshot().digest().to_owned();
        let routed = self.router.key(&mut self.shell, layer, KeyEvent::Down { chord, repeat, text: None });
        let changed = layer.snapshot().digest() != before;
        let press = match &routed {
            Routed::Dispatched(dispatch) if dispatch.invocation.row == TWIST_ROW => {
                if matches!(dispatch.receipt.status, Status::Committed(_)) {
                    self.committed.push(dispatch.invocation.args.clone());
                    Press::Committed
                } else {
                    Press::Refused
                }
            }
            Routed::Refused { row, .. } if row == TWIST_ROW => Press::Refused,
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

    /// The layer's state.
    pub fn state(&mut self) -> Result<Snapshot, String> {
        Ok(self.layer()?.snapshot().clone())
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
    /// committed press with the kernel, and compares. `shown` is the engine revision of the newest frame the shell saw
    /// presented and its state's digest as the host adopted it (revision 1 is the solved state; each committed press
    /// adds one). The answer is the run row's `correctness_detail` object.
    pub fn check(&mut self, shown: Option<(u64, String)>) -> Value {
        let live = self.layer.take().map(|layer| layer.snapshot().digest().to_owned());
        let reopened = self.reopened_digest();
        let replayed = self.replay();
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

    /// The digest of the state that a reopen of the closed session's journal replays.
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
        Ok(layer.snapshot().digest().to_owned())
    }

    /// The digest of the solved state, then of the state after each committed press, by the kernel alone.
    fn replay(&self) -> Result<Vec<String>, String> {
        let pinned = menu(&self.exact, &self.record)?;
        let mut state = self.kernel.solved();
        let mut digests = vec![state.digest().to_owned()];
        for args in &self.committed {
            let resolution = rotation::resolve(&self.exact, Some(&pinned), &binding(args)?)
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
}
