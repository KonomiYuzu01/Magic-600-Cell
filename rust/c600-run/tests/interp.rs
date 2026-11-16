//! The interpreter (LP2, M2): checked surface programs run under the `lp1-exec-draft-1` meter, one row per operation.
//!
//! Rows are written out by hand from DET.55 to DET.59 and the module documentation of `c600_run::interp`; the frame
//! layout and the operation names are M2 drafts. The contract cases named here are covered in part: the frozen traces
//! themselves replay in `tests/traces.rs`.

use std::cell::Cell;

use c600_check::check;
use c600_ir::build::{call, call_in, exact, module, nat, param, prim, program, record, var};
use c600_ir::{Callee, Expr, Lit, Prim, Program, Ty};
use c600_run::interp::{
    code, load, Cancel, CancelFlag, Host, Loaded, NoHost, Options, Request, RowTariff, Run, Sink, SourceRanges,
};
use c600_run::meter::{self, Amounts, Interruption, Outcome, SourceRange, Summary};
use c600_run::rng::{self, Cursor};
use c600_run::value::Value;
use c600_syntax::{lower, parse, sha256_hex, Lowered};
use num_bigint::{BigInt, Sign};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const HEADER: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";

fn lowered(sources: &[&str]) -> Lowered {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    lower(&files).unwrap_or_else(|e| panic!("{e}"))
}

/// Parses, lowers, checks and loads source files; the checker must accept them.
fn build(sources: &[&str]) -> (Lowered, Loaded) {
    let lowered = lowered(sources);
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let loaded = load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"));
    (lowered, loaded)
}

/// One module `m` with the given declarations.
fn m(declarations: &str) -> String {
    format!("{HEADER}{declarations}")
}

struct Ranges<'a>(&'a Lowered);

impl SourceRanges for Ranges<'_> {
    fn range(&self, module: &str, function: &str, path: &str) -> Option<SourceRange> {
        let map = self.0.source(module)?;
        let span = map.span(function, path)?;
        Some(SourceRange { digest: map.digest.clone(), start: span.start, end: span.end })
    }
}

fn run(loaded: &Loaded, function: &str, args: Vec<Value>, options: &Options) -> Run {
    loaded.run("m", function, args, &mut NoHost, options).unwrap_or_else(|r| panic!("{r:?}"))
}

type Expected<'a> = (&'a str, u128, u128, u128, u128, u128);

/// The rows as (operation path, steps, kernel cost, memory, output, released).
fn assert_rows(summary: &Summary, expected: &[Expected]) {
    let got: Vec<_> =
        summary.rows.iter().map(|r| (r.at.clone(), r.steps, r.kernel_cost, r.memory, r.output, r.released)).collect();
    let want: Vec<_> = expected.iter().map(|&(a, s, k, m, o, r)| (a.to_string(), s, k, m, o, r)).collect();
    assert_eq!(got, want);
}

fn ats(summary: &Summary) -> Vec<&str> {
    summary.rows.iter().map(|r| r.at.as_str()).collect()
}

fn output(run: &Run) -> &str {
    std::str::from_utf8(run.output.as_deref().expect("a completed run")).expect("UTF-8")
}

const ADD: &str = "fn add(x: Nat, y: Nat) -> Nat effects [] {\n    return exact_add(x, y);\n}\n\n\
                   export fn main(a: Nat) -> Nat effects [] {\n    return add(a, 2);\n}\n";

