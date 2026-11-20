//! Saved sets (`set.save`; U1-057, U4-037, PS1.28; LP2 draft): the exact set an entry's record names, its digest and
//! the record the layer journals for it.
//!
//! `set.save`'s entry (the representations tool's `save_set`) writes the one record `set.save`, `{kind, members,
//! name}`, as the runtime writes a value (`c600-json-1`, FP.5): an integer is `{"int": "<decimal>"}`. When the job
//! completes, the layer reads that record ([`SavedSet::from_entry`]) and writes, in the completion's one durable
//! action, the saved set's own record instead: `set/<name>` ([`saved_set_record`]) holding [`SavedSet::payload`]. The
//! completion's result reference carries the exact set's digest ([`SavedSet::digest`]), not the output's.
//!
//! - The exact set is `{kind, members, model}`: the kind, `identity` or `position`; the members ascending and distinct,
//!   each a piece or a position below the kernel's `PIECES`; and the identity of the model the job read. The name and
//!   the capture revision are outside it, so saves of the same members of one kind under one model share a digest.
//! - Its digest is the SHA-256 of its FP.5 bytes as the runtime encodes the value: sorted keys, compact separators,
//!   literal Unicode with the runtime's escapes, integers as `{"int": "<decimal>"}` and one terminal line feed.
//!   [`SavedSet::digest`] is a pure function of the entry's record and the model identity, so a host computes the same
//!   digest without the layer.
//! - The record the layer writes holds the exact set's fields, the name, and the capture revision: KN0.2's
//!   `RevisionRef` of the revision the job read, its counters as integers in the same value form.
//! - Names under `set/` are saved sets. Only a `set.save` completion writes them; `workspace.set` and `worksheet.save`
//!   refuse them with `KN5:input.argument_form`.
//! - A save under a name that holds a saved set replaces it only when the invocation carries `replace: true`, a draft
//!   boolean beside the row's table arguments (U1-057: named-set replacement stays explicit). Otherwise it is refused
//!   with the draft code `KN5:input.saved_set_exists`: at admission, with no job, and at completion, writing nothing,
//!   when another save took the name meanwhile.

use c600_kernel::json::{self, Value};

use crate::receipt::{code, Error, Revision};

/// The row whose completion writes a saved set.
pub const SET_SAVE_ROW: &str = "set.save";
/// The one record the row's entry writes: `{kind, members, name}`.
pub const SET_SAVE_RECORD: &str = "set.save";
/// The prefix of a saved set's record name.
pub const SAVED_SET_PREFIX: &str = "set/";
/// The draft boolean argument that lets a save replace the saved set of the same name.
pub const REPLACE: &str = "replace";

/// The kinds of a saved set, as `set.save`'s `kind` names them.
const KINDS: [&str; 2] = ["identity", "position"];

/// The record name of the saved set `name`.
pub fn saved_set_record(name: &str) -> String {
    format!("{SAVED_SET_PREFIX}{name}")
}

/// A saved set as its entry's record names it, with its members in canonical order: ascending and distinct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedSet {
    pub name: String,
    pub kind: &'static str,
    pub members: Vec<u64>,
}

impl SavedSet {
    /// Reads the entry's record `{kind, members, name}` in the runtime's value form. A record of another form is
    /// refused with `KN2:input.named_object_invalid`: other or missing fields, a kind other than `identity` or
    /// `position`, a name that is not text, or a member that is not `{"int": "<decimal>"}` in canonical decimal for a
    /// piece or a position below the kernel's `PIECES`. An empty list and repeated members pass, as at admission.
    pub fn from_entry(record: &Value) -> Result<SavedSet, Error> {
        let invalid = |detail: &str| {
            Error::new(code::NAMED_OBJECT_INVALID, format!("the entry's record `{SET_SAVE_RECORD}` {detail}"))
        };
        let Some(fields) = record.as_object().filter(|fields| fields.len() == 3) else {
            return Err(invalid("is not `{kind, members, name}`"));
        };
        let kind = fields.get("kind").and_then(Value::as_str).and_then(|kind| KINDS.into_iter().find(|k| *k == kind));
        let Some(kind) = kind else {
            return Err(invalid("names neither the kind `identity` nor `position`"));
        };
        let Some(name) = fields.get("name").and_then(Value::as_str) else {
            return Err(invalid("has no text name"));
        };
        let Some(members) = fields.get("members").and_then(Value::as_array) else {
            return Err(invalid("has no list of members"));
        };
        let Some(mut members) = members.iter().map(member).collect::<Option<Vec<u64>>>() else {
            return Err(invalid("has a member that is not a piece or a position"));
        };
        members.sort_unstable();
        members.dedup();
        Ok(SavedSet { name: name.to_owned(), kind, members })
    }

