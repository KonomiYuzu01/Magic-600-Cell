//! LP2 plan, M2: DET.65's frozen traces run as programs. Every trace the typed IR draft can express runs through
//! `Loaded::run` under the `lp1-exec-draft-1` meter, and each of its metered rows and its cutoff is compared with the
//! frozen vector. Source and fixture evidence only: no timing is measured.
//!
//! # The mapping
//!
//! A frozen trace is abstract: named operations with given charges (CV.2, CV.3). A realisation is a checked `.6cs`
//! program with its arguments and host, and a list of every row of the program's run, in order. A row is either a
//! frozen row, which it must equal exactly (steps, kernel cost, memory, output, released), or the program's own
//! overhead: reading the arguments, the entry call and its frame, a `display_if` condition, `let` bindings, scope
//! exits, returns and the top-level output unit, which a fixture trace lists only when it models a complete program
//! (CV.2). The limits follow LP3's mapping (`c600-runtime/tests/ct_rte_04.rs`): a probe with wide limits measures the
//! overhead. A trace that completes gets the program's use plus the trace's slack, and the run must use what the probe
//! used: where the trace has no slack, as in all but `alias-release` (four steps), that is exactly its limits, and
//! equality is admitted in all four dimensions. A trace that stops gets, on top of the program's charges before its
//! stopped row, the trace's headroom before that row, and the run must stop there, in the frozen phase and scope, with
//! the frozen denied row. The probe must complete, so a trace that stops on a value probes with other arguments that
//! read alike (`probe_args`).
//!
//! The output profile differs (CV.5): the fixture's bytes are PS1.2's retained JSON, the product's are `c600-json-1`'s,
//! UTF-8 with a terminal line feed. A realisation publishes a value whose product bytes are as many as the fixture's:
//! `"abcde"` and a line feed for the fixture's `"\u00e9"` (eight bytes), `{}` and a line feed for `"x"` (three). Its
//! fused output unit (DET.59) is then the frozen `emit` row: encoding work, buffer and bytes in one admission, in
//! DET.56's order. The one exception is `output-retained-at-cutoff`'s denied unit `0`: one fixture byte, while no
//! product unit is shorter than three. Its kernel-cost headroom gets the two bytes of difference, so its stop stays on
//! output with the frozen denied row.
//!
//! # Realised: 20 of 40
//!
//! | Trace | Clause | Program |
//! | --- | --- | --- |
//! | `nested-calls` | DET.58 | `outer()` calls `inner()`, which reads `k`; neither frame has a slot (16 bytes) |
//! | `equal-to-limit` | DET.56 | `limit(7)`: `hold()` binds `k`'s result, released at its scope's exit; a visit of `y` |
//! | `simultaneous-exhaustion` | DET.56 | `show()`: the unit of `publish_view(record { })`, refused on kernel cost |
//! | `simultaneous-memory-output` | DET.56 | the same unit, refused on memory while output is short too |
//! | `step-precedence` | DET.56 | the same unit, refused on steps |
//! | `generator` | DET.61 | `three()` from the trace's seed and path: two draws, then one refused on kernel cost |
//! | `generator-alias-cursor` | DET.61 | `twins()`: `g` started at `root`, aliased as `h`; both draw block 0 |
//! | `generator-returned-owner` | DET.61 | `owner()`: `make()` starts a generator at `root` and returns it |
//! | `generator-child-paths` | DET.61 | `children()`: streams at `root/0` and `root/1`, one draw from each |
//! | `approx-branch-true` | DET.64 | `pick(1, 2, r)`: `display_if` takes `r.v` (two visits) |
//! | `approx-branch-false` | DET.64 | `pick(5, 2, r)`: `display_if` takes `x` (one visit) |
//! | `display-stop-source-range` | DET.56 | `pick(1, 2, r)` stopped on steps at `r.v`, after a non-BMP character |
//! | `output-equality` | DET.59 | `equal("abcde")`: one eight-byte unit at equality |
//! | `output-retained-at-cutoff` | DET.59 | `retain("abcde", "")`: one unit published, the second refused on output |
//! | `oversized-arithmetic` | DET.57 | `mul(2^80, 2^80)`: `exact_mul` of 11 limbs by 11, refused on kernel cost |
//! | `oversized-admitted` | DET.57 | the same product admitted at its limits, published, released at the scope's exit |
//! | `arithmetic-reservation` | DET.57 | `mul(0, 2^80)`: 12 bytes reserved, 10 released as the product's 2 settle |
//! | `alias-release` | DET.58 | `release()`: `mk()` aliases `a` as `b` and returns `b`, so `a` is released first |
//! | `alias-still-live` | DET.58 | `live()`: the same alias, then a five-byte literal refused on memory |
//! | `pointwise-failure` | DET.55 | `check(7)`: `assert` of a false comparison ends the run with its error code |
//!
//! The snapshot `k` is priced as the trace's kernel row (CV.4): the batch's kernel cost, a reservation of the result
//! and the scratch, and a result of exactly the trace's result size, so the scratch is released after the row as the
//! fixture's is.
//!
//! The exact-arithmetic traces price `exact_mul` with the test tariffs (DET.57): eight-bit limbs, one kernel unit per
//! pair of limbs, and a reservation of one byte plus the operands' bits in bytes. The product is published, and the
//! published unit is the trace's value.
//!
//! `let` scopes nest, so the alias traces, which release `a` before its alias `b`, cross a call: `mk` binds `a`,
//! aliases it as `b` and returns `b`, which leaves with its own owner while `mk`'s scope exit releases `a`. The
//! trace's `c` is a literal of the caller, or in `alias-still-live` of a further callee `yz` whose frame has as many
//! slots as `mk`'s (two unit `let`s): the overhead's memory peak inside `mk` is then exactly the live memory at `c`
//! plus the trace's headroom, so the memory stop needs no raise.
//!
//! `pointwise-failure` stops on a value: its probe runs `check(1)`, whose comparison holds, and the run's `7` reads
//! alike (one digit), so every row before the assertion is the same. `check` binds the comparison with `let`, so the
//! assertion visits a borrowed predicate and releases nothing, as the trace's does; the comparison's byte goes at the
//! scope's exit, or with the cleanup after the failure.
//!
//! The generator traces seed in two ways. `generator` draws with `random_draw` from the run's own generator, which the
//! run starts at the trace's seed and path. The other three start generators of their own with `rng_init` (LP2
//! draft), whose stream path is the run's path followed by its segments: they run at the trace's seed and the empty
//! path, so that each `rng_init` names the trace's path in full, and the run's own start, `entry#rng_init` on the
//! empty path (49 bytes of key input), is overhead. `make`'s frame has no slot, as the trace's call has none, and its
//! return moves the generator to its caller. The child paths are named streams: child jobs, which assign a child
//! invocation's index (DET.61), wait for M6.
//!
//! # Not realised: 20 of 40
//!
//! [`NOT_EXPRESSIBLE`] lists each with its reason, and `every_frozen_trace_is_realised_or_listed_once` holds the two
//! lists to the 40 frozen vectors.

#[path = "../../c600-check/tests/support/json.rs"]
#[allow(dead_code)]
mod json;

use c600_check::check;
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink, SourceRanges};
use c600_run::meter::{code, frame_size, Amounts, Outcome, Row, SourceRange, Summary, Tariffs};
use c600_run::value::Value;
use c600_syntax::{lower, parse, sha256_hex, Lowered};
use json::Json;
use num_bigint::{BigInt, BigUint};

