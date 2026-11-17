//! Generators as values at run time (DET.61; LP2 draft until the IR freeze, PHS.49). `rng_init` starts a cursor on
//! the stream of the run's seed whose path is the run's assigned path followed by the call's segments; it is priced
//! by the length of the hashed key input and holds a generator's 41 units. `rng_draw` draws the cursor's next block as
//! a natural, one step and the block tariff, holding the result's 33 units. Each owner holds its own cursor: a borrowed
//! argument draws from its owner's cursor, a move hands it on, and an alias (a borrowed value bound or returned)
//! copies it, so that the copy and the original repeat the same blocks from there on. A run without a seed is not
//! granted a generator, and a generator has no canonical form, so it is neither an entry's argument nor its result.
//! The exhausted cursor is `c600-run`'s `rng.rs` unit test: a run cannot draw 2^64 blocks. The source half is
//! `c600-syntax`'s `generators.rs`; the frozen generator traces run as programs in `det65_programs.rs`.

mod conformance;

use std::sync::Arc;

use c600_run::interp::code::NONCANONICAL_VALUE;
use c600_run::interp::{NoHost, Options, Run};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Outcome, Row, GENERATOR_SIZE};
use c600_run::rng::{key_input_len, stream_key, Cursor};
use c600_run::value::{equal, Generator, Value};
use conformance::*;
use num_bigint::{BigInt, Sign};

const SEED: [u8; 32] = [7; 32];

/// Block `i` of the stream at `path` under [`SEED`], as the natural a draw returns.
fn block(path: &[&str], i: usize) -> Value {
    let (key, _) = stream_key(&SEED, path).expect("a valid stream");
    let mut cursor = Cursor::new(key);
    let blocks: Vec<[u8; 32]> = (0..=i).map(|_| cursor.draw().expect("a block")).collect();
    Value::int(BigInt::from_bytes_be(Sign::Plus, &blocks[i]))
}

/// Runs `m.f` of a module that requires `random`, seeded at `path` when one is given.
fn run_at(declarations: &str, path: Option<&[&str]>) -> Run {
    let (_, loaded) = build(&[&m_requiring("random", declarations)]);
    let mut options = Options::new(WIDE);
    options.seed = path.map(|p| (&SEED[..], p));
    loaded.run("m", "f", vec![], &mut NoHost, &options).expect("a run")
}

/// Runs `m.f`, seeded at `["root"]`.
fn run(declarations: &str) -> Run {
    run_at(declarations, Some(&["root"]))
}

fn charges(row: &Row) -> [u128; 5] {
    [row.steps, row.kernel_cost, row.memory, row.output, row.released]
}

/// The rows of one operation, by their path, with their charges.
fn rows<'r>(run: &'r Run, op: &str) -> Vec<(&'r str, [u128; 5])> {
    let suffix = format!("#{op}");
    run.summary.rows.iter().filter(|row| row.at.ends_with(&suffix)).map(|row| (row.at.as_str(), charges(row))).collect()
}

/// Checks that the run completed with `want` and released everything.
fn returns(run: &Run, want: Value) {
    let s = &run.summary;
    assert_eq!((s.outcome, s.code), (Outcome::Completed, None), "{:?}", s.stop);
    let value = run.value.as_ref().expect("a returned value");
    assert!(equal(value, &want), "returned {value:?}, want {want:?}");
    assert_eq!(s.live_after_cleanup, 0, "cleanup releases every generator");
}

fn stopped(run: &Run, code: &str) {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Error, Some(code)), "{:?}", run.summary.stop);
}

/// An `rng_init` row: one step, the input-byte tariff of 1 per hashed byte, and the generator's 41 units.
fn init_row(path: &[&str]) -> [u128; 5] {
    [1, key_input_len(32, path), GENERATOR_SIZE, 0, 0]
}

/// An `rng_draw` row: one step, the block tariff of 3, and a 256-bit natural's 33 units.
const DRAW_ROW: [u128; 5] = [1, 3, 33, 0, 0];

