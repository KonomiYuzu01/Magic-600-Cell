//! Vectors for the crate's strict JSON reader and its canonical writer ([`c600_kernel::json`]). The canonical texts
//! are what Python writes with `json.dumps(json.loads(text), sort_keys=True, separators=(",", ":"))`.

use c600_kernel::json::{self, parse, JsonError, Value, MAX_DEPTH};

fn refused(text: &str) -> &'static str {
    parse(text.as_bytes()).unwrap_err().0
}

/// A JSON escape of one UTF-16 unit, a backslash, `u` and four hexadecimal digits, built at run time.
fn esc(hex: &str) -> String {
    format!("{}u{hex}", '\\')
}

#[test]
fn numbers_keep_their_kind() {
    assert_eq!(parse(b"1").unwrap(), Value::Int(1));
    assert_eq!(parse(b"-0").unwrap(), Value::Int(0));
    assert_eq!(parse(b"9223372036854775807").unwrap(), Value::Int(i64::MAX));
    assert_eq!(parse(b"-9223372036854775808").unwrap(), Value::Int(i64::MIN));
    assert_eq!(parse(b"9223372036854775808").unwrap(), Value::Number("9223372036854775808".into()));
    assert_eq!(parse(b"1.0").unwrap(), Value::Number("1.0".into()));
    assert_eq!(parse(b"1e2").unwrap(), Value::Number("1e2".into()));
    // Only an integer lexeme is an integer: 0.4 checks `type(x) == int`.
    for text in ["true", "1.0", "1e2", "\"1\"", "null", "9223372036854775808"] {
        assert_eq!(parse(text.as_bytes()).unwrap().as_i64(), None, "{text}");
    }
}

#[test]
fn malformed_documents_are_refused() {
    assert_eq!(refused("NaN"), "Malformed JSON value");
    assert_eq!(refused("Infinity"), "Malformed JSON value");
    assert_eq!(refused("-Infinity"), "Malformed JSON number");
    assert_eq!(refused("1."), "Malformed JSON number");
    assert_eq!(refused("01"), "Trailing bytes after the JSON value");
    assert_eq!(refused("1 2"), "Trailing bytes after the JSON value");
    assert_eq!(refused("[1,]"), "Malformed JSON value");
    assert_eq!(refused("[1 2]"), "Malformed JSON array");
    assert_eq!(refused(r#"{"a" 1}"#), "Malformed JSON");
    assert_eq!(refused(r#"{"a":1 "b":2}"#), "Malformed JSON object");
    assert_eq!(refused(r#"{"a":1,"a":2}"#), "Duplicate JSON key");
    assert_eq!(refused("tru"), "Malformed JSON literal");
    for units in [esc("d800"), esc("dc00"), esc("d800") + &esc("0041"), esc("d800") + "A"] {
        assert_eq!(refused(&format!("\"{units}\"")), "Lone surrogate in JSON string", "{units}");
    }
    assert_eq!(refused(r#""\x""#), "Malformed JSON escape");
    assert_eq!(refused(&format!("\"{}\"", esc("12g4"))), "Malformed JSON escape");
    assert_eq!(refused("\"a\u{1}b\""), "Control character in JSON string");
    assert_eq!(refused("\"abc"), "Unterminated JSON string");
    assert_eq!(parse(b"\"\xff\"").unwrap_err(), JsonError("JSON text is not UTF-8"));
    assert_eq!(parse(b" 1 \n").unwrap(), Value::Int(1));
}

#[test]
fn nesting_is_bounded() {
    let nested = |depth: usize| format!("{}{}", "[".repeat(depth), "]".repeat(depth));
    assert!(parse(nested(MAX_DEPTH).as_bytes()).is_ok());
    assert_eq!(refused(&nested(MAX_DEPTH + 1)), "JSON nesting is too deep");
}

#[test]
fn canonical_text_matches_python() {
    let (e_acute, one, delete) = (esc("00e9"), esc("0001"), esc("007f"));
    let smiley = '\u{1F600}';
    let escapes = format!(r#""caf{e_acute} {smiley} / \" \\ \b\f\n\r\t {one} {delete}""#);
    let written = format!(r#""caf{e_acute} {}{} / \" \\ \b\f\n\r\t {one} {delete}""#, esc("d83d"), esc("de00"));
    let cases = [
        (r#"{ "b" : [1, true, null, false], "a" : "x" }"#, r#"{"a":"x","b":[1,true,null,false]}"#),
        (escapes.as_str(), written.as_str()),
        (r#"{"z":{},"y":[],"":-5}"#, r#"{"":-5,"y":[],"z":{}}"#),
        ("[-0, 0, 12, -9223372036854775808]", "[0,0,12,-9223372036854775808]"),
    ];
    for (text, expected) in cases {
        assert_eq!(parse(text.as_bytes()).unwrap().canonical(), expected, "{text}");
    }
    let pair = format!("\"{}{}\"", esc("d83d"), esc("de00"));
    assert_eq!(parse(pair.as_bytes()).unwrap(), Value::String(smiley.into()));
    // A float keeps the lexeme it was read with (Python would write 1.5); no digest of this crate contains one.
    assert_eq!(parse(b"[1.50]").unwrap().canonical(), "[1.50]");
}

#[test]
fn accessors_and_the_object_helper() {
    let value = json::object([("b", Value::Int(2)), ("a", Value::String("x".into()))]);
    assert_eq!(value.canonical(), r#"{"a":"x","b":2}"#);
    assert_eq!(value.get("a").and_then(Value::as_str), Some("x"));
    assert_eq!(value.get("b").and_then(Value::as_i64), Some(2));
    assert_eq!(value.get("c"), None);
    assert_eq!(Value::Int(1).get("a"), None);
    assert!(value.as_object().is_some() && value.as_array().is_none());
    assert_eq!(parse(b"[1]").unwrap().as_array(), Some(&[Value::Int(1)][..]));
}
