//! LP2 plan, M2: the 40 frozen contract-vector traces (DET.65) replayed through the reference meter, and the meter
//! boundary vectors derived from them.
//!
//! The replay follows the abstract model of `check_vectors.py`. Its frames, named owners and abstract operations
//! belong to the fixture, not to the interpreter. Every charge, owner, release, child scope, generator draw and
//! tariff comes from the library.

#[path = "../../c600-check/tests/support/json.rs"]
#[allow(dead_code)]
mod json;

use c600_check::check;
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::{
    code, frame_size, integer_size, Amounts, Dim, IntOp, Meter, Outcome, OwnerId, Position, SourceRange, Stop, Tariffs,
    GENERATOR_SIZE,
};
use c600_run::rng::{stream_key, Cursor};
use c600_run::value::Value;
use c600_syntax::{lower, parse};
use json::Json;
use num_bigint::{BigInt, BigUint};
use std::collections::HashMap;

const VECTORS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/architecture/1.0/language/contract-vectors/vectors.json");

fn fixtures() -> Json {
    let text = std::fs::read_to_string(VECTORS).unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    Json::parse(&text).unwrap_or_else(|e| panic!("vectors.json does not parse: {e}"))
}

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::str).unwrap_or_else(|| panic!("text field `{key}`"))
}

fn arr<'a>(j: &'a Json, key: &str) -> &'a [Json] {
    j.get(key).and_then(Json::arr).unwrap_or_else(|| panic!("array field `{key}`"))
}

fn uint(j: &Json, key: &str) -> u128 {
    let n = j.get(key).and_then(Json::num).unwrap_or_else(|| panic!("number field `{key}`"));
    n.parse().unwrap_or_else(|e| panic!("`{key}` is not a nonnegative integer: {e}"))
}

fn uint_or(j: &Json, key: &str, default: u128) -> u128 {
    if j.get(key).is_some() {
        uint(j, key)
    } else {
        default
    }
}

/// An exact integer given as a number or as decimal text (`int(op["a"])`).
fn int(j: &Json, key: &str) -> BigInt {
    let field = j.get(key).unwrap_or_else(|| panic!("integer field `{key}`"));
    let digits = field.num().or(field.str()).expect("an integer");
    digits.parse().unwrap_or_else(|e| panic!("`{key}` is not an integer: {e}"))
}

