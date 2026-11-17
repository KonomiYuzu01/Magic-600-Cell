//! Canonical output bytes (DET.59): `c600-json-1` (FP.5) with FP.6's exact and display numbers.
//!
//! - Keys are sorted by Unicode scalar order, separators are compact and the text ends in exactly one line feed.
//! - Strings escape `"` and `\`, use `\b \t \n \f \r`, and write other controls below U+0020 as lowercase `\u00xx`.
//! - Exact numbers are `{"int":"…"}`, `{"rat":["n","d"]}` and `{"q5":["a","b","d"]}`; an approximation is
//!   `{"f64":"<16 lowercase hex digits>"}`.
//! - A word is an array of its signed letters, each of magnitude at most 1200; a word with a larger letter has no form.
//!   No letter of either alphabet is larger (chapter 01): a 600-cell-Full generator is ±1 to ±1200, and a jumbling
//!   menu letter's grip and item are indices below 600. A letter of 0 is not a generator, but it keeps its form here:
//!   `c600-jumble-tool` reads a `Word` as flattened `[grip, item]` indices, which start at 0, and the kernel refuses
//!   a generator of 0 when it acts. This JSON form is an M2 draft; LG2.18's word keys stay open.
//! - A menu word (LB1.8; LP2 draft) is `{"letters":[[cap,item],…],"menu":{…}}`: its letters, each a pair of indices
//!   below 600, and its alphabet's identity block with its five fields under their names, so a stored or crossing
//!   word names its menu. A menu word whose block is empty names no alphabet and has no form.
//! - A list is an array of its elements in order (LP2 draft). A word equality key has no form: it is never output as
//!   letters (LG2.13).
//! - A set is an array of its distinct elements and a map an array of its `{"key":…,"value":…}` entries, both in
//!   canonical key order (DET.9; LP2 drafts). A set or map out of that order has no canonical form.
//! - A compressed word has no form in the LP2 draft; its letters are output after an explicit `expand`.
//!
//! The encoder walks with an explicit stack, so a deep value cannot overflow the host stack.

use std::cmp::Ordering;

use crate::value::{entry_key, value_order, Text, Value};

/// Why a value has no canonical form: a closure, a function, a receipt, a word equality key, a compressed word, an
/// approximation that is not exactly a binary64 number, a word with a letter of magnitude above 1200, a menu word that
/// names no alphabet, a record with a repeated key, or a set or map out of canonical key order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noncanonical;