const VECTORS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/architecture/1.0/language/contract-vectors/vectors.json");

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The frozen traces no program of the IR draft can express, and why.
const NOT_EXPRESSIBLE: [(&str, &str); 20] = [
    ("child-allowances", "the IR has no child scope (child jobs, M6)"),
    ("child-return-owner", "the IR has no child scope"),
    ("root-before-child", "the IR has no child scope"),
    ("dimension-before-scope", "the IR has no child scope"),
    ("nested-child-ledger", "the IR has no child scope"),
    ("child-decode-error-releases-reservation", "the IR has no child scope and no byte decoding"),
    (
        "generator-nested-same-name",
        "its call's frame has no slot, yet the callee binds `g` and `a` and releases them with the frame; the IR binds \
         a name with `let`, which takes a frame slot and releases at its scope's exit (DET.57)",
    ),
    ("text-comparison", "the IR has no text order"),
    ("decode-scalar", "the IR has no byte decoding: arguments and host results cross as values"),
    ("decode-surrogate", "the IR has no byte decoding: arguments and host results cross as values"),
    ("abstract-trace-charges", "the IR has no loop or flow check"),
    ("final-loop-test", "the IR has no loop; recursion adds call frames the trace's loop does not have"),
    ("streaming", "the interpreter has no streaming cursor: a word crosses whole"),
    ("streaming-complete-split", "the interpreter has no streaming cursor: a word crosses whole"),
    ("streaming-complete-coalesced", "the interpreter has no streaming cursor: a word crosses whole"),
    ("form2-equal-limit", "display form 2 needs execution contract `lp1-exec-region-draft-1`, which has no executor"),
    ("form2-one-short", "display form 2 needs execution contract `lp1-exec-region-draft-1`, which has no executor"),
    (
        "form2-untaken-branch",
        "display form 2 needs execution contract `lp1-exec-region-draft-1`, which has no executor",
    ),
    ("form2-publication-equality", "display form 2 needs `lp1-exec-region-draft-1`, which has no executor"),
    ("form2-publication-output-cutoff", "display form 2 needs `lp1-exec-region-draft-1`, which has no executor"),
];

// ---------------------------------------------------------------------------------------------------------------
// The frozen vectors

fn fixtures() -> Json {
    let text = std::fs::read_to_string(VECTORS).unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    Json::parse(&text).unwrap_or_else(|e| panic!("vectors.json does not parse: {e}"))
}

fn text<'a>(j: &'a Json, key: &str) -> &'a str {
    j.get(key).and_then(Json::str).unwrap_or_else(|| panic!("text field `{key}`"))
}

fn uint(j: &Json, key: &str) -> u128 {
    let n = j.get(key).and_then(Json::num).unwrap_or_else(|| panic!("number field `{key}`"));
    n.parse().unwrap_or_else(|e| panic!("`{key}` is not a nonnegative integer: {e}"))
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex digits")).collect()
}

fn amounts(j: &Json) -> Amounts {
    Amounts {
        steps: uint(j, "steps"),
        kernel_cost: uint(j, "kernel_cost"),
        memory: uint(j, "memory"),
        output: uint(j, "output"),
    }
}

fn all(root: &Json) -> &[Json] {
    root.get("vectors").and_then(Json::arr).expect("vectors")
}

fn vector<'a>(root: &'a Json, id: &str) -> &'a Json {
    all(root).iter().find(|v| text(v, "id") == id).unwrap_or_else(|| panic!("no frozen trace {id}"))
}

struct TraceStop {
    at: String,
    phase: String,
    scope: String,
    /// The source digest and byte interval.
    source: Option<(String, usize, usize)>,
}

/// A frozen trace's `given` and `expected`, as far as a realisation compares them.
struct Trace<'a> {
    given: &'a Json,
    limits: Amounts,
    outcome: String,
    code: Option<String>,
    stop: Option<TraceStop>,
    used: Amounts,
    rows: Vec<(String, [u128; 5])>,
    values: Vec<(String, Json)>,
}

impl<'a> Trace<'a> {
    fn read(vector: &'a Json) -> Trace<'a> {
        let given = vector.get("given").expect("given");
        let expected = vector.get("expected").expect("expected");
        let stop = match expected.get("stop") {
            Some(Json::Null) | None => None,
            Some(s) => Some(TraceStop {
                at: text(s, "at").into(),
                phase: text(s, "phase").into(),
                scope: text(s, "scope").into(),
                source: s
                    .get("source")
                    .map(|r| (text(r, "source").into(), uint(r, "start") as usize, uint(r, "end") as usize)),
            }),
        };
        let rows = expected.get("charges").and_then(Json::arr).expect("charges");
        let rows = rows
            .iter()
            .map(|row| {
                let row = row.arr().expect("a charge row");
                let n = |i: usize| row[i].num().expect("a charge").parse::<u128>().expect("a nonnegative charge");
                (row[0].str().expect("an operation").to_string(), [n(1), n(2), n(3), n(4), n(5)])
            })
            .collect();
        let values = match expected.get("values") {
            Some(Json::Obj(fields)) => fields.clone(),
            other => panic!("values: {other:?}"),
        };
        assert_eq!(expected.get("children").and_then(Json::arr).map(<[Json]>::len), Some(0), "no child records");
        Trace {
            given,
            limits: amounts(given.get("limits").expect("limits")),
            outcome: text(expected, "outcome").into(),
            code: expected.get("code").and_then(Json::str).map(str::to_string),
            stop,
            used: amounts(expected.get("used").expect("used")),
            rows,
            values,
        }
    }

    /// The trace's first operation `name`, at any depth.
    fn first(&self, name: &str) -> Option<&'a Json> {
        fn find<'j>(ops: &'j [Json], name: &str) -> Option<&'j Json> {
            ops.iter().find_map(|op| {
                if op.get("op").and_then(Json::str) == Some(name) {
                    return Some(op);
                }
                ["body", "then", "else"].iter().find_map(|k| op.get(k).and_then(Json::arr).and_then(|b| find(b, name)))
            })
        }
        find(self.given.get("trace").and_then(Json::arr).expect("a trace"), name)
    }

    /// The trace's kernel operation, at any depth.
    fn kernel(&self) -> Option<&'a Json> {
        self.first("kernel")
    }

    /// The seed and stream path of the trace's first `rng_init`, at any depth, if it has one.
    fn seed(&self) -> Option<(Vec<u8>, Vec<&'a str>)> {
        self.first("rng_init").map(|init| {
            let path = init.get("path").and_then(Json::arr).expect("a path");
            (hex(text(init, "seed")), path.iter().map(|p| p.str().expect("a path segment")).collect())
        })
    }

    fn value(&self, at: &str) -> Vec<u8> {
        let (_, v) = self.values.iter().find(|(k, _)| k == at).unwrap_or_else(|| panic!("no value `{at}`"));
        hex(v.str().expect("hex bytes"))
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The programs

/// One checked module `m` that requires the given capabilities.
struct Program {
    source: String,
    lowered: Lowered,
    loaded: Loaded,
}

fn program(requires: &str, declarations: &str) -> Program {
    let source = format!("edition \"6cs-draft-1\";\nmodule m requires [{requires}];\n\n{declarations}");
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{source}\n{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let loaded = load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"));
    Program { source, lowered, loaded }
}

struct Ranges<'a>(&'a Lowered);

impl SourceRanges for Ranges<'_> {
    fn range(&self, module: &str, function: &str, path: &str) -> Option<SourceRange> {
        let map = self.0.source(module)?;
        let span = map.span(function, path)?;
        Some(SourceRange { digest: map.digest.clone(), start: span.start, end: span.end })
    }
}

