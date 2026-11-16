//! CT-RTE-04, in part: "one frozen abstract trace preserves exact results, charges, stopping points and source
//! associations across admitted executors". The executors here are the interpreter in process and the runtime in its
//! own process (route RT1); the traces are DET.65's frozen vectors. Source and synthetic evidence only: no timing is
//! measured, and no instruction count takes part.
//!
//! # The logical mapping
//!
//! A frozen trace is abstract: named operations with given charges. A realisation is a `.6cs` program and a table that
//! maps every trace row, in order, onto one row of the program's run. A mapped row must carry exactly the trace row's
//! charges (steps, kernel cost, memory, output, released). The program's other rows are the realisation's own
//! overhead: reading the arguments, the entry call and its frame, a `display_if` condition, binding results to names,
//! returns, and encoding and emitting the entry's result.
//!
//! The limits give every mapped row the trace's headroom. A probe run with wide limits measures the overhead. For a
//! trace that completes, each limit is the program's use plus the trace's slack (limit minus use; memory's use is the
//! peak). For a trace that stops at a row, each cumulative limit is the program's charges before the mapped row plus
//! the trace's headroom before its row; the memory limit is the program's live memory before that row plus the trace's
//! memory headroom, raised to the program's earlier peak unless the trace stops on memory (a realisation that would
//! need the raise there is refused). The stopped operation, admitted in the probe, must charge what the trace's
//! operation would (`stop_would`), so the same headroom stops it in the same phase.
//!
//! Both executors then run each job with those limits and must agree on everything: summary, rows, stop, output, value
//! and every host call in order. The run must also map onto the trace: outcome and code, the mapped rows' charges, the
//! stop at the mapped row with the trace's phase and scope, the source association, and the trace's values.
//!
//! # Realised: 8 of 40
//!
//! | Trace | Clause | Realisation |
//! | --- | --- | --- |
//! | `approx-branch-true` | DET.64 | `pick(1, 2, r)`: `display_if` takes `r.v` (two visits) |
//! | `approx-branch-false` | DET.64 | `pick(5, 2, r)`: `display_if` takes `x` (one visit) |
//! | `display-stop-source-range` | DET.56 | `pick(1, 2, r)` stopped on steps at `r.v`, after a non-BMP character |
//! | `step-precedence` | DET.56 | `show()`: the encoding pass of `publish_view(record { })`, refused on steps |
//! | `simultaneous-exhaustion` | DET.56 | the same pass, steps admitted, refused on kernel cost |
//! | `simultaneous-memory-output` | DET.56 | the same pass, kernel cost admitted at its limit, refused on memory |
//! | `equal-to-limit` | DET.56 | `limit(7)`: a host read priced as the trace's kernel row, released at scope exit |
//! | `generator` | DET.61 | `draws()` from the trace's seed and path: two draws, published, then a refused draw |
//!
//! The trace's `display_if` condition is a given value that holds no memory. `pick` binds its comparison with `let`,
//! so the `display_if` row visits a borrowed condition and releases nothing; the comparison's byte is released at the
//! scope's exit, as overhead. In `display-stop-source-range` the trace's untaken branch is empty; the program's
//! untaken branch is never observed.
//!
//! The abstract `emit` of `"x"` charges its 3-byte encoding to kernel cost, memory and output in one row (DET.59).
//! The interpreter charges an output unit in two rows: the encoding pass (kernel cost and memory), then the
//! publication (output). The three DET.56 realisations stop on the encoding pass, which would charge what the trace's
//! `emit` would to steps, kernel cost and memory, and no output. They exercise steps before kernel cost and memory,
//! and kernel cost before memory, but no dimension's precedence over output: `simultaneous-memory-output` stops on
//! memory with the trace's rows, code and phase, while its output headroom is never tested on that row.
//!
//! The trace's generator key and counter are not observable through a run; the drawn values are, through the
//! published view.
//!
//! # Not realised: 32 of 40
//!
//! [`UNREALISED`] lists each with its reason; the test holds the two lists to the 40 frozen vectors. In short: the IR
//! has no child scope (6 traces), loop, pointwise assertion, exact multiplication, text order, byte decoding or
//! streaming cursor; `let` scopes nest; lowered frames have a slot more than the trace's; a run has one generator,
//! seeded at the entry; the interpreter prices an output unit in two rows; and the region contract
//! `lp1-exec-region-draft-1` has no executor yet.

