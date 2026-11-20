//! The stage checker (RPI.68 to RPI.71, RPI.73, RPI.75; LG2.24, LG3.17, LG5.12; LP2 draft): the structural rules
//! [`check`] applies to every stage module, the limits [`admit_stage`] adds, and the refusals at the boundary between
//! programs and stages. The limits here are test values: RM.38's weights and the fixed checker bounds come from
//! measurement and are not set (RPI.79). The source form is tested in `c600-syntax`'s `stages.rs`.

use std::collections::BTreeMap;

use c600_check::{
    admit, admit_built_in, admit_stage, check, Diagnostic, Grant, Policy, Profile, StageLimits, STAGE_RULE_VIOLATION,
    TYPE_MISMATCH,
};
use c600_ir::build::*;
use c600_ir::{
    Accounting, Capability, EffectBound, ExactRole, Expr, Module, ModuleKind, Prim, Program, QTy, StageKind, StageOp,
    Ty,
};

/// Every weight 1 and every tier enabled; bounds wide enough for the tests' stages.
fn limits() -> StageLimits {
    let mut weights: BTreeMap<String, u64> = StageOp::ALL.iter().map(|op| (op.name().to_string(), 1)).collect();
    for name in ["approx_add", "approx_less", "approx_and", "approx_not", "display_if", "index"] {
        weights.insert(name.into(), 1);
    }
    StageLimits { tier: 3, weights, max_ops: 64, max_overdraw: 4, max_loop_count: 8, max_loop_depth: 2 }
}

fn record_ty(names: [&str; 3]) -> QTy {
    exact(Ty::Record(names.iter().map(|n| (n.to_string(), f32())).collect()))
}

fn position_ty() -> QTy {
    record_ty(["x", "y", "z"])
}

fn colour_ty() -> QTy {
    record_ty(["r", "g", "b"])
}

fn position(x: Expr, y: Expr, z: Expr) -> Expr {
    record(vec![("x", x), ("y", y), ("z", z)])
}

fn zero() -> Expr {
    f32_lit("0")
}

fn op(op: StageOp, args: Vec<Expr>) -> Expr {
    stage_op(op, args)
}

fn index(array: &str, at: u64) -> Expr {
    prim(Prim::Index, vec![var(array), nat(at)])
}

fn add(a: Expr, b: Expr) -> Expr {
    prim(Prim::ApproxAdd, vec![a, b])
}

/// A vertex stage `lens` whose entry `main(x, t, knots)` has a body of every construct the stage language has: a call
/// of a stage helper, `exclude`, `index` of the declared array `knots` (length 4), `repeat`, `display_if`, records and
/// tier 1 and tier 2 operations. With every weight 1 it counts 15 operations.
fn lens(ops: u64) -> Module {
    let scale = helper("scale", vec![param("v", f32())], f32(), op(StageOp::Mul, vec![var("v"), f32_lit("0.5")]));
    let body = let_(
        "s",
        call("scale", vec![var("x")]),
        let_(
            "cut",
            op(StageOp::Exclude, vec![op(StageOp::Sub, vec![var("x"), index("knots", 1)]), f32_lit("0.25")]),
            let_(
                "r",
                repeat(nat(3), var("s"), "acc", f32(), add(var("acc"), index("knots", 2))),
                position(
                    display_if(approx_less(var("t"), f32_lit("1")), var("r"), op(StageOp::Neg, vec![var("r")])),
                    op(StageOp::Sqrt, vec![op(StageOp::Abs, vec![var("t")])]),
                    zero(),
                ),
            ),
        ),
    );
    let params = vec![param("x", f32()), param("t", f32()), param("knots", exact(Ty::List(Box::new(f32()))))];
    let main = helper("main", params, position_ty(), body).exported();
    with_arrays(stage("lens", StageKind::Vertex, ops, 1, vec![scale, main]), &[("knots", 4)])
}

