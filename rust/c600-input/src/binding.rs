//! EV4.3's binding and its pure resolution.
//!
//! A [`Binding`] maps a sequence of physical chords and a routing class to H-08's stable row ID and the arguments the
//! row takes. Each argument is an absolute value or a relative name ([`Arg`]); the router resolves every relative name
//! with the context that fixes it at the press, before the envelope, so the invocation carries absolute arguments
//! only and a later change of that context never reinterprets it.

use c600_kernel::json::Value;

use crate::chord::Chord;

/// The routing classes of DT-M1 Outcome C2 and D3, in routing order (EV4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoutingClass {
    /// A key-capture field, which records the next chord.
    Capture,
    /// A text field, which takes every character key, IME composition, the clipboard keys and text undo.
    Text,
    /// A focused list or view, which takes only its navigation keys.
    List,
    /// The global map, for every other key wherever the focus sits.
    Global,
}

/// The input destination: the one mode that changes what a key does (EV4.4; DT-M1 C4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Destination {
    /// Twist keys twist the puzzle.
    Live,
    /// Twist keys append to the selected draft phase.
    Draft,
}

/// One argument of a binding: an absolute value, or a relative name resolved at the press.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    /// An absolute value fixed in the binding.
    Fixed(Value),
    /// The editor's draft (`WordRef`).
    Draft,
    /// The ID of the preview the shell presented last.
    PresentedPreview,
    /// LI.9's action hint for `session.undo`.
    UndoHint,
    /// LI.9's action hint for `session.redo`.
    RedoHint,
    /// The layer's revision reference (KN0.2).
    Revision,
    /// The cap of grip slot `n` in the applied slot map (EV4.8).
    SlotCap(usize),
    /// The transported frame of grip slot `n` in the applied slot map (EV4.8).
    SlotFrame(usize),
    /// The cap of the current grip.
    GripCap,
    /// The selected draft phase.
    Phase,
    /// The preview's scrub position moved by this many letters, never below zero.
    Scrub(i64),
    /// The focused list's ID.
    List,
    /// The ID of the focused list's entry this many places from the selection, held at the list's ends.
    ListEntry(i64),
    /// The object of the focused list's selected entry.
    Selected,
    /// The result the focused list shows.
    ListResult,
    /// The object of the focused list's row `n`, counted from 1 (DT-J1 O9).
    ListRow(usize),
}

/// EV4.3's binding, as applied under one layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub sequence: Vec<Chord>,
    pub context: RoutingClass,
    /// H-08's stable dotted row ID.
    pub command: String,
    pub args: Vec<(String, Arg)>,
    /// The layout the sequence was resolved under, for a binding recorded by character; `None` for one recorded by
    /// physical key code only.
    pub layout: Option<&'static str>,
    /// The destination the binding serves; `None` serves both.
    pub destination: Option<Destination>,
}

/// What [`resolve_binding`] reads besides the key sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoutingContext {
    pub class: RoutingClass,
    pub destination: Destination,
}

/// The answer of [`resolve_binding`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution<'a> {
    Bound(&'a Binding),
    /// The sequence so far is a proper prefix of a bound sequence: a pending prefix group (K1).
    Prefix,
    Unhandled,
}

/// `resolve_binding(key_event, routing_context, bindings, layout) -> Binding | unhandled` (EV4.3), pure. `sequence`
/// is the pending prefix followed by the event's chord. A binding answers only in its own routing class, for its
/// destination, and, when it was resolved by character, under the layout it was resolved for.
pub fn resolve_binding<'a>(
    sequence: &[Chord],
    context: RoutingContext,
    bindings: &'a [Binding],
    layout: &str,
) -> Resolution<'a> {
    if sequence.is_empty() {
        return Resolution::Unhandled;
    }
    let live = |b: &&Binding| {
        b.context == context.class
            && b.destination.map_or(true, |d| d == context.destination)
            && b.layout.map_or(true, |l| l == layout)
    };
    if let Some(binding) = bindings.iter().filter(live).find(|b| b.sequence == sequence) {
        return Resolution::Bound(binding);
    }
    if bindings.iter().filter(live).any(|b| b.sequence.len() > sequence.len() && b.sequence.starts_with(sequence)) {
        return Resolution::Prefix;
    }
    Resolution::Unhandled
}
