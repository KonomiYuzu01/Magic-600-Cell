//! The identity tower at run time (T1c; T1 drafts until the IR freeze, PHS.49): a sum value's construction and case,
//! `imply` along LG2.11's edges, cost arithmetic, the tower values' sizes and canonical forms, and the kernel rows as
//! host requests, run on hand-built IR against a fake kernel host. The checker's half of each row is
//! `c600-check/tests/tower.rs`; the source forms are T1b's and the kernel host rows over c600-jumble and c600-kernel
//! are T2's (`c600-tools/tests/tower_rows.rs` and `c600-jumble-tool/tests/tower_rows.rs`). Synthetic evidence only.

use c600_ir::build::{
    call, case, construct, exact, exact_add, exact_mul, helper, imply, kernel, let_, module, named, nat, param, pat,
    prim, program, publish_view, read_snapshot, set, sum_type, use_exact, var, word, write_workspace,
};
use c600_ir::{CostUnit, ExactRole, Expr, Function, Goal, KernelOp, Level, Lit, Prim, Program, Puzzle, Ty};
use c600_run::encode::encode;
use c600_run::interp::code::{HOST_EXHAUSTED, IDENTITY_MISMATCH, IR_REJECTED, LEVEL_NOT_HELD, NONCANONICAL_VALUE};
use c600_run::interp::{load, Host, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::{NOT_GRANTED, POINTWISE_FAILED};
use c600_run::meter::{Amounts, Outcome, Row};
use c600_run::tower::{Digest, Handle, HandleTy, Holds, IdKind, SumTy};
use c600_run::value::Value;
use num_bigint::BigInt;

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The fake host's tariff of every kernel row.
const TARIFF: RowTariff = RowTariff { kernel_cost: 7, result_bound: 256 };

/// The hex of every digest the fake host makes: two bytes.
const HEX: &str = "c0de";

/// A fake kernel host. It answers a kernel row with a value of the row's declared type, records every request, and
/// can refuse a tariff, replace an answer or declare an answer too large for its bound.
#[derive(Default)]
struct TowerHost {
    /// Every kernel request: its row, its arguments and its declared result type.
    asked: Vec<(KernelOp, Vec<Value>, Ty)>,
    /// Snapshot reads answered by name.
    snapshots: Vec<(&'static str, Value)>,
    refuse: Option<&'static str>,
    answer: Option<Value>,
    oversized: bool,
    next: u64,
    sunk: usize,
}

impl TowerHost {
    fn handle(&mut self, ty: HandleTy) -> Value {
        self.next += 1;
        Value::handle(Handle::new(ty, self.next, if self.oversized { 1024 } else { 48 }))
    }

    /// A value of `ty`: a fresh handle, a digest, a certificate for a check's verdict, or a sum value's first
    /// constructor with payloads of their types.
    fn make(&mut self, ty: &Ty, args: &[Value]) -> Value {
        if let Some(h) = HandleTy::of(ty) {
            return self.handle(h);
        }
        match ty {
            Ty::Digest(p, l) => Value::digest(*p, *l, HEX).unwrap(),
            Ty::Text => Value::text("no witness"),
            Ty::Nat => Value::int(0),
            Ty::Holds(p, g) => {
                let bound = match args.get(1) {
                    Some(Value::Int(b)) => Some((**b).clone()),
                    _ => None,
                };
                Value::holds(Holds::new(*g, Digest::new(*p, g.level(), HEX).unwrap(), bound).unwrap())
            }
            Ty::Record(fields) => {
                let values: Vec<_> = fields.iter().map(|(n, f)| (n.clone(), self.make(&f.ty, args))).collect();
                Value::record(values.iter().map(|(n, v)| (n.as_str(), v.clone())).collect())
            }
            _ => {
                let sum = SumTy::of(ty).unwrap_or_else(|| panic!("no fake answer of {ty:?}"));
                let (ctor, payload) = ty.builtin_ctors().expect("a built-in sum type").swap_remove(0);
                let payload = payload.iter().map(|t| self.make(&t.ty, args)).collect();
                Value::sum(sum, ctor, payload)
            }
        }
    }
}

impl Host for TowerHost {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match (request, self.refuse) {
            (Request::Kernel { .. }, Some(code)) => Err(code),
            _ => Ok(TARIFF),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match *request {
            Request::Kernel { op, args, ty } => {
                self.asked.push((op, args.to_vec(), ty.ty.clone()));
                match self.answer.clone() {
                    Some(answer) => Ok(answer),
                    None => Ok(self.make(&ty.ty, args)),
                }
            }
            Request::Snapshot { query, .. } => {
                self.snapshots.iter().find(|(n, _)| *n == query).map(|(_, v)| v.clone()).ok_or(NOT_GRANTED)
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        self.sunk += 1;
        Ok(())
    }
}

fn text(s: &str) -> Expr {
    Expr::Lit(Lit::Text(s.into()))
}

fn int(s: &str) -> Expr {
    Expr::Lit(Lit::Int(s.into()))
}

fn state(p: Puzzle) -> Expr {
    kernel(KernelOp::Snapshot(p), vec![], exact(Ty::State(p)))
}

fn shape() -> Ty {
    named("m", "Shape")
}

/// A program whose module `m` declares `Shape { circle(Nat), rect(Nat, Nat), point }` and exports `f() -> result`.
fn prog(result: Ty, body: Expr, extra: Vec<Function>) -> Program {
    let decl = sum_type(
        "Shape",
        vec![("circle", vec![exact(Ty::Nat)]), ("rect", vec![exact(Ty::Nat), exact(Ty::Nat)]), ("point", vec![])],
    );
    let mut functions = vec![helper("f", vec![], exact(result), body).exported()];
    functions.extend(extra);
    program(vec![module("m", functions).with_types(vec![decl])])
}

fn run_on(p: &Program, host: &mut TowerHost) -> Run {
    load(p).unwrap_or_else(|r| panic!("{r:?}")).run("m", "f", vec![], host, &Options::new(WIDE)).unwrap()
}

fn ran(result: Ty, body: Expr) -> (Run, TowerHost) {
    let mut host = TowerHost::default();
    let run = run_on(&prog(result, body, vec![]), &mut host);
    (run, host)
}

fn nat_of(run: &Run) -> i64 {
    match run.value.as_ref() {
        Some(Value::Int(n)) => i64::try_from(&**n).unwrap(),
        other => panic!("{other:?}: {:?}", run.summary.stop),
    }
}

fn rows<'a>(run: &'a Run, op: &str) -> Vec<&'a Row> {
    let end = format!("#{op}");
    run.summary.rows.iter().filter(|r| r.at.ends_with(&end)).collect()
}

fn canonical(v: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode(v, &mut bytes).unwrap();
    bytes
}

fn completed(run: &Run) {
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?}", run.summary.stop);
    assert_eq!(run.summary.live_after_cleanup, 0);
}

/// CT-LG2-01 (part: the interpreter's half; the kernel host rows over c600-jumble and c600-kernel run in
/// `c600-jumble-tool/tests/tower_rows.rs` (T2); not run: the source half (T1b)). A case takes every constructor of a
/// declared type to its arm and binds its payloads, `_` binds nothing, and the default arm takes a constructor no arm
/// names, whether the case borrows its scrutinee through a parameter or owns a construction. An owned scrutinee's bound
/// payloads are charged in `branch` and released by the arm's `scope_exit`; a binder returned whole moves out. A
/// constructor no arm and no default takes, and a pattern of the wrong arity, stop with `LG4:input.ir_rejected`; a
/// construction of the wrong arity, of an unknown constructor or of a derivation is refused when loaded.
#[test]
fn ct_lg2_01_a_case_takes_every_constructor_its_payloads_and_the_default() {
    let arms = || {
        vec![
            (pat("circle", &["r"]), exact_mul(var("r"), var("r"))),
            (pat("rect", &["w", "h"]), exact_mul(var("w"), var("h"))),
        ]
    };
    let area = helper("area", vec![param("s", exact(shape()))], exact(Ty::Nat), case(var("s"), arms(), Some(nat(0))));
    for (ctor, args, want) in [("circle", vec![nat(5)], 25), ("rect", vec![nat(3), nat(4)], 12), ("point", vec![], 0)] {
        let mut host = TowerHost::default();
        let borrowed = call("area", vec![construct(shape(), ctor, args.clone())]);
        let r = run_on(&prog(Ty::Nat, borrowed, vec![area.clone()]), &mut host);
        completed(&r);
        assert_eq!(nat_of(&r), want, "{ctor}, borrowed");
        let owned = case(construct(shape(), ctor, args.clone()), arms(), Some(nat(0)));
        let (r, _) = ran(Ty::Nat, owned);
        completed(&r);
        assert_eq!(nat_of(&r), want, "{ctor}, owned");
        let bound: u128 = if args.is_empty() { 0 } else { 2 * args.len() as u128 };
        assert_eq!(rows(&r, "branch").iter().map(|row| row.memory).collect::<Vec<_>>(), [bound], "{ctor}");
        assert_eq!(rows(&r, "scope_exit").len(), usize::from(!args.is_empty()), "{ctor}");
    }

    let (r, _) = ran(
        Ty::Nat,
        case(construct(shape(), "rect", vec![nat(3), nat(4)]), vec![(pat("rect", &["_", "h"]), var("h"))], None),
    );
    completed(&r);
    assert_eq!(nat_of(&r), 4);
    assert_eq!(rows(&r, "branch")[0].memory, 2, "only `h` is bound");
    let (r, _) =
        ran(Ty::Nat, case(construct(shape(), "circle", vec![nat(7)]), vec![(pat("circle", &["r"]), var("r"))], None));
    completed(&r);
    assert_eq!(nat_of(&r), 7);

    let (r, _) = ran(Ty::Nat, case(construct(shape(), "point", vec![]), vec![(pat("circle", &["r"]), var("r"))], None));
    assert_eq!(r.summary.code, Some(IR_REJECTED));
    assert!(rows(&r, "branch").is_empty());
    let (r, _) = ran(
        Ty::Nat,
        case(construct(shape(), "rect", vec![nat(3), nat(4)]), vec![(pat("rect", &["w"]), var("w"))], None),
    );
    assert_eq!(r.summary.code, Some(IR_REJECTED));

    for body in [
        construct(shape(), "rect", vec![nat(1)]),
        construct(shape(), "square", vec![nat(1)]),
        construct(Ty::Derivation(Puzzle::Full), "derived", vec![state(Puzzle::Full)]),
    ] {
        let refused = load(&prog(shape(), body.clone(), vec![])).err().unwrap_or_else(|| panic!("{body:?}"));
        assert_eq!(refused.code, IR_REJECTED, "{body:?}");
    }

    // A built-in sum type: a partial result of portable data is output in its canonical form; a declared sum value
    // has none.
    let total = construct(Ty::Partial(Box::new(exact(Ty::Nat))), "total", vec![nat(3)]);
    let (r, _) = ran(Ty::Partial(Box::new(exact(Ty::Nat))), total);
    completed(&r);
    let want = Value::sum(SumTy::Partial, "total", vec![Value::int(3)]);
    assert_eq!(r.output.as_deref(), Some(&canonical(&want)[..]));
    assert!(String::from_utf8(canonical(&want)).unwrap().starts_with("{\"total\":["));
    let (r, _) = ran(shape(), construct(shape(), "point", vec![]));
    assert_eq!(r.summary.code, Some(NONCANONICAL_VALUE));
}

/// CT-LG2-09 (part: the interpreter's half; the kernel host rows over c600-jumble and c600-kernel run in
/// `c600-tools/tests/tower_rows.rs` and `c600-jumble-tool/tests/tower_rows.rs` (T2); not run: the source half (T1b)).
/// Unknown never passes as false: a verdict's default arm takes `unknown` alone, after arms for `yes` and `no`, and a
/// case whose default would take `unknown` together with another constructor stops with `LG4:input.ir_rejected` before
/// its step.
#[test]
fn ct_lg2_09_a_verdict_default_takes_unknown_alone() {
    let verdict = || construct(Ty::Verdict(Puzzle::Full, Level::Labels, None), "unknown", vec![text("no witness")]);
    let yes = || (pat("yes", &["_"]), nat(1));
    let (r, _) = ran(Ty::Nat, case(verdict(), vec![yes(), (pat("no", &["_"]), nat(2))], Some(nat(3))));
    completed(&r);
    assert_eq!(nat_of(&r), 3);
    let (r, _) = ran(Ty::Nat, case(verdict(), vec![yes()], Some(nat(3))));
    assert_eq!(r.summary.code, Some(IR_REJECTED));
    assert!(rows(&r, "branch").is_empty());
    let (r, _) = ran(Ty::Nat, case(verdict(), vec![yes(), (pat("unknown", &["why"]), nat(4))], Some(nat(3))));
    completed(&r);
    assert_eq!(nat_of(&r), 4);
}

/// The host's verdict of `check(goal, snapshot<Jumbling>)`, with `bound` for the low-height goal, cased: its `yes`
/// certificate is `h` in `then`, and `no` and `unknown` give 0.
fn certified(puzzle: Puzzle, goal: Goal, bound: Option<Expr>, then: Expr) -> Expr {
    let mut args = vec![state(puzzle)];
    args.extend(bound);
    let verdict = kernel(KernelOp::Check(goal), args, exact(Ty::Verdict(puzzle, goal.level(), Some(goal))));
    case(verdict, vec![(pat("yes", &["h"]), then), (pat("no", &["_"]), nat(0)), (pat("unknown", &["_"]), nat(0))], None)
}

/// CT-LG2-10 (part: the interpreter's half; the kernel host rows over c600-jumble and c600-kernel and their goals run
/// in `c600-tools/tests/tower_rows.rs` and `c600-jumble-tool/tests/tower_rows.rs` (T2); not run: the source half (T1b)
/// and a goal fixture of equal labels with unequal poses). `imply` takes a host's certificate along LG2.11's edges
/// only, as one pure step whose memory is the new certificate's size: exact pose to sticker test and on to low height,
/// label checkpoint to lattice placement. No edge enters the retained checkpoint, sticker test to label checkpoint is
/// no edge, and the lattice placement is not defined on 600-cell-Full: each stops with `LG4:input.ir_rejected` before
/// the step. A negative bound, and a low-height certificate taken to another bound, stop with
/// `LG4:input.pointwise_failed`. An `imply` whose bound does not fit its goal is refused when loaded.
#[test]
fn ct_lg2_10_imply_follows_the_edges_and_keeps_the_bound() {
    use Goal::*;
    let jumbling = Puzzle::Jumbling;
    let to = |goal: Goal, bound: Option<&str>| {
        let mut args = vec![var("h")];
        args.extend(bound.map(int));
        let_("g", imply(goal, args), nat(1))
    };
    for (puzzle, from, from_bound, goal, bound, want) in [
        (jumbling, ExactPose, None, StickerTest, None, None),
        (jumbling, ExactPose, None, LowHeight, Some("4"), None),
        (jumbling, LabelCheckpoint, None, LatticePlacement, None, None),
        (jumbling, RetainedCheckpoint, None, LabelCheckpoint, None, None),
        (jumbling, LowHeight, Some("3"), LowHeight, Some("3"), None),
        (jumbling, ExactPose, None, RetainedCheckpoint, None, Some(IR_REJECTED)),
        (jumbling, StickerTest, None, LabelCheckpoint, None, Some(IR_REJECTED)),
        (jumbling, StickerTest, None, ExactPose, None, Some(IR_REJECTED)),
        (Puzzle::Full, StickerTest, None, LatticePlacement, None, Some(IR_REJECTED)),
        (jumbling, LatticePlacement, None, LowHeight, Some("-1"), Some(POINTWISE_FAILED)),
        (jumbling, LowHeight, Some("3"), LowHeight, Some("4"), Some(POINTWISE_FAILED)),
    ] {
        let case = format!("{puzzle:?} {from:?} {from_bound:?} to {goal:?} {bound:?}");
        let (r, host) = ran(Ty::Nat, certified(puzzle, from, from_bound.map(int), to(goal, bound)));
        assert_eq!(host.asked.len(), 2, "{case}");
        let imply_rows = rows(&r, "imply");
        match want {
            None => {
                completed(&r);
                assert_eq!(nat_of(&r), 1, "{case}");
                let bound_size = bound.map_or(0, |b| 1 + (b.parse::<u8>().unwrap() > 0) as u128);
                assert_eq!(
                    imply_rows.iter().map(|row| (row.kernel_cost, row.memory)).collect::<Vec<_>>(),
                    [(0, 4 + 2 + bound_size)],
                    "{case}"
                );
            }
            Some(code) => {
                assert_eq!(r.summary.code, Some(code), "{case}");
                assert!(imply_rows.is_empty(), "{case}");
            }
        }
    }

    // The low-height goal takes its bound and no other goal takes one.
    for (goal, args) in [(LowHeight, vec![var("h")]), (StickerTest, vec![var("h"), nat(1)])] {
        let body = certified(jumbling, ExactPose, None, let_("g", imply(goal, args), nat(1)));
        assert_eq!(load(&prog(Ty::Nat, body, vec![])).err().map(|r| r.code), Some(IR_REJECTED), "{goal:?}");
    }

    // The same edges on the certificates themselves: `implied` is defined exactly where LG2.11's closure leads, and
    // a low-height certificate keeps its bound.
    let subject = Digest::new(jumbling, Level::Poses, HEX).unwrap();
    for from in Goal::ALL {
        let bound = |g: Goal| g.takes_bound().then(|| BigInt::from(3));
        let h = Holds::new(from, subject.clone(), bound(from)).unwrap();
        for goal in Goal::ALL {
            let got = h.implied(goal, bound(goal).as_ref());
            assert_eq!(got.is_ok(), from.implies(goal), "{from:?} to {goal:?}");
            if let Ok(implied) = got {
                assert_eq!(
                    (implied.goal(), implied.subject(), implied.bound()),
                    (goal, &subject, bound(goal).as_ref())
                );
            }
        }
        if from.implies(LowHeight) && from != LowHeight {
            assert!(h.implied(LowHeight, Some(&BigInt::from(9))).is_ok(), "{from:?}");
        }
    }
}

/// CT-LG2-06 (part: the interpreter's half; not run: the source half (T1b), the budget and seconds clauses and the
/// kernel host rows over c600-jumble and c600-kernel (T2)). Costs of one puzzle and unit add, order and compare as
/// their counts do, a cost's count is an exact `Nat`, and a count becomes a cost for no kernel units; costs of two
/// units or two puzzles, and a negative count, stop with `LG4:input.ir_rejected`.
#[test]
fn ct_lg2_06_costs_add_order_and_compare_within_their_puzzle_and_unit() {
    use CostUnit::*;
    let cost = |p: Puzzle, u: CostUnit, n: u64| prim(Prim::ToCost(p, u), vec![nat(n)]);
    let j = Puzzle::Jumbling;
    let (r, _) = ran(Ty::Cost(j, Generator), exact_add(cost(j, Generator, 3), cost(j, Generator, 4)));
    completed(&r);
    let seven = Value::cost(j, Generator, 7).unwrap();
    assert_eq!(r.output.as_deref(), Some(&canonical(&seven)[..]));
    assert_eq!(
        String::from_utf8(canonical(&seven)).unwrap().trim_end(),
        "{\"cost\":{\"count\":{\"int\":\"7\"},\"puzzle\":\"Jumbling\",\"unit\":\"generator\"}}"
    );
    assert_eq!(
        rows(&r, "to_cost").iter().map(|row| (row.kernel_cost, row.memory)).collect::<Vec<_>>(),
        [(0, 2), (0, 2)]
    );

    let (r, _) = ran(Ty::Nat, prim(Prim::CostValue, vec![exact_add(cost(j, MenuLetter, 3), cost(j, MenuLetter, 4))]));
    completed(&r);
    assert_eq!(nat_of(&r), 7);
    for (op, a, b, want) in [
        (Prim::ExactLess, 3, 4, true),
        (Prim::ExactLess, 4, 3, false),
        (Prim::ExactEq, 3, 3, true),
        (Prim::ExactEq, 3, 4, false),
    ] {
        let (r, _) = ran(Ty::Bool, prim(op, vec![cost(j, UiAction, a), cost(j, UiAction, b)]));
        completed(&r);
        assert!(matches!(r.value, Some(Value::Bool(x)) if x == want), "{op:?} {a} {b}");
    }

    for body in [
        exact_add(cost(j, Generator, 3), cost(j, MenuLetter, 4)),
        exact_add(cost(j, Generator, 3), cost(Puzzle::Full, Generator, 4)),
        prim(Prim::ExactLess, vec![cost(j, Generator, 3), cost(Puzzle::Full, Generator, 4)]),
        prim(Prim::ExactLess, vec![cost(j, Generator, 3), cost(j, UiAction, 4)]),
        exact_add(cost(j, Generator, 3), nat(4)),
        prim(Prim::ToCost(j, Generator), vec![int("-1")]),
        prim(Prim::CostValue, vec![nat(4)]),
    ] {
        let (r, _) = ran(Ty::Nat, body.clone());
        assert_eq!(r.summary.code, Some(IR_REJECTED), "{body:?}");
    }
}

/// A kernel row reaches the host as `Request::Kernel` with its row, its arguments and its declared type, and is one
/// step priced by the host's tariff: its kernel cost, and its result bound as memory. The answer must have the declared
/// type, indices included, and stay within the bound. A refused tariff charges the row with nothing and asks nothing;
/// an answer of another type stops the run with `LG4:input.ir_rejected` after the row, and one over its bound with
/// `KN7:resource_limit.host_exhausted`. A mistyped argument and a negative bound stop before the row.
#[test]
fn kernel_rows_ask_the_host_at_its_tariff_and_check_the_answer() {
    let j = Puzzle::Jumbling;
    let level = kernel(KernelOp::Project(Level::Coset), vec![state(j)], exact(Ty::LevelValue(j, Level::Coset)));
    let digest = || kernel(KernelOp::Digest, vec![level.clone()], exact(Ty::Digest(j, Level::Coset)));
    let (r, host) = ran(Ty::Digest(j, Level::Coset), digest());
    completed(&r);
    let want = Value::digest(j, Level::Coset, HEX).unwrap();
    assert_eq!(r.output.as_deref(), Some(&canonical(&want)[..]));
    let ops: Vec<_> = host.asked.iter().map(|(op, _, _)| *op).collect();
    assert_eq!(ops, [KernelOp::Snapshot(j), KernelOp::Project(Level::Coset), KernelOp::Digest]);
    let args: Vec<Vec<String>> =
        host.asked.iter().map(|(_, a, _)| a.iter().map(|v| format!("{v:?}")).collect()).collect();
    assert!(args[0].is_empty());
    assert!(args[1][0].contains("State(Jumbling)") && args[2][0].contains("LevelValue(Jumbling, Coset)"), "{args:?}");
    let types: Vec<_> = host.asked.iter().map(|(_, _, t)| t.clone()).collect();
    assert_eq!(types, [Ty::State(j), Ty::LevelValue(j, Level::Coset), Ty::Digest(j, Level::Coset)]);
    let charged: Vec<_> = rows(&r, "kernel").iter().map(|row| (row.steps, row.kernel_cost, row.memory)).collect();
    assert_eq!(charged, [(1, 7, 256); 3]);

    // A refused tariff.
    let mut host = TowerHost { refuse: Some(NOT_GRANTED), ..TowerHost::default() };
    let r = run_on(&prog(Ty::State(j), state(j), vec![]), &mut host);
    assert_eq!(r.summary.code, Some(NOT_GRANTED));
    assert!(host.asked.is_empty());
    assert_eq!(rows(&r, "kernel").iter().map(|row| (row.kernel_cost, row.memory)).collect::<Vec<_>>(), [(0, 0)]);

    // Answers of another type, of the other puzzle or level, or over the bound.
    let mistyped = [
        Value::int(1),
        Value::digest(Puzzle::Full, Level::Coset, HEX).unwrap(),
        Value::digest(j, Level::Shape, HEX).unwrap(),
        Value::handle(Handle::new(HandleTy::LevelValue(j, Level::Coset), 1, 8)),
    ];
    for answer in mistyped {
        let host = TowerHost::default();
        let p = prog(Ty::Nat, let_("d", digest(), nat(0)), vec![]);
        let loaded = load(&p).unwrap();
        // The first two rows are answered by type; the digest row gets the replaced answer.
        let mut replacing = Replacing { inner: host, at: 2, answer };
        let r = loaded.run("m", "f", vec![], &mut replacing, &Options::new(WIDE)).unwrap();
        assert_eq!(r.summary.code, Some(IR_REJECTED), "{:?}", replacing.answer);
        assert_eq!(rows(&r, "kernel").len(), 3, "the row is charged before its answer is checked");
    }
    let mut host = TowerHost { oversized: true, ..TowerHost::default() };
    let r = run_on(&prog(Ty::Nat, let_("s", state(j), nat(0)), vec![]), &mut host);
    assert_eq!(r.summary.code, Some(HOST_EXHAUSTED));

    // A mistyped argument and a negative bound stop before the row and ask nothing.
    for body in [
        kernel(KernelOp::Project(Level::Labels), vec![nat(1)], exact(Ty::LevelValue(j, Level::Labels))),
        kernel(KernelOp::Digest, vec![state(j)], exact(Ty::Digest(j, Level::Poses))),
        kernel(
            KernelOp::Check(Goal::LowHeight),
            vec![state(j), text("4")],
            exact(Ty::Verdict(j, Level::Poses, Some(Goal::LowHeight))),
        ),
    ] {
        let (r, host) = ran(Ty::Nat, let_("x", body.clone(), nat(0)));
        assert_eq!(r.summary.code, Some(IR_REJECTED), "{body:?}");
        assert!(host.asked.iter().all(|(op, _, _)| matches!(op, KernelOp::Snapshot(_))), "{body:?}");
    }
    let low = kernel(
        KernelOp::Check(Goal::LowHeight),
        vec![state(j), int("-1")],
        exact(Ty::Verdict(j, Level::Poses, Some(Goal::LowHeight))),
    );
    let (r, host) = ran(Ty::Nat, let_("x", low, nat(0)));
    assert_eq!(r.summary.code, Some(POINTWISE_FAILED));
    assert_eq!(host.asked.len(), 1);
    assert_eq!(rows(&r, "kernel").len(), 1, "only the snapshot's row");
    // A row of the wrong arity is refused when loaded.
    let refused = load(&prog(Ty::Nat, kernel(KernelOp::Digest, vec![], exact(Ty::Digest(j, Level::Poses))), vec![]));
    assert_eq!(refused.err().map(|r| r.code), Some(IR_REJECTED));
}

/// A host that replaces the answer of its `at`-th kernel request.
struct Replacing {
    inner: TowerHost,
    at: usize,
    answer: Value,
}

impl Host for Replacing {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.inner.tariff(request)
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let made = self.inner.perform(request)?;
        Ok(if self.inner.asked.len() == self.at + 1 { self.answer.clone() } else { made })
    }
    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.inner.sink(sink, value, bytes)
    }
}

