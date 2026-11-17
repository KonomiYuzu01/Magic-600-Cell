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

/// CT-DET-09 and CT-DET-27 (part: the reference model over the library meter; RT1's runs are in `conformance_det.rs`;
/// not run: RT2 and RT3 (no owner yet)): the meter boundary vectors (LP2 plan, M2). Each completed, child-free frozen
/// trace runs again in two ways:
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

fn cells(at: &str, c: [u128; 5]) -> Cells {
    (at.to_string(), c)
}

// The traces the IR draft can express also run as programs through `Loaded::run`, each program row against its
// frozen row and each cutoff against the frozen stop: `tests/det65_programs.rs`.

// The frozen traces under other limits. The reference model runs a trace again under limits it was not frozen with,
// and each run is checked against what DET.56 predicts from the rows of a run that completed.

/// `limits` with one dimension set to `value`.
fn with_limit(limits: Amounts, dim: Dim, value: u128) -> Amounts {
    let mut limits = limits;
    match dim {
        Dim::Steps => limits.steps = value,
        Dim::KernelCost => limits.kernel_cost = value,
        Dim::Memory => limits.memory = value,
        Dim::Output => limits.output = value,
    }
    limits
}

/// Where a run stops, predicted by DET.56.
struct Predicted {
    /// The first denied row.
    row: usize,
    /// The first dimension that denies it, in the order steps, kernel cost, memory, output.
    dim: Dim,
    /// The counters at the stop: the rows before it, and the denied row's step unless the step was denied.
    used: Amounts,
    /// The live memory before the denied row.
    live: u128,
}

/// The first of a completed run's rows that `limits` deny, if any. Steps, kernel cost and output are cumulative;
/// memory is the live memory before the row plus its request; equality is admitted.
fn predict(rows: &[Cells], limits: Amounts) -> Option<Predicted> {
    let (mut used, mut live) = (Amounts::default(), 0u128);
    for (row, (_, [steps, kernel, memory, output, released])) in rows.iter().enumerate() {
        let need = Amounts {
            steps: used.steps + steps,
            kernel_cost: used.kernel_cost + kernel,
            memory: live + memory,
            output: used.output + output,
        };
        if let Some(dim) = Dim::ALL.into_iter().find(|&dim| need.get(dim) > limits.get(dim)) {
            if dim != Dim::Steps {
                used.steps += steps;
            }
            return Some(Predicted { row, dim, used, live });
        }
        used = Amounts { memory: used.memory.max(need.memory), ..need };
        live = live + memory - released;
    }
    None
}

/// The source range a stop at the operation path `at` reports: its own entry in the source map, or the trace's range.
fn source_of<'a>(given: &'a Json, at: &str) -> Option<&'a Json> {
    given.get("source_map").and_then(|map| map.get(at)).or_else(|| given.get("source"))
}

/// Runs the reference model under `limits` and checks the result against `predict` over `rows`, the rows of a
/// completed run of the same trace: the outcome, code, stop, scope and source range, the rows, the counters and the
/// live memory at the stop, and that cleanup releases everything. Returns the result.
fn check_predicted(given: &Json, rows: &[Cells], limits: Amounts, context: &str) -> Json {
    let actual = Model::new(given, limits, Tariffs::TEST).result();
    let got = frozen_rows(&actual);
    match predict(rows, limits) {
        None => {
            assert_eq!(text(&actual, "outcome"), "completed", "{context}");
            assert_eq!(got, rows, "{context}");
        }
        Some(p) => {
            let at = rows[p.row].0.as_str();
            let code = actual.get("code").and_then(Json::str);
            assert_eq!((text(&actual, "outcome"), code), ("cut_off", Some(p.dim.code())), "{context}");
            let stop = actual.get("stop").unwrap();
            assert_eq!(
                (text(stop, "at"), text(stop, "phase"), text(stop, "scope")),
                (at, p.dim.name(), "root"),
                "{context}"
            );
            assert_eq!(stop.get("source"), source_of(given, at), "{context}");
            let step = if p.dim == Dim::Steps { 0 } else { rows[p.row].1[0] };
            assert_eq!(got, [&rows[..p.row], &[cells(at, [step, 0, 0, 0, 0])]].concat(), "{context}");
            assert_eq!(amounts(actual.get("used").unwrap()), p.used, "{context}");
            assert_eq!(uint(&actual, "live_before_cleanup"), p.live, "{context}");
        }
    }
    assert_eq!(uint(&actual, "live_after_cleanup"), 0, "{context}");
    actual
}

