//! Child scopes at run time (DET.60; LP2 draft until the IR freeze, PHS.49). `child` costs one step in the enclosing
//! scopes. Its body then runs in a scope of its own under the effective caps: each requested cap cut to what every
//! enclosing scope has left, an omitted cap being that remainder. The body's `call` and `return` are charged in the
//! child, its frame borrows the enclosing frame's slots without charge, and its result moves to the parent. A stop of
//! the child's own scope, a cut-off or a domain error, unwinds the child: its owners and reservations go uncharged, and
//! its stop branch runs in the parent with the outcome as a status, `cut_off` or `error`, after a `status` row of four
//! bytes; a stop branch without a parameter takes neither. A limit of an enclosing scope stops that scope, the root's
//! the whole run, and an interruption stops the whole run. Each child is recorded with its requested and effective
//! caps, its use, its outcome and the memory it returned.
//! An `rng_init` in a child streams from the run's path, then the child's invocation index under its parent, counted
//! from 0 per parent (DET.61). The source half is `c600-syntax`'s `child_scopes.rs`; the frozen child traces run as
//! programs in `det65_programs.rs`.

mod conformance;

use std::cell::Cell;

use c600_run::interp::{Cancel, NoHost, Options, Run};
use c600_run::meter::code::{CANCELLED_BY_USER, POINTWISE_FAILED, STEPS};
use c600_run::meter::{Amounts, ChildRecord, Interruption, Outcome, Row};
use c600_run::rng::{stream_key, Cursor};
use c600_run::value::{equal, Value};
use conformance::*;
use num_bigint::{BigInt, Sign};

const SEED: [u8; 32] = [7; 32];

fn run_with(declarations: &str, options: &Options) -> Run {
    let (_, loaded) = build(&[&m_requiring("random", declarations)]);
    loaded.run("m", "f", vec![], &mut NoHost, options).expect("a run")
}

fn run(declarations: &str) -> Run {
    run_with(declarations, &Options::new(WIDE))
}

/// The run completed with `want` and released everything.
fn returns(run: &Run, want: Value) {
    let s = &run.summary;
    assert_eq!((s.outcome, s.code), (Outcome::Completed, None), "{:?}", s.stop);
    let value = run.value.as_ref().expect("a returned value");
    assert!(equal(value, &want), "returned {value:?}, want {want:?}");
    assert_eq!(s.live_after_cleanup, 0);
}

fn charges(row: &Row) -> [u128; 5] {
    [row.steps, row.kernel_cost, row.memory, row.output, row.released]
}

/// The charges of the rows at `at`, in order.
fn rows(run: &Run, at: &str) -> Vec<[u128; 5]> {
    run.summary.rows.iter().filter(|r| r.at == at).map(charges).collect()
}

/// The root's use and live memory after the row at `at`, as its rows add up.
fn after(run: &Run, at: &str) -> Amounts {
    let end = run.summary.rows.iter().position(|r| r.at == at).unwrap_or_else(|| panic!("no row {at}"));
    let mut used = Amounts::default();
    let mut live = 0;
    for r in &run.summary.rows[..=end] {
        used.steps += r.steps;
        used.kernel_cost += r.kernel_cost;
        used.output += r.output;
        live = live + r.memory - r.released;
    }
    used.memory = live;
    used
}

fn amounts(steps: u128, kernel_cost: u128, memory: u128, output: u128) -> Amounts {
    Amounts { steps, kernel_cost, memory, output }
}

/// `f` with the budget effect, returning a child with the given caps, body and stop branch, both of result `Nat`.
fn child(caps: &str, body: &str, otherwise: &str) -> String {
    format!(
        "export fn f() -> Nat effects [budget] {{\n    return child(record {{ {caps} }}, fn() -> Nat {{ {body} }}, \
         fn(s: Status) -> Nat {{ {otherwise} }});\n}}\n"
    )
}

/// A stop branch that tells the two statuses apart.
const STATUS: &str =
    "return match s {\n        tag(\"cut_off\") => 10;\n        tag(\"error\") => 20;\n        _ => 0;\n    };";