/// CT-LG2-02 (part: the interpreter's half; the kernel host rows over c600-jumble and c600-kernel run in
/// `c600-tools/tests/tower_rows.rs` and `c600-jumble-tool/tests/tower_rows.rs` (T2); not run: the source half (T1b)).
/// An identity of one puzzle never stands for the other's: a comparison of two puzzles' states and a goal checked on a
/// puzzle it is not defined on stop with `KN1:input.identity_mismatch` after the row's step, without asking the host,
/// and a host answer of the other puzzle's type is refused.
#[test]
fn ct_lg2_02_states_of_two_puzzles_never_meet() {
    let compare = kernel(
        KernelOp::Compare(Level::Labels),
        vec![state(Puzzle::Full), state(Puzzle::Jumbling)],
        exact(Ty::Verdict(Puzzle::Full, Level::Labels, None)),
    );
    let check = kernel(
        KernelOp::Check(Goal::ExactPose),
        vec![state(Puzzle::Full)],
        exact(Ty::Verdict(Puzzle::Full, Level::Poses, Some(Goal::ExactPose))),
    );
    for body in [compare, check] {
        let (r, host) = ran(Ty::Nat, let_("v", body.clone(), nat(0)));
        assert_eq!(r.summary.code, Some(IDENTITY_MISMATCH), "{body:?}");
        assert!(host.asked.iter().all(|(op, _, _)| matches!(op, KernelOp::Snapshot(_))), "{body:?}");
        assert_eq!(rows(&r, "kernel").last().map(|row| (row.steps, row.kernel_cost, row.memory)), Some((1, 0, 0)));
    }
    // A comparison within one puzzle asks the host.
    let same = kernel(
        KernelOp::Compare(Level::Labels),
        vec![state(Puzzle::Full), state(Puzzle::Full)],
        exact(Ty::Verdict(Puzzle::Full, Level::Labels, None)),
    );
    let (r, host) = ran(Ty::Nat, let_("v", same, nat(0)));
    completed(&r);
    assert_eq!(host.asked.last().map(|(op, _, _)| *op), Some(KernelOp::Compare(Level::Labels)));
    // The host's state of the other puzzle is no state of this one.
    let mut host = Replacing {
        inner: TowerHost::default(),
        at: 0,
        answer: Value::handle(Handle::new(HandleTy::State(Puzzle::Full), 1, 8)),
    };
    let r = load(&prog(Ty::Nat, let_("s", state(Puzzle::Jumbling), nat(0)), vec![]))
        .unwrap()
        .run("m", "f", vec![], &mut host, &Options::new(WIDE))
        .unwrap();
    assert_eq!(r.summary.code, Some(IR_REJECTED));
}

