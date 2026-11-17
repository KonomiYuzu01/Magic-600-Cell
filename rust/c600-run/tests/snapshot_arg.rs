//! A snapshot read whose query takes an argument (LP2 draft; LP3's plan, section 2): `read_snapshot<T>("q", x)` reads
//! the engine's row `q` for `x`, such as a word's protection, for LB2's star and recipe construction and Net and Strict
//! protection. The checker types the argument as portable exact data, the interpreter hands the host the argument's
//! canonical bytes after DET.59's priced encoding pass, and the boundary always asks the engine and counts the read as
//! state-derived data.

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, EXACTNESS_VIOLATION, TYPE_MISMATCH};
use c600_digest::sha256;
use c600_ir::build::{exact, fn_ref, helper, module, program, read_snapshot_with, unit};
use c600_ir::{Effect, Program, Ty};
use c600_run::boundary::{Boundary, Context, Pins, Place, Store, Stored};
use c600_run::encode::encode;
use c600_run::interp::code::{IR_REJECTED, NONCANONICAL_VALUE};
use c600_run::interp::{load, Host, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, field, Author, Json, Pin, Producer, Profile};
use c600_run::value::{equal, Value};
use c600_syntax::{lower, parse};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const HEAD: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n";

/// The engine side: a literal read answers 1, a read with an argument 7; it records every argument it is handed.
#[derive(Default)]
struct Engine {
    tariffs: usize,
    asked: Vec<(String, Vec<u8>)>,
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        self.tariffs += 1;
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match *request {
            Request::Snapshot { .. } => Ok(Value::int(1)),
            Request::SnapshotWith { query, arg, .. } => {
                self.asked.push((query.to_string(), arg.to_vec()));
                Ok(Value::int(7))
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

fn lowered(body: &str) -> Program {
    let file = parse(format!("{HEAD}{body}").as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    lower(&[file]).unwrap_or_else(|e| panic!("{e}")).program
}

fn run(p: &Program, entry: &str, args: Vec<Value>, host: &mut dyn Host) -> Run {
    load(p).unwrap_or_else(|r| panic!("{r:?}")).run("m", entry, args, host, &Options::new(WIDE)).unwrap()
}

fn canonical(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode(v, &mut out).expect("a canonical value");
    out
}

const READER: &str = "export fn protection(w: Word) -> Nat effects [snapshot] {\n    \
    return read_snapshot<Nat>(\"protection\", w);\n}\n";

/// The read has the literal read's effect, row and state-read fact, so the built-in column lists it as `Read(query)`
/// and capability separation counts it. The host gets the query, the argument's canonical bytes and the type: DET.59's
/// encoding row comes first, and the kernel row counts the bytes it hands over as output, as a preview's request does.
#[test]
fn a_read_with_an_argument_hands_the_host_the_arguments_canonical_bytes() {
    let p = lowered(READER);
    let report = check(&p).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let summary = report.summary("m", "protection").expect("the entry");
    assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]));
    assert_eq!(summary.rows, BTreeSet::from([Row::Read("protection".into())]));
    assert!(separation(summary, &Policy::interim()).reads_state);

    let word = Value::word(&[1, -2]);
    let mut engine = Engine::default();
    let ran = run(&p, "protection", vec![word.clone()], &mut engine);
    assert_eq!(ran.summary.outcome, Outcome::Completed, "{:?}", ran.summary.stop);
    assert!(equal(ran.value.as_ref().expect("a value"), &Value::int(7)));
    let bytes = canonical(&word);
    assert_eq!(engine.asked, [("protection".to_string(), bytes.clone())]);
    assert_eq!(provenance::parse(&bytes), Some(Json::List(vec![Json::Int(1), Json::Int(-2)])));

    let ops: Vec<&str> = ran.summary.rows.iter().map(|r| r.at.rsplit('#').next().unwrap_or_default()).collect();
    let k = ops.iter().position(|op| *op == "kernel").expect("a kernel row");
    assert_eq!(ops[k - 1], "encode", "{ops:?}");
    let kernel = &ran.summary.rows[k];
    assert_eq!((kernel.kernel_cost, kernel.output), (9, bytes.len() as u128));
}

/// A function, closure or receipt has no canonical bytes: the checker refuses it as an argument, and the same IR built
/// by hand stops at the encoding row before the host is asked. An approximate argument is refused by the checker and,
/// unchecked, by the interpreter, so no approximation reaches an exact read's answer.
#[test]
fn an_argument_without_canonical_bytes_or_with_an_approximation_never_reaches_the_host() {
    let f = helper("f", vec![], exact(Ty::Unit), unit());
    let read = helper("read", vec![], exact(Ty::Nat), read_snapshot_with("protection", fn_ref("f"), exact(Ty::Nat)));
    let hand = program(vec![module("m", vec![f, read.exported()])]);
    let report = check(&hand).unwrap();
    assert!(!report.is_admitted() && report.codes().contains(&TYPE_MISMATCH), "{:#?}", report.diagnostics);
    let mut engine = Engine::default();
    assert_eq!(run(&hand, "read", vec![], &mut engine).summary.code, Some(NONCANONICAL_VALUE));
    assert_eq!((engine.tariffs, engine.asked.len()), (0, 0));

    let approx = lowered(
        "export fn read(x: Nat) -> Nat effects [snapshot] {\n    \
         return read_snapshot<Nat>(\"protection\", approx53(x));\n}\n",
    );
    let report = check(&approx).unwrap();
    assert!(!report.is_admitted() && report.codes().contains(&EXACTNESS_VIOLATION), "{:#?}", report.diagnostics);
    let mut engine = Engine::default();
    assert_eq!(run(&approx, "read", vec![Value::int(2)], &mut engine).summary.code, Some(IR_REJECTED));
    assert_eq!((engine.tariffs, engine.asked.len()), (0, 0));
}

/// The argument is exact portable data: a display-qualified value would lose its qualifier at the host (LG2.19), and a
/// word equality key or a receipt has no portable form (owner decision of 16 November 2026 under PHS.52).
#[test]
fn the_checker_refuses_a_display_qualified_or_unportable_argument() {
    for (body, code) in [
        (
            "export fn read() -> Nat effects [snapshot, budget] {\n    \
             return read_snapshot<Nat>(\"protection\", used_steps());\n}\n",
            EXACTNESS_VIOLATION,
        ),
        (
            "export fn read(w: Word) -> Nat effects [snapshot] {\n    \
             return read_snapshot<Nat>(\"protection\", word_key(w));\n}\n",
            TYPE_MISMATCH,
        ),
        (
            "export fn read(w: Word) -> Nat effects [snapshot, preview] {\n    \
             return read_snapshot<Nat>(\"protection\", propose(w, target(\"t\")));\n}\n",
            TYPE_MISMATCH,
        ),
        // The same refusals one level down: a container keeps what it holds.
        (
            "export fn read() -> Nat effects [snapshot, budget] {\n    \
             return read_snapshot<Nat>(\"protection\", record { s = used_steps() });\n}\n",
            EXACTNESS_VIOLATION,
        ),
        (
            "export fn read(x: Nat) -> Nat effects [snapshot] {\n    \
             return read_snapshot<Nat>(\"protection\", record { a = approx53(x) });\n}\n",
            EXACTNESS_VIOLATION,
        ),
        (
            "export fn read(w: Word) -> Nat effects [snapshot] {\n    \
             return read_snapshot<Nat>(\"protection\", record { k = word_key(w) });\n}\n",
            TYPE_MISMATCH,
        ),
        (
            "export fn read(w: Word) -> Nat effects [snapshot] {\n    \
             return read_snapshot<Nat>(\"protection\", list<CompressedWord>(compress(w)));\n}\n",
            TYPE_MISMATCH,
        ),
    ] {
        let report = check(&lowered(body)).unwrap();
        assert!(!report.is_admitted() && report.codes().contains(&code), "{body}: {:#?}", report.diagnostics);
    }
    let fine = "export fn read(w: Word, n: Nat) -> Nat effects [snapshot] {\n    \
                return read_snapshot<Nat>(\"protection\", record { n = n, w = w });\n}\n";
    let report = check(&lowered(fine)).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
}

/// Nothing the read holds stays live: the encoding buffer and an owned argument are released after the answer, so a
/// run ends with as much live memory as a literal read's.
#[test]
fn the_read_releases_its_buffer_and_an_owned_argument() {
    let p = lowered(
        "export fn borrowed(w: Word) -> Nat effects [snapshot] {\n    \
         return read_snapshot<Nat>(\"protection\", w);\n}\n\n\
         export fn owned(w: Word) -> Nat effects [snapshot] {\n    \
         return read_snapshot<Nat>(\"protection\", concat(w, w));\n}\n\n\
         export fn literal(w: Word) -> Nat effects [snapshot] {\n    \
         return read_snapshot<Nat>(\"protection\");\n}\n",
    );
    let live = |entry: &str| {
        let ran = run(&p, entry, vec![Value::word(&[1, -2, 3])], &mut Engine::default());
        assert_eq!(ran.summary.outcome, Outcome::Completed, "{entry}: {:?}", ran.summary.stop);
        (ran.summary.live_before_cleanup, ran.summary.live_after_cleanup)
    };
    assert_eq!(live("borrowed"), live("literal"));
    assert_eq!(live("owned"), live("literal"));
}

fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

/// The boundary asks the engine for a read with an argument even when the workspace holds a record of the query's
/// name, and counts the read as state-derived data although the run's context says otherwise: the record a run then
/// stores has a block that may read state. A literal read of the same name takes the stored record and its block.
#[test]
fn the_boundary_asks_the_engine_and_counts_a_state_read() {
    const KEEPERS: &str = "export fn keep_with(w: Word) -> Unit effects [snapshot, workspace] {\n    \
        return write_workspace<Nat>(\"kept\", read_snapshot<Nat>(\"protection\", w));\n}\n\n\
        export fn keep_literal() -> Unit effects [snapshot, workspace] {\n    \
        return write_workspace<Nat>(\"kept\", read_snapshot<Nat>(\"protection\"));\n}\n";
    let p = lowered(KEEPERS);
    assert!(check(&p).unwrap().is_admitted());
    let quiet = provenance::canonical(&provenance::manual(&contract()));
    let mut kept = Vec::new();
    for (entry, args) in [("keep_with", vec![Value::word(&[3])]), ("keep_literal", vec![])] {
        let mut store = Store::default();
        store.keep(&quiet);
        let stored = Stored { value: Value::int(5), bytes: canonical(&Value::int(5)), block: quiet.clone() };
        store.records.insert(Place::Workspace("protection".into()), stored);
        let producer = Producer {
            contract: contract(),
            checker: None,
            author: Author::Own,
            module: sha256(KEEPERS.as_bytes()),
            entry: entry.into(),
            profile: Profile::ToolAuthoring,
        };
        // The context withholds the static state-read fact, so the block's field is the boundary's own count.
        let context = Context {
            run: "run".into(),
            base: "rev-7".into(),
            producer,
            reads_state: false,
            inputs: vec![quiet.clone()],
            pins: Pins { contracts: vec![contract()], checkers: vec![] },
        };
        let mut engine = Engine::default();
        let mut boundary = Boundary::new(&mut engine, &mut store, context);
        let ran = load(&p).unwrap().run("m", entry, args, &mut boundary, &Options::new(WIDE)).unwrap();
        drop(boundary);
        assert_eq!(ran.summary.outcome, Outcome::Completed, "{entry}: {:?}", ran.summary.stop);
        let record = &store.records[&Place::Workspace("kept".into())];
        let block = provenance::read(&record.block).expect("a block");
        kept.push((engine.asked.len(), field::may_read_state(&block), record.value.clone()));
    }
    assert_eq!((kept[0].0, kept[0].1), (1, true), "the argument read asks the engine");
    assert!(equal(&kept[0].2, &Value::int(7)));
    assert_eq!((kept[1].0, kept[1].1), (0, false), "the literal read takes the stored record");
    assert!(equal(&kept[1].2, &Value::int(5)));
}