fn with_arrays(mut m: Module, arrays: &[(&str, u64)]) -> Module {
    if let ModuleKind::Stage(d) = &mut m.kind {
        d.arrays = arrays.iter().map(|(n, l)| (n.to_string(), *l)).collect();
    }
    m
}

/// A vertex stage `v` whose exported entry `main(x)` returns a position with `x` set to `body`.
fn vertex(body: Expr) -> Module {
    let main = helper("main", vec![param("x", f32())], position_ty(), position(body, zero(), zero())).exported();
    stage("v", StageKind::Vertex, 16, 1, vec![main])
}

/// A tool module with one exported helper `go() -> Unit`.
fn tool(body: Expr) -> Module {
    module("tool", vec![helper("go", vec![], exact(Ty::Unit), body).exported()])
}

fn findings(p: &Program) -> Vec<(&'static str, String, String)> {
    let r = check(p).unwrap_or_else(|e| panic!("{e}"));
    r.diagnostics.iter().map(|d| (d.code, d.function.clone(), d.path.clone())).collect()
}

fn at(code: &'static str, function: &str, path: &str) -> (&'static str, String, String) {
    (code, function.to_string(), path.to_string())
}

fn violation(function: &str, path: &str) -> (&'static str, String, String) {
    at(STAGE_RULE_VIOLATION, function, path)
}

/// The stage findings of a vertex stage `v` whose entry's `x` is `body`, which is at path `0` of the entry.
fn in_x(body: Expr) -> Vec<(&'static str, String, String)> {
    findings(&program(vec![vertex(body)]))
}

fn admission_codes(p: &Program, module: &str, entry: &str, l: &StageLimits) -> Vec<(&'static str, String, String)> {
    let a = admit_stage(p, module, entry, l).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(a.stage.is_some(), a.diagnostics.is_empty(), "{:#?}", a.diagnostics);
    a.diagnostics.iter().map(|d: &Diagnostic| (d.code, d.function.clone(), d.path.clone())).collect()
}

/// CT-LG2-15 (part: a stage is checked under RPI.68 to RPI.71 the same in each of the three profiles; a violation is
/// `LG2:type.stage_rule_violation` in each; no unchecked stage reaches the adapter, since only a stage admission
/// makes a `CheckedStage`; not run: RPI.72's raster bound, RPI.74's CPU reference evaluator, RPI.75's ID pass and
/// RPI.76's host statuses, which are the adapter's and the host's, and RM.38's measured weights and bounds).
#[test]
fn a_stage_is_checked_alike_in_every_profile_and_admitted_only_when_clean() {
    let clean = program(vec![lens(15), tool(unit())]);
    assert_eq!(findings(&clean), vec![]);
    let admitted = admit_stage(&clean, "lens", "main", &limits()).unwrap();
    assert!(admitted.diagnostics.is_empty(), "{:#?}", admitted.diagnostics);
    let stage = admitted.stage.expect("a checked stage");
    assert_eq!((stage.module(), stage.entry(), stage.kind()), ("lens", "main", StageKind::Vertex));
    assert_eq!(stage.counted_ops(), 15);
    assert_eq!(stage.definition().iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["lens"]);

    // A stage that reads the clock: one finding, the same under each profile's admission of the program's tool.
    let faulty = program(vec![vertex(Expr::ReadClock), tool(unit())]);
    let want = vec![violation("main", "0")];
    for profile in [Profile::Solving, Profile::ToolAuthoring, Profile::Research] {
        for (grant, policy) in [(Grant::none(), Policy::interim())] {
            let a = admit(&faulty, "tool", "go", profile, &grant, &policy).unwrap();
            let got: Vec<_> = a.diagnostics.iter().map(|d| (d.code, d.function.clone(), d.path.clone())).collect();
            assert_eq!(got, want, "{profile:?}");
            for d in &a.diagnostics {
                assert_eq!(d.module, "v");
            }
        }
        let clean_admission = admit(&clean, "tool", "go", profile, &Grant::none(), &Policy::interim()).unwrap();
        assert!(clean_admission.diagnostics.is_empty(), "{profile:?}: {:#?}", clean_admission.diagnostics);
    }
    let built_in = admit_built_in(&faulty, "tool", "go", &Default::default(), &Grant::none(), &Policy::interim());
    let got: Vec<_> =
        built_in.unwrap().diagnostics.iter().map(|d| (d.code, d.function.clone(), d.path.clone())).collect();
    assert_eq!(got, want);
    let refused = admit_stage(&faulty, "v", "main", &limits()).unwrap();
    assert!(refused.stage.is_none());
    assert_eq!(refused.codes(), [STAGE_RULE_VIOLATION]);
}

