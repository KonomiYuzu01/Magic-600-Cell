//! The identity tower and the domain families of the IR draft (T1 drafts until the IR freeze, PHS.49): the puzzle,
//! level, goal and cost-unit indices, the built-in and declared sum types, the kernel rows and `imply`, checked on
//! hand-built IR. Each conformance row's source half waits for T1b's lowering and its run-time half for T1c's values
//! and host rows.

mod support;

use c600_check::{
    check, COST_UNIT_MIX, EXACTNESS_VIOLATION, IDENTITY_MISMATCH, LEVEL_MISMATCH, MODE_MISMATCH, REACHABILITY_MISSING,
    TYPE_MISMATCH, VERDICT_REQUIRED, WORD_KEY_NOT_EXECUTABLE,
};
use c600_ir::build::*;
use c600_ir::{CostUnit, ExactRole, Expr, Function, Goal, KernelOp, Level, Param, Prim, Program, Puzzle, Ty, TypeDecl};
use support::{codes, mismatch, only, report, tool};

use Puzzle::{Full, Jumbling};

/// A helper `f` with the given parameters, result type and body.
fn f(params: Vec<Param>, result: Ty, body: Expr) -> Function {
    helper("f", params, exact(result), body)
}

fn p(name: &str, ty: Ty) -> Param {
    param(name, exact(ty))
}

fn eq(a: Expr, b: Expr) -> Expr {
    prim(Prim::ExactEq, vec![a, b])
}

/// The codes of a program holding `f` alone.
fn alone(params: Vec<Param>, result: Ty, body: Expr) -> Vec<&'static str> {
    codes(&tool(vec![f(params, result, body)]))
}

/// The codes of a value of type `supplied` passed as the argument of a parameter of type `expected`.
fn as_argument(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    let g = helper("g", vec![p("x", expected.clone())], exact(Ty::Unit), unit());
    codes(&tool(vec![g, f(vec![p("a", supplied.clone())], Ty::Unit, call("g", vec![var("a")]))]))
}

/// The codes of a value of type `supplied` returned where the result type is `expected`.
fn as_result(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    alone(vec![p("a", supplied.clone())], expected.clone(), var("a"))
}

/// The codes of a value of type `supplied` returned through the generic `id<T>(x: T) -> T` where the result type is
/// `expected`: substitution keeps every index (LG2.1).
fn through_generic(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    let t = || exact(Ty::Var("T".into()));
    let id = helper("id", vec![param("x", t())], t(), var("x"));
    codes(&tool(vec![id, f(vec![p("a", supplied.clone())], expected.clone(), call("id", vec![var("a")]))]))
}

/// The codes of an exact equality of a value of type `a` and one of type `b`.
fn compared(a: &Ty, b: &Ty) -> Vec<&'static str> {
    alone(vec![p("a", a.clone()), p("b", b.clone())], Ty::Bool, eq(var("a"), var("b")))
}

/// The three ways a value meets a declared type, which agree on its code.
fn every_way(expected: &Ty, supplied: &Ty) -> Vec<&'static str> {
    let arg = as_argument(expected, supplied);
    assert_eq!(as_result(expected, supplied), arg, "{expected:?} <- {supplied:?} as a result");
    assert_eq!(through_generic(expected, supplied), arg, "{expected:?} <- {supplied:?} through a generic");
    arg
}

fn partial(ty: Ty) -> Ty {
    Ty::Partial(Box::new(exact(ty)))
}

fn state(puzzle: Puzzle) -> Ty {
    Ty::State(puzzle)
}

/// A program of one module `tool` with declared types.
fn typed(types: Vec<TypeDecl>, functions: Vec<Function>) -> Program {
    program(vec![module("tool", functions).with_types(types)])
}

/// `type Shape { dot; line(Nat) }`.
fn shape_decl() -> TypeDecl {
    sum_type("Shape", vec![("dot", vec![]), ("line", vec![exact(Ty::Nat)])])
}

fn shape() -> Ty {
    named("tool", "Shape")
}

/// The arms of a case that names every constructor of `Shape`.
fn shape_arms() -> Vec<(c600_ir::Pattern, Expr)> {
    vec![(pat("dot", &[]), nat(0)), (pat("line", &["n"]), var("n"))]
}

/// The codes of a function of a typed program.
fn shape_codes(params: Vec<Param>, result: Ty, body: Expr) -> Vec<&'static str> {
    codes(&typed(vec![shape_decl()], vec![f(params, result, body)]))
}

