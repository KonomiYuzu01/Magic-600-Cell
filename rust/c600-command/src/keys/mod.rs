//! The key map (chapter 04, EV4.3, EV4.4 and EV4.7; DT-M1 Outcome C2, C4, K1 and K5; LP4-8): physical chords, the
//! declared layouts, bindings to absolute arguments, EV4.7's checks and the draft default map.
//!
//! One checked key map serves the shell's key route ([`crate::Shell::press`]) and the input router of `c600-input`,
//! which re-exports this module. LP3 built it in `c600-input` (C5); LP4-8 moved it here, so that the shell's map is
//! checked as the router's is and one default map remains ([`default_specs`]). Every binding is checked whole: a new
//! one is added by [`KeyMap::bind`], which runs the duplicate, prefix and layout checks of the whole map in each
//! routing class and under every declared layout, and a finding refuses it with the map unchanged. Nothing here reads
//! a device, so no test of it is Windows input or IME evidence (CT-EV4-05 stays a windows test).
//!
//! # Drafts
//!
//! G6 has no keymap table, chapter 05 owns the persisted key-map forms, and the command table, frozen at LP3 (PHS.53),
//! holds no 1.0 key map; these choices stay drafts until chapter 05's key-map forms settle them.
//!
//! - Routing class names (EV4.4; DT-M1 C2 and D3): [`RoutingClass::Capture`], [`RoutingClass::Text`],
//!   [`RoutingClass::List`] (a list or a view) and [`RoutingClass::Global`]. H-08's eight contexts are not read for
//!   routing; the engine still decides availability per revision. A focused view is modelled as a list of entries.
//! - Chord form (EV4.7; K5): a physical key code in the W3C UI Events `code` notation (`KeyQ`, `Digit7`, `ArrowUp`)
//!   with Ctrl, Shift, Alt and Meta, written `Ctrl+Shift+Enter` with the modifiers in that order. A character intent is
//!   written `Ctrl+'z'`; only undo and redo, of the move history and of the layout history, use one (Q2 (b)).
//!   [`BindingSpec`] and [`Binding`] stand in for chapter 05's persisted intent and resolved-map forms.
//! - Layouts (EV4.7): US QWERTY and French AZERTY, as tables of the characters each key types unshifted and with
//!   Shift, without dead keys or AltGr levels. A character intent resolves to the key whose unshifted character it is,
//!   or whose Shift character it is with Shift added; a capital letter is its small letter with Shift.
//! - Key-map checks (EV4.7): a duplicate of two bindings recorded the same way is `EV4:input.binding_conflict`. A
//!   collision that exists only because a character intent resolved onto another binding's chord under a declared
//!   layout is `EV4:input.binding_layout_mismatch`, named with that layout, as is a character that a declared layout
//!   does not type; an undeclared active layout is the same code with no layout named. The prefix-code check ignores
//!   the destination; two bindings for different destinations are not duplicates. The class rule (EV4.4; DT-M1 C2 and
//!   fix-first item 2: no text-class or capture-class binding, since those fields keep their keys; list-class bindings
//!   reach only [`LIST_ROWS`], since a list takes only its navigation keys) reports `EV4:input.binding_conflict`, read
//!   as a binding that conflicts with the keys its routing class takes itself. EV4.3 lists no other key-map code and
//!   chapter 13 registers no row of its own for this rule; a dedicated row would be a change to chapter 13 (PHS.52). A
//!   binding with no chord is `EV4:input.binding_prefix_conflict` once, since it is a prefix of every sequence, and
//!   takes no part in the pair checks.
//! - The global map ([`default_specs`]): `Ctrl+Enter` and `Ctrl+Shift+Enter` preview and execute (0.4's keys);
//!   `Ctrl+'z'` and `Ctrl+'y'` undo and redo; `Ctrl+Alt+'z'` and `Ctrl+Alt+'y'` undo and redo the layout history
//!   (ADR-D01.14); `Escape` closes the innermost transient layer (`transient.close`) and, before that, a pending prefix
//!   group (C4, K1), so it never cancels a preview; `Ctrl+Shift+Backspace` cancels the presented preview; `F9` toggles
//!   the on-screen keyboard (0.4's key); the prefix group `Ctrl+KeyG` then `KeyF` or `KeyI` toggles the frame display
//!   or the inverse; the arrows and `Equal`/`Minus` rotate and zoom the `global` view's camera;
//!   `Alt+ArrowLeft`/`Alt+ArrowRight` scrub the presented preview; `KeyQ`, `KeyW`, `KeyE` and `KeyR` select grip slots
//!   0 to 3; `KeyJ`, `KeyK` and `KeyL`, with Shift for the inverse, are three twist keys, which run `twist.apply` in
//!   the live destination and `operation.phase.append_twist` in the draft one (C4, C7). The full slot block and the
//!   seven axis keys (K2) wait for G6. A focused list or view: the arrows (`list.move`), Enter (`object.inspect`) and
//!   Digit1 to Digit9 and Digit0 (`list.row.copy` of rows 1 to 10, DT-J1 O9).
//! - Placeholder values: the camera rotations and zoom steps and the camera's view (`"global"`) are fixed JSON
//!   placeholders.
//! - The twist keys' exact requests (A6; KN3.7): a twist key types one item of the twist menu the host pins, pressed
//!   directly ([`Arg::MenuItem`]); [`TWIST_KEYS`] maps `KeyJ`, `KeyK` and `KeyL` to items 1, 2 and 3 and their Shift
//!   chords to the inverse items 1, 5 and 9 under the I_a menu's order. Which key types which exact request settles
//!   nothing in the chapters: G6 owns the keys, K2's seven axis keys among them.

pub mod binding;
pub mod chord;
pub mod keymap;
pub mod layout;

pub use binding::{resolve_binding, Arg, Binding, Destination, Resolution, RoutingClass, RoutingContext};
pub use chord::{Chord, Key, Mods};
pub use keymap::{check, default_specs, BindingSpec, ChordIntent, Finding, KeyMap, LIST_ROWS, TWIST_KEYS};
pub use layout::Layout;

/// The shell's draft active layout until the platform layer reports one: US QWERTY, declared with French AZERTY.
pub const DRAFT_LAYOUT: &str = "us-qwerty";