const STREAMS: &str = "export fn f() -> record { a: Nat, b: Nat, c: Nat, d: Nat } effects [random] {\n    \
                       let g = rng_init(\"a\");\n    let h = rng_init(\"b\", \"c\");\n    let e = rng_init();\n    \
                       return record { a = rng_draw(g), b = rng_draw(g), c = rng_draw(h), d = rng_draw(e) };\n}\n";

/// DET.61 (LP2 draft): `rng_init` starts the stream at the run's path followed by its segments, none or more, on the
/// run's seed, and successive draws are the stream's blocks 0, 1, ... Re-initialising the run's own path reproduces
/// the run's stream, deliberately (DET.61). Each start is priced by its hashed input, 13 bytes of domain, the seed's
/// 32, 4 for the segment count and 4 per segment with its bytes, after the run's own start at `entry#rng_init`.
#[test]
fn rng_init_starts_a_stream_below_the_runs_path_and_rng_draw_draws_its_blocks() {
    let ran = run(STREAMS);
    let want = Value::record(vec![
        ("a", block(&["root", "a"], 0)),
        ("b", block(&["root", "a"], 1)),
        ("c", block(&["root", "b", "c"], 0)),
        ("d", block(&["root"], 0)),
    ]);
    returns(&ran, want);
    let starts: Vec<[u128; 5]> = rows(&ran, "rng_init").into_iter().map(|(_, c)| c).collect();
    let paths: [&[&str]; 4] = [&["root"], &["root", "a"], &["root", "b", "c"], &["root"]];
    assert_eq!(starts, paths.map(init_row));
    assert_eq!(rows(&ran, "rng_init")[0].0, "entry#rng_init", "the run's own generator starts first");
    assert_eq!([starts[0][1], starts[1][1], starts[2][1]], [57, 62, 67], "DET.61's key input lengths");
    assert_eq!(rows(&ran, "rng_draw").into_iter().map(|(_, c)| c).collect::<Vec<_>>(), [DRAW_ROW; 4]);

    // Another run path moves every stream below it; the segments stay.
    let other = run_at(STREAMS, Some(&["root", "7"]));
    let want = Value::record(vec![
        ("a", block(&["root", "7", "a"], 0)),
        ("b", block(&["root", "7", "a"], 1)),
        ("c", block(&["root", "7", "b", "c"], 0)),
        ("d", block(&["root", "7"], 0)),
    ]);
    returns(&other, want);
}

/// DET.61 (LP2 draft): binding a generator to a second name is an alias, a copy of the cursor at the full 41 units, so
/// the copy and the original draw the same block; as the frozen trace `generator-alias-cursor` does.
#[test]
fn an_alias_copies_the_cursor() {
    let ran = run("export fn f() -> record { a: Nat, b: Nat } effects [random] {\n    let g = rng_init(\"a\");\n    \
         let h = g;\n    return record { a = rng_draw(g), b = rng_draw(h) };\n}\n");
    returns(&ran, Value::record(vec![("a", block(&["root", "a"], 0)), ("b", block(&["root", "a"], 0))]));
    assert_eq!(rows(&ran, "alias"), [("m.f:1#alias", [1, 0, GENERATOR_SIZE, 0, 0])]);
}

/// DET.61 (LP2 draft): a borrowed argument refers to its owner's cursor, so a callee's draw advances the caller's
/// generator, and a move hands the cursor on with its counter.
#[test]
fn a_borrowed_or_moved_generator_shares_its_owners_cursor() {
    let borrowed = run("fn take(g: Generator) -> Nat effects [random] {\n    return rng_draw(g);\n}\n\n\
         export fn f() -> record { a: Nat, b: Nat } effects [random] {\n    let g = rng_init(\"a\");\n    \
         return record { a = take(g), b = rng_draw(g) };\n}\n");
    returns(&borrowed, Value::record(vec![("a", block(&["root", "a"], 0)), ("b", block(&["root", "a"], 1))]));
    assert!(rows(&borrowed, "alias").is_empty(), "a borrow copies nothing: {:?}", borrowed.summary.rows);

    let moved = run(
        "fn take(g: Generator) -> Nat effects [random] {\n    let x = rng_draw(g);\n    return rng_draw(g);\n}\n\n\
         export fn f() -> Nat effects [random] {\n    let g = rng_init(\"a\");\n    let x = rng_draw(g);\n    \
         return take(g);\n}\n",
    );
    returns(&moved, block(&["root", "a"], 2));
    assert!(rows(&moved, "alias").is_empty(), "a move copies nothing: {:?}", moved.summary.rows);
}