/// CT-LG2-01 (part: the IR half; not run: the source half (T1b) and the run-time half (T1c)). A case accounts for
/// every constructor of a declared or built-in sum type, and generics keep every puzzle, level and unit index.
#[test]
fn ct_lg2_01_cases_cover_every_constructor_and_generics_keep_indices() {
    let s = || vec![p("s", shape())];
    // A declared sum type: a case that names every constructor, or has a `_` arm, is admitted.
    assert_eq!(shape_codes(s(), Ty::Nat, case(var("s"), shape_arms(), None)), [] as [&str; 0]);
    let some = vec![(pat("line", &["n"]), var("n"))];
    assert_eq!(shape_codes(s(), Ty::Nat, case(var("s"), some.clone(), Some(nat(0)))), [] as [&str; 0]);
    // A case that leaves a constructor out has no `_` arm to take it.
    let d = only(&typed(vec![shape_decl()], vec![f(s(), Ty::Nat, case(var("s"), some, None))]));
    assert_eq!(mismatch(&d), (TYPE_MISMATCH, "f", "root", "dot | line", "line"));
    // A pattern names a constructor of the scrutinee's type and binds each payload value.
    let wrong = vec![(pat("dot", &[]), nat(0)), (pat("arc", &["n"]), var("n"))];
    assert!(shape_codes(s(), Ty::Nat, case(var("s"), wrong, None)).contains(&TYPE_MISMATCH));
    let binds_two = vec![(pat("dot", &[]), nat(0)), (pat("line", &["n", "m"]), var("n"))];
    assert_eq!(shape_codes(s(), Ty::Nat, case(var("s"), binds_two, None)), [TYPE_MISMATCH]);
    // A case on a type that is no sum type.
    let on_nat = case(var("n"), vec![(pat("dot", &[]), nat(0))], Some(nat(1)));
    let d = only(&typed(vec![shape_decl()], vec![f(vec![p("n", Ty::Nat)], Ty::Nat, on_nat)]));
    assert_eq!((d.code, d.path.as_str()), (TYPE_MISMATCH, "0"));
    // Construction: the constructor's payload in number and type.
    assert_eq!(shape_codes(vec![], shape(), construct(shape(), "line", vec![nat(2)])), [] as [&str; 0]);
    assert_eq!(shape_codes(vec![], shape(), construct(shape(), "dot", vec![])), [] as [&str; 0]);
    assert_eq!(shape_codes(vec![], shape(), construct(shape(), "line", vec![])), [TYPE_MISMATCH]);
    assert_eq!(shape_codes(vec![], shape(), construct(shape(), "arc", vec![nat(2)])), [TYPE_MISMATCH]);
    let d = only(&typed(vec![shape_decl()], vec![f(vec![], shape(), construct(shape(), "line", vec![word(&[1])]))]));
    assert_eq!((d.code, d.path.as_str()), (TYPE_MISMATCH, "0"));
    // A built-in sum type: a partial result and a reachability claim.
    let pv = || vec![p("v", partial(Ty::Nat))];
    let all = vec![
        (pat("total", &["n"]), var("n")),
        (pat("partial", &["n", "_"]), var("n")),
        (pat("undefined", &["_"]), nat(0)),
    ];
    assert_eq!(alone(pv(), Ty::Nat, case(var("v"), all.clone(), None)), [] as [&str; 0]);
    assert_eq!(alone(pv(), Ty::Nat, case(var("v"), all[..2].to_vec(), None)), [TYPE_MISMATCH]);
    // The `missing` record of a partial result: `off`, `uncovered` and `collisions`.
    let counts = vec![
        (pat("total", &["_"]), nat(0)),
        (pat("partial", &["_", "m"]), field(var("m"), "collisions")),
        (pat("undefined", &["_"]), nat(0)),
    ];
    assert_eq!(alone(pv(), Ty::Nat, case(var("v"), counts, None)), [] as [&str; 0]);
    let claims = c600_ir::ReachKind::ALL.map(|k| (pat(k.name(), &["_"]), nat(1)));
    let r = || vec![p("r", Ty::Reachability(Jumbling))];
    assert_eq!(alone(r(), Ty::Nat, case(var("r"), claims.to_vec(), None)), [] as [&str; 0]);
    assert_eq!(alone(r(), Ty::Nat, case(var("r"), claims[1..].to_vec(), None)), [TYPE_MISMATCH]);
    // Generics and inference keep the puzzle, the level and the unit.
    assert_eq!(through_generic(&Ty::Piece(Jumbling), &Ty::Piece(Full)), [IDENTITY_MISMATCH]);
    let at = |l| Ty::LevelValue(Jumbling, l);
    assert_eq!(through_generic(&at(Level::Poses), &at(Level::Labels)), [LEVEL_MISMATCH]);
    let cost = |u| Ty::Cost(Full, u);
    assert_eq!(through_generic(&cost(CostUnit::Generator), &cost(CostUnit::UiAction)), [COST_UNIT_MIX]);
    assert_eq!(through_generic(&Ty::Piece(Full), &Ty::Piece(Full)), [] as [&str; 0]);
    // A generic callable parameter keeps them too: `apply<T, E>(f: (T) -> E, x: T) -> E`.
    let (tv, ev) = (|| exact(Ty::Var("T".into())), || exact(Ty::Var("E".into())));
    let fe = fn_ty(vec![tv()], ev(), bound(&[], &["e"]));
    let apply =
        helper("apply", vec![param("g", exact(fe)), param("x", tv())], ev(), call_value(var("g"), vec![var("x")]));
    let to_digest = closure(
        vec![p("v", at(Level::Labels))],
        exact(Ty::Digest(Jumbling, Level::Labels)),
        kernel(KernelOp::Digest, vec![var("v")], exact(Ty::Digest(Jumbling, Level::Labels))),
    );
    let use_it = |result: Ty| {
        let body = call("apply", vec![to_digest.clone(), var("v")]);
        codes(&tool(vec![apply.clone(), f(vec![p("v", at(Level::Labels))], result, body)]))
    };
    assert_eq!(use_it(Ty::Digest(Jumbling, Level::Labels)), [] as [&str; 0]);
    assert_eq!(use_it(Ty::Digest(Jumbling, Level::Poses)), [LEVEL_MISMATCH]);
    assert_eq!(use_it(Ty::Digest(Full, Level::Labels)), [IDENTITY_MISMATCH]);
}

