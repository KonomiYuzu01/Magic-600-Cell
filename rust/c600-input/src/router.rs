//! The router: key events to command-layer invocations, in chapter 04's fixed routing order (EV4.4), with held keys
//! repeating only the allowlisted rows (EV4.6).
//!
//! The router drives `c600-command` through its public API only: it resolves a binding, resolves the binding's
//! relative names at the press, and passes the absolute arguments to [`Shell::prepare`] and [`Shell::send`], so a key
//! bound to a kernel row reaches the layer with no interpreter step (EV4.5). It holds no queue: every key-down is
//! dispatched at once, taken by a field, or dropped.

use std::collections::BTreeMap;

use c600_command::{code, CommandLayer, Direction, Error, Invocation, Kind, Receipt, Shell, Store};
use c600_kernel::json::{self, Value};
use c600_kernel::request::{menu_request, MenuPin};

use crate::binding::{resolve_binding, Arg, Binding, Destination, Resolution, RoutingClass, RoutingContext};
use crate::chord::{Chord, Key};
use crate::keymap::{BindingSpec, Finding, KeyMap};
use crate::layout::Layout;

/// One synthetic key event, as the platform layer would deliver it after reading labels and IME.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    /// A key-down. `repeat` is the platform's auto-repeat flag; `text` is the character the key typed, if any.
    Down {
        chord: Chord,
        repeat: bool,
        text: Option<char>,
    },
    Up(Key),
    /// IME composition text.
    Compose(String),
    /// The window lost the keyboard focus.
    FocusLost,
}

impl KeyEvent {
    fn chord(text: &str) -> Chord {
        Chord::parse(text).unwrap_or_else(|| panic!("`{text}` is not a chord"))
    }

    /// A key-down that types nothing.
    pub fn down(chord: &str) -> KeyEvent {
        KeyEvent::Down { chord: KeyEvent::chord(chord), repeat: false, text: None }
    }

    /// A key-down that types `text`.
    pub fn typed(chord: &str, text: char) -> KeyEvent {
        KeyEvent::Down { chord: KeyEvent::chord(chord), repeat: false, text: Some(text) }
    }

    /// An auto-repeat key-down, flagged as the platform flags one.
    pub fn repeat(chord: &str) -> KeyEvent {
        KeyEvent::Down { chord: KeyEvent::chord(chord), repeat: true, text: None }
    }

    pub fn up(chord: &str) -> KeyEvent {
        KeyEvent::Up(KeyEvent::chord(chord).key)
    }
}

/// One entry of a focused list or view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The entry's ID (`EntryId`).
    pub id: Value,
    /// The canonical object the entry shows (`CanonicalObjectRef`).
    pub object: Value,
}

/// A focused list or view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListFocus {
    /// `ListId`.
    pub list: Value,
    /// The result the rows come from (`ResultRef`), such as DT-J1's repair list.
    pub result: Value,
    pub entries: Vec<Entry>,
    pub selected: usize,
}

/// What has the keyboard focus inside the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Focus {
    /// The puzzle views and the workspace, with no field or list that takes keys.
    Puzzle,
    /// A key-capture field.
    Capture,
    /// A text field.
    Text,
    /// A list or view.
    List(ListFocus),
}

impl Focus {
    pub fn class(&self) -> RoutingClass {
        match self {
            Focus::Puzzle => RoutingClass::Global,
            Focus::Capture => RoutingClass::Capture,
            Focus::Text => RoutingClass::Text,
            Focus::List(_) => RoutingClass::List,
        }
    }
}

/// One grip slot of a slot map: an absolute cap and its transported frame (EV4.8).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// `CapId`.
    pub cap: Value,
    /// `FrameRef`.
    pub frame: Value,
}

