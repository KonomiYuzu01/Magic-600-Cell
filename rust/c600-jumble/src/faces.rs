//! Exposed-face geometry (render feature J10; JI.41, JI.45): the exact geometry producer's answer.
//!
//! JI.41's exposed-face meshes need two prerequisites: certified exact region and cut-cell geometry (face identity,
//! an exact-source digest, converted vertices and topology) and an admitted size profile. This edition has neither:
//! JI.41 and the renderer feature matrix (row J10) leave the geometry production and the size measurement to the EN2
//! and renderer lines. So the producer answers every request with JI.45's `KN2:input.exposed_geometry_unavailable`.
//! The refusal names the feature, the exact input it was asked about and each missing prerequisite, states that no
//! draft is retained, and gives JI.45's continuation. It invents no mesh, face, count or size, and the state is
//! unchanged: the call only reads it.

use std::collections::BTreeMap;

use c600_kernel::json::Value;

use crate::state::State;
use crate::{codes, Context, CONTRACT_REVISION};

/// The feature the refusal names: JI.41's "Exposed-face meshes", the renderer feature matrix's row J10.
pub const FEATURE: &str = "exposed_face_meshes";
/// The renderer feature matrix's row.
pub const RENDER_ROW: &str = "J10";
/// The prerequisites JI.41 names, in its order; this edition has none of them.
pub const MISSING: [&str; 2] = ["certified_geometry", "size_profile"];
/// JI.45's continuation for the code.
pub const CONTINUATION: &str = "report the feature unavailable; request it again only after certified geometry and \
                                an admitted size profile exist, or choose a supported feature";

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn obj(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect::<BTreeMap<_, _>>())
}

/// The exact input reference: the identity block's fields and the state's digest ([`State::digest`]).
fn source(ctx: &Context, state: &State) -> Value {
    let menu = state.menu();
    obj(vec![
        ("model_id", text(&ctx.model.model_id)),
        ("menu_id", menu.map_or(Value::Null, |m| text(&m.identity))),
        ("menu_order", menu.map_or(Value::Null, |m| text(&m.order))),
        ("contract_rev", text(CONTRACT_REVISION)),
        ("state_digest", text(&state.digest(ctx))),
    ])
}

/// The exposed faces of a configuration: in this edition always JI.45's refusal (module docs). The answer has the
/// shape of the other observations' refusals (`status`, `code`, `message`) and JI.45's context: `feature` and
/// `render_row`, `source`, `missing`, `draft` (null: no draft is retained) and `continuation`.
pub fn exposed_faces(ctx: &Context, state: &State) -> Value {
    obj(vec![
        ("status", text("refused")),
        ("code", text(codes::EXPOSED_GEOMETRY_UNAVAILABLE)),
        ("message", text("certified exposed-face geometry and an admitted size profile are absent")),
        ("feature", text(FEATURE)),
        ("render_row", text(RENDER_ROW)),
        ("source", source(ctx, state)),
        ("missing", Value::Array(MISSING.iter().map(|m| text(m)).collect())),
        ("draft", Value::Null),
        ("continuation", text(CONTINUATION)),
    ])
}
