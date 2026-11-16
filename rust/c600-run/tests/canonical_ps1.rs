//! LP2 conformance tests for chapter 05's canonical profiles (PS1.3 to PS1.10) and the format packet's byte rules
//! (FP.5 to FP.9), with the 6,775-digit integer of CT-KN3-02 and the compression-metadata half of CT-DET-23.
//!
//! Every vector is read in place from the frozen format packet. The product has no general record reader and no gzip
//! writer yet: both belong to the session store (M6). These tests therefore compose the product's own pieces the way
//! such a reader must:
//! - FP.5's active read is `provenance::parse` followed by byte equality with `provenance::canonical`, which is what
//!   `provenance::read` does before FP.17's structural checks;
//! - FP.6's exact wrappers are read with `value::parse_integer`, `value::parse_ratio` and the reducing constructors,
//!   and must re-encode through the run-time encoder to the same bytes;
//! - decoded gzip bytes come from the kernel's retained-asset reader `archive::gunzip`, compiled in from its source.
//!
//! Evidence kind: source and fixture. One run covers one platform; nothing here is Windows/DirectX evidence.

#[path = "../../c600-check/tests/support/json.rs"]
#[allow(dead_code)]
mod json;

#[path = "../../c600-kernel/src/archive.rs"]
#[allow(dead_code)]
mod archive;

#[path = "../../c600-kernel/src/json.rs"]
#[allow(dead_code)]
mod retained;

use std::sync::Arc;

use c600_digest::sha256_hex;
use c600_ir::build::{exact, helper, module, program};
use c600_ir::{Expr, Lit, Ty};
use c600_run::approx::Approx;
use c600_run::encode::{encode, Noncanonical};
use c600_run::interp::{code, load, NoHost, Options};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, Json as Block, Pin};
use c600_run::value::{self, Quad, Ratio, Value};
use json::Json;
use num_bigint::BigInt;

const NONCANONICAL: &str = code::NONCANONICAL_VALUE;
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
const PACKET: &str = "docs/architecture/1.0/language/format-packet/";
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

fn repository_bytes(path: &str) -> Vec<u8> {
    let full = format!("{ROOT}{path}");
    std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"))
}

fn packet(name: &str) -> Json {
    let bytes = repository_bytes(&format!("{PACKET}{name}"));
    let text = String::from_utf8(bytes).expect("UTF-8");
    Json::parse(&text).unwrap_or_else(|e| panic!("{name} does not parse: {e}"))
}

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::str).unwrap_or_else(|| panic!("text field `{key}`"))
}

fn arr<'a>(j: &'a Json, key: &str) -> &'a [Json] {
    j.get(key).and_then(Json::arr).unwrap_or_else(|| panic!("array field `{key}`"))
}

fn hex(text: &str) -> Vec<u8> {
    assert!(text.len() % 2 == 0, "odd hex length");
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex digit")).collect()
}

/// A value of the vectors file in the product's JSON model. The vectors' JSON numbers are integer tokens.
fn block(j: &Json) -> Block {
    match j {
        Json::Null => Block::Null,
        Json::Bool(b) => Block::Bool(*b),
        Json::Num(n) => Block::Int(n.parse().unwrap_or_else(|e| panic!("`{n}` is no integer token: {e}"))),
        Json::Str(s) => Block::Text(s.clone()),
        Json::Arr(items) => Block::List(items.iter().map(block).collect()),
        Json::Obj(fields) => Block::Object(fields.iter().map(|(k, v)| (k.clone(), block(v))).collect()),
    }
}

/// FP.5's active read: strict parsing, then byte equality with the canonical re-encoding. Parseable different bytes
/// are refused like unparseable ones; the store's reader (M6) owns the split between this code and a format error.
fn read_new(bytes: &[u8]) -> Result<Block, &'static str> {
    let value = provenance::parse(bytes).ok_or(NONCANONICAL)?;
    if provenance::canonical(&value) == bytes {
        Ok(value)
    } else {
        Err(NONCANONICAL)
    }
}

