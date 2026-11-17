//! Loops at run time (DET.55; LP2 draft until the IR freeze, PHS.49). `repeat(n, init, fn(acc: T) -> T { ... })`
//! runs in its function's frame: the initial value fills a slot of the frame, and the body runs `n` times, each time
//! with `acc` bound to the value so far, its result refilling the slot. Every loop test is a one-step row of its own,
//! the final false one included, and the loop ends with its `scope_exit`, which releases the count and moves the
//! value so far out. An operation inside an iteration names it in its locator, `label:path#body:k > `. The source
//! half is `c600-syntax`'s `loops.rs`; the frozen trace `final-loop-test` runs as a program in `det65_programs.rs`.

mod conformance;

use c600_check::check;
use c600_run::interp::{code, load, NoHost, Options, Run};
use c600_run::meter::code::STEPS;
use c600_run::meter::{Amounts, Outcome, Row};
use c600_run::value::{equal, Value};
use conformance::*;

fn run_with(declarations: &str, options: &Options) -> Run {
    let (_, loaded) = build(&[&m(declarations)]);
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

/// The steps of the rows before the first row at `at`.
fn steps_before(run: &Run, at: &str) -> u128 {
    let end = run.summary.rows.iter().position(|r| r.at == at).unwrap_or_else(|| panic!("no row {at}"));
    run.summary.rows[..end].iter().map(|r| r.steps).sum()
}

/// `f` returning a loop over `Nat` from 0 that adds 2 per iteration.
fn add_two(count: u32) -> String {
    format!(
        "export fn f() -> Nat effects [] {{\n    \
         return repeat({count}, 0, fn(acc: Nat) -> Nat {{ return exact_add(acc, 2); }});\n}}\n"
    )
}

/// CT-DET-28 (synthetic, part: loop tests on RT1; not run: flow checks, since the IR has no flow check, and RT2 and
/// RT3 (no owner yet)): a loop runs its body as many times as its count says, each run reading the value so far as
/// `acc`. Each loop test is a one-step row of its own, the final false one included; the rows of iteration k are
/// located below `#body:k`, and its owned result refills the slot with a `visit` that releases the value it replaces.
/// The loop's `scope_exit` releases the count, and its value is the last one.
#[test]
fn a_loop_runs_its_body_count_times() {
    let r = run(&add_two(2));
    returns(&r, Value::int(4));
    let order: Vec<&str> = r.summary.rows.iter().map(|row| row.at.as_str()).collect();
    assert_eq!(
        order,
        [
            "entry#call",
            "m.f:0#value",
            "m.f:1#value",
            "m.f:root#visit",
            "m.f:root#loop_test:0",
            "m.f:root#body:0 > m.f:2.0#visit",
            "m.f:root#body:0 > m.f:2.1#value",
            "m.f:root#body:0 > m.f:2#exact_add",
            "m.f:root#body:0 > m.f:root#visit",
            "m.f:root#loop_test:1",
            "m.f:root#body:1 > m.f:2.0#visit",
            "m.f:root#body:1 > m.f:2.1#value",
            "m.f:root#body:1 > m.f:2#exact_add",
            "m.f:root#body:1 > m.f:root#visit",
            "m.f:root#loop_test:2",
            "m.f:root#scope_exit",
            "m.f:root#return",
            "entry#emit",
        ]
    );
    for k in 0..3 {
        assert_eq!(rows(&r, &format!("m.f:root#loop_test:{k}")), [[1, 0, 0, 0, 0]], "test {k}");
    }
    assert_eq!(rows(&r, "m.f:root#visit"), [[1, 0, 0, 0, 0]], "the owned initial value moves into the slot");
    for (k, old) in [(0, 0), (1, 2)] {
        let refill = rows(&r, &format!("m.f:root#body:{k} > m.f:root#visit"));
        assert_eq!(refill, [[1, 0, 0, 0, Value::int(old).size()]], "iteration {k} replaces {old}");
    }
    assert_eq!(rows(&r, "m.f:root#scope_exit"), [[1, 0, 0, 0, Value::int(2).size()]], "the count goes at the exit");
}

/// A count of zero runs no iteration: the loop is its first test, false, and its `scope_exit`, and the initial value
/// is the loop's value.
#[test]
fn a_count_of_zero_tests_once() {
    let r = run("export fn f() -> Nat effects [] {\n    \
                 return repeat(0, 5, fn(acc: Nat) -> Nat { return exact_add(acc, 1); });\n}\n");
    returns(&r, Value::int(5));
    assert_eq!(rows(&r, "m.f:root#loop_test:0"), [[1, 0, 0, 0, 0]]);
    assert!(rows(&r, "m.f:root#loop_test:1").is_empty(), "one test");
    assert!(r.summary.rows.iter().all(|row| !row.at.contains("#body:")), "no iteration");
    assert_eq!(rows(&r, "m.f:root#scope_exit"), [[1, 0, 0, 0, Value::int(0).size()]]);
}

/// The final false test takes a step of its own (DET.55): with the steps used before it, the run stops there with a
/// denied row, before the loop's `scope_exit`; with one step more, it stops at the `scope_exit`. The cleanup releases
/// the count and the value so far.
#[test]
fn the_final_test_takes_a_step_of_its_own() {
    let program = add_two(2);
    let wide = run(&program);
    let before = steps_before(&wide, "m.f:root#loop_test:2");
    for (steps, at) in [(before, "m.f:root#loop_test:2"), (before + 1, "m.f:root#scope_exit")] {
        let cut = run_with(&program, &Options::new(Amounts { steps, ..WIDE }));
        let s = &cut.summary;
        assert_eq!((s.outcome, s.code), (Outcome::CutOff, Some(STEPS)), "{steps} steps");
        let stop = s.stop.as_ref().expect("a stop");
        assert_eq!((stop.at.as_str(), stop.phase, stop.scope.as_str()), (at, "steps", "root"));
        assert_eq!(rows(&cut, at), [[0, 0, 0, 0, 0]], "the denied row");
        assert_eq!(s.live_after_cleanup, 0);
    }
}

/// A body whose result is the loop's slot itself refills it with a `visit` that releases nothing. A body whose result
/// borrows a value outside the slot refills it with an `alias`, a new owner of its size; the value it replaces is
/// released in the same row, after admission (DET.56), so it cannot fund its replacement.
#[test]
fn a_refill_keeps_its_slot_or_aliases_a_borrow() {
    let same = run("export fn f() -> Nat effects [] {\n    \
                    return repeat(2, 7, fn(acc: Nat) -> Nat { return acc; });\n}\n");
    returns(&same, Value::int(7));
    for k in 0..2 {
        assert_eq!(rows(&same, &format!("m.f:root#body:{k} > m.f:root#visit")), [[1, 0, 0, 0, 0]], "iteration {k}");
    }
    let borrowed = "export fn f() -> Text effects [] {\n    let t = \"abc\";\n    \
                    return repeat(2, \"\", fn(acc: Text) -> Text { return t; });\n}\n";
    let r = run(borrowed);
    returns(&r, Value::text("abc"));
    let (empty, abc) = (Value::text("").size(), Value::text("abc").size());
    assert_eq!(rows(&r, "m.f:1#body:0 > m.f:1#alias"), [[1, 0, abc, 0, empty]]);
    assert_eq!(rows(&r, "m.f:1#body:1 > m.f:1#alias"), [[1, 0, abc, 0, abc]]);
    // At the second refill the old `abc` is still live: memory for a third copy is needed, not just a second.
    let before: Vec<&Row> = r.summary.rows.iter().take_while(|row| row.at != "m.f:1#body:1 > m.f:1#alias").collect();
    let live: u128 =
        before.iter().map(|row| row.memory).sum::<u128>() - before.iter().map(|row| row.released).sum::<u128>();
    let short = run_with(borrowed, &Options::new(Amounts { memory: live + abc - 1, ..WIDE }));
    let stop = short.summary.stop.as_ref().expect("a stop");
    assert_eq!((stop.at.as_str(), stop.phase), ("m.f:1#body:1 > m.f:1#alias", "memory"));
    assert_eq!(short.summary.live_after_cleanup, 0);
}

/// Loops nest: an operation names every running iteration, outermost first, and each loop has a slot of its own. The
/// inner loop's initial value borrows the outer `acc`, so it fills its slot with an `alias`.
#[test]
fn nested_loops_name_each_iteration() {
    let r = run("export fn f() -> Nat effects [] {\n    return repeat(2, 0, fn(acc: Nat) -> Nat {\n        \
                 return repeat(3, acc, fn(inner: Nat) -> Nat { return exact_add(inner, 1); });\n    });\n}\n");
    returns(&r, Value::int(6));
    assert_eq!(rows(&r, "m.f:root#body:1 > m.f:2#loop_test:3"), [[1, 0, 0, 0, 0]], "the inner loop's final test");
    assert_eq!(rows(&r, "m.f:root#body:1 > m.f:2#body:2 > m.f:2.2#exact_add").len(), 1);
    assert_eq!(rows(&r, "m.f:root#body:0 > m.f:2#alias").len(), 1, "the inner initial value borrows `acc`");
    let tests = r.summary.rows.iter().filter(|row| row.at.contains("loop_test:")).count();
    assert_eq!(tests, 3 + 2 * 4, "the outer loop's three tests and each inner loop's four");
}

/// A loop's body may call: the callee's rows name the iteration before the call site. A loop in a child scope runs
/// under the child's caps, and a cut-off in an iteration unwinds the child, its count and value so far included; the
/// stop branch then runs in the parent.
#[test]
fn a_loop_calls_and_runs_in_a_child() {
    let r = run("fn g(x: Nat) -> Nat effects [] {\n    return exact_add(x, 3);\n}\n\n\
                 export fn f() -> Nat effects [] {\n    \
                 return repeat(2, 0, fn(acc: Nat) -> Nat { return g(acc); });\n}\n");
    returns(&r, Value::int(6));
    assert_eq!(rows(&r, "m.f:root#body:1 > m.f:2#call").len(), 1);
    assert_eq!(rows(&r, "m.f:root#body:1 > m.f:2 > m.g:root#exact_add").len(), 1);
    let r =
        run("export fn f() -> Nat effects [budget] {\n    return child(record { steps = 12 }, fn() -> Nat {\n        \
                 return repeat(100, 0, fn(acc: Nat) -> Nat { return exact_add(acc, 1); });\n    \
                 }, fn() -> Nat { return 9; });\n}\n");
    returns(&r, Value::int(9));
    let child = &r.summary.children[0];
    assert_eq!((child.outcome, child.code, child.live_after_return), (Outcome::CutOff, Some(STEPS), 0));
    let stop = child.stop.as_ref().expect("a stop");
    assert!(stop.at.contains("#body:1 > "), "the cap ends the second iteration: {}", stop.at);
}

/// The checker admits only an exact natural count (`c600-syntax`'s `loops.rs`). A program that reaches the run
/// unchecked with a negative count is refused at the first test, which would read it, with `LG4:input.ir_rejected`:
/// the initial value has filled the slot, the test is not charged, and the cleanup releases the count and the initial
/// value.
#[test]
fn an_unchecked_negative_count_is_refused_at_the_first_test() {
    let source = m("export fn f(n: Int) -> Nat effects [] {\n    \
                    return repeat(n, 0, fn(acc: Nat) -> Nat { return acc; });\n}\n");
    let lowered = lowered(&[&source]);
    assert!(!check(&lowered.program).expect("a report").is_admitted(), "an integer count is no natural");
    let loaded = load(&lowered.program).expect("the loader does not check types");
    let r = loaded.run("m", "f", vec![Value::int(-1)], &mut NoHost, &Options::new(WIDE)).expect("a run");
    let s = &r.summary;
    assert_eq!((s.outcome, s.code), (Outcome::Error, Some(code::IR_REJECTED)));
    assert_eq!(s.stop.as_ref().map(|stop| stop.at.as_str()), Some("m.f:root#loop_test:0"));
    assert_eq!(rows(&r, "m.f:root#visit"), [[1, 0, 0, 0, 0]], "the initial value fills the slot first");
    assert!(rows(&r, "m.f:root#loop_test:0").is_empty(), "no test is charged");
    assert_eq!(s.live_after_cleanup, 0);
}