mod support;

#[path = "../../c600-check/tests/support/json.rs"]
#[allow(dead_code)]
mod json;

use c600_digest::sha256_hex;
use c600_ir::Capability;
use c600_run::interp::RowTariff;
use c600_run::meter::{Amounts, Outcome, Row, Summary, Tariffs};
use c600_run::value::Value;
use c600_runtime::{run_in_process, Ended, JobOptions, Runtime};
use json::Json;
use num_bigint::BigInt;
use support::{compare, header, job, ran, runtime, Session, WIDE};

const VECTORS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/architecture/1.0/language/contract-vectors/vectors.json");

/// The realisations' module. The comment's character outside the basic plane precedes every node, so a node's byte
/// offset and its character offset differ, as the emoji does in `display-stop-source-range`'s source text.
const PROGRAM: &str = "# Realisations of frozen DET.65 traces. \u{1f600} parts byte and character offsets.\n\
    export fn pick(x: Nat, y: Nat, r: record { v: Nat }) -> Display<Nat> effects [] {\n    \
    let c = approx_less(approx53(x), approx53(y));\n    return display_if(c, r.v, x);\n}\n\n\
    export fn show() -> Unit effects [view] {\n    return publish_view(record { });\n}\n\n\
    fn hold() -> Unit effects [snapshot] {\n    let x = read_snapshot<record { }>(\"k\");\n    return ();\n}\n\n\
    export fn limit(y: Nat) -> Nat effects [snapshot] {\n    let u = hold();\n    return y;\n}\n\n\
    export fn draws() -> Nat effects [random, view] {\n    let a = random_draw();\n    let b = random_draw();\n    \
    let v = publish_view(record { a = a, b = b });\n    let c = random_draw();\n    return a;\n}\n";

/// The frozen traces this file does not realise, and why.
const UNREALISED: [(&str, &str); 32] = [
    ("alias-release", "`let` scopes nest: `a` outlives its alias `b`, so it cannot be released first"),
    ("alias-still-live", "`let` scopes nest: `a` outlives its alias `b`, so it cannot be released first"),
    ("nested-calls", "every lowered frame has at least one slot (24 bytes); the trace's frames have none (16)"),
    ("child-allowances", "the IR has no child scope (DET.60 runs on the meter's traces only)"),
    ("child-return-owner", "the IR has no child scope"),
    ("root-before-child", "the IR has no child scope"),
    ("dimension-before-scope", "the IR has no child scope"),
    ("nested-child-ledger", "the IR has no child scope"),
    ("child-decode-error-releases-reservation", "the IR has no child scope and no byte decoding"),
    ("generator-child-paths", "a run has one generator, seeded at the entry; the IR cannot start another"),
    ("generator-alias-cursor", "the generator is not a value: the IR cannot alias it"),
    ("generator-returned-owner", "the generator is seeded before the entry call, never inside a call"),
    ("generator-nested-same-name", "a run has one generator; the IR cannot start a second in a call"),
    ("text-comparison", "the IR has no text order"),
    ("decode-scalar", "the IR has no byte decoding: arguments and host results cross as values"),
    ("decode-surrogate", "the IR has no byte decoding: arguments and host results cross as values"),
    ("abstract-trace-charges", "the IR has no loop, pointwise assertion or flow check"),
    ("final-loop-test", "the IR has no loop; recursion adds call frames the trace's loop does not have"),
    ("pointwise-failure", "the IR has no pointwise assertion"),
    ("oversized-arithmetic", "the IR has no exact multiplication"),
    ("oversized-admitted", "the IR has no exact multiplication"),
    ("arithmetic-reservation", "the IR has no exact multiplication"),
    ("streaming", "the interpreter has no streaming cursor: a word crosses whole"),
    ("streaming-complete-split", "the interpreter has no streaming cursor: a word crosses whole"),
    ("streaming-complete-coalesced", "the interpreter has no streaming cursor: a word crosses whole"),
    ("output-retained-at-cutoff", "an output unit takes two rows (`encode`, then `emit` or `publish_view`)"),
    ("output-equality", "an output unit takes two rows (`encode`, then `emit` or `publish_view`)"),
    ("form2-equal-limit", "execution contract `lp1-exec-region-draft-1` has no executor yet"),
    ("form2-one-short", "execution contract `lp1-exec-region-draft-1` has no executor yet"),
    ("form2-untaken-branch", "execution contract `lp1-exec-region-draft-1` has no executor yet"),
    ("form2-publication-equality", "execution contract `lp1-exec-region-draft-1` has no executor yet"),
    ("form2-publication-output-cutoff", "execution contract `lp1-exec-region-draft-1` has no executor yet"),
];