enum Task<'a> {
    Value(&'a Value),
    Key(&'a Text),
    Byte(u8),
}

pub(crate) fn string(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
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

fn exact(kind: &str, parts: &[String], out: &mut Vec<u8>) {
    out.extend_from_slice(format!("{{\"{kind}\":").as_bytes());
    if let [one] = parts {
        string(one, out);
    } else {
        out.push(b'[');
        for (i, p) in parts.iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            string(p, out);
        }
        out.push(b']');
    }
    out.push(b'}');
}

/// Appends the canonical encoding of `value` and its terminal line feed to `out`. On an error `out` may hold part of an
/// encoding, which the caller discards.
pub fn encode(value: &Value, out: &mut Vec<u8>) -> Result<(), Noncanonical> {
    let mut stack = vec![Task::Value(value)];
    while let Some(task) = stack.pop() {
        let value = match task {
            Task::Byte(b) => {
                out.push(b);
                continue;
            }
            Task::Key(k) => {
                string(k.as_str(), out);
                out.push(b':');
                continue;
            }
            Task::Value(v) => v,
        };
        match value {
            Value::Unit => out.extend_from_slice(b"null"),
            Value::Bool(b) | Value::ApproxBool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
            Value::Int(n) => exact("int", &[n.to_str_radix(10)], out),
            Value::Rat(r) => exact("rat", &[r.num.to_str_radix(10), r.den.to_str_radix(10)], out),
            Value::Q5(q) => exact("q5", &[q.a.to_str_radix(10), q.b.to_str_radix(10), q.d.to_str_radix(10)], out),
            Value::Approx(x) => {
                let bits = x.f64_bits().ok_or(Noncanonical)?;
                exact("f64", &[format!("{bits:016x}")], out);
            }
            Value::Text(t) | Value::Target(t) | Value::Tag(t) | Value::Colour(t) => string(t.as_str(), out),
            // No alphabet has a letter of magnitude above 1200 (chapter 01). A literal is checked when it is read;
            // this check refuses a word built outside a run, so it never enters one as an argument, where inverses
            // negate letters.
            Value::Word(w) => {
                if w.iter().any(|l| l.unsigned_abs() > 1200) {
                    return Err(Noncanonical);
                }
                out.push(b'[');
                for (i, l) in w.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    out.extend_from_slice(l.to_string().as_bytes());
                }
                out.push(b']');
            }
            // The letters are bounded when the word is built (`value::MenuWord::new`). The identity block's fields are
            // written in key order: `contract_rev`, `menu_id`, `menu_order`, `model_id`, `puzzle`.
            Value::MenuWord(w) => {
                if !w.identity().names_alphabet() {
                    return Err(Noncanonical);
                }
                out.extend_from_slice(b"{\"letters\":[");
                for (k, [cap, item]) in w.letters().iter().enumerate() {
                    if k > 0 {
                        out.push(b',');
                    }
                    out.extend_from_slice(format!("[{cap},{item}]").as_bytes());
                }
                out.extend_from_slice(b"],\"menu\":{");
                let mut fields = w.identity().fields();
                fields.sort_by_key(|(name, _)| *name);
                for (k, (name, text)) in fields.iter().enumerate() {
                    if k > 0 {
                        out.push(b',');
                    }
                    string(name, out);
                    out.push(b':');
                    string(text, out);
                }
                out.extend_from_slice(b"}}");
            }
            // An end test's verdict: its outcome's tag and its test's row, in key order (LP2 draft).
            Value::EndTest(test, outcome) => {
                out.extend_from_slice(b"{\"outcome\":");
                string(outcome.tag(), out);
                out.extend_from_slice(b",\"test\":");
                string(test.row(), out);
                out.push(b'}');
            }
            Value::Record(r) => {
                let fields = r.fields();
                let mut order: Vec<usize> = (0..fields.len()).collect();
                order.sort_by(|&i, &j| fields[i].0.as_str().cmp(fields[j].0.as_str()));
                if order.windows(2).any(|w| fields[w[0]].0.as_str() == fields[w[1]].0.as_str()) {
                    return Err(Noncanonical);
                }
                out.push(b'{');
                stack.push(Task::Byte(b'}'));
                for (n, &i) in order.iter().enumerate().rev() {
                    stack.push(Task::Value(&fields[i].1));
                    stack.push(Task::Key(&fields[i].0));
                    if n > 0 {
                        stack.push(Task::Byte(b','));
                    }
                }
            }
            // A list is a JSON array of its elements in order; a set's elements and a map's `{ key, value }` entries
            // are arrays in canonical key order, which a value built outside a run must already have (LP2 drafts).
            Value::List(l) | Value::Set(l) | Value::Map(l) => {
                if !in_key_order(value) {
                    return Err(Noncanonical);
                }
                out.push(b'[');
                stack.push(Task::Byte(b']'));
                for (i, item) in l.items().iter().enumerate().rev() {
                    stack.push(Task::Value(item));
                    if i > 0 {
                        stack.push(Task::Byte(b','));
                    }
                }
            }
            // A word key is never output as letters (LG2.13); the LP2 draft gives it no canonical form. Nor does it give
            // one to a compressed word, whose output would be its priced expansion (DET.59).
            Value::Closure(_) | Value::Fn(_) | Value::Receipt(_) | Value::WordEqualityKey(_) | Value::Compressed(_) => {
                return Err(Noncanonical)
            }
        }
    }
    out.push(b'\n');
    Ok(())
}

/// Whether a set holds distinct elements in canonical key order, and a map `{ key, value }` entries under distinct keys
/// in that order (DET.9). Other values pass.
fn in_key_order(value: &Value) -> bool {
    let increasing = |keys: Vec<&Value>| keys.windows(2).all(|w| value_order(w[0], w[1]) == Ordering::Less);
    match value {
        Value::Set(s) => increasing(s.items().iter().collect()),
        Value::Map(m) => {
            let entry = |e: &Value| match e {
                Value::Record(r) => r.fields().iter().map(|(k, _)| k.as_str()).eq(["key", "value"]),
                _ => false,
            };
            m.items().iter().all(entry) && increasing(m.items().iter().map(entry_key).collect())
        }
        _ => true,
    }
}