/// CT-DET-28 (part): a call and its return are operations of their own. The callee's frame is 16 + 8·5 bytes
/// (two parameters, three temporaries), the caller's 16 + 8·4. The return releases the frame and the owned argument
/// temporary; the top-level result is encoded and emitted (DET.59).
#[test]
fn calls_and_returns_keep_their_operations() {
    let (_, loaded) = build(&[&m(ADD)]);
    let r = run(&loaded, "main", vec![Value::int(5)], &Options::new(WIDE));
    assert_eq!(r.summary.outcome, Outcome::Completed);
    assert_rows(
        &r.summary,
        &[
            ("entry#read", 1, 12, 2, 0, 0),
            ("entry#call", 1, 0, 48, 0, 0),
            ("m.main:0#visit", 1, 0, 0, 0, 0),
            ("m.main:1#value", 1, 0, 2, 0, 0),
            ("m.main:root#call", 1, 0, 56, 0, 0),
            ("m.main:root > m.add:0#visit", 1, 0, 0, 0, 0),
            ("m.main:root > m.add:1#visit", 1, 0, 0, 0, 0),
            ("m.main:root > m.add:root#exact_add", 1, 2, 2, 0, 0),
            ("m.main:root > m.add:root#return", 1, 0, 0, 0, 56 + 2),
            ("m.main:root#return", 1, 0, 0, 0, 48 + 2),
            ("entry#encode", 1, 13, 13, 0, 1),
            ("entry#emit", 1, 0, 0, 12, 12 + 2),
        ],
    );
    assert_eq!(r.summary.used, Amounts { steps: 12, kernel_cost: 27, memory: 110, output: 12 });
    assert_eq!((r.summary.live_before_cleanup, r.summary.live_after_cleanup), (0, 0));
    assert_eq!(output(&r), "{\"int\":\"7\"}\n");
}

/// CT-DET-32 (part): `display_if` costs one step plus the selected branch only (DET.64).
#[test]
fn display_if_charges_the_selected_branch_only() {
    let source = m("fn shade(n: Nat) -> Colour effects [] {\n    \
                    return match exact_less(n, 3) {\n        true => colour(\"blue\");\n        _ => colour(\"navy\");\n    };\n}\n\n\
                    export fn pick(x: Nat, y: Nat) -> Display<Colour> effects [] {\n    \
                    return display_if(approx_less(approx53(x), approx53(y)), colour(\"red\"), shade(x));\n}\n");
    let (_, loaded) = build(&[&source]);
    let options = Options::new(WIDE);

    let red = run(&loaded, "pick", vec![Value::int(1), Value::int(2)], &options);
    assert_eq!(output(&red), "\"red\"\n");
    let rows = ats(&red.summary);
    assert!(rows.contains(&"m.pick:1#value"));
    assert!(!rows.iter().any(|a| a.starts_with("m.pick:2")), "{rows:#?}");
    let condition = rows.iter().position(|&a| a == "m.pick:root#display_if").expect("a display_if row");
    // The condition is a one-byte owned value, released after admission.
    assert_eq!(red.summary.rows[condition].released, 1);
    assert_eq!(rows[condition + 1], "m.pick:1#value");

    let navy = run(&loaded, "pick", vec![Value::int(5), Value::int(2)], &options);
    assert_eq!(output(&navy), "\"navy\"\n");
    let rows = ats(&navy.summary);
    assert!(!rows.contains(&"m.pick:1#value"));
    assert!(rows.contains(&"m.pick:2#call"));
    assert!(rows.contains(&"m.pick:2 > m.shade:0#exact_less"));
    assert!(rows.contains(&"m.pick:2 > m.shade:root#branch"));
    // Both runs share the condition's rows. The else branch takes eight operations (the argument's visit, the call,
    // the scrutinee's three, the match, the arm's literal and the return) where the then branch takes one.
    assert_eq!(navy.summary.used.steps - red.summary.used.steps, 8 - 1);
}

/// Exact sums stay exact end to end and publish in FP.6's canonical forms.
#[test]
fn exact_arithmetic_is_exact_end_to_end() {
    let source = m("export fn sums() -> record { phi: Q5, r: Rat, less: Bool, same: Bool } effects [] {\n    \
                    let one = exact_add(q5(1/2, 1/2), q5(1/2, -1/2));\n    \
                    return record { phi = one, r = exact_add(1/2, 1/3), less = exact_less(1/3, 1/2), same = exact_eq(one, 1) };\n}\n");
    let (_, loaded) = build(&[&source]);
    let r = run(&loaded, "sums", vec![], &Options::new(WIDE));
    assert_eq!(
        output(&r),
        "{\"less\":true,\"phi\":{\"q5\":[\"1\",\"0\",\"1\"]},\"r\":{\"rat\":[\"5\",\"6\"]},\"same\":true}\n"
    );
    assert_eq!((r.summary.live_before_cleanup, r.summary.live_after_cleanup), (0, 0));
}