struct Realisation {
    trace: &'static str,
    entry: &'static str,
    args: fn() -> Vec<Value>,
    /// The job options for the given limits, from the trace's `given`.
    options: fn(&Json, Amounts) -> JobOptions,
    /// Prepares the caller's session from the trace's `given`.
    setup: fn(&Json, &mut Session),
    /// Every trace row's `at`, in order, and the program row it maps to.
    map: &'static [(&'static str, &'static str)],
    /// For a stopping trace: steps, kernel cost and memory its stopped operation would charge if admitted.
    stop_would: Option<[u128; 3]>,
    /// The program text of the node that a stop's source range must cover.
    source_node: Option<&'static str>,
    /// Holds the trace's expected values to what the run delivered.
    values: fn(&[(String, Json)], &Session),
}

fn plain(_: &Json, limits: Amounts) -> JobOptions {
    JobOptions::new(limits)
}

fn untouched(_: &Json, _: &mut Session) {}

fn no_values(values: &[(String, Json)], _: &Session) {
    assert!(values.is_empty(), "a realisation without an observation for the trace's values: {values:?}");
}

fn pick_args(x: u32, y: u32) -> Vec<Value> {
    vec![Value::int(x), Value::int(y), Value::record(vec![("v", Value::int(3))])]
}

/// The trace's first operation.
fn first_op(given: &Json) -> &Json {
    given.get("trace").and_then(Json::arr).and_then(|t| t.first()).expect("a trace with an operation")
}

/// The job's seed and path are the trace's `rng_init`.
fn seeded(given: &Json, limits: Amounts) -> JobOptions {
    let init = first_op(given);
    assert_eq!(text(init, "op"), "rng_init");
    let seed = hex(text(init, "seed"));
    let path = init.get("path").and_then(Json::arr).expect("a path").iter().map(|p| p.str().unwrap().to_string());
    JobOptions { seed: Some((seed, path.collect())), grant: vec![Capability::Randomness], ..JobOptions::new(limits) }
}

/// The snapshot `k` is priced as the trace's kernel row: the batch's kernel cost, a reservation of its result and
/// scratch bytes, and a result of exactly the trace's result size.
fn kernel_snapshot(given: &Json, session: &mut Session) {
    let k = first_op(given);
    assert_eq!(text(k, "op"), "kernel");
    let result = Value::record(vec![]);
    assert_eq!(result.size(), uint(k, "result_bytes"), "an empty record has the trace's result size");
    let tariff = RowTariff {
        kernel_cost: Tariffs::TEST.kernel(uint(k, "items")),
        result_bound: uint(k, "result_bytes") + uint(k, "scratch"),
    };
    session.snapshots.push(("k", tariff, result));
}

/// `draws` publishes `record { a = draw-0, b = draw-1 }` before its refused draw.
fn published_draws(values: &[(String, Json)], session: &Session) {
    assert_eq!(values.len(), 2);
    let draw = |name: &str| {
        let hex = values.iter().find(|(k, _)| k == name).and_then(|(_, v)| v.str()).expect("a drawn block");
        Value::int(BigInt::parse_bytes(hex.as_bytes(), 16).expect("hex"))
    };
    let mut bytes = Vec::new();
    c600_run::encode::encode(&Value::record(vec![("a", draw("draw-0")), ("b", draw("draw-1"))]), &mut bytes)
        .expect("a canonical record");
    assert_eq!(session.views, vec![bytes], "the drawn values are the trace's");
}

const PICK_TRUE: &[(&str, &str)] = &[
    ("display", "m.pick:1#display_if"),
    ("display/then/a", "m.pick:1.1.0#visit"),
    ("display/then/b", "m.pick:1.1#visit"),
];
const PICK_FALSE: &[(&str, &str)] = &[("display", "m.pick:1#display_if"), ("display/else/a", "m.pick:1.2#visit")];
const SHOW: &[(&str, &str)] = &[("emit", "m.show:root#publish_view")];