/// Faults of declared sum types are faults of the IR, not diagnostics (T1 draft): a duplicate type or constructor, an
/// unknown or unexported type, an exported signature with an unexported type, and a payload that holds a type
/// variable or a callable.
#[test]
fn declared_type_faults_are_ir_errors() {
    let user = |ty: Ty| f(vec![p("s", ty)], Ty::Unit, unit());
    let fails = |prog: Program| assert!(check(&prog).is_err(), "{prog:#?}");
    fails(typed(vec![shape_decl(), shape_decl()], vec![]));
    fails(typed(vec![shape_decl(), sum_type("Mark", vec![("dot", vec![])])], vec![]));
    fails(typed(vec![], vec![user(shape())]));
    fails(typed(vec![shape_decl()], vec![user(named("tool", "Square"))]));
    let var_payload = sum_type("Box", vec![("full", vec![exact(Ty::Var("T".into()))])]);
    fails(typed(vec![var_payload], vec![]));
    let callable = sum_type("Hook", vec![("call", vec![exact(fn_ty(vec![], exact(Ty::Unit), bound(&[], &[])))])]);
    fails(typed(vec![callable], vec![]));
    fails(typed(vec![shape_decl()], vec![user(shape()).exported()]));
    assert!(check(&typed(vec![shape_decl().exported()], vec![user(shape()).exported()])).is_ok());
    // Another module uses an exported type only.
    let other = |decl: TypeDecl| {
        let geo = module("geo", vec![]).with_types(vec![decl]);
        program(vec![geo, module("tool", vec![user(named("geo", "Shape"))])])
    };
    fails(other(shape_decl()));
    assert!(check(&other(shape_decl().exported())).unwrap().is_admitted());
}

/// CT-LG2-02 (part: the IR half; not run: the source half (T1b) and the run-time half (T1c)). An identity of one
/// puzzle never stands for the other's, and a pose, a sticker-class value, a label digest and a pose-table index are
/// four types.
#[test]
fn ct_lg2_02_identities_keep_their_puzzle_and_kind() {
    let ids: [fn(Puzzle) -> Ty; 3] = [Ty::Piece, Ty::Sticker, Ty::Slot];
    for id in ids {
        assert_eq!(every_way(&id(Jumbling), &id(Full)), [IDENTITY_MISMATCH]);
        assert_eq!(every_way(&id(Full), &id(Jumbling)), [IDENTITY_MISMATCH]);
        assert_eq!(every_way(&id(Jumbling), &id(Jumbling)), [] as [&str; 0]);
        assert_eq!(compared(&id(Full), &id(Jumbling)), [IDENTITY_MISMATCH]);
        assert_eq!(compared(&id(Full), &id(Full)), [] as [&str; 0]);
        // A pose-table index is per revision and never an identity.
        assert_eq!(every_way(&id(Jumbling), &Ty::PoseTableIndex), [IDENTITY_MISMATCH]);
    }
    // A generic binding of two puzzles' pieces: `pick<T>(a: T, b: T) -> T`.
    let t = || exact(Ty::Var("T".into()));
    let pick = helper("pick", vec![param("a", t()), param("b", t())], t(), var("a"));
    let both = vec![p("a", Ty::Piece(Full)), p("b", Ty::Piece(Jumbling))];
    let d = only(&tool(vec![pick, f(both, Ty::Piece(Full), call("pick", vec![var("a"), var("b")]))]));
    assert_eq!(mismatch(&d), (IDENTITY_MISMATCH, "f", "1", "Piece<Full>", "Piece<Jumbling>"));
    // Pose, sticker class, label digest and pose-table index are not interchangeable.
    let kinds = [
        Ty::Pose,
        Ty::LevelValue(Jumbling, Level::StickerClass),
        Ty::Digest(Jumbling, Level::Labels),
        Ty::PoseTableIndex,
    ];
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            let found = as_argument(a, b);
            if i == j {
                assert_eq!(found, [] as [&str; 0], "{a:?}");
            } else if (a, b) == (&Ty::Pose, &Ty::PoseTableIndex) {
                assert_eq!(found, [IDENTITY_MISMATCH]);
            } else {
                assert_eq!(found, [TYPE_MISMATCH], "{a:?} <- {b:?}");
            }
        }
    }
    // Digests compare at one puzzle and one level.
    let digest = |p, l| Ty::Digest(p, l);
    assert_eq!(compared(&digest(Jumbling, Level::Labels), &digest(Jumbling, Level::Labels)), [] as [&str; 0]);
    assert_eq!(compared(&digest(Jumbling, Level::Labels), &digest(Jumbling, Level::Poses)), [LEVEL_MISMATCH]);
    assert_eq!(compared(&digest(Full, Level::Labels), &digest(Jumbling, Level::Labels)), [IDENTITY_MISMATCH]);
    assert_eq!(every_way(&digest(Full, Level::Labels), &digest(Full, Level::Position)), [LEVEL_MISMATCH]);
    // Records and collections keep the index of every element.
    let list = |ty| Ty::List(Box::new(exact(ty)));
    assert_eq!(every_way(&list(Ty::Piece(Jumbling)), &list(Ty::Piece(Full))), [IDENTITY_MISMATCH]);
    let rec = |ty| Ty::Record(vec![("piece".into(), exact(ty))]);
    assert_eq!(as_argument(&rec(Ty::Sticker(Jumbling)), &rec(Ty::Sticker(Full))), [IDENTITY_MISMATCH]);
}

