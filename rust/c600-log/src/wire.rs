//! The bytes of a 1.0 log: `c600-json-1` (FP.5), `c600-gzip-stored-1` (FP.7), the PS1.7 `u32le-base64` carrier, the
//! FP.8 `DigestRef` and the FP.9 limits. Reading is strict: a value is accepted only in its one canonical encoding.

use std::borrow::Cow;
use std::collections::BTreeMap;

use c600_kernel::archive::crc32;
use c600_kernel::json::{self, Value};

use crate::problem::{code, Gate, Problem};

/// FP.9 `record_stored`: the most bytes a stored record may have.
pub const RECORD_STORED: usize = 71_303_168;
/// FP.9 `record_decoded`: the most bytes a decoded record may have.
pub const RECORD_DECODED: usize = 67_108_864;
/// FP.9 `json_depth`: the deepest nesting of arrays and objects.
pub const JSON_DEPTH: usize = 64;
/// FP.7: a record of more decoded bytes than this is stored as gzip, any other as its raw bytes.
pub const GZIP_THRESHOLD: usize = 250_000;
/// FP.5: the largest bounded JSON integer.
pub const MAX_INTEGER: i64 = 9_007_199_254_740_991;

/// FP.7's fixed member header: no flags, no time, no extra fields, operating system 255.
const GZIP_HEADER: [u8; 10] = [0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0xff];
/// FP.7's stored block size.
const BLOCK: usize = 65_535;

fn size(at: &str, message: &str) -> Problem {
    Problem::new(code::PERSISTENCE_SIZE, Gate::Format, at, message)
}

fn noncanonical(at: &str, message: &str) -> Problem {
    Problem::new(code::NONCANONICAL, Gate::Format, at, message)
}

// ---------------------------------------------------------------------------------------------------------------
// c600-json-1

/// The FP.5 bytes of a value, with its one terminal LF.
pub fn json_bytes(value: &Value) -> Result<Vec<u8>, Problem> {
    let mut out = String::new();
    write(value, &mut out, 0)?;
    out.push('\n');
    Ok(out.into_bytes())
}

