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
//! # Realised: 32 of 40
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
//! | `final-loop-test` | DET.55 | `spin()`: `repeat(1, 0, …)` visits `acc` once; the final test is refused on steps |
//! | `child-allowances` | DET.60 | `allow()`: a two-step child, cut off at its `return`; the root cuts its kernel cap |
//! | `child-return-owner` | DET.60 | `owned()`: a child's four-byte text moves to the parent, which drops it |
//! | `root-before-child` | DET.60 | `early()`: the root and the child run out of steps at the body's `return` |
//! | `dimension-before-scope` | DET.60 | `dim()`: a child's `k`, over its kernel cap while the root lacks memory |
//! | `nested-child-ledger` | DET.60 | `nest()`: a child in a child; both complete, each charged in its ancestors |
//! | `decode-scalar` | DET.62 | `d()`: the snapshot `b`'s four bytes decode into one scalar; 16 reserved, 12 released |
//! | `decode-surrogate` | DET.62 | `d()`: an encoded surrogate stops the run at its decoding, in the effect phase |
//! | `child-decode-error-releases-reservation` | DET.60 | `spill()`: a child decodes `FF`; the parent's literal fits |
//! | `streaming` | DET.63 | `go()`: `s` streams five letters in chunks of two; the last chunk is refused on kernel cost |
//! | `streaming-complete-split` | DET.63 | `go()`: the same stream completes; `s`'s scope exit drops the cursor |
//! | `streaming-complete-coalesced` | DET.63 | the same program and run: a transport partition is not accounted |
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
//! `final-loop-test` runs a loop (DET.55; LP2 draft): `spin` repeats once a body that visits `acc`, the value so
//! far. The loop's tests are the frozen `loop/test:k` rows and the body's visit is the frozen body tick; the moves
//! of the initial value into the loop's slot and of the body's result back into it are visits of the program's
//! own, as is the loop's scope exit, which the stop at the final test never reaches.
//!
//! The generator traces seed in two ways. `generator` draws with `random_draw` from the run's own generator, which the
//! run starts at the trace's seed and path. The other three start generators of their own with `rng_init` (LP2
//! draft), whose stream path is the run's path followed by its segments: they run at the trace's seed and the empty
//! path, so that each `rng_init` names the trace's path in full, and the run's own start, `entry#rng_init` on the
//! empty path (49 bytes of key input), is overhead. `make`'s frame has no slot, as the trace's call has none, and its
//! return moves the generator to its caller. The child paths are named streams here; a child scope's own stream path
//! (DET.61) is `child_scopes.rs`'s.
//!
//! The child traces run `child` scopes (DET.60; LP2 draft) in `CHILD`. Each body is one operation, so its frame has
//! no slot and its `call` and `return` reserve and release the trace's 16 bytes; each stop branch names no status,
//! so after an unwind it costs its own frame only. A trace's tick is the body's literal, its `return` or the
//! parent's `let` binding; its `alloc` is a four-byte text literal, and `parent-drop` the scope exit that releases
//! it. A trace that completes with child records takes its limits at its first `child` row instead: the program's
//! charges before that row plus the trace's headroom there, so the root leaves the child what the trace's root
//! leaves it and each requested cap is cut as the trace's is. A dimension that the overhead after the child needs
//! more of is raised to the probe's use. Each child record is compared field by field, the trace's scope names
//! mapped to the program's locators through the `child` rows. In `dimension-before-scope`, `k`'s 10 bytes do not
//! fit in the root either, as in the trace, so the child's kernel-cost stop shows DET.56's dimension order before
//! the scope order.
//!
//! The decoding traces decode bytes the host delivers as the snapshot `b` (DET.62; LP2 draft): bytes have no
//! canonical form, so they cannot cross as an argument. The read is a row of the program's own, and `let` binds its
//! result, so the decoding borrows the bytes and its row releases only the reservation's excess, as the trace's does.
//! `decode-surrogate` stops on its value: its probe's host delivers three ASCII bytes, which read alike.
//!
//! `child-decode-error-releases-reservation`'s child decodes bytes the code holds with `text_decode_hex` (LP2 draft):
//! they take no row and no slot, so the body's frame has none and its rows are its call and its decoding, as the
//! trace's child's are. The decoding stops the child in the effect phase with its reservation admitted, the unwind
//! releases it in the child and the root, and the stop branch's frame and empty text are overhead. The trace's
//! `parent` allocation is a five-scalar text literal of 20 bytes, which fits only because the child left nothing live.
//!
//! The streaming traces stream a compressed word with `word_stream` (DET.63; LP2 draft), whose rows are the frozen
//! `open` and `chunk:<position>` rows. The literal word and its compression are the program's own, and `go` passes the
//! word and the chunk size to `s`: parameters are borrowed, so the stream releases no operand, and `s`'s scope exit,
//! which releases the cursor alone, is the frozen `close`. The run has no transport between the interpreter and a
//! kernel, so the split and the coalesced trace are one program and one run.
//!
//! # Not realised: 8 of 40
//!
//! [`NOT_EXPRESSIBLE`] lists each with its reason, and `every_frozen_trace_is_realised_or_listed_once` holds the two
//! lists to the 40 frozen vectors.