/// CT-LG2-03 (part: the IR half; not run: the source half (T1b) and the run-time half (T1c), where a read of labels
/// off the lattice returns `partial` or `undefined`). A projection's partial result is cased, nothing lifts a coarser
/// level to a finer one, and a class is no equality.
#[test]
fn ct_lg2_03_projection_is_partial_and_nothing_lifts() {
    let project =
        |level, of: Puzzle| kernel(KernelOp::Project(level), vec![var("s")], exact(partial(Ty::LevelValue(of, level))));
    for level in Level::ALL {
        // The jumbling puzzle holds every level; 600-cell-Full holds the labels and the position only.
        let result = partial(Ty::LevelValue(Jumbling, level));
        assert_eq!(alone(vec![p("s", state(Jumbling))], result, project(level, Jumbling)), [] as [&str; 0]);
        let found = alone(vec![p("s", state(Full))], partial(Ty::LevelValue(Full, level)), project(level, Full));
        if level.held_by(Full) {
            assert_eq!(found, [] as [&str; 0], "{level:?}");
        } else {
            assert_eq!(found, [LEVEL_MISMATCH], "{level:?}");
        }
    }
    // The partial result is no level value: it is used through a case.
    let s = || vec![p("s", state(Jumbling))];
    let value = Ty::LevelValue(Jumbling, Level::Labels);
    let d = only(&tool(vec![f(s(), value.clone(), project(Level::Labels, Jumbling))]));
    assert_eq!(
        mismatch(&d),
        (LEVEL_MISMATCH, "f", "root", "LevelValue<Jumbling, labels>", "Partial<LevelValue<Jumbling, labels>>")
    );
    let digest = Ty::Digest(Jumbling, Level::Labels);
    let direct = kernel(KernelOp::Digest, vec![project(Level::Labels, Jumbling)], exact(digest.clone()));
    assert_eq!(alone(s(), digest.clone(), direct), [LEVEL_MISMATCH]);
    let arms = vec![
        (pat("total", &["v"]), let_("d", kernel(KernelOp::Digest, vec![var("v")], exact(digest)), nat(1))),
        (pat("partial", &["_", "m"]), field(var("m"), "off")),
        (pat("undefined", &["_"]), nat(0)),
    ];
    assert_eq!(alone(s(), Ty::Nat, case(project(Level::Labels, Jumbling), arms, None)), [] as [&str; 0]);
    // No map leads back to a finer level, and a level value is no state, so nothing projects it to the poses.
    for level in Level::ALL {
        assert_eq!(level.reaches(Level::Poses), level == Level::Poses, "{level:?}");
    }
    assert!(!Level::Labels.reaches(Level::StickerClass));
    let lifted =
        kernel(KernelOp::Project(Level::Poses), vec![var("v")], exact(partial(Ty::LevelValue(Jumbling, Level::Poses))));
    assert_eq!(
        alone(vec![p("v", value.clone())], partial(Ty::LevelValue(Jumbling, Level::Poses)), lifted),
        [REACHABILITY_MISSING]
    );
    assert_eq!(every_way(&Ty::LevelValue(Jumbling, Level::Poses), &value), [LEVEL_MISMATCH]);
    // A class is no equality: level values have no exact equality, and a comparison at the labels gives a labels
    // verdict, which no poses verdict takes.
    assert_eq!(compared(&value, &value), [TYPE_MISMATCH]);
    let compare = |level, result: Ty| {
        let ab = vec![p("a", state(Jumbling)), p("b", state(Jumbling))];
        let body = kernel(KernelOp::Compare(level), vec![var("a"), var("b")], exact(result.clone()));
        alone(ab, result, body)
    };
    let verdict = |l| Ty::Verdict(Jumbling, l, None);
    assert_eq!(compare(Level::Labels, partial(verdict(Level::Labels))), [] as [&str; 0]);
    // A kernel row's declared type is the type it returns.
    assert_eq!(compare(Level::Labels, partial(verdict(Level::Poses))), [TYPE_MISMATCH]);
    assert_eq!(every_way(&verdict(Level::Poses), &verdict(Level::Labels)), [LEVEL_MISMATCH]);
    // States of two puzzles never compare, and 600-cell-Full compares at its own levels only.
    let mixed = vec![p("a", state(Full)), p("b", state(Jumbling))];
    let body = kernel(KernelOp::Compare(Level::Labels), vec![var("a"), var("b")], exact(Ty::Unit));
    let d = only(&tool(vec![f(mixed, Ty::Unit, body)]));
    assert_eq!((d.code, d.path.as_str()), (IDENTITY_MISMATCH, "1"));
    let fulls = vec![p("a", state(Full)), p("b", state(Full))];
    let body = kernel(KernelOp::Compare(Level::Poses), vec![var("a"), var("b")], exact(Ty::Unit));
    let d = only(&tool(vec![f(fulls, Ty::Unit, body)]));
    assert_eq!((d.code, d.path.as_str()), (LEVEL_MISMATCH, "0"));
}