    /// The exact set `{kind, members, model}` in the runtime's value form.
    pub fn exact(&self, model: &str) -> Value {
        json::object([
            ("kind", Value::String(self.kind.to_owned())),
            ("members", Value::Array(self.members.iter().map(|&n| int(n)).collect())),
            ("model", Value::String(model.to_owned())),
        ])
    }

    /// The exact set's FP.5 bytes ([`SavedSet::exact`]): the bytes the runtime writes for the value.
    pub fn bytes(&self, model: &str) -> Vec<u8> {
        let mut out = Vec::new();
        write(&self.exact(model), &mut out);
        out.push(b'\n');
        out
    }

    /// The exact set's digest: the SHA-256 of [`SavedSet::bytes`], in lowercase hex.
    pub fn digest(&self, model: &str) -> String {
        c600_digest::sha256_hex(&self.bytes(model))
    }

    /// The record the layer writes for the saved set: the exact set's fields, the name, and the capture revision
    /// `revision` as KN0.2's `RevisionRef` with its counters in value form.
    pub fn payload(&self, model: &str, revision: &Revision) -> Value {
        let Value::Object(mut fields) = self.exact(model) else { unreachable!("the exact set is an object") };
        fields.insert("name".to_owned(), Value::String(self.name.clone()));
        let captured = match revision.to_json() {
            Value::Object(revision) => {
                let value = |value: Value| match value {
                    Value::Int(n) => int(n.unsigned_abs()),
                    other => other,
                };
                Value::Object(revision.into_iter().map(|(key, field)| (key, value(field))).collect())
            }
            other => other,
        };
        fields.insert("revision".to_owned(), captured);
        Value::Object(fields)
    }
}

/// An integer as the runtime writes it: `{"int": "<decimal>"}`.
fn int(n: u64) -> Value {
    json::object([("int", Value::String(n.to_string()))])
}

/// A member in the runtime's value form: `{"int": "<decimal>"}` with no sign and no leading zero, for a piece or a
/// position below the kernel's `PIECES`.
fn member(value: &Value) -> Option<u64> {
    let digits = value.as_object().filter(|fields| fields.len() == 1)?.get("int")?.as_str()?;
    let canonical =
        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && (digits == "0" || !digits.starts_with('0'));
    let n: u64 = digits.parse().ok().filter(|_| canonical)?;
    usize::try_from(n).is_ok_and(|n| n < c600_kernel::PIECES).then_some(n)
}

/// FP.5's bytes of a JSON value without the terminal line feed: object keys in the map's order, which is Unicode scalar
/// order, compact separators, and strings with the runtime's escapes.
fn write(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Int(n) => out.extend_from_slice(n.to_string().as_bytes()),
        Value::Number(lexeme) => out.extend_from_slice(lexeme.as_bytes()),
        Value::String(text) => string(text, out),
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write(item, out);
            }
            out.push(b']');
        }
        Value::Object(fields) => {
            out.push(b'{');
            for (i, (key, item)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                string(key, out);
                out.push(b':');
                write(item, out);
            }
            out.push(b'}');
        }
    }
}

/// A string as FP.5 writes it: literal Unicode, `"` and `\` escaped, `\b \t \n \f \r`, and other controls below U+0020
/// as lowercase `\u00xx`.
fn string(text: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in text.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{8}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{c}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes()),
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}