/// The realisations' host: the snapshot `k`, priced as the trace's kernel row (module docs), and published views,
/// delivered and recorded.
#[derive(Default)]
struct TraceHost {
    k: Option<(RowTariff, Value)>,
    views: Vec<Vec<u8>>,
}

impl TraceHost {
    fn for_trace(trace: &Trace) -> TraceHost {
        let k = trace.kernel().map(|k| {
            let result = k.get("result_bytes").map_or(0, |_| uint(k, "result_bytes"));
            let value = if result == 0 { Value::Unit } else { Value::record(vec![]) };
            assert_eq!(value.size(), result, "a result of the trace's size");
            let tariff = RowTariff {
                kernel_cost: Tariffs::TEST.kernel(uint(k, "items")),
                result_bound: result + uint(k, "scratch"),
            };
            (tariff, value)
        });
        TraceHost { k, views: Vec::new() }
    }
}

impl Host for TraceHost {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match (request, &self.k) {
            (Request::Snapshot { query: "k", .. }, Some((tariff, _))) => Ok(*tariff),
            _ => Err(code::NOT_GRANTED),
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match (request, &self.k) {
            (Request::Snapshot { query: "k", .. }, Some((_, value))) => Ok(value.clone()),
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

/// One frozen trace as a program.
#[derive(Clone, Copy)]
struct Realisation {
    trace: &'static str,
    requires: &'static str,
    declarations: &'static str,
    entry: &'static str,
    args: fn() -> Vec<Value>,
    /// The probe's arguments, where they differ: a trace that stops on a value probes with arguments that read alike
    /// and let the probe complete (module docs).
    probe_args: Option<fn() -> Vec<Value>>,
    /// Every row of the program's completed run, in order: its operation path, and the frozen row it is, if any.
    rows: &'static [(&'static str, Option<&'static str>)],
    /// For a stopping trace: steps, kernel cost, memory and output of the program's stopped operation, admitted.
    stop_would: Option<[u128; 4]>,
    /// Kernel-cost headroom added to the trace's before its stopped row (module docs: only the profile difference).
    kernel_extra: u128,
    /// The program text of the node that a stop's source range must cover.
    source_node: Option<&'static str>,
}

const NO_ARGS: fn() -> Vec<Value> = Vec::new;

fn charges(row: &Row) -> [u128; 5] {
    [row.steps, row.kernel_cost, row.memory, row.output, row.released]
}

fn listing(summary: &Summary) -> String {
    summary.rows.iter().map(|row| format!("\n  {} {:?}", row.at, charges(row))).collect()
}

/// Cumulative charges, live memory and its peak before a row.
#[derive(Default)]
struct Before {
    steps: u128,
    kernel: u128,
    output: u128,
    live: u128,
    peak: u128,
}

impl Before {
    fn of(rows: impl Iterator<Item = [u128; 5]>) -> Before {
        let mut b = Before::default();
        for [steps, kernel, memory, output, released] in rows {
            b.peak = b.peak.max(b.live + memory);
            b.live = b.live + memory - released;
            b.steps += steps;
            b.kernel += kernel;
            b.output += output;
        }
        b
    }
}

fn headroom(id: &str, limit: u128, before: u128) -> u128 {
    limit.checked_sub(before).unwrap_or_else(|| panic!("{id}: a trace above its limit before its stop"))
}

/// The run's limits: the trace's headroom on every frozen row (module docs).
fn limits(r: &Realisation, trace: &Trace, probe: &Summary, mapped: &[usize]) -> Amounts {
    let id = r.trace;
    let Some(stop) = &trace.stop else {
        let (l, u) = (trace.limits, trace.used);
        return Amounts {
            steps: probe.used.steps + headroom(id, l.steps, u.steps),
            kernel_cost: probe.used.kernel_cost + headroom(id, l.kernel_cost, u.kernel_cost),
            memory: probe.used.memory + headroom(id, l.memory, u.memory),
            output: probe.used.output + headroom(id, l.output, u.output),
        };
    };
    let s = trace.rows.len() - 1;
    let t = Before::of(trace.rows[..s].iter().map(|(_, c)| *c));
    let p = Before::of(probe.rows[..mapped[s]].iter().map(charges));
    let live = p.live + headroom(id, trace.limits.memory, t.live);
    let memory = if stop.phase == "memory" {
        assert!(p.peak <= live, "{id}: the overhead before the stop needs more memory than the trace's headroom");
        live
    } else {
        live.max(p.peak)
    };
    Amounts {
        steps: p.steps + headroom(id, trace.limits.steps, t.steps),
        kernel_cost: p.kernel + headroom(id, trace.limits.kernel_cost, t.kernel) + r.kernel_extra,
        memory,
        output: p.output + headroom(id, trace.limits.output, t.output),
    }
}

/// A realisation's program, its run with wide limits and its run at the mapped limits, with their hosts.
struct Realised {
    program: Program,
    probe: Run,
    run: Run,
    run_host: TraceHost,
}

fn run(p: &Program, r: &Realisation, trace: &Trace, limits: Amounts, host: &mut TraceHost) -> Run {
    run_with(p, r, r.args, trace, limits, host)
}

fn run_with(
    p: &Program,
    r: &Realisation,
    args: fn() -> Vec<Value>,
    trace: &Trace,
    limits: Amounts,
    host: &mut TraceHost,
) -> Run {
    // A program that starts its own generators names the trace's stream paths in full, below the run's empty path;
    // one that draws with `random_draw` draws from the run's own generator, at the trace's path (module docs).
    let streams = r.declarations.contains("rng_init(");
    let seed = trace.seed().map(|(seed, path)| (seed, if streams { Vec::new() } else { path }));
    let ranges = Ranges(&p.lowered);
    let mut options = Options::new(limits);
    if let Some((seed, path)) = &seed {
        options.seed = Some((&seed[..], &path[..]));
    }
    if r.source_node.is_some() {
        options.sources = Some(&ranges);
    }
    p.loaded.run("m", r.entry, args(), host, &options).unwrap_or_else(|e| panic!("{}: {e:?}", r.trace))
}

/// Runs one realisation against its frozen trace and checks every row and the cutoff (module docs).
fn realise(root: &Json, r: &Realisation) -> Realised {
    let id = r.trace;
    let trace = Trace::read(vector(root, id));
    let p = program(r.requires, r.declarations);

    let probe = run_with(&p, r, r.probe_args.unwrap_or(r.args), &trace, WIDE, &mut TraceHost::for_trace(&trace));
    let list = listing(&probe.summary);
    assert_eq!(probe.summary.outcome, Outcome::Completed, "{id}: the probe completes:{list}");
    let paths: Vec<&str> = probe.summary.rows.iter().map(|row| row.at.as_str()).collect();
    let named: Vec<&str> = r.rows.iter().map(|(at, _)| *at).collect();
    assert_eq!(paths, named, "{id}: every program row is a frozen row or named overhead:{list}");
    let mapped: Vec<usize> = r.rows.iter().enumerate().filter_map(|(i, (_, f))| f.map(|_| i)).collect();
    let frozen: Vec<&str> = r.rows.iter().filter_map(|(_, f)| *f).collect();
    let want: Vec<&str> = trace.rows.iter().map(|(at, _)| at.as_str()).collect();
    assert_eq!(frozen, want, "{id}: every frozen row is mapped, in order");
    let admitted = trace.rows.len() - usize::from(trace.stop.is_some());
    for (i, (at, want)) in trace.rows[..admitted].iter().enumerate() {
        assert_eq!(charges(&probe.summary.rows[mapped[i]]), *want, "{id}: frozen row `{at}`, wide limits:{list}");
    }
    match (&trace.stop, r.stop_would) {
        (Some(_), Some(would)) => {
            let row = charges(&probe.summary.rows[mapped[admitted]]);
            assert_eq!(row[..4], would, "{id}: the stopped operation's charges, admitted");
        }
        (None, None) => {}
        (stop, would) => panic!("{id}: a stop {} realised with {would:?}", stop.is_some()),
    }

    let limits = limits(r, &trace, &probe.summary, &mapped);
    let mut run_host = TraceHost::for_trace(&trace);
    let cut = run(&p, r, &trace, limits, &mut run_host);
    let s = &cut.summary;
    let list = listing(s);
    assert_eq!(s.outcome.name(), trace.outcome, "{id}: outcome:{list}");
    assert_eq!(s.code, trace.code.as_deref(), "{id}: code");
    let length = if trace.stop.is_some() { mapped[admitted] + 1 } else { probe.summary.rows.len() };
    assert_eq!(s.rows.len(), length, "{id}: the run ends at the mapped row:{list}");
    let before = length - usize::from(trace.stop.is_some());
    assert_eq!(s.rows[..before], probe.summary.rows[..before], "{id}: the limits change nothing before the stop");
    for (i, (at, want)) in trace.rows.iter().enumerate() {
        let row = &s.rows[mapped[i]];
        assert_eq!(charges(row), *want, "{id}: frozen row `{at}` and program row `{}`", row.at);
    }
    match (&trace.stop, &s.stop) {
        // Each limit is the probe's use plus the trace's slack: where the trace has none, equality is admitted.
        (None, None) => assert_eq!(s.used, probe.summary.used, "{id}: the run uses what the probe used"),
        (Some(want), Some(got)) => {
            assert_eq!(got.at, r.rows[mapped[admitted]].0, "{id}: the stop is the frozen stop's row");
            assert_eq!(want.at, trace.rows[admitted].0, "{id}: the frozen stop is its last row");
            assert_eq!((got.phase, got.scope.as_str()), (want.phase.as_str(), want.scope.as_str()), "{id}: stop");
            match (&want.source, r.source_node) {
                (None, None) => {}
                (Some((digest, start, end)), Some(node)) => {
                    // The trace's convention: the SHA-256 of its source text and a byte interval that a character
                    // outside the basic plane makes differ from a character interval.
                    let text = text(trace.given, "source_text");
                    assert_eq!(&sha256_hex(text.as_bytes()), digest);
                    assert!(text.get(*start..*end).is_some() && text[..*end].chars().count() != *end);
                    // The program's: the same convention for the stopped node.
                    let source = &p.source;
                    let at = source.find(node).expect("the node's text");
                    assert_eq!(source.matches(node).count(), 1, "the node's text is unique");
                    assert_ne!(source[..at].chars().count(), at, "a multi-byte character precedes the node");
                    let range = got.source.as_ref().expect("the stop carries its source range");
                    assert_eq!(
                        (range.digest.as_str(), range.start, range.end),
                        (sha256_hex(source.as_bytes()).as_str(), at, at + node.len()),
                        "{id}: source association"
                    );
                }
                (want, node) => panic!("{id}: a source association {want:?} realised by {node:?}"),
            }
        }
        (want, got) => panic!("{id}: the trace stops at {:?}, the run at {got:?}", want.as_ref().map(|s| &s.at)),
    }
    assert!(s.children.is_empty());
    assert_eq!(s.live_after_cleanup, 0, "{id}: cleanup releases everything");
    Realised { program: p, probe, run: cut, run_host }
}

// ---------------------------------------------------------------------------------------------------------------
// The realisations

const NESTED: &str = "fn inner() -> Unit effects [snapshot] {\n    return read_snapshot<Unit>(\"k\");\n}\n\n\
                      export fn outer() -> Unit effects [snapshot] {\n    return inner();\n}\n";

const NESTED_CALLS: Realisation = Realisation {
    trace: "nested-calls",
    requires: "",
    declarations: NESTED,
    entry: "outer",
    args: NO_ARGS,
    probe_args: None,
    rows: &[
        ("entry#call", Some("outer/call")),
        ("m.outer:root#call", Some("outer/inner/call")),
        ("m.outer:root > m.inner:root#kernel", Some("outer/inner/k")),
        ("m.outer:root > m.inner:root#return", Some("outer/inner/return")),
        ("m.outer:root#return", Some("outer/return")),
        ("entry#emit", None),
    ],
    stop_would: None,
    kernel_extra: 0,
    source_node: None,
};

/// CT-DET-26 (part: frames and their unwind in nested calls on RT1; not run: moved child results, since the IR has no
/// child construct (child jobs, M6), and RT2 and RT3 (no owner yet)), CT-DET-28 (synthetic, part: calls and returns as
/// one-step rows of their own) and CT-DET-31 (part: nested returns charge no output): the frozen trace `nested-calls`
/// runs as a program, `outer` calling `inner` calling one kernel row. A body that holds no operand has no frame slot,
/// so each `call` reserves and each `return` releases DET.57's 16 bytes, as the trace's frames do, and the five frozen
/// rows are the program's first five. At the trace's own limits the program runs every frozen row and stops only at
/// its own top-level unit, which the trace does not list (CV.2).
#[test]
fn ct_det_26_nested_calls_runs_as_a_program() {
    let root = fixtures();
    let done = realise(&root, &NESTED_CALLS);
    let loaded = &done.program.loaded;
    for f in ["outer", "inner"] {
        assert_eq!(loaded.frame_size(loaded.function("m", f).unwrap()), frame_size(0), "{f}: no slot");
    }
    assert_eq!(done.run.output.as_deref(), Some(&b"null\n"[..]));
    assert!(done.run.summary.rows[..5].iter().all(|r| r.output == 0), "nested returns are internal moves (DET.59)");

    let trace = Trace::read(vector(&root, "nested-calls"));
    let own = run(&done.program, &NESTED_CALLS, &trace, trace.limits, &mut TraceHost::for_trace(&trace));
    assert_eq!((own.summary.outcome, own.summary.code), (Outcome::CutOff, Some(code::STEPS)));
    let stop = own.summary.stop.as_ref().unwrap();
    assert_eq!((stop.at.as_str(), stop.phase), ("entry#emit", "steps"));
    assert_eq!(own.summary.rows[..5], done.probe.summary.rows[..5], "every frozen row is admitted at its limits");
}

/// CT-DET-27 (part: equality admission on RT1; not run: RT2 and RT3 (no owner yet)): the frozen trace
/// `equal-to-limit` runs as a program. A host read priced as the trace's kernel row reserves its result and scratch and
/// keeps the result, its `let` scope's exit drops it, and a visit is the last tick; with every limit at the program's
/// use, each dimension reaches its limit and the run completes.
#[test]
fn ct_det_27_equal_to_limit_runs_as_a_program() {
    realise(
        &fixtures(),
        &Realisation {
            trace: "equal-to-limit",
            requires: "",
            declarations:
                "fn hold() -> Unit effects [snapshot] {\n    let x = read_snapshot<record { }>(\"k\");\n    \
                           return ();\n}\n\n\
                           export fn limit(y: Nat) -> Nat effects [snapshot] {\n    let u = hold();\n    return y;\n}\n",
            entry: "limit",
            args: || vec![Value::int(7)],
            probe_args: None,
            rows: &[
                ("entry#read", None),
                ("entry#call", None),
                ("m.limit:0#call", None),
                ("m.limit:0 > m.hold:0#kernel", Some("k")),
                ("m.limit:0 > m.hold:root#visit", None),
                ("m.limit:0 > m.hold:1#value", None),
                ("m.limit:0 > m.hold:root#scope_exit", Some("drop")),
                ("m.limit:0 > m.hold:root#return", None),
                ("m.limit:root#visit", None),
                ("m.limit:1#visit", Some("last")),
                ("m.limit:root#scope_exit", None),
                ("m.limit:root#alias", None),
                ("m.limit:root#return", None),
                ("entry#emit", None),
            ],
            stop_would: None,
            kernel_extra: 0,
            source_node: None,
        },
    );
}

const SHOW: &str = "export fn show() -> Unit effects [view] {\n    return publish_view(record { });\n}\n";

/// CT-DET-27 (part: simultaneous failures and their precedence at one admission on RT1; not run: child scopes, since
/// the IR has no child construct (child jobs, M6), and RT2 and RT3 (no owner yet)): the frozen traces
/// `step-precedence`, `simultaneous-exhaustion` and `simultaneous-memory-output` run as one program that publishes
/// `{}`, a three-byte unit as the fixture's `"x"`. Its fused row requests the trace's `emit` charges in one admission
/// (DET.59), so each run stops in the frozen dimension with the frozen denied row: steps before everything, kernel cost
/// before memory and output, and memory before output.
#[test]
fn ct_det_27_simultaneous_short_dimensions_run_as_programs() {
    let root = fixtures();
    for trace in ["step-precedence", "simultaneous-exhaustion", "simultaneous-memory-output"] {
        let done = realise(
            &root,
            &Realisation {
                trace,
                requires: "",
                declarations: SHOW,
                entry: "show",
                args: NO_ARGS,
                probe_args: None,
                rows: &[
                    ("entry#call", None),
                    ("m.show:0#value", None),
                    ("m.show:root#publish_view", Some("emit")),
                    ("m.show:root#return", None),
                    ("entry#emit", None),
                ],
                stop_would: Some([1, 3, 3, 3]),
                kernel_extra: 0,
                source_node: None,
            },
        );
        assert!(done.run_host.views.is_empty(), "{trace}: a denied unit publishes nothing");
    }
}

/// CT-DET-29 (part: RT1 only; not run: RT2 and RT3 (no owner yet), and child stream paths, since the IR has no child
/// construct (child jobs, M6)): the frozen trace `generator` runs as a program with the trace's seed and stream path,
/// drawing three times. Its `rng_init` and draw rows are the frozen rows; the third draw is refused on kernel cost with
/// the frozen denied row, and the admitted draws are the frozen blocks.
#[test]
fn ct_det_29_generator_runs_as_a_program() {
    let root = fixtures();
    let done = realise(
        &root,
        &Realisation {
            trace: "generator",
            requires: "random",
            declarations: "export fn three() -> record { a: Nat, b: Nat, c: Nat } effects [random] {\n    \
                           return record { a = random_draw(), b = random_draw(), c = random_draw() };\n}\n",
            entry: "three",
            args: NO_ARGS,
            probe_args: None,
            rows: &[
                ("entry#rng_init", Some("init")),
                ("entry#call", None),
                ("m.three:0#rng_draw", Some("draw-0")),
                ("m.three:1#rng_draw", Some("draw-1")),
                ("m.three:2#rng_draw", Some("denied")),
                ("m.three:root#value", None),
                ("m.three:root#visit", None),
                ("m.three:root#visit", None),
                ("m.three:root#visit", None),
                ("m.three:root#return", None),
                ("entry#emit", None),
            ],
            stop_would: Some([1, 3, 33, 0]),
            kernel_extra: 0,
            source_node: None,
        },
    );
    let trace = Trace::read(vector(&root, "generator"));
    let output = std::str::from_utf8(done.probe.output.as_deref().unwrap()).unwrap();
    for (field, at) in [("a", "draw-0"), ("b", "draw-1")] {
        let block = BigUint::from_bytes_be(&trace.value(at));
        assert!(output.contains(&format!("\"{field}\":{{\"int\":\"{block}\"}}")), "{at}: {output}");
    }
}

const TWINS: Realisation = Realisation {
    trace: "generator-alias-cursor",
    requires: "random",
    declarations:
        "export fn twins() -> record { a: Nat, b: Nat } effects [random] {\n    let g = rng_init(\"root\");\n    \
                   let h = g;\n    return record { a = rng_draw(g), b = rng_draw(h) };\n}\n",
    entry: "twins",
    args: NO_ARGS,
    probe_args: None,
    rows: &[
        ("entry#rng_init", None),
        ("entry#call", None),
        ("m.twins:0#rng_init", Some("init")),
        ("m.twins:root#visit", None),
        ("m.twins:1.0#visit", None),
        ("m.twins:1#alias", Some("alias")),
        ("m.twins:1.1.0.0#visit", None),
        ("m.twins:1.1.0#rng_draw", Some("draw-g")),
        ("m.twins:1.1.1.0#visit", None),
        ("m.twins:1.1.1#rng_draw", Some("draw-h")),
        ("m.twins:1.1#value", None),
        ("m.twins:1.1#visit", None),
        ("m.twins:1.1#visit", None),
        ("m.twins:1#scope_exit", None),
        ("m.twins:root#scope_exit", None),
        ("m.twins:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: None,
    kernel_extra: 0,
    source_node: None,
};

const OWNER: Realisation = Realisation {
    trace: "generator-returned-owner",
    declarations: "fn make() -> Generator effects [random] {\n    return rng_init(\"root\");\n}\n\n\
                   export fn owner() -> Nat effects [random] {\n    let h = make();\n    return rng_draw(h);\n}\n",
    entry: "owner",
    rows: &[
        ("entry#rng_init", None),
        ("entry#call", None),
        ("m.owner:0#call", Some("make/call")),
        ("m.owner:0 > m.make:root#rng_init", Some("make/init")),
        ("m.owner:0 > m.make:root#return", Some("make/return")),
        ("m.owner:root#visit", None),
        ("m.owner:1.0#visit", None),
        ("m.owner:1#rng_draw", Some("draw-h")),
        ("m.owner:root#scope_exit", None),
        ("m.owner:root#return", None),
        ("entry#emit", None),
    ],
    ..TWINS
};

const CHILDREN: Realisation = Realisation {
    trace: "generator-child-paths",
    declarations: "export fn children() -> record { a: Nat, b: Nat } effects [random] {\n    \
                   let g0 = rng_init(\"root\", \"0\");\n    let a = rng_draw(g0);\n    \
                   let g1 = rng_init(\"root\", \"1\");\n    return record { a = a, b = rng_draw(g1) };\n}\n",
    entry: "children",
    rows: &[
        ("entry#rng_init", None),
        ("entry#call", None),
        ("m.children:0#rng_init", Some("init-0")),
        ("m.children:root#visit", None),
        ("m.children:1.0.0#visit", None),
        ("m.children:1.0#rng_draw", Some("draw-0")),
        ("m.children:1#visit", None),
        ("m.children:1.1.0#rng_init", Some("init-1")),
        ("m.children:1.1#visit", None),
        ("m.children:1.1.1.0#visit", None),
        ("m.children:1.1.1.1.0#visit", None),
        ("m.children:1.1.1.1#rng_draw", Some("draw-1")),
        ("m.children:1.1.1#value", None),
        ("m.children:1.1.1#alias", None),
        ("m.children:1.1.1#visit", None),
        ("m.children:1.1#scope_exit", None),
        ("m.children:1#scope_exit", None),
        ("m.children:root#scope_exit", None),
        ("m.children:root#return", None),
        ("entry#emit", None),
    ],
    ..TWINS
};

/// CT-DET-29 (part: named streams, aliases and a returned owner on RT1; not run: RT2 and RT3 (no owner yet), and child
/// indices assigned by child jobs (M6)): the frozen traces `generator-alias-cursor`, `generator-returned-owner` and
/// `generator-child-paths` run as programs that start their own generators (LP2 draft; module docs). An alias copies
/// the cursor at the full 41 units, so both names draw the same block; a generator started in a call leaves with its
/// owner when the call's frame is released; the child paths are named streams. Each drawn value is the trace's.
#[test]
fn ct_det_29_generator_values_run_as_programs() {
    let root = fixtures();
    let cases: [(&Realisation, &[(&str, &str)]); 3] = [
        (&TWINS, &[("\"a\":", "draw-g"), ("\"b\":", "draw-h")]),
        (&OWNER, &[("", "draw-h")]),
        (&CHILDREN, &[("\"a\":", "draw-0"), ("\"b\":", "draw-1")]),
    ];
    for (r, drawn) in cases {
        let done = realise(&root, r);
        let trace = Trace::read(vector(&root, r.trace));
        let output = std::str::from_utf8(done.run.output.as_deref().expect("a completed run")).expect("UTF-8");
        for (field, at) in drawn {
            let block = BigUint::from_bytes_be(&trace.value(at));
            assert!(output.contains(&format!("{field}{{\"int\":\"{block}\"}}")), "{}: {at}: {output}", r.trace);
        }
    }
}

/// The realisations' `display_if`. The comment's character outside the basic plane precedes every node, so a node's
/// byte offset and its character offset differ, as the emoji does in `display-stop-source-range`'s source text.
const PICK: &str = "# A character outside the basic plane, \u{1f600}, parts byte and character offsets.\n\
                    export fn pick(x: Nat, y: Nat, r: record { v: Nat }) -> Display<Nat> effects [] {\n    \
                    let c = approx_less(approx53(x), approx53(y));\n    return display_if(c, r.v, x);\n}\n";

const PICK_THEN: &[(&str, Option<&str>)] = &[
    ("entry#read", None),
    ("entry#read", None),
    ("entry#read", None),
    ("entry#call", None),
    ("m.pick:0.0.0#visit", None),
    ("m.pick:0.0#to_approx", None),
    ("m.pick:0.1.0#visit", None),
    ("m.pick:0.1#to_approx", None),
    ("m.pick:0#approx_less", None),
    ("m.pick:root#visit", None),
    ("m.pick:1.0#visit", None),
    ("m.pick:1#display_if", Some("display")),
    ("m.pick:1.1.0#visit", Some("display/then/a")),
    ("m.pick:1.1#visit", Some("display/then/b")),
    ("m.pick:root#scope_exit", None),
    ("m.pick:root#alias", None),
    ("m.pick:root#return", None),
    ("entry#emit", None),
];

const PICK_ELSE: &[(&str, Option<&str>)] = &[
    ("entry#read", None),
    ("entry#read", None),
    ("entry#read", None),
    ("entry#call", None),
    ("m.pick:0.0.0#visit", None),
    ("m.pick:0.0#to_approx", None),
    ("m.pick:0.1.0#visit", None),
    ("m.pick:0.1#to_approx", None),
    ("m.pick:0#approx_less", None),
    ("m.pick:root#visit", None),
    ("m.pick:1.0#visit", None),
    ("m.pick:1#display_if", Some("display")),
    ("m.pick:1.2#visit", Some("display/else/a")),
    ("m.pick:root#scope_exit", None),
    ("m.pick:root#alias", None),
    ("m.pick:root#return", None),
    ("entry#emit", None),
];

fn pick_args(x: u32, y: u32) -> Vec<Value> {
    vec![Value::int(x), Value::int(y), Value::record(vec![("v", Value::int(3))])]
}

/// CT-DET-32 (part: display form 1 on RT1; not run: form 2, since execution contract `lp1-exec-region-draft-1` has no
/// executor (no owner yet), and RT2 and RT3 (no owner yet)): the frozen traces `approx-branch-true`,
/// `approx-branch-false` and `display-stop-source-range` run as one program. `display_if` is one step and nothing
/// else, the taken branch's rows follow it and the untaken branch has none (DET.64); a run one step short stops on
/// the branch's second row with the source range of its node, a byte interval of the program text under its SHA-256,
/// as the trace's range is of its text.
#[test]
fn ct_det_32_display_traces_run_as_programs() {
    let root = fixtures();
    let base = Realisation {
        trace: "approx-branch-true",
        requires: "",
        declarations: PICK,
        entry: "pick",
        args: || pick_args(1, 2),
        probe_args: None,
        rows: PICK_THEN,
        stop_would: None,
        kernel_extra: 0,
        source_node: None,
    };
    realise(&root, &base);
    realise(&root, &Realisation { trace: "approx-branch-false", args: || pick_args(5, 2), rows: PICK_ELSE, ..base });
    realise(
        &root,
        &Realisation {
            trace: "display-stop-source-range",
            stop_would: Some([1, 0, 0, 0]),
            source_node: Some("r.v"),
            ..base
        },
    );
}

const OUTPUT: &str = "export fn equal(t: Text) -> Unit effects [view] {\n    return publish_view(t);\n}\n\n\
                      export fn retain(t: Text, u: Text) -> Unit effects [view] {\n    let a = publish_view(t);\n    \
                      return publish_view(u);\n}\n";

const OUTPUT_EQUALITY: Realisation = Realisation {
    trace: "output-equality",
    requires: "",
    declarations: OUTPUT,
    entry: "equal",
    args: || vec![Value::text("abcde")],
    probe_args: None,
    rows: &[
        ("entry#read", None),
        ("entry#call", None),
        ("m.equal:0#visit", None),
        ("m.equal:root#publish_view", Some("emit")),
        ("m.equal:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: None,
    kernel_extra: 0,
    source_node: None,
};

/// CT-DET-31 (part: output equality and retained partial output on RT1; not run: the run record's retained output
/// (M6), and RT2 and RT3 (no owner yet)): the frozen traces `output-equality` and `output-retained-at-cutoff` run as
/// programs whose units have the fixture's byte counts (module docs). Each unit is one fused row: its encoding work,
/// buffer and bytes in one admission (DET.56 step 2, DET.59), released after the publication. A unit at equality is
/// admitted, one byte short it is refused with only its step and publishes nothing, and a unit published before a
/// refused one stays published and charged.
#[test]
fn ct_det_31_output_traces_run_as_programs() {
    let root = fixtures();
    let equality = Trace::read(vector(&root, "output-equality"));
    let fixture = equality.value("emit");
    assert_eq!(fixture, b"\"\\u00e9\"", "PS1.2's bytes of \"\u{e9}\"");
    let done = realise(&root, &OUTPUT_EQUALITY);
    assert_eq!(done.run_host.views, [b"\"abcde\"\n"]);
    assert_eq!(done.run_host.views[0].len(), fixture.len(), "the product unit has the fixture's byte count");

    // At the unit itself: equality admits it and leaves no byte for the program's top-level unit; one byte short
    // refuses it with only its step.
    let p = &done.program;
    let unit = 3;
    let before = Before::of(done.probe.summary.rows[..unit].iter().map(charges));
    for (output, published) in [(before.output + 8, 1), (before.output + 7, 0)] {
        let mut host = TraceHost::default();
        let r = run(p, &OUTPUT_EQUALITY, &equality, Amounts { output, ..WIDE }, &mut host);
        assert_eq!((r.summary.outcome, r.summary.code), (Outcome::CutOff, Some(code::OUTPUT)));
        let stop = &r.summary.stop.as_ref().unwrap().at;
        assert_eq!(stop, if published == 1 { "entry#emit" } else { "m.equal:root#publish_view" });
        assert_eq!(host.views.len(), published);
        assert_eq!(r.summary.used.output, 8 * published as u128, "no byte of a refused unit is charged");
        if published == 0 {
            assert_eq!(charges(r.summary.rows.last().unwrap()), [1, 0, 0, 0, 0]);
        }
    }

    let retained = realise(
        &root,
        &Realisation {
            trace: "output-retained-at-cutoff",
            entry: "retain",
            args: || vec![Value::text("abcde"), Value::text("")],
            rows: &[
                ("entry#read", None),
                ("entry#read", None),
                ("entry#call", None),
                ("m.retain:0.0#visit", None),
                ("m.retain:0#publish_view", Some("first")),
                ("m.retain:root#visit", None),
                ("m.retain:1.0#visit", None),
                ("m.retain:1#publish_view", Some("denied")),
                ("m.retain:root#scope_exit", None),
                ("m.retain:root#return", None),
                ("entry#emit", None),
            ],
            stop_would: Some([1, 3, 3, 3]),
            kernel_extra: 3 - 1,
            ..OUTPUT_EQUALITY
        },
    );
    let trace = Trace::read(vector(&root, "output-retained-at-cutoff"));
    assert_eq!(trace.value("first").len(), 8);
    assert_eq!(retained.run_host.views, [b"\"abcde\"\n"], "only the admitted unit reaches the host");
    assert_eq!(retained.run.summary.used.output, trace.used.output, "the published unit stays charged");
}

const MUL: &str = "export fn mul(a: Int, b: Int) -> Unit effects [view] {\n    let x = exact_mul(a, b);\n    \
                   let v = publish_view(x);\n    return ();\n}\n";

fn power(n: u32) -> Value {
    Value::int(BigInt::from(1u8) << n)
}

/// `mk` binds `a`, aliases it as `b` and returns `b`; `yz`'s frame has as many slots as `mk`'s (module docs).
const ALIAS: &str = "fn mk() -> Nat effects [] {\n    let a = 65536;\n    let b = a;\n    return b;\n}\n\n\
                     fn mid() -> Nat effects [] {\n    let b = mk();\n    return 65537;\n}\n\n\
                     export fn release() -> Unit effects [] {\n    let c = mid();\n    return ();\n}\n\n\
                     fn yz() -> Nat effects [] {\n    let p = ();\n    let q = ();\n    return 16777216;\n}\n\n\
                     fn late() -> Nat effects [] {\n    let b = mk();\n    return yz();\n}\n\n\
                     export fn live() -> Unit effects [] {\n    let c = late();\n    return ();\n}\n";

const CHECK: &str =
    "export fn check(n: Nat) -> Unit effects [] {\n    let p = exact_less(n, 5);\n    return assert(p);\n}\n";

/// `mul`'s rows where the trace completes: the product is the frozen `mul` and its scope's exit the frozen `drop`.
const MUL_ROWS: &[(&str, Option<&str>)] = &[
    ("entry#read", None),
    ("entry#read", None),
    ("entry#call", None),
    ("m.mul:0.0#visit", None),
    ("m.mul:0.1#visit", None),
    ("m.mul:0#exact_mul", Some("mul")),
    ("m.mul:root#visit", None),
    ("m.mul:1.0.0#visit", None),
    ("m.mul:1.0#publish_view", None),
    ("m.mul:1#visit", None),
    ("m.mul:1.1#value", None),
    ("m.mul:1#scope_exit", None),
    ("m.mul:root#scope_exit", Some("drop")),
    ("m.mul:root#return", None),
    ("entry#emit", None),
];

/// `mul`'s rows where the trace stops at the product, its only row.
const MUL_STOP_ROWS: &[(&str, Option<&str>)] = &[
    ("entry#read", None),
    ("entry#read", None),
    ("entry#call", None),
    ("m.mul:0.0#visit", None),
    ("m.mul:0.1#visit", None),
    ("m.mul:0#exact_mul", Some("mul")),
    ("m.mul:root#visit", None),
    ("m.mul:1.0.0#visit", None),
    ("m.mul:1.0#publish_view", None),
    ("m.mul:1#visit", None),
    ("m.mul:1.1#value", None),
    ("m.mul:1#scope_exit", None),
    ("m.mul:root#scope_exit", None),
    ("m.mul:root#return", None),
    ("entry#emit", None),
];

/// CT-DET-28 (synthetic, part: oversized arithmetic on RT1; not run: text work, loop tests and flow checks, since the
/// IR has no text order, loop or flow check, and RT2 and RT3 (no owner yet)): the frozen traces `oversized-arithmetic`,
/// `oversized-admitted` and `arithmetic-reservation` run as one program, which binds `exact_mul(a, b)` and publishes
/// it. The product's row is the frozen `mul` row, priced from its operands' sizes (DET.57): 2^80 by itself is refused
/// on kernel cost one unit short and admitted at its limits, and a product by zero reserves for the operands' bits and
/// releases all but the product's two bytes in its own row. The scope's exit that releases the product is the frozen
/// `drop`, and the published product is the trace's value.
#[test]
fn ct_det_28_oversized_arithmetic_runs_as_programs() {
    let root = fixtures();
    let refused = Realisation {
        trace: "oversized-arithmetic",
        requires: "",
        declarations: MUL,
        entry: "mul",
        args: || vec![power(80), power(80)],
        probe_args: None,
        rows: MUL_STOP_ROWS,
        stop_would: Some([1, 121, 22, 0]),
        kernel_extra: 0,
        source_node: None,
    };
    assert!(realise(&root, &refused).run_host.views.is_empty(), "a refused product publishes nothing");
    let admitted = Realisation { trace: "oversized-admitted", rows: MUL_ROWS, stop_would: None, ..refused };
    let reservation =
        Realisation { trace: "arithmetic-reservation", args: || vec![Value::int(0), power(80)], ..admitted };
    for r in [admitted, reservation] {
        let done = realise(&root, &r);
        let trace = Trace::read(vector(&root, r.trace));
        let [(name, product)] = &trace.values[..] else { panic!("{}: one value", r.trace) };
        assert_eq!(name, "mul");
        let product = BigInt::parse_bytes(product.num().expect("a number").as_bytes(), 10).expect("decimal digits");
        let unit = format!("{{\"int\":\"{product}\"}}\n");
        assert_eq!(done.run_host.views, [unit.as_bytes()], "{}: the published product is the trace's", r.trace);
    }
}

/// CT-DET-26 (part: alias and drop across calls on RT1; not run: moved child results, since the IR has no child
/// construct (child jobs, M6), and RT2 and RT3 (no owner yet)): the frozen traces `alias-release` and
/// `alias-still-live` run as programs (module docs). `mk`'s alias reserves an owner of its own, its scope's exit
/// releases `a` while `b` leaves as the result, and the caller releases `b` after its own `c`. With `b` still live,
/// `alias-still-live`'s five-byte `c` is refused on memory with the frozen denied row.
#[test]
fn ct_det_26_alias_traces_run_as_programs() {
    let root = fixtures();
    let release = Realisation {
        trace: "alias-release",
        requires: "",
        declarations: ALIAS,
        entry: "release",
        args: NO_ARGS,
        probe_args: None,
        rows: &[
            ("entry#call", None),
            ("m.release:0#call", None),
            ("m.release:0 > m.mid:0#call", None),
            ("m.release:0 > m.mid:0 > m.mk:0#value", Some("a")),
            ("m.release:0 > m.mid:0 > m.mk:root#visit", None),
            ("m.release:0 > m.mid:0 > m.mk:1.0#visit", None),
            ("m.release:0 > m.mid:0 > m.mk:1#alias", Some("b")),
            ("m.release:0 > m.mid:0 > m.mk:1.1#visit", None),
            ("m.release:0 > m.mid:0 > m.mk:1#scope_exit", None),
            ("m.release:0 > m.mid:0 > m.mk:root#scope_exit", Some("drop-a")),
            ("m.release:0 > m.mid:0 > m.mk:root#return", None),
            ("m.release:0 > m.mid:root#visit", None),
            ("m.release:0 > m.mid:1#value", Some("c")),
            ("m.release:0 > m.mid:root#scope_exit", Some("drop-b")),
            ("m.release:0 > m.mid:root#return", None),
            ("m.release:root#visit", None),
            ("m.release:1#value", None),
            ("m.release:root#scope_exit", Some("drop-c")),
            ("m.release:root#return", None),
            ("entry#emit", None),
        ],
        stop_would: None,
        kernel_extra: 0,
        source_node: None,
    };
    realise(&root, &release);
    realise(
        &root,
        &Realisation {
            trace: "alias-still-live",
            entry: "live",
            rows: &[
                ("entry#call", None),
                ("m.live:0#call", None),
                ("m.live:0 > m.late:0#call", None),
                ("m.live:0 > m.late:0 > m.mk:0#value", Some("a")),
                ("m.live:0 > m.late:0 > m.mk:root#visit", None),
                ("m.live:0 > m.late:0 > m.mk:1.0#visit", None),
                ("m.live:0 > m.late:0 > m.mk:1#alias", Some("b")),
                ("m.live:0 > m.late:0 > m.mk:1.1#visit", None),
                ("m.live:0 > m.late:0 > m.mk:1#scope_exit", None),
                ("m.live:0 > m.late:0 > m.mk:root#scope_exit", Some("drop-a")),
                ("m.live:0 > m.late:0 > m.mk:root#return", None),
                ("m.live:0 > m.late:root#visit", None),
                ("m.live:0 > m.late:1#call", None),
                ("m.live:0 > m.late:1 > m.yz:0#value", None),
                ("m.live:0 > m.late:1 > m.yz:root#visit", None),
                ("m.live:0 > m.late:1 > m.yz:1.0#value", None),
                ("m.live:0 > m.late:1 > m.yz:1#visit", None),
                ("m.live:0 > m.late:1 > m.yz:1.1#value", Some("c")),
                ("m.live:0 > m.late:1 > m.yz:1#scope_exit", None),
                ("m.live:0 > m.late:1 > m.yz:root#scope_exit", None),
                ("m.live:0 > m.late:1 > m.yz:root#return", None),
                ("m.live:0 > m.late:root#scope_exit", None),
                ("m.live:0 > m.late:root#return", None),
                ("m.live:root#visit", None),
                ("m.live:1#value", None),
                ("m.live:root#scope_exit", None),
                ("m.live:root#return", None),
                ("entry#emit", None),
            ],
            stop_would: Some([1, 0, 5, 0]),
            ..release
        },
    );
}

/// CT-DET-28 (synthetic, part: assertions on RT1; not run: loop tests and flow checks, since the IR has no loop or flow
/// check, and RT2 and RT3 (no owner yet)): the frozen trace `pointwise-failure` runs as a program, `check(n)`, which
/// binds `exact_less(n, 5)` and asserts it. The predicate's visit is the frozen `predicate` row and the assertion the
/// frozen `assert` row, one step after the predicate (DET.55); `check(7)` ends there with `LG4:input.pointwise_failed`
/// in the effect phase, emits nothing, and the cleanup releases the comparison's byte.
#[test]
fn ct_det_28_pointwise_failure_runs_as_a_program() {
    let done = realise(
        &fixtures(),
        &Realisation {
            trace: "pointwise-failure",
            requires: "",
            declarations: CHECK,
            entry: "check",
            args: || vec![Value::int(7)],
            probe_args: Some(|| vec![Value::int(1)]),
            rows: &[
                ("entry#read", None),
                ("entry#call", None),
                ("m.check:0.0#visit", None),
                ("m.check:0.1#value", None),
                ("m.check:0#exact_less", None),
                ("m.check:root#visit", None),
                ("m.check:1.0#visit", Some("predicate")),
                ("m.check:1#assert", Some("assert")),
                ("m.check:root#scope_exit", None),
                ("m.check:root#return", None),
                ("entry#emit", None),
            ],
            stop_would: Some([1, 0, 0, 0]),
            kernel_extra: 0,
            source_node: None,
        },
    );
    assert!(done.run.output.is_none(), "a failed assertion emits nothing");
    assert_eq!(done.probe.output.as_deref(), Some(&b"null\n"[..]), "a held assertion is the unit");
}

/// Every frozen trace is realised in this file or listed in [`NOT_EXPRESSIBLE`] with its reason, exactly once.
#[test]
fn every_frozen_trace_is_realised_or_listed_once() {
    let root = fixtures();
    let realised = [
        "nested-calls",
        "equal-to-limit",
        "step-precedence",
        "simultaneous-exhaustion",
        "simultaneous-memory-output",
        "generator",
        "generator-alias-cursor",
        "generator-returned-owner",
        "generator-child-paths",
        "approx-branch-true",
        "approx-branch-false",
        "display-stop-source-range",
        "output-equality",
        "output-retained-at-cutoff",
        "oversized-arithmetic",
        "oversized-admitted",
        "arithmetic-reservation",
        "alias-release",
        "alias-still-live",
        "pointwise-failure",
    ];
    let mut ids: Vec<&str> = all(&root).iter().map(|v| text(v, "id")).collect();
    let mut accounted: Vec<&str> = realised.iter().copied().chain(NOT_EXPRESSIBLE.iter().map(|n| n.0)).collect();
    ids.sort_unstable();
    accounted.sort_unstable();
    assert_eq!(ids.len(), 40);
    assert_eq!(accounted, ids, "every frozen trace is realised or listed with its reason, exactly once");
}
