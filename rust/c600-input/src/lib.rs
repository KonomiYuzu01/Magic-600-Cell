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
//! G6 has no keymap table, chapter 05 owns the persisted key-map forms, and the command table, frozen at LP3 (PHS.53),
//! holds no 1.0 key map; these choices stay drafts until chapter 05's key-map forms settle them.
//!
//! - The key map's model (routing class names, the chord form, the two layout tables, EV4.7's checks, the default
//!   global map, its placeholder values and the twist keys' exact requests) lives in `c600_command::keys` since LP4-8,
//!   which keeps its drafts; this crate re-exports it in its old modules ([`binding`], [`chord`], [`keymap`],
//!   [`layout`]), so the shell's key route and this router share one checked map.
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
//! - The twist keys' requests at the press (A6; KN3.7): a twist key's [`Arg::MenuItem`] sends the grip's cap as `cap`
//!   and KN3.7's binding without its cap as `rotation`: the request `LX9B1|M|<set>|<order>|<index>`, the operation
//!   `menu_direct` and the menu's two digests, in `Binding::to_json`'s form. The host sets those digests in
//!   [`Context::menu`] when it pins the menu in the layer (`CommandLayer::pin_menu`), since the layer gives no read of
//!   its pin; with none, a twist key is refused before any envelope with `KN5:input.argument_form`. The keys act in the
//!   cap's fixed frame, not in a slot's transported frame (D11), and K2's other four axes have no key.
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
//!   yet (`KN5:input.row_not_built`). Tests set them directly.
//! - The applied grip-slot map (EV4.8; DT-M1 K2, K6; LP4-8): the layer holds it, and only the solver's `grip.rebind`
//!   changes it ([`c600_command::grip`]). Before the first rebind the host's default map applies, the slot map a
//!   [`Context`] is made with; [`Router::slots`] reads the applied map at each press, so a slot key resolves against
//!   the last rebind and nothing else (camera, filter, twist, focus or a restart) derives a new one.
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