/// CT-LG2-07 (part: the IR half; not run: the source half (T1b) and the run-time half (T1c)). Raw pose data enters as
/// a state only through `import` with a reachability claim, and a jumbling claim is no claim about 600-cell-Full.
#[test]
fn ct_lg2_07_raw_pose_data_is_no_state() {
    let record = Ty::Record(vec![("poses".into(), exact(Ty::PoseTable))]);
    for raw in [Ty::PoseTable, record, Ty::LevelValue(Jumbling, Level::Poses)] {
        assert_eq!(every_way(&state(Jumbling), &raw), [REACHABILITY_MISSING], "{raw:?}");
    }
    let import = |args: Vec<Expr>| kernel(KernelOp::Import, args, exact(state(Jumbling)));
    let tr = |claim: Ty| vec![p("t", Ty::PoseTable), p("r", claim)];
    let both = || vec![var("t"), var("r")];
    assert_eq!(alone(tr(Ty::Reachability(Jumbling)), state(Jumbling), import(both())), [] as [&str; 0]);
    // Without a claim, form validation establishes no reachability.
    let d = only(&tool(vec![f(tr(Ty::Reachability(Jumbling)), state(Jumbling), import(vec![var("t")]))]));
    assert_eq!((d.code, d.path.as_str()), (REACHABILITY_MISSING, "root"));
    let d = only(&tool(vec![f(tr(Ty::Evidence), state(Jumbling), import(both()))]));
    assert_eq!((d.code, d.path.as_str()), (REACHABILITY_MISSING, "1"));
    let d = only(&tool(vec![f(tr(Ty::Reachability(Full)), state(Jumbling), import(both()))]));
    assert_eq!((d.code, d.path.as_str()), (IDENTITY_MISMATCH, "1"));
    let three = import(vec![var("t"), var("r"), var("r")]);
    assert!(check(&tool(vec![f(tr(Ty::Reachability(Jumbling)), state(Jumbling), three)])).is_err());
    // A jumbling witness does not assert retained reachability.
    assert_eq!(every_way(&Ty::Reachability(Full), &Ty::Reachability(Jumbling)), [IDENTITY_MISMATCH]);
    let claim_of = |of: Puzzle, declared: Puzzle| {
        let body = kernel(KernelOp::Reachability, vec![var("s")], exact(Ty::Reachability(declared)));
        alone(vec![p("s", state(of))], Ty::Reachability(declared), body)
    };
    assert_eq!(claim_of(Full, Full), [] as [&str; 0]);
    assert_eq!(claim_of(Jumbling, Full), [TYPE_MISMATCH]);
    let snapshot = kernel(KernelOp::Snapshot(Full), vec![], exact(state(Full)));
    assert_eq!(alone(vec![], state(Full), snapshot), [] as [&str; 0]);
    // A program constructs a claim from evidence it was given, never the evidence, a certificate or a derivation.
    let witnessed = construct(Ty::Reachability(Jumbling), "witnessed", vec![var("e")]);
    assert_eq!(alone(vec![p("e", Ty::Evidence)], Ty::Reachability(Jumbling), witnessed), [] as [&str; 0]);
    assert_eq!(alone(vec![], Ty::Evidence, construct(Ty::Evidence, "evidence", vec![])), [TYPE_MISMATCH]);
    let holds = Ty::Holds(Jumbling, Goal::ExactPose);
    assert_eq!(alone(vec![], holds.clone(), construct(holds, "holds", vec![])), [TYPE_MISMATCH]);
    let derived = construct(Ty::Derivation(Jumbling), "derived", vec![var("s")]);
    assert_eq!(alone(vec![p("s", state(Jumbling))], Ty::Derivation(Jumbling), derived), [TYPE_MISMATCH]);
}

/// A verdict of the jumbling puzzle at the labels, of `check(sticker_test, s)`.
fn sticker_verdict() -> Ty {
    Ty::Verdict(Jumbling, Level::Labels, Some(Goal::StickerTest))
}