/// The context relative names resolve against. Its slot map is the host's default map, fixed when the context is made;
/// the applied map is the solver's last `grip.rebind` in the command layer, or this default before the first one
/// ([`Router::slots`]; EV4.8, DT-M1 K6). The camera, hover and filter fields are here so tests can change them, and no
/// argument reads them, so none of them derives a new applied map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub destination: Destination,
    slots: Vec<Slot>,
    /// The current grip: an index into the applied slot map.
    pub grip: Option<usize>,
    /// The twist menu's two digests (KN3.7). The host sets them with the menu it pins in the command layer
    /// (`CommandLayer::pin_menu`); a twist key's [`Arg::MenuItem`] resolves against them, and with none a twist key is
    /// refused before any envelope.
    pub menu: Option<MenuPin>,
    /// The selected draft phase (`PhaseId`).
    pub phase: Option<Value>,
    /// The preview's scrub position (`WordPosition`).
    pub scrub: i64,
    pub camera: Value,
    pub hover: Option<Value>,
    pub filter: Option<Value>,
}

impl Context {
    pub fn new(slots: Vec<Slot>) -> Context {
        Context {
            destination: Destination::Live,
            slots,
            grip: None,
            menu: None,
            phase: None,
            scrub: 0,
            camera: Value::Null,
            hover: None,
            filter: None,
        }
    }

    /// The host's default slot map, which applies until the solver's first rebind.
    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }
}

/// Why a held key's event was dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropReason {
    /// A repeat of a key the router does not hold: its key-down came before a focus loss or never came.
    NotHeld,
    /// A repeat of a key whose binding's row is not on the repeat allowlist, or of a key no row took; the row, if any.
    NotRepeatable(Option<String>),
}

/// One invocation the router sent: the binding, the envelope with its absolute arguments, and the receipt.
#[derive(Clone, Debug)]
pub struct Dispatch {
    pub binding: Binding,
    pub invocation: Invocation,
    pub receipt: Receipt,
}

/// Where one key event went.
#[derive(Clone, Debug)]
pub enum Routed {
    /// The key-capture field recorded the chord.
    Captured(Chord),
    /// The text field took the event.
    Text,
    /// Tab moved the focus inside the focused list or view; no row runs.
    Focus,
    /// The chord opened or continued a prefix group, which waits with no time-out (K1).
    Prefix,
    /// Escape, or a key that does not continue the group, closed the pending prefix group; nothing runs.
    PrefixClosed,
    /// An invocation reached the command layer.
    Dispatched(Box<Dispatch>),
    /// The shell refused the press before any envelope: a relative name had nothing to resolve to, or the shell's own
    /// gate refused. Nothing reached the layer.
    Refused {
        row: String,
        error: Error,
    },
    Dropped(DropReason),
    /// A key-up or a focus loss.
    Released,
    Unhandled,
}

impl Routed {
    pub fn dispatch(&self) -> Option<&Dispatch> {
        match self {
            Routed::Dispatched(dispatch) => Some(dispatch),
            _ => None,
        }
    }

    pub fn receipt(&self) -> Option<&Receipt> {
        self.dispatch().map(|dispatch| &dispatch.receipt)
    }
}

/// What took a held key's key-down, which decides its repeats.
#[derive(Clone, Debug)]
enum Take {
    Capture,
    Text,
    Binding(Box<Binding>),
    Other,
}

/// The headless input router.
pub struct Router {
    keymap: KeyMap,
    focus: Focus,
    pub context: Context,
    held: BTreeMap<Key, Take>,
    pending: Vec<Chord>,
    log: Vec<Dispatch>,
}

impl Router {
    pub fn new(keymap: KeyMap, context: Context) -> Router {
        Router { keymap, focus: Focus::Puzzle, context, held: BTreeMap::new(), pending: Vec::new(), log: Vec::new() }
    }

    pub fn keymap(&self) -> &KeyMap {
        &self.keymap
    }

