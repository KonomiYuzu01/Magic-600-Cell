//! The JSON the obligation translator reads: RFC 8259 text, and the canonical bytes of new records (PS1.3 to PS1.5)
//! as the format packet's reference reader reads and writes them (`check_vectors.py`'s `canonical` and
//! `read_canonical`, profile "new").
//!
//! Canonical bytes are compact JSON with object keys sorted by code point, UTF-8 left unescaped except for `"`, `\`
//! and control characters (named escapes where JSON has them, otherwise `\u00xx` in lower case), integers only, each
//! within 2^53 - 1, at most 64 nested containers, and one final line feed. A read refuses bytes that differ from the
//! canonical bytes of the value they decode to.

use std::collections::BTreeMap;

use super::{ObligationError, Reason};

/// The format packet's `record_decoded` row: bytes in one decoded new record.
pub const RECORD_DECODED: usize = 67_108_864;
/// The format packet's `json_depth` row: containers on the deepest path; a scalar root is 0.
pub const JSON_DEPTH: usize = 64;
/// The format packet's `integer_digits` row: decimal digits per exact integer.
pub const INTEGER_DIGITS: usize = 10_000;
/// 2^53 - 1, the largest integer a canonical new record holds.
const SAFE_INTEGER: &str = "9007199254740991";
/// A stack guard for general text. Canonical bytes are bounded by `JSON_DEPTH` after the parse, as the reference
/// reader bounds them.
const TEXT_DEPTH: usize = 512;

/// A JSON value. A number keeps its JSON text; an object keeps one value per key, in key order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(String),
    Text(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    /// The value under `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        self.as_object().and_then(|fields| fields.get(key))
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Json::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Object(fields) => Some(fields),
            _ => None,
        }
    }
}

fn noncanonical(detail: &str) -> ObligationError {
    ObligationError::new(Reason::Noncanonical, detail)
}

fn too_large(detail: &str) -> ObligationError {
    ObligationError::new(Reason::Size, detail)
}

/// Reads RFC 8259 text: whitespace is allowed, numbers keep their text, and a repeated key is refused. A refusal is
/// `PS1:input.noncanonical_value`.
pub fn parse(bytes: &[u8]) -> Result<Json, ObligationError> {
    let text = std::str::from_utf8(bytes).map_err(|_| noncanonical("the bytes are not UTF-8"))?;
    let mut parser = Parser { text, at: 0 };
    parser.space();
    let value = parser.value(0)?;
    parser.space();
    if parser.at != text.len() {
        return Err(noncanonical("text follows the JSON value"));
    }
    Ok(value)
}

/// Reads the canonical bytes of a new record. Bytes over `RECORD_DECODED` or a value nested deeper than `JSON_DEPTH`
/// refuse with `PS1:resource_limit.persistence_size`; anything else that is not the canonical bytes of its value
/// refuses with `PS1:input.noncanonical_value`.
pub fn read_canonical(bytes: &[u8]) -> Result<Json, ObligationError> {
    if bytes.len() > RECORD_DECODED {
        return Err(too_large("the record exceeds the decoded-record limit"));
    }
    let value = parse(bytes)?;
    if canonical(&value)? != bytes {
        return Err(noncanonical("the bytes are not the canonical bytes of their value"));
    }
    Ok(value)
}

/// The canonical bytes of `value`, with the final line feed. A number that is not an integer within 2^53 - 1 refuses
/// with `PS1:input.noncanonical_value`; more than `JSON_DEPTH` nested containers refuse with
/// `PS1:resource_limit.persistence_size`.
pub fn canonical(value: &Json) -> Result<Vec<u8>, ObligationError> {
    let mut out = String::new();
    write(value, 0, &mut out)?;
    out.push('\n');
    Ok(out.into_bytes())
}