/// CT-LG2-09 (part: the IR half; not run: the source half (T1b) and the run-time half (T1c)). Unknown never passes
/// as false, an off-lattice projection never passes as unknown, and a goal never weakens its level.
#[test]
fn ct_lg2_09_unknown_is_no_false_and_goals_keep_their_level() {
    let v = || vec![p("v", sticker_verdict())];
    // A verdict is no boolean, and a boolean no verdict.
    assert_eq!(alone(v(), Ty::Nat, if_(var("v"), nat(1), nat(0))), [VERDICT_REQUIRED]);
    assert_eq!(every_way(&Ty::Bool, &sticker_verdict()), [VERDICT_REQUIRED]);
    assert_eq!(every_way(&sticker_verdict(), &Ty::Bool), [VERDICT_REQUIRED]);
    assert_eq!(compared(&sticker_verdict(), &sticker_verdict()), [VERDICT_REQUIRED]);
    // A `_` arm takes `unknown` alone or not at all.
    let yes = || (pat("yes", &["_"]), nat(1));
    let no = || (pat("no", &["_"]), nat(0));
    let unknown = || (pat("unknown", &["_"]), nat(2));
    assert_eq!(alone(v(), Ty::Nat, case(var("v"), vec![yes(), no(), unknown()], None)), [] as [&str; 0]);
    assert_eq!(alone(v(), Ty::Nat, case(var("v"), vec![yes(), no()], Some(nat(2)))), [] as [&str; 0]);
    assert_eq!(alone(v(), Ty::Nat, case(var("v"), vec![unknown()], Some(nat(0)))), [] as [&str; 0]);
    let d = only(&tool(vec![f(v(), Ty::Nat, case(var("v"), vec![yes()], Some(nat(0))))]));
    assert_eq!((d.code, d.path.as_str()), (VERDICT_REQUIRED, "root"));
    assert_eq!(alone(v(), Ty::Nat, case(var("v"), vec![yes(), unknown()], None)), [TYPE_MISMATCH]);
    // A `yes` of a goal's check carries the goal's certificate.
    let cert =
        vec![(pat("yes", &["h"]), var("h")), (pat("no", &["_"]), var("h0")), (pat("unknown", &["_"]), var("h0"))];
    let holds = Ty::Holds(Jumbling, Goal::StickerTest);
    let params = vec![p("v", sticker_verdict()), p("h0", holds.clone())];
    assert_eq!(alone(params, holds, case(var("v"), cert, None)), [] as [&str; 0]);
    // An off-lattice projection is no unknown: a partial verdict is no verdict, and its case has no `unknown`.
    let pv = || vec![p("v", partial(Ty::Verdict(Jumbling, Level::Labels, None)))];
    assert_eq!(alone(pv(), Ty::Nat, if_(var("v"), nat(1), nat(0))), [LEVEL_MISMATCH]);
    let as_verdict = alone(pv(), Ty::Verdict(Jumbling, Level::Labels, None), var("v"));
    assert_eq!(as_verdict, [LEVEL_MISMATCH]);
    let off = case(var("v"), vec![(pat("unknown", &["_"]), nat(2))], Some(nat(0)));
    assert_eq!(alone(pv(), Ty::Nat, off), [TYPE_MISMATCH]);
    // A goal never weakens its level: a certificate or a verdict of another goal stands for none of this one's.
    let goal = |g| Ty::Holds(Jumbling, g);
    assert_eq!(every_way(&goal(Goal::ExactPose), &goal(Goal::StickerTest)), [LEVEL_MISMATCH]);
    assert_eq!(every_way(&goal(Goal::StickerTest), &goal(Goal::ExactPose)), [LEVEL_MISMATCH]);
    let exact_pose = Ty::Verdict(Jumbling, Level::Poses, Some(Goal::ExactPose));
    assert_eq!(every_way(&exact_pose, &sticker_verdict()), [LEVEL_MISMATCH]);
}

/// The codes of `imply` from a certificate of `from` to `to`, of a state of `puzzle`.
fn implied(puzzle: Puzzle, from: Goal, to: Goal) -> Vec<&'static str> {
    let mut args = vec![var("h")];
    if to.takes_bound() {
        args.push(nat(3));
    }
    alone(vec![p("h", Ty::Holds(puzzle, from))], Ty::Holds(puzzle, to), imply(to, args))
}

