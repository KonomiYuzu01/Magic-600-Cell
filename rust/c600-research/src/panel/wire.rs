//! The panel's values on the command layer's wire: an invocation from the panel's JSON, a job's arguments read back,
//! and the small JSON builders the answers use. The panel's values are `c600-session`'s FP.5 JSON; the command layer's
//! are the kernel's, read through the module's `kernel_value` macro.

use std::collections::BTreeMap;

use c600_command::{code, CommandLayer, Error, Receipt, Revision, Shell, Store};
use c600_session::persist::codec::{canonical, parse, Json};

/// A panel control's invocation: `row` with `args`, sent by the shell through the command layer (`Shell::invoke`, which
/// declares the shell's budget for a served row and mints no token for these rows).
pub fn invoke<S: Store>(
    shell: &mut Shell,
    layer: &mut CommandLayer<S>,
    row: &str,
    args: &Json,
) -> Result<Receipt, Error> {
    let value = kernel_value!(args)?;
    shell.invoke(layer, row, value)
}

/// A command-layer value, given as its canonical text, as the panel's JSON.
pub(crate) fn json_of(text: &str) -> Option<Json> {
    parse(text.as_bytes())
}

/// The command layer's revision as the panel's JSON (KN0.2's nine keys).
pub(crate) fn revision_json(revision: &Revision) -> Json {
    json_of(&revision.to_json().canonical()).unwrap_or(Json::Null)
}

/// The SHA-256 of a value's FP.5 bytes.
pub(crate) fn sha(value: &Json) -> String {
    c600_digest::sha256_hex(&canonical(value))
}

pub(crate) fn object(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

pub(crate) fn list(items: impl IntoIterator<Item = Json>) -> Json {
    Json::List(items.into_iter().collect())
}

pub(crate) fn int(n: usize) -> Json {
    Json::Int(i64::try_from(n).unwrap_or(i64::MAX))
}

pub(crate) fn opt_text(value: Option<&str>) -> Json {
    value.map_or(Json::Null, Json::text)
}

pub(crate) fn texts<'a>(values: impl IntoIterator<Item = &'a String>) -> Json {
    list(values.into_iter().map(|value| Json::text(value)))
}

/// Bytes the host holds as UTF-8 text (FP.5 records are), or null.
pub(crate) fn utf8(bytes: &[u8]) -> Json {
    std::str::from_utf8(bytes).map_or(Json::Null, Json::text)
}

fn form(detail: String) -> Error {
    Error::new(code::ARGUMENT_FORM, detail)
}

/// A required text argument.
pub(crate) fn arg_text<'a>(args: &'a Json, name: &str) -> Result<&'a str, Error> {
    args.get(name).and_then(Json::as_text).ok_or_else(|| form(format!("`{name}` is not text")))
}

/// An optional text argument.
pub(crate) fn arg_opt_text<'a>(args: &'a Json, name: &str) -> Result<Option<&'a str>, Error> {
    match args.get(name) {
        None | Some(Json::Null) => Ok(None),
        Some(_) => arg_text(args, name).map(Some),
    }
}

/// An optional natural number.
pub(crate) fn arg_opt_nat(args: &Json, name: &str) -> Result<Option<u64>, Error> {
    match args.get(name) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Int(n)) if *n >= 0 => Ok(Some(*n as u64)),
        Some(_) => Err(form(format!("`{name}` is not a natural number"))),
    }
}

/// A required natural number.
pub(crate) fn arg_nat(args: &Json, name: &str) -> Result<u64, Error> {
    arg_opt_nat(args, name)?.ok_or_else(|| form(format!("`{name}` is not a natural number")))
}

/// A required SHA-256 in FP.8's form: 64 lowercase hex digits.
pub(crate) fn arg_sha<'a>(args: &'a Json, name: &str) -> Result<&'a str, Error> {
    let text = arg_text(args, name)?;
    if c600_archive::is_sha256(text) {
        Ok(text)
    } else {
        Err(form(format!("`{name}` is not 64 lowercase hex digits")))
    }
}

/// A required `List<Text>`: a non-empty list of distinct texts.
pub(crate) fn arg_texts(args: &Json, name: &str) -> Result<Vec<String>, Error> {
    let Some(Json::List(items)) = args.get(name) else {
        return Err(form(format!("`{name}` is not a list")));
    };
    let mut out: Vec<String> = Vec::new();
    for item in items {
        let text = item.as_text().ok_or_else(|| form(format!("an item of `{name}` is not text")))?;
        if out.iter().any(|seen| seen == text) {
            return Err(form(format!("`{name}` names `{text}` twice")));
        }
        out.push(text.to_owned());
    }
    if out.is_empty() {
        return Err(form(format!("`{name}` is empty")));
    }
    Ok(out)
}

/// A required `Map<Text,Text>`: an object whose values are texts.
pub(crate) fn arg_map(args: &Json, name: &str) -> Result<BTreeMap<String, String>, Error> {
    let Some(Json::Object(fields)) = args.get(name) else {
        return Err(form(format!("`{name}` is not an object")));
    };
    fields
        .iter()
        .map(|(key, value)| match value.as_text() {
            Some(text) => Ok((key.clone(), text.to_owned())),
            None => Err(form(format!("`{name}.{key}` is not text"))),
        })
        .collect()
}
