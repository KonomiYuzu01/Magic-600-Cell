//! `twist.apply`'s exact rotation (draft; amendment A6, KN3.7 to KN3.9).
//!
//! The row's arguments carry KN3.7's request binding: `cap` names the cap, and `rotation` holds the binding's other
//! fields, `{"request", "operation", "menu"}`. Together they are the canonical binding `{request, cap, operation, menu}`
//! that [`Binding::from_json`] reads, so LI.5's request digest and the token's argument digest cover all of it. The
//! layer resolves the request against the twist menu the host pinned ([`crate::CommandLayer::pin_menu`]) with
//! `c600_jumble::rotation::resolve`, and on 600-cell-Full a resolved item becomes its retained word
//! (`c600_jumble::rotation::retained_word`, JI.35). The journal keeps the resolution's record beside the commit's
//! recipe ([`crate::Transaction::rotation`]); a reopen replays the recipe and runs neither the producer nor the chooser
//! again (KN3.9). `jumble.twist.menu` carries the same binding in its `letter` argument ([`binding_at`]).

use std::sync::Arc;

use c600_jumble::q5::matrix_from_json;
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::Value;
use c600_kernel::request::codes::ROTATION_REQUEST_MALFORMED;
use c600_kernel::request::{request_digest, Binding, Operation};

use crate::receipt::Error;

/// H-08's live-twist row (ONE.46, ONE.78).
pub const TWIST_ROW: &str = "twist.apply";

/// The fields of `twist.apply`'s `rotation` argument: KN3.7's binding without the `cap` the row names.
const ROTATION_FIELDS: [&str; 3] = ["menu", "operation", "request"];

/// The fields of a rotation record: [`c600_jumble::rotation::Resolution::to_json`]'s.
const RECORD_FIELDS: [&str; 5] = ["binding", "cap", "choice", "matrix", "request_digest"];

/// The twist menu the host pinned, with the exact model it is read in.
pub(crate) struct Twists {
    pub(crate) context: Arc<Context>,
    pub(crate) menu: Arc<Menu>,
}

/// KN3.7's binding from `twist.apply`'s arguments: `rotation`'s three fields and the row's `cap` ([`binding_at`]).
pub(crate) fn binding(args: &Value) -> Result<Binding, Error> {
    binding_at(args, "rotation")
}

/// KN3.7's binding from a twist row's arguments: argument `key`'s three fields (`rotation` for `twist.apply`, `letter`
/// for `jumble.twist.menu`; draft) and the row's `cap`. Anything else, an argument `key` that is not an object of
/// exactly those fields included, is refused with `KN2:input.rotation_request_malformed`, as [`Binding::from_json`]
/// refuses a malformed binding.
pub(crate) fn binding_at(args: &Value, key: &str) -> Result<Binding, Error> {
    let rotation = args.get(key).and_then(Value::as_object);
    let fits = |fields: &&std::collections::BTreeMap<String, Value>| {
        fields.len() == ROTATION_FIELDS.len() && ROTATION_FIELDS.iter().all(|field| fields.contains_key(*field))
    };
    let Some(rotation) = rotation.filter(fits) else {
        return Err(Error::new(
            ROTATION_REQUEST_MALFORMED,
            format!("`{key}` is KN3.7's binding without its cap: exactly `request`, `operation` and `menu`"),
        ));
    };
    let mut fields = rotation.clone();
    fields.insert("cap".to_owned(), args.get("cap").cloned().unwrap_or(Value::Null));
    Ok(Binding::from_json(&Value::Object(fields))?)
}

/// Whether `value` reads as the rotation record of a commit on 600-cell-Full: exactly the fields of
/// [`RECORD_FIELDS`]; a binding that [`Binding::from_json`] reads and that names a menu; the SHA-256 of the binding's
/// request bytes (KN3.8); the binding's cap; a `menu_direct` or `menu_drag` choice that matches the binding's
/// operation, with its index; and an exact matrix. Free mode never reaches 600-cell-Full, so a free choice does not
/// read.
pub(crate) fn record_reads(value: &Value) -> bool {
    let read = || -> Option<()> {
        let fields = value.as_object()?;
        (fields.len() == RECORD_FIELDS.len() && RECORD_FIELDS.iter().all(|field| fields.contains_key(*field)))
            .then_some(())?;
        let binding = Binding::from_json(&fields["binding"]).ok()?;
        (fields["request_digest"].as_str()? == request_digest(&binding.request)).then_some(())?;
        (fields["cap"].as_i64()? == i64::from(binding.cap)).then_some(())?;
        let choice = fields["choice"].as_object()?;
        (choice.get("index")?.as_i64()? >= 0).then_some(())?;
        let fits = match (choice.get("kind")?.as_str()?, &binding.operation) {
            ("menu_direct", Operation::MenuDirect) => choice.len() == 2,
            ("menu_drag", Operation::MenuDrag) => {
                choice.len() == 4 && choice.get("ties")?.as_array().is_some() && choice.contains_key("distance2")
            }
            _ => false,
        };
        fits.then_some(())?;
        matrix_from_json(&fields["matrix"]).ok().map(|_| ())
    };
    read().is_some()
}