fn encoded(v: &Value) -> Result<Vec<u8>, Noncanonical> {
    let mut out = Vec::new();
    encode(v, &mut out)?;
    Ok(out)
}

/// FP.6's exact wrappers, read with the product's integer and ratio parsers and reducing constructors. Readers refuse
/// unreduced values and never repair them, so the value must re-encode to the given wrapper's bytes. `f64` is a
/// display value and never an exact one.
fn exact_value(wire: &Block) -> Result<Value, &'static str> {
    let Block::Object(map) = wire else { return Err(NONCANONICAL) };
    let mut fields = map.iter();
    let (Some((kind, body)), None) = (fields.next(), fields.next()) else { return Err(NONCANONICAL) };
    let part = |j: &Block| j.as_text().map(str::to_string).ok_or(NONCANONICAL);
    let parts = |n: usize| match body {
        Block::List(items) if items.len() == n => items.iter().map(part).collect::<Result<Vec<_>, _>>(),
        _ => Err(NONCANONICAL),
    };
    let value = match kind.as_str() {
        "int" => Value::int(value::parse_integer(&part(body)?).ok_or(NONCANONICAL)?),
        "rat" => {
            let p = parts(2)?;
            let (n, d) = value::parse_ratio(&p[0], &p[1]).ok_or(NONCANONICAL)?;
            Value::Rat(Arc::new(Ratio::new(n, d)))
        }
        "q5" => {
            let p = parts(3)?;
            let (a, d) = value::parse_ratio(&p[0], &p[2]).ok_or(NONCANONICAL)?;
            let b = value::parse_integer(&p[1]).ok_or(NONCANONICAL)?;
            Value::Q5(Arc::new(Quad::new(a, b, d)))
        }
        _ => return Err(NONCANONICAL),
    };
    if encoded(&value).map_err(|_| NONCANONICAL)? != provenance::canonical(wire) {
        return Err(NONCANONICAL);
    }
    Ok(value)
}

/// The FP.6 large-integer vector, `10**6774 + 7`, in decimal.
fn large_integer() -> String {
    format!("1{}7", "0".repeat(6773))
}

/// CT-PS1-01 (part: one platform per run; not run: the headless Linux leg, since CI runs `cargo test` on its Windows
/// shards only and has no Linux job (no owner yet)): the format packet's six `c600-json-1` vectors give the frozen
/// bytes and digests through the product's writers (Unicode, every escape, scalar key order, no normalization and the
/// terminal LF); the retained profile keeps ASCII escapes and no LF; and the nine invalid texts are refused.
#[test]
fn ct_ps1_01_canonical_json_vectors() {
    let root = packet("vectors.json");
    let cases = arr(&root, "json");
    assert_eq!(cases.len(), 6, "the frozen file has six JSON vectors");
    for case in cases {
        let id = text(case, "id");
        let expected = hex(text(case, "expected_hex"));
        assert_eq!(sha256_hex(&expected), text(case, "expected_sha256"), "{id}: the vector's own digest");
        let given = block(case.get("given").expect("given"));
        let new = provenance::canonical(&given);
        match text(case, "profile") {
            "new" | "prototype" => {
                assert_eq!(new, expected, "{id}");
                assert_eq!(read_new(&expected).map(|v| provenance::canonical(&v)), Ok(expected.clone()), "{id}");
            }
            "retained" => {
                // The kernel's retained writer (Python's `json.dumps` with ASCII escapes) gives the retained bytes.
                let doc = retained::parse(&new).expect("valid JSON");
                assert_eq!(doc.canonical().as_bytes(), &expected[..], "{id}");
                assert_eq!(read_new(&expected).err(), Some(NONCANONICAL), "{id}: the profiles stay distinct");
            }
            other => panic!("{id}: unknown profile {other}"),
        }
    }

    // The run-time encoder writes the same string escapes, and orders keys by scalar, not by UTF-16 unit.
    let escaping = cases.iter().find(|c| text(c, "id") == "escaping").expect("the escaping vector");
    let s = escaping.get("given").and_then(|g| g.get("text")).and_then(Json::str).expect("text");
    let record = Value::record(vec![("text", Value::text(s))]);
    assert_eq!(encoded(&record).unwrap(), hex(text(escaping, "expected_hex")));
    let keys = Value::record(vec![("\u{10000}", Value::Unit), ("\u{e000}", Value::Unit)]);
    assert_eq!(encoded(&keys).unwrap(), "{\"\u{e000}\":null,\"\u{10000}\":null}\n".as_bytes());
    let composed = Value::record(vec![("b", Value::text("e\u{301}")), ("a", Value::text("\u{e9}"))]);
    assert_eq!(encoded(&composed).unwrap(), "{\"a\":\"\u{e9}\",\"b\":\"e\u{301}\"}\n".as_bytes());

    let invalid = arr(&root, "invalid_json");
    assert_eq!(invalid.len(), 9, "the frozen file has nine invalid JSON texts");
    for case in invalid {
        let id = text(case, "id");
        assert_eq!(read_new(text(case, "given").as_bytes()).err(), Some(text(case, "expected")), "{id}");
    }
}