fn hex(text: &str) -> Vec<u8> {
    assert!(text.len() % 2 == 0, "odd hex length");
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex digit")).collect()
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn amounts(j: &Json) -> Amounts {
    Amounts {
        steps: uint(j, "steps"),
        kernel_cost: uint(j, "kernel_cost"),
        memory: uint(j, "memory"),
        output: uint(j, "output"),
    }
}

fn source_range(j: &Json) -> SourceRange {
    SourceRange {
        digest: text(j, "source").to_string(),
        start: uint(j, "start") as usize,
        end: uint(j, "end") as usize,
    }
}

fn num(n: impl ToString) -> Json {
    Json::Num(n.to_string())
}

fn obj(fields: Vec<(&str, Json)>) -> Json {
    Json::Obj(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn amounts_json(a: Amounts) -> Json {
    obj(vec![
        ("steps", num(a.steps)),
        ("kernel_cost", num(a.kernel_cost)),
        ("memory", num(a.memory)),
        ("output", num(a.output)),
    ])
}

fn position_json(p: &Option<Position>) -> Json {
    let Some(p) = p else { return Json::Null };
    let mut fields = vec![
        ("at", Json::Str(p.at.clone())),
        ("phase", Json::Str(p.phase.into())),
        ("scope", Json::Str(p.scope.clone())),
    ];
    if let Some(s) = &p.source {
        fields.push((
            "source",
            obj(vec![("source", Json::Str(s.digest.clone())), ("start", num(s.start)), ("end", num(s.end))]),
        ));
    }
    obj(fields)
}

fn code_json(code: Option<&str>) -> Json {
    code.map_or(Json::Null, |c| Json::Str(c.into()))
}

/// Dictionary assignment: replace the value of `key`, or add it.
fn set<V>(map: &mut Vec<(String, V)>, key: &str, value: V) {
    match map.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = value,
        None => map.push((key.to_string(), value)),
    }
}

fn cursor_name(frame: &str, name: &str) -> String {
    if frame == "root" {
        name.to_string()
    } else {
        format!("{frame}:{name}")
    }
}

/// A live named owner of the abstract model.
struct Named {
    frame: String,
    name: String,
    id: OwnerId,
    cursor: Option<usize>,
}

/// `check_vectors.py`'s `Model` over the library meter.
struct Model<'a> {
    given: &'a Json,
    t: Tariffs,
    meter: Meter,
    owners: Vec<Named>,
    frames: HashMap<String, Option<String>>,
    cursors: Vec<Cursor>,
    generators: Vec<(String, usize)>,
    values: Vec<(String, Json)>,
    coverage: Vec<(String, u128)>,
}

impl<'a> Model<'a> {
    fn new(given: &'a Json, limits: Amounts, t: Tariffs) -> Model<'a> {
        Model {
            given,
            t,
            meter: Meter::new(limits),
            owners: Vec::new(),
            frames: HashMap::from([("root".to_string(), None)]),
            cursors: Vec::new(),
            generators: Vec::new(),
            values: Vec::new(),
            coverage: Vec::new(),
        }
    }

    fn source(&self, at: &str) -> Option<SourceRange> {
        let mapped = self.given.get("source_map").and_then(|m| m.get(at));
        mapped.or_else(|| self.given.get("source")).map(source_range)
    }

    fn charge(&mut self, at: &str, kernel: u128, memory: u128, output: u128) -> Result<(), Stop> {
        let source = self.source(at);
        self.meter.charge(at, source.as_ref(), kernel, memory, output)
    }

    fn own(&mut self, frame: &str, name: &str, size: u128) -> usize {
        assert!(!self.owners.iter().any(|o| o.frame == frame && o.name == name), "duplicate live owner");
        let id = self.meter.own(size);
        self.owners.push(Named { frame: frame.to_string(), name: name.to_string(), id, cursor: None });
        self.owners.len() - 1
    }

    fn bind_cursor(&mut self, owner: usize, cursor: Cursor) {
        self.cursors.push(cursor);
        let c = self.cursors.len() - 1;
        self.owners[owner].cursor = Some(c);
        let name = cursor_name(&self.owners[owner].frame, &self.owners[owner].name);
        set(&mut self.generators, &name, c);
    }

    fn lookup(&self, frame: &str, name: &str) -> usize {
        let mut frame = Some(frame.to_string());
        while let Some(f) = frame {
            if let Some(i) = self.owners.iter().position(|o| o.frame == f && o.name == name) {
                return i;
            }
            frame = self.frames[&f].clone();
        }
        panic!("missing owner: {name}")
    }

    fn drop_named(&mut self, index: usize) {
        let owner = self.owners.remove(index);
        self.meter.drop_owner(owner.id, true);
    }

    fn close_frame(&mut self, frame: &str, keep: Option<&str>, target: Option<&str>) -> u128 {
        let mut transferred = 0;
        let names: Vec<String> = self.owners.iter().filter(|o| o.frame == frame).map(|o| o.name.clone()).collect();
        for name in names {
            let i = self.owners.iter().position(|o| o.frame == frame && o.name == name).expect("owner");
            if Some(name.as_str()) != keep {
                self.drop_named(i);
                continue;
            }
            let mut owner = self.owners.remove(i);
            let parent = self.frames[frame].clone().expect("a frame has a parent");
            let target = target.expect("a returned owner names its target");
            assert!(
                !self.owners.iter().any(|o| o.frame == parent && o.name == target),
                "return overwrites a live owner"
            );
            if let Some(c) = owner.cursor {
                self.generators.retain(|(n, _)| *n != cursor_name(frame, &name));
                set(&mut self.generators, &cursor_name(&parent, target), c);
            }
            (owner.frame, owner.name) = (parent, target.to_string());
            transferred = self.meter.size(owner.id);
            self.owners.push(owner);
        }
        transferred
    }

    fn call(&mut self, op: &Json, at: &str, parent: &str) -> Result<u128, Stop> {
        let frame = format!("{at}/frame");
        let size = frame_size(uint_or(op, "slots", 0));
        self.charge(&format!("{at}/call"), 0, size, 0)?;
        self.frames.insert(frame.clone(), Some(parent.to_string()));
        self.own(&frame, "#frame", size);
        self.run(arr(op, "body"), &format!("{at}/"), &frame)?;
        self.charge(&format!("{at}/return"), 0, 0, 0)?;
        Ok(self.close_frame(&frame, op.get("returns").and_then(Json::str), op.get("as").and_then(Json::str)))
    }

    fn child(&mut self, op: &Json, at: &str, frame: &str) -> Result<(), Stop> {
        let source = self.source(&format!("{at}/child"));
        let child = self.meter.enter_child(at, source.as_ref(), amounts(op.get("limits").expect("limits")))?;
        let result = self.call(op, &format!("{at}/run"), frame);
        let left = self.meter.leave_child(child, result);
        // An unwound child's owners are gone from the meter; their names go too.
        let meter = &self.meter;
        self.owners.retain(|o| meter.is_live(o.id));
        left
    }

    fn run(&mut self, trace: &[Json], prefix: &str, frame: &str) -> Result<(), Stop> {
        let t = self.t;
        let region = self.given.get("execution_contract").and_then(Json::str) == Some("lp1-exec-region-draft-1");
        for op in trace {
            let kind = text(op, "op");
            let at = format!("{prefix}{}", text(op, "at"));
            match kind {
                "display_region" | "display_branch" => assert!(region, "display region needs its own identity"),
                "display_if" => assert!(!region, "form-1 display_if cannot be relabelled as form 2"),
                _ => {}
            }
            match kind {
                "tick" | "assert" | "flow_check" => {
                    self.charge(&at, 0, 0, 0)?;
                    let value = op.get("value").and_then(Json::bool);
                    if kind == "assert" && value != Some(true) {
                        return Err(self.meter.error(code::POINTWISE_FAILED, &at, None));
                    }
                    if kind == "flow_check" {
                        assert_eq!(value, Some(true), "fixtures must not select a reserved flow refusal");
                    }
                }
                "alloc" => {
                    let bytes = uint(op, "bytes");
                    self.charge(&at, 0, bytes, 0)?;
                    self.own(frame, text(op, "name"), bytes);
                }
                "alias" => {
                    let source = self.lookup(frame, text(op, "source"));
                    let size = self.meter.size(self.owners[source].id);
                    let cursor = self.owners[source].cursor.map(|c| self.cursors[c].clone());
                    self.charge(&at, 0, size, 0)?;
                    let owner = self.own(frame, text(op, "name"), size);
                    if let Some(cursor) = cursor {
                        self.bind_cursor(owner, cursor);
                    }
                }
                "drop" => {
                    let owner = self.lookup(frame, text(op, "name"));
                    self.charge(&at, 0, 0, 0)?;
                    self.drop_named(owner);
                }
                "call" => {
                    self.call(op, &at, frame)?;
                }
                "child" => self.child(op, &at, frame)?,
                "display_region" => {
                    let scope = format!("{at}/scope");
                    self.charge(&format!("{at}/display_enter"), 0, 0, 0)?;
                    self.frames.insert(scope.clone(), Some(frame.to_string()));
                    self.run(arr(op, "body"), &format!("{at}/"), &scope)?;
                    self.charge(&format!("{at}/display_exit"), 0, 0, 0)?;
                    self.close_frame(&scope, op.get("returns").and_then(Json::str), op.get("as").and_then(Json::str));
                }
                "loop" => {
                    let count = uint(op, "count");
                    for index in 0..=count {
                        self.charge(&format!("{at}/test:{index}"), 0, 0, 0)?;
                        if index < count {
                            self.run(arr(op, "body"), &format!("{at}/body:{index}/"), frame)?;
                        }
                    }
                }
                "branch" | "display_if" | "display_branch" => {
                    self.charge(&at, 0, 0, 0)?;
                    let branch =
                        if op.get("condition").and_then(Json::bool).expect("condition") { "then" } else { "else" };
                    self.run(arr(op, branch), &format!("{at}/{branch}/"), frame)?;
                }
                "kernel" => {
                    let (size, scratch) = (uint_or(op, "result_bytes", 0), uint_or(op, "scratch", 0));
                    self.charge(&at, t.kernel(uint(op, "items")), size + scratch, 0)?;
                    if size > 0 {
                        self.own(frame, text(op, "name"), size);
                    }
                    self.meter.release(scratch);
                }
                "add" | "mul" => {
                    let (a, b) = (int(op, "a"), int(op, "b"));
                    let row = if kind == "add" { IntOp::Add } else { IntOp::Mul };
                    let (cost, bound) = t.integer(row, a.bits().into(), b.bits().into());
                    self.charge(&at, cost, bound, 0)?;
                    let value = if kind == "add" { a + b } else { a * b };
                    let size = integer_size(value.bits().into());
                    self.own(frame, text(op, "name"), size);
                    self.meter.release(bound - size);
                    set(&mut self.values, &at, num(value));
                }
                "text_compare" => {
                    let (a, b) = (text(op, "a"), text(op, "b"));
                    let scalars = (a.chars().count() + b.chars().count()) as u128;
                    self.charge(&at, scalars * t.text_per_scalar, 0, 0)?;
                    set(&mut self.values, &at, num(a.cmp(b) as i8));
                }
                "text_decode" => {
                    let data = hex(text(op, "hex"));
                    let n = data.len() as u128;
                    self.charge(&at, n * t.decode_per_byte, 4 * n, 0)?;
                    let Ok(value) = std::str::from_utf8(&data) else {
                        return Err(self.meter.error(code::TEXT_SCALAR_INVALID, &at, None));
                    };
                    let size = 4 * value.chars().count() as u128;
                    self.own(frame, text(op, "name"), size);
                    self.meter.release(4 * n - size);
                    set(&mut self.values, &at, Json::Str(value.to_string()));
                }
                "emit" => {
                    let bytes = canonical(op.get("value").expect("value"));
                    let size = bytes.len() as u128;
                    self.charge(&at, size * t.encode_per_byte, size, size)?;
                    self.meter.release(size);
                    set(&mut self.values, &at, Json::Str(hex_of(&bytes)));
                }
                "rng_init" => {
                    let path: Vec<&str> = arr(op, "path").iter().map(|s| s.str().expect("path segment")).collect();
                    let (key, n) = stream_key(&hex(text(op, "seed")), &path).expect("valid generator input");
                    self.charge(&at, n as u128 * t.rng_init_per_byte, GENERATOR_SIZE, 0)?;
                    let owner = self.own(frame, text(op, "name"), GENERATOR_SIZE);
                    self.bind_cursor(owner, Cursor::new(key));
                }
                "rng_draw" => {
                    let generator = self.lookup(frame, text(op, "generator"));
                    let c = self.owners[generator].cursor.expect("a generator owner");
                    let bound = integer_size(256);
                    self.charge(&at, t.rng_block, bound, 0)?;
                    let Some(block) = self.cursors[c].draw() else {
                        return Err(self.meter.error(code::GENERATOR_EXHAUSTED, &at, None));
                    };
                    let size = integer_size(BigUint::from_bytes_be(&block).bits().into());
                    self.own(frame, text(op, "name"), size);
                    self.meter.release(bound - size);
                    set(&mut self.values, &at, Json::Str(hex_of(&block)));
                }
                "stream" => {
                    let cursor_bytes = uint(op, "cursor_bytes");
                    self.charge(&format!("{at}/open"), t.stream_setup, cursor_bytes, 0)?;
                    self.own(frame, text(op, "name"), cursor_bytes);
                    set(&mut self.coverage, &at, 0);
                    let (length, chunk, letter) =
                        (uint(op, "length"), uint(op, "chunk_size"), uint(op, "letter_bytes"));
                    assert!(chunk > 0, "a chunk has letters");
                    let mut start = 0;
                    while start < length {
                        let n = chunk.min(length - start);
                        let cost = n * (t.expand_per_letter + t.replay_per_letter);
                        self.charge(&format!("{at}/chunk:{start}"), cost, n * letter, 0)?;
                        let covered = self.coverage.iter().find(|(k, _)| *k == at).map_or(0, |c| c.1);
                        set(&mut self.coverage, &at, covered + n);
                        self.meter.release(n * letter);
                        start += chunk;
                    }
                }
                other => panic!("unknown abstract operation: {other}"),
            }
        }
        Ok(())
    }

    fn result(mut self) -> Json {
        let given = self.given;
        let stop = self.run(arr(given, "trace"), "", "root").err();
        let summary = self.meter.finish(stop);
        let rows = summary.rows.iter().map(|r| {
            let cells = [r.steps, r.kernel_cost, r.memory, r.output, r.released];
            Json::Arr(std::iter::once(Json::Str(r.at.clone())).chain(cells.into_iter().map(num)).collect())
        });
        let children = summary.children.iter().map(|c| {
            obj(vec![
                ("at", Json::Str(c.at.clone())),
                ("requested", amounts_json(c.requested)),
                ("effective", amounts_json(c.effective)),
                ("outcome", Json::Str(c.outcome.name().into())),
                ("code", code_json(c.code)),
                ("stop", position_json(&c.stop)),
                ("used", amounts_json(c.used)),
                ("returned_memory", num(c.returned_memory)),
                ("live_after_return", num(c.live_after_return)),
            ])
        });
        let generators = self.generators.iter().map(|(name, c)| {
            let cursor = &self.cursors[*c];
            (name.clone(), obj(vec![("key", Json::Str(hex_of(&cursor.key))), ("counter", num(cursor.counter))]))
        });
        obj(vec![
            ("outcome", Json::Str(summary.outcome.name().into())),
            ("code", code_json(summary.code)),
            ("stop", position_json(&summary.stop)),
            ("used", amounts_json(summary.used)),
            ("live_before_cleanup", num(summary.live_before_cleanup)),
            ("live_after_cleanup", num(summary.live_after_cleanup)),
            ("charges", Json::Arr(rows.collect())),
            ("values", Json::Obj(self.values)),
            ("coverage", Json::Obj(self.coverage.into_iter().map(|(k, v)| (k, num(v))).collect())),
            ("generators", Json::Obj(generators.collect())),
            ("children", Json::Arr(children.collect())),
        ])
    }
}

/// The fixture's output profile, "PS1.2 retained JSON (fixture only)": Python's `json.dumps` with sorted keys,
/// compact separators and ASCII escapes. It is test-only; the product output profile is an open contract (PHS.55).
fn canonical(value: &Json) -> Vec<u8> {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out.into_bytes()
}

fn write_canonical(value: &Json, out: &mut String) {
    match value {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => out.push_str(&python_number(n)),
        Json::Str(s) => write_string(s, out),
        Json::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Json::Obj(fields) => {
            let mut sorted: Vec<&(String, Json)> = fields.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            out.push('{');
            for (i, (k, v)) in sorted.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write_canonical(v, out);
            }
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
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
                for unit in c.encode_utf16(&mut [0u16; 2]) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out.push('"');
}

/// Python's text of a JSON number: an integer's decimal value, or a float's `repr`.
fn python_number(lexeme: &str) -> String {
    if !lexeme.contains(['.', 'e', 'E']) {
        return lexeme.parse::<BigInt>().expect("an integer").to_string();
    }
    let x: f64 = lexeme.parse().expect("a float");
    assert!(x.is_finite(), "allow_nan=False");
    let scientific = format!("{:e}", x.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("an exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let point = exponent.parse::<i32>().expect("an exponent") + 1;
    let body = if point <= -4 || point > 16 {
        let (first, rest) = digits.split_at(1);
        let e = point - 1;
        let dot = if rest.is_empty() { "" } else { "." };
        format!("{first}{dot}{rest}e{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
    } else if point <= 0 {
        format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}.0", "0".repeat(point as usize - digits.len()))
    } else {
        format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
    };
    format!("{}{body}", if x.is_sign_negative() { "-" } else { "" })
}

/// `check_vectors.py`'s `first_difference`: objects compare as key sets, arrays in order.
fn first_difference(expected: &Json, actual: &Json, path: &str) -> Option<String> {
    match (expected, actual) {
        (Json::Obj(e), Json::Obj(a)) => {
            let mut ek: Vec<&String> = e.iter().map(|(k, _)| k).collect();
            let mut ak: Vec<&String> = a.iter().map(|(k, _)| k).collect();
            ek.sort();
            ak.sort();
            if ek != ak {
                return Some(format!("{path}: fields differ: {ek:?} != {ak:?}"));
            }
            e.iter().find_map(|(k, v)| first_difference(v, actual.get(k)?, &format!("{path}.{k}")))
        }
        (Json::Arr(e), Json::Arr(a)) => {
            if e.len() != a.len() {
                return Some(format!("{path}: length {} != {}", e.len(), a.len()));
            }
            e.iter().zip(a).enumerate().find_map(|(i, (l, r))| first_difference(l, r, &format!("{path}[{i}]")))
        }
        (e, a) if std::mem::discriminant(e) == std::mem::discriminant(a) => {
            (e != a).then(|| format!("{path}: expected {e:?}, computed {a:?}"))
        }
        _ => Some(format!("{path}: type differs: expected {expected:?}, computed {actual:?}")),
    }
}

fn tariffs(j: &Json) -> Tariffs {
    let Json::Obj(fields) = j else { panic!("tariffs") };
    assert_eq!(fields.len(), 13, "the frozen tariff table has 13 rows");
    Tariffs {
        kernel_base: uint(j, "kernel_base"),
        kernel_per_item: uint(j, "kernel_per_item"),
        limb_bits: uint(j, "limb_bits"),
        add_per_limb: uint(j, "add_per_limb"),
        mul_per_limb_pair: uint(j, "mul_per_limb_pair"),
        text_per_scalar: uint(j, "text_per_scalar"),
        decode_per_byte: uint(j, "decode_per_byte"),
        encode_per_byte: uint(j, "encode_per_byte"),
        rng_init_per_byte: uint(j, "rng_init_per_byte"),
        rng_block: uint(j, "rng_block"),
        stream_setup: uint(j, "stream_setup"),
        expand_per_letter: uint(j, "expand_per_letter"),
        replay_per_letter: uint(j, "replay_per_letter"),
    }
}

/// The frozen file's header and its 40 traces.
fn traces(root: &Json) -> &[Json] {
    assert_eq!(text(root, "contract"), "lp1-exec-draft-1");
    assert_eq!(text(root, "tariff_status"), "test-only; unmeasured");
    assert_eq!(text(root, "output_profile"), "PS1.2 retained JSON (fixture only)");
    assert_eq!(tariffs(root.get("tariffs").expect("tariffs")), Tariffs::TEST);
    let vectors = arr(root, "vectors");
    assert_eq!(vectors.len(), 40, "the frozen file has 40 traces");
    vectors
}

#[test]
fn the_40_frozen_traces() {
    let root = fixtures();
    for vector in traces(&root) {
        let given = vector.get("given").expect("given");
        let actual = Model::new(given, amounts(given.get("limits").expect("limits")), Tariffs::TEST).result();
        if let Some(difference) = first_difference(vector.get("expected").expect("expected"), &actual, "result") {
            panic!("{}: {difference}", text(vector, "id"));
        }
    }
}

/// A negative control: the comparison sees a one-unit change in the totals, a charge row, a generator cursor and a
/// child record.
#[test]
fn the_comparison_is_not_vacuous() {
    fn bump(j: &mut Json, path: &[&str]) {
        let mut node = j;
        for key in path {
            node = match node {
                Json::Obj(fields) => &mut fields.iter_mut().find(|(k, _)| k == key).expect("field").1,
                Json::Arr(items) => &mut items[key.parse::<usize>().expect("index")],
                _ => panic!("no field {key}"),
            };
        }
        let Json::Num(n) = node else { panic!("not a number") };
        *n = (n.parse::<u128>().expect("count") + 1).to_string();
    }
    let root = fixtures();
    let (mut generators, mut children) = (0, 0);
    for vector in traces(&root) {
        let given = vector.get("given").expect("given");
        let expected = vector.get("expected").expect("expected");
        let actual = Model::new(given, amounts(given.get("limits").expect("limits")), Tariffs::TEST).result();
        let mut paths: Vec<Vec<String>> = vec![vec!["used".into(), "steps".into()]];
        if !arr(&actual, "charges").is_empty() {
            paths.push(vec!["charges".into(), "0".into(), "1".into()]);
        }
        if let Some(Json::Obj(fields)) = actual.get("generators") {
            if let Some((name, _)) = fields.first() {
                paths.push(vec!["generators".into(), name.clone(), "counter".into()]);
                generators += 1;
            }
        }
        if !arr(&actual, "children").is_empty() {
            paths.push(vec!["children".into(), "0".into(), "returned_memory".into()]);
            children += 1;
        }
        for path in paths {
            let mut changed = actual.clone();
            bump(&mut changed, &path.iter().map(String::as_str).collect::<Vec<_>>());
            assert!(first_difference(expected, &changed, "result").is_some(), "{}: {path:?}", text(vector, "id"));
        }
    }
    assert!(generators > 0 && children > 0, "the frozen traces exercise generators and children");
}

fn has_child(trace: &[Json]) -> bool {
    trace.iter().any(|op| {
        text(op, "op") == "child"
            || ["body", "then", "else"].iter().any(|k| op.get(k).and_then(Json::arr).is_some_and(has_child))
    })
}

/// The meter boundary vectors (LP2 plan, M2). Each completed, child-free frozen trace runs again in two ways:
/// - with every limit set to its own use: it completes with identical results, since equality is admitted;
/// - for each dimension it uses, with that limit one short: it stops with that dimension's code in the root scope at
///   the operation the frozen rows predict. The stopping row keeps its step unless the dimension is steps (DET.56).
#[test]
fn meter_boundary_vectors() {
    let root = fixtures();
    let mut checked = (0, 0);
    for vector in traces(&root) {
        let (id, given, expected) = (text(vector, "id"), vector.get("given").unwrap(), vector.get("expected").unwrap());
        if text(expected, "outcome") != "completed" || has_child(arr(given, "trace")) {
            continue;
        }
        let used = amounts(expected.get("used").unwrap());
        let actual = Model::new(given, used, Tariffs::TEST).result();
        if let Some(difference) = first_difference(expected, &actual, "result") {
            panic!("{id} at its own use: {difference}");
        }
        checked.0 += 1;
        let rows: Vec<[u128; 5]> = arr(expected, "charges")
            .iter()
            .map(|r| {
                let cells = r.arr().expect("row");
                std::array::from_fn(|i| cells[i + 1].num().expect("cell").parse().expect("count"))
            })
            .collect();
        for dim in Dim::ALL {
            if used.get(dim) == 0 {
                continue;
            }
            let mut limits = used;
            match dim {
                Dim::Steps => limits.steps -= 1,
                Dim::KernelCost => limits.kernel_cost -= 1,
                Dim::Memory => limits.memory -= 1,
                Dim::Output => limits.output -= 1,
            }
            // Predict the stopping row from the frozen rows: cumulative use, or live memory before the row.
            let (mut total, mut live, mut stop_row) = (0u128, 0u128, None);
            for (i, [steps, kernel, memory, output, released]) in rows.iter().copied().enumerate() {
                let (current, amount) = match dim {
                    Dim::Steps => (total, steps),
                    Dim::KernelCost => (total, kernel),
                    Dim::Memory => (live, memory),
                    Dim::Output => (total, output),
                };
                if current + amount > limits.get(dim) {
                    stop_row = Some((i, live));
                    break;
                }
                total += amount;
                live = live + memory - released;
            }
            let (row, live_before) = stop_row.unwrap_or_else(|| panic!("{id}: no row exceeds {}", dim.name()));
            let actual = Model::new(given, limits, Tariffs::TEST).result();
            let at = arr(expected, "charges")[row].arr().unwrap()[0].str().unwrap();
            let charges = arr(&actual, "charges");
            let context = format!("{id} with {} one short", dim.name());
            assert_eq!(text(&actual, "code"), dim.code(), "{context}");
            assert_eq!(text(&actual, "outcome"), "cut_off", "{context}");
            let stop = actual.get("stop").unwrap();
            assert_eq!(
                (text(stop, "at"), text(stop, "phase"), text(stop, "scope")),
                (at, dim.name(), "root"),
                "{context}"
            );
            assert_eq!(charges.len(), row + 1, "{context}");
            assert_eq!(&charges[..row], &arr(expected, "charges")[..row], "{context}");
            let step = if dim == Dim::Steps { "0" } else { "1" };
            let zeros = [step, "0", "0", "0", "0"].map(|n| Json::Num(n.to_string()));
            let last: Vec<Json> = std::iter::once(Json::Str(at.to_string())).chain(zeros).collect();
            assert_eq!(charges[row], Json::Arr(last), "{context}");
            assert_eq!(uint(&actual, "live_before_cleanup"), live_before, "{context}");
            assert_eq!(uint(&actual, "live_after_cleanup"), 0, "{context}");
            checked.1 += 1;
        }
    }
    eprintln!("meter boundary vectors: {} traces at their own use, {} one short", checked.0, checked.1);
    assert!(checked.0 >= 10 && checked.1 >= checked.0, "too few boundary vectors: {checked:?}");
}

// The frozen traces as programs. A trace is an abstract continuation list, not a program (CV.2), so a program row
// can differ from its frozen row only where the frozen text allows it; each test below states that difference and
// pins it exactly.

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// A charge row: its path and `[steps, kernel_cost, memory, output, released]`.
type Cells = (String, [u128; 5]);

fn vector<'a>(root: &'a Json, id: &str) -> &'a Json {
    traces(root).iter().find(|v| text(v, "id") == id).unwrap_or_else(|| panic!("no frozen trace {id}"))
}

fn frozen_rows(expected: &Json) -> Vec<Cells> {
    arr(expected, "charges")
        .iter()
        .map(|r| {
            let cells = r.arr().expect("row");
            let at = cells[0].str().expect("path").to_string();
            (at, std::array::from_fn(|i| cells[i + 1].num().expect("cell").parse().expect("count")))
        })
        .collect()
}

fn product_rows(run: &Run) -> Vec<Cells> {
    run.summary.rows.iter().map(|r| (r.at.clone(), [r.steps, r.kernel_cost, r.memory, r.output, r.released])).collect()
}

fn cells(at: &str, c: [u128; 5]) -> Cells {
    (at.to_string(), c)
}

/// Parses, lowers, checks and loads one module `m` that requires the given capabilities; the checker must accept it.
fn program(requires: &str, declarations: &str) -> Loaded {
    let source = format!("edition \"6cs-draft-1\";\nmodule m requires [{requires}];\n\n{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"))
}

/// The host of the trace programs. The snapshot query `k` is the fixture's kernel row with one item and a two-byte
/// scratch (CV.4: `1 + 2 * items`): it declares that scratch as its result bound and returns an empty result, so the
/// reservation is released after the row, as the fixture's scratch is. Published views are delivered and recorded.
#[derive(Default)]
struct TraceHost {
    performed: usize,
    views: Vec<Vec<u8>>,
}

impl Host for TraceHost {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Snapshot { query: "k", .. } => Ok(RowTariff { kernel_cost: 3, result_bound: 2 }),
            _ => Err(code::NOT_GRANTED),
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.performed += 1;
        match request {
            Request::Snapshot { query: "k", .. } => Ok(Value::Unit),
            _ => Err(code::NOT_GRANTED),
        }
    }
    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match sink {
            Sink::View => {
                self.views.push(bytes.to_vec());
                Ok(())
            }
            _ => Err(code::NOT_GRANTED),
        }
    }
}

