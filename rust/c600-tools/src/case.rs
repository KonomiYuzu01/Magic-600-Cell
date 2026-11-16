//! The frozen oracle cases (`C600-ORACLE-CASE-draft`, `work/experiments/language-lx/inputs/oracle/`) as the LP3
//! source route reads them.

use std::path::Path;

use c600_kernel::archive;
use c600_kernel::json::{self, Value};

/// The case format the runners read.
pub const CASE_FORMAT: &str = "C600-ORACLE-CASE-draft";
/// The largest decompressed case the runners read.
pub const CASE_LIMIT: usize = 1 << 30;

/// Reads a case file, gzip or plain JSON.
pub fn read_document(path: &Path) -> Result<Value, String> {
    let bytes = std::fs::read(path).map_err(|_| "cannot read a case file".to_string())?;
    let bytes = if bytes.starts_with(&[0x1f, 0x8b]) {
        archive::gunzip(&bytes, CASE_LIMIT).map_err(|error| format!("unreadable gzip case: {error}"))?
    } else {
        bytes
    };
    json::parse(&bytes).map_err(|error| format!("malformed case JSON: {error}"))
}

/// One step of a case: its index, its recipe and what the oracle expects of it.
#[derive(Clone, Copy, Debug)]
pub struct Step<'a> {
    pub index: i64,
    pub recipe: &'a Value,
    pub expect: Option<&'a Value>,
}

/// A case's steps in index order. A missing step list, a malformed or repeated index and a missing recipe are errors.
pub fn steps(case: &Value) -> Result<Vec<Step<'_>>, String> {
    let items = case.get("steps").and_then(Value::as_array).filter(|steps| !steps.is_empty());
    let items = items.ok_or("missing steps")?;
    let mut ordered = Vec::with_capacity(items.len());
    for item in items {
        let index = item.get("index").and_then(Value::as_i64).filter(|&index| index >= 0);
        let index = index.ok_or("malformed step index")?;
        let recipe = item.get("recipe").ok_or("missing step recipe")?;
        ordered.push(Step { index, recipe, expect: item.get("expect") });
    }
    ordered.sort_by_key(|step| step.index);
    if ordered.windows(2).any(|pair| pair[0].index == pair[1].index) {
        return Err("duplicate step index".into());
    }
    Ok(ordered)
}