/// The completed frozen traces without children: their id, given and expected result.
fn completed_child_free(root: &Json) -> Vec<(&str, &Json, &Json)> {
    traces(root)
        .iter()
        .map(|v| (text(v, "id"), v.get("given").unwrap(), v.get("expected").unwrap()))
        .filter(|(_, given, expected)| text(expected, "outcome") == "completed" && !has_child(arr(given, "trace")))
        .collect()
}

/// CT-DET-09 and CT-DET-30 (part: the reference model over the library meter; RT1's runs are in
/// `conformance_det.rs`; not run: RT2 and RT3 (no owner yet), and a search's coverage in a run record (M6)): each
/// completed child-free frozen trace, the generators with fixed keys among them, runs again under every limit from
/// zero up to its use in each dimension, the other limits at their use. Each run stops where DET.56 predicts from the
/// frozen rows, in that dimension and the root scope, with the frozen rows before the stop and the counters and live
/// memory they give. A stream's coverage is the letters of its admitted chunks, recorded once its open is admitted,
/// and a second run is identical.
#[test]
fn ct_det_09_every_limit_below_use_stops_at_the_predicted_row() {
    let root = fixtures();
    let (mut runs, mut streams) = (0, 0);
    for (id, given, expected) in completed_child_free(&root) {
        let rows = frozen_rows(expected);
        let used = amounts(expected.get("used").unwrap());
        for dim in Dim::ALL {
            for limit in 0..used.get(dim) {
                let limits = with_limit(used, dim, limit);
                let context = format!("{id} with {} {limit}", dim.name());
                let p = predict(&rows, limits).unwrap_or_else(|| panic!("{context}: no row is denied"));
                assert_eq!(p.dim, dim, "{context}");
                let actual = check_predicted(given, &rows, limits, &context);
                assert_eq!(Model::new(given, limits, Tariffs::TEST).result(), actual, "{context}: a second run");
                let admitted: Vec<&str> = rows[..p.row].iter().map(|(at, _)| at.as_str()).collect();
                let mut coverage = Vec::new();
                for op in arr(given, "trace").iter().filter(|op| text(op, "op") == "stream") {
                    let at = text(op, "at");
                    if !admitted.contains(&format!("{at}/open").as_str()) {
                        continue;
                    }
                    let (length, chunk) = (uint(op, "length"), uint(op, "chunk_size"));
                    let covered: u128 = (0..length)
                        .step_by(chunk as usize)
                        .filter(|start| admitted.contains(&format!("{at}/chunk:{start}").as_str()))
                        .map(|start| chunk.min(length - start))
                        .sum();
                    coverage.push((at.to_string(), num(covered)));
                    streams += 1;
                }
                assert_eq!(actual.get("coverage"), Some(&Json::Obj(coverage)), "{context}: coverage");
                runs += 1;
            }
        }
    }
    eprintln!("CT-DET-09: {runs} runs below use, {streams} of them with an admitted stream");
    assert!(runs >= 200 && streams > 0, "too few runs: {runs}, {streams}");
}

