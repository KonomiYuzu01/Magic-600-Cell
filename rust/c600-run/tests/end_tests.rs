//! LB3.2's end tests' verdicts at run time (LP2 draft until the IR freeze, PHS.49; the static halves are
//! `c600-syntax`'s `end_tests.rs`). A program reads a verdict from its test's engine row, for the word the solver named,
//! and uses it through a `match` on its outcome: a verdict matches the status tag of its outcome and no other pattern,
//! so `unestablished` reaches its own arm or a `_` arm that takes nothing else, never the arm of `fails`. A verdict that
//! leaves a run crosses as its outcome's tag and its test's row, so two tests' verdicts never share an encoding.

mod conformance;

use c600_check::check;
use c600_ir::{EndTest, EndTestOutcome, Ty};
use c600_run::encode::encode;
use c600_run::interp::{code, load, Host, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome;
use c600_run::value::{equal, Value};
use conformance::*;

/// The engine side: every end-test row answers the verdict it holds. It records each row and declared type it is
/// asked for.
struct Verdicts {
    answer: Value,
    asked: Vec<(String, Ty)>,
}

impl Verdicts {
    fn new(answer: Value) -> Verdicts {
        Verdicts { answer, asked: Vec::new() }
    }
}

impl Host for Verdicts {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, r: &Request) -> Result<Value, &'static str> {
        match *r {
            Request::SnapshotWith { query, ty, .. } => {
                self.asked.push((query.to_string(), ty.ty.clone()));
                Ok(self.answer.clone())
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Ok(())
    }
}

/// `sorted` names both established outcomes, so its `_` arm takes `unestablished` alone; `apart` names
/// `unestablished`, so its `_` arm takes the two established outcomes; `passed` returns the verdict itself.
const PROGRAM: &str = "export fn sorted(w: Word) -> Nat effects [snapshot] {\n    \
    let v = read_snapshot<LabelCheckpointVerdict>(\"label_checkpoint\", w);\n    \
    return match v {\n        tag(\"holds\") => 1;\n        tag(\"fails\") => 2;\n        _ => 0;\n    };\n}\n\n\
    export fn apart(w: Word) -> Nat effects [snapshot] {\n    \
    let v = read_snapshot<StickerTestVerdict>(\"sticker_test\", w);\n    \
    return match v {\n        tag(\"unestablished\") => 3;\n        _ => 0;\n    };\n}\n\n\
    export fn passed(w: Word) -> RetainedCheckpointVerdict effects [snapshot] {\n    \
    return read_snapshot<RetainedCheckpointVerdict>(\"retained_checkpoint\", w);\n}\n";

fn completed(run: &Run) {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
}

fn canonical(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode(v, &mut out).expect("a canonical value");
    out
}

/// CT-LG2-09 (part: unknown cannot pass as false at run time, for LB3.2's end tests' verdicts; LP2 draft: a match
/// picks the arm of the verdict's outcome, and `unestablished` never reaches the arm of `fails`; not run: the IR's
/// domain families of partial projections and goals, and KN0.5's general verdict, M4) and CT-LB3-01 (part: each end
/// test's row is asked for with its own verdict type and answers it; not run: the kernel's facts, LP3's `lb3.rs`).
#[test]
fn a_verdict_is_read_from_its_test_row_and_matched_on_its_outcome() {
    let (_, loaded) = build(&[&m(PROGRAM)]);
    let w = Value::word(&[1, -2]);
    let arms = [(EndTestOutcome::Holds, 1, 0), (EndTestOutcome::Fails, 2, 0), (EndTestOutcome::Unestablished, 0, 3)];
    for (outcome, sorted, apart) in arms {
        for (entry, test, arm) in [("sorted", EndTest::LabelCheckpoint, sorted), ("apart", EndTest::StickerTest, apart)]
        {
            let mut host = Verdicts::new(Value::EndTest(test, outcome));
            let ran = loaded.run("m", entry, vec![w.clone()], &mut host, &Options::new(WIDE)).unwrap();
            completed(&ran);
            assert!(equal(ran.value.as_ref().expect("a value"), &Value::int(arm)), "{entry}: {outcome:?}");
            assert_eq!(host.asked, [(test.row().to_string(), Ty::EndTest(test))], "{entry}");
        }
    }
}

/// CT-LB3-01 (part: a verdict leaves a run with its test and its outcome, LB3.2; LP2 draft; not run: the kernel's
/// facts). Its canonical form is a record of its outcome's tag and its test's row, in key order, and the line's end;
/// the record lies within the value's retained bound, which leaves the line's end to the encoding pass. Each of the
/// nine verdicts has an encoding of its own, and a verdict equals only itself, never the status tag of its outcome.
#[test]
fn a_verdict_leaves_a_run_with_its_test_and_outcome() {
    let mut seen = Vec::new();
    for test in EndTest::ALL {
        for outcome in EndTestOutcome::ALL {
            let v = Value::EndTest(test, outcome);
            let text = format!("{{\"outcome\":\"{}\",\"test\":\"{}\"}}", outcome.tag(), test.row());
            assert_eq!(canonical(&v), format!("{text}\n").as_bytes());
            assert_eq!(v.bound(), text.len() as u128, "{v:?}");
            assert!(!seen.contains(&text), "{text}");
            seen.push(text);
            assert!(equal(&v, &Value::EndTest(test, outcome)));
            assert!(!equal(&v, &Value::tag(outcome.tag())), "{v:?}");
            for other_test in EndTest::ALL {
                for other_outcome in EndTestOutcome::ALL {
                    let same = (other_test, other_outcome) == (test, outcome);
                    assert_eq!(equal(&v, &Value::EndTest(other_test, other_outcome)), same, "{v:?}");
                }
            }
        }
    }

    let (_, loaded) = build(&[&m(PROGRAM)]);
    let verdict = Value::EndTest(EndTest::RetainedCheckpoint, EndTestOutcome::Unestablished);
    let mut host = Verdicts::new(verdict.clone());
    let ran = loaded.run("m", "passed", vec![Value::word(&[3])], &mut host, &Options::new(WIDE)).unwrap();
    completed(&ran);
    assert!(equal(ran.value.as_ref().expect("a value"), &verdict));
    let expected: &[u8] = b"{\"outcome\":\"unestablished\",\"test\":\"retained_checkpoint\"}\n";
    assert_eq!(ran.output.as_deref(), Some(expected));
}

/// Generic functions over a verdict: `same` and `both` compare their type variable's values, which the checker admits
/// without knowing they are verdicts (it does not unify type variables, M1).
const GENERIC: &str = "fn same<T>(a: T, b: T) -> Bool effects [] {\n    return exact_eq(a, b);\n}\n\n\
    fn both<T>(a: List<T>, b: List<T>) -> Bool effects [] {\n    return exact_eq(a, b);\n}\n\n\
    export fn equal(w: Word) -> Bool effects [snapshot] {\n    \
    let v = read_snapshot<LabelCheckpointVerdict>(\"label_checkpoint\", w);\n    return same(v, v);\n}\n\n\
    export fn listed(w: Word) -> Bool effects [snapshot] {\n    \
    let v = read_snapshot<LabelCheckpointVerdict>(\"label_checkpoint\", w);\n    \
    return both(list<LabelCheckpointVerdict>(v), list<LabelCheckpointVerdict>(v));\n}\n";

/// A match whose `_` arm takes `unestablished` together with `fails`, which the checker refuses (`end_tests.rs` in
/// `c600-syntax`); the run sees it only when a program reaches it unchecked or behind a type variable.
const LOOSE: &str = "export fn loose(w: Word) -> Nat effects [snapshot] {\n    \
    let v = read_snapshot<LabelCheckpointVerdict>(\"label_checkpoint\", w);\n    \
    return match v {\n        tag(\"holds\") => 1;\n        _ => 0;\n    };\n}\n";

fn refused(run: &Run, what: &str) {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Error, Some(code::IR_REJECTED)), "{what}");
}