/// CT-LG2-03 (part: the interpreter's half; the kernel host rows over c600-jumble and c600-kernel, where a read of
/// labels off the lattice returns `partial`, run in `c600-jumble-tool/tests/tower_rows.rs` (T2); not run: the source
/// half (T1b)). A projection to a level its puzzle does not hold stops with `KN1:input.level_not_held` after the row's
/// step, without asking the host: 600-cell-Full holds the labels and the position only.
#[test]
fn ct_lg2_03_a_projection_needs_a_level_the_puzzle_holds() {
    for level in Level::ALL {
        let body =
            kernel(KernelOp::Project(level), vec![state(Puzzle::Full)], exact(Ty::LevelValue(Puzzle::Full, level)));
        let (r, host) = ran(Ty::Nat, let_("v", body, nat(0)));
        if level.held_by(Puzzle::Full) {
            completed(&r);
            assert_eq!(host.asked.len(), 2, "{level:?}");
        } else {
            assert_eq!(r.summary.code, Some(LEVEL_NOT_HELD), "{level:?}");
            assert_eq!(host.asked.len(), 1, "{level:?}");
        }
    }
}

/// CT-LG2-07 (part: the interpreter's half; the kernel host rows over c600-jumble and c600-kernel run in
/// `c600-tools/tests/tower_rows.rs` and `c600-jumble-tool/tests/tower_rows.rs` (T2); not run: the source half (T1b)).
/// Raw pose data enters as a state only through `import` with the jumbling puzzle's reachability claim: a claim about
/// 600-cell-Full stops with `KN1:input.identity_mismatch` without asking the host, and an import without a claim is
/// refused when loaded.
#[test]
fn ct_lg2_07_an_import_takes_the_jumbling_claim() {
    let table = || read_snapshot("table", exact(Ty::PoseTable));
    let claim = |p: Puzzle| kernel(KernelOp::Reachability, vec![state(p)], exact(Ty::Reachability(p)));
    let import = |p: Puzzle| kernel(KernelOp::Import, vec![table(), claim(p)], exact(Ty::State(Puzzle::Jumbling)));
    let host = || TowerHost {
        snapshots: vec![("table", Value::handle(Handle::new(HandleTy::PoseTable, 90, 32)))],
        ..TowerHost::default()
    };
    let mut h = host();
    let r = run_on(&prog(Ty::Nat, let_("s", import(Puzzle::Jumbling), nat(0)), vec![]), &mut h);
    completed(&r);
    assert_eq!(h.asked.last().map(|(op, a, _)| (*op, a.len())), Some((KernelOp::Import, 2)));
    let mut h = host();
    let r = run_on(&prog(Ty::Nat, let_("s", import(Puzzle::Full), nat(0)), vec![]), &mut h);
    assert_eq!(r.summary.code, Some(IDENTITY_MISMATCH));
    assert!(h.asked.iter().all(|(op, _, _)| *op != KernelOp::Import));
    let lone = kernel(KernelOp::Import, vec![table()], exact(Ty::State(Puzzle::Jumbling)));
    assert_eq!(load(&prog(Ty::Nat, let_("s", lone, nat(0)), vec![])).err().map(|r| r.code), Some(IR_REJECTED));
}