/// CT-DET-27 (part: the reference model over the library meter; RT1's runs are in `conformance_det.rs`; not run:
/// child scopes in a program, since the IR has no child construct (child jobs, M6), and RT2 and RT3 (no owner yet)):
/// at each row of each completed child-free frozen trace, every set of the dimensions the row requests is made one
/// short at that row, the other limits at their use. Every run stops where DET.56 predicts. When it stops at that row,
/// the first short dimension in the order steps, kernel cost, memory, output decides, whichever others are also short:
/// a failed step debits nothing, and a denied request keeps its step and debits none of the other three. With limits
/// equal to its use, each trace completes with its frozen result (equality is admitted). The child-scope precedence
/// vectors `root-before-child` and `dimension-before-scope` are compared row by row in `the_40_frozen_traces`.
#[test]
fn ct_det_27_every_set_of_short_dimensions_stops_in_precedence_order() {
    let root = fixtures();
    let (mut runs, mut at_row) = (0, 0);
    for (id, given, expected) in completed_child_free(&root) {
        let rows = frozen_rows(expected);
        let used = amounts(expected.get("used").unwrap());
        assert_eq!(&Model::new(given, used, Tariffs::TEST).result(), expected, "{id} at its use");
        let (mut before, mut live) = (Amounts::default(), 0u128);
        for (i, (at, [steps, kernel, memory, output, released])) in rows.iter().enumerate() {
            let request = Amounts { steps: *steps, kernel_cost: *kernel, memory: *memory, output: *output };
            let need = Amounts {
                steps: before.steps + steps,
                kernel_cost: before.kernel_cost + kernel,
                memory: live + memory,
                output: before.output + output,
            };
            let requested: Vec<Dim> = Dim::ALL.into_iter().filter(|&dim| request.get(dim) > 0).collect();
            for set in 1usize..(1 << requested.len()) {
                let short: Vec<Dim> =
                    (0..requested.len()).filter(|b| (set >> b) & 1 == 1).map(|b| requested[b]).collect();
                let limits = short.iter().fold(used, |limits, &dim| with_limit(limits, dim, need.get(dim) - 1));
                let names: Vec<&str> = short.iter().map(|dim| dim.name()).collect();
                let context = format!("{id} at {at} with {names:?} one short");
                let p = predict(&rows, limits).unwrap_or_else(|| panic!("{context}: no row is denied"));
                assert!(p.row <= i, "{context}");
                if p.row == i {
                    assert_eq!(p.dim, short[0], "{context}");
                    at_row += 1;
                }
                check_predicted(given, &rows, limits, &context);
                runs += 1;
            }
            before = need;
            live = live + memory - released;
        }
    }
    eprintln!("CT-DET-27: {runs} sets of short dimensions, {at_row} stopping at their own row");
    assert!(at_row >= 100 && runs >= at_row, "too few runs: {runs}, {at_row}");
}

/// CT-DET-20 (part: typed endings and cleanup over the reference model and the library meter; RT1's runs are in
/// `conformance_det.rs`; not run: the journal and the state hash under fault (M6) and a killed runtime (LP3)): each
/// frozen trace at its own limits, and each child-free one also at its use and with each limit at zero, ends typed:
/// `completed` with no code and no stop, `cut_off` with the code of the dimension its stop names, or `error` with a
/// code of the catalogue and its stop in phase `effect`. Every stop names its scope, every run releases all live
/// memory at cleanup, and a second run is identical.
#[test]
fn ct_det_20_every_trace_ends_typed_and_releases_everything() {
    let errors = [
        code::POINTWISE_FAILED,
        code::GENERATOR_INPUT,
        code::GENERATOR_EXHAUSTED,
        code::TEXT_SCALAR_INVALID,
        code::NOT_GRANTED,
    ];
    let root = fixtures();
    let mut outcomes: HashMap<String, usize> = HashMap::new();
    for vector in traces(&root) {
        let (id, given, expected) = (text(vector, "id"), vector.get("given").unwrap(), vector.get("expected").unwrap());
        let own = amounts(given.get("limits").unwrap());
        let mut cases = vec![("its limits".to_string(), own)];
        if !has_child(arr(given, "trace")) {
            cases.push(("its use".to_string(), amounts(expected.get("used").unwrap())));
            cases.extend(Dim::ALL.map(|dim| (format!("{} zero", dim.name()), with_limit(own, dim, 0))));
        }
        for (label, limits) in cases {
            let context = format!("{id} at {label}");
            let actual = Model::new(given, limits, Tariffs::TEST).result();
            assert_eq!(Model::new(given, limits, Tariffs::TEST).result(), actual, "{context}: a second run");
            let outcome = text(&actual, "outcome");
            let (code, stop) = (actual.get("code").and_then(Json::str), actual.get("stop").unwrap());
            match outcome {
                "completed" => assert_eq!((code, stop), (None, &Json::Null), "{context}"),
                "cut_off" => {
                    let dim = Dim::ALL.into_iter().find(|dim| dim.name() == text(stop, "phase"));
                    assert_eq!(code, dim.map(Dim::code), "{context}");
                }
                "error" => {
                    assert!(code.is_some_and(|c| errors.contains(&c)), "{context}: {code:?}");
                    assert_eq!(text(stop, "phase"), "effect", "{context}");
                }
                other => panic!("{context}: untyped outcome {other}"),
            }
            if outcome != "completed" {
                assert!(!text(stop, "scope").is_empty(), "{context}");
            }
            assert_eq!(uint(&actual, "live_after_cleanup"), 0, "{context}");
            *outcomes.entry(outcome.to_string()).or_default() += 1;
        }
    }
    eprintln!("CT-DET-20: {outcomes:?}");
    assert!(["completed", "cut_off", "error"].iter().all(|o| outcomes.get(*o).is_some_and(|n| *n > 0)));
}