/// CT-DET-26 (part: a moved child result on RT1; not run: RT2 and RT3 (no owner yet)): a completed child's body runs
/// in its own scope, its `call` and `return` charged there, and borrows the parent's `x` without a row of its own. Its
/// result moves to the parent, so the child returns its owner and keeps nothing live; the record keeps the requested
/// and effective caps, the child's use and the returned memory.
#[test]
fn a_completed_child_moves_its_result_to_the_parent() {
    let r = run("export fn f() -> Nat effects [budget] {\n    let x = 5;\n    return child(record { steps = 100 }, \
                 fn() -> Nat { return exact_add(x, 1); }, fn(s: Status) -> Nat { return 0; });\n}\n");
    returns(&r, Value::int(6));
    assert_eq!(rows(&r, "m.f:1#child"), [[1, 0, 0, 0, 0]], "one step in the parent");
    let entry = after(&r, "m.f:1#child");
    let left = amounts(100, WIDE.kernel_cost - entry.kernel_cost, WIDE.memory - entry.memory, WIDE.output);
    // The body's frame: no parameter, the borrowed `x` and two operand slots.
    assert_eq!(rows(&r, "m.f:1#call"), [[1, 0, 40, 0, 0]]);
    assert_eq!(rows(&r, "m.f:1 > m.f:1.0#return"), [[1, 0, 0, 0, 40]]);
    assert!(r.summary.rows.iter().all(|row| !row.at.ends_with("#alias")), "a borrow is not charged");
    let record = ChildRecord {
        at: "m.f:1".into(),
        requested: Amounts { steps: 100, ..left },
        effective: left,
        outcome: Outcome::Completed,
        code: None,
        stop: None,
        used: amounts(5, 2, 44, 0),
        returned_memory: 2,
        live_after_return: 0,
    };
    assert_eq!(r.summary.children, [record]);
}

/// CT-DET-26 (part: a borrowed child result on RT1): a body that returns a borrowed text gives it an owner in the
/// child, with an `alias` row of its size; the owner moves to the parent with the result.
#[test]
fn a_borrowed_result_gets_an_owner_in_the_child() {
    let r = run("export fn f() -> Text effects [budget] {\n    let t = \"abc\";\n    return child(record { }, \
                 fn() -> Text { return t; }, fn(s: Status) -> Text { return \"\"; });\n}\n");
    returns(&r, Value::text("abc"));
    assert_eq!(rows(&r, "m.f:1 > m.f:1.0#alias"), [[1, 0, 12, 0, 0]]);
    let child = &r.summary.children[0];
    assert_eq!((child.returned_memory, child.live_after_return), (12, 0));
}

/// A body cut off by its own cap is unwound: its frame and values go without release rows, the record keeps its stop,
/// and the stop branch runs in the parent with `cut_off`, after a `status` row of four bytes and its own `call`.
#[test]
fn a_cut_off_child_runs_its_stop_branch() {
    let r = run(&child("steps = 2", "return exact_add(exact_add(1, 2), 3);", STATUS));
    returns(&r, Value::int(10));
    let denied = "m.f:root > m.f:0.0.1#value";
    assert_eq!(rows(&r, denied), [[0, 0, 0, 0, 0]], "the denied step");
    assert_eq!(rows(&r, "m.f:root#status"), [[1, 0, 4, 0, 0]]);
    assert_eq!(rows(&r, "m.f:root#call").len(), 2, "the body's call and the stop branch's");
    let child = &r.summary.children[0];
    let stop = child.stop.as_ref().expect("a stop");
    assert_eq!((child.outcome, child.code), (Outcome::CutOff, Some(STEPS)));
    assert_eq!((stop.at.as_str(), stop.phase, stop.scope.as_str()), (denied, "steps", "m.f:root"));
    assert_eq!((child.used.steps, child.effective.steps, child.live_after_return), (2, 2, 0));
}