/// CT-LG2-11 (part: the interpreter's half; the kernel host rows over c600-jumble and c600-kernel run in
/// `c600-tools/tests/tower_rows.rs` and `c600-jumble-tool/tests/tower_rows.rs` (T2); not run: the source half (T1b)). A
/// derivation's request carries literal letters only: the word crosses as the program wrote it, never as a key. A word
/// equality key in its place stops with `LG4:input.ir_rejected` before the row, and a Full word for the jumbling
/// puzzle's state with `KN1:input.identity_mismatch`, neither asking the host to derive.
#[test]
fn ct_lg2_11_a_derivation_carries_literal_letters_only() {
    let derive = |p: Puzzle, w: Expr| kernel(KernelOp::Derive, vec![state(p), w], exact(Ty::Derivation(p)));
    let derived = |p: Puzzle, w: Expr| {
        case(derive(p, w), vec![(pat("derived", &["s"]), nat(1)), (pat("refused", &["_"]), nat(0))], None)
    };
    let (r, host) = ran(Ty::Nat, derived(Puzzle::Full, word(&[1, -2, 3])));
    completed(&r);
    assert_eq!(nat_of(&r), 1);
    let (op, args, _) = host.asked.last().unwrap();
    assert_eq!(*op, KernelOp::Derive);
    assert!(matches!(&args[0], Value::Handle(h) if h.ty() == HandleTy::State(Puzzle::Full)));
    assert_eq!(format!("{:?}", args[1]), format!("{:?}", Value::word(&[1, -2, 3])));
    assert!(matches!(args[1], Value::Word(_)));

    let key = prim(Prim::WordKey, vec![word(&[1, -2, 3])]);
    let (r, host) = ran(Ty::Nat, derived(Puzzle::Full, key));
    assert_eq!(r.summary.code, Some(IR_REJECTED));
    assert!(host.asked.iter().all(|(op, _, _)| *op != KernelOp::Derive));
    let (r, host) = ran(Ty::Nat, derived(Puzzle::Jumbling, word(&[1])));
    assert_eq!(r.summary.code, Some(IDENTITY_MISMATCH));
    assert!(host.asked.iter().all(|(op, _, _)| *op != KernelOp::Derive));
}