/// CT-DET-26 (part: the reference model over the library meter, for the frozen traces without children; RT1's frames
/// are in `det65_programs.rs`; not run: moved child results in a program, since the IR has no
/// child construct (child jobs, M6), cached owning handles, which wait for the kernel's cached rows (no owner yet),
/// and RT2 and RT3 (no owner yet)): in each child-free frozen trace under its own limits, the meter's memory use is
/// the highest live memory its rows reach, and the live memory at the end is what the rows reserve minus what they
/// release, so aliases, drops, replacements and frame unwinds move no unrecorded byte. An alias charges its source's
/// full size, and dropping the source leaves the alias charged.
#[test]
fn ct_det_26_peaks_and_live_memory_follow_the_rows() {
    let root = fixtures();
    let mut checked = 0;
    for vector in traces(&root) {
        let (id, given) = (text(vector, "id"), vector.get("given").unwrap());
        if has_child(arr(given, "trace")) {
            continue;
        }
        let actual = Model::new(given, amounts(given.get("limits").unwrap()), Tariffs::TEST).result();
        let (mut live, mut peak) = (0u128, 0u128);
        for (_, [_, _, memory, _, released]) in frozen_rows(&actual) {
            peak = peak.max(live + memory);
            live = live + memory - released;
        }
        let used = amounts(actual.get("used").unwrap());
        assert_eq!((used.memory, uint(&actual, "live_before_cleanup")), (peak, live), "{id}");
        assert_eq!(uint(&actual, "live_after_cleanup"), 0, "{id}");
        checked += 1;
    }
    assert!(checked >= 30, "{checked}");

    let given = vector(&root, "alias-still-live").get("given").unwrap();
    let actual = Model::new(given, amounts(given.get("limits").unwrap()), Tariffs::TEST).result();
    let rows = frozen_rows(&actual);
    assert_eq!(rows[..3], [cells("a", [1, 0, 4, 0, 0]), cells("b", [1, 0, 4, 0, 0]), cells("drop-a", [1, 0, 0, 0, 4])]);
    assert_eq!(
        (text(&actual, "code"), uint(&actual, "live_before_cleanup")),
        (code::MEMORY, 4),
        "the alias stays live"
    );
}

