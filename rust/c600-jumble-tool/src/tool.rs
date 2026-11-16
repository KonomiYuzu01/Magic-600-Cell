//! The built-in jumbling tool: its source, its entries, its module manifest (BND.27, BND.37) and its admission under
//! the built-in column of chapter 07 section 8, through `c600_tools::tool`.

use std::collections::BTreeSet;

use c600_digest::sha256_hex;
use c600_tools::tool::{BoundaryClass, Manifest, Tool};

/// The tool's source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const SOURCE: &str = include_str!("jumble_checker.6cs");
/// The tool's module.
pub const MODULE: &str = "jumble_checker";
/// The entry the solver invokes with the twist to check: a literal word of one letter.
pub const CHECK_TWIST: &str = "check_named_twist";
/// The entry the solver invokes with the word to check (LB3.8's `check_named_word`).
pub const CHECK_WORD: &str = "check_named_word";
/// The tool's entries, in source order.
pub const ENTRIES: [&str; 2] = [CHECK_TWIST, CHECK_WORD];
/// The command-layer row every entry's target names.
pub const ROW: &str = "operation.preview";
/// The LB6.5 candidate row the tool belongs to.
pub const CANDIDATE: &str = "Named twist/word checker and kept checks";
/// The declared class (BND.37): LB6.5's candidate class for [`CANDIDATE`].
pub const CLASSES: [BoundaryClass; 1] = [BoundaryClass::Checking];

/// The manifest of one entry of the jumbling tool: the tool's class and row set with that entry. `None` for a name
/// that is not one of [`ENTRIES`].
pub fn manifest(entry: &str) -> Option<Manifest> {
    let entry = ENTRIES.into_iter().find(|name| *name == entry)?;
    Some(Manifest {
        tool: "jumbling tool",
        candidate: CANDIDATE,
        module: MODULE,
        entry,
        classes: &CLASSES,
        rows: &[ROW],
        source_sha256: sha256_hex(SOURCE.as_bytes()),
        principle_record: false,
    })
}

/// The manifests of every entry, in source order.
pub fn manifests() -> Vec<Manifest> {
    ENTRIES.into_iter().filter_map(manifest).collect()
}

/// The tool's row set: the union of its entries' rows.
pub fn rows() -> BTreeSet<&'static str> {
    manifests().iter().flat_map(|manifest| manifest.rows.iter().copied()).collect()
}

/// One entry of the jumbling tool, parsed, lowered, admitted under the built-in column in the solving profile and
/// loaded (`c600_tools::tool::Tool::new`): it fails if the entry is not one of [`ENTRIES`], if the built-in column
/// does not admit it, or if the rows its proposals name are not the manifest's.
pub fn tool(entry: &str) -> Result<Tool, String> {
    let manifest = manifest(entry).ok_or_else(|| format!("the jumbling tool has no entry `{entry}`"))?;
    Tool::new(SOURCE, manifest)
}
