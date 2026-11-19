//! H-08's two rows for further views, `view.open` and `view.close` (EV3.2, RPI.81 to RPI.83; LP2 draft).
//!
//! The layer admits the rows and the host's view model ([`Views`]) does the rest: which views are open, Global and
//! Local, the view lane and RPI.82's budget. LP3's host implements it with `c600-view`'s `ViewModel::open_view` and
//! `close_view`, which the layer cannot depend on. Before the host is called, in this order: the host's view model
//! (`KN5:input.row_not_built` without one), the argument forms (`KN5:input.argument_form`), a `ViewSourceRef`'s
//! overlay row, which must be an overlay row of kind `view` (KN5.3) with arguments that fit it
//! (`KN5:input.argument_form`), current where they name a revision and resolved where they name a `FilterRef`, as for
//! an invocation of that row, and the caller: either row runs only inside a command the solver invoked, so a run
//! that is not inside one, as a recomputation started by a revision change is not, is refused with
//! `LG3:capability.not_granted`, and a view cannot open views (RPI.83). The host's refusals
//! (`EV3:input.view_id_in_use`, `EV3:input.view_not_closable`, `EV3:resource_limit.display_plan_ceiling`) reach the
//! receipt unchanged. An admitted open or close answers `accepted` with no job, and that is the layer's final answer
//! for either row: KN5.21 has no completion kind for them, and what follows (the view's first run, a close's cancelled
//! lane work) is the view model's, under its own command IDs.

use std::collections::BTreeMap;

use c600_kernel::json::Value;

use crate::receipt::Error;

/// `view.open`.
pub const VIEW_OPEN_ROW: &str = "view.open";
/// `view.close`.
pub const VIEW_CLOSE_ROW: &str = "view.close";

/// What fills a further view (RPI.83): `built_in(name)` or `overlay(row, module, arguments)`.
///
/// Its argument form (draft) is `{"built_in": Text}` or `{"overlay": {"row": Text, "module": Text, "arguments":
/// {..}}}`, `module` the 64 lowercase hex digits of the checked module's source digest and `arguments` the typed
/// arguments of the overlay row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewSourceRef {
    /// A built-in source by name.
    BuiltIn { name: String },
    /// An overlay row of kind `view`, its checked module's source digest and its typed arguments.
    Overlay { row: String, module: String, arguments: BTreeMap<String, Value> },
}

impl ViewSourceRef {
    /// The source `value` names, or `None` when it does not have the form.
    pub fn from_json(value: &Value) -> Option<ViewSourceRef> {
        let fields = value.as_object().filter(|fields| fields.len() == 1)?;
        if let Some(name) = fields.get("built_in") {
            return Some(ViewSourceRef::BuiltIn { name: name.as_str().filter(|name| !name.is_empty())?.to_owned() });
        }
        let overlay = fields.get("overlay")?.as_object().filter(|overlay| overlay.len() == 3)?;
        let row = overlay.get("row")?.as_str().filter(|row| !row.is_empty())?.to_owned();
        let module = overlay.get("module")?.as_str().filter(|module| hex_digest(module))?.to_owned();
        let arguments = overlay.get("arguments")?.as_object()?.clone();
        Some(ViewSourceRef::Overlay { row, module, arguments })
    }
}

/// A `ViewId` (LI.22's view namespace): a nonempty text.
pub(crate) fn view_id(value: &Value) -> bool {
    value.as_str().is_some_and(|view| !view.is_empty())
}

fn hex_digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The host's view model, as the command layer reaches it for `view.open` and `view.close` (EV3.2, RPI.83).
///
/// The layer calls it only for an invocation it has admitted, inside a command the solver invoked, and once per
/// command ID: a retry gets the stored receipt. `command` is the invocation's command ID.
pub trait Views: Send + Sync {
    /// Opens the further view `view` from `source`. An open view, Global and Local included, is
    /// `EV3:input.view_id_in_use`; opening past the view budget is `EV3:resource_limit.display_plan_ceiling` (RPI.82).
    fn open(&mut self, command: &str, view: &str, source: &ViewSourceRef) -> Result<(), Error>;

    /// Closes the further view `view`. Global, Local and an unknown view are `EV3:input.view_not_closable`.
    fn close(&mut self, command: &str, view: &str) -> Result<(), Error>;
}