/// CT-DET-29 (part): draws come from the counter-addressed generator `sha256-counter-draft-1` (DET.61). `rng_init`
/// charges the key input bytes and the 41-byte owner; each draw charges a block and reserves 33 bytes.
#[test]
fn generator_draws_match_the_cursor() {
    let source = "edition \"6cs-draft-1\";\nmodule m requires [random];\n\n\
                  export fn two() -> record { a: Nat, b: Nat } effects [random] {\n    \
                  return record { a = random_draw(), b = random_draw() };\n}\n";
    let (_, loaded) = build(&[source]);
    let seed = [7u8; 32];
    let path = ["test", "two"];
    let options = Options { seed: Some((&seed, &path)), ..Options::new(WIDE) };
    let r = run(&loaded, "two", vec![], &options);

    let (key, _) = rng::stream_key(&seed, &path).unwrap();
    let mut cursor = Cursor::new(key);
    let a = BigInt::from_bytes_be(Sign::Plus, &cursor.draw().unwrap());
    let b = BigInt::from_bytes_be(Sign::Plus, &cursor.draw().unwrap());
    assert_eq!(output(&r), format!("{{\"a\":{{\"int\":\"{a}\"}},\"b\":{{\"int\":\"{b}\"}}}}\n"));
    let first = &r.summary.rows[0];
    assert_eq!((first.at.as_str(), first.kernel_cost, first.memory), ("entry#rng_init", 13 + 32 + 4 + 8 + 7, 41));
    assert_eq!(rng::key_input_len(32, &path), 13 + 32 + 4 + 8 + 7);
    let draw = r.summary.rows.iter().find(|row| row.at == "m.two:0#rng_draw").unwrap();
    let size = Value::int(a.clone()).size();
    assert_eq!((draw.kernel_cost, draw.memory, draw.released), (3, 33, 33 - size));

    // Without a seed, a draw is not granted; with a seed of the wrong length, the run stops at `rng_init` after
    // its charge.
    let none = run(&loaded, "two", vec![], &Options::new(WIDE));
    assert_eq!(none.summary.code, Some(meter::code::NOT_GRANTED));
    assert_eq!(none.summary.stop.as_ref().unwrap().at, "m.two:0#rng_draw");
    let short = [7u8; 31];
    let bad = run(&loaded, "two", vec![], &Options { seed: Some((&short, &path)), ..Options::new(WIDE) });
    assert_eq!(bad.summary.code, Some(meter::code::GENERATOR_INPUT));
    assert_eq!(ats(&bad.summary), ["entry#rng_init"]);
}

#[derive(Default)]
struct Recorder {
    snapshot: Option<(RowTariff, Value)>,
    sunk: Vec<(String, Vec<u8>)>,
}

impl Host for Recorder {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match (request, &self.snapshot) {
            (Request::Snapshot { query: "pieces", .. }, Some((t, _))) => Ok(*t),
            _ => Err(meter::code::NOT_GRANTED),
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match (request, &self.snapshot) {
            (Request::Snapshot { .. }, Some((_, v))) => Ok(v.clone()),
            _ => Err(meter::code::NOT_GRANTED),
        }
    }
    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.sunk.push((format!("{sink:?}"), bytes.to_vec()));
        Ok(())
    }
}