/// CT-DET-28 (synthetic, part: the reference model over the library meter and its integer tariff; RT1's arithmetic,
/// assertions and text equality are in `conformance_det.rs`, its loop tests in `loops.rs`; not run: text order and
/// flow checks in a program, which the IR draft does not have (no owner yet)):
/// - a multiplication's kernel cost and reservation come from its operands' bit lengths, so a zero operand still pays
///   for the other operand's limbs; the reservation covers the result and shrinks to it, and a denied multiplication
///   computes nothing;
/// - a text comparison costs both complete scalar lengths, even when the first scalar decides, and orders by scalar
///   value without normalization or case folding (DET.62);
/// - each loop test, the final one included, and each call, predicate, assertion, flow check and return is a one-step
///   row of its own (DET.55); a false assertion is admitted and then ends the run before the operation it guards, and
///   an assertion denied its step stops on steps instead.
#[test]
fn ct_det_28_operations_keep_their_rows_and_operand_sized_tariffs() {
    let root = fixtures();
    let t = Tariffs::TEST;
    for id in ["oversized-arithmetic", "oversized-admitted", "arithmetic-reservation"] {
        let given = vector(&root, id).get("given").unwrap();
        let op = &arr(given, "trace")[0];
        assert_eq!(text(op, "op"), "mul", "{id}");
        let (a, b) = (int(op, "a"), int(op, "b"));
        let (cost, bound) = t.integer(IntOp::Mul, a.bits().into(), b.bits().into());
        let product = &a * &b;
        let size = integer_size(product.bits().into());
        assert!(size <= bound, "{id}: the reservation covers the result");
        let limits = amounts(given.get("limits").unwrap());
        let actual = Model::new(given, limits, t).result();
        let (rows, values) = (frozen_rows(&actual), actual.get("values").unwrap());
        if cost <= limits.kernel_cost {
            assert_eq!(rows[0], cells("mul", [1, cost, bound, 0, bound - size]), "{id}");
            assert_eq!(values.get("mul"), Some(&num(&product)), "{id}");
        } else {
            assert_eq!(rows, [cells("mul", [1, 0, 0, 0, 0])], "{id}");
            assert_eq!(values, &Json::Obj(Vec::new()), "{id}: a denied multiplication computes nothing");
        }
    }
    let op = &arr(vector(&root, "arithmetic-reservation").get("given").unwrap(), "trace")[0];
    let b_bits: u128 = int(op, "b").bits().into();
    let zero_times_b = b_bits.div_ceil(t.limb_bits) * t.mul_per_limb_pair;
    assert_eq!((int(op, "a"), t.integer(IntOp::Mul, 0, b_bits).0), (BigInt::from(0), zero_times_b));

    let given = vector(&root, "text-comparison").get("given").unwrap();
    let actual = Model::new(given, WIDE, t).result();
    let values = actual.get("values").unwrap();
    for (op, (at, row)) in arr(given, "trace").iter().zip(frozen_rows(&actual)) {
        let (a, b) = (text(op, "a"), text(op, "b"));
        let scalars = (a.chars().count() + b.chars().count()) as u128;
        assert_eq!(row, [1, scalars * t.text_per_scalar, 0, 0, 0], "{at}");
        assert_eq!(values.get(&at), Some(&num(a.chars().cmp(b.chars()) as i8)), "{at}");
    }
    assert_eq!(values.get("normalization"), Some(&num(1)), "no normalization");
    assert_eq!(values.get("case"), Some(&num(-1)), "no case folding");

    let given = vector(&root, "abstract-trace-charges").get("given").unwrap();
    let rows = frozen_rows(&Model::new(given, amounts(given.get("limits").unwrap()), t).result());
    let mut structure = Vec::new();
    for i in 0..2 {
        structure.push(cells(&format!("loop/test:{i}"), [1, 0, 0, 0, 0]));
        structure.push(cells(&format!("loop/body:{i}/f/call"), [1, 0, frame_size(0), 0, 0]));
        for op in ["predicate", "assert", "flow"] {
            structure.push(cells(&format!("loop/body:{i}/f/{op}"), [1, 0, 0, 0, 0]));
        }
        structure.push(cells(&format!("loop/body:{i}/f/return"), [1, 0, 0, 0, frame_size(0)]));
    }
    structure.push(cells("loop/test:2", [1, 0, 0, 0, 0]));
    assert_eq!(rows[..structure.len()], structure[..]);

    let given = vector(&root, "final-loop-test").get("given").unwrap();
    let rows = frozen_rows(&Model::new(given, WIDE, t).result());
    let test = [1, 0, 0, 0, 0];
    assert_eq!(rows, [cells("loop/test:0", test), cells("loop/body:0/body", test), cells("loop/test:1", test)]);
    for steps in 0..=3 {
        check_predicted(given, &rows, Amounts { steps, ..WIDE }, &format!("final-loop-test with steps {steps}"));
    }

    let given = vector(&root, "pointwise-failure").get("given").unwrap();
    let failed = Model::new(given, WIDE, t).result();
    assert_eq!((text(&failed, "outcome"), text(&failed, "code")), ("error", code::POINTWISE_FAILED));
    let stop = failed.get("stop").unwrap();
    assert_eq!((text(stop, "at"), text(stop, "phase")), ("assert", "effect"));
    assert_eq!(frozen_rows(&failed), [cells("predicate", test), cells("assert", test)], "the guarded tick has no row");
    let denied = Model::new(given, Amounts { steps: 1, ..WIDE }, t).result();
    assert_eq!((text(&denied, "code"), text(denied.get("stop").unwrap(), "at")), (code::STEPS, "assert"));
}

