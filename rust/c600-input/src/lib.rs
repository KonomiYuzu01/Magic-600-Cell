//! The input router of LP3 (chapter 04, EV4.3 to EV4.7; DT-M1 Outcome C2, C4, K1, K4 and K5), headless.
//!
//! [`Router::key`] takes one synthetic key event and routes it in EV4.4's fixed order: a key-capture field, a text
//! field, a focused list or view taking only its navigation keys, then the global map. A key that reaches a binding
//! has its relative names resolved at the press ([`Arg`]) and goes to the command layer through `c600-command`'s public
//! shell API ([`c600_command::Shell::prepare`], [`c600_command::Shell::send`]), with absolute arguments only. A held
//! key repeats only the rows EV4.6 allows; focus loss releases held keys; busy input is refused by the layer and never
//! queued here. [`KeyMap::apply`] runs EV4.7's checks per routing class and per declared layout before a map applies.
//!
//! Every event here is synthetic: nothing in this crate reads a device, and its tests are not Windows input evidence
//! (CT-EV4-05 stays a windows test).
//!
//! # Drafts
//!
//! G6 has no keymap table, chapter 05 owns the persisted key-map forms, and the command table freezes at LP3 (PHS.53);
//! these choices are drafts until then.
//!
//! - Routing class names (EV4.4; DT-M1 C2 and D3): [`RoutingClass::Capture`], [`RoutingClass::Text`],
//!   [`RoutingClass::List`] (a list or a view) and [`RoutingClass::Global`]. H-08's eight contexts are not read for
//!   routing; the engine still decides availability per revision. A focused view is modelled as a list of entries.
//! - Chord form (EV4.7; K5): a physical key code in the W3C UI Events `code` notation (`KeyQ`, `Digit7`, `ArrowUp`)
//!   with Ctrl, Shift, Alt and Meta, written `Ctrl+Shift+Enter` with the modifiers in that order. A character intent is
//!   written `Ctrl+'z'`; only undo and redo use one (Q2 (b)). [`BindingSpec`] and [`Binding`] stand in for chapter 05's
//!   persisted intent and resolved-map forms.
//! - Layouts (EV4.7): US QWERTY and French AZERTY, as tables of the characters each key types unshifted and with
//!   Shift, without dead keys or AltGr levels. A character intent resolves to the key whose unshifted character it is,
//!   or whose Shift character it is with Shift added; a capital letter is its small letter with Shift.
//! - Key-map checks (EV4.7): a duplicate of two bindings recorded the same way is `EV4:input.binding_conflict`. A
//!   collision that exists only because a character intent resolved onto another binding's chord under a declared
//!   layout is `EV4:input.binding_layout_mismatch`, named with that layout, as is a character that a declared layout
//!   does not type; an undeclared active layout is the same code with no layout named. The prefix-code check ignores the destination; two bindings for different
//!   destinations are not duplicates. The class rule (EV4.4; DT-M1 C2 and fix-first item 2: no text-class or
//!   capture-class binding, since those fields keep their keys; list-class bindings reach only [`LIST_ROWS`], since a
//!   list takes only its navigation keys) reports `EV4:input.binding_conflict`, read as a binding that conflicts with
//!   the keys its routing class takes itself. EV4.3 lists no other key-map code and chapter 13 registers no row of its
//!   own for this rule; a dedicated row would be a change to chapter 13 (PHS.52). A binding with no chord is
//!   `EV4:input.binding_prefix_conflict` once, since it is a prefix of every sequence, and takes no part in the pair
//!   checks.
//! - The allowlist (EV4.6; K4, fix-first item 2): a repeat runs a binding only when its row's H-08 `repeat` flag is set,
//!   read from the layer's table at the repeat. The stored table sets it on exactly [`REPEAT_ALLOWLIST`] (a test pins
//!   this); an overlay row's flag is always false.
//! - The repeat flag's source (EV4.6; R-18): a key-down is a repeat when the platform flags it or when the router still
//!   holds its key (a key-down with no key-up or focus loss since). A lost key-up therefore costs one press and never
//!   doubles a state change. A repeat keeps the binding its key-down resolved, resolves its relative names again at its
//!   own event and never starts another command. A repeat of a key the router does not hold is dropped, so after a
//!   focus loss nothing repeats until a fresh key-down.
//! - Focus (EV4.6): focus loss and, as a draft extension, any focus change inside the window release the held keys and
//!   the pending prefix group.
//! - The text field (EV4.4; C2): it takes every key-down that typed a character (AltGr levels included), a character
//!   key or Enter with no modifier but Shift, the editing keys with Ctrl or Shift, IME composition, and Ctrl+A, C, V,
//!   X, Y and Z, with or without Shift, by the character the active layout gives the key. Every other key goes to the
//!   global map, except that a text field keeps a global key bound to a commit-kind row: the commit works from any
//!   focus outside a text field (DT-M1 C4).
//! - The list or view (EV4.4; C2; DT-J1 O9): the arrows (`list.move` to the previous or next entry, held at the ends),
//!   Enter (`object.inspect` of the selected entry), Digit1 to Digit9 and Digit0 (`list.row.copy` of rows 1 to 10)
//!   and Tab or Shift+Tab, which is focus movement inside the shell and has no H-08 row.
//! - The global map ([`default_specs`]): `Ctrl+Enter` and `Ctrl+Shift+Enter` preview and execute (0.4's keys);
//!   `Ctrl+'z'` and `Ctrl+'y'` undo and redo; `Escape` closes the innermost transient layer (`transient.close`) and,
//!   before that, a pending prefix group (C4, K1), so it never cancels a preview; `Ctrl+Shift+Backspace` cancels the
//!   presented preview; `F9` toggles the on-screen keyboard (0.4's key); the prefix group `Ctrl+KeyG` then `KeyF` or
//!   `KeyI` toggles the frame display or the inverse; the arrows and `Equal`/`Minus` rotate and zoom the `global`
//!   view's camera; `Alt+ArrowLeft`/`Alt+ArrowRight` scrub the presented preview; `KeyQ`, `KeyW`, `KeyE` and `KeyR`
//!   select grip slots 0 to 3; `KeyJ`, `KeyK` and `KeyL`, with Shift for the inverse, are three twist keys, which run
//!   `twist.apply` in the live destination and `operation.phase.append_twist` in the draft one (C4, C7). The full slot
//!   block and the seven axis keys (K2) wait for G6.
//! - Placeholder values: the camera rotations and zoom steps and the camera's view (`"global"`) are fixed JSON
//!   placeholders.
//! - The twist keys' exact requests (A6; KN3.7): a twist key types one item of the twist menu the host pins, pressed
//!   directly ([`Arg::MenuItem`]). It sends the grip's cap as `cap` and KN3.7's binding without its cap as
//!   `rotation`: the request `LX9B1|M|<set>|<order>|<index>`, the operation `menu_direct` and the menu's two digests,
//!   in `Binding::to_json`'s form. The host sets those digests in [`Context::menu`] when it pins the menu in the layer
//!   (`CommandLayer::pin_menu`), since the layer gives no read of its pin; with none, a twist key is refused before
//!   any envelope with `KN5:input.argument_form`. [`TWIST_KEYS`] maps `KeyJ`, `KeyK` and `KeyL` to items 1, 2 and 3
//!   and their Shift chords to the inverse items 1, 5 and 9: under the I_a menu's order, at every cap c, the
//!   half-turn `[2c+1]` and the third-turns `[2c+2]` and `[2c+1, 2c+2]`, and their inverses. An item number names a
//!   rotation only under the order the binding names. The keys act in the cap's fixed frame, not in a slot's
//!   transported frame (D11), and K2's other four axes (items 4 and 7 with their inverses 8 and 6, and the half-turns
//!   10 and 11) have no key. Which key types which exact request settles nothing in the chapters: G6 owns the keys,
//!   K2's seven axis keys among them.
//! - Jumble on (EV4.4; DT-M1 C6): the router reads no Jumble mode, since EV4.4 names the destination as the one input
//!   mode. With Jumble on, a twist key still sends `twist.apply`, which the layer refuses
//!   (`KN5:input.retained_tool_needs_jumble_off`). C6's menu-twist row, `jumble.twist.menu`, takes the same rotation
//!   as its `letter`, with the cap and the revision, but no key binds that row yet.
//! - A live twist's release (BND.4, BND.5; no preview for a live twist, owner answer of 6 November 2026): the solver
//!   releases a live twist with the twist key's key-down, the one deliberate action that commits it. Its key-up and
//!   its repeats release nothing more, and a refused key-down (busy lane, no menu, no grip) releases nothing.
//! - A relative name with nothing to resolve to (no draft, grip, phase, list, entry or row) refuses the press before
//!   any envelope with `KN5:input.argument_form`, as `Shell::press` does for a missing draft; no presented preview is
//!   `KN5:stale.no_fresh_preview` and no undo target `KN5:stale.undo_superseded`.
//! - The router changes no workspace or view state itself: the grip, the destination, the selected phase, the list
//!   selection and the scrub position change when their rows run, and the headless layer does not build those rows
//!   yet (`KN5:input.row_not_built`). Tests set them directly; [`Context::rebind`] stands in for `grip.rebind`
//!   (EV4.8).
//! - Prefix groups (K1): a pending group has no time-out; Escape closes it; a key-down that does not continue it closes
//!   it and runs nothing; a key that a capture field, text field or list takes also closes it, so in a text field a
//!   character key ends a group and is typed. [`resolve_binding`] therefore answers a third way besides EV4.3's
//!   binding or unhandled: [`Resolution::Prefix`], for a sequence that is a proper prefix of a bound one.
//! - Busy input (EV4.6; DT-M1 Q6 (a)): the router holds no queue. A key-down that reaches a busy lane gets the layer's
//!   refusal (`KN5:busy.commit_in_flight`, `KN7:busy.lane_full`) as its receipt, and nothing runs later on its behalf.

#![forbid(unsafe_code)]

pub mod binding;
pub mod chord;
pub mod keymap;
pub mod layout;
pub mod router;

pub use binding::{resolve_binding, Arg, Binding, Destination, Resolution, RoutingClass, RoutingContext};
pub use chord::{Chord, Key, Mods};
pub use keymap::{check, default_specs, BindingSpec, ChordIntent, Finding, KeyMap, LIST_ROWS, TWIST_KEYS};
pub use layout::Layout;
pub use router::{Context, Dispatch, DropReason, Entry, Focus, KeyEvent, ListFocus, Routed, Router, Slot};

/// EV4.6's repeat allowlist: the camera, list-movement and preview-scrub rows.
pub const REPEAT_ALLOWLIST: [&str; 4] = ["camera.rotate", "camera.zoom", "list.move", "view.preview.scrub"];