/// CT-DET-31 (part): published bytes are canonical and priced before publication (DET.59). A nested return moves
/// its owned result; returning a borrow first creates a charged owner (DET.58).
#[test]
fn published_bytes_are_canonical_and_nested_returns_move() {
    let source = m("fn make(x: Nat) -> record { v: Nat } effects [] {\n    return record { v = x };\n}\n\n\
                    fn same(r: record { v: Nat }) -> record { v: Nat } effects [] {\n    return r;\n}\n\n\
                    export fn main(x: Nat) -> record { v: Nat } effects [] {\n    return same(make(x));\n}\n\n\
                    export fn show(x: Nat) -> Unit effects [view] {\n    \
                    return publish_view(record { b = x, a = \"é\\n\" });\n}\n");
    let (_, loaded) = build(&[&source]);
    let options = Options::new(WIDE);

    let r = run(&loaded, "main", vec![Value::int(5)], &options);
    assert_eq!(output(&r), "{\"v\":{\"int\":\"5\"}}\n");
    let rows = &r.summary.rows;
    let ops: Vec<_> = rows.iter().map(|r| r.at.as_str()).collect();
    assert_eq!(
        ops,
        [
            "entry#read",
            "entry#call",
            "m.main:0.0#visit",
            "m.main:0#call",
            "m.main:0 > m.make:0#visit",
            "m.main:0 > m.make:root#value",
            "m.main:0 > m.make:root#alias",
            "m.main:0 > m.make:root#return",
            "m.main:root#call",
            "m.main:root > m.same:root#visit",
            "m.main:root > m.same:root#alias",
            "m.main:root > m.same:root#return",
            "m.main:root#return",
            "entry#encode",
            "entry#emit",
        ]
    );
    // `make` returns its owned record without an alias; `same` returns a borrow of its parameter, which it retains
    // first (4 + 2 bytes). Its return releases its frame (16 + 8·2) and the argument it was lent (6).
    assert_eq!((rows[10].memory, rows[11].released), (6, 32 + 6));
    assert_eq!(rows[14].output, 18);

    let mut host = Recorder::default();
    let shown = loaded.run("m", "show", vec![Value::int(5)], &mut host, &options).unwrap();
    assert_eq!(shown.summary.outcome, Outcome::Completed);
    let published = "{\"a\":\"é\\n\",\"b\":{\"int\":\"5\"}}\n";
    assert_eq!(host.sunk, [("View".to_string(), published.as_bytes().to_vec())]);
    let row = shown.summary.rows.iter().find(|r| r.at == "m.show:root#publish_view").unwrap();
    // The publication releases the buffer and the record: header 4 + 8, field b retained (2), field a moved (8).
    assert_eq!((row.output, row.released), (29, 29 + 12 + 2 + 8));
    assert_eq!(shown.summary.used.output, 29 + 5, "the view's bytes and the top-level `null`");
}

/// DET.56: a stop names the operation, the call path, the phase, the scope and the source range of the operation.
#[test]
fn a_stop_names_the_call_path_and_the_source_range() {
    let source = m("fn probe() -> Nat effects [snapshot] {\n    return read_snapshot<Nat>(\"pieces\");\n}\n\n\
                    export fn main() -> Nat effects [snapshot] {\n    return exact_add(probe(), 1);\n}\n");
    let (lowered, loaded) = build(&[&source]);
    let ranges = Ranges(&lowered);
    let options = Options { sources: Some(&ranges), ..Options::new(WIDE) };

    let refused = run(&loaded, "main", vec![], &options);
    assert_eq!((refused.summary.outcome, refused.summary.code), (Outcome::Error, Some(meter::code::NOT_GRANTED)));
    let stop = refused.summary.stop.as_ref().unwrap();
    assert_eq!(
        (stop.at.as_str(), stop.phase, stop.scope.as_str()),
        ("m.main:0 > m.probe:root#kernel", "effect", "root")
    );
    let range = stop.source.as_ref().unwrap();
    assert_eq!(range.digest, sha256_hex(source.as_bytes()));
    assert_eq!(&source[range.start..range.end], "read_snapshot<Nat>(\"pieces\")");
    // The refused row keeps its step (DET.56: a domain error keeps its charges).
    assert_eq!(refused.summary.used.steps, 3);

    let cut = run(&loaded, "main", vec![], &Options { limits: Amounts { steps: 2, ..WIDE }, ..options });
    assert_eq!((cut.summary.outcome, cut.summary.code), (Outcome::CutOff, Some(meter::code::STEPS)));
    let stop = cut.summary.stop.as_ref().unwrap();
    assert_eq!((stop.at.as_str(), stop.phase), ("m.main:0 > m.probe:root#kernel", "steps"));
    assert_eq!(cut.summary.rows.last().map(|r| r.steps), Some(0), "a failed step debits nothing");

    // A granted read reserves the host row's bound and shrinks to the result; a result over its bound stops.
    let mut host = Recorder {
        snapshot: Some((RowTariff { kernel_cost: 5, result_bound: 3 }, Value::int(42))),
        ..Default::default()
    };
    let read = loaded.run("m", "main", vec![], &mut host, &options).unwrap();
    assert_eq!(read.output.as_deref(), Some(&b"{\"int\":\"43\"}\n"[..]));
    let row = read.summary.rows.iter().find(|r| r.at.ends_with("#kernel")).unwrap();
    assert_eq!((row.kernel_cost, row.memory, row.released), (5, 3, 1));
    let mut host = Recorder {
        snapshot: Some((RowTariff { kernel_cost: 5, result_bound: 1 }, Value::int(42))),
        ..Default::default()
    };
    let over = loaded.run("m", "main", vec![], &mut host, &options).unwrap();
    assert_eq!(over.summary.code, Some(code::HOST_EXHAUSTED));
}