fn write(value: &Value, out: &mut String, depth: usize) -> Result<(), Problem> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) if n.unsigned_abs() <= MAX_INTEGER as u64 => out.push_str(&n.to_string()),
        Value::Int(_) | Value::Number(_) => {
            return Err(Problem::format("record", "a number in c600-json-1 is a bounded integer"));
        }
        Value::String(text) => write_string(text, out),
        Value::Array(items) => {
            if depth + 1 > JSON_DEPTH {
                return Err(size("record", "the record nests deeper than 64 levels"));
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(item, out, depth + 1)?;
            }
            out.push(']');
        }
        Value::Object(fields) => {
            if depth + 1 > JSON_DEPTH {
                return Err(size("record", "the record nests deeper than 64 levels"));
            }
            out.push('{');
            // A BTreeMap of strings is in byte order, which for UTF-8 is scalar order (FP.5).
            for (i, (key, item)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write(item, out, depth + 1)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Reads stored record bytes: the FP.9 limits, FP.7's carrier, strict JSON, and byte equality with the value's FP.5
/// encoding, so that no other spelling of a value is accepted (PS1.3).
pub fn decode(bytes: &[u8]) -> Result<Value, Problem> {
    if bytes.len() > RECORD_STORED {
        return Err(size("bytes", "the record has more than 71,303,168 stored bytes"));
    }
    let text: Cow<[u8]> = if bytes.starts_with(&[0x1f, 0x8b]) {
        Cow::Owned(gunzip_stored(bytes)?)
    } else {
        if bytes.len() > RECORD_DECODED {
            return Err(size("bytes", "the record has more than 67,108,864 decoded bytes"));
        }
        if bytes.len() > GZIP_THRESHOLD {
            return Err(noncanonical("bytes", "a record of more than 250,000 bytes is stored as gzip"));
        }
        Cow::Borrowed(bytes)
    };
    let value = json::parse(&text).map_err(|error| Problem::format("record", error.0))?;
    check_numbers_and_depth(&value, 0)?;
    if json_bytes(&value)? != *text {
        return Err(noncanonical("record", "the bytes are not the c600-json-1 encoding of their value"));
    }
    Ok(value)
}

fn check_numbers_and_depth(value: &Value, depth: usize) -> Result<(), Problem> {
    match value {
        Value::Int(n) if n.unsigned_abs() > MAX_INTEGER as u64 => {
            Err(noncanonical("record", "an integer outside the bounded JSON range is written as an FP.6 string"))
        }
        Value::Number(_) => Err(noncanonical("record", "a number in c600-json-1 is a bounded integer")),
        Value::Array(items) => {
            if depth + 1 > JSON_DEPTH {
                return Err(size("record", "the record nests deeper than 64 levels"));
            }
            items.iter().try_for_each(|item| check_numbers_and_depth(item, depth + 1))
        }
        Value::Object(fields) => {
            if depth + 1 > JSON_DEPTH {
                return Err(size("record", "the record nests deeper than 64 levels"));
            }
            fields.values().try_for_each(|item| check_numbers_and_depth(item, depth + 1))
        }
        _ => Ok(()),
    }
}

/// The stored record bytes of a value: its FP.5 bytes, or FP.7's stored gzip of them when they pass 250,000 bytes.
pub fn encode(value: &Value) -> Result<Vec<u8>, Problem> {
    let bytes = json_bytes(value)?;
    if bytes.len() > RECORD_DECODED {
        return Err(size("record", "the record has more than 67,108,864 decoded bytes"));
    }
    Ok(if bytes.len() > GZIP_THRESHOLD { gzip_stored(&bytes) } else { bytes })
}

// ---------------------------------------------------------------------------------------------------------------
// c600-gzip-stored-1

/// FP.7's one gzip member of stored blocks.
pub fn gzip_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / BLOCK * 5 + 32);
    out.extend_from_slice(&GZIP_HEADER);
    let mut chunks = data.chunks(BLOCK).peekable();
    if chunks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(chunk) = chunks.next() {
        out.push(if chunks.peek().is_none() { 1 } else { 0 });
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

/// Reads FP.7's profile and nothing else: the fixed header, full stored blocks before the final one, the trailer, no
/// trailing bytes, and more than 250,000 decoded bytes.
fn gunzip_stored(bytes: &[u8]) -> Result<Vec<u8>, Problem> {
    if bytes.len() < GZIP_HEADER.len() || bytes[..GZIP_HEADER.len()] != GZIP_HEADER {
        return Err(noncanonical("bytes", "the gzip header is not c600-gzip-stored-1's"));
    }
    let truncated = || Problem::format("bytes", "the gzip member is truncated");
    let mut at = GZIP_HEADER.len();
    let mut out = Vec::new();
    loop {
        let kind = *bytes.get(at).ok_or_else(truncated)?;
        if kind > 1 {
            return Err(noncanonical("bytes", "a gzip block is not a stored block of c600-gzip-stored-1"));
        }
        let header = bytes.get(at + 1..at + 5).ok_or_else(truncated)?;
        let len = u16::from_le_bytes([header[0], header[1]]);
        let nlen = u16::from_le_bytes([header[2], header[3]]);
        if nlen != !len {
            return Err(Problem::format("bytes", "a stored block's length check fails"));
        }
        let final_block = kind == 1;
        if !final_block && len as usize != BLOCK {
            return Err(noncanonical("bytes", "a stored block before the last one is not full"));
        }
        let data = bytes.get(at + 5..at + 5 + len as usize).ok_or_else(truncated)?;
        if out.len() + data.len() > RECORD_DECODED {
            return Err(size("bytes", "the record has more than 67,108,864 decoded bytes"));
        }
        out.extend_from_slice(data);
        at += 5 + len as usize;
        if final_block {
            break;
        }
    }
    let trailer = bytes.get(at..at + 8).ok_or_else(truncated)?;
    let crc = u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
    let length = u32::from_le_bytes([trailer[4], trailer[5], trailer[6], trailer[7]]);
    if crc != crc32(&out) || length != out.len() as u32 {
        return Err(Problem::new(code::DIGEST_MISMATCH, Gate::Format, "bytes", "the gzip CRC or length check fails"));
    }
    if at + 8 != bytes.len() {
        return Err(noncanonical("bytes", "bytes follow the gzip member"));
    }
    if out.len() <= GZIP_THRESHOLD {
        return Err(noncanonical("bytes", "a record of at most 250,000 bytes is stored raw, not as gzip"));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------------------
// base64

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding.
pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> shift) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Strict standard base64: padding only at the end, unused bits zero, no other character.
pub fn unbase64(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index + 1 == bytes.len() / 4;
        let pad = chunk.iter().rev().take_while(|&&b| b == b'=').count();
        if pad > 2 || (pad > 0 && !last) {
            return None;
        }
        let mut n = 0u32;
        for &b in &chunk[..4 - pad] {
            let v = ALPHABET.iter().position(|&a| a == b)? as u32;
            n = (n << 6) | v;
        }
        n <<= 6 * pad as u32;
        let decoded = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        let keep = 3 - pad;
        if decoded[keep..].iter().any(|&b| b != 0) {
            return None;
        }
        out.extend_from_slice(&decoded[..keep]);
    }
    Some(out)
}

// ---------------------------------------------------------------------------------------------------------------
// Carriers and references

/// FP.8's `DigestRef`.
pub fn digest_ref(role: &str, sha256: &str) -> Value {
    json::object([("role", Value::String(role.to_owned())), ("sha256", Value::String(sha256.to_owned()))])
}

/// Whether a text is 64 lowercase hex digits (FP.8).
pub fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// The PS1.7 `u32le-base64` carrier of a state's labels. Its SHA-256 covers the decoded bytes, which for labels are
/// the bytes of the PS1.6 full-state hash.
pub fn labels_value(labels: &[i32]) -> Value {
    let bytes: Vec<u8> = labels.iter().flat_map(|label| (*label as u32).to_le_bytes()).collect();
    json::object([
        ("count", nat(labels.len() as u64)),
        ("data", Value::String(base64(&bytes))),
        ("encoding", Value::String("u32le-base64".to_owned())),
        ("sha256", Value::String(c600_digest::sha256_hex(&bytes))),
    ])
}

/// A Nat as FP.6's decimal string.
pub fn nat(n: u64) -> Value {
    Value::String(n.to_string())
}

/// A text, or null.
pub fn text_or_null(text: Option<&str>) -> Value {
    text.map_or(Value::Null, |text| Value::String(text.to_owned()))
}

/// An object from owned pairs.
pub fn object(pairs: Vec<(&str, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(key, value)| (key.to_owned(), value)).collect::<BTreeMap<_, _>>())
}

/// Reads an object's fields by name, path by path, and refuses any field it did not read (FP.10).
pub(crate) struct Fields<'a> {
    path: String,
    fields: &'a BTreeMap<String, Value>,
    read: Vec<&'a str>,
}