/// CT-LG2-15 (part: the stage's manifest and signatures, RPI.68 and RPI.69; not run: as above).
#[test]
fn a_stage_manifest_declares_no_capability_and_its_arrays() {
    let mut caps = vertex(zero());
    caps.capabilities.insert(Capability::Clock);
    assert_eq!(findings(&program(vec![caps])), vec![violation("", "manifest")]);

    // A network request is the stage checker's finding alone, not also the ordinary manifest's.
    let mut network = vertex(zero());
    network.capabilities.insert(Capability::Network);
    assert_eq!(findings(&program(vec![network])), vec![violation("", "manifest")]);

    // A module of stage helpers exports several functions, or none; the admission names the entry.
    let mut two = vertex(zero());
    two.functions.push(helper("other", vec![], f32(), zero()).exported());
    assert_eq!(findings(&program(vec![two])), vec![]);
    let mut none = vertex(zero());
    none.functions[0].exported = false;
    assert_eq!(findings(&program(vec![none])), vec![]);

    // An array names a `List<Approx<24>>` parameter of an exported function, once, with a length; an index into an
    // empty array is out of range at each of its positions.
    let undeclared = with_arrays(vertex(zero()), &[("knots", 4)]);
    assert_eq!(findings(&program(vec![undeclared])), vec![violation("", "manifest")]);
    let empty = with_arrays(lens(15), &[("knots", 0)]);
    let want = vec![violation("", "manifest"), violation("main", "1.0.0.1.1"), violation("main", "1.1.0.2.1.1")];
    assert_eq!(findings(&program(vec![empty])), want);
    let twice = with_arrays(lens(15), &[("knots", 4), ("knots", 4)]);
    assert_eq!(findings(&program(vec![twice])), vec![violation("", "manifest")]);

    // An exact parameter, a binary64 result, an array parameter of a function that is not exported.
    let mut exact_param = vertex(zero());
    exact_param.functions[0].params.push(param("n", exact(Ty::Nat)));
    assert_eq!(findings(&program(vec![exact_param])), vec![violation("main", "root")]);
    let wide = helper("wide", vec![], display(Ty::Approx(53)), f32_lit("1"));
    let mut wide_result = vertex(zero());
    wide_result.functions.push(wide);
    assert_eq!(findings(&program(vec![wide_result])), vec![violation("wide", "root")]);
    let mut hidden = lens(15);
    let list = exact(Ty::List(Box::new(f32())));
    hidden.functions.push(helper("peek", vec![param("knots", list)], f32(), index("knots", 0)));
    assert_eq!(findings(&program(vec![hidden])), vec![violation("peek", "root"), violation("peek", "0")]);

    // A command, a view, an effect annotation.
    let mut cmd = vertex(zero());
    cmd.functions[0].kind = c600_ir::FnKind::Command;
    assert_eq!(findings(&program(vec![cmd])), vec![violation("main", "root")]);
    let mut annotated = vertex(zero());
    annotated.functions[0].effects =
        Some(EffectBound { effects: [c600_ir::Effect::ReadSnapshot].into(), ..Default::default() });
    assert_eq!(findings(&program(vec![annotated])), vec![violation("main", "root")]);
}