/// CT-LG4-01 (part: a cut-off run repeats with the same stop point) and CT-LG4-04 (part: steps).
/// CT-DET-20, PHS.19: unbounded recursion ends at a budget without exhausting the host stack, and repeats
/// identically. The quiet meter keeps no rows, so a deep run's bookkeeping stays proportional to its live owners.
#[test]
fn deep_recursion_stops_cleanly_and_repeats() {
    let (_, loaded) =
        build(&[&m("export fn down(n: Nat) -> Nat effects [] {\n    return down(exact_add(n, 1));\n}\n")]);
    let options = Options { record: false, limits: Amounts { steps: 400_000, ..WIDE }, ..Options::new(WIDE) };
    let first = run(&loaded, "down", vec![Value::int(0)], &options);
    assert_eq!((first.summary.outcome, first.summary.code), (Outcome::CutOff, Some(meter::code::STEPS)));
    assert!(first.summary.rows.is_empty());
    // Two entry operations, then four per level: 99,999 calls, then the visit and the literal of the next level.
    let stop = first.summary.stop.as_ref().unwrap();
    assert!(stop.at.ends_with("m.down:0#exact_add"), "{}", &stop.at[stop.at.len() - 60..]);
    assert_eq!(stop.at.matches(" > ").count(), 99_999);
    assert_eq!(first.summary.used.steps, 400_000);
    let second = run(&loaded, "down", vec![Value::int(0)], &options);
    assert_eq!(first.summary, second.summary);
    assert_eq!(first.summary.live_after_cleanup, 0);
}

/// Counts down the operations it lets through, then interrupts.
struct After {
    left: Cell<u32>,
    why: Interruption,
}

impl Cancel for After {
    fn pending(&self) -> Option<Interruption> {
        match self.left.get() {
            0 => Some(self.why),
            n => {
                self.left.set(n - 1);
                None
            }
        }
    }
}

/// CT-LG4-06 (part: a replaced revision stops a run before its next operation with its own code; the view queue
/// waits for LP3).
/// DET.56, KN7.6: an interruption is observed before each operation, before its step check, and adds no row.
#[test]
fn an_interruption_is_observed_before_each_operation() {
    let (_, loaded) = build(&[&m(ADD)]);
    let after = After { left: Cell::new(4), why: Interruption::ByUser };
    let r = run(&loaded, "main", vec![Value::int(5)], &Options { cancel: Some(&after), ..Options::new(WIDE) });
    assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Cancelled, Some(meter::code::CANCELLED_BY_USER)));
    let stop = r.summary.stop.as_ref().unwrap();
    assert_eq!((stop.at.as_str(), stop.phase), ("m.main:root#call", "interruption"));
    assert_eq!((r.summary.rows.len(), r.summary.used.steps), (4, 4));
    assert_eq!(r.summary.live_after_cleanup, 0);

    let flag = CancelFlag::default();
    flag.cancel(Interruption::RevisionReplaced);
    let r = run(&loaded, "main", vec![Value::int(5)], &Options { cancel: Some(&flag), ..Options::new(WIDE) });
    assert_eq!(r.summary.code, Some(meter::code::CANCELLED_REVISION_REPLACED));
    assert_eq!(r.summary.stop.as_ref().unwrap().at, "entry#read");
    assert!(r.summary.rows.is_empty());
}