impl<'a> Fields<'a> {
    pub(crate) fn of(value: &'a Value, path: &str) -> Result<Fields<'a>, Problem> {
        match value {
            Value::Object(fields) => Ok(Fields { path: path.to_owned(), fields, read: Vec::new() }),
            _ => Err(Problem::format(path, "an object is required here")),
        }
    }

    pub(crate) fn path(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    pub(crate) fn value(&mut self, key: &'a str) -> Result<&'a Value, Problem> {
        let at = self.path(key);
        let (name, value) =
            self.fields.get_key_value(key).ok_or_else(|| Problem::format(at, "a required field is missing"))?;
        self.read.push(name.as_str());
        Ok(value)
    }

    pub(crate) fn optional(&mut self, key: &'a str) -> Option<&'a Value> {
        let (name, value) = self.fields.get_key_value(key)?;
        self.read.push(name.as_str());
        Some(value)
    }

    pub(crate) fn text(&mut self, key: &'a str) -> Result<String, Problem> {
        let at = self.path(key);
        self.value(key)?.as_str().map(str::to_owned).ok_or_else(|| Problem::format(at, "a text is required here"))
    }

    pub(crate) fn text_or_null(&mut self, key: &'a str) -> Result<Option<String>, Problem> {
        let at = self.path(key);
        match self.value(key)? {
            Value::Null => Ok(None),
            Value::String(text) => Ok(Some(text.clone())),
            _ => Err(Problem::format(at, "a text or null is required here")),
        }
    }

    pub(crate) fn boolean(&mut self, key: &'a str) -> Result<bool, Problem> {
        let at = self.path(key);
        match self.value(key)? {
            Value::Bool(b) => Ok(*b),
            _ => Err(Problem::format(at, "a boolean is required here")),
        }
    }

    pub(crate) fn nat(&mut self, key: &'a str) -> Result<u64, Problem> {
        let at = self.path(key);
        read_nat(self.value(key)?, &at)
    }

    pub(crate) fn array(&mut self, key: &'a str) -> Result<&'a [Value], Problem> {
        let at = self.path(key);
        self.value(key)?.as_array().ok_or_else(|| Problem::format(at, "a list is required here"))
    }

    /// The SHA-256 of a `DigestRef` of the given role.
    pub(crate) fn digest(&mut self, key: &'a str, role: &str) -> Result<String, Problem> {
        let at = self.path(key);
        read_digest(self.value(key)?, &at, role)
    }

    pub(crate) fn digest_or_null(&mut self, key: &'a str, role: &str) -> Result<Option<String>, Problem> {
        let at = self.path(key);
        match self.value(key)? {
            Value::Null => Ok(None),
            value => read_digest(value, &at, role).map(Some),
        }
    }

    /// Refuses any field that was not read.
    pub(crate) fn finish(self) -> Result<(), Problem> {
        match self.fields.keys().find(|key| !self.read.contains(&key.as_str())) {
            Some(key) => Err(Problem::format(self.path(key), "an unknown field outside extensions")),
            None => Ok(()),
        }
    }
}

/// A Nat: FP.6's decimal string without a sign or a leading zero, within u64.
pub(crate) fn read_nat(value: &Value, at: &str) -> Result<u64, Problem> {
    let text = value.as_str().ok_or_else(|| Problem::format(at, "a Nat is a decimal string"))?;
    let canonical = text == "0" || (text.starts_with(|c: char| ('1'..='9').contains(&c)));
    if !canonical || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Problem::format(at, "a Nat is a decimal string without sign or leading zero"));
    }
    text.parse().map_err(|_| Problem::new(code::PERSISTENCE_SIZE, Gate::Format, at, "the Nat is too large"))
}

pub(crate) fn read_digest(value: &Value, at: &str, role: &str) -> Result<String, Problem> {
    let mut fields = Fields::of(value, at)?;
    let found = fields.text("role")?;
    let sha256 = fields.text("sha256")?;
    fields.finish()?;
    if found != role {
        return Err(Problem::format(at, format!("the DigestRef's role is {found}, not {role}")));
    }
    if !is_sha256(&sha256) {
        return Err(Problem::format(at, "a SHA-256 is 64 lowercase hex digits"));
    }
    Ok(sha256)
}

/// The labels of a PS1.7 carrier, with its count and digest checked.
pub(crate) fn read_labels(value: &Value, at: &str) -> Result<Vec<i32>, Problem> {
    let mut fields = Fields::of(value, at)?;
    let encoding = fields.text("encoding")?;
    let count = fields.nat("count")?;
    let sha256 = fields.text("sha256")?;
    let data = fields.text("data")?;
    fields.finish()?;
    if encoding != "u32le-base64" {
        return Err(Problem::format(at, "labels are carried as u32le-base64"));
    }
    let bytes = unbase64(&data).ok_or_else(|| Problem::format(at, "the carrier's data is not strict base64"))?;
    if bytes.len() as u64 != count.saturating_mul(4) {
        return Err(Problem::format(at, "the carrier's count differs from its data"));
    }
    if !is_sha256(&sha256) {
        return Err(Problem::format(at, "a SHA-256 is 64 lowercase hex digits"));
    }
    if c600_digest::sha256_hex(&bytes) != sha256 {
        return Err(Problem::new(
            code::DIGEST_MISMATCH,
            Gate::Format,
            at,
            "the carrier's data differs from its SHA-256",
        ));
    }
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| i32::try_from(u32::from_le_bytes(*b)))
        .collect::<Result<_, _>>()
        .map_err(|_| Problem::format(at, "a label is outside the slots"))
}
