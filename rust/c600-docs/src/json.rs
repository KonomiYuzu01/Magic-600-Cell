//! A small JSON reader and writer for the record data files (standard library only). Numbers are integers: no record
//! field holds a fraction, and an exact value that is not an integer is written as text. Objects keep their written
//! order, so a part's position in a file is its position in an error message.

use std::fmt::Write as _;

/// A JSON value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Text(String),
    List(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The value under `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Json]> {
        match self {
            Json::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// An object from its fields in order.
    pub fn object<'a>(fields: impl IntoIterator<Item = (&'a str, Json)>) -> Json {
        Json::Object(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    pub fn text(s: &str) -> Json {
        Json::Text(s.to_string())
    }

    /// A compact text form with the keys in written order: the form the record checks compare and hash.
    pub fn compact(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Text(s) => quote(s, out),
            Json::List(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Why bytes are not JSON: the byte offset and what was expected there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: String,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "byte {}: {}", self.offset, self.message)
    }
}

/// Reads one JSON value that fills the bytes, apart from surrounding whitespace. A duplicate key is refused, so no
/// part of a record is silently replaced by a later one.
pub fn parse(bytes: &[u8]) -> Result<Json, JsonError> {
    let text =
        std::str::from_utf8(bytes).map_err(|e| JsonError { offset: e.valid_up_to(), message: "UTF-8".into() })?;
    let mut p = Parser { s: text.as_bytes(), at: 0 };
    p.space();
    let value = p.value(0)?;
    p.space();
    if p.at != p.s.len() {
        return Err(p.error("the end of the document"));
    }
    Ok(value)
}

struct Parser<'a> {
    s: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn error(&self, expected: &str) -> JsonError {
        JsonError { offset: self.at, message: format!("expected {expected}") }
    }

    fn space(&mut self) {
        while self.at < self.s.len() && matches!(self.s[self.at], b' ' | b'\n' | b'\r' | b'\t') {
            self.at += 1;
        }
    }

    fn eat(&mut self, b: u8) -> bool {
        if self.s.get(self.at) == Some(&b) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, JsonError> {
        if self.s[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(self.error(word))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > 64 {
            return Err(self.error("at most 64 levels of nesting"));
        }
        match self.s.get(self.at) {
            Some(b'n') => self.literal("null", Json::Null),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'"') => Ok(Json::Text(self.string()?)),
            Some(b'[') => {
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.eat(b']') {
                    return Ok(Json::List(items));
                }
                loop {
                    self.space();
                    items.push(self.value(depth + 1)?);
                    self.space();
                    if self.eat(b']') {
                        return Ok(Json::List(items));
                    }
                    if !self.eat(b',') {
                        return Err(self.error("',' or ']'"));
                    }
                }
            }
            Some(b'{') => {
                self.at += 1;
                let mut fields: Vec<(String, Json)> = Vec::new();
                self.space();
                if self.eat(b'}') {
                    return Ok(Json::Object(fields));
                }
                loop {
                    self.space();
                    let at = self.at;
                    let key = self.string()?;
                    if fields.iter().any(|(k, _)| *k == key) {
                        return Err(JsonError {
                            offset: at,
                            message: format!("a key other than the repeated `{key}`"),
                        });
                    }
                    self.space();
                    if !self.eat(b':') {
                        return Err(self.error("':'"));
                    }
                    self.space();
                    let value = self.value(depth + 1)?;
                    fields.push((key, value));
                    self.space();
                    if self.eat(b'}') {
                        return Ok(Json::Object(fields));
                    }
                    if !self.eat(b',') {
                        return Err(self.error("',' or '}'"));
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => {
                let start = self.at;
                self.eat(b'-');
                while self.at < self.s.len() && self.s[self.at].is_ascii_digit() {
                    self.at += 1;
                }
                if matches!(self.s.get(self.at), Some(b'.' | b'e' | b'E')) {
                    return Err(self.error("an integer: record numbers have no fraction or exponent"));
                }
                let digits = std::str::from_utf8(&self.s[start..self.at]).unwrap_or("");
                digits
                    .parse::<i64>()
                    .map(Json::Int)
                    .map_err(|_| JsonError { offset: start, message: "an integer that fits in 64 bits".into() })
            }
            _ => Err(self.error("a value")),
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        if !self.eat(b'"') {
            return Err(self.error("'\"'"));
        }
        let mut out = String::new();
        loop {
            let Some(&b) = self.s.get(self.at) else { return Err(self.error("the closing '\"'")) };
            match b {
                b'"' => {
                    self.at += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.at += 1;
                    let Some(&e) = self.s.get(self.at) else { return Err(self.error("an escape")) };
                    self.at += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let hex = self.s.get(self.at..self.at + 4).ok_or_else(|| self.error("four hex digits"))?;
                            let hex = std::str::from_utf8(hex).map_err(|_| self.error("four hex digits"))?;
                            let unit = u32::from_str_radix(hex, 16).map_err(|_| self.error("four hex digits"))?;
                            self.at += 4;
                            let c =
                                char::from_u32(unit).ok_or_else(|| self.error("a scalar value, not a surrogate"))?;
                            out.push(c);
                        }
                        _ => return Err(self.error("a known escape")),
                    }
                }
                b if b < 0x20 => return Err(self.error("no control character inside a string")),
                _ => {
                    let rest = std::str::from_utf8(&self.s[self.at..]).unwrap_or("");
                    let c = rest.chars().next().unwrap_or('\u{fffd}');
                    out.push(c);
                    self.at += c.len_utf8();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_back() {
        // The escape is assembled from two pieces so that no tool rewrites it into the character it names.
        let doc = [r#" {"a": [1, -2, {"b": null}], "c": "x\"y\"#, r#"u00e9", "d": true} "#].concat();
        let json = parse(doc.as_bytes()).unwrap();
        assert_eq!(json.compact(), "{\"a\":[1,-2,{\"b\":null}],\"c\":\"x\\\"y\u{e9}\",\"d\":true}");
        assert_eq!(json.get("a").and_then(Json::as_list).map(<[Json]>::len), Some(3));
    }

    #[test]
    fn refuses_duplicates_fractions_and_trailing_bytes() {
        assert!(parse(br#"{"a": 1, "a": 2}"#).is_err());
        assert!(parse(b"1.5").is_err());
        assert!(parse(b"[1] 2").is_err());
        assert!(parse(b"\"\x01\"").is_err());
    }
}