/// CT-DET-26 (part): a `let` of a borrow retains it with a charged owner (`alias`); a `let` of an owned value moves
/// it (`visit`). `scope_exit` releases the slot, moves it when the result is the whole slot, and retains a part of it
/// first when the result borrows one (DET.57, DET.58).
#[test]
fn bindings_alias_borrows_and_release_at_scope_exit() {
    let source = m("export fn keep(x: Nat) -> Nat effects [] {\n    let a = x;\n    let b = exact_add(a, 1);\n    return a;\n}\n\n\
                    export fn part(x: Nat) -> Nat effects [] {\n    let r = record { v = x };\n    return r.v;\n}\n");
    let (_, loaded) = build(&[&source]);
    let options = Options::new(WIDE);

    // `keep`'s frame: one parameter, two `let` slots and three temporaries, 16 + 8·6 bytes.
    let r = run(&loaded, "keep", vec![Value::int(5)], &options);
    assert_eq!(output(&r), "{\"int\":\"5\"}\n");
    assert_rows(
        &r.summary,
        &[
            ("entry#read", 1, 12, 2, 0, 0),
            ("entry#call", 1, 0, 64, 0, 0),
            ("m.keep:0#visit", 1, 0, 0, 0, 0),
            ("m.keep:root#alias", 1, 0, 2, 0, 0),
            ("m.keep:1.0.0#visit", 1, 0, 0, 0, 0),
            ("m.keep:1.0.1#value", 1, 0, 2, 0, 0),
            ("m.keep:1.0#exact_add", 1, 2, 2, 0, 2),
            ("m.keep:1#visit", 1, 0, 0, 0, 0),
            ("m.keep:1.1#visit", 1, 0, 0, 0, 0),
            ("m.keep:1#scope_exit", 1, 0, 0, 0, 2),
            ("m.keep:root#scope_exit", 1, 0, 0, 0, 0),
            ("m.keep:root#return", 1, 0, 0, 0, 64 + 2),
            ("entry#encode", 1, 13, 13, 0, 1),
            ("entry#emit", 1, 0, 0, 12, 12 + 2),
        ],
    );

    let r = run(&loaded, "part", vec![Value::int(5)], &options);
    assert_eq!(output(&r), "{\"int\":\"5\"}\n");
    let ops = ats(&r.summary);
    let exit = ops.iter().position(|&a| a == "m.part:root#scope_exit").unwrap();
    assert_eq!(ops[exit - 1], "m.part:root#alias");
    assert_eq!(r.summary.rows[exit - 1].memory, 2);
    assert_eq!(r.summary.rows[exit].released, 4 + 2, "the record's header and its retained field");
}

/// Closures capture in declaration order, through enclosing closures, with one `alias` per capture.
#[test]
fn closures_capture_in_declaration_order() {
    let source = m("export fn sum(x: Nat) -> Nat effects [] {\n    let a = 1;\n    let b = 1000;\n    \
                    let f = fn(z: Nat) -> Nat { return exact_add(z, exact_add(b, a)); };\n    return f(x);\n}\n\n\
                    export fn nested() -> Nat effects [] {\n    let a = 7;\n    \
                    let g = fn() -> Nat { let h = fn() -> Nat { return a; }; return h(); };\n    return g();\n}\n");
    let (_, loaded) = build(&[&source]);
    let options = Options::new(WIDE);
    let r = run(&loaded, "sum", vec![Value::int(5)], &options);
    assert_eq!(output(&r), "{\"int\":\"1006\"}\n");
    let closure: Vec<_> =
        r.summary.rows.iter().filter(|r| r.at.starts_with("m.sum:1.1.0#")).map(|r| (r.at.as_str(), r.memory)).collect();
    // `a` (1, two bytes) before `b` (1000, three bytes), although the body names `b` first.
    assert_eq!(closure, [("m.sum:1.1.0#value", 32), ("m.sum:1.1.0#alias", 2), ("m.sum:1.1.0#alias", 3)]);
    // The closure's frame: one parameter, two captures and its body's temporaries.
    assert!(r.summary.rows.iter().any(|r| r.at == "m.sum:1.1.1#call"));
    assert!(r.summary.rows.iter().any(|r| r.at.starts_with("m.sum:1.1.1 > m.sum:1.1.0.0")));

    let r = run(&loaded, "nested", vec![], &options);
    assert_eq!(output(&r), "{\"int\":\"7\"}\n");
}

