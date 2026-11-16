//! Strict JSON for the model files, the oracle cases and the traces.
//!
//! Numbers keep their kind. An integer lexeme that fits `i64` reads as [`Value::Int`]; any other number keeps its
//! lexeme in [`Value::Number`], so a float, a boolean or a string is never taken for an integer (0.4 checks
//! `type(x) == int`). Duplicate object keys, `NaN` and `Infinity`, trailing bytes, invalid UTF-8, lone surrogates and
//! nesting deeper than [`MAX_DEPTH`] are refused. [`Value::canonical`] writes what Python's
//! `json.dumps(value, sort_keys=True, separators=(",", ":"))` writes for null, booleans, integers, strings, arrays and
//! objects, with the same ASCII escapes. A float keeps the lexeme it was read with, which can differ from Python's
//! `repr`; no digest this crate computes contains a float. Error messages never quote the input.
//!
//! [`parse_lenient`] reads a document the same way, except in the regions its caller names: there a value Python's
//! `json` reads but this reader cannot represent (a string with a lone surrogate, an array or object deeper than
//! [`MAX_DEPTH`]) reads as null, and a member whose name has a lone surrogate is left out. The oracle adapter uses it
//! for the recipes of a case, which 0.4 refuses step by step rather than as a whole case.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The deepest nesting of arrays and objects a document may have.
pub const MAX_DEPTH: usize = 128;

/// The deepest nesting [`parse_lenient`] reads past in a lenient region. CPython 3.14.7's `json.loads` raises
/// `RecursionError` before 20,000 levels, so no case the reference wrote is deeper; the bound keeps memory small.
pub const MAX_SKIPPED_DEPTH: usize = 100_000;

/// Says from the path of a value whether [`parse_lenient`] reads it leniently. The path has one entry per enclosing
/// container: `None` inside an array, the member name inside an object (U+FFFD for a lone surrogate).
pub type Lenient = fn(&[Option<String>]) -> bool;

/// A JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    /// An integer lexeme that fits `i64`.
    Int(i64),
    /// Any other number, as its lexeme: a fraction, an exponent or an integer outside `i64`.
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

/// Why a document was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError(pub &'static str);

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for JsonError {}