/// DET.61 (LP2 draft): a callee that returns its borrowed generator returns an alias, an independent cursor at the
/// borrowed one's counter; a generator a callee starts and returns is moved to its caller, which draws on from it, as
/// the frozen trace `generator-returned-owner` does.
#[test]
fn a_returned_generator_is_an_alias_of_a_borrow_or_the_moved_owner() {
    let aliased = run("fn pass(g: Generator) -> Generator effects [] {\n    return g;\n}\n\n\
         export fn f() -> record { a: Nat, b: Nat } effects [random] {\n    let g = rng_init(\"a\");\n    \
         let h = pass(g);\n    return record { a = rng_draw(h), b = rng_draw(g) };\n}\n");
    returns(&aliased, Value::record(vec![("a", block(&["root", "a"], 0)), ("b", block(&["root", "a"], 0))]));
    assert_eq!(rows(&aliased, "alias").len(), 1, "the return of a borrow copies its cursor");

    let owned = run("fn make() -> Generator effects [random] {\n    return rng_init(\"a\");\n}\n\n\
         export fn f() -> record { a: Nat, b: Nat } effects [random] {\n    let g = make();\n    \
         return record { a = rng_draw(g), b = rng_draw(g) };\n}\n");
    returns(&owned, Value::record(vec![("a", block(&["root", "a"], 0)), ("b", block(&["root", "a"], 1))]));
    assert!(rows(&owned, "alias").is_empty(), "a returned owner moves");
}

/// DET.61 and CT-LG3-04's run-time half (LP2 draft): a run without a seed is not granted a generator. `rng_init`
/// charges its step alone and stops with `LG3:capability.not_granted`, as `random_draw` does.
#[test]
fn a_run_without_a_seed_is_not_granted_a_generator() {
    let ran = run_at(
        "export fn f() -> Nat effects [random] {\n    let g = rng_init(\"a\");\n    return rng_draw(g);\n}\n",
        None,
    );
    stopped(&ran, NOT_GRANTED);
    assert_eq!(rows(&ran, "rng_init"), [("m.f:0#rng_init", [1, 0, 0, 0, 0])], "no run generator and no charge");
    assert!(rows(&ran, "rng_draw").is_empty());
}

/// DET.59 and DET.61 (LP2 draft): a generator has no canonical form. An entry that returns one stops at its output
/// with `PS1:input.noncanonical_value`, and a run refuses one as an argument before it starts.
#[test]
fn a_generator_is_neither_an_entrys_argument_nor_its_result() {
    let ran = run("export fn f() -> Generator effects [random] {\n    return rng_init(\"a\");\n}\n");
    stopped(&ran, NONCANONICAL_VALUE);

    let (_, loaded) = build(&[&m_requiring(
        "random",
        "export fn f(g: Generator) -> Nat effects [random] {\n    return rng_draw(g);\n}\n",
    )]);
    let (key, _) = stream_key(&SEED, &["root"]).expect("a valid stream");
    let g = Value::Generator(Arc::new(Generator::new(Cursor::new(key))));
    let mut options = Options::new(WIDE);
    options.seed = Some((&SEED[..], &["root"][..]));
    let refused = loaded.run("m", "f", vec![g], &mut NoHost, &options).err().map(|e| e.code);
    assert_eq!(refused, Some(NONCANONICAL_VALUE));
}