#[path = "../../c600-check/tests/support/json.rs"]
#[allow(dead_code)]
mod json;

use c600_check::check;
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink, SourceRanges};
use c600_run::meter::{code, frame_size, Amounts, Outcome, Row, SourceRange, Summary, Tariffs};
use c600_run::value::{equal, Value};
use c600_syntax::{lower, parse, sha256_hex, Lowered};
use json::Json;
use num_bigint::{BigInt, BigUint};

const VECTORS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/architecture/1.0/language/contract-vectors/vectors.json");

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The frozen traces no program of the IR draft can express, and why.
const NOT_EXPRESSIBLE: [(&str, &str); 8] = [
    (
        "generator-nested-same-name",
        "its call's frame has no slot, yet the callee binds `g` and `a` and releases them with the frame; the IR binds \
         a name with `let`, which takes a frame slot and releases at its scope's exit (DET.57)",
    ),
    ("text-comparison", "the IR has no text order"),
    (
        "abstract-trace-charges",
        "its body's `flow_check` belongs to the unselected flow-label alternative (DET.55; LQ21, RTE.24): the IR \
         has no flow check",
    ),
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

fn trace_stop(stop: Option<&Json>) -> Option<TraceStop> {
    match stop {
        Some(Json::Null) | None => None,
        Some(s) => Some(TraceStop {
            at: text(s, "at").into(),
            phase: text(s, "phase").into(),
            scope: text(s, "scope").into(),
            source: s
                .get("source")
                .map(|r| (text(r, "source").into(), uint(r, "start") as usize, uint(r, "end") as usize)),
        }),
    }
}

/// A frozen child record (DET.60).
struct TraceChild {
    at: String,
    requested: Amounts,
    effective: Amounts,
    outcome: String,
    code: Option<String>,
    stop: Option<TraceStop>,
    used: Amounts,
    returned_memory: u128,
    live_after_return: u128,
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
    children: Vec<TraceChild>,
}

impl<'a> Trace<'a> {
    fn read(vector: &'a Json) -> Trace<'a> {
        let given = vector.get("given").expect("given");
        let expected = vector.get("expected").expect("expected");
        let stop = trace_stop(expected.get("stop"));
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
        let children = expected.get("children").and_then(Json::arr).expect("child records");
        let children = children
            .iter()
            .map(|c| TraceChild {
                at: text(c, "at").into(),
                requested: amounts(c.get("requested").expect("requested caps")),
                effective: amounts(c.get("effective").expect("effective caps")),
                outcome: text(c, "outcome").into(),
                code: c.get("code").and_then(Json::str).map(str::to_string),
                stop: trace_stop(c.get("stop")),
                used: amounts(c.get("used").expect("a child's use")),
                returned_memory: uint(c, "returned_memory"),
                live_after_return: uint(c, "live_after_return"),
            })
            .collect();
        Trace {
            given,
            limits: amounts(given.get("limits").expect("limits")),
            outcome: text(expected, "outcome").into(),
            code: expected.get("code").and_then(Json::str).map(str::to_string),
            stop,
            used: amounts(expected.get("used").expect("used")),
            rows,
            values,
            children,
        }
    }

    /// The index of the trace's row `name`.
    fn row(&self, name: &str) -> usize {
        self.rows.iter().position(|(at, _)| at == name).unwrap_or_else(|| panic!("no frozen row `{name}`"))
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
    /// The tariff and the result, if the trace delivers one.
    k: Option<(RowTariff, Option<Value>)>,
    views: Vec<Vec<u8>>,
    /// The bytes of the snapshot `b`, if the trace decodes given bytes (DET.62).
    bytes: Option<Vec<u8>>,
}

impl TraceHost {
    fn for_trace(trace: &Trace) -> TraceHost {
        let k = trace.kernel().map(|k| {
            let result = k.get("result_bytes").map_or(0, |_| uint(k, "result_bytes"));
            // `dimension-before-scope`'s ten bytes are never delivered: the child's kernel cap refuses the row.
            let value = match result {
                0 => Some(Value::Unit),
                4 => Some(Value::record(vec![])),
                _ => None,
            };
            if let Some(value) = &value {
                assert_eq!(value.size(), result, "a result of the trace's size");
            }
            let tariff = RowTariff {
                kernel_cost: Tariffs::TEST.kernel(uint(k, "items")),
                result_bound: result + k.get("scratch").map_or(0, |_| uint(k, "scratch")),
            };
            (tariff, value)
        });
        let bytes = trace.first("text_decode").map(|d| hex(text(d, "hex")));
        TraceHost { k, views: Vec::new(), bytes }
    }

    /// The probe's host: a trace that stops on its decoded bytes probes with as many ASCII bytes, which read alike
    /// and decode (module docs).
    fn probing(mut self, trace: &Trace) -> TraceHost {
        if let (Some(bytes), Some(_)) = (&mut self.bytes, &trace.stop) {
            bytes.fill(b'a');
        }
        self
    }
}

impl Host for TraceHost {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match (request, &self.k, &self.bytes) {
            (Request::Snapshot { query: "k", .. }, Some((tariff, _)), _) => Ok(*tariff),
            (Request::Snapshot { query: "b", .. }, _, Some(bytes)) => {
                Ok(RowTariff { kernel_cost: 0, result_bound: bytes.len() as u128 })
            }
            _ => Err(code::NOT_GRANTED),
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match (request, &self.k, &self.bytes) {
            (Request::Snapshot { query: "k", .. }, Some((_, value)), _) => {
                Ok(value.clone().expect("a result the trace delivers"))
            }
            (Request::Snapshot { query: "b", .. }, _, Some(bytes)) => Ok(Value::Bytes(bytes.as_slice().into())),
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
        // A trace with child records: the trace's headroom at its first `child` row, raised to the probe's use.
        let first = trace.children.iter().map(|c| trace.row(&format!("{}/child", c.at))).min();
        if let Some(a) = first {
            let t = Before::of(trace.rows[..a].iter().map(|(_, c)| *c));
            let p = Before::of(probe.rows[..mapped[a]].iter().map(charges));
            return Amounts {
                steps: (p.steps + headroom(id, l.steps, t.steps)).max(probe.used.steps),
                kernel_cost: (p.kernel + headroom(id, l.kernel_cost, t.kernel)).max(probe.used.kernel_cost),
                memory: (p.live + headroom(id, l.memory, t.live)).max(probe.used.memory),
                output: (p.output + headroom(id, l.output, t.output)).max(probe.used.output),
            };
        }
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
    limits: Amounts,
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

    let probe = run_with(
        &p,
        r,
        r.probe_args.unwrap_or(r.args),
        &trace,
        WIDE,
        &mut TraceHost::for_trace(&trace).probing(&trace),
    );
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
    // Child records (DET.60): a trace's scope is named by its `child` row, a program's by that row's locator.
    let row = |name: &str| r.rows[mapped[trace.row(name)]].0;
    let scope = |name: &str| match name {
        "root" => "root",
        _ => row(&format!("{name}/child")).strip_suffix("#child").expect("a child row"),
    };
    assert_eq!(s.children.len(), trace.children.len(), "{id}: child records");
    for (got, want) in s.children.iter().zip(&trace.children) {
        let at = want.at.as_str();
        assert_eq!(got.at, scope(at), "{id}: child `{at}`");
        let caps = (got.requested, got.effective, got.used);
        assert_eq!(caps, (want.requested, want.effective, want.used), "{id}: child `{at}`'s caps and use");
        let outcome = (got.outcome.name(), got.code, got.returned_memory, got.live_after_return);
        let frozen = (want.outcome.as_str(), want.code.as_deref(), want.returned_memory, want.live_after_return);
        assert_eq!(outcome, frozen, "{id}: child `{at}`'s outcome and memory");
        match (&got.stop, &want.stop) {
            (None, None) => {}
            (Some(g), Some(w)) => {
                let frozen = (row(&w.at), w.phase.as_str(), scope(&w.scope));
                assert_eq!((g.at.as_str(), g.phase, g.scope.as_str()), frozen, "{id}: child `{at}`'s stop");
                assert_eq!(w.source, None, "{id}: a frozen child stop with a source association");
            }
            (g, w) => panic!("{id}: child `{at}` stops at {g:?}, the trace's at {:?}", w.as_ref().map(|w| &w.at)),
        }
    }
    assert_eq!(s.live_after_cleanup, 0, "{id}: cleanup releases everything");
    Realised { program: p, probe, limits, run: cut, run_host }
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

/// CT-DET-26 (part: frames and their unwind in nested calls on RT1; not run: RT2 and RT3 (no owner yet)), CT-DET-28
/// (synthetic, part: calls and returns as
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

/// CT-DET-27 (part: simultaneous failures and their precedence at one admission on RT1; not run: RT2 and RT3 (no owner
/// yet)): the frozen traces
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

/// CT-DET-29 (part: RT1 only; not run: RT2 and RT3 (no owner yet)): the frozen trace `generator` runs as a program
/// with the trace's seed and stream path,
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

/// CT-DET-29 (part: named streams, aliases and a returned owner on RT1; not run: RT2 and RT3 (no owner yet)): the
/// frozen traces `generator-alias-cursor`, `generator-returned-owner` and
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

/// CT-DET-28 (synthetic, part: oversized arithmetic on RT1; not run: text work and flow checks, since the IR has no
/// text order or flow check, and RT2 and RT3 (no owner yet)): the frozen traces `oversized-arithmetic`,
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

/// CT-DET-26 (part: alias and drop across calls on RT1; not run: RT2 and RT3 (no owner yet)): the frozen traces
/// `alias-release` and
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

/// CT-DET-28 (synthetic, part: assertions on RT1; not run: flow checks, since the IR has no flow check, and RT2 and RT3
/// (no owner yet)): the frozen trace `pointwise-failure` runs as a program, `check(n)`, which
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

/// `spin` repeats once a body that visits `acc` (DET.55; LP2 draft; module docs).
const SPIN: &str =
    "export fn spin() -> Nat effects [] {\n    return repeat(1, 0, fn(acc: Nat) -> Nat { return acc; });\n}\n";

/// CT-DET-28 (synthetic, part: loop tests on RT1; not run: flow checks, since the IR has no flow check, and RT2 and
/// RT3 (no owner yet)): the frozen trace `final-loop-test` runs as a program, `spin()`, whose loop repeats once a body
/// that visits `acc` (DET.55; LP2 draft). Each loop test is a one-step row of its own, the final false one included:
/// at the trace's headroom that test is refused on steps with the frozen denied row, and the cleanup releases the
/// loop's slot and its count.
#[test]
fn ct_det_28_the_final_loop_test_runs_as_a_program() {
    realise(
        &fixtures(),
        &Realisation {
            trace: "final-loop-test",
            requires: "",
            declarations: SPIN,
            entry: "spin",
            args: NO_ARGS,
            probe_args: None,
            rows: &[
                ("entry#call", None),
                ("m.spin:0#value", None),
                ("m.spin:1#value", None),
                ("m.spin:root#visit", None),
                ("m.spin:root#loop_test:0", Some("loop/test:0")),
                ("m.spin:root#body:0 > m.spin:2#visit", Some("loop/body:0/body")),
                ("m.spin:root#body:0 > m.spin:root#visit", None),
                ("m.spin:root#loop_test:1", Some("loop/test:1")),
                ("m.spin:root#scope_exit", None),
                ("m.spin:root#return", None),
                ("entry#emit", None),
            ],
            stop_would: Some([1, 0, 0, 0]),
            kernel_extra: 0,
            source_node: None,
        },
    );
}

/// The child traces' programs (DET.60; LP2 draft; module docs): each body is one operation, and each stop branch
/// names no status.
const CHILD: &str = "export fn allow() -> Unit effects [budget] {\n    \
                     let u = child(record { steps = 2, kernel_cost = 99, memory = 99, output = 99 }, \
                     fn() -> Unit { return (); }, fn() -> Unit { return (); });\n    return ();\n}\n\n\
                     export fn owned() -> Unit effects [budget] {\n    \
                     let r = child(record { steps = 5, kernel_cost = 5, memory = 24, output = 5 }, \
                     fn() -> Text { return \"x\"; }, fn() -> Text { return \"y\"; });\n    return ();\n}\n\n\
                     export fn early() -> Unit effects [budget] {\n    \
                     let u = child(record { steps = 10, kernel_cost = 10, memory = 40, output = 10 }, \
                     fn() -> Unit { return (); }, fn() -> Unit { return (); });\n    return ();\n}\n\n\
                     export fn dim() -> Unit effects [budget, snapshot] {\n    \
                     let u = child(record { steps = 8, kernel_cost = 2, memory = 20, output = 10 }, \
                     fn() -> Unit { return read_snapshot<Unit>(\"k\"); }, fn() -> Unit { return (); });\n    \
                     return ();\n}\n\n\
                     export fn nest() -> Unit effects [budget] {\n    \
                     let u = child(record { steps = 10, kernel_cost = 10, memory = 60, output = 0 }, \
                     fn() -> Unit {\n        \
                     return child(record { steps = 4, kernel_cost = 4, memory = 20, output = 0 }, \
                     fn() -> Unit { return (); }, fn() -> Unit { return (); });\n    \
                     }, fn() -> Unit { return (); });\n    \
                     return ();\n}\n";

const ALLOW: Realisation = Realisation {
    trace: "child-allowances",
    requires: "",
    declarations: CHILD,
    entry: "allow",
    args: NO_ARGS,
    probe_args: None,
    rows: &[
        ("entry#call", None),
        ("m.allow:0#child", Some("c/child")),
        ("m.allow:0#call", Some("c/run/call")),
        ("m.allow:0 > m.allow:0.0#value", Some("c/run/a")),
        ("m.allow:0 > m.allow:0.0#return", Some("c/run/b")),
        ("m.allow:0#call", None),
        ("m.allow:0 > m.allow:0.1#value", None),
        ("m.allow:0 > m.allow:0.1#return", None),
        ("m.allow:root#visit", Some("parent")),
        ("m.allow:1#value", None),
        ("m.allow:root#scope_exit", None),
        ("m.allow:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: None,
    kernel_extra: 0,
    source_node: None,
};

const OWNED: Realisation = Realisation {
    trace: "child-return-owner",
    entry: "owned",
    rows: &[
        ("entry#call", None),
        ("m.owned:0#child", Some("c/child")),
        ("m.owned:0#call", Some("c/run/call")),
        ("m.owned:0 > m.owned:0.0#value", Some("c/run/a")),
        ("m.owned:0 > m.owned:0.0#return", Some("c/run/return")),
        ("m.owned:root#visit", None),
        ("m.owned:1#value", None),
        ("m.owned:root#scope_exit", Some("parent-drop")),
        ("m.owned:root#return", None),
        ("entry#emit", None),
    ],
    ..ALLOW
};

const EARLY: Realisation = Realisation {
    trace: "root-before-child",
    entry: "early",
    rows: &[
        ("entry#call", None),
        ("m.early:0#child", Some("c/child")),
        ("m.early:0#call", Some("c/run/call")),
        ("m.early:0 > m.early:0.0#value", Some("c/run/a")),
        ("m.early:0 > m.early:0.0#return", Some("c/run/b")),
        ("m.early:root#visit", None),
        ("m.early:1#value", None),
        ("m.early:root#scope_exit", None),
        ("m.early:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: Some([1, 0, 0, 0]),
    ..ALLOW
};

const DIM: Realisation = Realisation {
    trace: "dimension-before-scope",
    entry: "dim",
    rows: &[
        ("entry#call", None),
        ("m.dim:0#child", Some("c/child")),
        ("m.dim:0#call", Some("c/run/call")),
        ("m.dim:0 > m.dim:0.0#kernel", Some("c/run/k")),
        ("m.dim:0#call", None),
        ("m.dim:0 > m.dim:0.1#value", None),
        ("m.dim:0 > m.dim:0.1#return", None),
        ("m.dim:root#visit", Some("parent")),
        ("m.dim:1#value", None),
        ("m.dim:root#scope_exit", None),
        ("m.dim:root#return", None),
        ("entry#emit", None),
    ],
    ..ALLOW
};

const NEST: Realisation = Realisation {
    trace: "nested-child-ledger",
    entry: "nest",
    rows: &[
        ("entry#call", None),
        ("m.nest:0#child", Some("outer/child")),
        ("m.nest:0#call", Some("outer/run/call")),
        ("m.nest:0 > m.nest:0.0#child", Some("outer/run/inner/child")),
        ("m.nest:0 > m.nest:0.0#call", Some("outer/run/inner/run/call")),
        ("m.nest:0 > m.nest:0.0 > m.nest:0.0.0#value", Some("outer/run/inner/run/work")),
        ("m.nest:0 > m.nest:0.0 > m.nest:0.0.0#return", Some("outer/run/inner/run/return")),
        ("m.nest:0 > m.nest:0.0#return", Some("outer/run/return")),
        ("m.nest:root#visit", Some("parent")),
        ("m.nest:1#value", None),
        ("m.nest:root#scope_exit", None),
        ("m.nest:root#return", None),
        ("entry#emit", None),
    ],
    ..ALLOW
};

/// CT-DET-26 (part: a moved child result on RT1; not run: RT2 and RT3 (no owner yet)): the frozen trace
/// `child-return-owner` runs as a program (LP2 draft; module docs). The child's body allocates a four-byte text and
/// returns it: its owner moves to the parent, so the child returns four bytes and keeps nothing live, and the parent's
/// scope exit releases them as the trace's `parent-drop` does.
#[test]
fn ct_det_26_a_moved_child_result_runs_as_a_program() {
    realise(&fixtures(), &OWNED);
}

/// CT-DET-27 (part: child/root precedence on RT1; not run: RT2 and RT3 (no owner yet)): the frozen traces
/// `child-allowances`, `root-before-child`, `dimension-before-scope` and `nested-child-ledger` run as programs (LP2
/// draft; module docs). Each child's effective caps are its requested caps cut to what every ancestor has left after
/// the `child` step. When the root's steps run out with the child's, the root stops the whole run; when the child's
/// kernel cap and the root's memory fail at one admission, kernel cost comes first and the child stops alone; nested
/// children each keep a ledger of their own.
#[test]
fn ct_det_27_child_scopes_run_as_programs() {
    let root = fixtures();
    for r in [&ALLOW, &EARLY, &NEST] {
        realise(&root, r);
    }
    let done = realise(&root, &DIM);
    // The root lacks memory at `k` as the trace's root does, so the stop's scope shows the dimension order first.
    let trace = Trace::read(vector(&root, DIM.trace));
    let rows = &done.run.summary.rows;
    let at = rows.iter().position(|r| r.at == "m.dim:0 > m.dim:0.0#kernel").expect("the row k");
    let left = done.limits.memory - Before::of(rows[..at].iter().map(charges)).live;
    let frozen = trace.limits.memory - Before::of(trace.rows[..trace.row("c/run/k")].iter().map(|(_, c)| *c)).live;
    assert_eq!(left, frozen, "the root's free memory at k is the trace's");
    assert!(left < uint(trace.kernel().expect("the row k"), "result_bytes"), "the root lacks memory at k");
}

/// The decoding traces' program (DET.62; LP2 draft). Bytes have no canonical form, so they cannot cross as an
/// argument: the host delivers the trace's bytes as the snapshot `b`, a row of the program's own, and `let` binds
/// them, so the decoding borrows them and its row releases the reservation's excess alone.
const DECODE: &str = "export fn d() -> Text effects [snapshot] {\n    let b = read_snapshot<Bytes>(\"b\");\n    \
                      return text_decode(b);\n}\n";

const SCALAR: Realisation = Realisation {
    trace: "decode-scalar",
    requires: "",
    declarations: DECODE,
    entry: "d",
    args: NO_ARGS,
    probe_args: None,
    rows: &[
        ("entry#call", None),
        ("m.d:0#kernel", None),
        ("m.d:root#visit", None),
        ("m.d:1.0#visit", None),
        ("m.d:1#text_decode", Some("decode")),
        ("m.d:root#scope_exit", None),
        ("m.d:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: None,
    kernel_extra: 0,
    source_node: None,
};

/// The surrogate stops on its value: the probe decodes three ASCII bytes, which read alike (module docs).
const SURROGATE: Realisation = Realisation { trace: "decode-surrogate", stop_would: Some([1, 3, 12, 0]), ..SCALAR };

/// CT-DET-28 (part: captured bytes decode in one row on RT1; not run: RT2 and RT3 (no owner yet)): the frozen traces
/// `decode-scalar` and `decode-surrogate` run as programs (DET.62; LP2 draft). The row is one step with the input-byte
/// tariff on each byte and a reservation of 4 bytes per byte, settled to the text's 4 bytes per scalar; an encoded
/// surrogate stops the run in the effect phase at that row, with the reservation admitted and nothing bound.
#[test]
fn ct_det_28_byte_decoding_runs_as_programs() {
    let root = fixtures();
    let done = realise(&root, &SCALAR);
    let trace = Trace::read(vector(&root, SCALAR.trace));
    let want = trace.values.iter().find(|(k, _)| k == "decode").and_then(|(_, v)| v.str()).expect("the text");
    assert!(equal(done.run.value.as_ref().expect("the decoded text"), &Value::text(want)));
    let done = realise(&root, &SURROGATE);
    assert!(done.run.value.is_none(), "no value is bound");
}

/// `child-decode-error-releases-reservation`'s program (DET.60, DET.62; LP2 draft; module docs): the child's body
/// decodes bytes the code holds, and the parent then allocates a 20-byte text.
const SPILL: &str = "export fn spill() -> Unit effects [budget] {\n    \
                     let t = child(record { steps = 20, kernel_cost = 20, memory = 20, output = 0 }, \
                     fn() -> Text { return text_decode_hex(\"ff\"); }, fn() -> Text { return \"\"; });\n    \
                     let p = \"abcde\";\n    return ();\n}\n";

const RESERVATION: Realisation = Realisation {
    trace: "child-decode-error-releases-reservation",
    requires: "",
    declarations: SPILL,
    entry: "spill",
    args: NO_ARGS,
    probe_args: None,
    rows: &[
        ("entry#call", None),
        ("m.spill:0#child", Some("c/child")),
        ("m.spill:0#call", Some("c/run/call")),
        ("m.spill:0 > m.spill:0.0#text_decode", Some("c/run/decode")),
        ("m.spill:0#call", None),
        ("m.spill:0 > m.spill:0.1#value", None),
        ("m.spill:0 > m.spill:0.1#return", None),
        ("m.spill:root#visit", None),
        ("m.spill:1.0#value", Some("parent")),
        ("m.spill:1#visit", None),
        ("m.spill:1.1#value", None),
        ("m.spill:1#scope_exit", None),
        ("m.spill:root#scope_exit", None),
        ("m.spill:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: None,
    kernel_extra: 0,
    source_node: None,
};

/// CT-DET-28 (part: a decoding error in a child on RT1; not run: RT2 and RT3 (no owner yet)): the frozen trace
/// `child-decode-error-releases-reservation` runs as a program (DET.58, DET.60; LP2 draft; module docs). The child's
/// decoding of `FF` is admitted with its four-byte reservation and stops the child alone; the unwind releases the
/// reservation in the child and in the root, so the child returns nothing and keeps nothing live, and the root's
/// 20-byte literal fits in the memory the trace's root has there.
#[test]
fn ct_det_28_a_child_decoding_error_releases_its_reservation() {
    let root = fixtures();
    let done = realise(&root, &RESERVATION);
    let trace = Trace::read(vector(&root, RESERVATION.trace));
    let rows = &done.run.summary.rows;
    let at = rows.iter().position(|r| r.at == "m.spill:1.0#value").expect("the row parent");
    let left = done.limits.memory - Before::of(rows[..at].iter().map(charges)).live;
    let frozen = trace.limits.memory - Before::of(trace.rows[..trace.row("parent")].iter().map(|(_, c)| *c)).live;
    assert_eq!(left, frozen, "the root's free memory at parent is the trace's");
}

/// The streaming traces' program (DET.63; LP2 draft). `go` compresses the trace's five letters and passes the word
/// and the chunk size to `s`, which streams the word into `cursor` and returns: parameters are borrowed, so the
/// stream releases no operand, and `s`'s scope exit releases the cursor alone, the frozen `close`.
const STREAM: &str = "fn s(c: CompressedWord, k: Nat) -> Unit effects [] {\n    let cursor = word_stream(c, k);\n    \
                      return ();\n}\n\n\
                      export fn go() -> Unit effects [] {\n    let c = compress(word[+1, -2, +3, -4, +5]);\n    \
                      return s(c, 2);\n}\n";

const STREAM_SPLIT: Realisation = Realisation {
    trace: "streaming-complete-split",
    requires: "",
    declarations: STREAM,
    entry: "go",
    args: NO_ARGS,
    probe_args: None,
    rows: &[
        ("entry#call", None),
        ("m.go:0.0#value", None),
        ("m.go:0#compress", None),
        ("m.go:root#visit", None),
        ("m.go:1.0#visit", None),
        ("m.go:1.1#value", None),
        ("m.go:1#call", None),
        ("m.go:1 > m.s:0.0#visit", None),
        ("m.go:1 > m.s:0.1#visit", None),
        ("m.go:1 > m.s:0#word_open", Some("word/open")),
        ("m.go:1 > m.s:0#chunk:0", Some("word/chunk:0")),
        ("m.go:1 > m.s:0#chunk:2", Some("word/chunk:2")),
        ("m.go:1 > m.s:0#chunk:4", Some("word/chunk:4")),
        ("m.go:1 > m.s:root#visit", None),
        ("m.go:1 > m.s:1#value", None),
        ("m.go:1 > m.s:root#scope_exit", Some("close")),
        ("m.go:1 > m.s:root#return", None),
        ("m.go:root#scope_exit", None),
        ("m.go:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: None,
    kernel_extra: 0,
    source_node: None,
};

/// The same program: a transport partition has no place in a run's accounting (DET.63).
const STREAM_COALESCED: Realisation = Realisation { trace: "streaming-complete-coalesced", ..STREAM_SPLIT };

/// The same program, cut off on kernel cost before its last chunk expands a letter: the trace ends there, so its
/// probe's scope exit is overhead.
const STREAM_CUT: Realisation = Realisation {
    trace: "streaming",
    rows: &[
        ("entry#call", None),
        ("m.go:0.0#value", None),
        ("m.go:0#compress", None),
        ("m.go:root#visit", None),
        ("m.go:1.0#visit", None),
        ("m.go:1.1#value", None),
        ("m.go:1#call", None),
        ("m.go:1 > m.s:0.0#visit", None),
        ("m.go:1 > m.s:0.1#visit", None),
        ("m.go:1 > m.s:0#word_open", Some("word/open")),
        ("m.go:1 > m.s:0#chunk:0", Some("word/chunk:0")),
        ("m.go:1 > m.s:0#chunk:2", Some("word/chunk:2")),
        ("m.go:1 > m.s:0#chunk:4", Some("word/chunk:4")),
        ("m.go:1 > m.s:root#visit", None),
        ("m.go:1 > m.s:1#value", None),
        ("m.go:1 > m.s:root#scope_exit", None),
        ("m.go:1 > m.s:root#return", None),
        ("m.go:root#scope_exit", None),
        ("m.go:root#return", None),
        ("entry#emit", None),
    ],
    stop_would: Some([1, 3, 2, 0]),
    ..STREAM_SPLIT
};

/// CT-DET-30 (part: RT1's runs of the frozen streaming traces; not run: transport splits and cold caches, which RT1's
/// in-process stream does not have (no owner yet)): the frozen traces `streaming`, `streaming-complete-split` and
/// `streaming-complete-coalesced` run as programs (DET.63; LP2 draft). The two partitions are one program and one run:
/// the chunk schedule comes from the word's length and the declared chunk size alone. The cut-off run stops at the
/// chunk at position 4 with the cursor live and four letters covered, and its cleanup releases the cursor.
#[test]
fn ct_det_30_streaming_runs_as_programs() {
    let root = fixtures();
    let split = realise(&root, &STREAM_SPLIT);
    let coalesced = realise(&root, &STREAM_COALESCED);
    assert_eq!(split.run.summary.rows, coalesced.run.summary.rows, "a transport partition changes no row");
    let cut = realise(&root, &STREAM_CUT);
    let trace = vector(&root, STREAM_CUT.trace).get("expected").expect("expected");
    let s = &cut.run.summary;
    // The live charge at the stop beyond the program's own before the open is the trace's: the cursor alone.
    let open = s.rows.iter().position(|r| r.at.ends_with("#word_open")).expect("the open row");
    let overhead = Before::of(s.rows[..open].iter().map(charges)).live;
    assert_eq!(s.live_before_cleanup - overhead, uint(trace, "live_before_cleanup"), "the cursor is live at the stop");
    let covered: Vec<&str> = s.rows.iter().filter_map(|r| r.at.strip_prefix("m.go:1 > m.s:0#chunk:")).collect();
    assert_eq!(covered, ["0", "2", "4"], "chunks before the stop and the stopped one");
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
        "final-loop-test",
        "child-allowances",
        "child-return-owner",
        "root-before-child",
        "dimension-before-scope",
        "nested-child-ledger",
        "decode-scalar",
        "decode-surrogate",
        "child-decode-error-releases-reservation",
        "streaming",
        "streaming-complete-split",
        "streaming-complete-coalesced",
    ];
    let mut ids: Vec<&str> = all(&root).iter().map(|v| text(v, "id")).collect();
    let mut accounted: Vec<&str> = realised.iter().copied().chain(NOT_EXPRESSIBLE.iter().map(|n| n.0)).collect();
    ids.sort_unstable();
    accounted.sort_unstable();
    assert_eq!(ids.len(), 40);
    assert_eq!(accounted, ids, "every frozen trace is realised or listed with its reason, exactly once");
}