/// CT-LG2-15 (part: the body rules of RPI.69 and RPI.70: binary32 only, exact literals, static indices and loop
/// bounds, no branch on exact data, no allocation or callable, no recursion, and RPI.75's fragment rule; not run: as
/// above).
#[test]
fn a_stage_body_is_binary32_with_static_bounds_and_no_recursion() {
    assert_eq!(in_x(op(StageOp::Div, vec![var("x"), f32_lit("3")])), vec![]);
    // Literals: exact finite binary32 values only.
    for ok in ["0.5", "-1.25", "16777216", "3e2", "0.0"] {
        assert_eq!(in_x(f32_lit(ok)), vec![], "{ok}");
    }
    for bad in ["0.1", "16777217", "1e39", "340282356779733661637539395458142568448"] {
        assert_eq!(in_x(f32_lit(bad)), vec![violation("main", "0")], "{bad}");
    }
    let wide = Expr::Lit(c600_ir::Lit::Approx { decimal: "1".into(), precision: 53 });
    assert_eq!(in_x(wide), vec![violation("main", "0")]);
    assert_eq!(in_x(nat(1)), vec![violation("main", "0")]);
    // An exact branch, a match, a closure, a collection, a function value.
    assert_eq!(in_x(if_(Expr::Lit(c600_ir::Lit::Bool(true)), zero(), zero())), vec![violation("main", "0")]);
    let closure = call_value(closure(vec![], f32(), zero()), vec![]);
    assert_eq!(in_x(closure), vec![violation("main", "0")]);
    assert_eq!(in_x(let_("f", fn_ref("main"), zero())), vec![violation("main", "0.0")]);
    let list = let_("l", c600_ir::build::list(f32(), vec![zero()]), zero());
    assert_eq!(in_x(list), vec![violation("main", "0.0")]);
    // A decoding of bytes the code holds allocates a text (LP2 draft).
    assert_eq!(in_x(let_("t", text_decode_hex(b"x"), zero())), vec![violation("main", "0.0")]);
    // A display branch types both branches alike.
    let mixed = display_if(approx_less(var("x"), zero()), zero(), approx_less(var("x"), zero()));
    assert_eq!(in_x(mixed), vec![at(TYPE_MISMATCH, "main", "0.2")]);
    // An operation outside the table, and an operation of another tier's type.
    assert_eq!(in_x(exact_add(nat(1), nat(2))), vec![violation("main", "0")]);
    assert_eq!(in_x(op(StageOp::Sin, vec![approx_less(var("x"), zero())])), vec![at(TYPE_MISMATCH, "main", "0.0")]);

    // Indices: a declared array, a literal position below its length.
    let at_index = |i: u64| {
        let mut m = lens(15);
        let main = &mut m.functions[1];
        main.body = position(index("knots", i), zero(), zero());
        findings(&program(vec![m]))
    };
    assert_eq!(at_index(3), vec![]);
    assert_eq!(at_index(4), vec![violation("main", "0.1")]);
    assert_eq!(in_x(prim(Prim::Index, vec![var("x"), nat(0)])), vec![violation("main", "0.0")]);
    let mut array_value = lens(15);
    array_value.functions[1].body = let_("k", var("knots"), position(zero(), zero(), zero()));
    assert_eq!(findings(&program(vec![array_value])), vec![violation("main", "0")]);

    // Loops: a literal count only.
    let looped = |count: Expr| repeat(count, zero(), "acc", f32(), add(var("acc"), var("x")));
    assert_eq!(in_x(looped(nat(8))), vec![]);
    assert_eq!(in_x(looped(var("x"))), vec![violation("main", "0.0")]);

    // Recursion, direct and mutual, is refused at each call of the cycle.
    let mut direct = vertex(call("main", vec![var("x")]));
    direct.functions[0].body = position(field(call("main", vec![var("x")]), "x"), zero(), zero());
    assert_eq!(findings(&program(vec![direct])), vec![violation("main", "0.0")]);
    let a = helper("a", vec![param("v", f32())], f32(), call("b", vec![var("v")]));
    let b = helper("b", vec![param("v", f32())], f32(), call("a", vec![var("v")]));
    let mut mutual = vertex(call("a", vec![var("x")]));
    mutual.functions.extend([a, b]);
    let got = findings(&program(vec![mutual]));
    assert_eq!(got, vec![violation("a", "root"), violation("b", "root")]);

    // `exclude` takes a positive exact literal radius, in a vertex stage only (RPI.75: a fragment never discards).
    let cut = |radius: &str| let_("c", op(StageOp::Exclude, vec![var("x"), f32_lit(radius)]), var("x"));
    assert_eq!(in_x(cut("0.25")), vec![]);
    assert_eq!(in_x(cut("0")), vec![violation("main", "0.0.1")]);
    assert_eq!(in_x(cut("-1")), vec![violation("main", "0.0.1")]);
    let discard = let_("c", op(StageOp::Exclude, vec![zero(), f32_lit("1")]), zero());
    let colour = record(vec![("r", discard), ("g", zero()), ("b", zero())]);
    let fragment = stage("f", StageKind::Fragment, 4, 1, vec![helper("main", vec![], colour_ty(), colour).exported()]);
    assert_eq!(findings(&program(vec![fragment])), vec![violation("main", "0.0")]);
}

