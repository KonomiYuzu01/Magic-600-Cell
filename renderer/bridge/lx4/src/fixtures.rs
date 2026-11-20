//! The fixture folder that `c600-lx4-fixtures --out` writes (MANIFEST.json, EDITS.json, S/M/L and W/W00 to W49),
//! read and checked as lx4.py checks it: both formats, every module's digest, every edit's baseline digest, and the
//! digest of every edit's result. The editor starts each sample from these bytes.

use std::fs;
use std::path::{Component, Path, PathBuf};

use c600_kernel::json::{self, Value};
use c600_lx4::{Fixtures, Module, Replacement, EDIT_FORMAT, FORMAT};
use c600_syntax::sha256_hex;

/// One edit of the script, as EDITS.json lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub target: String,
    pub kind: String,
    pub base_sha256: String,
    pub replacements: Vec<Replacement>,
    pub result_sha256: String,
    /// The completion trigger's byte offset in the result.
    pub cursor: Option<usize>,
}

impl Edit {
    /// The inputs lx4.py posts for this edit after its Reset: per replacement one Select, then one WM_CHAR per UTF-16
    /// unit of the insert, or one VK_BACK key-down when the insert is empty.
    pub fn inputs(&self) -> usize {
        self.replacements.iter().map(|r| 1 + r.insert.encode_utf16().count().max(1)).sum()
    }
}

/// The checked fixtures: the module set and the script.
#[derive(Clone, Debug)]
pub struct Loaded {
    pub fixtures: Fixtures,
    pub edits: Vec<Edit>,
    /// W00's index in the workspace, where the exported-signature edit reaches the dependents.
    pub w00: usize,
}

/// Applies an edit's replacements to its baseline in descending offset order, as lx4.py posts them.
pub fn apply(base: &str, replacements: &[Replacement]) -> Result<String, String> {
    let mut out = base.to_string();
    let mut order: Vec<&Replacement> = replacements.iter().collect();
    order.sort_by_key(|r| std::cmp::Reverse(r.offset));
    for r in order {
        let end = r.offset + r.delete.len();
        if out.get(r.offset..end) != Some(r.delete.as_str()) {
            return Err(format!("the preimage at byte {} differs", r.offset));
        }
        out.replace_range(r.offset..end, &r.insert);
    }
    Ok(out)
}

fn read_json(folder: &Path, name: &str) -> Result<Value, String> {
    let bytes = fs::read(folder.join(name)).map_err(|error| format!("{name} cannot be read: {error}"))?;
    json::parse(&bytes).map_err(|error| format!("{name} is not JSON: {error}"))
}

fn text<'a>(value: &'a Value, key: &str, what: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| format!("{what} has no text `{key}`"))
}

fn number(value: &Value, key: &str, what: &str) -> Result<usize, String> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("{what} has no count `{key}`"))
}

fn list<'a>(value: &'a Value, key: &str, what: &str) -> Result<&'a [Value], String> {
    value.get(key).and_then(Value::as_array).ok_or_else(|| format!("{what} has no list `{key}`"))
}

/// A path the manifest names, inside the folder: relative, with no `..`, root or prefix.
fn inside(folder: &Path, path: &str) -> Result<PathBuf, String> {
    if path.is_empty() || !Path::new(path).components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(format!("the manifest names a path outside the folder: {path}"));
    }
    Ok(folder.join(path))
}

/// Reads and checks the fixture folder.
pub fn load(folder: &Path) -> Result<Loaded, String> {
    let manifest = read_json(folder, "MANIFEST.json")?;
    if manifest.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(format!("MANIFEST.json is not {FORMAT}"));
    }
    let mut singles = Vec::new();
    let mut workspace: Vec<Module> = Vec::new();
    for entry in list(&manifest, "modules", "MANIFEST.json")? {
        let name = text(entry, "name", "a manifest module")?;
        let path = text(entry, "path", name)?;
        let bytes = fs::read(inside(folder, path)?).map_err(|error| format!("{path} cannot be read: {error}"))?;
        if sha256_hex(&bytes) != text(entry, "sha256", name)? {
            return Err(format!("{path}: the digest differs from the manifest's"));
        }
        let source = String::from_utf8(bytes).map_err(|_| format!("{path} is not UTF-8"))?;
        let mut imports = Vec::new();
        for import in list(entry, "imports", name)? {
            let imported = import.as_str().ok_or_else(|| format!("{name} lists an import that is not text"))?;
            // repin needs every import to come earlier in the workspace.
            let index = workspace
                .iter()
                .position(|m| m.name == imported)
                .ok_or_else(|| format!("{name} imports {imported}, which is no earlier W module"))?;
            imports.push(index);
        }
        let module = Module {
            name: name.into(),
            module: text(entry, "module", name)?.into(),
            source,
            imports,
            fragments: vec![],
        };
        if name.starts_with('W') {
            workspace.push(module);
        } else if module.imports.is_empty() {
            singles.push(module);
        } else {
            return Err(format!("{name} is no W module but imports"));
        }
    }
    let w00 = workspace.iter().position(|m| m.name == "W00").ok_or("the manifest has no W00")?;
    let fixtures = Fixtures { singles, workspace };

    let script = read_json(folder, "EDITS.json")?;
    if script.get("format").and_then(Value::as_str) != Some(EDIT_FORMAT) {
        return Err(format!("EDITS.json is not {EDIT_FORMAT}"));
    }
    let mut edits = Vec::new();
    for (index, entry) in list(&script, "edits", "EDITS.json")?.iter().enumerate() {
        let what = format!("edit {index}");
        let mut replacements = Vec::new();
        for r in list(entry, "replacements", &what)? {
            replacements.push(Replacement {
                offset: number(r, "offset", &what)?,
                delete: text(r, "delete", &what)?.into(),
                insert: text(r, "insert", &what)?.into(),
            });
        }
        let edit = Edit {
            target: text(entry, "target", &what)?.into(),
            kind: text(entry, "kind", &what)?.into(),
            base_sha256: text(entry, "base_sha256", &what)?.into(),
            replacements,
            result_sha256: text(entry, "result_sha256", &what)?.into(),
            cursor: entry.get("cursor").and_then(Value::as_i64).and_then(|n| usize::try_from(n).ok()),
        };
        let module =
            fixtures.get(&edit.target).ok_or_else(|| format!("{what} edits {}, which is not listed", edit.target))?;
        if module.sha256() != edit.base_sha256 {
            return Err(format!("{what} ({} {}): the baseline digest differs", edit.target, edit.kind));
        }
        let result = apply(&module.source, &edit.replacements).map_err(|e| format!("{what}: {e}"))?;
        if sha256_hex(result.as_bytes()) != edit.result_sha256 {
            return Err(format!("{what} ({} {}): the result digest differs", edit.target, edit.kind));
        }
        edits.push(edit);
    }
    Ok(Loaded { fixtures, edits, w00 })
}