/// CT-PS1-02 (part; not run: the store's FP.6 record reader and its keep-the-source rule after a digit refusal
/// (PS1.33), both M6): exact integers beyond 2^64, reduced rationals and Q(√5) triples round-trip byte for byte through
/// the product's parsers, exact values and encoder; integers never pass through a float; the 6,775-digit vector
/// round-trips; and a 10,001-digit integer is refused without changing the text it was given.
#[test]
fn ct_ps1_02_exact_numbers_round_trip_without_floats() {
    let root = packet("vectors.json");
    let mut accepted = Vec::new();
    for case in arr(&root, "numbers") {
        let id = text(case, "id");
        if text(case, "expected") != "accepted" {
            continue;
        }
        let wire = block(case.get("given").expect("given"));
        let expected = hex(text(case, "expected_hex"));
        assert_eq!(provenance::canonical(&wire), expected, "{id}: the wrapper's canonical bytes");
        assert_eq!(sha256_hex(&expected), text(case, "expected_sha256"), "{id}");
        if id == "f64" {
            // Display-only binary64: the writer gives the vector's bytes for 1.0.
            let one = Value::Approx(Approx::parse(53, "1").expect("a decimal"));
            assert_eq!(encoded(&one).unwrap(), expected, "{id}");
        } else {
            let value = exact_value(&read_new(&expected).unwrap()).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(encoded(&value).unwrap(), expected, "{id}: round trip");
        }
        accepted.push(id);
    }
    assert_eq!(accepted, ["beyond-u64", "negative-big", "rational", "rational-zero", "q5", "f64"]);

    // Beyond 2^64 and 2^128, and 2^53 + 1, which a binary64 conversion would round to 2^53.
    for digits in ["18446744073709551617", "-340282366920938463463374607431768211457", "9007199254740993"] {
        let n = value::parse_integer(digits).expect("canonical integer text");
        let bytes = encoded(&Value::int(n.clone())).unwrap();
        assert_eq!(bytes, format!("{{\"int\":\"{digits}\"}}\n").into_bytes());
        let back = exact_value(&read_new(&bytes).unwrap()).unwrap();
        assert!(value::equal(&back, &Value::int(n)), "{digits}");
    }
    assert_ne!(9_007_199_254_740_993u64 as f64 as u64, 9_007_199_254_740_993, "the float path would round");

    // Rationals stay reduced, and a Q(√5) triple keeps its type with a zero √5 coefficient.
    let sum = value::add(&exact_value(&read_new(b"{\"rat\":[\"1\",\"6\"]}\n").unwrap()).unwrap(), &Value::int(0));
    assert_eq!(encoded(&sum.unwrap()).unwrap(), b"{\"rat\":[\"1\",\"6\"]}\n");
    let q5_one = exact_value(&read_new(b"{\"q5\":[\"1\",\"0\",\"1\"]}\n").unwrap()).unwrap();
    assert!(matches!(q5_one, Value::Q5(_)));

    // The 6,775-digit vector, signed and unsigned, and the digit limit just above it.
    let vector = root.get("large_integer").expect("large_integer");
    let digits = large_integer();
    assert_eq!(digits.len(), 6775);
    let bytes = encoded(&Value::int(value::parse_integer(&digits).unwrap())).unwrap();
    assert_eq!(bytes.len().to_string(), vector.get("expected_bytes").and_then(Json::num).unwrap());
    assert_eq!(sha256_hex(&bytes), text(vector, "expected_sha256"));
    assert_eq!(encoded(&exact_value(&read_new(&bytes).unwrap()).unwrap()).unwrap(), bytes);
    let negative = format!("-{digits}");
    assert!(value::parse_integer(&negative).is_some(), "the digit count excludes the sign");
    let over = "9".repeat(value::MAX_DIGITS + 1);
    let before = over.clone();
    assert_eq!(value::parse_integer(&over), None);
    assert_eq!(over, before, "a refusal leaves the given text as it was");
}

