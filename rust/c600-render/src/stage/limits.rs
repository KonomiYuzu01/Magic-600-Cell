//! The host's stage limits file (draft): one JSON object with the members of [`StageLimits`] and an optional `note`.
//! A member of another name, a weight of an operation outside the stage's table, a negative or nonintegral number and
//! a missing member are refused, so a misspelt limit cannot fall back to a default.

use std::collections::BTreeMap;

use c600_check::StageLimits;
use c600_ir::StageOp;
use c600_kernel::json::{parse, Value};

/// The operations a weight may name: the four ordinary stage primitives, `display_if`, `index` and the
/// [`StageOp`] names.
fn weight_names() -> Vec<&'static str> {
    let mut names = vec!["approx_add", "approx_less", "approx_and", "approx_not", "display_if", "index"];
    names.extend(StageOp::ALL.iter().map(|op| op.name()));
    names
}

fn natural(value: &Value, what: &str) -> Result<u64, String> {
    value.as_i64().and_then(|n| u64::try_from(n).ok()).ok_or_else(|| format!("`{what}` is no natural number"))
}

/// Read a limits file.
pub fn parse_limits(bytes: &[u8]) -> Result<StageLimits, String> {
    let value = parse(bytes).map_err(|e| format!("the limits file is no JSON: {e}"))?;
    let Some(members) = value.as_object() else { return Err("the limits file is no JSON object".into()) };
    const KEYS: [&str; 7] = ["tier", "weights", "max_ops", "max_overdraw", "max_loop_count", "max_loop_depth", "note"];
    if let Some(key) = members.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!("the limits file has the unknown member `{key}`"));
    }
    let member = |key: &str| members.get(key).ok_or_else(|| format!("the limits file has no `{key}`"));
    if let Some(note) = members.get("note") {
        note.as_str().ok_or("`note` is no string")?;
    }
    let tier = u8::try_from(natural(member("tier")?, "tier")?).map_err(|_| "`tier` is out of range".to_string())?;
    let Some(table) = member("weights")?.as_object() else { return Err("`weights` is no JSON object".into()) };
    let known = weight_names();
    let mut weights = BTreeMap::new();
    for (name, w) in table {
        if !known.contains(&name.as_str()) {
            return Err(format!("`weights` names `{name}`, which is no stage operation"));
        }
        weights.insert(name.clone(), natural(w, &format!("weights.{name}"))?);
    }
    let max_loop_depth = u32::try_from(natural(member("max_loop_depth")?, "max_loop_depth")?)
        .map_err(|_| "`max_loop_depth` is out of range".to_string())?;
    Ok(StageLimits {
        tier,
        weights,
        max_ops: natural(member("max_ops")?, "max_ops")?,
        max_overdraw: natural(member("max_overdraw")?, "max_overdraw")?,
        max_loop_count: natural(member("max_loop_count")?, "max_loop_count")?,
        max_loop_depth,
    })
}