/// A host's handle is never published, stored, handed to an exact sink, keyed or compared by its id: each output
/// refuses it as a value with no canonical form before the host is asked, an equality of two states (even of one
/// state with itself) and a set of states stop with `LG4:input.ir_rejected`, and two poses compare by the exact key
/// the host gave them, never by their ids.
#[test]
fn handles_are_never_published_stored_keyed_or_compared_by_id() {
    let full = Puzzle::Full;
    let outputs = [
        state(full),
        let_("u", publish_view(state(full)), nat(0)),
        let_("u", write_workspace("kept", exact(Ty::State(full)), state(full)), nat(0)),
        let_("u", use_exact(ExactRole::Evidence, state(full)), nat(0)),
    ];
    for body in outputs {
        let mut host = TowerHost::default();
        let result = if matches!(body, Expr::Kernel { .. }) { Ty::State(full) } else { Ty::Nat };
        let r = run_on(&prog(result, body.clone(), vec![]), &mut host);
        assert_eq!(r.summary.code, Some(NONCANONICAL_VALUE), "{body:?}");
        assert_eq!(host.sunk, 0, "{body:?}");
    }
    let same = let_("s", state(full), prim(Prim::ExactEq, vec![var("s"), var("s")]));
    let two = prim(Prim::ExactEq, vec![state(full), state(full)]);
    let keyed = let_("k", set(exact(Ty::State(full)), vec![state(full)]), nat(0));
    for body in [same, two, keyed] {
        let (r, _) = ran(Ty::Bool, body.clone());
        assert_eq!(r.summary.code, Some(IR_REJECTED), "{body:?}");
    }

    let pose = |id: u64, key: &str| Value::handle(Handle::new(HandleTy::Pose, id, 16).with_key(key).unwrap());
    let poses = || TowerHost {
        snapshots: vec![("a", pose(1, "k1")), ("b", pose(2, "k1")), ("c", pose(1, "k2"))],
        ..TowerHost::default()
    };
    let read = |n: &str| read_snapshot(n, exact(Ty::Pose));
    for (x, y, want) in [("a", "b", true), ("a", "c", false)] {
        let mut host = poses();
        let r = run_on(&prog(Ty::Bool, prim(Prim::ExactEq, vec![read(x), read(y)]), vec![]), &mut host);
        completed(&r);
        assert!(matches!(r.value, Some(Value::Bool(v)) if v == want), "{x} {y}");
    }
    let mut host = poses();
    let r = run_on(
        &prog(Ty::Nat, let_("k", set(exact(Ty::Pose), vec![read("a"), read("b"), read("c")]), nat(0)), vec![]),
        &mut host,
    );
    completed(&r);
    assert!(Handle::new(HandleTy::State(full), 1, 8).with_key("k").is_none(), "only a pose carries a key");
}