/// CT-PS1-03 (part; not run: the store's own reader of persisted Q(√5) entries and matrices, and its refusal path for
/// display wrappers (M6)): invalid Q(√5) denominators and unreduced triples, unreduced or negative-denominator
/// rationals, non-canonical integer text, duplicate keys and floats are refused; an `f64` wrapper is no exact value;
/// and the run-time writer never emits a non-finite or negative-zero display value.
#[test]
fn ct_ps1_03_invalid_exact_values_are_refused() {
    let root = packet("vectors.json");
    let mut refused = 0;
    for case in arr(&root, "numbers") {
        let id = text(case, "id");
        let expected = text(case, "expected");
        if expected == "accepted" {
            continue;
        }
        assert_eq!(expected, NONCANONICAL, "{id}");
        let wire = block(case.get("given").expect("given"));
        let bytes = provenance::canonical(&wire);
        assert_eq!(read_new(&bytes).and_then(|w| exact_value(&w)).err(), Some(NONCANONICAL), "{id}");
        refused += 1;
    }
    assert_eq!(refused, 9);

    for bad in [
        // Q(√5): zero and negative denominators, and an unreduced triple.
        "{\"q5\":[\"1\",\"1\",\"0\"]}\n",
        "{\"q5\":[\"1\",\"1\",\"-2\"]}\n",
        "{\"q5\":[\"0\",\"0\",\"5\"]}\n",
        "{\"q5\":[\"1\",\"2\"]}\n",
        // Rationals: zero denominator, unreduced zero.
        "{\"rat\":[\"1\",\"0\"]}\n",
        "{\"rat\":[\"0\",\"5\"]}\n",
        // A number token where FP.6 wants decimal text, a float and an exponent in a decision field.
        "{\"int\":5}\n",
        "{\"int\":1.0}\n",
        "{\"q5\":[\"1\",2e0,\"3\"]}\n",
        // Duplicate keys inside a wrapper, and two wrappers in one object.
        "{\"int\":\"1\",\"int\":\"2\"}\n",
        "{\"int\":\"1\",\"rat\":[\"1\",\"2\"]}\n",
        // A display value in an exact field.
        "{\"f64\":\"3ff0000000000000\"}\n",
    ] {
        assert_eq!(read_new(bad.as_bytes()).and_then(|w| exact_value(&w)).err(), Some(NONCANONICAL), "{bad}");
    }

    // Writers map either zero to positive zero and have no infinity or NaN: what is not binary64 is not written.
    for (decimal, bits) in [("0", Some(0)), ("-0", Some(0)), ("-1", Some(0xbff0_0000_0000_0000u64))] {
        assert_eq!(Approx::parse(53, decimal).unwrap().f64_bits(), bits, "{decimal}");
    }
    for decimal in ["1e400", "-1e400", "1e-400"] {
        let x = Value::Approx(Approx::parse(53, decimal).expect("a decimal"));
        assert_eq!(encoded(&x), Err(Noncanonical), "{decimal}");
    }
}