/// CT-DET-29 (part: the reference model over the library meter; RT1's draws are in
/// `det65_programs.rs` and `conformance_det.rs`; not run: child stream paths in a program, since
/// the IR has no child construct (child jobs, M6)): the frozen trace `generator` runs under every limit from zero to
/// its full use in each dimension. A denied initialization leaves no generator; afterwards the generator keeps its
/// key, its counter is the number of admitted draws, and each admitted draw is the block of its index: a draw denied
/// on any dimension consumes nothing (DET.61).
#[test]
fn ct_det_29_denied_draws_do_not_consume_the_stream() {
    let root = fixtures();
    let v = vector(&root, "generator");
    let (given, expected) = (v.get("given").unwrap(), v.get("expected").unwrap());
    let all = Model::new(given, WIDE, Tariffs::TEST).result();
    assert_eq!(text(&all, "outcome"), "completed");
    let rows = frozen_rows(&all);
    let frozen = frozen_rows(expected);
    assert_eq!((rows.len(), &rows[..3]), (4, &frozen[..3]));
    let key = text(expected.get("generators").unwrap().get("g").unwrap(), "key");
    let blocks = all.get("values").unwrap();
    let used = amounts(all.get("used").unwrap());
    let mut runs = 0;
    for dim in Dim::ALL {
        for limit in 0..=used.get(dim) {
            let context = format!("generator with {} {limit}", dim.name());
            let limits = with_limit(used, dim, limit);
            let actual = check_predicted(given, &rows, limits, &context);
            let admitted = predict(&rows, limits).map_or(rows.len(), |p| p.row);
            let generators = actual.get("generators").unwrap();
            if admitted == 0 {
                assert_eq!(generators, &Json::Obj(Vec::new()), "{context}: no generator before its initialization");
                continue;
            }
            let g = generators.get("g").unwrap();
            assert_eq!((text(g, "key"), uint(g, "counter")), (key, admitted as u128 - 1), "{context}");
            let values = actual.get("values").unwrap();
            for (i, (at, _)) in rows.iter().enumerate().skip(1) {
                assert_eq!(values.get(at), if i < admitted { blocks.get(at) } else { None }, "{context}: {at}");
            }
            runs += 1;
        }
    }
    assert!(runs > 100, "{runs}");
}

/// A trace given without its transport partitions.
fn without_transport(given: &Json) -> Json {
    let mut given = given.clone();
    let Json::Obj(fields) = &mut given else { panic!("a trace's given") };
    for (_, value) in fields.iter_mut().filter(|(key, _)| key == "trace") {
        let Json::Arr(ops) = value else { panic!("a trace") };
        for op in ops {
            let Json::Obj(op) = op else { panic!("an operation") };
            op.retain(|(key, _)| key != "transport_chunks");
        }
    }
    given
}