/// CT-LG2-10 (part: the IR half; not run: the source half (T1b) and the run-time half (T1c), with the goal fixtures).
/// `imply` follows LG2.11's sourced chain and checkpoint implications and nothing else; equal labels do not require
/// equal poses, and a label checkpoint supplies no retained word.
#[test]
fn ct_lg2_10_imply_follows_the_sourced_chain() {
    use Goal::*;
    let edges = [
        (ExactPose, StickerTest),
        (ExactPose, LatticePlacement),
        (ExactPose, LowHeight),
        (StickerTest, LatticePlacement),
        (StickerTest, LowHeight),
        (LatticePlacement, LowHeight),
        (RetainedCheckpoint, LabelCheckpoint),
        (RetainedCheckpoint, LatticePlacement),
        (RetainedCheckpoint, LowHeight),
        (LabelCheckpoint, LatticePlacement),
        (LabelCheckpoint, LowHeight),
    ];
    for from in Goal::ALL {
        for to in Goal::ALL {
            let follows = from == to || edges.contains(&(from, to));
            assert_eq!(from.implies(to), follows, "{from:?} => {to:?}");
            let found = implied(Jumbling, from, to);
            if follows {
                assert_eq!(found, [] as [&str; 0], "{from:?} => {to:?}");
            } else {
                assert_eq!(found, [LEVEL_MISMATCH], "{from:?} => {to:?}");
            }
        }
    }
    let note = |from, to| {
        let body = imply(to, vec![var("h")]);
        only(&tool(vec![f(vec![p("h", Ty::Holds(Jumbling, from))], Ty::Holds(Jumbling, to), body)])).note
    };
    assert!(note(LabelCheckpoint, RetainedCheckpoint).contains("supplies no retained word"));
    assert!(note(StickerTest, LabelCheckpoint).contains("use check(label_checkpoint, s)"));
    // The sticker test holds of 600-cell-Full states; the goals it implies are defined on jumbling states only.
    assert_eq!(implied(Full, StickerTest, StickerTest), [] as [&str; 0]);
    assert_eq!(implied(Full, StickerTest, LatticePlacement), [IDENTITY_MISMATCH]);
    // The low-height bound is an exact `Nat`.
    let h = || vec![p("h", Ty::Holds(Jumbling, ExactPose)), param("n", display(Ty::Nat))];
    let r = report(&tool(vec![f(h(), Ty::Holds(Jumbling, LowHeight), imply(LowHeight, vec![var("h"), var("n")]))]));
    assert!(r.diagnostics.iter().any(|d| (d.code, d.path.as_str()) == (EXACTNESS_VIOLATION, "1")), "{r:#?}");
    let int = vec![p("h", Ty::Holds(Jumbling, ExactPose)), p("n", Ty::Int)];
    let d = only(&tool(vec![f(int, Ty::Holds(Jumbling, LowHeight), imply(LowHeight, vec![var("h"), var("n")]))]));
    assert_eq!((d.code, d.path.as_str()), (TYPE_MISMATCH, "1"));
    assert!(check(&tool(vec![f(h(), Ty::Holds(Jumbling, LowHeight), imply(LowHeight, vec![var("h")]))])).is_err());
    // Equal labels do not require equal poses: a labels verdict is no poses verdict, and a label checkpoint's
    // certificate implies no exact pose.
    let poses = Ty::Verdict(Jumbling, Level::Poses, Some(ExactPose));
    assert_eq!(every_way(&poses, &Ty::Verdict(Jumbling, Level::Labels, Some(LabelCheckpoint))), [LEVEL_MISMATCH]);
    assert_eq!(implied(Jumbling, LabelCheckpoint, ExactPose), [LEVEL_MISMATCH]);
    // A label checkpoint's certificate is no word.
    assert_eq!(as_argument(&Ty::Word, &Ty::Holds(Jumbling, LabelCheckpoint)), [TYPE_MISMATCH]);
    // `check(g, s)` reads the goal at its own level, on the puzzles the goal is defined on.
    for goal in Goal::ALL {
        for puzzle in Puzzle::ALL {
            let mut args = vec![var("s")];
            if goal.takes_bound() {
                args.push(nat(3));
            }
            let verdict = Ty::Verdict(puzzle, goal.level(), Some(goal));
            let body = kernel(KernelOp::Check(goal), args, exact(verdict.clone()));
            let found = alone(vec![p("s", state(puzzle))], verdict, body);
            if goal.puzzles().contains(&puzzle) {
                assert_eq!(found, [] as [&str; 0], "{goal:?} on {puzzle:?}");
            } else {
                assert_eq!(found, [IDENTITY_MISMATCH], "{goal:?} on {puzzle:?}");
            }
        }
    }
}

/// CT-LG2-11 (part: the IR half; not run: the source half (T1b) and the run-time half (T1c)). A derivation takes a
/// literal word of its puzzle's alphabet, never a word equality key, and keys of two puzzles do not compare.
#[test]
fn ct_lg2_11_derive_takes_literal_words_only() {
    let derive = |of: Puzzle, w: Ty| {
        let body = kernel(KernelOp::Derive, vec![var("s"), var("w")], exact(Ty::Derivation(of)));
        only(&tool(vec![f(vec![p("s", state(of)), p("w", w)], Ty::Derivation(of), body)]))
    };
    let d = derive(Full, Ty::WordEqualityKey(Full));
    assert_eq!(mismatch(&d), (WORD_KEY_NOT_EXECUTABLE, "f", "1", "Word", "WordEqualityKey<Full>"));
    let d = derive(Jumbling, Ty::WordEqualityKey(Jumbling));
    assert_eq!(mismatch(&d), (WORD_KEY_NOT_EXECUTABLE, "f", "1", "MenuWord", "WordEqualityKey<Jumbling>"));
    // The alphabet follows the puzzle: Jumble off for 600-cell-Full, on for the jumbling puzzle.
    let d = derive(Full, Ty::MenuWord);
    assert_eq!((d.code, d.path.as_str()), (MODE_MISMATCH, "1"));
    assert_eq!(derive(Jumbling, Ty::Word).code, MODE_MISMATCH);
    // A derivation is cased: `derived` carries the state, `refused` its record.
    let sw = || vec![p("s", state(Full)), p("w", Ty::Word)];
    let derived = || kernel(KernelOp::Derive, vec![var("s"), var("w")], exact(Ty::Derivation(Full)));
    let arms = vec![(pat("derived", &["_"]), nat(0)), (pat("refused", &["r"]), field(var("r"), "prefix"))];
    assert_eq!(alone(sw(), Ty::Nat, case(derived(), arms, None)), [] as [&str; 0]);
    assert_eq!(alone(sw(), state(Full), derived()), [TYPE_MISMATCH]);
    // A key keeps its word's puzzle.
    let key = |w: Ty, result: Ty| alone(vec![p("w", w)], result, prim(Prim::WordKey, vec![var("w")]));
    assert_eq!(key(Ty::Word, Ty::WordEqualityKey(Full)), [] as [&str; 0]);
    assert_eq!(key(Ty::MenuWord, Ty::WordEqualityKey(Jumbling)), [] as [&str; 0]);
    assert_eq!(key(Ty::MenuWord, Ty::WordEqualityKey(Full)), [IDENTITY_MISMATCH]);
    assert_eq!(every_way(&Ty::WordEqualityKey(Jumbling), &Ty::WordEqualityKey(Full)), [IDENTITY_MISMATCH]);
    assert_eq!(compared(&Ty::WordEqualityKey(Full), &Ty::WordEqualityKey(Jumbling)), [IDENTITY_MISMATCH]);
    assert_eq!(compared(&Ty::WordEqualityKey(Full), &Ty::WordEqualityKey(Full)), [] as [&str; 0]);
}