/// A stop branch that names no status runs after no `status` row, in a frame without a parameter slot.
#[test]
fn a_stop_branch_without_a_status_takes_no_status_row() {
    let r = run("export fn f() -> Nat effects [budget] {\n    return child(record { steps = 2 }, \
                 fn() -> Nat { return exact_add(exact_add(1, 2), 3); }, fn() -> Nat { return 7; });\n}\n");
    returns(&r, Value::int(7));
    assert!(rows(&r, "m.f:root#status").is_empty(), "no status");
    let calls = rows(&r, "m.f:root#call");
    assert_eq!(calls.len(), 2, "the body's call and the stop branch's");
    assert_eq!(calls[1], [1, 0, 16, 0, 0], "a frame without slots");
    assert_eq!(r.summary.children[0].outcome, Outcome::CutOff);
}

/// A domain error of the body is the child's own: it unwinds the child, and the stop branch runs with `error`.
#[test]
fn a_child_absorbs_its_own_error() {
    let r = run(&child("", "let a = assert(exact_eq(1, 2));\n        return 1;", STATUS));
    returns(&r, Value::int(20));
    let child = &r.summary.children[0];
    let stop = child.stop.as_ref().expect("a stop");
    assert_eq!((child.outcome, child.code), (Outcome::Error, Some(POINTWISE_FAILED)));
    assert_eq!((stop.phase, stop.scope.as_str(), child.live_after_return), ("effect", "m.f:root", 0));
}

/// Each requested cap is cut to what the root has left after the `child` step; an omitted cap is that remainder, both
/// requested and effective, so it creates no allowance.
#[test]
fn caps_are_cut_to_what_the_parent_has_left() {
    let limits = amounts(40, 1000, 500, 100);
    let r = run_with(&child("steps = 1000, memory = 64", "return 1;", "return 0;"), &Options::new(limits));
    returns(&r, Value::int(1));
    let entry = after(&r, "m.f:root#child");
    let left = amounts(40 - entry.steps, 1000 - entry.kernel_cost, 500 - entry.memory, 100 - entry.output);
    let child = &r.summary.children[0];
    assert_eq!(child.requested, Amounts { steps: 1000, memory: 64, ..left });
    assert_eq!(child.effective, Amounts { memory: 64, ..left });
}

/// CT-DET-27 (part: child/root precedence on RT1; not run: RT2 and RT3 (no owner yet)): when the root's limit and the
/// child's cap, cut to it, fail together, the root is checked first, so the whole run stops and no stop branch runs.
/// The child is still unwound and recorded with the same stop.
#[test]
fn a_root_limit_stops_the_whole_run() {
    let source = child("steps = 100", "return exact_add(exact_add(1, 2), 3);", STATUS);
    let probe = run(&source);
    let entry = after(&probe, "m.f:root#child");
    let r = run_with(&source, &Options::new(Amounts { steps: entry.steps + 2, ..WIDE }));
    let s = &r.summary;
    assert_eq!((s.outcome, s.code), (Outcome::CutOff, Some(STEPS)));
    let stop = s.stop.as_ref().expect("a stop");
    assert_eq!((stop.at.as_str(), stop.scope.as_str()), ("m.f:root > m.f:0.0.1#value", "root"));
    assert!(rows(&r, "m.f:root#status").is_empty(), "no stop branch");
    let child = &s.children[0];
    assert_eq!((child.outcome, child.code, child.stop.as_ref()), (Outcome::CutOff, Some(STEPS), Some(stop)));
    assert_eq!((child.effective.steps, child.live_after_return, s.live_after_cleanup), (2, 0, 0));
}

/// A stop of the stop branch is the parent's: here the root has one step left after the child's own cut-off, which
/// the `status` row takes, so the stop branch's `call` stops the run.
#[test]
fn a_stop_branch_stops_in_the_parent() {
    let source = child("steps = 2", "return exact_add(exact_add(1, 2), 3);", STATUS);
    let probe = run(&source);
    let before = after(&probe, "m.f:root > m.f:0.0.1#value");
    let r = run_with(&source, &Options::new(Amounts { steps: before.steps + 1, ..WIDE }));
    let stop = r.summary.stop.as_ref().expect("a stop");
    assert_eq!((r.summary.outcome, stop.at.as_str(), stop.scope.as_str()), (Outcome::CutOff, "m.f:root#call", "root"));
    assert_eq!(rows(&r, "m.f:root#status"), [[1, 0, 4, 0, 0]]);
    assert_eq!(r.summary.children.len(), 1, "the child closed before its stop branch");
    assert_eq!(r.summary.live_after_cleanup, 0);
}