/// The tower's values with their DET.57 sizes and canonical forms (T1 drafts): an ID is 4 bytes, a digest its bytes,
/// a cost its count as an integer, a certificate a tag, its subject's bytes and its bound, a sum value a record of its
/// payloads; an ID, a digest, a cost and a partial result of portable data have canonical forms, and a pose-table
/// index, a certificate, a handle and every other sum value have none.
#[test]
fn tower_values_have_sizes_and_canonical_forms() {
    let form = |v: &Value| {
        let mut out = Vec::new();
        encode(v, &mut out).ok().map(|()| String::from_utf8(out).unwrap().trim_end().to_owned())
    };
    let id = Value::id(IdKind::Piece, Puzzle::Full, 12);
    assert_eq!(id.size(), 4);
    assert_eq!(form(&id).as_deref(), Some("{\"piece\":{\"index\":12,\"puzzle\":\"Full\"}}"));
    let d = Value::digest(Puzzle::Jumbling, Level::Labels, "00ff").unwrap();
    assert_eq!(d.size(), 2);
    assert_eq!(
        form(&d).as_deref(),
        Some("{\"digest\":{\"hex\":\"00ff\",\"level\":\"labels\",\"puzzle\":\"Jumbling\"}}")
    );
    assert!(
        Value::digest(Puzzle::Full, Level::Labels, "0F").is_none()
            && Value::digest(Puzzle::Full, Level::Labels, "").is_none()
    );
    let c = Value::cost(Puzzle::Full, CostUnit::UiAction, 300).unwrap();
    assert_eq!(c.size(), 3);
    assert!(Value::cost(Puzzle::Full, CostUnit::UiAction, -1).is_none());
    let subject = Digest::new(Puzzle::Jumbling, Level::Poses, "abcd").unwrap();
    let low = Holds::new(Goal::LowHeight, subject.clone(), Some(BigInt::from(300))).unwrap();
    assert_eq!(Value::holds(low).size(), 4 + 2 + 3);
    assert!(Holds::new(Goal::LowHeight, subject.clone(), None).is_none());
    assert!(Holds::new(Goal::ExactPose, subject.clone(), Some(BigInt::from(1))).is_none());
    assert!(Holds::new(Goal::LowHeight, subject.clone(), Some(BigInt::from(-1))).is_none());
    assert!(Holds::new(Goal::ExactPose, Digest::new(Puzzle::Full, Level::Poses, "ab").unwrap(), None).is_none());
    let partial = Value::sum(SumTy::Partial, "partial", vec![Value::int(3), Value::list(vec![])]);
    assert_eq!(partial.size(), 4 + 2 + 8 + Value::list(vec![]).size());
    let expected =
        format!("{{\"partial\":[{},{}]}}", form(&Value::int(3)).unwrap(), form(&Value::list(vec![])).unwrap());
    assert_eq!(form(&partial), Some(expected));
    let none = [
        Value::PoseTableIndex(3),
        Value::holds(Holds::new(Goal::StickerTest, subject, None).unwrap()),
        Value::handle(Handle::new(HandleTy::Evidence, 1, 8)),
        Value::sum(
            SumTy::Reachability(Puzzle::Jumbling),
            "open",
            vec![Value::handle(Handle::new(HandleTy::Evidence, 2, 8))],
        ),
        Value::sum(SumTy::Named { module: "m".into(), name: "Shape".into() }, "point", vec![]),
    ];
    for v in &none {
        assert_eq!(form(v), None, "{v:?}");
    }
    assert_eq!(
        Value::handle(Handle::new(HandleTy::Evidence, 1, 8)).size(),
        8,
        "a handle has the size its host declared"
    );
}