/// The pinned M7 fixture runs end to end. It is a syntax fixture: the checker refuses only its `ratio`, whose field
/// read erases the display qualifier of the record (LG2.19), so that function is not run.
#[test]
fn the_canonical_fixture_runs() {
    let lowered = lowered(&[include_str!("../../c600-syntax/tests/fixtures/canonical.6cs")]);
    let report = check(&lowered.program).unwrap();
    assert!(report.diagnostics.iter().all(|d| d.function == "ratio"), "{:#?}", report.diagnostics);
    let loaded = load(&lowered.program).unwrap();
    let options = Options::new(WIDE);
    let mut host = NoHost;
    let choose = loaded.run("pinned", "choose", vec![Value::int(1), Value::int(2)], &mut host, &options).unwrap();
    assert_eq!(choose.output.as_deref(), Some(&b"[1,-2]\n"[..]));
    let other = loaded.run("pinned", "choose", vec![Value::int(2), Value::int(1)], &mut host, &options).unwrap();
    assert_eq!(other.output.as_deref(), Some(&b"[3]\n"[..]));
    let values = loaded.run("pinned", "values", vec![], &mut host, &options).unwrap();
    let expected =
        "{\"label\":\"λ \\\"quoted\\\"\\n\",\"phi\":{\"q5\":[\"1\",\"1\",\"2\"]},\"ratio\":{\"rat\":[\"-3\",\"4\"]},\
                    \"scale\":{\"f64\":\"3fc0000000000000\"}}\n";
    assert_eq!(output(&values), expected);
    assert_eq!(values.summary.live_before_cleanup, 0);
}

fn refusal(program: &Program) -> (&'static str, String) {
    let r = load(program).expect_err("refused");
    (r.code, r.detail)
}

fn one(body: Expr) -> Program {
    program(vec![module(
        "m",
        vec![c600_ir::build::helper("f", vec![param("x", exact(Ty::Nat))], exact(Ty::Nat), body)],
    )])
}

/// The loader refuses IR it cannot run (`LG4:input.ir_version_unsupported`, `LG4:input.ir_rejected`), and a run
/// refuses an unknown entry, a wrong argument count and an argument without a canonical form.
#[test]
fn load_refuses_ir_it_cannot_run() {
    let mut other = one(var("x"));
    other.ir_version = "c600-ir-draft-0".into();
    assert_eq!(refusal(&other).0, code::IR_VERSION_UNSUPPORTED);
    for body in [
        var("y"),
        Expr::Call { callee: Callee::Unresolved { name: "g".into(), result: exact(Ty::Nat) }, args: vec![] },
        prim(Prim::ToExact, vec![var("x")]),
        prim(Prim::ToApprox(32), vec![var("x")]),
        prim(Prim::ExactAdd, vec![var("x")]),
        call("f", vec![]),
        call("g", vec![var("x")]),
        record(vec![("a", nat(1)), ("a", nat(2))]),
        Expr::Lit(Lit::Nat("01".into())),
        Expr::Match {
            scrutinee: Box::new(var("x")),
            arms: vec![(Lit::Int("-0".into()), nat(1))],
            default: Box::new(nat(2)),
        },
    ] {
        assert_eq!(refusal(&one(body.clone())).0, code::IR_REJECTED, "{body:?}");
    }
    let hidden = program(vec![
        module("a", vec![c600_ir::build::helper("g", vec![], exact(Ty::Nat), nat(1))]),
        module("b", vec![c600_ir::build::helper("f", vec![], exact(Ty::Nat), call_in("a", "g", vec![]))]),
    ]);
    assert!(refusal(&hidden).1.contains("not exported"));
    let twice = program(vec![module(
        "m",
        vec![
            c600_ir::build::helper("f", vec![], exact(Ty::Nat), nat(1)),
            c600_ir::build::helper("f", vec![], exact(Ty::Nat), nat(2)),
        ],
    )]);
    assert_eq!(refusal(&twice).0, code::IR_REJECTED);

    let loaded = load(&one(var("x"))).unwrap();
    let options = Options::new(WIDE);
    let entry =
        |name: &str, args: Vec<Value>| loaded.run("m", name, args, &mut NoHost, &options).map(|_| ()).unwrap_err();
    assert_eq!(entry("g", vec![Value::int(1)]).code, code::IR_REJECTED);
    assert_eq!(entry("f", vec![]).code, code::IR_REJECTED);
    assert_eq!(entry("f", vec![Value::Fn(0)]).code, code::NONCANONICAL_VALUE);
    let ok = loaded.run("m", "f", vec![Value::int(3)], &mut NoHost, &options).unwrap();
    assert_eq!(ok.output.as_deref(), Some(&b"{\"int\":\"3\"}\n"[..]));
}
