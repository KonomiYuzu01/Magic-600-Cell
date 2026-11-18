//! Pointwise and whole-state contracts (LG4.10, LB1.7; LP2 draft until the IR freeze): a pointwise `assert` is one
//! step of the run that holds it, and a whole-state contract runs once per revision through [`Revisions`], never
//! inside a call or a loop.

mod conformance;

use c600_check::{check, EFFECT_UNDECLARED, TYPE_MISMATCH};
use c600_ir::Ty;
use c600_run::contract::{ContractId, Revisions, Source};
use c600_run::interp::{code, Host, Options, Request, RowTariff, Sink};
use c600_run::meter::code::{NOT_GRANTED, POINTWISE_FAILED};
use c600_run::meter::Outcome;
use c600_run::value::Value;
use c600_syntax::{format, parse};
use conformance::*;

/// The engine at one revision: `settled` answers whether the revision is settled. It counts the rows it answers.
struct AtRevision {
    settled: bool,
    reads: u32,
}

fn at_revision(settled: bool) -> AtRevision {
    AtRevision { settled, reads: 0 }
}

impl Host for AtRevision {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 5, result_bound: 4 })
    }

    fn perform(&mut self, r: &Request) -> Result<Value, &'static str> {
        match *r {
            Request::Snapshot { query: "settled", ty } if ty.ty == Ty::Bool => {
                self.reads += 1;
                Ok(Value::Bool(self.settled))
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

const SOURCE: &str = "export contract settled() -> Bool effects [snapshot] {\n    \
    return read_snapshot<Bool>(\"settled\");\n}\n\n\
    export fn below(n: Nat) -> Nat effects [] {\n    \
    return repeat(n, 0, fn(acc: Nat) -> Nat {\n        let a = assert(exact_less(acc, 3));\n        \
    return exact_add(acc, 1);\n    });\n}\n";

fn id(program: &str) -> ContractId {
    ContractId { program: program.into(), module: "m".into(), function: "settled".into() }
}

/// The codes the checker gives one module `m` of `declarations`, with their functions and paths, in order.
fn findings(declarations: &str) -> Vec<(&'static str, String, String)> {
    let lowered = lowered(&[&m(declarations)]);
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    report.diagnostics.iter().map(|d| (d.code, d.function.clone(), d.path.clone())).collect()
}

fn found(code: &'static str, function: &str, path: &str) -> (&'static str, String, String) {
    (code, function.into(), path.into())
}

/// CT-LG4-05 (whole: a pointwise `assert` is one step of the run that holds it, charged once per check, also inside a
/// loop, and a false predicate stops the run there with `LG4:input.pointwise_failed`; a whole-state contract runs
/// once per revision through `Revisions`: asked again at its revision it runs nothing and asks the host nothing, and a
/// changed revision or program runs it again; no function calls or holds one, also from a loop's closure or another
/// contract, so it is never inserted in a per-call loop; a test that runs it, like this one, is its agreement test in
/// CI)
#[test]
fn ct_lg4_05_pointwise_contracts_cost_steps_and_whole_state_contracts_run_per_revision() {
    let (_, loaded) = build(&[&m(SOURCE)]);
    for n in [2u64, 3, 4] {
        let run = loaded.run("m", "below", vec![Value::int(n)], &mut at_revision(true), &Options::new(WIDE));
        let run = run.unwrap_or_else(|r| panic!("{r:?}"));
        let asserts: Vec<_> = run.summary.rows.iter().filter(|r| r.at.ends_with("#assert")).collect();
        // One assertion per turn, each one step with no kernel cost and no memory.
        assert_eq!(asserts.len() as u64, n);
        assert!(asserts.iter().all(|r| (r.steps, r.kernel_cost, r.memory) == (1, 0, 0)));
        if n < 4 {
            assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None));
        } else {
            // The fourth turn's predicate is false: the run stops at its assertion, before the guarded sum.
            assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Error, Some(POINTWISE_FAILED)));
            let last = run.summary.rows.last().expect("rows");
            assert_eq!(last.at, "m.below:root#body:3 > m.below:2.0#assert");
        }
    }

    let mut revisions = Revisions::default();
    let mut ask = |program: &str, revision: &str, settled: bool| {
        let mut host = at_revision(settled);
        let (run, source) = revisions
            .at(&loaded, &id(program), revision, &mut host, &Options::new(WIDE))
            .unwrap_or_else(|r| panic!("{r:?}"));
        assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None));
        let verdict = match run.value {
            Some(Value::Bool(v)) => Some(v),
            _ => None,
        };
        (verdict, source, host.reads)
    };
    assert_eq!(ask("p1", "r1", true), (Some(true), Source::Ran, 1));
    // Asked again at its revision, the contract does not run: the host is not asked, whatever it would answer.
    assert_eq!(ask("p1", "r1", false), (Some(true), Source::Kept, 0));
    // A changed revision runs it again and replaces the kept run, so the first revision runs again too.
    assert_eq!(ask("p1", "r2", false), (Some(false), Source::Ran, 1));
    assert_eq!(ask("p1", "r1", true), (Some(true), Source::Ran, 1));
    // So does a changed program.
    assert_eq!(ask("p2", "r1", true), (Some(true), Source::Ran, 1));
    assert_eq!(revisions.revision(&id("p1")), Some("r1"));

    // A function that is no whole-state contract is refused and runs nothing.
    let below = ContractId { function: "below".into(), ..id("p1") };
    let mut host = at_revision(true);
    let refused = revisions.at(&loaded, &below, "r1", &mut host, &Options::new(WIDE)).map(|_| ()).unwrap_err();
    assert_eq!((refused.code, host.reads), (code::IR_REJECTED, 0));

    // The checker: no function calls or holds a whole-state contract, also from a loop's closure or a contract.
    let settled =
        "contract settled() -> Bool effects [snapshot] {\n    return read_snapshot<Bool>(\"settled\");\n}\n\n";
    assert_eq!(findings(settled), vec![]);
    let callers = format!(
        "{settled}fn direct() -> Bool effects [snapshot] {{\n    return settled();\n}}\n\n\
         fn looped(n: Nat) -> Bool effects [snapshot] {{\n    \
         return repeat(n, true, fn(acc: Bool) -> Bool {{ return settled(); }});\n}}\n\n\
         fn held() -> Unit effects [] {{\n    let f = settled;\n    return ();\n}}\n\n\
         contract twice() -> Bool effects [snapshot] {{\n    return settled();\n}}\n"
    );
    assert_eq!(
        findings(&callers),
        vec![
            found(EFFECT_UNDECLARED, "direct", "root"),
            found(EFFECT_UNDECLARED, "looped", "2"),
            found(EFFECT_UNDECLARED, "held", "0"),
            found(EFFECT_UNDECLARED, "twice", "root"),
        ]
    );
}

/// A whole-state contract is a function of its revision alone: no parameter, an exact `Bool` result, and no need
/// beyond reading the snapshot and child budgets. Its declaration formats as it parses.
#[test]
fn a_whole_state_contract_reads_its_revision_and_nothing_else() {
    let file = parse(m(SOURCE).as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file), Some(m(SOURCE)));

    let shapes = "contract given(n: Nat) -> Bool effects [] {\n    return true;\n}\n\n\
        contract counted() -> Nat effects [] {\n    return 1;\n}\n\n\
        contract shown() -> Unit effects [view] {\n    return publish_view(true);\n}\n";
    assert_eq!(
        findings(shapes),
        vec![
            found(TYPE_MISMATCH, "given", "root"),
            found(TYPE_MISMATCH, "counted", "root"),
            found(TYPE_MISMATCH, "shown", "root"),
            found(EFFECT_UNDECLARED, "shown", "root"),
        ]
    );
}