fn realisations() -> Vec<Realisation> {
    let base = Realisation {
        trace: "",
        entry: "pick",
        args: || pick_args(1, 2),
        options: plain,
        setup: untouched,
        map: PICK_TRUE,
        stop_would: None,
        source_node: None,
        values: no_values,
    };
    let show =
        |trace| Realisation { trace, entry: "show", args: Vec::new, map: SHOW, stop_would: Some([1, 3, 3]), ..base };
    vec![
        Realisation { trace: "approx-branch-true", ..base },
        Realisation { trace: "approx-branch-false", args: || pick_args(5, 2), map: PICK_FALSE, ..base },
        Realisation {
            trace: "display-stop-source-range",
            stop_would: Some([1, 0, 0]),
            source_node: Some("r.v"),
            ..base
        },
        show("step-precedence"),
        show("simultaneous-exhaustion"),
        show("simultaneous-memory-output"),
        Realisation {
            trace: "equal-to-limit",
            entry: "limit",
            args: || vec![Value::int(7)],
            setup: kernel_snapshot,
            map: &[
                ("k", "m.limit:0 > m.hold:0#kernel"),
                ("drop", "m.limit:0 > m.hold:root#scope_exit"),
                ("last", "m.limit:1#visit"),
            ],
            ..base
        },
        Realisation {
            trace: "generator",
            entry: "draws",
            args: Vec::new,
            options: seeded,
            map: &[
                ("init", "entry#rng_init"),
                ("draw-0", "m.draws:0#rng_draw"),
                ("draw-1", "m.draws:1.0#rng_draw"),
                ("denied", "m.draws:1.1.1.0#rng_draw"),
            ],
            stop_would: Some([1, 3, 33]),
            values: published_draws,
            ..base
        },
    ]
}

// ---------------------------------------------------------------------------------------------------------------
// The frozen vectors

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

struct TraceStop {
    at: String,
    phase: String,
    scope: String,
    /// The source digest and byte interval.
    source: Option<(String, usize, usize)>,
}

struct Trace {
    limits: Amounts,
    outcome: String,
    code: Option<String>,
    stop: Option<TraceStop>,
    used: Amounts,
    rows: Vec<(String, [u128; 5])>,
    values: Vec<(String, Json)>,
    source_text: Option<String>,
}