fn run_with(loaded: &Loaded, function: &str, host: &mut TraceHost, options: &Options) -> Run {
    loaded.run("m", function, vec![], host, options).unwrap_or_else(|r| panic!("{r:?}"))
}

/// CT-DET-26, CT-DET-28 and CT-DET-31 (part: RT1 only; not run: RT2 and RT3, which LQ9 did not select (no owner
/// yet)): the frozen trace `nested-calls` runs as a program, `outer` calling `inner` calling one kernel row. Frozen
/// row i is program row i, and they differ only where the frozen text allows:
/// - each frame has one slot more than the fixture's zero-slot frames (CV.2): the M2 draft frame layout gives a body's
///   result a temporary slot, so `call` reserves and `return` releases 16 + 8 bytes (DET.57);
/// - the program ends with its top-level output unit, `encode` and `emit` of `null` (DET.59), which a fixture trace
///   lists only when it models a complete program (CV.2).
///
/// The kernel row agrees exactly, nested returns charge no output, and the program completes at exactly the frozen
/// limits plus those differences.
#[test]
fn ct_det_26_nested_calls_runs_as_a_program() {
    let root = fixtures();
    let v = vector(&root, "nested-calls");
    let (given, expected) = (v.get("given").unwrap(), v.get("expected").unwrap());
    let frozen = frozen_rows(expected);
    let loaded = program(
        "",
        "fn inner() -> Unit effects [snapshot] {\n    return read_snapshot<Unit>(\"k\");\n}\n\n\
         export fn outer() -> Unit effects [snapshot] {\n    return inner();\n}\n",
    );
    let slot = frame_size(1) - frame_size(0);
    assert_eq!((frame_size(0), slot), (16, 8));
    for f in ["outer", "inner"] {
        assert_eq!(loaded.frame_size(loaded.function("m", f).unwrap()), frame_size(1), "{f}: one temporary slot");
    }

    let mut host = TraceHost::default();
    let run = run_with(&loaded, "outer", &mut host, &Options::new(WIDE));
    assert_eq!(run.summary.outcome, Outcome::Completed);
    assert_eq!(run.output.as_deref(), Some(&b"null\n"[..]));
    let rows = product_rows(&run);
    let paths: Vec<&str> = rows.iter().map(|(at, _)| at.as_str()).collect();
    assert_eq!(
        paths,
        [
            "entry#call",
            "m.outer:root#call",
            "m.outer:root > m.inner:root#kernel",
            "m.outer:root > m.inner:root#return",
            "m.outer:root#return",
            "entry#encode",
            "entry#emit"
        ]
    );
    let one_slot = [[0, 0, slot, 0, 0], [0, 0, slot, 0, 0], [0; 5], [0, 0, 0, 0, slot], [0, 0, 0, 0, slot]];
    assert_eq!(frozen.len(), one_slot.len());
    for (i, ((at, row), delta)) in frozen.iter().zip(one_slot).enumerate() {
        let want: [u128; 5] = std::array::from_fn(|c| row[c] + delta[c]);
        assert_eq!(rows[i].1, want, "frozen row {at} against program row {}", rows[i].0);
    }
    assert_eq!(rows[5..], [cells("entry#encode", [1, 5, 5, 0, 0]), cells("entry#emit", [1, 0, 0, 5, 5])]);
    assert!(rows[..5].iter().all(|(_, c)| c[3] == 0), "nested returns are internal moves, not output (DET.59)");
    assert_eq!(host.performed, 1);

    // Under the frozen limits the second frame does not fit: 24 + 24 > 34.
    let limits = amounts(given.get("limits").unwrap());
    let cut = run_with(&loaded, "outer", &mut TraceHost::default(), &Options::new(limits));
    assert_eq!((cut.summary.outcome, cut.summary.code), (Outcome::CutOff, Some(code::MEMORY)));
    assert_eq!(cut.summary.stop.as_ref().unwrap().at, "m.outer:root#call");
    assert_eq!(product_rows(&cut).last(), Some(&cells("m.outer:root#call", [1, 0, 0, 0, 0])));

    // At the frozen use plus exactly those differences, every dimension is at equality and the run completes.
    let used = amounts(expected.get("used").unwrap());
    let plus = Amounts {
        steps: used.steps + 2,
        kernel_cost: used.kernel_cost + 5,
        memory: used.memory + 2 * slot,
        output: used.output + 5,
    };
    let exact = run_with(&loaded, "outer", &mut TraceHost::default(), &Options::new(plus));
    assert_eq!((exact.summary.outcome, exact.summary.used), (Outcome::Completed, plus));
    assert_eq!(product_rows(&exact), rows);
}