/// CT-LG3-10 (whole: a stage admits only `pure`; a direct or transitive snapshot, workspace, preview, file,
/// randomness, clock, budget or other effect request is refused, in the stage or in any function it calls).
#[test]
fn a_stage_admits_only_pure_directly_and_transitively() {
    let effects: Vec<Expr> = vec![
        read_snapshot("positions", f32()),
        read_snapshot_with("positions", zero(), f32()),
        publish_view(zero()),
        write_workspace("r", f32(), zero()),
        propose(word(&[1]), target("operation.preview")),
        Expr::FileRead { path: "a".into(), ty: Ty::Approx(24) },
        Expr::FileWrite { path: "a".into(), value: Box::new(zero()) },
        Expr::RandomDraw,
        rng_init(&["a"]),
        Expr::ReadClock,
        use_exact(ExactRole::Budget, nat(1)),
        Expr::Accounting(Accounting::UsedSteps),
        Expr::Declassified { id: "d".into(), ty: Ty::Approx(24) },
    ];
    for e in effects {
        assert_eq!(in_x(e.clone()), vec![violation("main", "0")], "{e:?}");
    }
    // Transitively through a stage helper in another stage module: refused there, and the admission of the calling
    // stage carries it.
    let probe = helper("probe", vec![], f32(), read_snapshot("positions", f32())).exported();
    let lib = stage("lib", StageKind::Vertex, 1, 1, vec![probe]);
    let user = vertex(call_in("lib", "probe", vec![]));
    let p = program(vec![lib, user]);
    assert_eq!(findings(&p), vec![violation("probe", "root")]);
    let a = admit_stage(&p, "v", "main", &limits()).unwrap();
    assert!(a.stage.is_none());
    assert_eq!(
        a.diagnostics.iter().map(|d| (d.module.as_str(), d.code)).collect::<Vec<_>>(),
        [("lib", STAGE_RULE_VIOLATION)]
    );
    // Through an ordinary function, even a pure one: a stage calls stage functions only, of its own kind.
    let pure = module("util", vec![helper("half", vec![param("v", f32())], f32(), var("v")).exported()]);
    let caller = vertex(call_in("util", "half", vec![var("x")]));
    assert_eq!(findings(&program(vec![pure, caller])), vec![violation("main", "0")]);
    let colour = helper("tint", vec![], f32(), zero()).exported();
    let fragment_lib = stage("tints", StageKind::Fragment, 1, 1, vec![colour]);
    let mixed = vertex(call_in("tints", "tint", vec![]));
    assert_eq!(findings(&program(vec![fragment_lib, mixed])), vec![violation("main", "0")]);
    // An unresolved call.
    let unresolved =
        Expr::Call { callee: c600_ir::Callee::Unresolved { name: "f".into(), result: f32() }, args: vec![] };
    assert_eq!(in_x(unresolved), vec![violation("main", "0")]);
}

