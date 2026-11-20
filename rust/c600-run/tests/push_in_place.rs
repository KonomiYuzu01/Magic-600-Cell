//! `push` extends its list in place when it holds the list's only handle (LP2; the runtime lead from C9's U7 limit
//! windows, where a list grown one push at a time took quadratic time). A list read from a `let` slot that is dead from
//! the push on gives up the slot's handle first. The price is the one of a new list either way: these tests check the
//! values a run returns where the slot is still read, inside a loop, or captured after the push, that a push priced
//! in place and one that copies are charged alike, and that a long chain of pushes runs in linear time.

mod conformance;

use std::time::{Duration, Instant};

use c600_run::interp::{NoHost, Options, Run};
use c600_run::meter::{Outcome, Row};
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

fn nats(items: &[u32]) -> Value {
    Value::list(items.iter().map(|&i| Value::int(i)).collect())
}

/// `f` returning a list grown by `count` pushes, each of the list's length, with `body` as the loop's body.
fn grown(count: u32, body: &str) -> String {
    format!(
        "export fn f() -> List<Nat> effects [] {{\n    \
         return repeat({count}, list<Nat>(), fn(done: List<Nat>) -> List<Nat> {{ {body} }});\n}}\n"
    )
}

/// The loop's body that pushes onto the value so far, whose slot is dead from the push on.
const IN_PLACE: &str = "return push(done, length(done));";

/// The same push, with the value so far read again after it, so the push copies.
const COPIED: &str = "let longer = push(done, length(done));\n        let seen = length(done);\n        return longer;";

/// The charges of the `push` rows, in order.
fn pushes(run: &Run) -> Vec<[u128; 5]> {
    let charges = |r: &Row| [r.steps, r.kernel_cost, r.memory, r.output, r.released];
    run.summary.rows.iter().filter(|r| r.at.ends_with("#push")).map(charges).collect()
}

/// A loop whose body pushes onto its own value so far builds the list the copying form builds, and each push is
/// charged alike: the price is that of a new list of the pushed length, however the list is built. (The linear-time
/// test below is the one that shows the first form extends in place.)
#[test]
fn a_push_in_place_is_priced_as_a_copy() {
    let (in_place, copied) = (run(&grown(5, IN_PLACE)), run(&grown(5, COPIED)));
    returns(&in_place, nats(&[0, 1, 2, 3, 4]));
    returns(&copied, nats(&[0, 1, 2, 3, 4]));
    let charges = pushes(&in_place);
    assert_eq!(charges.len(), 5);
    assert_eq!(charges, pushes(&copied));
    assert_eq!(charges.iter().map(|c| c[1]).collect::<Vec<_>>(), [1, 2, 3, 4, 5], "one unit per element built");
}

/// Pushes on a list no slot holds extend it in place too.
#[test]
fn nested_pushes_build_the_list() {
    let r = run("export fn f() -> List<Nat> effects [] {\n    return push(push(push(list<Nat>(), 1), 2), 3);\n}\n");
    returns(&r, nats(&[1, 2, 3]));
}

/// A slot read after the push keeps its list: the push copies.
#[test]
fn a_slot_read_after_the_push_keeps_its_list() {
    let r = run("export fn f() -> Nat effects [] {\n    let xs = list<Nat>(1);\n    let ys = push(xs, 2);\n    \
                 return exact_add(length(xs), length(ys));\n}\n");
    returns(&r, Value::int(3));
}

/// A slot bound outside a loop is read again by the loop's next iteration: a push on it inside the loop copies.
#[test]
fn a_slot_bound_outside_a_loop_keeps_its_list() {
    let r = run("export fn f() -> Nat effects [] {\n    let xs = list<Nat>(7);\n    \
                 return repeat(3, 0, fn(acc: Nat) -> Nat { return exact_add(acc, length(push(xs, acc))); });\n}\n");
    returns(&r, Value::int(6));
}

/// A child scope that takes the slot after the push reads the list as it was: the push copies.
#[test]
fn a_slot_a_child_takes_after_the_push_keeps_its_list() {
    let r = run("export fn f() -> Nat effects [budget] {\n    let xs = list<Nat>(1);\n    let ys = push(xs, 2);\n    \
                 return child(record { steps = 100 }, fn() -> Nat { return exact_add(length(xs), length(ys)); }, \
                 fn(s: Status) -> Nat { return 0; });\n}\n");
    returns(&r, Value::int(3));
}

/// Another handle on the list, here a second slot bound to it, keeps the list as it was: the push copies, although
/// its own slot is dead from the push on.
#[test]
fn a_list_another_slot_holds_is_copied() {
    let r = run("export fn f() -> Nat effects [] {\n    let xs = list<Nat>(1);\n    let kept = xs;\n    \
                 let ys = push(xs, 2);\n    return exact_add(length(kept), length(ys));\n}\n");
    returns(&r, Value::int(3));
}

/// 2^17 pushes, each onto the list so far, run in linear time: a copy per push would copy about 2^33 elements, which
/// takes minutes. The run keeps no rows, and its price is unchanged (about 2^33 kernel units).
#[test]
fn a_list_grown_one_push_at_a_time_takes_linear_time() {
    let count = 1u32 << 17;
    let source = format!(
        "export fn f() -> Nat effects [] {{\n    return length(repeat({count}, list<Nat>(), \
         fn(done: List<Nat>) -> List<Nat> {{ {IN_PLACE} }}));\n}}\n"
    );
    let (_, loaded) = build(&[&m(&source)]);
    let options = Options { record: false, ..Options::new(WIDE) };
    let start = Instant::now();
    let r = loaded.run("m", "f", vec![], &mut NoHost, &options).expect("a run");
    let elapsed = start.elapsed();
    returns(&r, Value::int(count));
    assert!(elapsed < Duration::from_secs(20), "{count} pushes took {elapsed:?}");
}