/// CT-PS1-05 (part; not run: checkouts on other machines and operating systems, and a reader of captured proof sources
/// (no owner yet)): the R-03 rule holds in this checkout. Line-ending conversion is off, the seven pinned format-packet
/// files have their pinned digests, the inventory digest is FP.5's digest of the pins, and changed bytes give a
/// different source pin and a refused canonical block instead of the old digest (PS1.8).
#[test]
fn ct_ps1_05_pinned_bytes_survive_checkout() {
    let attributes = String::from_utf8(repository_bytes(".gitattributes")).unwrap();
    assert!(attributes.lines().any(|l| l.trim() == "* -text"), "line-ending conversion is off");

    let inventory = packet("candidate.json");
    let pins = arr(&inventory, "candidate");
    assert_eq!(pins.len(), 7);
    for pin in pins {
        let path = text(pin, "path");
        let bytes = repository_bytes(path);
        assert_eq!(sha256_hex(&bytes), text(pin, "sha256"), "{path}: checkout bytes differ from the pin");
        assert!(!bytes.windows(2).any(|w| w == b"\r\n"), "{path}: new architecture text is LF only");
    }
    let listed = provenance::canonical(&block(inventory.get("candidate").unwrap()));
    assert_eq!(sha256_hex(&listed), text(&inventory, "candidate_sha256"), "FP.1's inventory digest");

    // A CRLF copy is other bytes: its pin differs, and a canonical block read back from it is refused.
    let source = repository_bytes(&format!("{PACKET}check_vectors.py"));
    let crlf: Vec<u8> = source.iter().flat_map(|&b| if b == b'\n' { vec![b'\r', b'\n'] } else { vec![b] }).collect();
    assert_ne!(Pin::of("p3", &source), Pin::of("p3", &crlf));
    let stored = provenance::canonical(&provenance::manual(&Pin::of("p3", &source)));
    assert!(provenance::read(&stored).is_ok());
    let mut converted = stored.clone();
    converted.pop();
    converted.extend_from_slice(b"\r\n");
    assert_eq!(provenance::read(&converted).err(), Some(provenance::INCOMPATIBLE));
    assert_eq!(read_new(&converted).err(), Some(NONCANONICAL));
}

const HEADER: &str = "1f8b08000000000000ff";