const NESTED: &str = "export fn f() -> Nat effects [budget] {\n    \
                      return child(record { steps = OUTER }, fn() -> Nat {\n        \
                      return child(record { steps = INNER }, fn() -> Nat { return exact_add(exact_add(1, 2), 3); }, \
                      fn(t: Status) -> Nat { return 40; });\n    }, fn(s: Status) -> Nat { return 30; });\n}\n";

fn nested(outer: u128, inner: u128) -> Run {
    run(&NESTED.replace("OUTER", &outer.to_string()).replace("INNER", &inner.to_string()))
}

/// CT-DET-27 (part: an outer child before an inner one on RT1): nested children each take a cap cut to every
/// ancestor's remainder. When the outer cap and the inner one fail together, the outer child stops: the inner child
/// unwinds without its stop branch, and the outer one's runs. An inner cap that fails alone runs the inner stop branch
/// inside the outer child, which completes.
#[test]
fn an_outer_cap_stops_the_outer_child() {
    let r = nested(4, 100);
    returns(&r, Value::int(30));
    let [inner, outer] = r.summary.children.as_slice() else { panic!("{:#?}", r.summary.children) };
    assert_eq!((inner.at.as_str(), outer.at.as_str()), ("m.f:root > m.f:0", "m.f:root"));
    assert_eq!((inner.requested.steps, inner.effective.steps), (100, 4 - 2), "the outer call and child steps");
    let stop = outer.stop.as_ref().expect("a stop");
    assert_eq!(stop.scope, "m.f:root");
    assert_eq!((inner.outcome, inner.stop.as_ref()), (Outcome::CutOff, Some(stop)));
    assert_eq!((outer.live_after_return, inner.live_after_return), (0, 0));

    let r = nested(100, 2);
    returns(&r, Value::int(40));
    let [inner, outer] = r.summary.children.as_slice() else { panic!("{:#?}", r.summary.children) };
    assert_eq!(inner.stop.as_ref().map(|s| s.scope.as_str()), Some("m.f:root > m.f:0"));
    assert_eq!((inner.outcome, outer.outcome, outer.returned_memory), (Outcome::CutOff, Outcome::Completed, 2));
}

/// Interrupts the run at the `n`th operation that asks.
struct At(Cell<usize>);

impl Cancel for At {
    fn pending(&self) -> Option<Interruption> {
        let left = self.0.get();
        self.0.set(left.wrapping_sub(1));
        (left == 1).then_some(Interruption::ByUser)
    }
}

/// An interruption inside a child stops the whole run: the child unwinds without its stop branch.
#[test]
fn an_interruption_in_a_child_stops_the_whole_run() {
    // The entry's call, the child's step, the body's call, then the body's first operation.
    let at = At(Cell::new(4));
    let options = Options { cancel: Some(&at), ..Options::new(WIDE) };
    let r = run_with(&child("steps = 8", "return exact_add(exact_add(1, 2), 3);", STATUS), &options);
    let s = &r.summary;
    assert_eq!((s.outcome, s.code), (Outcome::Cancelled, Some(CANCELLED_BY_USER)));
    assert_eq!(s.stop.as_ref().map(|p| p.at.as_str()), Some("m.f:root > m.f:0.0.0#value"));
    assert!(rows(&r, "m.f:root#status").is_empty(), "no stop branch");
    assert_eq!((s.children[0].outcome, s.live_after_cleanup), (Outcome::Cancelled, 0));
}

/// Block 0 of the stream at `path` under [`SEED`], as the natural a draw returns.
fn block(path: &[&str]) -> Value {
    nth_block(path, 0)
}

