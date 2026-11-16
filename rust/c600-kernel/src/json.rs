//! Strict JSON for the model files, the oracle cases and the traces.
//!
//! Numbers keep their kind. An integer lexeme that fits `i64` reads as [`Value::Int`]; any other number keeps its
//! lexeme in [`Value::Number`], so a float, a boolean or a string is never taken for an integer (0.4 checks
//! `type(x) == int`). Duplicate object keys, `NaN` and `Infinity`, trailing bytes, invalid UTF-8, lone surrogates and
//! nesting deeper than [`MAX_DEPTH`] are refused. [`Value::canonical`] writes what Python's
//! `json.dumps(value, sort_keys=True, separators=(",", ":"))` writes for null, booleans, integers, strings, arrays and
//! objects, with the same ASCII escapes. A float keeps the lexeme it was read with, which can differ from Python's
//! `repr`; no digest this crate computes contains a float. Error messages never quote the input.

use std::collections::BTreeMap;
use std::fmt;

/// The deepest nesting of arrays and objects a document may have.
pub const MAX_DEPTH: usize = 128;

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
    let text = std::str::from_utf8(bytes).map_err(|_| JsonError("JSON text is not UTF-8"))?;
    let mut parser = Parser { text, bytes, pos: 0 };
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
}

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
        match self.peek() {
            Some(b'n') => self.literal("null", Value::Null),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'"') => Ok(Value::String(self.string()?)),
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
                loop {
                    items.push(self.value(depth + 1)?);
                    self.space();
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
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
                self.space();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    return Ok(Value::Object(fields));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.space();
                    self.expect(b':')?;
                    let item = self.value(depth + 1)?;
                    if fields.insert(key, item).is_some() {
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
}