/// CT-LG2-06 (part: the IR half of the puzzle and unit indices; not run: the source half (T1b), the run-time half
/// (T1c) and the budget and seconds clauses). Costs add, order and compare only with costs of their puzzle and unit.
#[test]
fn ct_lg2_06_costs_keep_their_puzzle_and_unit() {
    use CostUnit::*;
    let c = Ty::Cost;
    let op = |op: Prim, a: Ty, b: Ty, result: Ty| {
        alone(vec![p("a", a), p("b", b)], result, prim(op, vec![var("a"), var("b")]))
    };
    let same = c(Full, Generator);
    assert_eq!(op(Prim::ExactAdd, same.clone(), same.clone(), same.clone()), [] as [&str; 0]);
    assert_eq!(op(Prim::ExactLess, same.clone(), same.clone(), Ty::Bool), [] as [&str; 0]);
    assert_eq!(op(Prim::ExactEq, same.clone(), same.clone(), Ty::Bool), [] as [&str; 0]);
    for other in [c(Full, MenuLetter), c(Full, UiAction), c(Jumbling, Generator), Ty::Nat] {
        assert_eq!(op(Prim::ExactAdd, same.clone(), other.clone(), Ty::Unit), [COST_UNIT_MIX], "{other:?}");
        assert_eq!(op(Prim::ExactLess, same.clone(), other.clone(), Ty::Bool), [COST_UNIT_MIX], "{other:?}");
        assert_eq!(op(Prim::ExactEq, same.clone(), other.clone(), Ty::Bool), [COST_UNIT_MIX], "{other:?}");
        assert_eq!(every_way(&same, &other), [COST_UNIT_MIX], "{other:?}");
    }
    // A count first: the refusal is at the count.
    let d = only(&tool(vec![f(vec![p("a", Ty::Nat), p("b", same.clone())], Ty::Unit, exact_add(var("a"), var("b")))]));
    assert_eq!((d.code, d.path.as_str()), (COST_UNIT_MIX, "0"));
    // A product of costs is no cost.
    assert_eq!(op(Prim::ExactMul, same.clone(), same.clone(), Ty::Unit), [TYPE_MISMATCH, TYPE_MISMATCH]);
    // A cost starts from an exact count and gives one back.
    let to_cost = |n: Expr| prim(Prim::ToCost(Full, Generator), vec![n]);
    assert_eq!(alone(vec![], same.clone(), to_cost(nat(3))), [] as [&str; 0]);
    assert_eq!(alone(vec![p("a", same.clone())], Ty::Nat, prim(Prim::CostValue, vec![var("a")])), [] as [&str; 0]);
    let r = report(&tool(vec![f(vec![param("n", display(Ty::Nat))], same.clone(), to_cost(var("n")))]));
    assert!(r.diagnostics.iter().any(|d| (d.code, d.path.as_str()) == (EXACTNESS_VIOLATION, "0")), "{r:#?}");
    assert_eq!(alone(vec![], c(Full, MenuLetter), to_cost(nat(3))), [COST_UNIT_MIX]);
}

/// The tower's keys and portable data (T1 draft): identities, poses, digests and costs are keys; identities, digests
/// and costs are portable data for a goal or an evidence sink. A pose-table index and the host values (a state, a level
/// value, a pose table, a claim, evidence, a verdict, a certificate, a derivation) are neither, and a pose is a key but
/// no portable data.
#[test]
fn tower_keys_and_portable_data() {
    let set_of = |ty: Ty| {
        let body = set(exact(ty.clone()), vec![var("v")]);
        check(&tool(vec![f(vec![p("v", ty.clone())], Ty::Set(Box::new(exact(ty))), body)])).map(|r| r.codes())
    };
    let goal = |ty: Ty| alone(vec![p("v", ty)], Ty::Unit, use_exact(ExactRole::Goal, var("v")));
    let data = [Ty::Piece(Full), Ty::Sticker(Jumbling), Ty::Slot(Full), Ty::Digest(Jumbling, Level::Labels)];
    for ty in data.into_iter().chain([Ty::Cost(Full, CostUnit::Generator)]) {
        assert_eq!(set_of(ty.clone()), Ok(vec![]), "{ty:?}");
        assert_eq!(goal(ty.clone()), [] as [&str; 0], "{ty:?}");
    }
    assert_eq!(set_of(Ty::Pose), Ok(vec![]));
    assert_eq!(goal(Ty::Pose), [TYPE_MISMATCH]);
    let host = [
        Ty::PoseTableIndex,
        state(Jumbling),
        Ty::LevelValue(Jumbling, Level::Poses),
        Ty::PoseTable,
        Ty::Reachability(Jumbling),
        Ty::Evidence,
        sticker_verdict(),
        Ty::Holds(Jumbling, Goal::ExactPose),
        Ty::Derivation(Full),
    ];
    for ty in host {
        // A set's element type that is no key is a fault of the IR, as for every other type (LG2.20).
        assert!(set_of(ty.clone()).is_err(), "{ty:?}");
        assert_eq!(goal(ty.clone()), [TYPE_MISMATCH], "{ty:?}");
    }
}