impl Trace {
    fn read(vector: &Json) -> Trace {
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
            limits: amounts(given.get("limits").expect("limits")),
            outcome: text(expected, "outcome").into(),
            code: expected.get("code").and_then(Json::str).map(str::to_string),
            stop,
            used: amounts(expected.get("used").expect("used")),
            rows,
            values,
            source_text: given.get("source_text").and_then(Json::str).map(str::to_string),
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The mapping

fn charges(row: &Row) -> [u128; 5] {
    [row.steps, row.kernel_cost, row.memory, row.output, row.released]
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

/// The realisation's limits: the trace's headroom on every mapped row (module docs).
fn limits(id: &str, trace: &Trace, probe: &Summary, mapped: &[usize]) -> Amounts {
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
        kernel_cost: p.kernel + headroom(id, trace.limits.kernel_cost, t.kernel),
        memory,
        output: p.output + headroom(id, trace.limits.output, t.output),
    }
}

fn realise(rt: &mut Runtime, source: &str, r: &Realisation, vector: &Json) {
    let id = r.trace;
    let given = vector.get("given").expect("given");
    let trace = Trace::read(vector);
    let job_with = |limits| job(source, r.entry, (r.args)(), (r.options)(given, limits));
    let setup = |s: &mut Session| (r.setup)(given, s);

    // The probe: wide limits, in process.
    let mut session = Session::default();
    setup(&mut session);
    let probe = match run_in_process(&job_with(WIDE), &mut session, None) {
        Ended::Ran(run) => run.summary,
        Ended::Refused(refused) => panic!("{id}: {refused:#?}"),
    };
    let listing = probe.rows.iter().map(|row| format!("\n  {} {:?}", row.at, charges(row))).collect::<String>();
    assert_eq!(probe.outcome, Outcome::Completed, "{id}: the probe completes:{listing}");
    assert_eq!(r.map.len(), trace.rows.len(), "{id}: every trace row is mapped");
    let mut mapped = Vec::new();
    for ((trace_at, program_at), (at, _)) in r.map.iter().zip(&trace.rows) {
        assert_eq!(trace_at, at, "{id}: the map follows the trace's order");
        let from = mapped.last().map_or(0, |i| i + 1);
        let i = probe.rows[from..].iter().position(|row| row.at == *program_at);
        let i = i.unwrap_or_else(|| panic!("{id}: no row `{program_at}` after row {from} of the probe:{listing}"));
        mapped.push(from + i);
    }
    if let Some(would) = r.stop_would {
        let row = &probe.rows[mapped[mapped.len() - 1]];
        assert_eq!([row.steps, row.kernel_cost, row.memory], would, "{id}: the stopped operation's charge, admitted");
    }

    let limits = limits(id, &trace, &probe, &mapped);
    let (end, session) = compare(rt, &job_with(limits), setup);
    let summary = &ran(&end).summary;

    assert_eq!(summary.outcome.name(), trace.outcome, "{id}: outcome");
    assert_eq!(summary.code, trace.code.as_deref(), "{id}: code");
    let length = if trace.stop.is_some() { mapped[mapped.len() - 1] + 1 } else { probe.rows.len() };
    assert_eq!(summary.rows.len(), length, "{id}: the run ends at the mapped row:{listing}");
    let admitted = length - usize::from(trace.stop.is_some());
    assert_eq!(summary.rows[..admitted], probe.rows[..admitted], "{id}: the limits change nothing before the stop");
    for (i, (at, want)) in trace.rows.iter().enumerate() {
        let row = &summary.rows[mapped[i]];
        assert_eq!(charges(row), *want, "{id}: trace row `{at}` and program row `{}`", row.at);
    }
    assert!(summary.children.is_empty());

    match (&trace.stop, &summary.stop) {
        (None, None) => {}
        (Some(want), Some(got)) => {
            let program_at = r.map.iter().find(|(at, _)| *at == want.at).expect("the stop row is mapped").1;
            assert_eq!(got.at, program_at, "{id}: stop position");
            assert_eq!((got.phase, got.scope.as_str()), (want.phase.as_str(), want.scope.as_str()), "{id}: stop");
            match (&want.source, r.source_node) {
                (None, None) => {}
                (Some((digest, start, end)), Some(node)) => {
                    // The trace's convention: the SHA-256 of the source text and a byte interval that a multi-byte
                    // character makes differ from a character interval.
                    let text = trace.source_text.as_deref().expect("the trace's source text");
                    assert_eq!(&sha256_hex(text.as_bytes()), digest);
                    assert!(text.get(*start..*end).is_some() && text[..*end].chars().count() != *end);
                    // The program's: the same convention for the stopped node.
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
    (r.values)(&trace.values, &session);
}

/// CT-RTE-04 in part: eight DET.65 traces realised as programs keep their exact results, charges, stopping points
/// and source associations on both executors; the other 32 are listed with the reason they are not realised.
#[test]
fn ct_rte_04_partial_det65_traces_on_both_executors() {
    let text_of = std::fs::read_to_string(VECTORS).unwrap_or_else(|e| panic!("cannot read {VECTORS}: {e}"));
    let vectors = Json::parse(&text_of).unwrap_or_else(|e| panic!("vectors.json does not parse: {e}"));
    let all = vectors.get("vectors").and_then(Json::arr).expect("vectors");
    let realised = realisations();

    let mut ids: Vec<&str> = all.iter().map(|v| text(v, "id")).collect();
    let mut accounted: Vec<&str> = realised.iter().map(|r| r.trace).chain(UNREALISED.iter().map(|u| u.0)).collect();
    ids.sort_unstable();
    accounted.sort_unstable();
    assert_eq!(ids.len(), 40);
    assert_eq!(accounted, ids, "every frozen trace is realised or listed with its reason, exactly once");

    let source = format!("{}{PROGRAM}", header("random"));
    let mut rt = runtime();
    for r in &realised {
        let vector = all.iter().find(|v| text(v, "id") == r.trace).expect("a frozen trace");
        realise(&mut rt, &source, r, vector);
    }
    assert_eq!(rt.spawns(), 1, "one child ran every realisation");
}
