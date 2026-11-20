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
//! The limits give every mapped row the trace's headroom. A probe run with wide limits measures the overhead (with
//! other inputs only where the trace stops on a value: below). For a trace that completes, each limit is the
//! program's use plus the trace's slack (limit minus use; memory's use is the peak), and the run uses what the probe
//! used. For a trace that stops at a row, each cumulative limit is the program's charges before the mapped row plus
//! the trace's headroom before its row; the memory limit is the program's live memory before that row plus the trace's
//! memory headroom, raised to the program's earlier peak unless the trace stops on memory (a realisation that would
//! need the raise there is refused). The stopped operation, admitted in the probe, must charge what the trace's
//! operation would (`stop_would`), so the same headroom stops it in the same phase.
//!
//! Both executors then run each job with those limits and must agree on everything: summary, rows, stop, output, value
//! and every host call in order. The run must also map onto the trace: outcome and code, the mapped rows' charges, the
//! stop at the mapped row with the trace's phase and scope, the source association, and the trace's values.
//!
//! # Realised: 32 of 40
//!
//! | Trace | Clause | Realisation |
//! | --- | --- | --- |
//! | `approx-branch-true` | DET.64 | `pick(1, 2, r)`: `display_if` takes `r.v` (two visits) |
//! | `approx-branch-false` | DET.64 | `pick(5, 2, r)`: `display_if` takes `x` (one visit) |
//! | `display-stop-source-range` | DET.56 | `pick(1, 2, r)` stopped on steps at `r.v`, after a non-BMP character |
//! | `step-precedence` | DET.56 | `show()`: the output unit of `publish_view(record { })`, refused on steps |
//! | `simultaneous-exhaustion` | DET.56 | the same unit, steps admitted, refused on kernel cost |
//! | `simultaneous-memory-output` | DET.56 | the same unit, kernel cost admitted at its limit, refused on memory |
//! | `equal-to-limit` | DET.56 | `limit(7)`: a host read priced as the trace's kernel row, released at scope exit |
//! | `generator` | DET.61 | `draws()` from the trace's seed and path: two draws, published, then a refused draw |
//! | `generator-alias-cursor` | DET.61 | `twins()`: `g` started at `root`, aliased as `h`; both draws published |
//! | `generator-returned-owner` | DET.61 | `owner()`: `make()` starts a generator at `root` and returns it |
//! | `generator-child-paths` | DET.61 | `children()`: streams at `root/0` and `root/1`, one draw from each |
//! | `nested-calls` | DET.58 | `outer()` calls `inner()`, which reads `k`; neither frame has a slot (16 bytes) |
//! | `output-equality` | DET.59 | `equal("abcde")`: one eight-byte unit at equality |
//! | `output-retained-at-cutoff` | DET.59 | `retain("abcde", "")`: one unit published, the second refused on output |
//! | `oversized-arithmetic` | DET.57 | `mul(2^80, 2^80)`: `exact_mul` of 11 limbs by 11, refused on kernel cost |
//! | `oversized-admitted` | DET.57 | the same product admitted at its limits, published, released at the scope's exit |
//! | `arithmetic-reservation` | DET.57 | `mul(0, 2^80)`: 12 bytes reserved, 10 released as the product's 2 settle |
//! | `alias-release` | DET.58 | `release()`: `mk()` aliases `a` as `b` and returns `b`, so `a` is released first |
//! | `alias-still-live` | DET.58 | `live()`: the same alias, then a five-byte literal refused on memory |
//! | `pointwise-failure` | DET.55 | `check(7)`: `assert` of a false comparison ends the run with its error code |
//! | `child-allowances` | DET.60 | `allow()`: a two-step child, cut off at its `return`; the root cuts its kernel cap |
//! | `child-return-owner` | DET.60 | `owned()`: a child's four-byte text moves to the parent, which drops it |
//! | `root-before-child` | DET.60 | `early()`: the root and the child run out of steps at the body's `return` |
//! | `dimension-before-scope` | DET.60 | `dim()`: a child's `k`, over its kernel cap while the root lacks memory |
//! | `nested-child-ledger` | DET.60 | `nest()`: a child in a child; both complete, each charged in its ancestors |
//! | `child-decode-error-releases-reservation` | DET.60 | `spill()`: a child decodes `FF`; the parent's literal fits |
//! | `final-loop-test` | DET.55 | `spin()`: `repeat(1, 0, …)` visits `acc` once; the final test is refused on steps |
//! | `decode-scalar` | DET.62 | `decode()`: the snapshot `b` holds four bytes of one scalar; 16 reserved, 12 released |
//! | `decode-surrogate` | DET.62 | `decode()`: an encoded surrogate stops the run at its decoding (effect phase) |
//! | `streaming` | DET.63 | `go()`: `s` streams five letters in chunks of two; the last is refused on kernel cost |
//! | `streaming-complete-split` | DET.63 | `go()`: the same stream completes; `s`'s scope exit drops the cursor |
//! | `streaming-complete-coalesced` | DET.63 | the split trace's job: a transport partition is not accounted |
//!
//! The trace's `display_if` condition is a given value that holds no memory. `pick` binds its comparison with `let`,
//! so the `display_if` row visits a borrowed condition and releases nothing; the comparison's byte is released at the
//! scope's exit, as overhead. In `display-stop-source-range` the trace's untaken branch is empty; the program's
//! untaken branch is never observed.
//!
//! An abstract `emit` charges its encoding to kernel cost, memory and output in one row (DET.59), and so does the
//! interpreter's output unit since LP2's M2 meter: its encoding work, buffer and bytes in one admission, in DET.56's
//! order. The output profile differs (CV.5): the trace's bytes are PS1.2's retained JSON, the program's are
//! `c600-json-1`'s, UTF-8 with a terminal line feed. A realisation publishes a value whose bytes are as many as the
//! trace's: `{}` and a line feed for `"x"` (three), `"abcde"` and a line feed for `"\u00e9"` (eight). The three
//! DET.56 realisations then exercise steps before everything, kernel cost before memory and output, and memory before
//! output: `simultaneous-memory-output` stops on memory while its output headroom is short too. The one exception is
//! `output-retained-at-cutoff`'s refused unit `0`: one byte in the trace, while no unit of the program is shorter
//! than three. Its kernel-cost headroom gets the two bytes of difference (`kernel_extra`), so it still stops on output
//! with the trace's refused row.
//!
//! The trace's generator key and counter are not observable through a run; the drawn values are, through the
//! published view. `generator` draws with `random_draw` from the run's own generator, which the job starts at the
//! trace's seed and path. The other three start generators of their own with `rng_init` (LP2 draft), whose stream path
//! is the run's path followed by its segments: their job has the trace's seed and the empty path, so that each
//! `rng_init` names the trace's path in full, and the run's own start on the empty path is overhead. `make`'s frame
//! has no slot, as the trace's call has none. The child paths are named streams here; a child scope's own stream
//! path (DET.61) is `c600-run`'s `child_scopes.rs`'s.
//!
//! The child traces run `child` scopes (DET.60; LP2 draft). Each body is one operation, so its frame has no slot
//! and its `call` and `return` reserve and release the trace's 16 bytes; each stop branch names no status, so after
//! an unwind it costs its own frame only. A trace's tick is the body's literal, its `return` or the parent's `let`
//! binding; its `alloc` is a four-byte text literal, and `parent-drop` the scope exit that releases it. A trace that
//! completes with child records takes its limits at its first `child` row instead: the program's charges before
//! that row plus the trace's headroom there, so the root leaves the child what the trace's root leaves it and each
//! requested cap is cut as the trace's is. A dimension that the overhead after the child needs more of is raised to
//! the probe's use. Each child record is compared field by field, the trace's scope names mapped to the program's
//! locators through the `child` rows. In `dimension-before-scope`, `k`'s ten bytes do not fit in the root either,
//! as in the trace, so the child's kernel-cost stop shows DET.56's dimension order before the scope order; its
//! result is never delivered. In `child-decode-error-releases-reservation` the body decodes bytes the code holds with
//! `text_decode_hex` (LP2 draft), which take no row and no slot, so the child's rows are its call and its decoding,
//! as the trace's are. The decoding stops the child in the effect phase, the unwind releases its reservation in the
//! child and the root, and the parent's five-scalar literal of 20 bytes, the trace's `parent`, fits only because the
//! child left nothing live.
//!
//! The exact-arithmetic traces price `exact_mul` with the test tariffs: eight-bit limbs, one kernel unit per pair of
//! limbs, and a reservation of one byte plus the operands' bits in bytes; the product is the published view.
//!
//! `let` scopes nest, so the alias traces, which release `a` before its alias `b`, cross a call: `mk` binds `a`,
//! aliases it as `b` and returns `b`, which leaves with its own owner while `mk`'s scope exit releases `a`. The
//! trace's `c` is a literal of the caller, or in `alias-still-live` of a further callee `yz` whose frame has as many
//! slots as `mk`'s (two unit `let`s): the overhead's memory peak inside `mk` is then exactly the live memory at `c`
//! plus the trace's headroom, so the memory stop needs no raise.
//!
//! A trace that ends in a domain error stops on a value, while the probe must complete. `pointwise-failure`'s probe
//! runs `check(1)`, whose comparison holds; the run's argument `7` reads alike (one digit), so every row before the
//! assertion is the same.
//!
//! `final-loop-test` runs `repeat` (LP2 draft), a loop in its function's frame with no call per iteration, as the
//! trace's loop has none. Its body's `visit` of `acc` is the trace's tick; filling and refilling `acc`, and the count
//! and the initial value, are overhead. The run is refused on steps at `loop_test:1`, the final false test, which
//! would cost one step (DET.55).
//!
//! The decoding traces decode bytes the host delivers as the snapshot `b` (DET.62; LP2 draft), as LP2's
//! `det65_programs.rs` does: bytes have no canonical form, so they cannot cross as an argument; the host's answer
//! crosses to the child in the wire's transfer form (value tag 25). The read is a row of the program's own, and `let`
//! binds its result, so the decoding borrows the bytes and its row releases only the reservation's excess, as the
//! trace's does. The decoded text, the trace's `t`, is the entry's value. `decode-surrogate` stops on its value, so
//! its probe's host delivers as many ASCII bytes instead (`probe_setup`), which read alike and decode.
//!
//! The streaming traces stream a compressed word with `word_stream` (DET.63; LP2 draft), as LP2's
//! `det65_programs.rs` does; its rows are the frozen `open` and `chunk:<position>` rows. The literal word and its
//! compression are the program's own, and `go` passes the word and the chunk size to `s`: parameters are borrowed, so
//! the stream releases no operand, and `s`'s scope exit, which releases the cursor alone, is the frozen `close`.
//! `streaming` is cut off on kernel cost at the chunk at position 4, whose step is admitted, before it expands a
//! letter; its probe's scope exit is overhead.
//!
//! LP2 runs the split and the coalesced trace as one program and one run, since a run has no transport between the
//! interpreter and a kernel. That holds on the child path too. A word stream stays in its run (the wire codec refuses
//! `Value::WordStream`; only its type crosses, as tag 29), so `go` takes no stream and returns none. `word_stream` is
//! one operation of the interpreter, which calls no host between its open and its last chunk, so no host call or answer
//! falls among a stream's rows. A `cancel` may arrive while a stream runs: an interruption is observed before each step
//! (DET.56), and each chunk is a step, so a cancelled stream stops at a chunk boundary (LP2's `word_stream`). Neither
//! these traces nor this test sends one. These programs call no host at all (`streamed` holds their host calls to
//! none), so each run crosses as its job's frame and its end's frame, and the end carries every row and charge whole.
//! The test holds the two partitions' ends to each other; both run the same job under the same limits, so this checks
//! only that the run repeats. It holds the cut-off run's live memory at the stop, beyond the program's own before the
//! open, to the trace's live cursor. The child path gives the coalesced trace nothing more to check.
//!
//! # Not realised: 8 of 40
//!
//! [`UNREALISED`] lists each with its reason; the test holds the two lists to the 40 frozen vectors. In short: the IR
//! has no flow check or text order; no call frame without a slot holds a binding (`generator-nested-same-name`); and
//! the region contract `lp1-exec-region-draft-1` has no executor yet.