    /// The applied grip-slot map (EV4.8): the map of the solver's last `grip.rebind`, which `layer` holds, or the
    /// context's default map before the first one. Only the rebind command changes it.
    pub fn slots<S: Store>(&self, layer: &CommandLayer<S>) -> Vec<Slot> {
        match layer.slot_map() {
            Some(map) => map
                .slots
                .into_iter()
                .map(|slot| Slot { cap: Value::Int(i64::from(slot.cap)), frame: slot.frame })
                .collect(),
            None => self.context.slots.clone(),
        }
    }

    /// Checks and applies a new key map; on a finding the current map stays (EV4.7).
    pub fn apply(&mut self, specs: &[BindingSpec], declared: &[Layout], active: &str) -> Result<(), Vec<Finding>> {
        self.keymap = KeyMap::apply(specs, declared, active)?;
        self.release();
        Ok(())
    }

    pub fn focus(&self) -> &Focus {
        &self.focus
    }

    /// Moves the focus inside the window. Held keys and a pending prefix are released, as on a focus loss.
    pub fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
        self.release();
    }

    /// The focused list, for the view to move its selection without moving the focus.
    pub fn list_mut(&mut self) -> Option<&mut ListFocus> {
        match &mut self.focus {
            Focus::List(list) => Some(list),
            _ => None,
        }
    }

    /// The keys the router holds: each has had a key-down and no key-up or focus loss since.
    pub fn held(&self) -> Vec<Key> {
        self.held.keys().copied().collect()
    }

    /// The pending prefix group's chords.
    pub fn pending(&self) -> &[Chord] {
        &self.pending
    }

    /// Every invocation the router sent, in order.
    pub fn log(&self) -> &[Dispatch] {
        &self.log
    }

    fn release(&mut self) {
        self.held.clear();
        self.pending.clear();
    }

    /// Routes one key event. A key-down is a repeat when the platform flags it or when the router still holds the key;
    /// a repeat runs only a binding whose row's H-08 `repeat` flag is set, and only while its key is held.
    pub fn key<S: Store>(&mut self, shell: &mut Shell, layer: &mut CommandLayer<S>, event: KeyEvent) -> Routed {
        match event {
            KeyEvent::FocusLost => {
                self.release();
                Routed::Released
            }
            KeyEvent::Up(key) => {
                self.held.remove(&key);
                Routed::Released
            }
            KeyEvent::Compose(_) => match self.focus {
                Focus::Text => Routed::Text,
                _ => Routed::Unhandled,
            },
            KeyEvent::Down { chord, repeat, text } => match self.held.get(&chord.key).cloned() {
                Some(take) => self.repeat(shell, layer, take),
                None if repeat => Routed::Dropped(DropReason::NotHeld),
                None => {
                    let (take, routed) = self.press(shell, layer, chord, text);
                    self.held.insert(chord.key, take);
                    routed
                }
            },
        }
    }

    fn repeat<S: Store>(&mut self, shell: &mut Shell, layer: &mut CommandLayer<S>, take: Take) -> Routed {
        match take {
            Take::Text => Routed::Text,
            Take::Binding(binding) => match layer.table().row(&binding.command).is_some_and(|row| row.repeat) {
                true => self.dispatch(shell, layer, &binding),
                false => Routed::Dropped(DropReason::NotRepeatable(Some(binding.command.clone()))),
            },
            Take::Capture | Take::Other => Routed::Dropped(DropReason::NotRepeatable(None)),
        }
    }

    /// A fresh key-down, in routing order: key capture, text, a focused list or view, then the global map.
    fn press<S: Store>(
        &mut self,
        shell: &mut Shell,
        layer: &mut CommandLayer<S>,
        chord: Chord,
        text: Option<char>,
    ) -> (Take, Routed) {
        let pending = std::mem::take(&mut self.pending);
        let class = self.focus.class();
        match class {
            RoutingClass::Capture => return (Take::Capture, Routed::Captured(chord)),
            RoutingClass::Text if self.text_takes(chord, text) => return (Take::Text, Routed::Text),
            RoutingClass::List => {
                if chord.key == Key::TAB && !chord.mods.command() {
                    return (Take::Other, Routed::Focus);
                }
                if let Some(binding) = self.bound(&[chord], RoutingClass::List) {
                    let routed = self.dispatch(shell, layer, &binding);
                    return (Take::Binding(Box::new(binding)), routed);
                }
            }
            RoutingClass::Text | RoutingClass::Global => {}
        }
        if !pending.is_empty() && chord.key == Key::ESCAPE && chord.mods == Default::default() {
            return (Take::Other, Routed::PrefixClosed);
        }
        let mut sequence = pending.clone();
        sequence.push(chord);
        let context = RoutingContext { class: RoutingClass::Global, destination: self.context.destination };
        let found = match resolve_binding(&sequence, context, self.keymap.bindings(), self.keymap.layout().id()) {
            Resolution::Bound(binding) => Some(binding.clone()),
            Resolution::Prefix => {
                self.pending = sequence;
                return (Take::Other, Routed::Prefix);
            }
            Resolution::Unhandled => None,
        };
        match found {
            // DT-M1 C4: the commit works from any focus outside a text field, so a text field keeps a global key
            // bound to a commit row.
            Some(binding) if class == RoutingClass::Text && self.commits(layer, &binding) => (Take::Text, Routed::Text),
            Some(binding) => {
                let routed = self.dispatch(shell, layer, &binding);
                (Take::Binding(Box::new(binding)), routed)
            }
            None if pending.is_empty() => (Take::Other, Routed::Unhandled),
            None => (Take::Other, Routed::PrefixClosed),
        }
    }

    fn bound(&self, sequence: &[Chord], class: RoutingClass) -> Option<Binding> {
        let context = RoutingContext { class, destination: self.context.destination };
        match resolve_binding(sequence, context, self.keymap.bindings(), self.keymap.layout().id()) {
            Resolution::Bound(binding) => Some(binding.clone()),
            Resolution::Prefix | Resolution::Unhandled => None,
        }
    }

    fn commits<S: Store>(&self, layer: &CommandLayer<S>, binding: &Binding) -> bool {
        layer.table().row(&binding.command).is_some_and(|row| row.kind == Kind::Commit)
    }

    /// Whether a text field takes a key-down: any key that typed a character (AltGr levels included), a character key
    /// with no modifier but Shift, Enter with no modifier but Shift, the editing keys with Ctrl or Shift, and the
    /// office chords Ctrl+A, C, V, X, Y and Z (with or without Shift) by the character the active layout gives them.
    fn text_takes(&self, chord: Chord, text: Option<char>) -> bool {
        let mods = chord.mods;
        if text.is_some() {
            return true;
        }
        if !mods.command() && (chord.key.types_character() || chord.key.is_editing() || chord.key == Key::ENTER) {
            return true;
        }
        if mods.ctrl && !mods.alt && !mods.meta {
            if chord.key.is_editing() {
                return true;
            }
            return self.keymap.layout().base(chord.key).is_some_and(|c| "acvxyz".contains(c));
        }
        false
    }

    /// Resolves the binding's relative names at this press and sends the invocation.
    fn dispatch<S: Store>(&mut self, shell: &mut Shell, layer: &mut CommandLayer<S>, binding: &Binding) -> Routed {
        let refused = |error: Error| Routed::Refused { row: binding.command.clone(), error };
        let args = match self.arguments(shell, layer, binding) {
            Ok(args) => args,
            Err(error) => return refused(error),
        };
        let invocation = match shell.prepare(layer, &binding.command, args) {
            Ok(invocation) => invocation,
            Err(error) => return refused(error),
        };
        let receipt = shell.send(layer, invocation.clone());
        let dispatch = Dispatch { binding: binding.clone(), invocation, receipt };
        self.log.push(dispatch.clone());
        Routed::Dispatched(Box::new(dispatch))
    }

    fn arguments<S: Store>(&self, shell: &Shell, layer: &CommandLayer<S>, binding: &Binding) -> Result<Value, Error> {
        let missing = |what: &str| Error::new(code::ARGUMENT_FORM, format!("`{}`: {what}", binding.command));
        let list = || match &self.focus {
            Focus::List(list) => Ok(list),
            _ => Err(missing("no list or view has the focus")),
        };
        let slots = self.slots(layer);
        let slot = |n: usize| slots.get(n).ok_or_else(|| missing("no such grip slot"));
        let mut args = BTreeMap::new();
        for (name, arg) in &binding.args {
            let value = match arg {
                Arg::Fixed(value) => value.clone(),
                Arg::Draft => shell.draft.clone().ok_or_else(|| missing("the editor holds no draft"))?,
                Arg::PresentedPreview => match shell.presented() {
                    Some(preview) => Value::Int(preview.id as i64),
                    None => return Err(Error::new(code::NO_FRESH_PREVIEW, "no presented preview")),
                },
                Arg::UndoHint | Arg::RedoHint => {
                    let direction = if *arg == Arg::UndoHint { Direction::Undo } else { Direction::Redo };
                    let hint = layer.undo_hint(direction);
                    hint.ok_or_else(|| Error::new(code::UNDO_SUPERSEDED, "no eligible target"))?.to_json()
                }
                Arg::LayoutUndoHint | Arg::LayoutRedoHint => {
                    let direction = if *arg == Arg::LayoutUndoHint { Direction::Undo } else { Direction::Redo };
                    let hint = layer.layout_hint(direction);
                    hint.ok_or_else(|| Error::new(code::UNDO_SUPERSEDED, "no eligible target"))?.to_json()
                }
                Arg::Revision => layer.revision().to_json(),
                Arg::SlotCap(n) => slot(*n)?.cap.clone(),
                Arg::SlotFrame(n) => slot(*n)?.frame.clone(),
                Arg::GripCap => slot(self.context.grip.ok_or_else(|| missing("no grip"))?)?.cap.clone(),
                Arg::MenuItem(index) => {
                    menu_item(self.context.menu.as_ref().ok_or_else(|| missing("no twist menu is pinned"))?, *index)
                }
                Arg::Phase => self.context.phase.clone().ok_or_else(|| missing("no draft phase is selected"))?,
                Arg::Scrub(step) => Value::Int((self.context.scrub + step).max(0)),
                Arg::List => list()?.list.clone(),
                Arg::ListEntry(step) => {
                    let list = list()?;
                    let last = list.entries.len().checked_sub(1).ok_or_else(|| missing("the list is empty"))?;
                    let at = (list.selected as i64 + step).clamp(0, last as i64) as usize;
                    list.entries[at].id.clone()
                }
                Arg::Selected => {
                    let list = list()?;
                    list.entries.get(list.selected).ok_or_else(|| missing("no entry is selected"))?.object.clone()
                }
                Arg::ListResult => list()?.result.clone(),
                Arg::ListRow(n) => {
                    let list = list()?;
                    let row = n.checked_sub(1).and_then(|i| list.entries.get(i));
                    row.ok_or_else(|| missing("the list has no such row"))?.object.clone()
                }
            };
            args.insert(name.clone(), value);
        }
        Ok(Value::Object(args))
    }
}

/// Item `index` of the pinned menu, pressed directly, as KN3.7's binding without the cap the row names: the
/// `{request, operation, menu}` object of `twist.apply`'s `rotation` argument, in `Binding::to_json`'s form. The
/// request bytes are ASCII: two hexadecimal digests and a decimal index.
fn menu_item(pin: &MenuPin, index: u64) -> Value {
    let request = String::from_utf8_lossy(&menu_request(&pin.set, &pin.order, index)).into_owned();
    let text = |s: &str| Value::String(s.to_owned());
    json::object([
        ("request", Value::String(request)),
        ("operation", json::object([("kind", text("menu_direct"))])),
        ("menu", json::object([("set", text(&pin.set)), ("order", text(&pin.order))])),
    ])
}