/// The rows of a run before its top-level `entry#encode`: its index, and their steps, kernel cost, output and the
/// live memory after them.
fn before_unit(rows: &[Cells]) -> (usize, Amounts) {
    let at = rows.iter().position(|(path, _)| path == "entry#encode").expect("a top-level unit");
    let mut before = Amounts::default();
    for (_, [steps, kernel, memory, output, released]) in &rows[..at] {
        before.steps += steps;
        before.kernel_cost += kernel;
        before.memory = before.memory + memory - released;
        before.output += output;
    }
    (at, before)
}

/// CT-DET-31 and CT-DET-27 (part: RT1 only; not run: RT2 and RT3 (no owner yet)): the frozen traces
/// `output-equality`, `simultaneous-exhaustion`, `simultaneous-memory-output` and `step-precedence` run as programs
/// that return a text as their top-level output unit.
///
/// The fixture's `emit` is a fused, size-known encoding and publication row (CV.5). A program value carries no
/// retained byte length, so RT1 runs DET.59's other form: a preceding encoding pass priced on the value's retained
/// bound, then the publication of the exact bytes. The fixture's one row is the program's `encode` and `emit` rows,
/// and its byte count is a different profile's (CV.5): PS1.2 retained JSON escapes "é" as six ASCII characters
/// between quotes, 8 bytes; the product profile writes its UTF-8 between quotes and a line feed, 5 bytes. What DET.56
/// and DET.59 pin holds: equality is admitted, a unit one byte short is
/// not published and its row keeps only its step, and each simultaneous case stops in the frozen trace's dimension,
/// with the frozen denied row's cells.
#[test]
fn ct_det_31_output_equality_runs_as_a_program() {
    let root = fixtures();
    let expected = vector(&root, "output-equality").get("expected").unwrap();
    assert_eq!(frozen_rows(expected), [cells("emit", [1, 8, 8, 8, 8])]);
    assert_eq!(hex(text(expected.get("values").unwrap(), "emit")), b"\x22\x5cu00e9\x22");

    let loaded = program("", "export fn e() -> Text effects [] {\n    return \"é\";\n}\n");
    let run = run_with(&loaded, "e", &mut TraceHost::default(), &Options::new(WIDE));
    assert_eq!(run.output.as_deref(), Some("\"é\"\n".as_bytes()));
    let rows = product_rows(&run);
    // `encode`: the work and the buffer on the retained bound (two quotes, six bytes per scalar, the line feed), the
    // buffer shrunk to the 5 bytes; `emit`: the 5 bytes, then the buffer and the 4-byte text are released.
    let (at, _) = before_unit(&rows);
    assert_eq!(rows[at..], [cells("entry#encode", [1, 9, 9, 0, 4]), cells("entry#emit", [1, 0, 0, 5, 9])]);

    let used = run.summary.used;
    let exact = run_with(&loaded, "e", &mut TraceHost::default(), &Options::new(used));
    assert_eq!((exact.summary.outcome, exact.summary.used), (Outcome::Completed, used));
    let short = run_with(&loaded, "e", &mut TraceHost::default(), &Options::new(Amounts { output: 4, ..used }));
    assert_eq!((short.summary.outcome, short.summary.code), (Outcome::CutOff, Some(code::OUTPUT)));
    let stop = short.summary.stop.as_ref().unwrap();
    assert_eq!((stop.at.as_str(), stop.phase, stop.scope.as_str()), ("entry#emit", "output", "root"));
    assert_eq!(short.output, None);
    assert_eq!(product_rows(&short), [&rows[..at + 1], &[cells("entry#emit", [1, 0, 0, 0, 0])]].concat());
    assert_eq!(short.summary.used.output, 0, "no byte of the denied unit is charged");

    // The simultaneous cases. A ten-letter text makes the unit's encoding reservation the largest live request of the
    // run, so a memory limit can fall short at the unit alone. Each dimension the frozen trace leaves short of its
    // `emit` (one step; `"x"`: three bytes of work, buffer and output) is made one short at the program's unit.
    let loaded = program("", "export fn x() -> Text effects [] {\n    return \"xxxxxxxxxx\";\n}\n");
    let rows = product_rows(&run_with(&loaded, "x", &mut TraceHost::default(), &Options::new(WIDE)));
    let (at, before) = before_unit(&rows);
    let (encode, emit) = (rows[at].1, rows[at + 1].1);
    let peak = rows[..at]
        .iter()
        .scan(0u128, |live, (_, c)| {
            let peak = *live + c[2];
            *live = peak - c[4];
            Some(peak)
        })
        .max()
        .unwrap();
    assert!(before.memory + encode[2] > peak, "the unit's reservation is the largest live request");
    for id in ["simultaneous-exhaustion", "simultaneous-memory-output", "step-precedence"] {
        let v = vector(&root, id);
        let (given, expected) = (v.get("given").unwrap(), v.get("expected").unwrap());
        let frozen = amounts(given.get("limits").unwrap());
        let short = |limit: u128, need: u128| u128::from(limit < need);
        let limits = Amounts {
            steps: before.steps + 2 - 2 * short(frozen.steps, 1),
            kernel_cost: before.kernel_cost + encode[1] - short(frozen.kernel_cost, 3),
            memory: before.memory + encode[2] - short(frozen.memory, 3),
            output: before.output + emit[3] - short(frozen.output, 3),
        };
        let run = run_with(&loaded, "x", &mut TraceHost::default(), &Options::new(limits));
        assert_eq!(run.summary.outcome, Outcome::CutOff, "{id}");
        assert_eq!(run.summary.code, Some(text(expected, "code")), "{id}");
        let stop = run.summary.stop.as_ref().unwrap();
        let frozen_stop = expected.get("stop").unwrap();
        assert_eq!((stop.at.as_str(), stop.phase), ("entry#encode", text(frozen_stop, "phase")), "{id}");
        let denied = frozen_rows(expected).pop().unwrap().1;
        assert_eq!(product_rows(&run), [&rows[..at], &[cells("entry#encode", denied)]].concat(), "{id}");
    }
}

