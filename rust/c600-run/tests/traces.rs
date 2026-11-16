//! LP2 plan, M2: the 40 frozen contract-vector traces (DET.65) replayed through the reference meter, and the meter
//! boundary vectors derived from them.
//!
//! The replay follows the abstract model of `check_vectors.py`. Its frames, named owners and abstract operations
//! belong to the fixture, not to the interpreter. Every charge, owner, release, child scope, generator draw and
//! tariff comes from the library.

#[path = "../../c600-check/tests/support/json.rs"]
#[allow(dead_code)]
mod json;

use c600_run::meter::{
    code, frame_size, integer_size, Amounts, Dim, IntOp, Meter, OwnerId, Position, SourceRange, Stop, Tariffs,
    GENERATOR_SIZE,
};
use c600_run::rng::{stream_key, Cursor};
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