/// CT-LG2-09 (part: unknown cannot pass as false behind a type variable, for LB3.2's verdicts; LP2 draft; not run: the
/// IR's domain families of partial projections and goals, and KN0.5's general verdict, M4). Where the checker cannot
/// see a verdict, the run refuses its misuse with `LG4:input.ir_rejected` before the step: an exact equality whose
/// operand is or holds a verdict, and a match on a verdict whose `_` arm takes `unestablished` with another outcome,
/// whichever outcome the verdict has, so the outcome decides nothing about the refusal.
#[test]
fn a_verdict_the_checker_cannot_see_is_refused_by_the_run() {
    let generic = lowered(&[&m(GENERIC)]);
    assert!(check(&generic.program).unwrap().is_admitted());
    let (_, loaded) = build(&[&m(GENERIC)]);
    for outcome in EndTestOutcome::ALL {
        for entry in ["equal", "listed"] {
            let mut host = Verdicts::new(Value::EndTest(EndTest::LabelCheckpoint, outcome));
            let ran = loaded.run("m", entry, vec![Value::word(&[1])], &mut host, &Options::new(WIDE)).unwrap();
            refused(&ran, &format!("{entry}: {outcome:?}"));
        }
    }

    let unchecked = lowered(&[&m(LOOSE)]);
    assert!(!check(&unchecked.program).unwrap().is_admitted());
    let loaded = load(&unchecked.program).unwrap_or_else(|r| panic!("{r:?}"));
    for outcome in EndTestOutcome::ALL {
        let mut host = Verdicts::new(Value::EndTest(EndTest::LabelCheckpoint, outcome));
        let ran = loaded.run("m", "loose", vec![Value::word(&[1])], &mut host, &Options::new(WIDE)).unwrap();
        refused(&ran, &format!("loose: {outcome:?}"));
    }
}