fn write(value: &Json, depth: usize, out: &mut String) -> Result<(), ObligationError> {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(text) => out.push_str(&integer(text)?),
        Json::Text(text) => quote(text, out),
        Json::Array(items) => {
            if depth + 1 > JSON_DEPTH {
                return Err(too_large("the value nests more containers than the depth limit"));
            }
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write(item, depth + 1, out)?;
            }
            out.push(']');
        }
        Json::Object(fields) => {
            if depth + 1 > JSON_DEPTH {
                return Err(too_large("the value nests more containers than the depth limit"));
            }
            out.push('{');
            for (index, (key, item)) in fields.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                quote(key, out);
                out.push(':');
                write(item, depth + 1, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// A JSON number as a canonical integer: no fraction or exponent, `-0` read as 0, at most 2^53 - 1 in magnitude.
fn integer(text: &str) -> Result<String, ObligationError> {
    let digits = text.strip_prefix('-').unwrap_or(text);
    let plain =
        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) && (digits == "0" || !digits.starts_with('0'));
    if !plain {
        return Err(noncanonical("a number is not an integer"));
    }
    if digits.len() > SAFE_INTEGER.len() || (digits.len() == SAFE_INTEGER.len() && digits > SAFE_INTEGER) {
        return Err(noncanonical("an integer exceeds 2^53 - 1"));
    }
    Ok(if digits == "0" { "0".to_string() } else { text.to_string() })
}

fn quote(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A decimal integer as the obligation vocabulary writes one (`nat`, `int`, `rat`): text matching
/// `0|-?[1-9][0-9]*` with at most `INTEGER_DIGITS` digits. Malformed text refuses with
/// `PS1:input.noncanonical_value` and too many digits with `PS1:resource_limit.persistence_size`, as the reference
/// reader's `int_value` does. Returns whether the value is negative.
pub fn decimal(value: &Json) -> Result<(&str, bool), ObligationError> {
    let text = value.as_text().ok_or_else(|| noncanonical("an exact integer is not decimal text"))?;
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let well_formed = !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && if negative { !digits.starts_with('0') } else { digits == "0" || !digits.starts_with('0') };
    if !well_formed {
        return Err(noncanonical("an exact integer is not canonical decimal text"));
    }
    if digits.len() > INTEGER_DIGITS {
        return Err(too_large("an exact integer exceeds the digit limit"));
    }
    Ok((text, negative))
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, ObligationError> {
        if self.text[self.at..].starts_with(word) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(noncanonical("the text is not JSON"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, ObligationError> {
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Json::Text(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(noncanonical("the text is not JSON")),
        }
    }

    fn enter(&mut self, depth: usize) -> Result<(), ObligationError> {
        if depth + 1 > TEXT_DEPTH {
            return Err(noncanonical("the text nests too deeply to read"));
        }
        self.at += 1;
        self.space();
        Ok(())
    }

    fn object(&mut self, depth: usize) -> Result<Json, ObligationError> {
        self.enter(depth)?;
        let mut fields = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Json::Object(fields));
        }
        loop {
            if self.peek() != Some(b'"') {
                return Err(noncanonical("an object key is not a string"));
            }
            let key = self.string()?;
            self.space();
            if self.peek() != Some(b':') {
                return Err(noncanonical("an object key has no value"));
            }
            self.at += 1;
            self.space();
            let item = self.value(depth + 1)?;
            if fields.insert(key, item).is_some() {
                return Err(noncanonical("an object repeats a key"));
            }
            self.space();
            match self.peek() {
                Some(b',') => {
                    self.at += 1;
                    self.space();
                }
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Json::Object(fields));
                }
                _ => return Err(noncanonical("an object is not closed")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, ObligationError> {
        self.enter(depth)?;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.space();
            match self.peek() {
                Some(b',') => {
                    self.at += 1;
                    self.space();
                }
                Some(b']') => {
                    self.at += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(noncanonical("an array is not closed")),
            }
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
        self.at - start
    }

    fn number(&mut self) -> Result<Json, ObligationError> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        let lead = self.peek();
        let count = self.digits();
        if count == 0 || (lead == Some(b'0') && count > 1) {
            return Err(noncanonical("a number is not JSON"));
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if self.digits() == 0 {
                return Err(noncanonical("a number is not JSON"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return Err(noncanonical("a number is not JSON"));
            }
        }
        Ok(Json::Number(self.text[start..self.at].to_string()))
    }

    fn hex4(&mut self) -> Result<u32, ObligationError> {
        let digits = self.text.get(self.at..self.at + 4).ok_or_else(|| noncanonical("an escape is cut short"))?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(noncanonical("an escape is not hexadecimal"));
        }
        self.at += 4;
        u32::from_str_radix(digits, 16).map_err(|_| noncanonical("an escape is not hexadecimal"))
    }

    fn string(&mut self) -> Result<String, ObligationError> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let c = self.text[self.at..].chars().next().ok_or_else(|| noncanonical("a string is not closed"))?;
            self.at += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let escape = self.peek().ok_or_else(|| noncanonical("an escape is cut short"))?;
                    self.at += 1;
                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let code = match first {
                                0xD800..=0xDBFF => {
                                    if !self.text[self.at..].starts_with("\\u") {
                                        return Err(noncanonical("a string holds a lone surrogate"));
                                    }
                                    self.at += 2;
                                    let second = self.hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&second) {
                                        return Err(noncanonical("a string holds a lone surrogate"));
                                    }
                                    0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                                }
                                0xDC00..=0xDFFF => return Err(noncanonical("a string holds a lone surrogate")),
                                code => code,
                            };
                            out.push(char::from_u32(code).ok_or_else(|| noncanonical("an escape names no character"))?);
                        }
                        _ => return Err(noncanonical("a string holds an unknown escape")),
                    }
                }
                c if (c as u32) < 0x20 => return Err(noncanonical("a string holds a raw control character")),
                c => out.push(c),
            }
        }
    }
}