/// CT-DET-31 (part: RT1 only; not run: RT2 and RT3 (no owner yet)): the frozen trace `output-retained-at-cutoff` runs
/// as a program that publishes `"é"` and then `0`, the second unit one byte short. As in the frozen trace, the first
/// unit is published and stays charged, and the second is denied in phase `output` with the frozen `denied` row's
/// cells: one step and nothing else, and none of its bytes reach the host (DET.59). Each unit has its own priced
/// encoding pass before its publication, as in `ct_det_31_output_equality_runs_as_a_program`.
#[test]
fn ct_det_31_output_retained_at_cutoff_runs_as_a_program() {
    let root = fixtures();
    let expected = vector(&root, "output-retained-at-cutoff").get("expected").unwrap();
    let frozen = frozen_rows(expected);
    assert_eq!(frozen, [cells("first", [1, 8, 8, 8, 8]), cells("denied", [1, 0, 0, 0, 0])]);
    assert_eq!(uint(expected.get("used").unwrap(), "output"), 8, "the first unit stays charged");

    let loaded = program(
        "",
        "export fn two() -> Unit effects [view] {\n    let a = publish_view(\"é\");\n    return publish_view(0);\n}\n",
    );
    let mut host = TraceHost::default();
    let all = run_with(&loaded, "two", &mut host, &Options::new(WIDE));
    assert_eq!(host.views, [&b"\"\xc3\xa9\"\n"[..], &b"{\"int\":\"0\"}\n"[..]]);
    let rows = product_rows(&all);
    let second = rows.iter().position(|(at, _)| at == "m.two:1#publish_view").expect("the second publication");
    assert_eq!(rows[second].1[3], 12);

    let mut host = TraceHost::default();
    let cut = run_with(&loaded, "two", &mut host, &Options::new(Amounts { output: 5 + 12 - 1, ..WIDE }));
    assert_eq!((cut.summary.outcome, cut.summary.code), (Outcome::CutOff, Some(text(expected, "code"))));
    let stop = cut.summary.stop.as_ref().unwrap();
    let frozen_stop = expected.get("stop").unwrap();
    assert_eq!(
        (stop.at.as_str(), stop.phase, stop.scope.as_str()),
        ("m.two:1#publish_view", text(frozen_stop, "phase"), text(frozen_stop, "scope"))
    );
    assert_eq!(product_rows(&cut), [&rows[..second], &[cells("m.two:1#publish_view", frozen[1].1)]].concat());
    assert_eq!(host.views, [&b"\"\xc3\xa9\"\n"[..]], "only the admitted unit reaches the host");
    assert_eq!(cut.summary.used.output, 5, "the published unit stays charged");
    assert_eq!(cut.summary.live_after_cleanup, 0);
}