/// CT-LG2-15 (part: no program calls, holds or runs a stage, and no stage result returns to a program, RPI.68 and
/// RPI.73; not run: as above).
#[test]
fn no_program_calls_holds_or_runs_a_stage() {
    let args = vec![f32_lit("1"), f32_lit("0"), c600_ir::build::list(f32(), vec![])];
    let calls = tool(let_("p", call_in("lens", "main", args), unit()));
    let got = findings(&program(vec![lens(15), calls]));
    assert!(got.contains(&violation("go", "0")), "{got:?}");
    let holds = tool(let_("f", Expr::FnRef { module: Some("lens".into()), name: "main".into() }, unit()));
    let got = findings(&program(vec![lens(15), holds]));
    assert!(got.contains(&violation("go", "0")), "{got:?}");
    // A stage is never a run's entry.
    let p = program(vec![lens(15)]);
    for profile in [Profile::Solving, Profile::ToolAuthoring, Profile::Research] {
        let a = admit(&p, "lens", "main", profile, &Grant::none(), &Policy::interim()).unwrap();
        assert!(a.diagnostics.iter().any(|d| d.code == STAGE_RULE_VIOLATION && d.path == "entry"), "{profile:?}");
    }
    let a = admit_built_in(&p, "lens", "main", &Default::default(), &Grant::none(), &Policy::interim()).unwrap();
    assert!(a.diagnostics.iter().any(|d| d.code == STAGE_RULE_VIOLATION && d.path == "entry"));
    // A stage operation outside a stage module is malformed IR: lowering refuses it there.
    let outside = tool(let_("y", stage_op(StageOp::Neg, vec![f32_lit("1")]), unit()));
    assert!(check(&program(vec![outside])).is_err());
    // `admit_stage` takes a function of a stage module only.
    assert!(admit_stage(&program(vec![tool(unit())]), "tool", "go", &limits()).is_err());
    assert!(admit_stage(&program(vec![lens(15)]), "lens", "absent", &limits()).is_err());
}