/// Block `n` of the stream at `path` under [`SEED`], as the natural a draw returns.
fn nth_block(path: &[&str], n: usize) -> Value {
    let (key, _) = stream_key(&SEED, path).expect("a valid stream");
    let mut cursor = Cursor::new(key);
    let mut bytes = cursor.draw().expect("a block");
    for _ in 0..n {
        bytes = cursor.draw().expect("a block");
    }
    Value::int(BigInt::from_bytes_be(Sign::Plus, &bytes))
}

/// A child borrows its parent's inputs (DET.60), and a borrowed generator draws from its owner's cursor (DET.61). A
/// draw the child made before its cut-off stays made: the unwind releases the child's owners and rolls no cursor
/// back, so the stop branch, then the parent, draw the next blocks of the parent's stream.
#[test]
fn a_cut_off_child_keeps_the_draws_it_made() {
    let source = "export fn f() -> record { a: Nat, b: Nat } effects [budget, random] {\n    \
                  let g = rng_init(\"s\");\n    \
                  let a = child(record { steps = 4 }, fn() -> Nat {\n        \
                  let x = rng_draw(g);\n        \
                  return exact_add(exact_add(exact_add(x, 1), 2), 3);\n    \
                  }, fn(s: Status) -> Nat { return rng_draw(g); });\n    \
                  return record { a = a, b = rng_draw(g) };\n}\n";
    let path = ["root"];
    let options = Options { seed: Some((&SEED, &path)), ..Options::new(WIDE) };
    let r = run_with(source, &options);
    let stream = ["root", "s"];
    returns(&r, Value::record(vec![("a", nth_block(&stream, 1)), ("b", nth_block(&stream, 2))]));
    let child = &r.summary.children[0];
    assert_eq!((child.outcome, child.code), (Outcome::CutOff, Some(STEPS)));
}

/// CT-DET-29 (part: child stream paths on RT1; not run: RT2 and RT3 (no owner yet)): a child's `rng_init` streams from
/// the run's path, then the child's invocation index under its parent, counted from 0 for each parent, then its own
/// segments (DET.61). A child draws no new seed: `random_draw` in a child draws the run's generator, and a stop branch,
/// which runs in the parent, streams from the parent's path.
#[test]
fn child_streams_take_the_invocation_path() {
    let draw = "return rng_draw(rng_init(\"s\"));";
    let source = format!(
        "export fn f() -> record {{ a: Nat, b: Nat, c: Nat, d: Nat, e: Nat }} effects [budget, random] {{\n    \
         let a = child(record {{ }}, fn() -> Nat {{ {draw} }}, fn(s: Status) -> Nat {{ return 0; }});\n    \
         let b = child(record {{ }}, fn() -> Nat {{\n        \
         return child(record {{ }}, fn() -> Nat {{ {draw} }}, fn(t: Status) -> Nat {{ return 0; }});\n    }}, \
         fn(s: Status) -> Nat {{ return 0; }});\n    \
         let c = child(record {{ }}, fn() -> Nat {{ return random_draw(); }}, \
         fn(s: Status) -> Nat {{ return 0; }});\n    \
         let e = child(record {{ steps = 1 }}, fn() -> Nat {{ {draw} }}, fn(s: Status) -> Nat {{ {draw} }});\n    \
         return record {{ a = a, b = b, c = c, d = rng_draw(rng_init(\"s\")), e = e }};\n}}\n"
    );
    let path = ["root"];
    let options = Options { seed: Some((&SEED, &path)), ..Options::new(WIDE) };
    let r = run_with(&source, &options);
    let want = Value::record(vec![
        ("a", block(&["root", "0", "s"])),
        ("b", block(&["root", "1", "0", "s"])),
        ("c", block(&["root"])),
        ("d", block(&["root", "s"])),
        ("e", block(&["root", "s"])),
    ]);
    returns(&r, want);
    let ats: Vec<&str> = r.summary.children.iter().map(|c| c.at.as_str()).collect();
    assert_eq!(ats.len(), 5, "{ats:?}");
}