/// FP.7's `c600-gzip-stored-1`, written from the clause for these tests: the fixed header, stored blocks of at most
/// 65,535 bytes (`00` before a non-final block, `01` before the final one, LEN and NLEN in little-endian order), one
/// empty final block for empty input, and the CRC32 and size trailer. It is a fixture; the product writer is M6's.
fn stored_gzip(data: &[u8]) -> Vec<u8> {
    let mut out = hex(HEADER);
    let mut blocks: Vec<&[u8]> = data.chunks(65_535).collect();
    if blocks.is_empty() {
        blocks.push(&[]);
    }
    let last = blocks.len() - 1;
    for (i, block) in blocks.into_iter().enumerate() {
        out.push((i == last) as u8);
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&archive::crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

fn overhead(n: usize) -> usize {
    18 + 5 * n.div_ceil(65_535).max(1)
}

/// CT-PS1-06 (part; not run: the product's FP.7 writer and its strict one-member reader, both M6; the kernel's
/// `gunzip` is the retained-asset reader): the three gzip vectors are exactly the selected profile (fixed header, the
/// stored-block algorithm, CRC32 and size trailer, FP.7's overhead), their stored digests are the frozen ones, and
/// their decoded bytes and decoded digests are checked separately from them. The decoder bounds output before it
/// expands a block.
#[test]
fn ct_ps1_06_gzip_vectors_follow_the_stored_profile() {
    let root = packet("vectors.json");
    let cases = arr(&root, "gzip");
    assert_eq!(cases.len(), 3);
    for case in cases {
        let id = text(case, "id");
        let given = text(case, "given").as_bytes();
        let stored = hex(text(case, "expected_hex"));
        assert_eq!(sha256_hex(&stored), text(case, "expected_sha256"), "{id}: stored digest");
        assert_eq!(stored, stored_gzip(given), "{id}: the profile's own bytes");
        assert_eq!(&stored[..10], &hex(HEADER)[..], "{id}: header");
        assert_eq!(stored.len(), given.len() + overhead(given.len()), "{id}: overhead");

        let decoded = archive::gunzip(&stored, given.len()).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(decoded, given, "{id}: decoded bytes");
        assert_eq!(sha256_hex(&decoded), sha256_hex(given), "{id}: decoded digest");
        assert_ne!(sha256_hex(&decoded), sha256_hex(&stored), "{id}: the two digests differ");
        if !given.is_empty() {
            assert!(archive::gunzip(&stored, given.len() - 1).is_err(), "{id}: output bounded before expansion");
        }
    }

    // The block boundary: 65,535 bytes take one block, 65,536 take two.
    for n in [65_535, 65_536] {
        let data: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
        let stored = stored_gzip(&data);
        assert_eq!(stored.len() - n, overhead(n), "{n}");
        assert_eq!(stored[10], (n <= 65_535) as u8, "{n}: the first block is final only when it holds everything");
        assert_eq!(archive::gunzip(&stored, n).unwrap(), data, "{n}");
    }
}

fn nested(containers: usize, open: &str, inner: &str, close: &str) -> Vec<u8> {
    format!("{}{inner}{}\n", open.repeat(containers), close.repeat(containers)).into_bytes()
}

/// CT-PS1-07 (part: the integer_digits and json_depth rows and the decoded-size bound of the gzip reader; not run:
/// the store's record, source, graph, array, bundle and archive rows (M6), the legacy log, part and preference rows
/// (the importer, no owner yet); the recipe rows are the kernel's `recipe_limits_are_those_of_0_4` (M4)): each runnable
/// row is accepted at its boundary and refused one unit above, and neither stored blocks nor a second member let the
/// cumulative decoded size pass its bound.
#[test]
fn ct_ps1_07_limit_rows_hold_at_their_boundary() {
    let root = packet("vectors.json");
    let limits = root.get("limits").expect("limits");
    let row = |name: &str| -> (usize, usize) {
        let given = arr(limits.get(name).expect("a row"), "given");
        (given[0].num().unwrap().parse().unwrap(), given[1].num().unwrap().parse().unwrap())
    };

    // integer_digits: the parser, the FP.6 reader and the IR loader agree; the sign is not a digit.
    let (digits, above) = row("integer_digits");
    assert_eq!((digits, above), (value::MAX_DIGITS, value::MAX_DIGITS + 1));
    let at = format!("1{}", "0".repeat(digits - 1));
    let over = format!("1{}", "0".repeat(above - 1));
    for (t, ok) in [(at.clone(), true), (format!("-{at}"), true), (over.clone(), false), (format!("-{over}"), false)] {
        assert_eq!(value::parse_integer(&t).is_some(), ok, "{} digits", t.trim_start_matches('-').len());
        let wire = format!("{{\"int\":\"{t}\"}}\n");
        assert_eq!(read_new(wire.as_bytes()).and_then(|w| exact_value(&w)).is_ok(), ok);
        let lit = program(vec![module("m", vec![helper("f", vec![], exact(Ty::Int), Expr::Lit(Lit::Int(t)))])]);
        match load(&lit) {
            Ok(loaded) => {
                assert!(ok);
                let run = loaded.run("m", "f", vec![], &mut NoHost, &Options::new(WIDE)).unwrap();
                assert_eq!(run.summary.outcome, Outcome::Completed);
            }
            Err(refusal) => {
                assert!(!ok);
                assert_eq!(refusal.code, code::IR_REJECTED);
            }
        }
    }

    // json_depth: containers on the deepest path, whether the innermost container is empty or holds a scalar.
    let (depth, above) = row("json_depth");
    for (open, inner, close) in [("[", "", "]"), ("[", "1", "]"), ("{\"k\":", "{}", "}"), ("{\"k\":", "1", "}")] {
        // An object path written as `{"k":` repeated ends in its own innermost container `{}`.
        let shift = usize::from(inner == "{}");
        let at = nested(depth - shift, open, inner, close);
        let over = nested(above - shift, open, inner, close);
        assert!(read_new(&at).is_ok(), "{depth} containers of {open}{inner}{close}");
        assert_eq!(read_new(&over).err(), Some(NONCANONICAL), "{above} containers of {open}{inner}{close}");
    }

    // Decoded size: the bound covers every stored block of a member and every member, not each one alone.
    let data: Vec<u8> = (0..70_000u32).map(|i| (i % 253) as u8).collect();
    let stored = stored_gzip(&data);
    assert_eq!(archive::gunzip(&stored, data.len()).unwrap(), data);
    assert!(archive::gunzip(&stored, data.len() - 1).is_err(), "two blocks, each within the bound");
    let (a, b) = (stored_gzip(&data[..40_000]), stored_gzip(&data[40_000..]));
    let two = [a, b].concat();
    let refused = archive::gunzip(&two, data.len() - 1).expect_err("two members, each within the bound");
    assert_eq!(refused.0, "DEFLATE output exceeds its limit");
}

/// CT-KN3-02 (part: the 6,775-digit integer; not run: synthetic pose entries beyond 2^64, which wait for the jumbling
/// half's pose store (M5)): the integer round-trips exactly through the exact value, exact arithmetic, the canonical
/// codec and an interpreted program's output.
#[test]
fn ct_kn3_02_a_6775_digit_integer_round_trips() {
    let digits = large_integer();
    let n = value::parse_integer(&digits).expect("within the digit limit");
    assert_eq!(n.to_string(), digits);
    let bytes = encoded(&Value::int(n.clone())).unwrap();
    let back = exact_value(&read_new(&bytes).unwrap()).unwrap();
    assert!(value::equal(&back, &Value::int(n.clone())));
    let next = value::add(&back, &Value::int(1)).unwrap();
    assert!(value::equal(&next, &Value::int(&n + 1)));
    assert_eq!(encoded(&next).unwrap(), format!("{{\"int\":\"1{}8\"}}\n", "0".repeat(6773)).into_bytes());
    assert!(BigInt::from(u64::MAX) < n);

    let lit = program(vec![module("m", vec![helper("f", vec![], exact(Ty::Nat), Expr::Lit(Lit::Nat(digits)))])]);
    let loaded = load(&lit).expect("the literal loads");
    let run = loaded.run("m", "f", vec![], &mut NoHost, &Options::new(WIDE)).unwrap();
    assert_eq!(run.output, Some(bytes));
}

/// CT-DET-23 (part: content identity under compression metadata; not run: atomic publication and its interruption at
/// every boundary, which wait for the session store's publication path (M6)): stored bytes that differ only in gzip
/// metadata (mtime, OS, a file name) have different stored digests and the same decoded bytes, so the canonical content
/// digest and the exact value read from it are unchanged (DET.47).
#[test]
fn ct_det_23_content_identity_survives_compression_metadata() {
    let root = packet("vectors.json");
    let case = arr(&root, "gzip").iter().find(|c| text(c, "id") == "gzip-json").expect("gzip-json");
    let stored = hex(text(case, "expected_hex"));
    let mut timed = stored.clone();
    timed[4..8].copy_from_slice(&0x6553_f100u32.to_le_bytes());
    timed[9] = 3;
    let mut named = stored[..10].to_vec();
    named[3] = 0x08;
    named.extend_from_slice(b"record.json\0");
    named.extend_from_slice(&stored[10..]);

    let content = archive::gunzip(&stored, 1 << 20).unwrap();
    let identity = sha256_hex(&provenance::canonical(&read_new(&content).unwrap()));
    for variant in [&timed, &named] {
        assert_ne!(sha256_hex(variant), sha256_hex(&stored), "the stored digest sees the metadata");
        let decoded = archive::gunzip(variant, 1 << 20).unwrap();
        assert_eq!(decoded, content);
        assert_eq!(sha256_hex(&provenance::canonical(&read_new(&decoded).unwrap())), identity);
    }
}