/// CT-DET-30 (part: the reference model over the library meter; not run: transport splits, cold caches and
/// cancellation in an executor, since streaming words are not built (`rust/README.md` lists KN2.10 as not built; no
/// owner yet)): the frozen pair `streaming-complete-split` and `streaming-complete-coalesced` differs only in its
/// transport partition, and its frozen results are equal. The chunk rows follow the semantic schedule (DET.63): one row
/// per chunk of the declared size in order, a final short chunk with its actual share, chunk tariffs that add up to
/// the stream's full tariff, and the setup cost on the open row only. Under every limit from zero to the use in each
/// dimension, both partitions stop at the same row with the same counters and coverage, and the coverage is always a
/// chunk boundary: a denied chunk advances nothing.
#[test]
fn ct_det_30_transport_partitions_change_no_charge_or_stop() {
    let root = fixtures();
    let pair = ["streaming-complete-split", "streaming-complete-coalesced"].map(|id| vector(&root, id));
    let given = pair.map(|v| v.get("given").unwrap());
    let transport = given.map(|g| arr(g, "trace")[0].get("transport_chunks").cloned());
    assert!(transport[0].is_some() && transport[0] != transport[1], "{transport:?}");
    assert_eq!(without_transport(given[0]), without_transport(given[1]));
    let expected = pair[0].get("expected").unwrap();
    assert_eq!(pair[1].get("expected").unwrap(), expected);

    let t = Tariffs::TEST;
    let op = &arr(given[0], "trace")[0];
    let (length, chunk, letter) = (uint(op, "length"), uint(op, "chunk_size"), uint(op, "letter_bytes"));
    let rows = frozen_rows(expected);
    assert_eq!(rows[0], cells("word/open", [1, t.stream_setup, uint(op, "cursor_bytes"), 0, 0]));
    let chunks: Vec<&Cells> = rows.iter().filter(|(at, _)| at.starts_with("word/chunk:")).collect();
    let starts: Vec<u128> = (0..length).step_by(chunk as usize).collect();
    assert_eq!(chunks.len(), starts.len());
    for ((at, c), start) in chunks.iter().zip(&starts) {
        let n = chunk.min(length - start);
        assert_eq!(at, &format!("word/chunk:{start}"));
        assert_eq!(c[..3], [1, n * (t.expand_per_letter + t.replay_per_letter), n * letter], "{at}");
    }
    let total: u128 = chunks.iter().map(|(_, c)| c[1]).sum();
    assert_eq!(total, length * (t.expand_per_letter + t.replay_per_letter), "the chunks partition the full tariff");

    let used = amounts(expected.get("used").unwrap());
    let mut runs = 0;
    for dim in Dim::ALL {
        for limit in 0..=used.get(dim) {
            let limits = with_limit(used, dim, limit);
            let context = format!("streaming with {} {limit}", dim.name());
            let [split, coalesced] = given.map(|g| check_predicted(g, &rows, limits, &context));
            assert_eq!(split, coalesced, "{context}");
            if let Some(covered) = split.get("coverage").and_then(|c| c.get("word")) {
                let covered: u128 = covered.num().unwrap().parse().unwrap();
                assert!(covered % chunk == 0 || covered == length, "{context}: coverage {covered}");
            }
            runs += 1;
        }
    }
    assert!(runs > 20, "{runs}");
}

/// The display operations of a trace: the paths of their boundary rows, and the path prefix of each untaken branch.
fn display(trace: &[Json], prefix: &str, boundary: &mut Vec<String>, untaken: &mut Vec<String>) {
    for op in trace {
        let at = format!("{prefix}{}", text(op, "at"));
        match text(op, "op") {
            "display_if" | "display_branch" => {
                boundary.push(at.clone());
                let taken = op.get("condition").and_then(Json::bool).expect("condition");
                let (yes, no) = if taken { ("then", "else") } else { ("else", "then") };
                untaken.push(format!("{at}/{no}/"));
                display(arr(op, yes), &format!("{at}/{yes}/"), boundary, untaken);
            }
            "display_region" => {
                boundary.extend([format!("{at}/display_enter"), format!("{at}/display_exit")]);
                display(arr(op, "body"), &format!("{at}/"), boundary, untaken);
            }
            _ => {}
        }
    }
}