/// CT-DET-29 (part: RT1 only; not run: RT2 and RT3 (no owner yet), and child stream paths, since the IR has no child
/// construct yet): the frozen trace `generator` runs as a program with the fixture's seed and stream path, drawing
/// three times. Its `rng_init` and draw rows equal the frozen rows; the program adds its entry `call` between the
/// initialization and the first draw. At the frozen limits plus that row's step and frame, the third draw is denied
/// on kernel cost with the frozen denied row, and the admitted draws are the frozen blocks.
#[test]
fn ct_det_29_generator_runs_as_a_program() {
    let root = fixtures();
    let v = vector(&root, "generator");
    let (given, expected) = (v.get("given").unwrap(), v.get("expected").unwrap());
    let init = &arr(given, "trace")[0];
    assert_eq!(text(init, "op"), "rng_init");
    let seed = hex(text(init, "seed"));
    let path: Vec<&str> = arr(init, "path").iter().map(|s| s.str().expect("a segment")).collect();
    let frozen = frozen_rows(expected);

    let loaded = program(
        "random",
        "export fn three() -> record { a: Nat, b: Nat, c: Nat } effects [random] {\n    \
         return record { a = random_draw(), b = random_draw(), c = random_draw() };\n}\n",
    );
    let frame = loaded.frame_size(loaded.function("m", "three").unwrap());
    let options = |limits| Options { seed: Some((&seed[..], &path[..])), ..Options::new(limits) };
    let all = run_with(&loaded, "three", &mut TraceHost::default(), &options(WIDE));
    let rows = product_rows(&all);
    assert_eq!(rows[0], cells("entry#rng_init", frozen[0].1));
    assert_eq!(rows[1], cells("entry#call", [1, 0, frame, 0, 0]));
    assert_eq!(rows[2], cells("m.three:0#rng_draw", frozen[1].1));
    assert_eq!(rows[3], cells("m.three:1#rng_draw", frozen[2].1));
    let values = expected.get("values").unwrap();
    let output = std::str::from_utf8(all.output.as_deref().unwrap()).unwrap();
    for (field, at) in [("a", "draw-0"), ("b", "draw-1")] {
        let block = BigUint::from_bytes_be(&hex(text(values, at)));
        assert!(output.contains(&format!("\"{field}\":{{\"int\":\"{block}\"}}")), "{at}: {output}");
    }

    let frozen_limits = amounts(given.get("limits").unwrap());
    let limits = Amounts { steps: frozen_limits.steps + 1, memory: frozen_limits.memory + frame, ..frozen_limits };
    let cut = run_with(&loaded, "three", &mut TraceHost::default(), &options(limits));
    assert_eq!((cut.summary.outcome, cut.summary.code), (Outcome::CutOff, Some(text(expected, "code"))));
    assert_eq!(cut.summary.stop.as_ref().unwrap().at, "m.three:2#rng_draw");
    assert_eq!(product_rows(&cut), [&rows[..4], &[cells("m.three:2#rng_draw", frozen[3].1)]].concat());
    let used = amounts(expected.get("used").unwrap());
    assert_eq!(cut.summary.used, Amounts { steps: used.steps + 1, memory: used.memory + frame, ..used });
}
