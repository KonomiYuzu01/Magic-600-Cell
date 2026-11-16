//! Vectors for the crate's strict JSON reader and its canonical writer ([`c600_kernel::json`]). The canonical texts
//! are what Python writes with `json.dumps(json.loads(text), sort_keys=True, separators=(",", ":"))`.

use c600_kernel::json::{self, parse, JsonError, Value, MAX_DEPTH, MAX_SKIPPED_DEPTH};

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

/// The lenient region of these vectors: everything under the root member `r`, as the oracle adapter names the recipes
/// of a case.
fn under_r(path: &[Option<String>]) -> bool {
    matches!(path, [Some(name), ..] if name == "r")
}

fn everywhere(_: &[Option<String>]) -> bool {
    true
}

fn nowhere(_: &[Option<String>]) -> bool {
    false
}

fn lenient(text: &str) -> Result<Value, &'static str> {
    json::parse_lenient(text.as_bytes(), under_r).map_err(|error| error.0)
}

/// `levels` nested arrays around `inner`.
fn deep(inner: &str, levels: usize) -> String {
    format!("{}{inner}{}", "[".repeat(levels), "]".repeat(levels))
}

#[test]
fn a_lenient_region_reads_lone_surrogates_and_deep_nesting_as_null() {
    let (lone, high, low) = (esc("d800"), esc("d83d"), esc("de00"));
    let text = format!(r#"{{"r":["{lone}","a{lone}b","{high}{low}",{{"{lone}":1,"k":"{lone}"}}],"s":"x"}}"#);
    assert_eq!(
        lenient(&text).unwrap().canonical(),
        format!(r#"{{"r":[null,null,"{high}{low}",{{"k":null}}],"s":"x"}}"#)
    );
    assert_eq!(lenient(&format!(r#"{{"r":"{lone}"}}"#)).unwrap().canonical(), r#"{"r":null}"#);
    // Two different lone names are two members, both left out; the same name twice is still refused.
    let names = |second: &str| format!(r#"{{"r":{{"{lone}":1,"{}":2}}}}"#, esc(second));
    assert_eq!(lenient(&names("d801")).unwrap().canonical(), r#"{"r":{}}"#);
    assert_eq!(lenient(&names("D800")), Err("Duplicate JSON key"));
    // The root object is at depth 0 and `r`'s value at depth 1, so 127 arrays remain around the null.
    let mut expected = Value::Null;
    for _ in 1..MAX_DEPTH {
        expected = Value::Array(vec![expected]);
    }
    let read = lenient(&format!(r#"{{"r":{}}}"#, deep("1", MAX_DEPTH + 5))).unwrap();
    assert_eq!(read.get("r"), Some(&expected));
    // Outside the region nothing changes.
    assert_eq!(lenient(&format!(r#"{{"r":1,"s":"{lone}"}}"#)), Err("Lone surrogate in JSON string"));
    assert_eq!(lenient(&format!(r#"{{"s":{{"{lone}":1}}}}"#)), Err("Lone surrogate in JSON string"));
    assert_eq!(lenient(&format!(r#"{{"s":{}}}"#, deep("1", MAX_DEPTH + 5))), Err("JSON nesting is too deep"));
}

#[test]
fn skipped_nesting_is_still_checked() {
    let lone = esc("d800");
    let skipped = |inner: &str| lenient(&format!(r#"{{"r":{}}}"#, deep(inner, MAX_DEPTH + 5)));
    let fine = format!(r#"{{"a":[{{}},[],"{lone}",-1.5e3,true,false,null],"b":{{"{lone}":1}}}}, " x ""#);
    assert_eq!(skipped(&fine).unwrap().canonical().matches('[').count(), MAX_DEPTH - 1);
    assert_eq!(skipped("1,"), Err("Malformed JSON value"));
    assert_eq!(skipped("1 2"), Err("Malformed JSON array"));
    assert_eq!(skipped(r#"{"a":1 "b":2}"#), Err("Malformed JSON object"));
    assert_eq!(skipped(r#"{"a" 1}"#), Err("Malformed JSON"));
    assert_eq!(skipped(r#"{"a":1,"a":2}"#), Err("Duplicate JSON key"));
    assert_eq!(skipped(&format!(r#"{{"{lone}":1,"{}":2}}"#, esc("D800"))), Err("Duplicate JSON key"));
    assert_eq!(skipped("tru"), Err("Malformed JSON literal"));
    assert_eq!(skipped("1."), Err("Malformed JSON number"));
    assert_eq!(skipped(r#""\x""#), Err("Malformed JSON escape"));
    assert_eq!(skipped("\"a\u{1}b\""), Err("Control character in JSON string"));
    assert_eq!(lenient(&format!(r#"{{"r":{}"#, "[".repeat(MAX_DEPTH + 5))), Err("Malformed JSON value"));
    assert_eq!(lenient(&format!(r#"{{"r":{} x"#, deep("1", MAX_DEPTH + 5))), Err("Malformed JSON object"));
    // Skipping is iterative and bounded.
    let nested = |levels: usize| deep("", levels);
    assert!(json::parse_lenient(nested(MAX_SKIPPED_DEPTH).as_bytes(), everywhere).is_ok());
    let refused = json::parse_lenient(nested(MAX_SKIPPED_DEPTH + 1).as_bytes(), everywhere);
    assert_eq!(refused, Err(JsonError("JSON nesting is too deep")));
}

#[test]
fn a_lenient_parse_reads_everything_else_as_the_strict_parse() {
    let (lone, high, low) = (esc("d800"), esc("d83d"), esc("de00"));
    let texts = [
        r#"{ "b" : [1, true, null, false], "a" : "x" }"#.to_owned(),
        format!(r#""caf{} {high}{low} / \" \\ \b\f\n\r\t""#, esc("00e9")),
        "[-0, 0, 12, -9223372036854775808, 9223372036854775808, 1.50, 1e2]".to_owned(),
        r#"{"z":{},"y":[],"":-5}"#.to_owned(),
        deep("1", MAX_DEPTH),
        deep("1", MAX_DEPTH + 1),
        format!(r#"["{lone}"]"#),
        format!(r#"{{"{lone}":1}}"#),
        r#"{"a":1,"a":2}"#.to_owned(),
        "[1,]".to_owned(),
        "[1 2]".to_owned(),
        "01".to_owned(),
        "\"abc".to_owned(),
    ];
    for text in &texts {
        assert_eq!(json::parse_lenient(text.as_bytes(), nowhere), parse(text.as_bytes()), "{text}");
    }
    // A lenient region changes nothing that the strict reader can represent.
    for text in &texts[..5] {
        assert_eq!(json::parse_lenient(text.as_bytes(), everywhere), parse(text.as_bytes()), "{text}");
    }
}