/// CT-DET-32 (part: both display forms over the reference model and the library meter; RT1's form 1 is in
/// `tests/interp.rs`; not run: form 2 in a program, since RT1 runs only form 1 (no owner yet), and LG2.20 isolation and
/// accounting reproduction across routes, since RT2 and RT3 are not built (no owner yet)): in each frozen trace with a
/// display operation, the untaken branch has no row, and each boundary operation (`display_if`, `display_enter`,
/// `display_branch`, `display_exit`) is one step and nothing else. In each trace with source text, every range's digest
/// is the text's SHA-256 and the range lies on scalar boundaries; under every limit from zero to the use in each
/// dimension, a run stops where DET.56 predicts and its stop carries the source range of the operation that stopped,
/// which for form 2's one-short case is the exit's range; the frozen stops carry theirs.
#[test]
fn ct_det_32_display_branches_and_stops_keep_their_charges_and_source() {
    let root = fixtures();
    let (mut displays, mut stops) = (0, 0);
    for vector in traces(&root) {
        let (id, given) = (text(vector, "id"), vector.get("given").unwrap());
        let (mut boundary, mut untaken) = (Vec::new(), Vec::new());
        display(arr(given, "trace"), "", &mut boundary, &mut untaken);
        if !boundary.is_empty() {
            let actual = Model::new(given, amounts(given.get("limits").unwrap()), Tariffs::TEST).result();
            for (at, c) in frozen_rows(&actual) {
                assert!(!untaken.iter().any(|u| at.starts_with(u.as_str())), "{id}: {at} is in an untaken branch");
                if boundary.contains(&at) {
                    assert!(c == [1, 0, 0, 0, 0] || c == [0; 5], "{id}: {at} {c:?}");
                }
            }
            displays += 1;
        }
        let Some(source_text) = given.get("source_text").and_then(Json::str) else { continue };
        let digest = c600_digest::sha256_hex(source_text.as_bytes());
        let mapped: Vec<&Json> = match given.get("source_map") {
            Some(Json::Obj(map)) => map.iter().map(|(_, range)| range).collect(),
            _ => Vec::new(),
        };
        for range in given.get("source").into_iter().chain(mapped) {
            assert_eq!(text(range, "source"), digest, "{id}");
            let (start, end) = (uint(range, "start") as usize, uint(range, "end") as usize);
            assert!(start <= end && end <= source_text.len(), "{id}: {start}..{end}");
            assert!(source_text.is_char_boundary(start) && source_text.is_char_boundary(end), "{id}: {start}..{end}");
        }
        let expected = vector.get("expected").unwrap();
        if let Some(stop) = expected.get("stop").filter(|stop| **stop != Json::Null) {
            assert_eq!(stop.get("source"), source_of(given, text(stop, "at")), "{id}: the frozen stop");
            assert!(stop.get("source").is_some(), "{id}: the frozen stop has a range");
        }
        let all = Model::new(given, WIDE, Tariffs::TEST).result();
        assert_eq!(text(&all, "outcome"), "completed", "{id}");
        let rows = frozen_rows(&all);
        let used = amounts(all.get("used").unwrap());
        for dim in Dim::ALL {
            for limit in 0..used.get(dim) {
                let context = format!("{id} with {} {limit}", dim.name());
                let actual = check_predicted(given, &rows, with_limit(used, dim, limit), &context);
                assert!(actual.get("stop").unwrap().get("source").is_some(), "{context}: the stop has a range");
                stops += 1;
            }
        }
    }
    eprintln!("CT-DET-32: {displays} traces with a display operation, {stops} stops with a source range");
    assert!(displays >= 8 && stops > 20, "too few: {displays}, {stops}");
}