impl Value {
    /// The member `key` of an object; `None` for a missing key or a value that is not an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(fields) => fields.get(key),
            _ => None,
        }
    }

    /// The integer of an [`Value::Int`]; `None` for every other value, booleans and floats included.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// The canonical text: sorted keys, no spaces, ASCII escapes.
    pub fn canonical(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(true) => out.push_str("true"),
            Value::Bool(false) => out.push_str("false"),
            Value::Int(value) => out.push_str(&value.to_string()),
            Value::Number(lexeme) => out.push_str(lexeme),
            Value::String(text) => write_string(text, out),
            Value::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Value::Object(fields) => {
                out.push('{');
                for (i, (key, item)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(key, out);
                    out.push(':');
                    item.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// An object from fixed pairs, for writing.
pub fn object<const N: usize>(pairs: [(&str, Value); N]) -> Value {
    Value::Object(pairs.into_iter().map(|(key, value)| (key.to_owned(), value)).collect())
}

fn write_string(text: &str, out: &mut String) {
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
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// Parses one document. Whitespace may surround it; nothing else may follow it.
pub fn parse(bytes: &[u8]) -> Result<Value, JsonError> {
    read(bytes, None)
}

/// Parses one document like [`parse`], except where `lenient` accepts the path of a value: there a string with a lone
/// surrogate and an array or object nested deeper than [`MAX_DEPTH`] read as [`Value::Null`], and an object member
/// whose name has a lone surrogate is left out. Bad grammar, repeated member names (by their UTF-16 units) and nesting
/// beyond [`MAX_SKIPPED_DEPTH`] are refused everywhere, and every other value reads as [`parse`] reads it.
pub fn parse_lenient(bytes: &[u8], lenient: Lenient) -> Result<Value, JsonError> {
    read(bytes, Some(lenient))
}

fn read(bytes: &[u8], lenient: Option<Lenient>) -> Result<Value, JsonError> {
    let text = std::str::from_utf8(bytes).map_err(|_| JsonError("JSON text is not UTF-8"))?;
    let mut parser = Parser { text, bytes, pos: 0, lenient, path: Vec::new() };
    let value = parser.value(0)?;
    parser.space();
    if parser.pos != bytes.len() {
        return Err(JsonError("Trailing bytes after the JSON value"));
    }
    Ok(value)
}

struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    pos: usize,
    lenient: Option<Lenient>,
    /// The path of the current value, kept only for a lenient parse.
    path: Vec<Option<String>>,
}

/// A member name: text, or in a lenient object the UTF-16 units of a name with a lone surrogate.
enum Key {
    Text(String),
    Lone(Vec<u16>),
}

/// The open containers while [`Parser::skip`] reads past nesting: `None` for an array, the names so far for an object.
type Open = Vec<Option<BTreeSet<Vec<u16>>>>;

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), JsonError> {
        if self.peek() != Some(byte) {
            return Err(JsonError("Malformed JSON"));
        }
        self.pos += 1;
        Ok(())
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, JsonError> {
        if !self.text[self.pos..].starts_with(word) {
            return Err(JsonError("Malformed JSON literal"));
        }
        self.pos += word.len();
        Ok(value)
    }

    fn value(&mut self, depth: usize) -> Result<Value, JsonError> {
        self.space();
        let lenient = self.lenient.is_some_and(|lenient| lenient(&self.path));
        match self.peek() {
            Some(b'n') => self.literal("null", Value::Null),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'"') if lenient => Ok(String::from_utf16(&self.units()?).map_or(Value::Null, Value::String)),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'[' | b'{') if lenient && depth >= MAX_DEPTH => {
                self.skip(depth)?;
                Ok(Value::Null)
            }
            Some(b'[') => {
                if depth >= MAX_DEPTH {
                    return Err(JsonError("JSON nesting is too deep"));
                }
                self.pos += 1;
                let mut items = Vec::new();
                self.space();
                if self.peek() == Some(b']') {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                self.enter(|| None);
                loop {
                    items.push(self.value(depth + 1)?);
                    self.space();
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
                            self.path.pop();
                            return Ok(Value::Array(items));
                        }
                        _ => return Err(JsonError("Malformed JSON array")),
                    }
                }
            }
            Some(b'{') => {
                if depth >= MAX_DEPTH {
                    return Err(JsonError("JSON nesting is too deep"));
                }
                self.pos += 1;
                let mut fields = BTreeMap::new();
                // The names a lenient object left out, so that a repeated one is still refused.
                let mut left_out = BTreeSet::new();
                self.space();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    return Ok(Value::Object(fields));
                }
                loop {
                    self.space();
                    let key = if lenient {
                        let units = self.units()?;
                        String::from_utf16(&units).map_or(Key::Lone(units), Key::Text)
                    } else {
                        Key::Text(self.string()?)
                    };
                    self.space();
                    self.expect(b':')?;
                    self.enter(|| match &key {
                        Key::Text(name) => Some(name.clone()),
                        Key::Lone(units) => Some(String::from_utf16_lossy(units)),
                    });
                    let item = self.value(depth + 1)?;
                    self.path.pop();
                    let new = match key {
                        Key::Text(name) => fields.insert(name, item).is_none(),
                        Key::Lone(units) => left_out.insert(units),
                    };
                    if !new {
                        return Err(JsonError("Duplicate JSON key"));
                    }
                    self.space();
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            return Ok(Value::Object(fields));
                        }
                        _ => return Err(JsonError("Malformed JSON object")),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(JsonError("Malformed JSON value")),
        }
    }

    /// Extends the path of a lenient parse; a strict parse keeps no path.
    fn enter(&mut self, entry: impl FnOnce() -> Option<String>) {
        if self.lenient.is_some() {
            self.path.push(entry());
        }
    }

    /// Reads past an array or object that starts at `depth` in a lenient region without keeping it, checking what
    /// [`Parser::value`] checks there: the grammar, the strings and repeated member names. It is iterative, so deep
    /// nesting cannot exhaust the stack.
    fn skip(&mut self, depth: usize) -> Result<(), JsonError> {
        let mut open: Open = Vec::new();
        loop {
            // A value starts here.
            self.space();
            match self.peek() {
                Some(bracket @ (b'[' | b'{')) => {
                    if depth + open.len() >= MAX_SKIPPED_DEPTH {
                        return Err(JsonError("JSON nesting is too deep"));
                    }
                    self.pos += 1;
                    self.space();
                    if bracket == b'[' {
                        open.push(None);
                        if self.peek() != Some(b']') {
                            continue;
                        }
                    } else {
                        open.push(Some(BTreeSet::new()));
                        if self.peek() != Some(b'}') {
                            self.member(&mut open)?;
                            continue;
                        }
                    }
                }
                Some(b'"') => {
                    self.units()?;
                }
                Some(b'n') => {
                    self.literal("null", Value::Null)?;
                }
                Some(b't') => {
                    self.literal("true", Value::Null)?;
                }
                Some(b'f') => {
                    self.literal("false", Value::Null)?;
                }
                Some(b'-' | b'0'..=b'9') => {
                    self.number()?;
                }
                _ => return Err(JsonError("Malformed JSON value")),
            }
            // A value or an empty container ends here: close what ends, then start the next item.
            loop {
                let array = match open.last() {
                    None => return Ok(()),
                    Some(names) => names.is_none(),
                };
                self.space();
                match (self.peek(), array) {
                    (Some(b','), true) => {
                        self.pos += 1;
                        break;
                    }
                    (Some(b','), false) => {
                        self.pos += 1;
                        self.member(&mut open)?;
                        break;
                    }
                    (Some(b']'), true) | (Some(b'}'), false) => {
                        self.pos += 1;
                        open.pop();
                    }
                    (_, true) => return Err(JsonError("Malformed JSON array")),
                    (_, false) => return Err(JsonError("Malformed JSON object")),
                }
            }
        }
    }

    /// Reads a member name and its colon for [`Parser::skip`], refusing a name the innermost object already has.
    fn member(&mut self, open: &mut Open) -> Result<(), JsonError> {
        self.space();
        let name = self.units()?;
        if let Some(Some(names)) = open.last_mut() {
            if !names.insert(name) {
                return Err(JsonError("Duplicate JSON key"));
            }
        }
        self.space();
        self.expect(b':')
    }

    fn digits(&mut self) -> Result<(), JsonError> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(JsonError("Malformed JSON number"));
        }
        Ok(())
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        let mut integer = true;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        if self.peek() == Some(b'0') {
            self.pos += 1;
        } else {
            self.digits()?;
        }
        if self.peek() == Some(b'.') {
            integer = false;
            self.pos += 1;
            self.digits()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integer = false;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            self.digits()?;
        }
        let lexeme = &self.text[start..self.pos];
        if integer {
            if let Ok(value) = lexeme.parse::<i64>() {
                return Ok(Value::Int(value));
            }
        }
        Ok(Value::Number(lexeme.to_owned()))
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self.bytes.get(self.pos..self.pos + 4).ok_or(JsonError("Malformed JSON escape"))?;
        let mut value = 0;
        for &digit in digits {
            value = value * 16 + (digit as char).to_digit(16).ok_or(JsonError("Malformed JSON escape"))?;
        }
        self.pos += 4;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while matches!(self.peek(), Some(byte) if byte != b'"' && byte != b'\\' && byte >= 0x20) {
                self.pos += 1;
            }
            // The text is valid UTF-8 and the run stops only at ASCII bytes, so the slice is whole characters.
            out.push_str(&self.text[start..self.pos]);
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let escape = self.peek().ok_or(JsonError("Unterminated JSON string"))?;
                    self.pos += 1;
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
                            let mut code = self.hex4()?;
                            if (0xd800..0xdc00).contains(&code) {
                                if self.bytes.get(self.pos..self.pos + 2) != Some(b"\\u") {
                                    return Err(JsonError("Lone surrogate in JSON string"));
                                }
                                self.pos += 2;
                                let low = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&low) {
                                    return Err(JsonError("Lone surrogate in JSON string"));
                                }
                                code = 0x10000 + ((code - 0xd800) << 10) + (low - 0xdc00);
                            }
                            out.push(char::from_u32(code).ok_or(JsonError("Lone surrogate in JSON string"))?);
                        }
                        _ => return Err(JsonError("Malformed JSON escape")),
                    }
                }
                Some(_) => return Err(JsonError("Control character in JSON string")),
                None => return Err(JsonError("Unterminated JSON string")),
            }
        }
    }

    /// A string of a lenient region as UTF-16 units, as Python's `json` reads it: a surrogate pair of two escapes
    /// becomes one character and a lone surrogate stays. Everything else is checked as in [`Parser::string`].
    fn units(&mut self) -> Result<Vec<u16>, JsonError> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            let start = self.pos;
            while matches!(self.peek(), Some(byte) if byte != b'"' && byte != b'\\' && byte >= 0x20) {
                self.pos += 1;
            }
            out.extend(self.text[start..self.pos].encode_utf16());
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let escape = self.peek().ok_or(JsonError("Unterminated JSON string"))?;
                    self.pos += 1;
                    out.push(match escape {
                        b'"' | b'\\' | b'/' => u16::from(escape),
                        b'b' => 0x8,
                        b'f' => 0xc,
                        b'n' => 0xa,
                        b'r' => 0xd,
                        b't' => 0x9,
                        // Four hexadecimal digits are at most 0xffff.
                        b'u' => self.hex4()? as u16,
                        _ => return Err(JsonError("Malformed JSON escape")),
                    });
                }
                Some(_) => return Err(JsonError("Control character in JSON string")),
                None => return Err(JsonError("Unterminated JSON string")),
            }
        }
    }
}