/// CT-LG2-15 (part: RPI.70's tiers and RPI.71's declared costs and checker bounds, with test limits; not run: as
/// above).
#[test]
fn admission_charges_tiers_costs_and_bounds() {
    let p = program(vec![lens(15)]);
    assert_eq!(admission_codes(&p, "lens", "main", &limits()), vec![]);
    // A tier above the enabled ones: `approx_sqrt` is tier 2.
    let tier1 = StageLimits { tier: 1, ..limits() };
    assert_eq!(admission_codes(&p, "lens", "main", &tier1), vec![violation("main", "1.1.1.1")]);
    // An operation without a measured weight.
    let mut unweighted = limits();
    unweighted.weights.remove("approx_abs");
    assert_eq!(admission_codes(&p, "lens", "main", &unweighted), vec![violation("main", "1.1.1.1.0")]);
    // Declared costs: below the count, above the bounds.
    assert_eq!(admission_codes(&program(vec![lens(14)]), "lens", "main", &limits()), vec![violation("", "manifest")]);
    let tight = StageLimits { max_ops: 14, ..limits() };
    assert_eq!(admission_codes(&program(vec![lens(15)]), "lens", "main", &tight), vec![violation("", "manifest")]);
    let mut overdrawn = lens(15);
    if let ModuleKind::Stage(d) = &mut overdrawn.kind {
        d.overdraw = 5;
    }
    assert_eq!(admission_codes(&program(vec![overdrawn]), "lens", "main", &limits()), vec![violation("", "manifest")]);
    // Loop bounds: the count, and the nesting through inlined calls.
    let short = StageLimits { max_loop_count: 2, ..limits() };
    assert_eq!(admission_codes(&p, "lens", "main", &short), vec![violation("main", "1.1.0.0")]);
    let inner =
        helper("inner", vec![param("v", f32())], f32(), repeat(nat(2), var("v"), "a", f32(), add(var("a"), var("v"))));
    let nested = repeat(nat(2), var("x"), "b", f32(), call("inner", vec![var("b")]));
    let mut deep = vertex(nested);
    deep.functions.push(inner);
    let p = program(vec![deep]);
    assert_eq!(admission_codes(&p, "v", "main", &limits()), vec![]);
    let flat = StageLimits { max_loop_depth: 1, ..limits() };
    assert_eq!(admission_codes(&p, "v", "main", &flat), vec![violation("inner", "root")]);
    // A function walked more than once, called twice or inside a loop, is refused once at each place.
    let root = helper("root", vec![param("v", f32())], f32(), op(StageOp::Sqrt, vec![var("v")]));
    let twice = add(call("root", vec![var("x")]), repeat(nat(2), var("x"), "c", f32(), call("root", vec![var("c")])));
    let mut called = vertex(twice);
    called.functions.push(root);
    let called = program(vec![called]);
    assert_eq!(admission_codes(&called, "v", "main", &tier1), vec![violation("root", "root")]);
    assert_eq!(admission_codes(&called, "v", "main", &limits()), vec![]);
    // The count multiplies: 2 × (2 × 1) = 4 operations.
    let a = admit_stage(&p, "v", "main", &limits()).unwrap();
    assert_eq!(a.stage.unwrap().counted_ops(), 4);
}

/// CT-LG2-15 (part: the adapter names the entry, which is exported, takes every declared array and returns its
/// kind's position, RPI.68 and RPI.69; not run: as above).
#[test]
fn admission_names_an_exported_entry_of_the_stage_kind() {
    // The entry's result is the stage kind's: a position for a vertex stage, a colour for a fragment stage.
    let colour = helper("main", vec![], colour_ty(), record(vec![("r", zero()), ("g", zero()), ("b", zero())]));
    let fragment = program(vec![stage("f", StageKind::Fragment, 4, 1, vec![colour.clone().exported()])]);
    assert_eq!(admission_codes(&fragment, "f", "main", &limits()), vec![]);
    let wrong = program(vec![stage("f", StageKind::Vertex, 4, 1, vec![colour.exported()])]);
    assert_eq!(findings(&wrong), vec![]);
    assert_eq!(admission_codes(&wrong, "f", "main", &limits()), vec![violation("main", "entry")]);

    // A function that is not exported is no entry.
    let mut none = vertex(zero());
    none.functions[0].exported = false;
    assert_eq!(admission_codes(&program(vec![none]), "v", "main", &limits()), vec![violation("main", "entry")]);

    // A second exported function of `lens` may be an entry only if it takes the declared array.
    let mut two = lens(15);
    let alt = helper("alt", vec![param("x", f32())], position_ty(), position(var("x"), zero(), zero()));
    two.functions.push(alt.exported());
    let p = program(vec![two]);
    assert_eq!(findings(&p), vec![]);
    assert_eq!(admission_codes(&p, "lens", "main", &limits()), vec![]);
    assert_eq!(admission_codes(&p, "lens", "alt", &limits()), vec![violation("alt", "entry")]);
    // `scale` is neither exported, nor a position, nor takes `knots`: three refusals at the entry.
    let want = vec![violation("scale", "entry"), violation("scale", "entry"), violation("scale", "entry")];
    assert_eq!(admission_codes(&p, "lens", "scale", &limits()), want);
}