mod support;

#[path = "../../c600-check/tests/support/json.rs"]
#[allow(dead_code)]
mod json;

use c600_digest::sha256_hex;
use c600_ir::Capability;
use c600_run::interp::{RowTariff, Run};
use c600_run::meter::{Amounts, Outcome, Row, Summary, Tariffs};
use c600_run::value::{equal, Value};
use c600_runtime::{run_in_process, Ended, JobOptions, Runtime};
use json::Json;
use num_bigint::BigInt;
use support::{assert_same_end, compare, header, job, ran, runtime, Session, WIDE};

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
    let v = publish_view(record { a = a, b = b });\n    let c = random_draw();\n    return a;\n}\n\n\
    export fn twins() -> Unit effects [random, view] {\n    let g = rng_init(\"root\");\n    let h = g;\n    \
    return publish_view(record { a = rng_draw(g), b = rng_draw(h) });\n}\n\n\
    fn make() -> Generator effects [random] {\n    return rng_init(\"root\");\n}\n\n\
    export fn owner() -> Unit effects [random, view] {\n    let h = make();\n    \
    return publish_view(rng_draw(h));\n}\n\n\
    export fn children() -> Unit effects [random, view] {\n    let g0 = rng_init(\"root\", \"0\");\n    \
    let a = rng_draw(g0);\n    let g1 = rng_init(\"root\", \"1\");\n    \
    return publish_view(record { a = a, b = rng_draw(g1) });\n}\n\n\
    fn inner() -> Unit effects [snapshot] {\n    return read_snapshot<Unit>(\"k\");\n}\n\n\
    export fn outer() -> Unit effects [snapshot] {\n    return inner();\n}\n\n\
    export fn equal(t: Text) -> Unit effects [view] {\n    return publish_view(t);\n}\n\n\
    export fn retain(t: Text, u: Text) -> Unit effects [view] {\n    let a = publish_view(t);\n    \
    return publish_view(u);\n}\n\n\
    export fn mul(a: Int, b: Int) -> Unit effects [view] {\n    let x = exact_mul(a, b);\n    \
    let v = publish_view(x);\n    return ();\n}\n\n\
    fn mk() -> Nat effects [] {\n    let a = 65536;\n    let b = a;\n    return b;\n}\n\n\
    fn mid() -> Nat effects [] {\n    let b = mk();\n    return 65537;\n}\n\n\
    export fn release() -> Unit effects [] {\n    let c = mid();\n    return ();\n}\n\n\
    fn yz() -> Nat effects [] {\n    let p = ();\n    let q = ();\n    return 16777216;\n}\n\n\
    fn late() -> Nat effects [] {\n    let b = mk();\n    return yz();\n}\n\n\
    export fn live() -> Unit effects [] {\n    let c = late();\n    return ();\n}\n\n\
    export fn check(n: Nat) -> Unit effects [] {\n    let p = exact_less(n, 5);\n    return assert(p);\n}\n\n\
    export fn allow() -> Unit effects [budget] {\n    \
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
    fn() -> Unit { return read_snapshot<Unit>(\"k\"); }, fn() -> Unit { return (); });\n    return ();\n}\n\n\
    export fn nest() -> Unit effects [budget] {\n    \
    let u = child(record { steps = 10, kernel_cost = 10, memory = 60, output = 0 }, fn() -> Unit {\n        \
    return child(record { steps = 4, kernel_cost = 4, memory = 20, output = 0 }, \
    fn() -> Unit { return (); }, fn() -> Unit { return (); });\n    \
    }, fn() -> Unit { return (); });\n    return ();\n}\n\n\
    export fn spin() -> Nat effects [] {\n    return repeat(1, 0, fn(acc: Nat) -> Nat { return acc; });\n}\n\n\
    export fn decode() -> Text effects [snapshot] {\n    let b = read_snapshot<Bytes>(\"b\");\n    \
    return text_decode(b);\n}\n\n\
    fn s(c: CompressedWord, k: Nat) -> Unit effects [] {\n    let cursor = word_stream(c, k);\n    return ();\n}\n\n\
    export fn go() -> Unit effects [] {\n    let c = compress(word[+1, -2, +3, -4, +5]);\n    return s(c, 2);\n}\n\n\
    export fn spill() -> Unit effects [budget] {\n    \
    let t = child(record { steps = 20, kernel_cost = 20, memory = 20, output = 0 }, \
    fn() -> Text { return text_decode_hex(\"ff\"); }, fn() -> Text { return \"\"; });\n    \
    let p = \"abcde\";\n    return ();\n}\n";

/// The frozen traces this file does not realise, and why.
const UNREALISED: [(&str, &str); 8] = [
    (
        "generator-nested-same-name",
        "its call's frame has no slot, yet the callee binds `g` and `a` and releases them with the frame; the IR binds \
         a name with `let`, which takes a frame slot and releases at its scope's exit",
    ),
    ("text-comparison", "the IR has no text order"),
    (
        "abstract-trace-charges",
        "its body's `flow_check` belongs to the unselected flow-label alternative (DET.55; LQ21, RTE.24): the IR has \
         no flow check",
    ),
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
    /// The probe's arguments where they differ from the run's: a trace that ends in a domain error stops on a value,
    /// while the probe must complete. The rows before the stop must be the same (module docs).
    probe_args: Option<fn() -> Vec<Value>>,
    /// The job options for the given limits, from the trace's `given`.
    options: fn(&Json, Amounts) -> JobOptions,
    /// Prepares the caller's session from the trace's `given`.
    setup: fn(&Json, &mut Session),
    /// The probe's session where it differs from the run's: a trace that stops on decoded bytes probes with bytes
    /// that read alike and decode (module docs).
    probe_setup: Option<fn(&Json, &mut Session)>,
    /// Every trace row's `at`, in order, and the program row it maps to.
    map: &'static [(&'static str, &'static str)],
    /// For a stopping trace: steps, kernel cost, memory and output its stopped operation would charge if admitted.
    stop_would: Option<[u128; 4]>,
    /// Kernel-cost headroom added to the trace's before its stopped row: only the output profile's difference
    /// (module docs).
    kernel_extra: u128,
    /// The program text of the node that a stop's source range must cover.
    source_node: Option<&'static str>,
    /// Holds the trace's expected values to what the run delivered: its value or what its host received.
    values: fn(&[(String, Json)], &Run, &Session),
}

fn plain(_: &Json, limits: Amounts) -> JobOptions {
    JobOptions::new(limits)
}

fn untouched(_: &Json, _: &mut Session) {}

fn no_values(values: &[(String, Json)], _: &Run, _: &Session) {
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

/// The job's seed is the trace's first `rng_init`, at any depth, and its path is empty: a program that starts its own
/// generators names the trace's stream paths in full (module docs).
fn streams(given: &Json, limits: Amounts) -> JobOptions {
    let seed = hex(text(first_of(given, "rng_init"), "seed"));
    JobOptions { seed: Some((seed, Vec::new())), grant: vec![Capability::Randomness], ..JobOptions::new(limits) }
}

/// The trace's first operation `op`, at any depth.
fn first_of<'a>(given: &'a Json, op: &str) -> &'a Json {
    fn find<'a>(ops: &'a [Json], op: &str) -> Option<&'a Json> {
        ops.iter().find_map(|o| {
            if o.get("op").and_then(Json::str) == Some(op) {
                return Some(o);
            }
            ["body", "then", "else"].iter().find_map(|k| o.get(k).and_then(Json::arr).and_then(|b| find(b, op)))
        })
    }
    find(given.get("trace").and_then(Json::arr).expect("a trace"), op).unwrap_or_else(|| panic!("a `{op}` operation"))
}

/// The snapshot `k` is priced as the trace's kernel row, at any depth: the batch's kernel cost, a reservation of its
/// result and scratch bytes, and a result of exactly the trace's result size (`Unit` for none, else an empty record).
fn kernel_snapshot(given: &Json, session: &mut Session) {
    let k = first_of(given, "kernel");
    let bytes = k.get("result_bytes").map_or(0, |_| uint(k, "result_bytes"));
    let result = if bytes == 0 { Value::Unit } else { Value::record(vec![]) };
    assert_eq!(result.size(), bytes, "a result of the trace's size");
    let tariff =
        RowTariff { kernel_cost: Tariffs::TEST.kernel(uint(k, "items")), result_bound: bytes + uint(k, "scratch") };
    session.snapshots.push(("k", tariff, result));
}

/// `dimension-before-scope`'s `k`, priced as the trace's kernel row: the batch's kernel cost and a reservation of its
/// ten result bytes. The child's kernel cap refuses it, so its result, a placeholder here, is never delivered.
fn refused_snapshot(given: &Json, session: &mut Session) {
    let k = first_of(given, "kernel");
    let tariff =
        RowTariff { kernel_cost: Tariffs::TEST.kernel(uint(k, "items")), result_bound: uint(k, "result_bytes") };
    session.snapshots.push(("k", tariff, Value::Unit));
}

/// A generator realisation publishes the trace's drawn values in the trace's order, one as itself and two as
/// `record { a, b }`: `draws` publishes `record { a = draw-0, b = draw-1 }` before its refused draw.
fn published_draws(values: &[(String, Json)], _: &Run, session: &Session) {
    let drawn: Vec<Value> = values
        .iter()
        .map(|(_, v)| v.str().expect("a drawn block"))
        .map(|hex| Value::int(BigInt::parse_bytes(hex.as_bytes(), 16).expect("hex")))
        .collect();
    let view = match drawn.as_slice() {
        [one] => one.clone(),
        [a, b] => Value::record(vec![("a", a.clone()), ("b", b.clone())]),
        _ => panic!("one or two drawn values: {values:?}"),
    };
    let mut bytes = Vec::new();
    c600_run::encode::encode(&view, &mut bytes).expect("a canonical value");
    assert_eq!(session.views, vec![bytes], "the drawn values are the trace's");
}

/// `2^n`, an integer argument of `n + 1` bits.
fn power(n: u32) -> Value {
    Value::int(BigInt::from(1u8) << n)
}

/// `mul` publishes its product, the trace's one value.
fn published_product(values: &[(String, Json)], _: &Run, session: &Session) {
    let [(name, product)] = values else { panic!("one value in the trace's values: {values:?}") };
    assert_eq!(name, "mul");
    let product = BigInt::parse_bytes(product.num().expect("a number").as_bytes(), 10).expect("decimal digits");
    let mut bytes = Vec::new();
    c600_run::encode::encode(&Value::int(product), &mut bytes).expect("a canonical integer");
    assert_eq!(session.views, vec![bytes], "the product is the trace's");
}

/// The program's unit has the trace's byte count (module docs): `"abcde"` and a line feed for the trace's
/// `"\u00e9"`. Only an admitted unit reaches the host.
fn eight_byte_unit(values: &[(String, Json)], _: &Run, session: &Session) {
    let [(_, fixture)] = values else { panic!("one unit in the trace's values: {values:?}") };
    let fixture = hex(fixture.str().expect("hex bytes"));
    assert_eq!(fixture, b"\"\\u00e9\"", "PS1.2's bytes of \"\u{e9}\"");
    assert_eq!(session.views, [b"\"abcde\"\n"], "only the admitted unit reaches the host");
    assert_eq!(session.views[0].len(), fixture.len(), "the program's unit has the trace's byte count");
}

/// The trace's bytes to decode (DET.62).
fn trace_bytes(given: &Json) -> Vec<u8> {
    hex(text(first_of(given, "text_decode"), "hex"))
}

/// The host delivers `bytes` as the snapshot `b` (module docs): no kernel cost, and a reservation of exactly the
/// bytes, so the read settles nothing.
fn deliver(session: &mut Session, bytes: Vec<u8>) {
    let tariff = RowTariff { kernel_cost: 0, result_bound: bytes.len() as u128 };
    session.snapshots.push(("b", tariff, Value::Bytes(bytes.into())));
}

/// The snapshot `b` holds the trace's bytes.
fn snapshot_bytes(given: &Json, session: &mut Session) {
    deliver(session, trace_bytes(given));
}

/// The probe of a decoding that stops on its bytes: as many ASCII bytes, which read alike and decode (module docs).
fn ascii_bytes(given: &Json, session: &mut Session) {
    deliver(session, vec![b'a'; trace_bytes(given).len()]);
}

/// `decode` returns the decoded text, the trace's one value; a decoding that stops binds nothing.
fn decoded_text(values: &[(String, Json)], run: &Run, _: &Session) {
    match values {
        [] => assert!(run.value.is_none(), "no value is bound: {:?}", run.value),
        [(name, t)] => {
            assert_eq!(name, "decode");
            let want = Value::text(t.str().expect("the decoded text"));
            let got = run.value.as_ref().expect("the decoded text");
            assert!(equal(got, &want), "the decoded text is the trace's: {got:?}");
        }
        _ => panic!("at most one value in the trace's values: {values:?}"),
    }
}

/// The streaming traces have no values. `go` calls no host, so its run crosses between the host and the child as its
/// job and its end alone, and no host call or answer falls among the stream's rows (module docs).
fn streamed(values: &[(String, Json)], run: &Run, session: &Session) {
    no_values(values, run, session);
    assert!(session.log.is_empty(), "no host call: {:?}", session.log);
}

const PICK_TRUE: &[(&str, &str)] = &[
    ("display", "m.pick:1#display_if"),
    ("display/then/a", "m.pick:1.1.0#visit"),
    ("display/then/b", "m.pick:1.1#visit"),
];
const PICK_FALSE: &[(&str, &str)] = &[("display", "m.pick:1#display_if"), ("display/else/a", "m.pick:1.2#visit")];
const SHOW: &[(&str, &str)] = &[("emit", "m.show:root#publish_view")];
const MUL: &[(&str, &str)] = &[("mul", "m.mul:0#exact_mul"), ("drop", "m.mul:root#scope_exit")];
const ALIAS: &[(&str, &str)] = &[
    ("a", "m.release:0 > m.mid:0 > m.mk:0#value"),
    ("b", "m.release:0 > m.mid:0 > m.mk:1#alias"),
    ("drop-a", "m.release:0 > m.mid:0 > m.mk:root#scope_exit"),
    ("c", "m.release:0 > m.mid:1#value"),
    ("drop-b", "m.release:0 > m.mid:root#scope_exit"),
    ("drop-c", "m.release:root#scope_exit"),
];
const ALLOW: &[(&str, &str)] = &[
    ("c/child", "m.allow:0#child"),
    ("c/run/call", "m.allow:0#call"),
    ("c/run/a", "m.allow:0 > m.allow:0.0#value"),
    ("c/run/b", "m.allow:0 > m.allow:0.0#return"),
    ("parent", "m.allow:root#visit"),
];
const OWNED: &[(&str, &str)] = &[
    ("c/child", "m.owned:0#child"),
    ("c/run/call", "m.owned:0#call"),
    ("c/run/a", "m.owned:0 > m.owned:0.0#value"),
    ("c/run/return", "m.owned:0 > m.owned:0.0#return"),
    ("parent-drop", "m.owned:root#scope_exit"),
];
const EARLY: &[(&str, &str)] = &[
    ("c/child", "m.early:0#child"),
    ("c/run/call", "m.early:0#call"),
    ("c/run/a", "m.early:0 > m.early:0.0#value"),
    ("c/run/b", "m.early:0 > m.early:0.0#return"),
];
const DIM: &[(&str, &str)] = &[
    ("c/child", "m.dim:0#child"),
    ("c/run/call", "m.dim:0#call"),
    ("c/run/k", "m.dim:0 > m.dim:0.0#kernel"),
    ("parent", "m.dim:root#visit"),
];
const NEST: &[(&str, &str)] = &[
    ("outer/child", "m.nest:0#child"),
    ("outer/run/call", "m.nest:0#call"),
    ("outer/run/inner/child", "m.nest:0 > m.nest:0.0#child"),
    ("outer/run/inner/run/call", "m.nest:0 > m.nest:0.0#call"),
    ("outer/run/inner/run/work", "m.nest:0 > m.nest:0.0 > m.nest:0.0.0#value"),
    ("outer/run/inner/run/return", "m.nest:0 > m.nest:0.0 > m.nest:0.0.0#return"),
    ("outer/run/return", "m.nest:0 > m.nest:0.0#return"),
    ("parent", "m.nest:root#visit"),
];
const SPILL: &[(&str, &str)] = &[
    ("c/child", "m.spill:0#child"),
    ("c/run/call", "m.spill:0#call"),
    ("c/run/decode", "m.spill:0 > m.spill:0.0#text_decode"),
    ("parent", "m.spill:1.0#value"),
];
const STILL_LIVE: &[(&str, &str)] = &[
    ("a", "m.live:0 > m.late:0 > m.mk:0#value"),
    ("b", "m.live:0 > m.late:0 > m.mk:1#alias"),
    ("drop-a", "m.live:0 > m.late:0 > m.mk:root#scope_exit"),
    ("c", "m.live:0 > m.late:1 > m.yz:1.1#value"),
];
const SPIN: &[(&str, &str)] = &[
    ("loop/test:0", "m.spin:root#loop_test:0"),
    ("loop/body:0/body", "m.spin:root#body:0 > m.spin:2#visit"),
    ("loop/test:1", "m.spin:root#loop_test:1"),
];
const DECODE: &[(&str, &str)] = &[("decode", "m.decode:1#text_decode")];
const STREAM: &[(&str, &str)] = &[
    ("word/open", "m.go:1 > m.s:0#word_open"),
    ("word/chunk:0", "m.go:1 > m.s:0#chunk:0"),
    ("word/chunk:2", "m.go:1 > m.s:0#chunk:2"),
    ("word/chunk:4", "m.go:1 > m.s:0#chunk:4"),
    ("close", "m.go:1 > m.s:root#scope_exit"),
];

fn realisations() -> Vec<Realisation> {
    let base = Realisation {
        trace: "",
        entry: "pick",
        args: || pick_args(1, 2),
        probe_args: None,
        options: plain,
        setup: untouched,
        probe_setup: None,
        map: PICK_TRUE,
        stop_would: None,
        kernel_extra: 0,
        source_node: None,
        values: no_values,
    };
    let show =
        |trace| Realisation { trace, entry: "show", args: Vec::new, map: SHOW, stop_would: Some([1, 3, 3, 3]), ..base };
    let stream = |trace| Realisation { trace, entry: "go", args: Vec::new, map: STREAM, values: streamed, ..base };
    vec![
        Realisation { trace: "approx-branch-true", ..base },
        Realisation { trace: "approx-branch-false", args: || pick_args(5, 2), map: PICK_FALSE, ..base },
        Realisation {
            trace: "display-stop-source-range",
            stop_would: Some([1, 0, 0, 0]),
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
            stop_would: Some([1, 3, 33, 0]),
            values: published_draws,
            ..base
        },
        Realisation {
            trace: "generator-alias-cursor",
            entry: "twins",
            args: Vec::new,
            options: streams,
            map: &[
                ("init", "m.twins:0#rng_init"),
                ("alias", "m.twins:1#alias"),
                ("draw-g", "m.twins:1.1.0.0#rng_draw"),
                ("draw-h", "m.twins:1.1.0.1#rng_draw"),
            ],
            values: published_draws,
            ..base
        },
        Realisation {
            trace: "generator-returned-owner",
            entry: "owner",
            args: Vec::new,
            options: streams,
            map: &[
                ("make/call", "m.owner:0#call"),
                ("make/init", "m.owner:0 > m.make:root#rng_init"),
                ("make/return", "m.owner:0 > m.make:root#return"),
                ("draw-h", "m.owner:1.0#rng_draw"),
            ],
            values: published_draws,
            ..base
        },
        Realisation {
            trace: "generator-child-paths",
            entry: "children",
            args: Vec::new,
            options: streams,
            map: &[
                ("init-0", "m.children:0#rng_init"),
                ("draw-0", "m.children:1.0#rng_draw"),
                ("init-1", "m.children:1.1.0#rng_init"),
                ("draw-1", "m.children:1.1.1.0.1#rng_draw"),
            ],
            values: published_draws,
            ..base
        },
        Realisation {
            trace: "nested-calls",
            entry: "outer",
            args: Vec::new,
            setup: kernel_snapshot,
            map: &[
                ("outer/call", "entry#call"),
                ("outer/inner/call", "m.outer:root#call"),
                ("outer/inner/k", "m.outer:root > m.inner:root#kernel"),
                ("outer/inner/return", "m.outer:root > m.inner:root#return"),
                ("outer/return", "m.outer:root#return"),
            ],
            ..base
        },
        Realisation {
            trace: "output-equality",
            entry: "equal",
            args: || vec![Value::text("abcde")],
            map: &[("emit", "m.equal:root#publish_view")],
            values: eight_byte_unit,
            ..base
        },
        Realisation {
            trace: "output-retained-at-cutoff",
            entry: "retain",
            args: || vec![Value::text("abcde"), Value::text("")],
            map: &[("first", "m.retain:0#publish_view"), ("denied", "m.retain:1#publish_view")],
            stop_would: Some([1, 3, 3, 3]),
            kernel_extra: 3 - 1,
            values: eight_byte_unit,
            ..base
        },
        // 2^80 · 2^80: eleven limbs by eleven, and a reservation of 1 + ceil(162 / 8) bytes (DET.57).
        Realisation {
            trace: "oversized-arithmetic",
            entry: "mul",
            args: || vec![power(80), power(80)],
            map: &MUL[..1],
            stop_would: Some([1, 121, 22, 0]),
            ..base
        },
        Realisation {
            trace: "oversized-admitted",
            entry: "mul",
            args: || vec![power(80), power(80)],
            map: MUL,
            values: published_product,
            ..base
        },
        // 0 · 2^80: one limb by eleven; the reservation of 12 bytes settles on the product's 2.
        Realisation {
            trace: "arithmetic-reservation",
            entry: "mul",
            args: || vec![Value::int(0), power(80)],
            map: MUL,
            values: published_product,
            ..base
        },
        Realisation { trace: "alias-release", entry: "release", args: Vec::new, map: ALIAS, ..base },
        Realisation {
            trace: "alias-still-live",
            entry: "live",
            args: Vec::new,
            map: STILL_LIVE,
            stop_would: Some([1, 0, 5, 0]),
            ..base
        },
        Realisation {
            trace: "pointwise-failure",
            entry: "check",
            args: || vec![Value::int(7)],
            probe_args: Some(|| vec![Value::int(1)]),
            map: &[("predicate", "m.check:1.0#visit"), ("assert", "m.check:1#assert")],
            stop_would: Some([1, 0, 0, 0]),
            ..base
        },
        Realisation { trace: "child-allowances", entry: "allow", args: Vec::new, map: ALLOW, ..base },
        Realisation { trace: "child-return-owner", entry: "owned", args: Vec::new, map: OWNED, ..base },
        Realisation {
            trace: "root-before-child",
            entry: "early",
            args: Vec::new,
            map: EARLY,
            stop_would: Some([1, 0, 0, 0]),
            ..base
        },
        Realisation {
            trace: "dimension-before-scope",
            entry: "dim",
            args: Vec::new,
            setup: refused_snapshot,
            map: DIM,
            ..base
        },
        Realisation { trace: "nested-child-ledger", entry: "nest", args: Vec::new, map: NEST, ..base },
        Realisation {
            trace: "child-decode-error-releases-reservation",
            entry: "spill",
            args: Vec::new,
            map: SPILL,
            ..base
        },
        Realisation {
            trace: "final-loop-test",
            entry: "spin",
            args: Vec::new,
            map: SPIN,
            stop_would: Some([1, 0, 0, 0]),
            ..base
        },
        Realisation {
            trace: "decode-scalar",
            entry: "decode",
            args: Vec::new,
            setup: snapshot_bytes,
            map: DECODE,
            values: decoded_text,
            ..base
        },
        // The probe decodes three ASCII bytes, admitted at the surrogate's charges (module docs).
        Realisation {
            trace: "decode-surrogate",
            entry: "decode",
            args: Vec::new,
            setup: snapshot_bytes,
            probe_setup: Some(ascii_bytes),
            map: DECODE,
            stop_would: Some([1, 3, 12, 0]),
            values: decoded_text,
            ..base
        },
        // Cut off before the last chunk expands a letter: the trace ends there, so the probe's `close` is overhead.
        Realisation { map: &STREAM[..4], stop_would: Some([1, 3, 2, 0]), ..stream("streaming") },
        stream("streaming-complete-split"),
        stream("streaming-complete-coalesced"),
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

struct Trace {
    limits: Amounts,
    outcome: String,
    code: Option<String>,
    stop: Option<TraceStop>,
    used: Amounts,
    rows: Vec<(String, [u128; 5])>,
    values: Vec<(String, Json)>,
    children: Vec<TraceChild>,
    source_text: Option<String>,
}

impl Trace {
    fn read(vector: &Json) -> Trace {
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
            limits: amounts(given.get("limits").expect("limits")),
            outcome: text(expected, "outcome").into(),
            code: expected.get("code").and_then(Json::str).map(str::to_string),
            stop,
            used: amounts(expected.get("used").expect("used")),
            rows,
            values,
            children,
            source_text: given.get("source_text").and_then(Json::str).map(str::to_string),
        }
    }

    /// The index of the trace's row `name`.
    fn row(&self, name: &str) -> usize {
        self.rows.iter().position(|(at, _)| at == name).unwrap_or_else(|| panic!("no frozen row `{name}`"))
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

/// The realisation's limits: the trace's headroom on every mapped row, and for a stop `kernel_extra` (module docs).
fn limits(id: &str, trace: &Trace, probe: &Summary, mapped: &[usize], kernel_extra: u128) -> Amounts {
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
        kernel_cost: p.kernel + headroom(id, trace.limits.kernel_cost, t.kernel) + kernel_extra,
        memory,
        output: p.output + headroom(id, trace.limits.output, t.output),
    }
}

/// Realises one trace on both executors and returns the child's end.
fn realise(rt: &mut Runtime, source: &str, r: &Realisation, vector: &Json) -> Ended {
    let id = r.trace;
    let given = vector.get("given").expect("given");
    let trace = Trace::read(vector);
    let job_with = |args: fn() -> Vec<Value>, limits| job(source, r.entry, args(), (r.options)(given, limits));
    let setup = |s: &mut Session| (r.setup)(given, s);

    // The probe: wide limits, in process.
    let mut session = Session::default();
    (r.probe_setup.unwrap_or(r.setup))(given, &mut session);
    let probe = match run_in_process(&job_with(r.probe_args.unwrap_or(r.args), WIDE), &mut session, None) {
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
        let admitted = [row.steps, row.kernel_cost, row.memory, row.output];
        assert_eq!(admitted, would, "{id}: the stopped operation's charges, admitted");
    }
    assert!(r.kernel_extra == 0 || r.stop_would.is_some(), "{id}: extra kernel-cost headroom for a stop only");

    let limits = limits(id, &trace, &probe, &mapped, r.kernel_extra);
    let (end, session) = compare(rt, &job_with(r.args, limits), setup);
    let summary = &ran(&end).summary;

    let run = summary.rows.iter().map(|row| format!("\n  {} {:?}", row.at, charges(row))).collect::<String>();
    assert_eq!(
        summary.outcome.name(),
        trace.outcome,
        "{id}: outcome, stop {:?}, limits {limits:?}:{run}",
        summary.stop
    );
    assert_eq!(summary.code, trace.code.as_deref(), "{id}: code");
    let length = if trace.stop.is_some() { mapped[mapped.len() - 1] + 1 } else { probe.rows.len() };
    assert_eq!(summary.rows.len(), length, "{id}: the run ends at the mapped row:{listing}");
    let admitted = length - usize::from(trace.stop.is_some());
    assert_eq!(summary.rows[..admitted], probe.rows[..admitted], "{id}: the limits change nothing before the stop");
    for (i, (at, want)) in trace.rows.iter().enumerate() {
        let row = &summary.rows[mapped[i]];
        assert_eq!(charges(row), *want, "{id}: trace row `{at}` and program row `{}`", row.at);
    }
    // Child records (DET.60): a trace's scope is named by its `child` row, a program's by that row's locator.
    let program_at = |name: &str| {
        r.map.iter().find(|(at, _)| *at == name).unwrap_or_else(|| panic!("{id}: no mapped row `{name}`")).1
    };
    let scope = |name: &str| match name {
        "root" => "root",
        _ => program_at(&format!("{name}/child")).strip_suffix("#child").expect("a child row"),
    };
    assert_eq!(summary.children.len(), trace.children.len(), "{id}: child records");
    for (got, want) in summary.children.iter().zip(&trace.children) {
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
                let frozen = (program_at(&w.at), w.phase.as_str(), scope(&w.scope));
                assert_eq!((g.at.as_str(), g.phase, g.scope.as_str()), frozen, "{id}: child `{at}`'s stop");
                assert_eq!(w.source, None, "{id}: a frozen child stop with a source association");
            }
            (g, w) => panic!("{id}: child `{at}` stops at {g:?}, the trace's at {:?}", w.as_ref().map(|w| &w.at)),
        }
    }
    assert_eq!(summary.live_after_cleanup, 0, "{id}: cleanup releases everything");

    match (&trace.stop, &summary.stop) {
        // Each limit is the probe's use plus the trace's slack: where the trace has none, equality is admitted.
        (None, None) => assert_eq!(summary.used, probe.used, "{id}: the run uses what the probe used"),
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
    (r.values)(&trace.values, ran(&end), &session);
    end
}

/// CT-RTE-04 in part: 32 DET.65 traces realised as programs keep their exact results, charges, stopping points,
/// child records and source associations on both executors; the other 8 are listed with the reason they are not
/// realised.
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
    let vector = |id: &str| all.iter().find(|v| text(v, "id") == id).expect("a frozen trace");
    let mut ends = Vec::new();
    for r in &realised {
        ends.push((r.trace, realise(&mut rt, &source, r, vector(r.trace))));
    }
    assert_eq!(rt.spawns(), 1, "one child ran every realisation");

    // The streaming traces (module docs): the two transport partitions are one job and end alike; at the cut-off
    // stream's stop, the run's live memory beyond the program's own before the open is the trace's: the cursor.
    let end = |id: &str| &ends.iter().find(|(trace, _)| *trace == id).expect("a realised trace").1;
    assert_same_end(end("streaming-complete-split"), end("streaming-complete-coalesced"));
    let cut = &ran(end("streaming")).summary;
    let open = cut.rows.iter().position(|row| row.at.ends_with("#word_open")).expect("the open row");
    let own = Before::of(cut.rows[..open].iter().map(charges)).live;
    let frozen = uint(vector("streaming").get("expected").expect("expected"), "live_before_cleanup");
    assert_eq!(cut.live_before_cleanup - own, frozen, "the cursor alone is live at the stop beyond the program's own");
}
