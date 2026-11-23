//! Shared fixtures of the research library's tests: modules lowered, checked and loaded, runs on `c600-run` with no
//! host, value builders and readers, and a small deterministic generator for synthetic fixtures. Every value here is
//! a source or fixture value, and no search output is research evidence (LP5 plan, section 9, reading B).

#![allow(dead_code)]

use c600_check::check;
use c600_library::lower_sources;
use c600_run::interp::{load, Loaded, NoHost, Options, Run};
use c600_run::meter::{Amounts, Outcome};
use c600_run::value::Value;
use c600_syntax::{format, parse};

/// Run limits wide enough for every fixture.
pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The repository's root.
pub fn repository() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The sources lowered together, checked with no diagnostic and loaded.
pub fn loaded(sources: &[&str]) -> Loaded {
    for source in sources {
        let file = parse(source.as_bytes());
        assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
        assert_eq!(format(&file).as_deref(), Some(*source), "the source is in the canonical layout");
    }
    let program = lower_sources(sources).expect("the sources lower together");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&program).expect("the program loads")
}

/// A run of `module.function` on `args` under `options`, with no host.
pub fn run_with(loaded: &Loaded, module: &str, function: &str, args: Vec<Value>, options: &Options) -> Run {
    loaded.run(module, function, args, &mut NoHost, options).expect("the run starts")
}

/// The value of a run of `module.function` that completes under [`WIDE`] with no code.
pub fn call(loaded: &Loaded, module: &str, function: &str, args: Vec<Value>) -> Value {
    let run = run_with(loaded, module, function, args, &Options::new(WIDE));
    assert_eq!(
        run.summary.outcome,
        Outcome::Completed,
        "{module}.{function}: {:?} {:?}",
        run.summary.code,
        run.summary.stop
    );
    assert_eq!(run.summary.code, None);
    run.value.expect("a value")
}

pub fn nat(n: u64) -> Value {
    Value::int(n)
}

pub fn int(n: i64) -> Value {
    Value::int(n)
}

pub fn text(s: &str) -> Value {
    Value::text(s)
}

pub fn boolean(b: bool) -> Value {
    Value::Bool(b)
}

pub fn list(items: Vec<Value>) -> Value {
    Value::list(items)
}

pub fn nats(values: &[u64]) -> Value {
    Value::list(values.iter().map(|&n| Value::int(n)).collect())
}

pub fn texts(values: &[&str]) -> Value {
    Value::list(values.iter().map(|s| Value::text(s)).collect())
}

/// A record; the fields must be given in the order the source declares them.
pub fn record(fields: Vec<(&str, Value)>) -> Value {
    Value::record(fields)
}

/// The field `name` of a record value.
pub fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    match value {
        Value::Record(r) => r.get(name).unwrap_or_else(|| panic!("no field {name} in {value:?}")),
        other => panic!("not a record: {other:?}"),
    }
}

/// An integer value in decimal.
pub fn decimal(value: &Value) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        other => panic!("not an integer: {other:?}"),
    }
}

pub fn as_i64(value: &Value) -> i64 {
    decimal(value).parse().unwrap_or_else(|_| panic!("an integer that fits 64 bits: {value:?}"))
}

pub fn as_u64(value: &Value) -> u64 {
    decimal(value).parse().unwrap_or_else(|_| panic!("a natural that fits 64 bits: {value:?}"))
}

pub fn as_bool(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        other => panic!("not a boolean: {other:?}"),
    }
}

pub fn as_text(value: &Value) -> String {
    match value {
        Value::Text(t) => t.as_str().to_string(),
        other => panic!("not a text: {other:?}"),
    }
}

pub fn items(value: &Value) -> &[Value] {
    match value {
        Value::List(l) => l.items(),
        other => panic!("not a list: {other:?}"),
    }
}

pub fn u64s(value: &Value) -> Vec<u64> {
    items(value).iter().map(as_u64).collect()
}

pub fn i64s(value: &Value) -> Vec<i64> {
    items(value).iter().map(as_i64).collect()
}

pub fn strings(value: &Value) -> Vec<String> {
    items(value).iter().map(as_text).collect()
}

/// A small deterministic generator (SplitMix64) for synthetic fixtures; its seeds are written in the tests.
pub struct Fixtures(u64);

impl Fixtures {
    pub fn new(seed: u64) -> Fixtures {
        Fixtures(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// A permutation of `0..n`.
    pub fn permutation(&mut self, n: usize) -> Vec<u64> {
        let mut p: Vec<u64> = (0..n as u64).collect();
        for i in (1..n).rev() {
            let j = self.below(i as u64 + 1) as usize;
            p.swap(i, j);
        }
        p
    }
}
